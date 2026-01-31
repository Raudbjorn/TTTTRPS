//! Library Management Commands
//!
//! Commands for listing, deleting, updating, and managing library documents.

use tauri::State;

use crate::commands::AppState;
use super::types::{UpdateLibraryDocumentRequest, IngestResult, IngestProgress};

// ============================================================================
// Library Document Management
// ============================================================================

/// List all documents from the library (persisted in Meilisearch)
#[tauri::command]
pub async fn list_library_documents(
    state: State<'_, AppState>,
) -> Result<Vec<crate::core::search::LibraryDocumentMetadata>, String> {
    state.search_client
        .list_library_documents()
        .await
        .map_err(|e| format!("Failed to list documents: {}", e))
}

/// Delete a document from the library (removes metadata and content chunks)
#[tauri::command]
pub async fn delete_library_document(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.search_client
        .delete_library_document_with_content(&id)
        .await
        .map_err(|e| format!("Failed to delete document: {}", e))
}

/// Update a library document's TTRPG metadata
#[tauri::command]
pub async fn update_library_document(
    request: UpdateLibraryDocumentRequest,
    state: State<'_, AppState>,
) -> Result<crate::core::search::LibraryDocumentMetadata, String> {
    // Fetch existing document
    let mut doc = state.search_client
        .get_library_document(&request.id)
        .await
        .map_err(|e| format!("Failed to fetch document: {}", e))?
        .ok_or_else(|| format!("Document not found: {}", request.id))?;

    // Update TTRPG metadata fields
    doc.game_system = request.game_system;
    doc.setting = request.setting;
    doc.content_type = request.content_type;
    doc.publisher = request.publisher;

    // Save updated document
    state.search_client
        .save_library_document(&doc)
        .await
        .map_err(|e| format!("Failed to save document: {}", e))?;

    log::info!("Updated library document metadata: {}", request.id);
    Ok(doc)
}

/// Rebuild library metadata from existing content indices.
///
/// Scans all content indices for unique sources and creates metadata entries
/// for sources that don't already have entries. Useful for migrating legacy data.
#[tauri::command]
pub async fn rebuild_library_metadata(
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let created = state.search_client
        .rebuild_library_metadata()
        .await
        .map_err(|e| format!("Failed to rebuild metadata: {}", e))?;

    Ok(created.len())
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

    // Get the document metadata to find the file path
    let doc = state.search_client
        .get_library_document(&id)
        .await
        .map_err(|e| format!("Failed to get document: {}", e))?
        .ok_or_else(|| "Document not found".to_string())?;

    let file_path = doc.file_path
        .ok_or_else(|| "Document has no file path - cannot re-ingest".to_string())?;

    // Verify file still exists
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("Original file no longer exists: {}", file_path));
    }

    log::info!("Clearing and re-ingesting document: {} ({})", doc.name, id);

    // Delete existing content and metadata
    state.search_client
        .delete_library_document_with_content(&id)
        .await
        .map_err(|e| format!("Failed to delete existing content: {}", e))?;

    // Emit progress for clearing
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "clearing".to_string(),
        progress: 0.05,
        message: format!("Cleared old content, re-ingesting {}...", doc.name),
        source_name: doc.name.clone(),
    });

    // Re-ingest using the existing ingest logic
    let source_type = Some(doc.source_type.clone());

    // Call the internal ingestion logic, preserving the original document ID
    ingest_document_with_progress_internal(
        file_path,
        source_type,
        Some(id),  // Preserve original ID on re-ingestion
        app,
        state,
    ).await
}

/// Internal ingestion logic shared by ingest_document_with_progress and clear_and_reingest.
///
/// Uses the two-phase pipeline (extract → raw index → chunk index) for all supported formats.
///
/// # Arguments
/// * `original_id` - If provided, preserves the original document ID (useful for re-ingestion)
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

    let source_type = source_type.unwrap_or_else(|| "document".to_string());

    // Emit initial progress
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "starting".to_string(),
        progress: 0.0,
        message: format!("Starting ingestion for {}...", source_name),
        source_name: source_name.clone(),
    });

    // Use two-phase pipeline for ingestion
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "extracting".to_string(),
        progress: 0.1,
        message: format!("Extracting content from {}...", source_name),
        source_name: source_name.clone(),
    });

    let (extraction, chunking) = state.ingestion_pipeline
        .ingest_two_phase(
            &state.search_client,
            &path_buf,
            None, // No title override
        )
        .await
        .map_err(|e| format!("Ingestion failed: {}", e))?;

    // Save document metadata (preserve original_id if provided for re-ingestion)
    let library_doc = crate::core::search::LibraryDocumentMetadata {
        id: original_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        name: source_name.clone(),
        source_type: source_type.clone(),
        file_path: Some(path.clone()),
        page_count: extraction.page_count as u32,
        chunk_count: chunking.chunk_count as u32,
        character_count: extraction.total_chars as u64,
        content_index: chunking.chunks_index.clone(),
        status: "ready".to_string(),
        error_message: None,
        ingested_at: chrono::Utc::now().to_rfc3339(),
        // TTRPG metadata from extraction (auto-detected)
        game_system: extraction.ttrpg_metadata.game_system.clone(),
        setting: None,
        content_type: extraction.ttrpg_metadata.content_category.clone(),
        publisher: None,
    };

    if let Err(e) = state.search_client.save_library_document(&library_doc).await {
        log::warn!("Failed to save library document metadata: {}. Document indexed but may not persist.", e);
    }

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

    Ok(IngestResult {
        page_count: extraction.page_count,
        character_count: extraction.total_chars,
        source_name: extraction.source_name,
    })
}
