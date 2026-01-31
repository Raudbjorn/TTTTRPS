//! TTRPG Document Commands
//!
//! Commands for querying TTRPG documents (monsters, spells, items, etc.)
//! from the database.

use crate::database::TtrpgOps;
use crate::with_db;

// ============================================================================
// TTRPG Document Query Commands
// ============================================================================

/// List TTRPG documents by source document ID
#[tauri::command]
pub async fn list_ttrpg_documents_by_source(
    source_document_id: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<crate::database::TTRPGDocumentRecord>, String> {
    with_db!(db, |db| db.list_ttrpg_documents_by_source(&source_document_id))
}

/// List TTRPG documents by element type
#[tauri::command]
pub async fn list_ttrpg_documents_by_type(
    element_type: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<crate::database::TTRPGDocumentRecord>, String> {
    with_db!(db, |db| db.list_ttrpg_documents_by_type(&element_type))
}

/// List TTRPG documents by game system
#[tauri::command]
pub async fn list_ttrpg_documents_by_system(
    game_system: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<crate::database::TTRPGDocumentRecord>, String> {
    with_db!(db, |db| db.list_ttrpg_documents_by_system(&game_system))
}

/// Search TTRPG documents by name pattern
#[tauri::command]
pub async fn search_ttrpg_documents_by_name(
    name_pattern: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<crate::database::TTRPGDocumentRecord>, String> {
    with_db!(db, |db| db.search_ttrpg_documents_by_name(&name_pattern))
}

/// List TTRPG documents by challenge rating range
#[tauri::command]
pub async fn list_ttrpg_documents_by_cr(
    min_cr: f64,
    max_cr: f64,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<crate::database::TTRPGDocumentRecord>, String> {
    with_db!(db, |db| db.list_ttrpg_documents_by_cr(min_cr, max_cr))
}

/// Get a specific TTRPG document by ID
#[tauri::command]
pub async fn get_ttrpg_document(
    id: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Option<crate::database::TTRPGDocumentRecord>, String> {
    with_db!(db, |db| db.get_ttrpg_document(&id))
}

/// Get attributes for a TTRPG document
#[tauri::command]
pub async fn get_ttrpg_document_attributes(
    document_id: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<crate::database::TTRPGDocumentAttribute>, String> {
    with_db!(db, |db| db.get_ttrpg_document_attributes(&document_id))
}

/// Find TTRPG documents by attribute
#[tauri::command]
pub async fn find_ttrpg_documents_by_attribute(
    attribute_type: String,
    attribute_value: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<crate::database::TTRPGDocumentRecord>, String> {
    with_db!(db, |db| db.find_ttrpg_documents_by_attribute(&attribute_type, &attribute_value))
}

/// Delete a TTRPG document
#[tauri::command]
pub async fn delete_ttrpg_document(
    id: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<(), String> {
    with_db!(db, |db| db.delete_ttrpg_document(&id))
}

/// Get TTRPG document statistics
#[tauri::command]
pub async fn get_ttrpg_document_stats(
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<crate::database::TTRPGDocumentStats, String> {
    with_db!(db, |db| db.get_ttrpg_document_stats())
}

/// Count TTRPG documents grouped by type
#[tauri::command]
pub async fn count_ttrpg_documents_by_type(
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<(String, i64)>, String> {
    with_db!(db, |db| db.count_ttrpg_documents_by_type())
}

/// Get TTRPG ingestion job status
#[tauri::command]
pub async fn get_ttrpg_ingestion_job(
    job_id: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Option<crate::database::TTRPGIngestionJob>, String> {
    with_db!(db, |db| db.get_ttrpg_ingestion_job(&job_id))
}

/// Get TTRPG ingestion job for a document
#[tauri::command]
pub async fn get_ttrpg_ingestion_job_by_document(
    document_id: String,
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Option<crate::database::TTRPGIngestionJob>, String> {
    with_db!(db, |db| db.get_ttrpg_ingestion_job_by_document(&document_id))
}

/// List pending TTRPG ingestion jobs
#[tauri::command]
pub async fn list_pending_ttrpg_ingestion_jobs(
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<crate::database::TTRPGIngestionJob>, String> {
    with_db!(db, |db| db.list_pending_ttrpg_ingestion_jobs())
}

/// List active TTRPG ingestion jobs
#[tauri::command]
pub async fn list_active_ttrpg_ingestion_jobs(
    db: tauri::State<'_, std::sync::Arc<tokio::sync::RwLock<Option<crate::database::Database>>>>,
) -> Result<Vec<crate::database::TTRPGIngestionJob>, String> {
    with_db!(db, |db| db.list_active_ttrpg_ingestion_jobs())
}
