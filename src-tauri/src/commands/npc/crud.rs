//! NPC CRUD Commands
//!
//! Commands for retrieving, listing, updating, deleting, and searching NPCs.

use tauri::State;

use crate::commands::AppState;
use crate::core::npc_gen::NPC;
use crate::database::NpcOps;

// Helper function for enum serialization
fn serialize_enum_to_string<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .map(|s| s.trim_matches('"').to_string())
        .unwrap_or_default()
}

// ============================================================================
// NPC CRUD Commands
// ============================================================================

/// Retrieve an NPC by ID (from store or database fallback)
#[tauri::command]
pub async fn get_npc(id: String, state: State<'_, AppState>) -> Result<Option<NPC>, String> {
    if let Some(npc) = state.npc_store.get(&id) {
        return Ok(Some(npc));
    }

    if let Some(record) = state.database.get_npc(&id).await.map_err(|e| e.to_string())? {
        if let Some(json) = record.data_json {
             let npc: NPC = serde_json::from_str(&json).map_err(|e| e.to_string())?;
             state.npc_store.add(npc.clone(), record.campaign_id.as_deref());
             return Ok(Some(npc));
        }
    }
    Ok(None)
}

/// List all NPCs for a campaign
#[tauri::command]
pub async fn list_npcs(campaign_id: Option<String>, state: State<'_, AppState>) -> Result<Vec<NPC>, String> {
    let records = state.database.list_npcs(campaign_id.as_deref()).await.map_err(|e| e.to_string())?;
    let mut npcs = Vec::new();

    for r in records {
        if let Some(json) = r.data_json {
             if let Ok(npc) = serde_json::from_str::<NPC>(&json) {
                 npcs.push(npc);
             }
        }
    }

    if npcs.is_empty() {
        let mem_npcs = state.npc_store.list(campaign_id.as_deref());
        if !mem_npcs.is_empty() {
            return Ok(mem_npcs);
        }
    }

    Ok(npcs)
}

/// Update an existing NPC in store and database
#[tauri::command]
pub async fn update_npc(npc: NPC, state: State<'_, AppState>) -> Result<(), String> {
    state.npc_store.update(npc.clone());

    let personality_json = serde_json::to_string(&npc.personality).map_err(|e| e.to_string())?;
    let stats_json = npc.stats.as_ref().map(|s| serde_json::to_string(s).unwrap_or_default());
    let role_str = serialize_enum_to_string(&npc.role);
    let data_json = serde_json::to_string(&npc).map_err(|e| e.to_string())?;

    let created_at = if let Some(old) = state.database.get_npc(&npc.id).await.map_err(|e| e.to_string())? {
        old.created_at
    } else {
        chrono::Utc::now().to_rfc3339()
    };

    let (campaign_id, location_id, voice_profile_id, quest_hooks) = if let Some(old) = state.database.get_npc(&npc.id).await.map_err(|e| e.to_string())? {
        (old.campaign_id, old.location_id, old.voice_profile_id, old.quest_hooks)
    } else {
        (None, None, None, None)
    };

    let record = crate::database::NpcRecord {
        id: npc.id.clone(),
        campaign_id,
        name: npc.name.clone(),
        role: role_str,
        personality_id: None,
        personality_json,
        data_json: Some(data_json),
        stats_json,
        notes: Some(npc.notes.clone()),
        location_id,
        voice_profile_id,
        quest_hooks,
        created_at,
    };

    state.database.save_npc(&record).await.map_err(|e| e.to_string())?;

    Ok(())
}

/// Delete an NPC from store and database
#[tauri::command]
pub async fn delete_npc(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.npc_store.delete(&id);
    state.database.delete_npc(&id).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Search NPCs by query string
#[tauri::command]
pub fn search_npcs(
    query: String,
    campaign_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<NPC>, String> {
    Ok(state.npc_store.search(&query, campaign_id.as_deref()))
}
