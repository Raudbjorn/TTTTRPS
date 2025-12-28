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
    pub session_count: u32,
    pub player_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotSummary {
    pub id: String,
    pub description: String,
    pub created_at: String,
    pub snapshot_type: String,
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

// ============================================================================
// Session Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSession {
    pub id: String,
    pub campaign_id: String,
    pub session_number: u32,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub session_number: u32,
    pub duration_mins: u64,
    pub combat_count: usize,
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
