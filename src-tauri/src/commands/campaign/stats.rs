//! Campaign Stats Commands
//!
//! Commands for retrieving campaign statistics.

use tauri::State;

use crate::commands::AppState;
use crate::core::campaign_manager::CampaignStats;
use crate::core::session_manager::SessionStatus;
use crate::database::NpcOps;

// ============================================================================
// Campaign Stats Commands
// ============================================================================

/// Get statistics for a campaign including session count, NPC count, and playtime.
#[tauri::command]
pub async fn get_campaign_stats(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<CampaignStats, String> {
    // 1. Get Session Stats
    let sessions = state.session_manager.list_sessions(&campaign_id);
    let session_count = sessions.len();
    let total_playtime_minutes: i64 = sessions.iter()
        .filter_map(|s| s.duration_minutes)
        .sum();

    // Find last played (most recent active/ended session)
    let last_played = sessions.iter()
        .filter(|s| s.status != SessionStatus::Planned)
        .map(|s| s.started_at) // Approximate default to started_at for sort
        .max();

    // 2. Get NPC Count
    // Helper to get count from DB/Store
    let npc_count = {
        let npcs = state.database.list_npcs(Some(&campaign_id)).await.unwrap_or_default();
        npcs.len()
    };

    Ok(CampaignStats {
        session_count,
        npc_count,
        total_playtime_minutes,
        last_played,
    })
}
