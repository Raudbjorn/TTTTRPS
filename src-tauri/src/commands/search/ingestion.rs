//! Document Ingestion Commands
//!
//! Commands for ingesting documents into the search index.

use std::path::Path;
use tauri::State;

use crate::commands::AppState;
use super::types::{IngestOptions, TwoPhaseIngestResult, IngestResult, IngestProgress};

// ============================================================================
// Document Ingestion Commands
// ============================================================================

/// Ingest a document using two-phase pipeline (simplified interface).
///
/// This is a convenience wrapper around `ingest_document_two_phase` that
/// uses the two-phase workflow (extract → raw index → chunk index).
#[tauri::command]
pub async fn ingest_document(
    path: String,
    options: Option<IngestOptions>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let path_obj = Path::new(&path);
    if !path_obj.exists() {
        return Err(format!("File not found: {}", path));
    }

    let opts = options.unwrap_or_default();

    // Use two-phase pipeline for ingestion
    let (extraction, chunking) = state.ingestion_pipeline
        .ingest_two_phase(
            &state.search_client,
            path_obj,
            opts.title_override.as_deref(),
        )
        .await
        .map_err(|e| format!("Ingestion failed: {}", e))?;

    Ok(format!(
        "Ingested '{}': {} pages → {} chunks (indexes: {}, {})",
        extraction.source_name,
        extraction.page_count,
        chunking.chunk_count,
        extraction.raw_index,
        chunking.chunks_index
    ))
}

/// Ingest a document using two-phase pipeline with per-document indexes.
///
/// Phase 1: Extract pages to `<slug>-raw` index (one doc per page)
/// Phase 2: Create semantic chunks in `<slug>` index with provenance tracking
///
/// This enables page number attribution in search results by tracking
/// which raw pages each chunk was derived from.
#[tauri::command]
pub async fn ingest_document_two_phase(
    app: tauri::AppHandle,
    path: String,
    title_override: Option<String>,
    state: State<'_, AppState>,
) -> Result<TwoPhaseIngestResult, String> {
    use tauri::Emitter;
    use crate::core::meilisearch_pipeline::{MeilisearchPipeline, generate_source_slug};

    let path_buf = std::path::PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(format!("File not found: {}", path));
    }

    let source_name = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Generate slug for progress messages
    let slug = generate_source_slug(&path_buf, title_override.as_deref());

    log::info!("Starting two-phase ingestion for '{}' (slug: {})", source_name, slug);

    // Emit initial progress
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "starting".to_string(),
        progress: 0.0,
        message: format!("Starting two-phase ingestion for {}...", source_name),
        source_name: source_name.clone(),
    });

    let pipeline = MeilisearchPipeline::with_defaults();

    // Phase 1: Extract to raw pages
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "extracting".to_string(),
        progress: 0.1,
        message: format!("Phase 1: Extracting pages from {}...", source_name),
        source_name: source_name.clone(),
    });

    let extraction = pipeline
        .extract_to_raw(&state.search_client, &path_buf, title_override.as_deref())
        .await
        .map_err(|e| format!("Extraction failed: {}", e))?;

    log::info!(
        "Phase 1 complete: {} pages extracted to '{}' (system: {:?})",
        extraction.page_count,
        extraction.raw_index,
        extraction.ttrpg_metadata.game_system
    );

    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "extracted".to_string(),
        progress: 0.5,
        message: format!("Extracted {} pages, creating semantic chunks...", extraction.page_count),
        source_name: source_name.clone(),
    });

    // Phase 2: Create semantic chunks
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "chunking".to_string(),
        progress: 0.6,
        message: format!("Phase 2: Creating semantic chunks for {}...", source_name),
        source_name: source_name.clone(),
    });

    let chunking = pipeline
        .chunk_from_raw(&state.search_client, &extraction)
        .await
        .map_err(|e| format!("Chunking failed: {}", e))?;

    log::info!(
        "Phase 2 complete: {} chunks created in '{}' from {} pages",
        chunking.chunk_count,
        chunking.chunks_index,
        chunking.pages_consumed
    );

    // Done!
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "complete".to_string(),
        progress: 1.0,
        message: format!(
            "Ingested {} pages -> {} chunks (indexes: {}, {})",
            extraction.page_count,
            chunking.chunk_count,
            extraction.raw_index,
            chunking.chunks_index
        ),
        source_name: source_name.clone(),
    });

    Ok(TwoPhaseIngestResult {
        slug: extraction.slug,
        source_name: extraction.source_name,
        raw_index: extraction.raw_index,
        chunks_index: chunking.chunks_index,
        page_count: extraction.page_count,
        chunk_count: chunking.chunk_count,
        total_chars: extraction.total_chars,
        game_system: extraction.ttrpg_metadata.game_system,
        content_category: extraction.ttrpg_metadata.content_category,
    })
}

/// Import a pre-extracted layout JSON file (Anthropic format).
///
/// This command imports JSON files that contain pre-extracted document layout
/// with pages and elements, bypassing the extraction step.
#[tauri::command]
pub async fn import_layout_json(
    app: tauri::AppHandle,
    path: String,
    title_override: Option<String>,
    state: State<'_, AppState>,
) -> Result<TwoPhaseIngestResult, String> {
    use tauri::Emitter;
    use crate::ingestion::layout_json::LayoutDocument;

    let path_buf = std::path::PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(format!("File not found: {}", path));
    }

    // Verify it's a valid layout JSON
    if !LayoutDocument::is_layout_json(&path_buf) {
        return Err("File does not appear to be a valid layout JSON file".to_string());
    }

    let source_name = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    log::info!("Importing layout JSON: {}", source_name);

    // Emit initial progress
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "importing".to_string(),
        progress: 0.1,
        message: format!("Importing layout JSON: {}...", source_name),
        source_name: source_name.clone(),
    });

    // Import using the pipeline method
    let extraction = state.ingestion_pipeline
        .import_layout_json(
            &state.search_client,
            &path_buf,
            title_override.as_deref(),
        )
        .await
        .map_err(|e| format!("Layout JSON import failed: {}", e))?;

    // Emit progress for chunking
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "chunking".to_string(),
        progress: 0.5,
        message: format!("Chunking {} pages...", extraction.page_count),
        source_name: source_name.clone(),
    });

    // Process raw pages into semantic chunks (same as two-phase)
    let chunking = state.ingestion_pipeline
        .chunk_from_raw(
            &state.search_client,
            &extraction,
        )
        .await
        .map_err(|e| format!("Chunking failed: {}", e))?;

    log::info!(
        "Layout JSON import complete: {} -> {} pages -> {} chunks",
        source_name, extraction.page_count, chunking.chunk_count
    );

    // Emit completion
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "complete".to_string(),
        progress: 1.0,
        message: format!(
            "Imported {} pages -> {} chunks (indexes: {}, {})",
            extraction.page_count,
            chunking.chunk_count,
            extraction.raw_index,
            chunking.chunks_index
        ),
        source_name: source_name.clone(),
    });

    Ok(TwoPhaseIngestResult {
        slug: extraction.slug,
        source_name: extraction.source_name,
        raw_index: extraction.raw_index,
        chunks_index: chunking.chunks_index,
        page_count: extraction.page_count,
        chunk_count: chunking.chunk_count,
        total_chars: extraction.total_chars,
        game_system: extraction.ttrpg_metadata.game_system,
        content_category: extraction.ttrpg_metadata.content_category,
    })
}

/// Ingest a PDF document using two-phase pipeline.
#[tauri::command]
pub async fn ingest_pdf(
    path: String,
    state: State<'_, AppState>,
) -> Result<IngestResult, String> {
    let path_buf = std::path::Path::new(&path);

    // Use two-phase pipeline for ingestion
    let (extraction, _chunking) = state.ingestion_pipeline
        .ingest_two_phase(
            &state.search_client,
            path_buf,
            None, // No title override
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(IngestResult {
        page_count: extraction.page_count,
        character_count: extraction.total_chars,
        source_name: extraction.source_name,
    })
}
