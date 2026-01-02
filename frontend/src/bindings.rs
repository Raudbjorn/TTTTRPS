//! Tauri IPC Bindings
//!
//! Wrapper functions for calling Tauri commands from the frontend.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// ============================================================================
// Tauri Invoke
// ============================================================================

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = "invoke")]
    async fn invoke_raw(cmd: &str, args: JsValue) -> JsValue;

    // Dialog plugin - file picker
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "dialog"], js_name = "open")]
    async fn dialog_open(options: JsValue) -> JsValue;

    // Event listener - for progress events
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = "listen")]
    fn event_listen(event: &str, handler: &js_sys::Function) -> JsValue;
}

/// Listen for Tauri events (returns unlisten function)
pub fn listen_event<F>(event_name: &str, callback: F) -> JsValue
where
    F: Fn(JsValue) + 'static,
{
    let closure = wasm_bindgen::closure::Closure::wrap(Box::new(callback) as Box<dyn Fn(JsValue)>);
    let result = event_listen(event_name, closure.as_ref().unchecked_ref());
    closure.forget(); // Prevent closure from being dropped
    result
}

/// Invoke a Tauri command with typed arguments and response
pub async fn invoke<A: Serialize, R: for<'de> Deserialize<'de>>(
    cmd: &str,
    args: &A,
) -> Result<R, String> {
    let args_js = serde_wasm_bindgen::to_value(args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = invoke_raw(cmd, args_js).await;

    // Check if result is an error
    if result.is_undefined() || result.is_null() {
        return Err("Command returned null/undefined".to_string());
    }

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

/// Invoke a Tauri command with no arguments
pub async fn invoke_no_args<R: for<'de> Deserialize<'de>>(cmd: &str) -> Result<R, String> {
    #[derive(Serialize)]
    struct Empty {}
    invoke(cmd, &Empty {}).await
}

// ============================================================================
// File Dialog
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct FileFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenDialogOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<FileFilter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple: Option<bool>,
}

/// Open a file picker dialog
/// Returns the selected file path(s) or None if cancelled
pub async fn open_file_dialog(options: OpenDialogOptions) -> Option<String> {
    let options_js = serde_wasm_bindgen::to_value(&options).ok()?;
    let result = dialog_open(options_js).await;

    if result.is_null() || result.is_undefined() {
        return None;
    }

    // Result can be a string (single file) or array (multiple files)
    // For single file mode, it returns the path directly
    serde_wasm_bindgen::from_value(result).ok()
}

/// Open a file picker for PDF documents
pub async fn pick_pdf_file() -> Option<String> {
    open_file_dialog(OpenDialogOptions {
        title: Some("Select PDF Document".to_string()),
        filters: Some(vec![
            FileFilter {
                name: "PDF Documents".to_string(),
                extensions: vec!["pdf".to_string()],
            },
            FileFilter {
                name: "All Files".to_string(),
                extensions: vec!["*".to_string()],
            },
        ]),
        default_path: None,
        directory: Some(false),
        multiple: Some(false),
    }).await
}

/// Open a file picker for any supported document type
pub async fn pick_document_file() -> Option<String> {
    open_file_dialog(OpenDialogOptions {
        title: Some("Select Document".to_string()),
        filters: Some(vec![
            FileFilter {
                name: "All Supported".to_string(),
                extensions: vec![
                    "pdf".to_string(),
                    "epub".to_string(),
                    "mobi".to_string(),
                    "azw".to_string(),
                    "azw3".to_string(),
                    "docx".to_string(),
                    "txt".to_string(),
                    "md".to_string(),
                    "markdown".to_string(),
                ],
            },
            FileFilter {
                name: "PDF".to_string(),
                extensions: vec!["pdf".to_string()],
            },
            FileFilter {
                name: "EPUB".to_string(),
                extensions: vec!["epub".to_string()],
            },
            FileFilter {
                name: "MOBI/AZW".to_string(),
                extensions: vec!["mobi".to_string(), "azw".to_string(), "azw3".to_string()],
            },
            FileFilter {
                name: "DOCX".to_string(),
                extensions: vec!["docx".to_string()],
            },
            FileFilter {
                name: "Text/Markdown".to_string(),
                extensions: vec!["txt".to_string(), "md".to_string(), "markdown".to_string()],
            },
            FileFilter {
                name: "All Files".to_string(),
                extensions: vec!["*".to_string()],
            },
        ]),
        default_path: None,
        directory: Some(false),
        multiple: Some(false),
    }).await
}

// ============================================================================
// Request/Response Types (match backend commands.rs)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequestPayload {
    pub message: String,
    pub system_prompt: Option<String>,
    pub context: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponsePayload {
    pub content: String,
    pub model: String,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMSettings {
    pub provider: String,
    pub api_key: Option<String>,
    pub host: Option<String>,
    pub model: String,
    pub embedding_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub provider: String,
    pub healthy: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    pub page_count: usize,
    pub character_count: usize,
    pub source_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub provider: String, // Enum locally handled as string
    pub cache_dir: Option<String>,
    pub default_voice_id: Option<String>,
    pub elevenlabs: Option<ElevenLabsConfig>,
    pub fish_audio: Option<FishAudioConfig>,
    pub ollama: Option<OllamaConfig>,
    pub openai: Option<OpenAIVoiceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElevenLabsConfig {
    pub api_key: String,
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FishAudioConfig {
    pub api_key: String,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIVoiceConfig {
    pub api_key: String,
    pub model: String,
    pub voice: String,
}

/// Voice information from a TTS provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Voice {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub description: Option<String>,
    pub preview_url: Option<String>,
    pub labels: Vec<String>,
}

// ============================================================================
// Voice Commands
// ============================================================================

/// List available OpenAI TTS voices (static list, no API key needed)
pub async fn list_openai_voices() -> Result<Vec<Voice>, String> {
    invoke_no_args("list_openai_voices").await
}

/// List available OpenAI TTS models
pub async fn list_openai_tts_models() -> Result<Vec<(String, String)>, String> {
    invoke_no_args("list_openai_tts_models").await
}

/// List available ElevenLabs voices (requires API key)
pub async fn list_elevenlabs_voices(api_key: String) -> Result<Vec<Voice>, String> {
    #[derive(Serialize)]
    struct Args {
        api_key: String,
    }
    invoke("list_elevenlabs_voices", &Args { api_key }).await
}

/// List voices from the currently configured voice provider
pub async fn list_available_voices() -> Result<Vec<Voice>, String> {
    invoke_no_args("list_available_voices").await
}

// ============================================================================
// LLM Commands
// ============================================================================

pub async fn configure_llm(settings: LLMSettings) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        settings: LLMSettings,
    }
    invoke("configure_llm", &Args { settings }).await
}

pub async fn chat(payload: ChatRequestPayload) -> Result<ChatResponsePayload, String> {
    #[derive(Serialize)]
    struct Args {
        payload: ChatRequestPayload,
    }
    invoke("chat", &Args { payload }).await
}

pub async fn check_llm_health() -> Result<HealthStatus, String> {
    invoke_no_args("check_llm_health").await
}

pub async fn get_llm_config() -> Result<Option<LLMSettings>, String> {
    invoke_no_args("get_llm_config").await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: Option<String>,
    pub parameter_size: Option<String>,
}

pub async fn list_ollama_models(host: String) -> Result<Vec<OllamaModel>, String> {
    #[derive(Serialize)]
    struct Args {
        host: String,
    }
    invoke("list_ollama_models", &Args { host }).await
}

/// Generic model info for cloud providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

pub async fn list_claude_models(api_key: Option<String>) -> Result<Vec<ModelInfo>, String> {
    #[derive(Serialize)]
    struct Args {
        api_key: Option<String>,
    }
    invoke("list_claude_models", &Args { api_key }).await
}

pub async fn list_openai_models(api_key: Option<String>) -> Result<Vec<ModelInfo>, String> {
    #[derive(Serialize)]
    struct Args {
        api_key: Option<String>,
    }
    invoke("list_openai_models", &Args { api_key }).await
}

pub async fn list_gemini_models(api_key: Option<String>) -> Result<Vec<ModelInfo>, String> {
    #[derive(Serialize)]
    struct Args {
        api_key: Option<String>,
    }
    invoke("list_gemini_models", &Args { api_key }).await
}

/// List OpenRouter models (no auth required - uses public API)
pub async fn list_openrouter_models() -> Result<Vec<ModelInfo>, String> {
    invoke_no_args("list_openrouter_models").await
}

/// List models for any provider via LiteLLM catalog (no auth required)
pub async fn list_provider_models(provider: String) -> Result<Vec<ModelInfo>, String> {
    #[derive(Serialize)]
    struct Args {
        provider: String,
    }
    invoke("list_provider_models", &Args { provider }).await
}

pub async fn configure_voice(config: VoiceConfig) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        config: VoiceConfig,
    }
    invoke("configure_voice", &Args { config }).await
}

pub async fn get_voice_config() -> Result<VoiceConfig, String> {
    invoke_no_args("get_voice_config").await
}

pub async fn get_vector_store_status() -> Result<String, String> {
    invoke("get_vector_store_status", &()).await
}

pub async fn speak(text: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        text: String,
    }
    invoke("speak", &Args { text }).await
}

// ============================================================================
// Credential Commands
// ============================================================================

pub async fn save_api_key(provider: String, api_key: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        provider: String,
        api_key: String,
    }
    invoke("save_api_key", &Args { provider, api_key }).await
}

pub async fn get_api_key(provider: String) -> Result<Option<String>, String> {
    #[derive(Serialize)]
    struct Args {
        provider: String,
    }
    invoke("get_api_key", &Args { provider }).await
}

pub async fn list_stored_providers() -> Result<Vec<String>, String> {
    invoke_no_args("list_stored_providers").await
}

// ============================================================================
// Document Commands
// ============================================================================

/// Parse PDF and return stats (does NOT index)
pub async fn ingest_pdf(path: String) -> Result<IngestResult, String> {
    #[derive(Serialize)]
    struct Args {
        path: String,
    }
    invoke("ingest_pdf", &Args { path }).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestOptions {
    pub source_type: String,
    pub campaign_id: Option<String>,
}

/// Ingest document into Meilisearch (indexes the content)
pub async fn ingest_document(path: String, options: Option<IngestOptions>) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        path: String,
        options: Option<IngestOptions>,
    }
    invoke("ingest_document", &Args { path, options }).await
}

/// Progress event from document ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestProgress {
    pub stage: String,
    pub progress: f32,
    pub message: String,
    pub source_name: String,
}

/// Ingest document with progress reporting via events
pub async fn ingest_document_with_progress(path: String, source_type: Option<String>) -> Result<IngestResult, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        path: String,
        source_type: Option<String>,
    }
    invoke("ingest_document_with_progress", &Args { path, source_type }).await
}

// ============================================================================
// Campaign Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    pub id: String,
    pub name: String,
    pub system: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub settings: CampaignSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CampaignStats {
    pub session_count: usize,
    pub npc_count: usize,
    pub total_playtime_minutes: i64,
    pub last_played: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CampaignSettings {
    pub theme: String,
    #[serde(default)]
    pub theme_weights: ThemeWeights,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotSummary {
    pub id: String,
    pub description: String,
    pub created_at: String,
    pub snapshot_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeWeights {
    pub fantasy: f32,
    pub cosmic: f32,
    pub terminal: f32,
    pub noir: f32,
    pub neon: f32,
}

impl Default for ThemeWeights {
    fn default() -> Self {
        Self {
            fantasy: 1.0,
            cosmic: 0.0,
            terminal: 0.0,
            noir: 0.0,
            neon: 0.0,
        }
    }
}

// ============================================================================
// Campaign Commands
// ============================================================================

pub async fn list_campaigns() -> Result<Vec<Campaign>, String> {
    invoke_no_args("list_campaigns").await
}

pub async fn create_campaign(name: String, system: String) -> Result<Campaign, String> {
    #[derive(Serialize)]
    struct Args {
        name: String,
        system: String,
    }
    invoke("create_campaign", &Args { name, system }).await
}

pub async fn get_campaign(id: String) -> Result<Option<Campaign>, String> {
    #[derive(Serialize)]
    struct Args {
        id: String,
    }
    invoke("get_campaign", &Args { id }).await
}

pub async fn delete_campaign(id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        id: String,
    }
    invoke("delete_campaign", &Args { id }).await
}

pub async fn get_campaign_theme(campaign_id: String) -> Result<ThemeWeights, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("get_campaign_theme", &Args { campaign_id }).await
}

pub async fn set_campaign_theme(campaign_id: String, weights: ThemeWeights) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        weights: ThemeWeights,
    }
    invoke("set_campaign_theme", &Args { campaign_id, weights }).await
}

pub async fn get_theme_preset(system: String) -> Result<ThemeWeights, String> {
    #[derive(Serialize)]
    struct Args {
        system: String,
    }
    invoke("get_theme_preset", &Args { system }).await
}

pub async fn list_snapshots(campaign_id: String) -> Result<Vec<SnapshotSummary>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("list_snapshots", &Args { campaign_id }).await
}

pub async fn create_snapshot(campaign_id: String, description: String) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        description: String,
    }
    invoke("create_snapshot", &Args { campaign_id, description }).await
}

pub async fn restore_snapshot(campaign_id: String, snapshot_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        snapshot_id: String,
    }
    invoke("restore_snapshot", &Args { campaign_id, snapshot_id }).await
}

pub async fn get_campaign_stats(campaign_id: String) -> Result<CampaignStats, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("get_campaign_stats", &Args { campaign_id }).await
}

pub async fn generate_campaign_cover(campaign_id: String, title: String) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        title: String,
    }
    invoke("generate_campaign_cover", &Args { campaign_id, title }).await
}

// ============================================================================
// Session Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameSession {
    pub id: String,
    pub campaign_id: String,
    pub session_number: u32,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub campaign_id: String,
    pub session_number: u32,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_minutes: Option<i64>,
    pub status: String,
    pub note_count: usize,
    pub had_combat: bool,
    pub order_index: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatState {
    pub id: String,
    pub round: u32,
    pub current_turn: usize,
    pub combatants: Vec<Combatant>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Combatant {
    pub id: String,
    pub name: String,
    pub initiative: i32,
    pub hp_current: i32,
    pub hp_max: i32,
    pub combatant_type: String,
    pub conditions: Vec<String>,
    pub is_active: bool,
}

// ============================================================================
// Session Commands
// ============================================================================

pub async fn start_session(campaign_id: String, session_number: u32) -> Result<GameSession, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        session_number: u32,
    }
    invoke("start_session", &Args { campaign_id, session_number }).await
}

pub async fn get_session(session_id: String) -> Result<Option<GameSession>, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke("get_session", &Args { session_id }).await
}

pub async fn get_active_session(campaign_id: String) -> Result<Option<GameSession>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("get_active_session", &Args { campaign_id }).await
}

pub async fn list_sessions(campaign_id: String) -> Result<Vec<SessionSummary>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("list_sessions", &Args { campaign_id }).await
}

pub async fn end_session(session_id: String) -> Result<SessionSummary, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke("end_session", &Args { session_id }).await
}

pub async fn reorder_session(session_id: String, new_order: i32) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        new_order: i32,
    }
    invoke("reorder_session", &Args { session_id, new_order }).await
}

// ============================================================================
// Combat Commands
// ============================================================================

pub async fn start_combat(session_id: String) -> Result<CombatState, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke("start_combat", &Args { session_id }).await
}

pub async fn end_combat(session_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke("end_combat", &Args { session_id }).await
}

pub async fn get_combat(session_id: String) -> Result<Option<CombatState>, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke("get_combat", &Args { session_id }).await
}

pub async fn add_combatant(
    session_id: String,
    name: String,
    initiative: i32,
    combatant_type: String,
) -> Result<Combatant, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        name: String,
        initiative: i32,
        combatant_type: String,
    }
    invoke("add_combatant", &Args { session_id, name, initiative, combatant_type }).await
}

pub async fn remove_combatant(session_id: String, combatant_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        combatant_id: String,
    }
    invoke("remove_combatant", &Args { session_id, combatant_id }).await
}

pub async fn next_turn(session_id: String) -> Result<Option<Combatant>, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke("next_turn", &Args { session_id }).await
}

pub async fn damage_combatant(session_id: String, combatant_id: String, amount: i32) -> Result<i32, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        combatant_id: String,
        amount: i32,
    }
    invoke("damage_combatant", &Args { session_id, combatant_id, amount }).await
}

pub async fn heal_combatant(session_id: String, combatant_id: String, amount: i32) -> Result<i32, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        combatant_id: String,
        amount: i32,
    }
    invoke("heal_combatant", &Args { session_id, combatant_id, amount }).await
}

pub async fn add_condition(session_id: String, combatant_id: String, condition_name: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        combatant_id: String,
        condition_name: String,
    }
    invoke("add_condition", &Args { session_id, combatant_id, condition_name }).await
}

pub async fn remove_condition(session_id: String, combatant_id: String, condition_name: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        combatant_id: String,
        condition_name: String,
    }
    invoke("remove_condition", &Args { session_id, combatant_id, condition_name }).await
}

// ============================================================================
// Character Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub system: String,
    pub character_type: String,
    pub level: Option<u32>,
    pub attributes: Vec<AttributeValue>,
    pub skills: Vec<String>,
    pub backstory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeValue {
    pub name: String,
    pub value: i32,
    pub modifier: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationOptions {
    pub system: String,
    pub character_type: Option<String>,
    pub level: Option<u32>,
    pub name: Option<String>,
    pub include_backstory: bool,
}

// ============================================================================
// Character Commands
// ============================================================================

pub async fn generate_character(options: GenerationOptions) -> Result<Character, String> {
    #[derive(Serialize)]
    struct Args {
        options: GenerationOptions,
    }
    invoke("generate_character", &Args { options }).await
}

pub async fn get_supported_systems() -> Result<Vec<String>, String> {
    invoke_no_args("get_supported_systems").await
}

// ============================================================================
// Utility Commands
// ============================================================================

pub async fn get_app_version() -> Result<String, String> {
    invoke_no_args("get_app_version").await
}

pub async fn get_system_info() -> Result<SystemInfo, String> {
    invoke_no_args("get_system_info").await
}

// ============================================================================
// Meilisearch Types and Commands
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeilisearchStatus {
    pub healthy: bool,
    pub host: String,
    pub document_counts: Option<std::collections::HashMap<String, u64>>,
}

pub async fn check_meilisearch_health() -> Result<MeilisearchStatus, String> {
    invoke_no_args("check_meilisearch_health").await
}

pub async fn reindex_library(index_name: Option<String>) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        index_name: Option<String>,
    }
    invoke("reindex_library", &Args { index_name }).await
}

// ============================================================================
// Search Types and Commands
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    pub limit: usize,
    pub source_type: Option<String>,
    pub campaign_id: Option<String>,
    pub index: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultPayload {
    pub content: String,
    pub source: String,
    pub source_type: String,
    pub page_number: Option<u32>,
    pub score: f32,
    pub index: String,
}

pub async fn search(query: String, options: Option<SearchOptions>) -> Result<Vec<SearchResultPayload>, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
        options: Option<SearchOptions>,
    }
    invoke("search", &Args { query, options }).await
}

pub async fn semantic_search(query: String, limit: usize) -> Result<Vec<SearchResultPayload>, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
        limit: usize,
    }
    invoke("semantic_search", &Args { query, limit }).await
}

pub async fn keyword_search(query: String, limit: usize) -> Result<Vec<SearchResultPayload>, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
        limit: usize,
    }
    invoke("keyword_search", &Args { query, limit }).await
}

// ============================================================================
// Usage Tracking Types and Commands
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageStats {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_requests: u32,
    pub estimated_cost_usd: f64,
    pub provider_breakdown: std::collections::HashMap<String, ProviderUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub requests: u32,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUsage {
    pub session_input_tokens: u64,
    pub session_output_tokens: u64,
    pub session_requests: u32,
    pub session_cost_usd: f64,
}

pub async fn get_usage_stats() -> Result<UsageStats, String> {
    invoke_no_args("get_usage_stats").await
}

pub async fn get_session_usage() -> Result<SessionUsage, String> {
    invoke_no_args("get_session_usage").await
}

pub async fn reset_usage_stats() -> Result<(), String> {
    invoke_no_args("reset_usage_stats").await
}

// ============================================================================
// NPC Conversation Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcConversation {
    pub id: String,
    pub npc_id: String,
    pub campaign_id: String,
    pub messages_json: String,
    pub unread_count: u32,
    pub last_message_at: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub parent_message_id: Option<String>,
    pub created_at: String,
}

// ============================================================================
// NPC Conversation Commands
// ============================================================================

pub async fn list_npc_conversations(campaign_id: String) -> Result<Vec<NpcConversation>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("list_npc_conversations", &Args { campaign_id }).await
}

pub async fn get_npc_conversation(npc_id: String) -> Result<NpcConversation, String> {
    #[derive(Serialize)]
    struct Args {
        npc_id: String,
    }
    invoke("get_npc_conversation", &Args { npc_id }).await
}

pub async fn add_npc_message(npc_id: String, content: String, role: String, parent_id: Option<String>) -> Result<ConversationMessage, String> {
    #[derive(Serialize)]
    struct Args {
        npc_id: String,
        content: String,
        role: String,
        parent_id: Option<String>,
    }
    invoke("add_npc_message", &Args { npc_id, content, role, parent_id }).await
}

pub async fn mark_npc_read(npc_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        npc_id: String,
    }
    invoke("mark_npc_read", &Args { npc_id }).await
}

// ============================================================================
// NPC Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NPC {
    pub id: String,
    pub name: String,
    pub role: String, // Stringified enum
    pub appearance: AppearanceDescription,
    pub personality: NPCPersonality,
    pub voice: VoiceDescription,
    pub stats: Option<Character>,
    pub relationships: Vec<NPCRelationship>,
    pub secrets: Vec<String>,
    pub hooks: Vec<PlotHook>,
    pub notes: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceDescription {
    pub age: String,
    pub height: String,
    pub build: String,
    pub hair: String,
    pub eyes: String,
    pub skin: String,
    pub distinguishing_features: Vec<String>,
    pub clothing: String,
    pub demeanor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NPCPersonality {
    pub traits: Vec<String>,
    pub ideals: Vec<String>,
    pub bonds: Vec<String>,
    pub flaws: Vec<String>,
    pub mannerisms: Vec<String>,
    pub speech_patterns: Vec<String>,
    pub motivations: Vec<String>,
    pub fears: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceDescription {
    pub pitch: String,
    pub pace: String,
    pub accent: Option<String>,
    pub vocabulary: String,
    pub sample_phrases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NPCRelationship {
    pub target_id: Option<String>,
    pub target_name: String,
    pub relationship_type: String,
    pub disposition: i32,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotHook {
    pub description: String,
    pub hook_type: String, // Enum stringified
    pub urgency: String, // Enum stringified
    pub reward_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NPCGenerationOptions {
    pub system: Option<String>,
    pub name: Option<String>,
    pub role: Option<String>,
    pub race: Option<String>,
    pub occupation: Option<String>,
    pub location: Option<String>,
    pub theme: Option<String>,
    pub generate_stats: bool,
    pub generate_backstory: bool,
    pub personality_depth: String,
    pub include_hooks: bool,
    pub include_secrets: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NpcSummary {
    pub id: String,
    pub name: String,
    pub role: String,
    pub avatar_url: String,
    pub status: String,
    pub last_message: String,
    pub unread_count: u32,
    pub last_active: String,
}

// ============================================================================
// NPC Commands
// ============================================================================

pub async fn generate_npc(options: NPCGenerationOptions, campaign_id: Option<String>) -> Result<NPC, String> {
    #[derive(Serialize)]
    struct Args {
        options: NPCGenerationOptions,
        campaign_id: Option<String>,
    }
    invoke("generate_npc", &Args { options, campaign_id }).await
}

pub async fn get_npc(id: String) -> Result<Option<NPC>, String> {
    #[derive(Serialize)]
    struct Args {
        id: String,
    }
    invoke("get_npc", &Args { id }).await
}

pub async fn list_npcs(campaign_id: Option<String>) -> Result<Vec<NPC>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: Option<String>,
    }
    invoke("list_npcs", &Args { campaign_id }).await
}

pub async fn update_npc(npc: NPC) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        npc: NPC,
    }
    invoke("update_npc", &Args { npc }).await
}

pub async fn delete_npc(id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        id: String,
    }
    invoke("delete_npc", &Args { id }).await
}

pub async fn list_npc_summaries(campaign_id: String) -> Result<Vec<NpcSummary>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("list_npc_summaries", &Args { campaign_id }).await
}

pub async fn reply_as_npc(npc_id: String) -> Result<ConversationMessage, String> {
    #[derive(Serialize)]
    struct Args {
        npc_id: String,
    }
    invoke("reply_as_npc", &Args { npc_id }).await
}

// ============================================================================
// Voice Queue Types and Commands
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VoiceStatus {
    Pending,
    Processing,
    Playing,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedVoice {
    pub id: String,
    pub text: String,
    pub voice_id: String,
    pub status: VoiceStatus,
    pub created_at: String,
}

pub async fn queue_voice(text: String, voice_id: Option<String>) -> Result<QueuedVoice, String> {
    #[derive(Serialize)]
    struct Args {
        text: String,
        voice_id: Option<String>,
    }
    invoke("queue_voice", &Args { text, voice_id }).await
}

pub async fn get_voice_queue() -> Result<Vec<QueuedVoice>, String> {
    invoke_no_args("get_voice_queue").await
}

pub async fn cancel_voice(queue_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        queue_id: String,
    }
    invoke("cancel_voice", &Args { queue_id }).await
}
