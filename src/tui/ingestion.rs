//! Async ingestion pipeline orchestrator for TUI.
//!
//! Extracts text from a document, chunks it semantically, optionally
//! generates embeddings, and stores the chunks in SurrealDB — sending
//! progress events through the TUI event channel at each phase.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::core::search::embeddings::EmbeddingProvider;
use crate::core::storage::ingestion::{ingest_chunks, ChunkData};
use crate::core::storage::models::update_library_item_status;
use crate::core::storage::surrealdb::SurrealStorage;
use crate::ingestion::chunker::SemanticChunker;
use crate::ingestion::kreuzberg_extractor::DocumentExtractor;
use crate::tui::events::{AppEvent, IngestionProgressKind};

/// Batch size for embedding generation.
const EMBEDDING_BATCH_SIZE: usize = 32;

/// Run the full ingestion pipeline: extract -> chunk -> embed -> store.
///
/// Sends `IngestionProgress` events through `event_tx` at each phase.
/// On failure, sets the library item status to "error" and sends an `Error` event.
///
/// If `embedding_provider` is `None`, chunks are stored without embeddings.
///
/// Returns the number of chunks stored on success.
pub async fn run_ingestion_pipeline(
    file_path: PathBuf,
    library_item_id: String,
    content_type: String,
    storage: SurrealStorage,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
) -> Result<usize, String> {
    let db = storage.db();
    let item_id = library_item_id.clone();

    // Helper to send progress events (best-effort, ignore channel errors)
    let send_progress = |phase: IngestionProgressKind| {
        let _ = event_tx.send(AppEvent::IngestionProgress {
            library_item_id: item_id.clone(),
            phase,
        });
    };

    // ── 1. Extract ───────────────────────────────────────────────────────
    let progress_tx = event_tx.clone();
    let progress_item_id = library_item_id.clone();

    let extractor = DocumentExtractor::with_ocr();
    let extracted = extractor
        .extract(&file_path, Some(move |progress: f32, status: &str| {
            let _ = progress_tx.send(AppEvent::IngestionProgress {
                library_item_id: progress_item_id.clone(),
                phase: IngestionProgressKind::Extracting {
                    progress,
                    status: status.to_string(),
                },
            });
        }))
        .await
        .map_err(|e| format!("Extraction failed: {e}"))?;

    // ── 2. Chunk ─────────────────────────────────────────────────────────
    let chunker = SemanticChunker::new();
    let source_id = &library_item_id;

    let content_chunks = if let Some(ref pages) = extracted.pages {
        let page_tuples: Vec<(u32, String)> = pages
            .iter()
            .map(|p| (p.page_number as u32, p.content.clone()))
            .collect();
        chunker.chunk_with_pages(&page_tuples, source_id)
    } else {
        chunker.chunk_text(&extracted.content, source_id)
    };

    let chunk_count = content_chunks.len();
    send_progress(IngestionProgressKind::Chunking { chunk_count });

    // ── 3. Convert to ChunkData ──────────────────────────────────────────
    let mut chunk_data: Vec<ChunkData> = content_chunks
        .into_iter()
        .map(|cc| ChunkData {
            content: cc.content,
            content_type: content_type.clone(),
            page_number: cc.page_number.map(|p| p as i32),
            section_path: cc.section.clone(),
            chapter_title: cc.chapter_title,
            section_title: cc.subsection_title,
            chunk_type: Some(cc.chunk_type),
            semantic_keywords: if cc.semantic_keywords.is_empty() {
                None
            } else {
                Some(cc.semantic_keywords)
            },
            ..Default::default()
        })
        .collect();

    // ── 4. Embed (optional) ──────────────────────────────────────────────
    if let Some(ref provider) = embedding_provider {
        let total = chunk_data.len();
        send_progress(IngestionProgressKind::Embedding {
            processed: 0,
            total,
        });

        let model_name = provider.name().to_string();
        let mut processed = 0;

        for batch_start in (0..total).step_by(EMBEDDING_BATCH_SIZE) {
            let batch_end = (batch_start + EMBEDDING_BATCH_SIZE).min(total);
            let texts: Vec<&str> = chunk_data[batch_start..batch_end]
                .iter()
                .map(|c| c.content.as_str())
                .collect();

            match provider.embed_batch(&texts).await {
                Ok(embeddings) => {
                    for (i, embedding) in embeddings.into_iter().enumerate() {
                        chunk_data[batch_start + i].embedding = Some(embedding);
                        chunk_data[batch_start + i].embedding_model =
                            Some(model_name.clone());
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Embedding batch {batch_start}..{batch_end} failed: {e} — storing without vectors"
                    );
                    // Continue without embeddings for this batch
                }
            }

            processed = batch_end;
            send_progress(IngestionProgressKind::Embedding { processed, total });
        }

        let embedded_count = chunk_data.iter().filter(|c| c.embedding.is_some()).count();
        log::info!(
            "Embedded {embedded_count}/{total} chunks with {model_name}"
        );
    }

    // ── 5. Store ─────────────────────────────────────────────────────────
    let total = chunk_data.len();
    send_progress(IngestionProgressKind::Storing { stored: 0, total });

    // ingest_chunks auto-sets library_item status to "ready" on success
    let inserted = ingest_chunks(db, &library_item_id, chunk_data)
        .await
        .map_err(|e| format!("Storage failed: {e}"))?;

    // Update page_count from extraction metadata
    if extracted.page_count > 0 {
        let _ = db
            .query("UPDATE type::thing('library_item', $id) SET page_count = $pages, updated_at = time::now()")
            .bind(("id", library_item_id.clone()))
            .bind(("pages", extracted.page_count as i32))
            .await;
    }

    send_progress(IngestionProgressKind::Complete {
        chunk_count: inserted,
    });

    Ok(inserted)
}

/// Wrapper that runs the pipeline and handles errors by updating library item status.
pub async fn run_ingestion_with_error_handling(
    file_path: PathBuf,
    library_item_id: String,
    content_type: String,
    storage: SurrealStorage,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
) {
    let storage_for_error = storage.clone();
    let item_id = library_item_id.clone();

    match run_ingestion_pipeline(
        file_path,
        library_item_id,
        content_type,
        storage,
        embedding_provider,
        event_tx.clone(),
    )
    .await
    {
        Ok(count) => {
            log::info!("Ingestion complete: {count} chunks stored for {item_id}");
        }
        Err(error) => {
            log::error!("Ingestion failed for {item_id}: {error}");
            let db = storage_for_error.db();
            let _ = update_library_item_status(db, &item_id, "error", Some(&error)).await;
            let _ = event_tx.send(AppEvent::IngestionProgress {
                library_item_id: item_id,
                phase: IngestionProgressKind::Error(error),
            });
        }
    }
}
