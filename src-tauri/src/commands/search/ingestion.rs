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
///
/// TODO: Phase 3 Migration - Update to use EmbeddedSearch/MeilisearchLib
#[tauri::command]
#[allow(unused_variables)]
pub async fn ingest_document(
    path: String,
    options: Option<IngestOptions>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let path_obj = Path::new(&path);
    if !path_obj.is_file() {
        return Err(format!("File not found or is a directory: {}", path));
    }

    let _opts = options.unwrap_or_default();

    // TODO: Migrate to embedded MeilisearchLib
    // The old implementation used:
    //   state.ingestion_pipeline.ingest_two_phase(&state.search_client, ...)
    //
    // The new implementation should use:
    //   state.embedded_search.inner() -> &MeilisearchLib
    //   Update MeilisearchPipeline to work with MeilisearchLib
    //
    // Access via: state.embedded_search.inner()
    let _meili = state.embedded_search.inner();

    log::warn!(
        "ingest_document() called but not yet migrated to embedded MeilisearchLib. Path: {}",
        path
    );

    // Return error for now - full migration in Phase 3 Task 5
    Err(format!(
        "Document ingestion not yet available - migration in progress. Path: {}",
        path
    ))
}

/// Ingest a document using two-phase pipeline with per-document indexes.
///
/// Phase 1: Extract pages to `<slug>-raw` index (one doc per page)
/// Phase 2: Create semantic chunks in `<slug>` index with provenance tracking
///
/// This enables page number attribution in search results by tracking
/// which raw pages each chunk was derived from.
///
/// TODO: Phase 3 Migration - Update MeilisearchPipeline to work with EmbeddedSearch/MeilisearchLib
#[tauri::command]
#[allow(unused_variables)]
pub async fn ingest_document_two_phase(
    app: tauri::AppHandle,
    path: String,
    title_override: Option<String>,
    state: State<'_, AppState>,
) -> Result<TwoPhaseIngestResult, String> {
    use tauri::Emitter;
    use crate::core::meilisearch_pipeline::generate_source_slug;

    let path_buf = std::path::PathBuf::from(&path);
    if !path_buf.is_file() {
        return Err(format!("File not found or is a directory: {}", path));
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

    // TODO: Migrate to embedded MeilisearchLib
    // The old implementation used:
    //   pipeline.extract_to_raw(&state.search_client, ...) -> Phase 1
    //   pipeline.chunk_from_raw(&state.search_client, &extraction) -> Phase 2
    //
    // The new implementation should update MeilisearchPipeline to accept &MeilisearchLib:
    //   state.embedded_search.inner() -> &MeilisearchLib
    //   meili.add_documents(uid, docs, primary_key) for indexing
    //
    // Access via: state.embedded_search.inner()
    let _meili = state.embedded_search.inner();

    log::warn!(
        "ingest_document_two_phase() called but not yet migrated to embedded MeilisearchLib. Path: {}",
        path
    );

    // Emit error progress
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "error".to_string(),
        progress: 0.0,
        message: "Two-phase ingestion not yet available - migration in progress".to_string(),
        source_name: source_name.clone(),
    });

    // Return error for now - full migration in Phase 3 Task 5
    Err(format!(
        "Two-phase ingestion not yet available - migration in progress. Path: {}",
        path
    ))
}

/// Import a pre-extracted layout JSON file (Anthropic format).
///
/// This command imports JSON files that contain pre-extracted document layout
/// with pages and elements, bypassing the extraction step.
///
/// TODO: Phase 3 Migration - Update to use EmbeddedSearch/MeilisearchLib
#[tauri::command]
#[allow(unused_variables)]
pub async fn import_layout_json(
    app: tauri::AppHandle,
    path: String,
    title_override: Option<String>,
    state: State<'_, AppState>,
) -> Result<TwoPhaseIngestResult, String> {
    use tauri::Emitter;
    use crate::ingestion::layout_json::LayoutDocument;

    let path_buf = std::path::PathBuf::from(&path);
    if !path_buf.is_file() {
        return Err(format!("File not found or is a directory: {}", path));
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

    // TODO: Migrate to embedded MeilisearchLib
    // The old implementation used:
    //   state.ingestion_pipeline.import_layout_json(&state.search_client, ...)
    //   state.ingestion_pipeline.chunk_from_raw(&state.search_client, &extraction)
    //
    // The new implementation should update MeilisearchPipeline to accept &MeilisearchLib:
    //   state.embedded_search.inner() -> &MeilisearchLib
    //
    // Access via: state.embedded_search.inner()
    let _meili = state.embedded_search.inner();

    log::warn!(
        "import_layout_json() called but not yet migrated to embedded MeilisearchLib. Path: {}",
        path
    );

    // Emit error progress
    let _ = app.emit("ingest-progress", IngestProgress {
        stage: "error".to_string(),
        progress: 0.0,
        message: "Layout JSON import not yet available - migration in progress".to_string(),
        source_name: source_name.clone(),
    });

    // Return error for now - full migration in Phase 3 Task 5
    Err(format!(
        "Layout JSON import not yet available - migration in progress. Path: {}",
        path
    ))
}

/// Ingest a PDF document using two-phase pipeline.
///
/// TODO: Phase 3 Migration - Update to use EmbeddedSearch/MeilisearchLib
#[tauri::command]
#[allow(unused_variables)]
pub async fn ingest_pdf(
    path: String,
    state: State<'_, AppState>,
) -> Result<IngestResult, String> {
    let path_buf = std::path::PathBuf::from(&path);
    if !path_buf.is_file() {
        return Err(format!("File not found or is a directory: {}", path));
    }

    // TODO: Migrate to embedded MeilisearchLib
    // The old implementation used:
    //   state.ingestion_pipeline.ingest_two_phase(&state.search_client, ...)
    //
    // Access via: state.embedded_search.inner()
    let _meili = state.embedded_search.inner();

    log::warn!(
        "ingest_pdf() called but not yet migrated to embedded MeilisearchLib. Path: {}",
        path
    );

    // Return error for now - full migration in Phase 3 Task 5
    Err(format!(
        "PDF ingestion not yet available - migration in progress. Path: {}",
        path
    ))
}
