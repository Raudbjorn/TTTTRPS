//! Tauri Commands
//!
//! All Tauri IPC commands exposed to the frontend.

use tauri::State;
use std::sync::RwLock;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

// Core modules
use crate::core::llm::{LLMConfig, LLMClient, ChatMessage, ChatRequest, MessageRole};
use crate::core::campaign_manager::{
    CampaignManager, Campaign, SessionNote, SnapshotSummary
};
use crate::core::session_manager::{
    SessionManager, GameSession, SessionSummary, CombatState, Combatant,
    CombatantType, create_common_condition
};
use crate::core::character_gen::{CharacterGenerator, GenerationOptions, Character};
use crate::core::npc_gen::{NPCGenerator, NPCGenerationOptions, NPC, NPCStore};
use crate::core::credentials::CredentialManager;
use crate::core::voice::{VoiceClient, VoiceConfig, VoiceProvider, SynthesisRequest, OutputFormat};
use crate::core::audio::AudioVolumes;
use crate::ingestion::pdf_parser::PDFParser;

// ============================================================================
// Application State
// ============================================================================

pub struct AppState {
    pub llm_config: RwLock<Option<LLMConfig>>,
    pub campaign_manager: CampaignManager,
    pub session_manager: SessionManager,
    pub npc_store: NPCStore,
    pub credentials: CredentialManager,
    pub voice_client: RwLock<Option<VoiceClient>>,
    // Note: AudioPlayer is not stored in state because rodio's OutputStream
    // is not Send+Sync. Audio playback is handled via lazy initialization.
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            llm_config: RwLock::new(None),
            campaign_manager: CampaignManager::new(),
            session_manager: SessionManager::new(),
            npc_store: NPCStore::new(),
            credentials: CredentialManager::with_service("ttrpg-assistant"),
            voice_client: RwLock::new(None),
        }
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequestPayload {
    pub message: String,
    pub system_prompt: Option<String>,
    pub context: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponsePayload {
    pub content: String,
    pub model: String,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LLMSettings {
    pub provider: String,
    pub api_key: Option<String>,
    pub host: Option<String>,
    pub model: String,
    pub embedding_model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    pub provider: String,
    pub healthy: bool,
    pub message: String,
}

// ============================================================================
// LLM Commands
// ============================================================================

#[tauri::command]
pub fn configure_llm(
    settings: LLMSettings,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let config = match settings.provider.as_str() {
        "ollama" => LLMConfig::Ollama {
            host: settings.host.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model: settings.model,
            embedding_model: settings.embedding_model.or(Some("nomic-embed-text".to_string())),
        },
        "claude" => LLMConfig::Claude {
            api_key: settings.api_key.clone().ok_or("Claude requires an API key")?,
            model: settings.model,
            max_tokens: 4096,
        },
        "gemini" => LLMConfig::Gemini {
            api_key: settings.api_key.clone().ok_or("Gemini requires an API key")?,
            model: settings.model,
        },
        _ => return Err(format!("Unknown provider: {}", settings.provider)),
    };

    // Store API key securely if provided
    if let Some(api_key) = &settings.api_key {
        let key_name = format!("{}_api_key", settings.provider);
        let _ = state.credentials.store_secret(&key_name, api_key);
    }

    let client = LLMClient::new(config.clone());
    let provider_name = client.provider_name().to_string();

    *state.llm_config.write().unwrap() = Some(config);

    Ok(format!("Configured {} provider successfully", provider_name))
}

#[tauri::command]
pub async fn chat(
    payload: ChatRequestPayload,
    state: State<'_, AppState>,
) -> Result<ChatResponsePayload, String> {
    let config = state.llm_config.read().unwrap()
        .clone()
        .ok_or("LLM not configured. Please configure in Settings.")?;

    let client = LLMClient::new(config);

    let mut messages = vec![];

    // Add context messages if provided
    if let Some(context) = &payload.context {
        for ctx in context {
            messages.push(ChatMessage {
                role: MessageRole::User,
                content: ctx.clone(),
            });
        }
    }

    // Add the main message
    messages.push(ChatMessage {
        role: MessageRole::User,
        content: payload.message,
    });

    let request = ChatRequest {
        messages,
        system_prompt: Some(payload.system_prompt.unwrap_or_else(|| {
            "You are a helpful TTRPG Game Master assistant. Help the user with their tabletop RPG questions, \
             provide rules clarifications, generate content, and assist with running their campaign.".to_string()
        })),
        temperature: Some(0.7),
        max_tokens: Some(2048),
    };

    let response = client.chat(request).await.map_err(|e| e.to_string())?;

    Ok(ChatResponsePayload {
        content: response.content,
        model: response.model,
        input_tokens: response.usage.as_ref().map(|u| u.input_tokens),
        output_tokens: response.usage.as_ref().map(|u| u.output_tokens),
    })
}

#[tauri::command]
pub async fn check_llm_health(state: State<'_, AppState>) -> Result<HealthStatus, String> {
    let config_opt = state.llm_config.read().unwrap().clone();

    match config_opt {
        Some(config) => {
            let client = LLMClient::new(config);
            let provider = client.provider_name().to_string();

            match client.health_check().await {
                Ok(healthy) => Ok(HealthStatus {
                    provider: provider.clone(),
                    healthy,
                    message: if healthy {
                        format!("{} is available", provider)
                    } else {
                        format!("{} is not responding", provider)
                    },
                }),
                Err(e) => Ok(HealthStatus {
                    provider,
                    healthy: false,
                    message: e.to_string(),
                }),
            }
        }
        None => Ok(HealthStatus {
            provider: "none".to_string(),
            healthy: false,
            message: "No LLM configured".to_string(),
        }),
    }
}

#[tauri::command]
pub fn get_llm_config(state: State<'_, AppState>) -> Result<Option<LLMSettings>, String> {
    let config = state.llm_config.read().unwrap();

    Ok(config.as_ref().map(|c| match c {
        LLMConfig::Ollama { host, model, embedding_model } => LLMSettings {
            provider: "ollama".to_string(),
            api_key: None,
            host: Some(host.clone()),
            model: model.clone(),
            embedding_model: embedding_model.clone(),
        },
        LLMConfig::Claude { model, .. } => LLMSettings {
            provider: "claude".to_string(),
            api_key: Some("********".to_string()),
            host: None,
            model: model.clone(),
            embedding_model: None,
        },
        LLMConfig::Gemini { model, .. } => LLMSettings {
            provider: "gemini".to_string(),
            api_key: Some("********".to_string()),
            host: None,
            model: model.clone(),
            embedding_model: None,
        },
    }))
}

// ============================================================================
// Campaign Commands
// ============================================================================

#[tauri::command]
pub fn list_campaigns(state: State<'_, AppState>) -> Result<Vec<Campaign>, String> {
    Ok(state.campaign_manager.list_campaigns())
}

#[tauri::command]
pub fn create_campaign(
    name: String,
    system: String,
    state: State<'_, AppState>,
) -> Result<Campaign, String> {
    Ok(state.campaign_manager.create_campaign(&name, &system))
}

#[tauri::command]
pub fn get_campaign(id: String, state: State<'_, AppState>) -> Result<Option<Campaign>, String> {
    Ok(state.campaign_manager.get_campaign(&id))
}

#[tauri::command]
pub fn update_campaign(
    campaign: Campaign,
    auto_snapshot: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.campaign_manager.update_campaign(campaign, auto_snapshot)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_campaign(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.campaign_manager.delete_campaign(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_snapshot(
    campaign_id: String,
    description: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    state.campaign_manager.create_snapshot(&campaign_id, &description)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_snapshots(campaign_id: String, state: State<'_, AppState>) -> Result<Vec<SnapshotSummary>, String> {
    Ok(state.campaign_manager.list_snapshots(&campaign_id))
}

#[tauri::command]
pub fn restore_snapshot(
    campaign_id: String,
    snapshot_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.campaign_manager.restore_snapshot(&campaign_id, &snapshot_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_campaign(campaign_id: String, state: State<'_, AppState>) -> Result<String, String> {
    state.campaign_manager.export_to_json(&campaign_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_campaign(
    json: String,
    new_id: bool,
    state: State<'_, AppState>,
) -> Result<String, String> {
    state.campaign_manager.import_from_json(&json, new_id)
        .map_err(|e| e.to_string())
}

// ============================================================================
// Session Notes Commands
// ============================================================================

#[tauri::command]
pub fn add_campaign_note(
    campaign_id: String,
    content: String,
    tags: Vec<String>,
    session_number: Option<u32>,
    state: State<'_, AppState>,
) -> Result<SessionNote, String> {
    Ok(state.campaign_manager.add_note(&campaign_id, &content, tags, session_number))
}

#[tauri::command]
pub fn get_campaign_notes(campaign_id: String, state: State<'_, AppState>) -> Result<Vec<SessionNote>, String> {
    Ok(state.campaign_manager.get_notes(&campaign_id))
}

#[tauri::command]
pub fn search_campaign_notes(
    campaign_id: String,
    query: String,
    tags: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> Result<Vec<SessionNote>, String> {
    let tags_ref = tags.as_deref();
    Ok(state.campaign_manager.search_notes(&campaign_id, &query, tags_ref))
}

#[tauri::command]
pub fn delete_campaign_note(
    campaign_id: String,
    note_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.campaign_manager.delete_note(&campaign_id, &note_id)
        .map_err(|e| e.to_string())
}

// ============================================================================
// Session Commands
// ============================================================================

#[tauri::command]
pub fn start_session(
    campaign_id: String,
    session_number: u32,
    state: State<'_, AppState>,
) -> Result<GameSession, String> {
    Ok(state.session_manager.start_session(&campaign_id, session_number))
}

#[tauri::command]
pub fn get_session(session_id: String, state: State<'_, AppState>) -> Result<Option<GameSession>, String> {
    Ok(state.session_manager.get_session(&session_id))
}

#[tauri::command]
pub fn get_active_session(campaign_id: String, state: State<'_, AppState>) -> Result<Option<GameSession>, String> {
    Ok(state.session_manager.get_active_session(&campaign_id))
}

#[tauri::command]
pub fn list_sessions(campaign_id: String, state: State<'_, AppState>) -> Result<Vec<SessionSummary>, String> {
    Ok(state.session_manager.list_sessions(&campaign_id))
}

#[tauri::command]
pub fn end_session(session_id: String, state: State<'_, AppState>) -> Result<SessionSummary, String> {
    state.session_manager.end_session(&session_id)
        .map_err(|e| e.to_string())
}

// ============================================================================
// Combat Commands
// ============================================================================

#[tauri::command]
pub fn start_combat(session_id: String, state: State<'_, AppState>) -> Result<CombatState, String> {
    state.session_manager.start_combat(&session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn end_combat(session_id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.session_manager.end_combat(&session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_combat(session_id: String, state: State<'_, AppState>) -> Result<Option<CombatState>, String> {
    Ok(state.session_manager.get_combat(&session_id))
}

#[tauri::command]
pub fn add_combatant(
    session_id: String,
    name: String,
    initiative: i32,
    combatant_type: String,
    state: State<'_, AppState>,
) -> Result<Combatant, String> {
    let ctype = match combatant_type.as_str() {
        "player" => CombatantType::Player,
        "npc" => CombatantType::NPC,
        "monster" => CombatantType::Monster,
        "ally" => CombatantType::Ally,
        _ => CombatantType::Monster,
    };

    state.session_manager.add_combatant_quick(&session_id, &name, initiative, ctype)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_combatant(
    session_id: String,
    combatant_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.session_manager.remove_combatant(&session_id, &combatant_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn next_turn(session_id: String, state: State<'_, AppState>) -> Result<Option<Combatant>, String> {
    state.session_manager.next_turn(&session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_current_combatant(session_id: String, state: State<'_, AppState>) -> Result<Option<Combatant>, String> {
    Ok(state.session_manager.get_current_combatant(&session_id))
}

#[tauri::command]
pub fn damage_combatant(
    session_id: String,
    combatant_id: String,
    amount: i32,
    state: State<'_, AppState>,
) -> Result<i32, String> {
    state.session_manager.damage_combatant(&session_id, &combatant_id, amount)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn heal_combatant(
    session_id: String,
    combatant_id: String,
    amount: i32,
    state: State<'_, AppState>,
) -> Result<i32, String> {
    state.session_manager.heal_combatant(&session_id, &combatant_id, amount)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_condition(
    session_id: String,
    combatant_id: String,
    condition_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let condition = create_common_condition(&condition_name)
        .ok_or_else(|| format!("Unknown condition: {}", condition_name))?;

    state.session_manager.add_condition(&session_id, &combatant_id, condition)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_condition(
    session_id: String,
    combatant_id: String,
    condition_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.session_manager.remove_condition(&session_id, &combatant_id, &condition_name)
        .map_err(|e| e.to_string())
}

// ============================================================================
// Character Generation Commands
// ============================================================================

#[tauri::command]
pub fn generate_character(options: GenerationOptions) -> Result<Character, String> {
    CharacterGenerator::generate(&options)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_supported_systems() -> Vec<String> {
    vec![
        "D&D 5e".to_string(),
        "Pathfinder 2e".to_string(),
        "Call of Cthulhu".to_string(),
        "Cyberpunk".to_string(),
        "Shadowrun".to_string(),
        "Fate Core".to_string(),
        "World of Darkness".to_string(),
    ]
}

// ============================================================================
// NPC Commands
// ============================================================================

#[tauri::command]
pub fn generate_npc(
    options: NPCGenerationOptions,
    campaign_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<NPC, String> {
    let generator = NPCGenerator::new();
    let npc = generator.generate_quick(&options);

    state.npc_store.add(npc.clone(), campaign_id.as_deref());

    Ok(npc)
}

#[tauri::command]
pub fn get_npc(id: String, state: State<'_, AppState>) -> Result<Option<NPC>, String> {
    Ok(state.npc_store.get(&id))
}

#[tauri::command]
pub fn list_npcs(campaign_id: Option<String>, state: State<'_, AppState>) -> Result<Vec<NPC>, String> {
    Ok(state.npc_store.list(campaign_id.as_deref()))
}

#[tauri::command]
pub fn update_npc(npc: NPC, state: State<'_, AppState>) -> Result<(), String> {
    state.npc_store.update(npc);
    Ok(())
}

#[tauri::command]
pub fn delete_npc(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.npc_store.delete(&id);
    Ok(())
}

#[tauri::command]
pub fn search_npcs(
    query: String,
    campaign_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<NPC>, String> {
    Ok(state.npc_store.search(&query, campaign_id.as_deref()))
}

// ============================================================================
// Document Ingestion Commands
// ============================================================================

#[tauri::command]
pub fn ingest_pdf(path: String) -> Result<IngestResult, String> {
    let path = std::path::Path::new(&path);

    let pages = PDFParser::extract_text_with_pages(path)
        .map_err(|e| e.to_string())?;

    let total_chars: usize = pages.iter().map(|(_, text)| text.len()).sum();

    Ok(IngestResult {
        page_count: pages.len(),
        character_count: total_chars,
        source_name: path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IngestResult {
    pub page_count: usize,
    pub character_count: usize,
    pub source_name: String,
}

// ============================================================================
// Voice Synthesis Commands
// ============================================================================

#[tauri::command]
pub fn configure_voice(
    provider: String,
    api_key: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let config = match provider.as_str() {
        "elevenlabs" => VoiceConfig {
            provider: VoiceProvider::ElevenLabs {
                api_key: api_key.ok_or("ElevenLabs requires an API key")?,
                model_id: None,
            },
            cache_dir: Some(PathBuf::from("./voice_cache")),
            default_voice_id: None,
        },
        "disabled" => VoiceConfig::default(),
        _ => return Err(format!("Unknown voice provider: {}", provider)),
    };

    let client = VoiceClient::new(config);
    *state.voice_client.write().unwrap() = Some(client);

    Ok(format!("Voice provider configured: {}", provider))
}

#[tauri::command]
pub async fn synthesize_voice(
    text: String,
    voice_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Clone the client to avoid holding the lock across await
    let client = {
        let guard = state.voice_client.read().unwrap();
        guard.as_ref().ok_or("Voice not configured")?.clone()
    };

    let request = SynthesisRequest {
        text,
        voice_id,
        settings: None,
        output_format: OutputFormat::Mp3,
    };

    let result = client.synthesize(request).await
        .map_err(|e| e.to_string())?;

    Ok(result.audio_path.to_string_lossy().to_string())
}

// ============================================================================
// Audio Playback Commands
// ============================================================================

// Note: Audio playback uses rodio which requires the OutputStream to stay
// on the same thread. For Tauri, we handle this by creating the audio player
// on-demand in the main thread context.

#[tauri::command]
pub fn get_audio_volumes() -> AudioVolumes {
    AudioVolumes::default()
}

#[tauri::command]
pub fn get_sfx_categories() -> Vec<String> {
    crate::core::audio::get_sfx_categories()
}

// ============================================================================
// Credential Commands
// ============================================================================

#[tauri::command]
pub fn save_api_key(
    provider: String,
    api_key: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let key_name = format!("{}_api_key", provider);
    state.credentials.store_secret(&key_name, &api_key)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_api_key(provider: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    let key_name = format!("{}_api_key", provider);
    match state.credentials.get_secret(&key_name) {
        Ok(key) => Ok(Some(key)),
        Err(crate::core::credentials::CredentialError::NotFound(_)) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn delete_api_key(provider: String, state: State<'_, AppState>) -> Result<(), String> {
    let key_name = format!("{}_api_key", provider);
    state.credentials.delete_secret(&key_name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_stored_providers(state: State<'_, AppState>) -> Vec<String> {
    state.credentials.list_llm_providers()
}

// ============================================================================
// Utility Commands
// ============================================================================

#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub fn get_system_info() -> SystemInfo {
    SystemInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub version: String,
}

// ============================================================================
// Voice Preset Commands
// ============================================================================

#[tauri::command]
pub fn get_voice_presets() -> Vec<crate::core::voice::NPCVoicePreset> {
    crate::core::voice::get_voice_presets()
}
