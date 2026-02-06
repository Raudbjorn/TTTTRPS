//! Library Management Commands
//!
//! Commands for listing, deleting, updating, and managing library documents.

use std::time::Duration;

use tauri::State;

use crate::commands::AppState;
use crate::core::search::{
    all_indexes, LibraryDocumentMetadata, INDEX_LIBRARY_METADATA, TASK_TIMEOUT_SHORT_SECS,
};
use super::types::{UpdateLibraryDocumentRequest, IngestResult, IngestProgress};

// ============================================================================
// Library Document Management
// ============================================================================

/// List all documents from the library (persisted in Meilisearch)
#[tauri::command]
pub async fn list_library_documents(
    state: State<'_, AppState>,
) -> Result<Vec<LibraryDocumentMetadata>, String> {
    let meili = state.embedded_search.clone_inner();

    tokio::task::spawn_blocking(move || {
        // First check if the index exists
        let index_exists = meili
            .index_exists(INDEX_LIBRARY_METADATA)
            .map_err(|e| e.to_string())?;

        if !index_exists {
            log::debug!("Library metadata index does not exist yet, returning empty list");
            return Ok(Vec::new());
        }

        // Get all documents from the library metadata index
        // Using a high limit since we want all documents
        let (_total, docs) = meili
            .get_documents(INDEX_LIBRARY_METADATA, 0, 10000)
            .map_err(|e| format!("Failed to list library documents: {}", e))?;

        // Deserialize each document into LibraryDocumentMetadata
        docs.into_iter()
            .map(|doc| {
                serde_json::from_value(doc)
                    .map_err(|e| format!("Failed to deserialize library document: {}", e))
            })
            .collect()
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Delete a document from the library (removes metadata and content chunks)
#[tauri::command]
pub async fn delete_library_document(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let meili = state.embedded_search.clone_inner();
    let doc_id = id.clone();

    tokio::task::spawn_blocking(move || {
        log::info!("Deleting library document: {}", doc_id);

        // First, get the document to find its content_index
        let doc = meili
            .get_document(INDEX_LIBRARY_METADATA, &doc_id)
            .map_err(|e| format!("Failed to get library document: {}", e))?;

        let metadata: LibraryDocumentMetadata = serde_json::from_value(doc)
            .map_err(|e| format!("Failed to deserialize library document: {}", e))?;

        let content_index = &metadata.content_index;
        log::debug!(
            "Library document {} uses content index: {}",
            doc_id,
            content_index
        );

        // Delete content chunks from the content index
        // Content chunks have IDs that start with the document ID followed by a separator
        // We need to search for all documents with source matching this document
        if let Ok(exists) = meili.index_exists(content_index) {
            if exists {
                // Search for content chunks belonging to this document
                // Using filter on source field which contains the document path/name
                let search_query = meilisearch_lib::SearchQuery::empty()
                    .with_pagination(0, 10000)
                    .with_attributes_to_retrieve(vec!["id".to_string()]);

                if let Ok(results) = meili.search(content_index, search_query) {
                    // Collect IDs of chunks that belong to this document
                    // Chunk IDs typically follow the pattern: {doc_id}-{chunk_index}
                    let chunk_ids: Vec<String> = results
                        .hits
                        .iter()
                        .filter_map(|hit| {
                            hit.document
                                .get("id")
                                .and_then(|v| v.as_str())
                                .filter(|chunk_id| chunk_id.starts_with(&doc_id))
                                .map(String::from)
                        })
                        .collect();

                    if !chunk_ids.is_empty() {
                        log::debug!(
                            "Deleting {} content chunks from index {}",
                            chunk_ids.len(),
                            content_index
                        );

                        let task = meili
                            .delete_documents_batch(content_index, chunk_ids)
                            .map_err(|e| format!("Failed to delete content chunks: {}", e))?;

                        meili
                            .wait_for_task(task.uid, Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)))
                            .map_err(|e| format!("Failed waiting for content deletion: {}", e))?;
                    }
                }
            }
        }

        // Delete the metadata document
        let task = meili
            .delete_document(INDEX_LIBRARY_METADATA, &doc_id)
            .map_err(|e| format!("Failed to delete library document metadata: {}", e))?;

        meili
            .wait_for_task(task.uid, Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)))
            .map_err(|e| format!("Failed waiting for metadata deletion: {}", e))?;

        log::info!("Successfully deleted library document: {}", doc_id);
        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Update a library document's TTRPG metadata
#[tauri::command]
pub async fn update_library_document(
    request: UpdateLibraryDocumentRequest,
    state: State<'_, AppState>,
) -> Result<LibraryDocumentMetadata, String> {
    let meili = state.embedded_search.clone_inner();

    tokio::task::spawn_blocking(move || {
        log::info!("Updating library document metadata: {}", request.id);

        // Get the existing document
        let doc = meili
            .get_document(INDEX_LIBRARY_METADATA, &request.id)
            .map_err(|e| format!("Failed to get library document: {}", e))?;

        let mut metadata: LibraryDocumentMetadata = serde_json::from_value(doc)
            .map_err(|e| format!("Failed to deserialize library document: {}", e))?;

        // Update the TTRPG metadata fields if provided
        if request.game_system.is_some() {
            metadata.game_system = request.game_system;
        }
        if request.setting.is_some() {
            metadata.setting = request.setting;
        }
        if request.content_type.is_some() {
            metadata.content_type = request.content_type;
        }
        if request.publisher.is_some() {
            metadata.publisher = request.publisher;
        }

        // Save the updated document
        let doc_value = serde_json::to_value(&metadata)
            .map_err(|e| format!("Failed to serialize library document: {}", e))?;

        let task = meili
            .add_documents(INDEX_LIBRARY_METADATA, vec![doc_value], Some("id".to_string()))
            .map_err(|e| format!("Failed to save library document: {}", e))?;

        meili
            .wait_for_task(task.uid, Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)))
            .map_err(|e| format!("Failed waiting for document update: {}", e))?;

        log::info!("Successfully updated library document: {}", request.id);
        Ok(metadata)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Rebuild library metadata from existing content indices.
///
/// Scans all content indices for unique sources and creates metadata entries
/// for sources that don't already have entries. Useful for migrating legacy data.
#[tauri::command]
pub async fn rebuild_library_metadata(
    state: State<'_, AppState>,
) -> Result<usize, String> {
    // all_indexes is imported at the top of the module
    use std::collections::{HashMap, HashSet};

    let meili = state.embedded_search.clone_inner();

    tokio::task::spawn_blocking(move || {
        log::info!("Rebuilding library metadata from content indices");

        // Get existing library document IDs to avoid duplicates
        let existing_ids: HashSet<String> = {
            let exists = meili
                .index_exists(INDEX_LIBRARY_METADATA)
                .map_err(|e| e.to_string())?;

            if exists {
                let (_total, docs) = meili
                    .get_documents(INDEX_LIBRARY_METADATA, 0, 10000)
                    .unwrap_or((0, Vec::new()));

                docs.iter()
                    .filter_map(|doc| doc.get("id").and_then(|v| v.as_str()).map(String::from))
                    .collect()
            } else {
                HashSet::new()
            }
        };

        log::debug!("Found {} existing library documents", existing_ids.len());

        // Collect unique sources from all content indices
        // Map: source -> (content_index, page_count, chunk_count, character_count)
        let mut source_stats: HashMap<String, (String, u32, u32, u64)> = HashMap::new();

        let content_indexes = all_indexes();

        for index_name in content_indexes {
            let exists = meili.index_exists(index_name).unwrap_or(false);
            if !exists {
                continue;
            }

            log::debug!("Scanning index: {}", index_name);

            // Search for all documents in this index
            let search_query = meilisearch_lib::SearchQuery::empty()
                .with_pagination(0, 10000)
                .with_attributes_to_retrieve(vec![
                    "id".to_string(),
                    "source".to_string(),
                    "page_number".to_string(),
                    "content".to_string(),
                ]);

            if let Ok(results) = meili.search(index_name, search_query) {
                for hit in results.hits {
                    if let Some(source) = hit.document.get("source").and_then(|v| v.as_str()) {
                        let source = source.to_string();

                        let page = hit
                            .document
                            .get("page_number")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32;

                        let char_count = hit
                            .document
                            .get("content")
                            .and_then(|v| v.as_str())
                            .map(|s| s.len() as u64)
                            .unwrap_or(0);

                        let entry =
                            source_stats.entry(source).or_insert((index_name.to_string(), 0, 0, 0));
                        entry.1 = entry.1.max(page); // max page number
                        entry.2 += 1; // chunk count
                        entry.3 += char_count; // total characters
                    }
                }
            }
        }

        log::debug!("Found {} unique sources across content indices", source_stats.len());

        // Create metadata entries for sources that don't exist
        let mut created_count = 0;
        let now = chrono::Utc::now().to_rfc3339();

        for (source, (content_index, max_page, chunk_count, char_count)) in source_stats {
            // Generate ID from source (file path)
            let id = generate_document_id(&source);

            if existing_ids.contains(&id) {
                log::debug!("Skipping existing document: {}", id);
                continue;
            }

            // Extract file name from source path
            let name = std::path::Path::new(&source)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&source)
                .to_string();

            // Detect source type from extension
            let source_type = std::path::Path::new(&source)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("document")
                .to_lowercase();

            let metadata = LibraryDocumentMetadata {
                id: id.clone(),
                name,
                source_type,
                file_path: Some(source),
                page_count: max_page + 1, // Pages are 0-indexed
                chunk_count,
                character_count: char_count,
                content_index,
                status: "ready".to_string(),
                error_message: None,
                ingested_at: now.clone(),
                game_system: None,
                setting: None,
                content_type: None,
                publisher: None,
            };

            let doc_value = serde_json::to_value(&metadata)
                .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

            let task = meili
                .add_documents(INDEX_LIBRARY_METADATA, vec![doc_value], Some("id".to_string()))
                .map_err(|e| format!("Failed to add metadata document: {}", e))?;

            meili
                .wait_for_task(task.uid, Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)))
                .map_err(|e| format!("Failed waiting for metadata add: {}", e))?;

            created_count += 1;
            log::debug!("Created metadata for: {}", id);
        }

        log::info!(
            "Rebuilt library metadata: created {} new entries",
            created_count
        );
        Ok(created_count)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Generate a document ID from a file path
fn generate_document_id(source: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    format!("doc-{:x}", hasher.finish())
}

/// Clear a document's content and re-ingest from the original file.
///
/// Useful when ingestion produced garbage content (e.g., failed font decoding)
/// and you want to try again (possibly with OCR this time).
#[tauri::command]
pub async fn clear_and_reingest_document(
    id: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<IngestResult, String> {
    use tauri::Emitter;

    let meili = state.embedded_search.clone_inner();
    let doc_id = id.clone();

    // Step 1: Get the document metadata to find the file path
    let (file_path, source_type) = tokio::task::spawn_blocking(move || {
        let doc = meili
            .get_document(INDEX_LIBRARY_METADATA, &doc_id)
            .map_err(|e| format!("Failed to get library document: {}", e))?;

        let metadata: LibraryDocumentMetadata = serde_json::from_value(doc)
            .map_err(|e| format!("Failed to deserialize library document: {}", e))?;

        let file_path = metadata.file_path.ok_or_else(|| {
            format!(
                "Cannot re-ingest document {}: no file path stored",
                doc_id
            )
        })?;

        Ok::<_, String>((file_path, metadata.source_type))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))??;

    log::info!("Re-ingesting document {} from path: {}", id, file_path);

    // Emit progress event
    let _ = app.emit(
        "ingest-progress",
        IngestProgress {
            stage: "clearing".to_string(),
            progress: 0.1,
            message: format!("Clearing existing content for re-ingestion..."),
            source_name: file_path.clone(),
        },
    );

    // Step 2: Delete the existing document and its content
    delete_library_document(id.clone(), state.clone())
        .await
        .map_err(|e| format!("Failed to clear existing document: {}", e))?;

    // Step 3: Re-ingest the document
    // Note: ingest_document_with_progress_internal is not yet migrated,
    // so this will fail with a migration-in-progress error.
    // When the ingestion pipeline is migrated, this will work.
    ingest_document_with_progress_internal(
        file_path,
        Some(source_type),
        Some(id), // Preserve the original document ID
        app,
        state,
    )
    .await
}

/// Internal ingestion logic shared by ingest_document_with_progress and clear_and_reingest.
///
/// Uses the two-phase pipeline (extract → raw index → chunk index) for all supported formats.
///
/// # Arguments
/// * `original_id` - If provided, preserves the original document ID (useful for re-ingestion)
///
/// TODO: Phase 3 Migration - This function needs to be updated to work with EmbeddedSearch/MeilisearchLib:
///   1. MeilisearchPipeline::ingest_two_phase() currently takes &SearchClient (HTTP SDK)
///   2. Need to either:
///      a) Update MeilisearchPipeline to accept &MeilisearchLib directly, OR
///      b) Create an adapter that implements the required operations using MeilisearchLib
///   3. save_library_document() also uses SearchClient - need to migrate to MeilisearchLib document ops
///
/// See: meili-dev/crates/meilisearch-lib/src/documents.rs for MeilisearchLib document operations
#[allow(unused_variables)]
pub(crate) async fn ingest_document_with_progress_internal(
    path: String,
    source_type: Option<String>,
    original_id: Option<String>,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<IngestResult, String> {
    use tauri::Emitter;

    let path_buf = std::path::PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(format!("File not found: {}", path));
    }

    let source_name = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let _source_type = source_type.unwrap_or_else(|| "document".to_string());

    // Emit initial progress
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "starting".to_string(),
        progress: 0.0,
        message: format!("Starting ingestion for {}...", source_name),
        source_name: source_name.clone(),
    });

    // TODO: Migrate to embedded MeilisearchLib
    // The old implementation used:
    //   state.ingestion_pipeline.ingest_two_phase(&state.search_client, ...)
    //   state.search_client.save_library_document(&library_doc)
    //
    // The new implementation should use:
    //   state.embedded_search.inner() -> &MeilisearchLib
    //   meili.add_documents(uid, docs, primary_key) for indexing
    //   meili.search(uid, query) for retrieval
    //
    // Access via: state.embedded_search.inner()
    let _meili = state.embedded_search.inner();

    log::warn!(
        "ingest_document_with_progress_internal() called but not yet migrated to embedded MeilisearchLib. Path: {}",
        path
    );

    // Emit error progress
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "error".to_string(),
        progress: 0.0,
        message: "Document ingestion not yet available - migration in progress".to_string(),
        source_name: source_name.clone(),
    });

    // Return error for now - full migration in Phase 3 Task 5
    Err(format!(
        "Document ingestion not yet available - migration in progress. Path: {}",
        path
    ))
}
