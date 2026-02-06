//! NPC Index Commands
//!
//! Commands for managing NPC Meilisearch indexes.
//!
//! TODO: These commands relied on meilisearch_sdk::Client which came from
//! the HTTP-based SearchClient. With embedded MeilisearchLib, the npc_gen
//! module functions (ensure_npc_indexes, get_npc_index_stats, clear_npc_indexes)
//! need to be migrated to use MeilisearchLib's direct API instead.

use tauri::State;

use crate::commands::AppState;
// TODO: Re-enable when npc_gen is migrated to MeilisearchLib
// use crate::core::npc_gen::{NpcIndexStats, ensure_npc_indexes, get_npc_index_stats};

// ============================================================================
// NPC Index Commands
// ============================================================================

/// Initialize NPC extension indexes in Meilisearch
///
/// TODO: Needs migration to MeilisearchLib. The ensure_npc_indexes function
/// expects meilisearch_sdk::Client which we no longer have with embedded search.
#[tauri::command]
pub async fn initialize_npc_indexes(
    _state: State<'_, AppState>,
) -> Result<(), String> {
    // TODO: Migrate to MeilisearchLib API
    // ensure_npc_indexes(state.search_client.get_client())
    //     .await
    //     .map_err(|e| e.to_string())
    log::warn!("initialize_npc_indexes: Not yet migrated to MeilisearchLib");
    Ok(())
}

/// Get NPC index statistics
///
/// TODO: Needs migration to MeilisearchLib. The get_npc_index_stats function
/// expects meilisearch_sdk::Client which we no longer have with embedded search.
#[tauri::command]
pub async fn get_npc_indexes_stats(
    _state: State<'_, AppState>,
) -> Result<crate::core::npc_gen::NpcIndexStats, String> {
    // TODO: Migrate to MeilisearchLib API
    // get_npc_index_stats(state.search_client.get_client())
    //     .await
    //     .map_err(|e| e.to_string())
    log::warn!("get_npc_indexes_stats: Not yet migrated to MeilisearchLib");
    Ok(crate::core::npc_gen::NpcIndexStats::default())
}

/// Clear NPC indexes
///
/// TODO: Needs migration to MeilisearchLib. The clear_npc_indexes function
/// expects meilisearch_sdk::Client which we no longer have with embedded search.
#[tauri::command]
pub async fn clear_npc_indexes(
    _state: State<'_, AppState>,
) -> Result<(), String> {
    // TODO: Migrate to MeilisearchLib API
    // crate::core::npc_gen::clear_npc_indexes(state.search_client.get_client())
    //     .await
    //     .map_err(|e| e.to_string())
    log::warn!("clear_npc_indexes: Not yet migrated to MeilisearchLib");
    Ok(())
}
