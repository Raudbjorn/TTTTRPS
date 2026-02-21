//! NPC Index Commands
//!
//! Commands for managing NPC Meilisearch indexes using embedded meilisearch-lib.
//! All MeilisearchLib methods are synchronous, so we wrap them in
//! `tokio::task::spawn_blocking` to avoid blocking the async runtime.

use tauri::State;

use crate::commands::AppState;

// ============================================================================
// NPC Index Commands
// ============================================================================

/// Initialize NPC extension indexes in embedded Meilisearch.
///
/// Creates vocabulary banks, name components, and exclamation template indexes
/// with their configured settings. Idempotent - safe to call multiple times.
#[tauri::command]
pub async fn initialize_npc_indexes(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let meili = state.embedded_search.clone_inner();

    tokio::task::spawn_blocking(move || {
        crate::core::npc_gen::indexes::ensure_npc_indexes(&meili)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e| e.to_string())
}

/// Get NPC index statistics.
///
/// Returns document counts for vocabulary banks, name components,
/// and exclamation template indexes.
#[tauri::command]
pub async fn get_npc_indexes_stats(
    state: State<'_, AppState>,
) -> Result<crate::core::npc_gen::NpcIndexStats, String> {
    let meili = state.embedded_search.clone_inner();

    tokio::task::spawn_blocking(move || {
        crate::core::npc_gen::indexes::get_npc_index_stats(&meili)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e| e.to_string())
}

/// Clear all NPC indexes.
///
/// Deletes all documents from vocabulary banks, name components,
/// and exclamation template indexes. The indexes themselves are preserved.
///
/// **Warning**: This will delete all indexed NPC generation data!
#[tauri::command]
pub async fn clear_npc_indexes(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let meili = state.embedded_search.clone_inner();

    tokio::task::spawn_blocking(move || {
        crate::core::npc_gen::indexes::clear_npc_indexes(&meili)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e| e.to_string())
}
