//! Voice Profile Commands
//!
//! Commands for managing voice profiles and linking them to NPCs.

use tauri::State;

use crate::core::voice::{
    VoiceProfile, VoiceProviderType, ProfileMetadata,
    Gender, AgeRange, get_dm_presets,
};
use crate::commands::AppState;
use crate::database::NpcOps;

// ============================================================================
// Voice Profile Commands
// ============================================================================

/// Create a new voice profile
#[tauri::command]
pub async fn create_voice_profile(
    name: String,
    provider: String,
    voice_id: String,
    metadata: Option<ProfileMetadata>,
    _state: State<'_, AppState>,
) -> Result<String, String> {
    let provider_type = match provider.as_str() {
        "elevenlabs" => VoiceProviderType::ElevenLabs,
        "openai" => VoiceProviderType::OpenAI,
        "fish_audio" => VoiceProviderType::FishAudio,
        "piper" => VoiceProviderType::Piper,
        "ollama" => VoiceProviderType::Ollama,
        "chatterbox" => VoiceProviderType::Chatterbox,
        "gpt_sovits" => VoiceProviderType::GptSoVits,
        "xtts_v2" => VoiceProviderType::XttsV2,
        "fish_speech" => VoiceProviderType::FishSpeech,
        "dia" => VoiceProviderType::Dia,
        _ => return Err(format!("Unknown provider: {}", provider)),
    };

    let mut profile = VoiceProfile::new(&name, provider_type, &voice_id);
    if let Some(meta) = metadata {
        profile = profile.with_metadata(meta);
    }

    Ok(profile.id)
}

/// Link a voice profile to an NPC
#[tauri::command]
pub async fn link_voice_profile_to_npc(
    profile_id: String,
    npc_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Some(mut record) = state.database.get_npc(&npc_id).await.map_err(|e| e.to_string())? {
        // Update the structured voice_profile_id field (source of truth)
        record.voice_profile_id = Some(profile_id.clone());

        // Also update data_json for backwards compatibility with legacy code that reads from JSON
        // TODO: Remove this once all consumers migrate to using the structured field
        if let Some(json) = &record.data_json {
            let mut npc: serde_json::Value = serde_json::from_str(json)
                .map_err(|e| e.to_string())?;
            npc["voice_profile_id"] = serde_json::json!(profile_id);
            record.data_json = Some(serde_json::to_string(&npc).map_err(|e| e.to_string())?);
        }

        state.database.save_npc(&record).await.map_err(|e| e.to_string())?;
    } else {
        return Err(format!("NPC not found: {}", npc_id));
    }
    Ok(())
}

/// Get the voice profile linked to an NPC
#[tauri::command]
pub async fn get_npc_voice_profile(
    npc_id: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    if let Some(record) = state.database.get_npc(&npc_id).await.map_err(|e| e.to_string())? {
        if let Some(json) = &record.data_json {
            let npc: serde_json::Value = serde_json::from_str(json)
                .map_err(|e| e.to_string())?;
            if let Some(profile_id) = npc.get("voice_profile_id").and_then(|v| v.as_str()) {
                return Ok(Some(profile_id.to_string()));
            }
        }
    }
    Ok(None)
}

/// Search voice profiles by query
#[tauri::command]
pub fn search_voice_profiles(query: String) -> Vec<VoiceProfile> {
    let query_lower = query.to_lowercase();
    get_dm_presets()
        .into_iter()
        .filter(|p| {
            p.name.to_lowercase().contains(&query_lower)
                || p.metadata.personality_traits.iter().any(|t| t.to_lowercase().contains(&query_lower))
                || p.metadata.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
                || p.metadata.description.as_ref().map_or(false, |d| d.to_lowercase().contains(&query_lower))
        })
        .collect()
}

/// Get voice profiles by gender
#[tauri::command]
pub fn get_voice_profiles_by_gender(gender: String) -> Vec<VoiceProfile> {
    let target_gender = match gender.to_lowercase().as_str() {
        "male" => Gender::Male,
        "female" => Gender::Female,
        "neutral" => Gender::Neutral,
        "nonbinary" | "non-binary" => Gender::NonBinary,
        _ => return Vec::new(),
    };
    get_dm_presets()
        .into_iter()
        .filter(|p| p.metadata.gender == target_gender)
        .collect()
}

/// Get voice profiles by age range
#[tauri::command]
pub fn get_voice_profiles_by_age(age_range: String) -> Vec<VoiceProfile> {
    let target_age = match age_range.to_lowercase().as_str() {
        "child" => AgeRange::Child,
        "young_adult" | "youngadult" => AgeRange::YoungAdult,
        "adult" => AgeRange::Adult,
        "middle_aged" | "middleaged" => AgeRange::MiddleAged,
        "elderly" => AgeRange::Elderly,
        _ => return Vec::new(),
    };
    get_dm_presets()
        .into_iter()
        .filter(|p| p.metadata.age_range == target_age)
        .collect()
}
