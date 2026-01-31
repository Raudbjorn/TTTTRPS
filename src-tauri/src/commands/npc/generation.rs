//! NPC Generation Commands
//!
//! Commands for generating new NPCs.

use tauri::State;

use crate::commands::AppState;
use crate::core::npc_gen::{NPCGenerator, NPCGenerationOptions, NPC};
use crate::database::NpcOps;

// Helper function for enum serialization
fn serialize_enum_to_string<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .map(|s| s.trim_matches('"').to_string())
        .unwrap_or_default()
}

// ============================================================================
// NPC Generation Commands
// ============================================================================

/// Generate a new NPC and save to store and database
#[tauri::command]
pub async fn generate_npc(
    options: NPCGenerationOptions,
    campaign_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<NPC, String> {
    let generator = NPCGenerator::new();
    let npc = generator.generate_quick(&options);

    // Save to memory store
    state.npc_store.add(npc.clone(), campaign_id.as_deref());

    // Save to Database
    let personality_json = serde_json::to_string(&npc.personality).map_err(|e| e.to_string())?;
    let stats_json = npc.stats.as_ref().map(|s| serde_json::to_string(s).unwrap_or_default());
    let role_str = serialize_enum_to_string(&npc.role);
    let data_json = serde_json::to_string(&npc).map_err(|e| e.to_string())?;

    let record = crate::database::NpcRecord {
        id: npc.id.clone(),
        campaign_id: campaign_id.clone(),
        name: npc.name.clone(),
        role: role_str,
        personality_id: None,
        personality_json,
        data_json: Some(data_json),
        stats_json,
        notes: Some(npc.notes.clone()),
        location_id: None,
        voice_profile_id: None,
        quest_hooks: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    state.database.save_npc(&record).await.map_err(|e| e.to_string())?;

    Ok(npc)
}
