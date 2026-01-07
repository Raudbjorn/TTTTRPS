//! Tauri IPC Bindings
//!
//! Wrapper functions for calling Tauri commands from the frontend.

use serde::{Deserialize, Serialize};
use serde_json::json;
use wasm_bindgen::prelude::*;

// ============================================================================
// Tauri Invoke
// ============================================================================

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = "invoke", catch)]
    async fn invoke_raw(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

    // Dialog plugin - file picker
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "dialog"], js_name = "open")]
    async fn dialog_open(options: JsValue) -> JsValue;

    // Event listener - for progress events (returns Promise<UnlistenFn>)
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = "listen")]
    fn event_listen(event: &str, handler: &js_sys::Function) -> js_sys::Promise;
}

/// Listen for Tauri events (returns unlisten function wrapped in Promise)
/// Note: In Tauri 2, listen() is async and returns a Promise
pub fn listen_event<F>(event_name: &str, callback: F) -> JsValue
where
    F: Fn(JsValue) + 'static,
{
    let closure = wasm_bindgen::closure::Closure::wrap(Box::new(callback) as Box<dyn Fn(JsValue)>);
    let promise = event_listen(event_name, closure.as_ref().unchecked_ref());
    closure.forget(); // Prevent closure from being dropped

    // The promise resolves to the unlisten function
    // We return the promise as JsValue for compatibility
    promise.into()
}

/// Invoke a Tauri command with typed arguments and response
pub async fn invoke<A: Serialize, R: for<'de> Deserialize<'de>>(
    cmd: &str,
    args: &A,
) -> Result<R, String> {
    let args_js = serde_wasm_bindgen::to_value(args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = invoke_raw(cmd, args_js).await
        .map_err(|e| {
            serde_wasm_bindgen::from_value::<String>(e)
                .unwrap_or_else(|_| "Unknown invoke error".to_string())
        })?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

/// Invoke a Tauri command with no arguments
pub async fn invoke_no_args<R: for<'de> Deserialize<'de>>(cmd: &str) -> Result<R, String> {
    #[derive(Serialize)]
    struct Empty {}
    invoke(cmd, &Empty {}).await
}

/// Invoke a Tauri command that returns void (Result<(), String>)
/// This handles the case where null/undefined is a valid success response
pub async fn invoke_void<A: Serialize>(cmd: &str, args: &A) -> Result<(), String> {
    let args_js = serde_wasm_bindgen::to_value(args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = invoke_raw(cmd, args_js).await
        .map_err(|e| {
            serde_wasm_bindgen::from_value::<String>(e)
                .unwrap_or_else(|_| "Unknown invoke error".to_string())
        })?;

    // For void commands, null/undefined means success
    // Only check for error object with __TAURI_ERROR__ or similar patterns if we needed to,
    // but the catch above handles the rejection case.
    if !result.is_null() && !result.is_undefined() {
        // Double check if it's a success value that looks like an error string (unlikely for void but safe)
        if let Ok(err_str) = serde_wasm_bindgen::from_value::<String>(result.clone()) {
            if !err_str.is_empty() {
                // This path might not be hit if backend rejects on error, but keeping for safety
                return Err(err_str);
            }
        }
    }
    Ok(())
}

/// Invoke a Tauri command with no arguments that returns void
pub async fn invoke_void_no_args(cmd: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct Empty {}
    invoke_void(cmd, &Empty {}).await
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
    pub personality_id: Option<String>,
    pub context: Option<Vec<String>>,
    pub use_rag: bool,
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
    pub piper: Option<PiperConfig>,
    pub coqui: Option<CoquiConfig>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiperConfig {
    pub models_dir: Option<String>,
    pub length_scale: f32,
    pub noise_scale: f32,
    pub noise_w: f32,
    pub sentence_silence: f32,
    pub speaker_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoquiConfig {
    pub port: u16,
    pub model: String,
    pub speaker: Option<String>,
    pub language: Option<String>,
    pub speed: f32,
    pub speaker_wav: Option<String>,
    pub temperature: f32,
    pub top_k: u32,
    pub top_p: f32,
    pub repetition_penalty: f32,
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

/// List all available voices from all providers
pub async fn list_all_voices() -> Result<Vec<Voice>, String> {
    invoke_no_args("list_all_voices").await
}

/// Synthesize and play TTS for the given text and voice ID
pub async fn play_tts(text: String, voice_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        text: String,
        voice_id: String,
    }
    invoke_void("play_tts", &Args { text, voice_id }).await
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

// ============================================================================
// Piper Voice Download
// ============================================================================

/// Available Piper voice from Hugging Face repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailablePiperVoice {
    pub key: String,
    pub name: String,
    pub language: PiperLanguage,
    pub quality: String,
    pub num_speakers: u32,
    pub sample_rate: u32,
    pub files: PiperVoiceFiles,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiperLanguage {
    pub code: String,
    pub family: String,
    pub region: String,
    pub name_native: String,
    pub name_english: String,
    pub country_english: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiperVoiceFiles {
    pub model: PiperFileInfo,
    pub config: PiperFileInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiperFileInfo {
    pub size_bytes: u64,
    pub md5_digest: String,
}

/// Popular/recommended Piper voice entry (key, name, description)
pub type PopularPiperVoice = (String, String, String);

/// List all downloadable Piper voices from Hugging Face (requires network)
pub async fn list_downloadable_piper_voices() -> Result<Vec<AvailablePiperVoice>, String> {
    invoke_no_args("list_downloadable_piper_voices").await
}

/// Get popular/recommended Piper voices (no network call, instant)
pub async fn get_popular_piper_voices() -> Result<Vec<PopularPiperVoice>, String> {
    invoke_no_args("get_popular_piper_voices").await
}

/// Download a Piper voice by key (e.g., "en_US-lessac-medium")
/// Returns the path to the downloaded model file
pub async fn download_piper_voice(voice_key: String, quality: Option<String>) -> Result<String, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        voice_key: String,
        quality: Option<String>,
    }
    invoke("download_piper_voice", &Args { voice_key, quality }).await
}

pub async fn get_vector_store_status() -> Result<String, String> {
    invoke("get_vector_store_status", &()).await
}

/// Audio data returned from speak command for frontend playback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakResult {
    /// Base64-encoded audio data
    pub audio_data: String,
    /// Audio format (e.g., "wav")
    pub format: String,
}

pub async fn speak(text: String) -> Result<Option<SpeakResult>, String> {
    #[derive(Serialize)]
    struct Args {
        text: String,
    }
    invoke("speak", &Args { text }).await
}

// ============================================================================
// Voice Provider Installation
// ============================================================================

/// Installation status for a voice provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallStatus {
    pub provider: VoiceProviderType,
    pub installed: bool,
    pub version: Option<String>,
    pub binary_path: Option<String>,
    pub voices_available: u32,
    pub install_method: InstallMethod,
    pub install_instructions: Option<String>,
}

/// How to install a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallMethod {
    PackageManager(String),
    Python(String),
    Binary(String),
    Docker(String),
    Manual(String),
    AppManaged,
}

/// Check installation status for a specific voice provider
pub async fn check_voice_provider_status(provider: VoiceProviderType) -> Result<InstallStatus, String> {
    #[derive(Serialize)]
    struct Args {
        provider: VoiceProviderType,
    }
    invoke("check_voice_provider_status", &Args { provider }).await
}

/// Check installation status for all local voice providers
pub async fn check_voice_provider_installations() -> Result<Vec<InstallStatus>, String> {
    invoke_no_args("check_voice_provider_installations").await
}

/// Install a voice provider (Piper or Coqui)
pub async fn install_voice_provider(provider: VoiceProviderType) -> Result<InstallStatus, String> {
    #[derive(Serialize)]
    struct Args {
        provider: VoiceProviderType,
    }
    invoke("install_voice_provider", &Args { provider }).await
}

// ============================================================================
// Claude Code CLI Commands
// ============================================================================

/// Status of Claude Code CLI installation and authentication
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClaudeCodeStatus {
    /// Whether the CLI binary is installed
    pub installed: bool,
    /// Whether the user is logged in
    pub logged_in: bool,
    /// Whether the claude-code-bridge skill is installed
    pub skill_installed: bool,
    /// CLI version if available
    pub version: Option<String>,
    /// User email if logged in
    pub user_email: Option<String>,
    /// Error message if any
    pub error: Option<String>,
}

/// Get Claude Code CLI status (installed, logged in, version)
pub async fn get_claude_code_status() -> Result<ClaudeCodeStatus, String> {
    invoke_no_args("get_claude_code_status").await
}

/// Spawn the Claude Code login flow (opens browser for OAuth)
pub async fn claude_code_login() -> Result<(), String> {
    invoke_void_no_args("claude_code_login").await
}

/// Logout from Claude Code
pub async fn claude_code_logout() -> Result<(), String> {
    invoke_void_no_args("claude_code_logout").await
}

/// Install the claude-code-bridge skill to Claude Code
pub async fn claude_code_install_skill() -> Result<(), String> {
    invoke_void_no_args("claude_code_install_skill").await
}

/// Install Claude Code CLI via npm (opens terminal)
pub async fn claude_code_install_cli() -> Result<(), String> {
    invoke_void_no_args("claude_code_install_cli").await
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
    invoke_void("save_api_key", &Args { provider, api_key }).await
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

pub type ThemeWeights = std::collections::HashMap<String, f32>;

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
    pub theme_weights: std::collections::HashMap<String, f32>,
    pub voice_enabled: bool,
    pub auto_transcribe: bool,
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
    invoke("get_campaign", &json!({ "id": id })).await
}

pub async fn delete_campaign(id: String) -> Result<(), String> {
    invoke_void("delete_campaign", &json!({ "id": id })).await
}

/// Archive a campaign (soft delete, can be restored later)
pub async fn archive_campaign(id: String) -> Result<(), String> {
    invoke_void("archive_campaign", &json!({ "id": id })).await
}

/// Restore an archived campaign
pub async fn restore_campaign(id: String) -> Result<(), String> {
    invoke_void("restore_campaign", &json!({ "id": id })).await
}

/// List archived campaigns
pub async fn list_archived_campaigns() -> Result<Vec<Campaign>, String> {
    invoke("list_archived_campaigns", &json!({})).await
}

pub async fn get_campaign_theme(campaign_id: String) -> Result<ThemeWeights, String> {
    invoke("get_campaign_theme", &json!({ "campaign_id": campaign_id })).await
}

pub async fn set_campaign_theme(campaign_id: String, weights: ThemeWeights) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        weights: ThemeWeights,
    }
    invoke_void("set_campaign_theme", &Args { campaign_id, weights }).await
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
    invoke_void("restore_snapshot", &Args { campaign_id, snapshot_id }).await
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
    #[serde(alias = "current_hp")]
    pub hp_current: i32,
    #[serde(alias = "max_hp")]
    pub hp_max: i32,
    #[serde(alias = "armor_class")]
    pub ac: Option<i32>,
    #[serde(alias = "temp_hp")]
    pub hp_temp: Option<i32>,
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
    invoke_void("reorder_session", &Args { session_id, new_order }).await
}

// ============================================================================
// TASK-014: Timeline Commands
// ============================================================================

/// Timeline event type (frontend version matching backend)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineEventType {
    SessionStart,
    SessionPause,
    SessionResume,
    SessionEnd,
    CombatStart,
    CombatEnd,
    CombatRoundStart,
    CombatTurnStart,
    CombatDamage,
    CombatHealing,
    CombatDeath,
    NoteAdded,
    NoteEdited,
    NoteDeleted,
    #[serde(rename = "npc_interaction")]
    NPCInteraction,
    #[serde(rename = "npc_dialogue")]
    NPCDialogue,
    #[serde(rename = "npc_mood")]
    NPCMood,
    LocationChange,
    SceneChange,
    PlayerAction,
    PlayerRoll,
    SkillCheck,
    SavingThrow,
    ConditionApplied,
    ConditionRemoved,
    ConditionExpired,
    ItemAcquired,
    ItemUsed,
    ItemLost,
    Custom(String),
}

/// Event severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum TimelineEventSeverity {
    Trace,
    Info,
    Notable,
    Important,
    Critical,
}

/// Entity reference in timeline events
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineEntityRef {
    pub entity_type: String,
    pub entity_id: String,
    pub name: String,
    pub role: Option<String>,
}

/// Timeline event from backend
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineEventData {
    pub id: String,
    pub session_id: String,
    pub event_type: TimelineEventType,
    pub timestamp: String,
    pub title: String,
    pub description: String,
    pub severity: TimelineEventSeverity,
    pub entity_refs: Vec<TimelineEntityRef>,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    pub tags: Vec<String>,
}

/// Combat summary for timeline
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimelineCombatSummary {
    pub encounters: usize,
    pub total_rounds: u32,
    pub damage_dealt: Option<i32>,
    pub healing_done: Option<i32>,
    pub deaths: usize,
}

/// Key moment from session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineKeyMoment {
    pub title: String,
    pub description: String,
    pub time_offset_minutes: i64,
    pub severity: TimelineEventSeverity,
    pub event_type: TimelineEventType,
}

/// Timeline summary from backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineSummaryData {
    pub session_id: String,
    pub duration_minutes: i64,
    pub total_events: usize,
    pub combat: TimelineCombatSummary,
    pub key_moments: Vec<TimelineKeyMoment>,
    pub npcs_encountered: Vec<TimelineEntityRef>,
    pub locations_visited: Vec<TimelineEntityRef>,
    pub items_acquired: Vec<String>,
    pub conditions_applied: Vec<String>,
    pub tags_used: Vec<String>,
}

/// Add a timeline event to a session
pub async fn add_timeline_event(
    session_id: String,
    event_type: String,
    title: String,
    description: String,
    severity: Option<String>,
    entity_refs: Option<Vec<TimelineEntityRef>>,
    tags: Option<Vec<String>>,
    metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
) -> Result<TimelineEventData, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        event_type: String,
        title: String,
        description: String,
        severity: Option<String>,
        entity_refs: Option<Vec<TimelineEntityRef>>,
        tags: Option<Vec<String>>,
        metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
    }
    invoke(
        "add_timeline_event",
        &Args {
            session_id,
            event_type,
            title,
            description,
            severity,
            entity_refs,
            tags,
            metadata,
        },
    )
    .await
}

/// Get all timeline events for a session
pub async fn get_session_timeline(session_id: String) -> Result<Vec<TimelineEventData>, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke("get_session_timeline", &Args { session_id }).await
}

/// Get timeline summary for a session
pub async fn get_timeline_summary(session_id: String) -> Result<TimelineSummaryData, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke("get_timeline_summary", &Args { session_id }).await
}

/// Get timeline events filtered by type
pub async fn get_timeline_events_by_type(
    session_id: String,
    event_type: String,
) -> Result<Vec<TimelineEventData>, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        event_type: String,
    }
    invoke("get_timeline_events_by_type", &Args { session_id, event_type }).await
}

/// Generate a session summary narrative from timeline
pub async fn generate_session_summary(session_id: String) -> Result<String, String> {
    // First get the timeline summary
    let summary = get_timeline_summary(session_id.clone()).await?;

    // Format into a narrative string
    let mut narrative = String::new();
    narrative.push_str(&format!(
        "Session Summary (Duration: {} minutes, {} events)\n\n",
        summary.duration_minutes, summary.total_events
    ));

    // Key moments
    if !summary.key_moments.is_empty() {
        narrative.push_str("KEY MOMENTS:\n");
        for moment in &summary.key_moments {
            narrative.push_str(&format!(
                "- [{:?}] {} - {}\n",
                moment.severity, moment.title, moment.description
            ));
        }
        narrative.push('\n');
    }

    // Combat
    if summary.combat.encounters > 0 {
        narrative.push_str(&format!(
            "COMBAT: {} encounter(s), {} total rounds",
            summary.combat.encounters, summary.combat.total_rounds
        ));
        if let Some(damage) = summary.combat.damage_dealt {
            narrative.push_str(&format!(", {} damage dealt", damage));
        }
        if let Some(healing) = summary.combat.healing_done {
            narrative.push_str(&format!(", {} healing done", healing));
        }
        if summary.combat.deaths > 0 {
            narrative.push_str(&format!(", {} death(s)", summary.combat.deaths));
        }
        narrative.push_str("\n\n");
    }

    // NPCs
    if !summary.npcs_encountered.is_empty() {
        narrative.push_str("NPCs ENCOUNTERED: ");
        let names: Vec<&str> = summary.npcs_encountered.iter().map(|n| n.name.as_str()).collect();
        narrative.push_str(&names.join(", "));
        narrative.push_str("\n\n");
    }

    // Locations
    if !summary.locations_visited.is_empty() {
        narrative.push_str("LOCATIONS VISITED: ");
        let names: Vec<&str> = summary.locations_visited.iter().map(|l| l.name.as_str()).collect();
        narrative.push_str(&names.join(", "));
        narrative.push_str("\n\n");
    }

    // Items
    if !summary.items_acquired.is_empty() {
        narrative.push_str("ITEMS ACQUIRED: ");
        narrative.push_str(&summary.items_acquired.join(", "));
        narrative.push_str("\n\n");
    }

    Ok(narrative)
}

// ============================================================================
// TASK-017: Session Notes Types
// ============================================================================

/// Note category for organization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NoteCategory {
    General,
    Combat,
    Character,
    Location,
    Plot,
    Quest,
    Loot,
    Rules,
    Meta,
    Worldbuilding,
    Dialogue,
    Secret,
    #[serde(untagged)]
    Custom(String),
}

impl Default for NoteCategory {
    fn default() -> Self {
        NoteCategory::General
    }
}

/// Entity type for linking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NoteEntityType {
    NPC,
    Player,
    Location,
    Item,
    Quest,
    Session,
    Campaign,
    Combat,
    #[serde(untagged)]
    Custom(String),
}

/// Entity link in a note
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteEntityLink {
    pub entity_type: NoteEntityType,
    pub entity_id: String,
    pub entity_name: String,
    pub linked_at: String,
}

/// Session note data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionNote {
    pub id: String,
    pub session_id: String,
    pub campaign_id: String,
    pub title: String,
    pub content: String,
    pub category: NoteCategory,
    pub tags: Vec<String>,
    pub linked_entities: Vec<NoteEntityLink>,
    pub is_pinned: bool,
    pub is_private: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// AI categorization response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorizationResponse {
    pub suggested_category: String,
    pub suggested_tags: Vec<String>,
    pub detected_entities: Vec<DetectedEntity>,
    pub confidence: f32,
    pub reasoning: Option<String>,
}

/// Entity detected by AI in note content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedEntity {
    pub entity_type: String,
    pub name: String,
    pub context: Option<String>,
}

// ============================================================================
// TASK-017: Session Notes Commands
// ============================================================================

/// Create a new session note
pub async fn create_session_note(
    session_id: String,
    campaign_id: String,
    title: String,
    content: String,
    category: Option<String>,
    tags: Option<Vec<String>>,
    is_pinned: Option<bool>,
    is_private: Option<bool>,
) -> Result<SessionNote, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        campaign_id: String,
        title: String,
        content: String,
        category: Option<String>,
        tags: Option<Vec<String>>,
        is_pinned: Option<bool>,
        is_private: Option<bool>,
    }
    invoke("create_session_note", &Args {
        session_id,
        campaign_id,
        title,
        content,
        category,
        tags,
        is_pinned,
        is_private,
    }).await
}

/// Get a session note by ID
pub async fn get_session_note(note_id: String) -> Result<Option<SessionNote>, String> {
    #[derive(Serialize)]
    struct Args {
        note_id: String,
    }
    invoke("get_session_note", &Args { note_id }).await
}

/// Update an existing session note
pub async fn update_session_note(note: SessionNote) -> Result<SessionNote, String> {
    #[derive(Serialize)]
    struct Args {
        note: SessionNote,
    }
    invoke("update_session_note", &Args { note }).await
}

/// Delete a session note
pub async fn delete_session_note(note_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        note_id: String,
    }
    invoke_void("delete_session_note", &Args { note_id }).await
}

/// List all notes for a session
pub async fn list_session_notes(session_id: String) -> Result<Vec<SessionNote>, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke("list_session_notes", &Args { session_id }).await
}

/// Search notes with optional session filter
pub async fn search_session_notes(
    query: String,
    session_id: Option<String>,
) -> Result<Vec<SessionNote>, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
        session_id: Option<String>,
    }
    invoke("search_session_notes", &Args { query, session_id }).await
}

/// Get notes filtered by category
pub async fn get_notes_by_category(
    category: String,
    session_id: Option<String>,
) -> Result<Vec<SessionNote>, String> {
    #[derive(Serialize)]
    struct Args {
        category: String,
        session_id: Option<String>,
    }
    invoke("get_notes_by_category", &Args { category, session_id }).await
}

/// Get notes with a specific tag
pub async fn get_notes_by_tag(tag: String) -> Result<Vec<SessionNote>, String> {
    #[derive(Serialize)]
    struct Args {
        tag: String,
    }
    invoke("get_notes_by_tag", &Args { tag }).await
}

/// AI-powered note categorization
pub async fn categorize_note_ai(
    title: String,
    content: String,
) -> Result<CategorizationResponse, String> {
    #[derive(Serialize)]
    struct Args {
        title: String,
        content: String,
    }
    invoke("categorize_note_ai", &Args { title, content }).await
}

/// Link an entity to a note
pub async fn link_entity_to_note(
    note_id: String,
    entity_type: String,
    entity_id: String,
    entity_name: String,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        note_id: String,
        entity_type: String,
        entity_id: String,
        entity_name: String,
    }
    invoke_void("link_entity_to_note", &Args {
        note_id,
        entity_type,
        entity_id,
        entity_name,
    }).await
}

/// Unlink an entity from a note
pub async fn unlink_entity_from_note(
    note_id: String,
    entity_id: String,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        note_id: String,
        entity_id: String,
    }
    invoke_void("unlink_entity_from_note", &Args { note_id, entity_id }).await
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
    invoke_void("end_combat", &Args { session_id }).await
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
    add_combatant_full(session_id, name, initiative, combatant_type, None, None, None).await
}

/// Add combatant with full options including HP and AC
pub async fn add_combatant_full(
    session_id: String,
    name: String,
    initiative: i32,
    combatant_type: String,
    hp_current: Option<i32>,
    hp_max: Option<i32>,
    armor_class: Option<i32>,
) -> Result<Combatant, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        name: String,
        initiative: i32,
        combatant_type: String,
        hp_current: Option<i32>,
        hp_max: Option<i32>,
        armor_class: Option<i32>,
    }
    invoke("add_combatant", &Args { session_id, name, initiative, combatant_type, hp_current, hp_max, armor_class }).await
}

pub async fn remove_combatant(session_id: String, combatant_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        combatant_id: String,
    }
    invoke_void("remove_combatant", &Args { session_id, combatant_id }).await
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
    invoke_void("add_condition", &Args { session_id, combatant_id, condition_name }).await
}

pub async fn remove_condition(session_id: String, combatant_id: String, condition_name: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        combatant_id: String,
        condition_name: String,
    }
    invoke_void("remove_condition", &Args { session_id, combatant_id, condition_name }).await
}

// ============================================================================
// Advanced Condition Types (TASK-015)
// ============================================================================

/// Duration types for conditions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConditionDurationType {
    Turns,
    Rounds,
    Minutes,
    Hours,
    #[serde(alias = "EndOfNextTurn")]
    EndOfNextTurn,
    #[serde(alias = "StartOfNextTurn")]
    StartOfNextTurn,
    #[serde(alias = "EndOfSourceTurn")]
    EndOfSourceTurn,
    #[serde(alias = "UntilSave")]
    UntilSave,
    #[default]
    #[serde(alias = "UntilRemoved")]
    UntilRemoved,
    #[serde(alias = "Permanent")]
    Permanent,
}

impl ConditionDurationType {
    pub fn to_string_key(&self) -> &'static str {
        match self {
            Self::Turns => "turns",
            Self::Rounds => "rounds",
            Self::Minutes => "minutes",
            Self::Hours => "hours",
            Self::EndOfNextTurn => "end_of_next_turn",
            Self::StartOfNextTurn => "start_of_next_turn",
            Self::EndOfSourceTurn => "end_of_source_turn",
            Self::UntilSave => "until_save",
            Self::UntilRemoved => "until_removed",
            Self::Permanent => "permanent",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Turns => "Turns",
            Self::Rounds => "Rounds",
            Self::Minutes => "Minutes",
            Self::Hours => "Hours",
            Self::EndOfNextTurn => "End of Next Turn",
            Self::StartOfNextTurn => "Start of Next Turn",
            Self::EndOfSourceTurn => "End of Source's Turn",
            Self::UntilSave => "Until Save",
            Self::UntilRemoved => "Until Removed",
            Self::Permanent => "Permanent",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Turns,
            Self::Rounds,
            Self::Minutes,
            Self::EndOfNextTurn,
            Self::StartOfNextTurn,
            Self::UntilSave,
            Self::UntilRemoved,
            Self::Permanent,
        ]
    }
}

/// Effect type for conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionEffect {
    pub description: String,
    pub mechanic: Option<String>,
}

/// Advanced condition with duration tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedCondition {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub effects: Vec<ConditionEffect>,
    #[serde(default)]
    pub duration_type: ConditionDurationType,
    pub remaining: Option<u32>,
    pub source_id: Option<String>,
    pub source_name: Option<String>,
    pub save_type: Option<String>,
    pub save_dc: Option<u32>,
    pub applied_at_round: Option<u32>,
    pub applied_at_turn: Option<usize>,
}

impl AdvancedCondition {
    pub fn duration_display(&self) -> String {
        match self.remaining {
            Some(n) if n > 0 => format!("{} {}", n, self.duration_type.display_name()),
            None => match self.duration_type {
                ConditionDurationType::UntilSave => {
                    if let (Some(save_type), Some(dc)) = (&self.save_type, self.save_dc) {
                        format!("Until {} save (DC {})", save_type, dc)
                    } else {
                        "Until Save".to_string()
                    }
                }
                _ => self.duration_type.display_name().to_string(),
            },
            _ => "Expired".to_string(),
        }
    }
}

/// Request to add an advanced condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddConditionRequest {
    pub session_id: String,
    pub combatant_id: String,
    pub condition_name: String,
    pub duration_type: Option<String>,
    pub duration_value: Option<u32>,
    pub source_id: Option<String>,
    pub source_name: Option<String>,
    pub save_type: Option<String>,
    pub save_dc: Option<u32>,
}

pub async fn add_condition_advanced(request: AddConditionRequest) -> Result<(), String> {
    invoke_void("add_condition_advanced", &request).await
}

pub async fn remove_condition_by_id(session_id: String, combatant_id: String, condition_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args { session_id: String, combatant_id: String, condition_id: String }
    invoke_void("remove_condition_by_id", &Args { session_id, combatant_id, condition_id }).await
}

pub async fn get_combatant_conditions(session_id: String, combatant_id: String) -> Result<Vec<AdvancedCondition>, String> {
    #[derive(Serialize)]
    struct Args { session_id: String, combatant_id: String }
    invoke("get_combatant_conditions", &Args { session_id, combatant_id }).await
}

pub async fn tick_conditions_end_of_turn(session_id: String, combatant_id: String) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args { session_id: String, combatant_id: String }
    invoke("tick_conditions_end_of_turn", &Args { session_id, combatant_id }).await
}

pub async fn tick_conditions_start_of_turn(session_id: String, combatant_id: String) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args { session_id: String, combatant_id: String }
    invoke("tick_conditions_start_of_turn", &Args { session_id, combatant_id }).await
}

pub async fn list_condition_templates() -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args {}
    invoke("list_condition_templates", &Args {}).await
}

pub const STANDARD_CONDITIONS: &[&str] = &[
    "Blinded", "Charmed", "Deafened", "Exhaustion", "Frightened", "Grappled",
    "Incapacitated", "Invisible", "Paralyzed", "Petrified", "Poisoned",
    "Prone", "Restrained", "Stunned", "Unconscious",
];

pub fn get_condition_description(name: &str) -> Option<&'static str> {
    match name.to_lowercase().as_str() {
        "blinded" => Some("Can't see. Auto-fails sight checks. Attacks have advantage against, disadvantage on attacks."),
        "charmed" => Some("Can't attack charmer. Charmer has advantage on social checks."),
        "deafened" => Some("Can't hear. Auto-fails hearing checks."),
        "exhaustion" => Some("Cumulative levels with increasing penalties."),
        "frightened" => Some("Disadvantage on checks/attacks while fear source visible. Can't move closer."),
        "grappled" => Some("Speed 0. Ends if grappler incapacitated or removed from reach."),
        "incapacitated" => Some("Can't take actions or reactions."),
        "invisible" => Some("Can't be seen. Attacks against have disadvantage, attacks have advantage."),
        "paralyzed" => Some("Incapacitated, can't move/speak. Auto-fail STR/DEX saves. Attacks have advantage, crits in 5ft."),
        "petrified" => Some("Transformed to stone. Incapacitated, resistant to damage, immune to poison/disease."),
        "poisoned" => Some("Disadvantage on attacks and ability checks."),
        "prone" => Some("Can only crawl. Disadvantage on attacks. Advantage/disadvantage based on distance."),
        "restrained" => Some("Speed 0. Attacks against have advantage. Disadvantage on attacks and DEX saves."),
        "stunned" => Some("Incapacitated, can't move. Auto-fail STR/DEX saves. Attacks have advantage."),
        "unconscious" => Some("Incapacitated, drops items, falls prone. Auto-fail STR/DEX. Attacks have advantage, crits in 5ft."),
        _ => None,
    }
}

// ============================================================================
// Character Types (TASK-018: Enhanced Multi-System Character Generation)
// ============================================================================

/// Generated character with full system-specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub system: String,
    pub concept: String,
    pub race: Option<String>,
    #[serde(rename = "class")]
    pub character_class: Option<String>,
    pub level: u32,
    pub attributes: std::collections::HashMap<String, CharacterAttributeValue>,
    pub skills: std::collections::HashMap<String, i32>,
    pub traits: Vec<CharacterTrait>,
    pub equipment: Vec<CharacterEquipment>,
    pub background: CharacterBackground,
    pub backstory: Option<String>,
    pub notes: String,
    pub portrait_prompt: Option<String>,
}

/// Attribute with base value and modifiers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterAttributeValue {
    pub base: i32,
    pub modifier: i32,
    pub temp_bonus: i32,
}

/// Character trait (feat, ability, aspect, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterTrait {
    pub name: String,
    pub trait_type: String,
    pub description: String,
    pub mechanical_effect: Option<String>,
}

/// Character equipment item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterEquipment {
    pub name: String,
    pub category: String,
    pub description: String,
    pub stats: std::collections::HashMap<String, String>,
}

/// Character background info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharacterBackground {
    pub origin: String,
    pub occupation: Option<String>,
    pub motivation: String,
    pub connections: Vec<String>,
    pub secrets: Vec<String>,
    pub history: String,
}

/// Legacy attribute format for backwards compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeValue {
    pub name: String,
    pub value: i32,
    pub modifier: Option<i32>,
}

/// Options for character generation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenerationOptions {
    pub system: Option<String>,
    pub name: Option<String>,
    pub concept: Option<String>,
    pub race: Option<String>,
    #[serde(rename = "class")]
    pub character_class: Option<String>,
    pub background: Option<String>,
    pub level: Option<u32>,
    pub point_buy: Option<u32>,
    pub random_stats: bool,
    pub include_equipment: bool,
    pub include_backstory: bool,
    pub backstory_length: Option<String>,
    pub theme: Option<String>,
    pub campaign_setting: Option<String>,
}

/// Information about a game system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSystemInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub races: Vec<String>,
    pub classes: Vec<String>,
    pub backgrounds: Vec<String>,
    pub attributes: Vec<String>,
    pub has_levels: bool,
    pub max_level: Option<u32>,
}

// ============================================================================
// Character Commands (TASK-018)
// ============================================================================

/// Generate a character with default options (backward compatible)
pub async fn generate_character(options: GenerationOptions) -> Result<Character, String> {
    #[derive(Serialize)]
    struct Args {
        options: GenerationOptions,
    }
    invoke("generate_character_advanced", &Args { options }).await
}

/// Generate a character with advanced options
pub async fn generate_character_advanced(options: GenerationOptions) -> Result<Character, String> {
    #[derive(Serialize)]
    struct Args {
        options: GenerationOptions,
    }
    invoke("generate_character_advanced", &Args { options }).await
}

/// Get list of supported system names
pub async fn get_supported_systems() -> Result<Vec<String>, String> {
    invoke_no_args("get_supported_systems").await
}

/// Get detailed info for all systems
pub async fn list_system_info() -> Result<Vec<GameSystemInfo>, String> {
    invoke_no_args("list_system_info").await
}

/// Get info for a specific system
pub async fn get_game_system_info(system: String) -> Result<Option<GameSystemInfo>, String> {
    #[derive(Serialize)]
    struct Args {
        system: String,
    }
    invoke("get_system_info", &Args { system }).await
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
// Hybrid Search Types and Commands
// ============================================================================

/// Options for hybrid search
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HybridSearchOptions {
    /// Maximum results to return
    #[serde(default)]
    pub limit: usize,
    /// Source type filter
    pub source_type: Option<String>,
    /// Campaign ID filter
    pub campaign_id: Option<String>,
    /// Index to search (None = federated search)
    pub index: Option<String>,
    /// Override semantic weight (0.0 - 1.0)
    pub semantic_weight: Option<f32>,
    /// Override keyword weight (0.0 - 1.0)
    pub keyword_weight: Option<f32>,
}

/// Hybrid search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResultPayload {
    pub content: String,
    pub source: String,
    pub source_type: String,
    pub page_number: Option<u32>,
    pub score: f32,
    pub index: String,
    pub keyword_rank: Option<usize>,
    pub semantic_rank: Option<usize>,
}

/// Hybrid search response with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResponse {
    pub results: Vec<HybridSearchResultPayload>,
    pub total_hits: usize,
    pub original_query: String,
    pub expanded_query: Option<String>,
    pub corrected_query: Option<String>,
    pub processing_time_ms: u64,
    pub hints: Vec<String>,
}

/// Query expansion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExpansionResult {
    pub original: String,
    pub expanded_query: String,
    pub was_expanded: bool,
    pub expansions: Vec<ExpansionInfo>,
    pub hints: Vec<String>,
}

/// Expansion info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpansionInfo {
    pub original: String,
    pub expanded_to: Vec<String>,
    pub category: String,
}

/// Spell correction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionResult {
    pub original_query: String,
    pub corrected_query: String,
    pub corrections: Vec<SpellingSuggestion>,
    pub has_corrections: bool,
}

/// Spelling suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellingSuggestion {
    pub original: String,
    pub suggestion: String,
    pub distance: usize,
    pub confidence: f64,
}

/// Perform hybrid search with RRF fusion
pub async fn hybrid_search(query: String, options: Option<HybridSearchOptions>) -> Result<HybridSearchResponse, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
        options: Option<HybridSearchOptions>,
    }
    invoke("hybrid_search", &Args { query, options }).await
}

/// Get search suggestions for autocomplete
pub async fn get_search_suggestions(partial: String) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args {
        partial: String,
    }
    invoke("get_search_suggestions", &Args { partial }).await
}

/// Get search hints for a query
pub async fn get_search_hints(query: String) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
    }
    invoke("get_search_hints", &Args { query }).await
}

/// Expand a query with TTRPG synonyms
pub async fn expand_query(query: String) -> Result<QueryExpansionResult, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
    }
    invoke("expand_query", &Args { query }).await
}

/// Correct spelling in a query
pub async fn correct_query(query: String) -> Result<CorrectionResult, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
    }
    invoke("correct_query", &Args { query }).await
}

/// Copy text to system clipboard
pub async fn copy_to_clipboard(text: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        text: String,
    }
    invoke("copy_to_clipboard", &Args { text }).await
}

// ============================================================================
// Usage Tracking Types and Commands (TASK-022 - Enhanced)
// See end of file for full implementation
// ============================================================================

// Session usage types for chat component (backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUsage {
    pub session_input_tokens: u64,
    pub session_output_tokens: u64,
    pub session_requests: u32,
    pub session_cost_usd: f64,
}

impl Default for SessionUsage {
    fn default() -> Self {
        Self {
            session_input_tokens: 0,
            session_output_tokens: 0,
            session_requests: 0,
            session_cost_usd: 0.0,
        }
    }
}

pub async fn get_session_usage() -> Result<SessionUsage, String> {
    // Map from the new UsageStats format
    let stats: Result<UsageStats, String> = invoke_no_args("get_usage_stats").await;
    match stats {
        Ok(s) => Ok(SessionUsage {
            session_input_tokens: s.total_input_tokens,
            session_output_tokens: s.total_output_tokens,
            session_requests: s.total_requests,
            session_cost_usd: s.total_cost_usd,
        }),
        Err(e) => Err(e),
    }
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
    invoke_void("mark_npc_read", &Args { npc_id }).await
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    invoke_void("update_npc", &Args { npc }).await
}

pub async fn delete_npc(id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        id: String,
    }
    invoke_void("delete_npc", &Args { id }).await
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
// Voice Queue Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VoiceStatus {
    Queued,
    Playing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueuedVoice {
    pub id: String,
    pub text: String,
    pub voice_id: Option<String>,
    pub status: VoiceStatus,
    pub created_at: String,
}

// ============================================================================
// Voice Queue Commands
// ============================================================================

pub async fn get_voice_queue() -> Result<Vec<QueuedVoice>, String> {
    invoke_no_args("get_voice_queue").await
}

pub async fn queue_voice(text: String, voice_id: Option<String>) -> Result<QueuedVoice, String> {
    #[derive(Serialize)]
    struct Args {
        text: String,
        voice_id: Option<String>,
    }
    invoke("queue_voice", &Args { text, voice_id }).await
}

// ============================================================================
// Campaign Versioning Types (TASK-006)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionSummary {
    pub id: String,
    pub campaign_id: String,
    pub version_number: u64,
    pub description: String,
    pub version_type: String,
    pub created_at: String,
    pub created_by: Option<String>,
    pub tags: Vec<String>,
    pub size_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignVersion {
    pub id: String,
    pub campaign_id: String,
    pub version_number: u64,
    pub description: String,
    pub version_type: String,
    pub created_at: String,
    pub created_by: Option<String>,
    pub data_snapshot: String,
    pub data_hash: String,
    pub parent_version_id: Option<String>,
    pub tags: Vec<String>,
    pub size_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignDiff {
    pub from_version_id: String,
    pub to_version_id: String,
    pub from_version_number: u64,
    pub to_version_number: u64,
    pub changes: Vec<DiffEntry>,
    pub stats: DiffStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    pub path: String,
    pub operation: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiffStats {
    pub added_count: usize,
    pub removed_count: usize,
    pub modified_count: usize,
    pub total_changes: usize,
}

// ============================================================================
// Campaign Versioning Commands
// ============================================================================

pub async fn create_campaign_version(
    campaign_id: String,
    description: String,
    version_type: String,
) -> Result<VersionSummary, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        description: String,
        version_type: String,
    }
    invoke("create_campaign_version", &Args { campaign_id, description, version_type }).await
}

pub async fn list_campaign_versions(campaign_id: String) -> Result<Vec<VersionSummary>, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String }
    invoke("list_campaign_versions", &Args { campaign_id }).await
}

pub async fn get_campaign_version(campaign_id: String, version_id: String) -> Result<CampaignVersion, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, version_id: String }
    invoke("get_campaign_version", &Args { campaign_id, version_id }).await
}

pub async fn compare_campaign_versions(
    campaign_id: String,
    from_version_id: String,
    to_version_id: String,
) -> Result<CampaignDiff, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, from_version_id: String, to_version_id: String }
    invoke("compare_campaign_versions", &Args { campaign_id, from_version_id, to_version_id }).await
}

pub async fn rollback_campaign(campaign_id: String, version_id: String) -> Result<Campaign, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, version_id: String }
    invoke("rollback_campaign", &Args { campaign_id, version_id }).await
}

pub async fn delete_campaign_version(campaign_id: String, version_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, version_id: String }
    invoke_void("delete_campaign_version", &Args { campaign_id, version_id }).await
}

pub async fn add_version_tag(campaign_id: String, version_id: String, tag: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, version_id: String, tag: String }
    invoke_void("add_version_tag", &Args { campaign_id, version_id, tag }).await
}

pub async fn mark_version_milestone(campaign_id: String, version_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, version_id: String }
    invoke_void("mark_version_milestone", &Args { campaign_id, version_id }).await
}

// ============================================================================
// World State Types (TASK-007)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InGameDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub era: Option<String>,
    pub calendar: String,
    pub time: Option<InGameTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InGameTime {
    pub hour: u8,
    pub minute: u8,
    pub period: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub campaign_id: String,
    pub current_date: InGameDate,
    pub events: Vec<WorldEvent>,
    pub locations: std::collections::HashMap<String, LocationState>,
    pub npc_relationships: Vec<NpcRelationshipState>,
    pub custom_fields: std::collections::HashMap<String, serde_json::Value>,
    pub updated_at: String,
    pub calendar_config: CalendarConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEvent {
    pub id: String,
    pub campaign_id: String,
    pub in_game_date: InGameDate,
    pub recorded_at: String,
    pub title: String,
    pub description: String,
    pub event_type: String,
    pub impact: String,
    pub location_ids: Vec<String>,
    pub npc_ids: Vec<String>,
    pub pc_ids: Vec<String>,
    pub consequences: Vec<String>,
    pub session_number: Option<u32>,
    pub is_public: bool,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationState {
    pub location_id: String,
    pub name: String,
    pub condition: String,
    pub ruler: Option<String>,
    pub controlling_faction: Option<String>,
    pub population: Option<u64>,
    pub notable_npcs: Vec<String>,
    pub active_effects: Vec<String>,
    pub resources: std::collections::HashMap<String, i32>,
    pub properties: std::collections::HashMap<String, serde_json::Value>,
    pub updated_at: String,
    pub last_accurate_date: InGameDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcRelationshipState {
    pub npc_id: String,
    pub target_id: String,
    pub target_type: String,
    pub disposition: i32,
    pub relationship_type: String,
    pub familiarity: u8,
    pub recent_interactions: Vec<InteractionRecord>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionRecord {
    pub in_game_date: InGameDate,
    pub description: String,
    pub disposition_change: i32,
    pub session_number: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarConfig {
    pub name: String,
    pub months_per_year: u8,
    pub days_per_month: Vec<u8>,
    pub month_names: Vec<String>,
    pub week_days: Vec<String>,
    pub eras: Vec<String>,
}

// ============================================================================
// World State Commands
// ============================================================================

pub async fn get_world_state(campaign_id: String) -> Result<WorldState, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String }
    invoke("get_world_state", &Args { campaign_id }).await
}

pub async fn update_world_state(world_state: WorldState) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args { world_state: WorldState }
    invoke_void("update_world_state", &Args { world_state }).await
}

pub async fn set_in_game_date(campaign_id: String, date: InGameDate) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, date: InGameDate }
    invoke_void("set_in_game_date", &Args { campaign_id, date }).await
}

pub async fn advance_in_game_date(campaign_id: String, days: i32) -> Result<InGameDate, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, days: i32 }
    invoke("advance_in_game_date", &Args { campaign_id, days }).await
}

pub async fn get_in_game_date(campaign_id: String) -> Result<InGameDate, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String }
    invoke("get_in_game_date", &Args { campaign_id }).await
}

pub async fn add_world_event(
    campaign_id: String,
    title: String,
    description: String,
    date: InGameDate,
    event_type: String,
    impact: String,
) -> Result<WorldEvent, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        title: String,
        description: String,
        date: InGameDate,
        event_type: String,
        impact: String,
    }
    invoke("add_world_event", &Args { campaign_id, title, description, date, event_type, impact }).await
}

pub async fn list_world_events(
    campaign_id: String,
    event_type: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<WorldEvent>, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, event_type: Option<String>, limit: Option<usize> }
    invoke("list_world_events", &Args { campaign_id, event_type, limit }).await
}

pub async fn delete_world_event(campaign_id: String, event_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, event_id: String }
    invoke_void("delete_world_event", &Args { campaign_id, event_id }).await
}

pub async fn set_location_state(campaign_id: String, location: LocationState) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, location: LocationState }
    invoke_void("set_location_state", &Args { campaign_id, location }).await
}

pub async fn get_location_state(campaign_id: String, location_id: String) -> Result<Option<LocationState>, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, location_id: String }
    invoke("get_location_state", &Args { campaign_id, location_id }).await
}

pub async fn list_locations(campaign_id: String) -> Result<Vec<LocationState>, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String }
    invoke("list_locations", &Args { campaign_id }).await
}

pub async fn update_location_condition(
    campaign_id: String,
    location_id: String,
    condition: String,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, location_id: String, condition: String }
    invoke_void("update_location_condition", &Args { campaign_id, location_id, condition }).await
}

pub async fn set_world_custom_field(
    campaign_id: String,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, key: String, value: serde_json::Value }
    invoke_void("set_world_custom_field", &Args { campaign_id, key, value }).await
}

pub async fn get_world_custom_field(campaign_id: String, key: String) -> Result<Option<serde_json::Value>, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, key: String }
    invoke("get_world_custom_field", &Args { campaign_id, key }).await
}

pub async fn list_world_custom_fields(campaign_id: String) -> Result<std::collections::HashMap<String, serde_json::Value>, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String }
    invoke("list_world_custom_fields", &Args { campaign_id }).await
}

pub async fn set_calendar_config(campaign_id: String, config: CalendarConfig) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, config: CalendarConfig }
    invoke_void("set_calendar_config", &Args { campaign_id, config }).await
}

pub async fn get_calendar_config(campaign_id: String) -> Result<Option<CalendarConfig>, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String }
    invoke("get_calendar_config", &Args { campaign_id }).await
}

// ============================================================================
// Entity Relationship Types (TASK-009)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRelationship {
    pub id: String,
    pub campaign_id: String,
    pub source_id: String,
    pub source_type: String,
    pub source_name: String,
    pub target_id: String,
    pub target_type: String,
    pub target_name: String,
    pub relationship_type: String,
    pub strength: String,
    pub is_active: bool,
    pub is_known: bool,
    pub description: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub tags: Vec<String>,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSummary {
    pub id: String,
    pub source_id: String,
    pub source_name: String,
    pub source_type: String,
    pub target_id: String,
    pub target_name: String,
    pub target_type: String,
    pub relationship_type: String,
    pub strength: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub stats: GraphStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub color: String,
    pub connection_count: usize,
    pub is_hub: bool,
    pub data: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub label: String,
    pub strength: u8,
    pub bidirectional: bool,
    pub is_active: bool,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub entity_type_counts: std::collections::HashMap<String, usize>,
    pub relationship_type_counts: std::collections::HashMap<String, usize>,
    pub most_connected_entities: Vec<(String, usize)>,
}

// ============================================================================
// Entity Relationship Commands
// ============================================================================

pub async fn create_entity_relationship(
    campaign_id: String,
    source_id: String,
    source_type: String,
    source_name: String,
    target_id: String,
    target_type: String,
    target_name: String,
    relationship_type: String,
    strength: Option<String>,
    description: Option<String>,
) -> Result<EntityRelationship, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        source_id: String,
        source_type: String,
        source_name: String,
        target_id: String,
        target_type: String,
        target_name: String,
        relationship_type: String,
        strength: Option<String>,
        description: Option<String>,
    }
    invoke("create_entity_relationship", &Args {
        campaign_id, source_id, source_type, source_name,
        target_id, target_type, target_name, relationship_type,
        strength, description,
    }).await
}

pub async fn get_entity_relationship(
    campaign_id: String,
    relationship_id: String,
) -> Result<Option<EntityRelationship>, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, relationship_id: String }
    invoke("get_entity_relationship", &Args { campaign_id, relationship_id }).await
}

pub async fn update_entity_relationship(relationship: EntityRelationship) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args { relationship: EntityRelationship }
    invoke_void("update_entity_relationship", &Args { relationship }).await
}

pub async fn delete_entity_relationship(campaign_id: String, relationship_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, relationship_id: String }
    invoke_void("delete_entity_relationship", &Args { campaign_id, relationship_id }).await
}

pub async fn list_entity_relationships(campaign_id: String) -> Result<Vec<RelationshipSummary>, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String }
    invoke("list_entity_relationships", &Args { campaign_id }).await
}

pub async fn get_relationships_for_entity(
    campaign_id: String,
    entity_id: String,
) -> Result<Vec<EntityRelationship>, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, entity_id: String }
    invoke("get_relationships_for_entity", &Args { campaign_id, entity_id }).await
}

pub async fn get_relationships_between_entities(
    campaign_id: String,
    entity_a: String,
    entity_b: String,
) -> Result<Vec<EntityRelationship>, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, entity_a: String, entity_b: String }
    invoke("get_relationships_between_entities", &Args { campaign_id, entity_a, entity_b }).await
}

pub async fn get_entity_graph(
    campaign_id: String,
    include_inactive: Option<bool>,
) -> Result<EntityGraph, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, include_inactive: Option<bool> }
    invoke("get_entity_graph", &Args { campaign_id, include_inactive }).await
}

pub async fn get_ego_graph(
    campaign_id: String,
    entity_id: String,
    depth: Option<usize>,
) -> Result<EntityGraph, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, entity_id: String, depth: Option<usize> }
    invoke("get_ego_graph", &Args { campaign_id, entity_id, depth }).await
}

// ============================================================================
// TASK-022: Usage Tracking Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageStats {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cached_tokens: u64,
    pub total_requests: u32,
    pub total_cost_usd: f64,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub by_provider: std::collections::HashMap<String, ProviderUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderUsage {
    pub provider: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub requests: u32,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CostBreakdown {
    pub total_cost_usd: f64,
    pub by_provider: std::collections::HashMap<String, ProviderCostDetails>,
    pub by_model: std::collections::HashMap<String, ModelCostDetails>,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderCostDetails {
    pub provider: String,
    pub total_cost_usd: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub requests: u32,
    pub avg_cost_per_request: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelCostDetails {
    pub model: String,
    pub provider: String,
    pub total_cost_usd: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub requests: u32,
    pub input_price_per_million: f64,
    pub output_price_per_million: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetLimit {
    pub limit_usd: f64,
    pub period: String,
    pub warning_threshold: f64,
    pub critical_threshold: f64,
    pub block_on_limit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetStatus {
    pub period: String,
    pub limit_usd: f64,
    pub spent_usd: f64,
    pub remaining_usd: f64,
    pub percentage_used: f64,
    pub status: String,
    pub period_ends_at: Option<String>,
}

// ============================================================================
// TASK-022: Usage Tracking Commands
// ============================================================================

pub async fn get_usage_stats() -> Result<UsageStats, String> {
    invoke_no_args("get_usage_stats").await
}

pub async fn get_usage_by_period(hours: i64) -> Result<UsageStats, String> {
    #[derive(Serialize)]
    struct Args { hours: i64 }
    invoke("get_usage_by_period", &Args { hours }).await
}

pub async fn get_cost_breakdown(hours: Option<i64>) -> Result<CostBreakdown, String> {
    #[derive(Serialize)]
    struct Args { hours: Option<i64> }
    invoke("get_cost_breakdown", &Args { hours }).await
}

pub async fn get_budget_status() -> Result<Vec<BudgetStatus>, String> {
    invoke_no_args("get_budget_status").await
}

pub async fn set_budget_limit(limit: BudgetLimit) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args { limit: BudgetLimit }
    invoke_void("set_budget_limit", &Args { limit }).await
}

pub async fn get_provider_usage(provider: String) -> Result<ProviderUsage, String> {
    #[derive(Serialize)]
    struct Args { provider: String }
    invoke("get_provider_usage", &Args { provider }).await
}

pub async fn reset_usage_session() -> Result<(), String> {
    invoke_void_no_args("reset_usage_session").await
}

// ============================================================================
// TASK-023: Search Analytics Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAnalyticsSummary {
    pub total_searches: u32,
    pub zero_result_searches: u32,
    pub click_through_rate: f64,
    pub avg_results_per_search: f64,
    pub avg_execution_time_ms: f64,
    pub top_queries: Vec<(String, u32)>,
    pub failed_queries: Vec<String>,
    pub cache_stats: CacheStats,
    pub by_search_type: std::collections::HashMap<String, u32>,
    pub period_start: String,
    pub period_end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PopularQuery {
    pub query: String,
    pub count: u32,
    pub click_through_rate: f64,
    pub avg_result_count: f64,
    pub last_searched: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub avg_time_saved_ms: f64,
    pub total_time_saved_ms: u64,
    pub top_cached_queries: Vec<(String, u32)>,
}

// ============================================================================
// TASK-023: Search Analytics Commands
// ============================================================================

pub async fn get_search_analytics(hours: i64) -> Result<SearchAnalyticsSummary, String> {
    #[derive(Serialize)]
    struct Args { hours: i64 }
    invoke("get_search_analytics", &Args { hours }).await
}

pub async fn get_popular_queries(limit: usize) -> Result<Vec<PopularQuery>, String> {
    #[derive(Serialize)]
    struct Args { limit: usize }
    invoke("get_popular_queries", &Args { limit }).await
}

pub async fn get_cache_stats() -> Result<CacheStats, String> {
    invoke_no_args("get_cache_stats").await
}

pub async fn get_trending_queries(limit: usize) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args { limit: usize }
    invoke("get_trending_queries", &Args { limit }).await
}

pub async fn get_zero_result_queries(hours: i64) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args { hours: i64 }
    invoke("get_zero_result_queries", &Args { hours }).await
}

pub async fn get_click_distribution() -> Result<std::collections::HashMap<usize, u32>, String> {
    invoke_no_args("get_click_distribution").await
}

pub async fn record_search_selection(
    search_id: String,
    query: String,
    result_index: usize,
    source: String,
    selection_delay_ms: u64,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        search_id: String,
        query: String,
        result_index: usize,
        source: String,
        selection_delay_ms: u64,
    }
    invoke_void("record_search_selection", &Args {
        search_id, query, result_index, source, selection_delay_ms
    }).await
}

// --- Database-Backed Analytics (Persistent) ---

/// Get search analytics summary from database (persistent)
pub async fn get_search_analytics_db(hours: i64) -> Result<SearchAnalyticsSummary, String> {
    #[derive(Serialize)]
    struct Args { hours: i64 }
    invoke("get_search_analytics_db", &Args { hours }).await
}

/// Get popular queries from database (persistent)
pub async fn get_popular_queries_db(limit: usize) -> Result<Vec<PopularQuery>, String> {
    #[derive(Serialize)]
    struct Args { limit: usize }
    invoke("get_popular_queries_db", &Args { limit }).await
}

/// Get cache statistics from database (persistent)
pub async fn get_cache_stats_db() -> Result<CacheStats, String> {
    invoke_no_args("get_cache_stats_db").await
}

/// Get trending queries from database (persistent)
pub async fn get_trending_queries_db(limit: usize) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args { limit: usize }
    invoke("get_trending_queries_db", &Args { limit }).await
}

/// Get queries with zero results from database (persistent)
pub async fn get_zero_result_queries_db(hours: i64) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args { hours: i64 }
    invoke("get_zero_result_queries_db", &Args { hours }).await
}

/// Get click position distribution from database (persistent)
pub async fn get_click_distribution_db() -> Result<std::collections::HashMap<usize, u32>, String> {
    invoke_no_args("get_click_distribution_db").await
}

/// Record a search event (writes to both in-memory and database)
pub async fn record_search_event(
    query: String,
    result_count: usize,
    execution_time_ms: u64,
    search_type: String,
    from_cache: bool,
    source_filter: Option<String>,
    campaign_id: Option<String>,
) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
        result_count: usize,
        execution_time_ms: u64,
        search_type: String,
        from_cache: bool,
        source_filter: Option<String>,
        campaign_id: Option<String>,
    }
    invoke("record_search_event", &Args {
        query, result_count, execution_time_ms, search_type, from_cache, source_filter, campaign_id
    }).await
}

/// Record a result selection (writes to both in-memory and database)
pub async fn record_search_selection_db(
    search_id: String,
    query: String,
    result_index: usize,
    source: String,
    selection_delay_ms: u64,
    was_helpful: Option<bool>,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        search_id: String,
        query: String,
        result_index: usize,
        source: String,
        selection_delay_ms: u64,
        was_helpful: Option<bool>,
    }
    invoke_void("record_search_selection_db", &Args {
        search_id, query, result_index, source, selection_delay_ms, was_helpful
    }).await
}

/// Clean up old search analytics records
pub async fn cleanup_search_analytics(days: i64) -> Result<u64, String> {
    #[derive(Serialize)]
    struct Args { days: i64 }
    invoke("cleanup_search_analytics", &Args { days }).await
}

// ============================================================================
// TASK-024: Security Audit Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditEvent {
    pub id: String,
    pub event_type: serde_json::Value, // Using Value for the complex enum
    pub severity: String,
    pub timestamp: String,
    pub context: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

// ============================================================================
// TASK-024: Security Audit Commands
// ============================================================================

pub async fn get_audit_logs(
    count: Option<usize>,
    min_severity: Option<String>,
) -> Result<Vec<SecurityAuditEvent>, String> {
    #[derive(Serialize)]
    struct Args {
        count: Option<usize>,
        min_severity: Option<String>,
    }
    invoke("get_audit_logs", &Args { count, min_severity }).await
}

pub async fn query_audit_logs(
    from_hours: Option<i64>,
    min_severity: Option<String>,
    event_types: Option<Vec<String>>,
    search_text: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SecurityAuditEvent>, String> {
    #[derive(Serialize)]
    struct Args {
        from_hours: Option<i64>,
        min_severity: Option<String>,
        event_types: Option<Vec<String>>,
        search_text: Option<String>,
        limit: Option<usize>,
    }
    invoke("query_audit_logs", &Args {
        from_hours, min_severity, event_types, search_text, limit
    }).await
}

pub async fn export_audit_logs(format: String, from_hours: Option<i64>) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        format: String,
        from_hours: Option<i64>,
    }
    invoke("export_audit_logs", &Args { format, from_hours }).await
}

pub async fn clear_old_logs(days: i64) -> Result<usize, String> {
    #[derive(Serialize)]
    struct Args { days: i64 }
    invoke("clear_old_logs", &Args { days }).await
}

pub async fn get_audit_summary() -> Result<std::collections::HashMap<String, usize>, String> {
    invoke_no_args("get_audit_summary").await
}

pub async fn get_security_events() -> Result<Vec<SecurityAuditEvent>, String> {
    invoke_no_args("get_security_events").await
}

// ============================================================================
// Streaming Chat Types and Commands (TASK-003)
// ============================================================================

/// A single chunk from a streaming LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk {
    /// Unique ID for this stream
    pub stream_id: String,
    /// The content delta (partial text)
    pub content: String,
    /// Provider that generated this chunk
    pub provider: String,
    /// Model used
    pub model: String,
    /// Whether this is the final chunk
    pub is_final: bool,
    /// Finish reason if final (stop, length, error, etc.)
    pub finish_reason: Option<String>,
    /// Token usage (only present in final chunk)
    pub usage: Option<TokenUsage>,
    /// Chunk index in stream (for ordering)
    pub index: u32,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Chat message type for streaming requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingChatMessage {
    pub role: String,
    pub content: String,
}

impl StreamingChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }
}

/// Start a streaming chat session
/// Returns the stream_id immediately, chunks arrive via 'chat-chunk' events
pub async fn stream_chat(
    messages: Vec<StreamingChatMessage>,
    system_prompt: Option<String>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    provided_stream_id: Option<String>,
) -> Result<String, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        messages: Vec<StreamingChatMessage>,
        system_prompt: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
        provided_stream_id: Option<String>,
    }
    invoke("stream_chat", &Args {
        messages,
        system_prompt,
        temperature,
        max_tokens,
        provided_stream_id,
    }).await
}

/// Cancel an active streaming chat
pub async fn cancel_stream(stream_id: String) -> Result<bool, String> {
    #[derive(Serialize)]
    struct Args {
        stream_id: String,
    }
    invoke("cancel_stream", &Args { stream_id }).await
}

/// Get list of currently active stream IDs
pub async fn get_active_streams() -> Result<Vec<String>, String> {
    invoke_no_args("get_active_streams").await
}

/// Listen for streaming chat chunks (sync version - deprecated)
/// Returns a Promise that resolves to the unlisten function
/// The callback receives ChatChunk events from the backend
pub fn listen_chat_chunks<F>(callback: F) -> JsValue
where
    F: Fn(ChatChunk) + 'static,
{
    listen_event("chat-chunk", move |event| {
        // The event payload is wrapped in { payload: ChatChunk }
        match serde_wasm_bindgen::from_value::<StreamEventWrapper>(event.clone()) {
            Ok(wrapper) => callback(wrapper.payload),
            Err(e) => {
                let json_str = js_sys::JSON::stringify(&event).unwrap_or(js_sys::JsString::from("?"));
                web_sys::console::error_2(
                    &JsValue::from_str("Failed to deserialize chat-chunk event:"),
                    &e.into()
                );
                web_sys::console::log_2(&JsValue::from_str("Event data:"), &json_str);
            }
        }
    })
}

/// Listen for streaming chat chunks (async version for Tauri 2)
/// Awaits the Promise and returns the unlisten function
/// The callback receives ChatChunk events from the backend
pub async fn listen_chat_chunks_async<F>(callback: F) -> JsValue
where
    F: Fn(ChatChunk) + 'static,
{
    use wasm_bindgen_futures::JsFuture;

    #[cfg(debug_assertions)]
    web_sys::console::log_1(&"[DEBUG] listen_chat_chunks_async: Setting up listener...".into());

    let promise = listen_event("chat-chunk", move |event| {
        // The event payload is wrapped in { payload: ChatChunk }
        match serde_wasm_bindgen::from_value::<StreamEventWrapper>(event.clone()) {
            Ok(wrapper) => {
                callback(wrapper.payload);
            }
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to parse chat-chunk event: {:?}", e).into());
            }
        }
    });

    // Await the promise to get the unlisten function
    match JsFuture::from(js_sys::Promise::from(promise)).await {
        Ok(unlisten) => {
            #[cfg(debug_assertions)]
            web_sys::console::log_1(&"[DEBUG] Listener registered successfully!".into());
            unlisten
        }
        Err(e) => {
            web_sys::console::error_1(&format!("Failed to register chat listener: {:?}", e).into());
            JsValue::NULL
        }
    }
}

/// Wrapper for Tauri event payload
#[derive(Debug, Clone, Deserialize)]
struct StreamEventWrapper {
    payload: ChatChunk,
}

// ============================================================================
// Voice Profile Types (TASK-004)
// ============================================================================

/// Age range categories for voice profiles
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgeRange {
    Child,
    YoungAdult,
    Adult,
    MiddleAged,
    Elderly,
}

impl Default for AgeRange {
    fn default() -> Self {
        Self::Adult
    }
}

impl AgeRange {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Child => "Child (0-12)",
            Self::YoungAdult => "Young Adult (13-25)",
            Self::Adult => "Adult (26-45)",
            Self::MiddleAged => "Middle-Aged (46-65)",
            Self::Elderly => "Elderly (65+)",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Child,
            Self::YoungAdult,
            Self::Adult,
            Self::MiddleAged,
            Self::Elderly,
        ]
    }
}

/// Gender categories for voice profiles
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
    Neutral,
    NonBinary,
}

impl Default for Gender {
    fn default() -> Self {
        Self::Neutral
    }
}

impl Gender {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Male => "Male",
            Self::Female => "Female",
            Self::Neutral => "Neutral",
            Self::NonBinary => "Non-Binary",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::Male, Self::Female, Self::Neutral, Self::NonBinary]
    }
}

/// Metadata for voice profiles
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileMetadata {
    pub age_range: AgeRange,
    pub gender: Gender,
    pub personality_traits: Vec<String>,
    pub linked_npc_ids: Vec<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

/// Voice settings for synthesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSettings {
    pub stability: f32,
    pub similarity_boost: f32,
    pub style: f32,
    pub use_speaker_boost: bool,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            stability: 0.5,
            similarity_boost: 0.75,
            style: 0.0,
            use_speaker_boost: true,
        }
    }
}

/// Voice provider types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoiceProviderType {
    ElevenLabs,
    FishAudio,
    OpenAI,
    Piper,
    Ollama,
    Chatterbox,
    GptSoVits,
    XttsV2,
    FishSpeech,
    Dia,
    Coqui,
    System,
    Disabled,
}

impl Default for VoiceProviderType {
    fn default() -> Self {
        Self::Disabled
    }
}

impl VoiceProviderType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ElevenLabs => "ElevenLabs",
            Self::FishAudio => "Fish Audio (Cloud)",
            Self::OpenAI => "OpenAI TTS",
            Self::Ollama => "Ollama",
            Self::Chatterbox => "Chatterbox",
            Self::GptSoVits => "GPT-SoVITS",
            Self::XttsV2 => "XTTS-v2 (Coqui)",
            Self::FishSpeech => "Fish Speech",
            Self::Dia => "Dia",
            Self::Coqui => "Coqui TTS",
            Self::Piper => "Piper (Local)",
            Self::System => "System TTS",
            Self::Disabled => "Disabled",
        }
    }

    pub fn to_string_key(&self) -> &'static str {
        match self {
            Self::ElevenLabs => "elevenlabs",
            Self::FishAudio => "fish_audio",
            Self::OpenAI => "openai",
            Self::Ollama => "ollama",
            Self::Chatterbox => "chatterbox",
            Self::GptSoVits => "gpt_sovits",
            Self::XttsV2 => "xtts_v2",
            Self::FishSpeech => "fish_speech",
            Self::Dia => "dia",
            Self::Coqui => "coqui",
            Self::Piper => "piper",
            Self::System => "system",
            Self::Disabled => "disabled",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::OpenAI,
            Self::ElevenLabs,
            Self::FishAudio,
            Self::Piper,
            Self::Coqui,
            Self::Ollama,
            Self::Chatterbox,
            Self::GptSoVits,
            Self::XttsV2,
            Self::FishSpeech,
            Self::Dia,
        ]
    }
}

/// A complete voice profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceProfile {
    pub id: String,
    pub name: String,
    pub provider: VoiceProviderType,
    pub voice_id: String,
    pub settings: VoiceSettings,
    pub metadata: ProfileMetadata,
    pub is_preset: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Statistics about voice profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileStats {
    pub total_user_profiles: usize,
    pub total_presets: usize,
    pub linked_npcs: usize,
    pub profiles_by_provider: std::collections::HashMap<String, usize>,
    pub profiles_by_gender: std::collections::HashMap<String, usize>,
}

// ============================================================================
// Voice Profile Commands (TASK-004)
// ============================================================================

/// List all voice profile presets (built-in DM personas)
pub async fn list_voice_presets() -> Result<Vec<VoiceProfile>, String> {
    invoke_no_args("list_voice_presets").await
}

/// List voice presets filtered by tag
pub async fn list_voice_presets_by_tag(tag: String) -> Result<Vec<VoiceProfile>, String> {
    #[derive(Serialize)]
    struct Args {
        tag: String,
    }
    invoke("list_voice_presets_by_tag", &Args { tag }).await
}

/// Get a specific voice preset by ID
pub async fn get_voice_preset(preset_id: String) -> Result<Option<VoiceProfile>, String> {
    #[derive(Serialize)]
    struct Args {
        preset_id: String,
    }
    invoke("get_voice_preset", &Args { preset_id }).await
}

/// Create a new voice profile
pub async fn create_voice_profile(
    name: String,
    provider: String,
    voice_id: String,
    metadata: Option<ProfileMetadata>,
) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        name: String,
        provider: String,
        voice_id: String,
        metadata: Option<ProfileMetadata>,
    }
    invoke("create_voice_profile", &Args { name, provider, voice_id, metadata }).await
}

/// Link a voice profile to an NPC
pub async fn link_voice_profile_to_npc(profile_id: String, npc_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        profile_id: String,
        npc_id: String,
    }
    invoke_void("link_voice_profile_to_npc", &Args { profile_id, npc_id }).await
}

/// Get the voice profile linked to an NPC
pub async fn get_npc_voice_profile(npc_id: String) -> Result<Option<String>, String> {
    #[derive(Serialize)]
    struct Args {
        npc_id: String,
    }
    invoke("get_npc_voice_profile", &Args { npc_id }).await
}

/// Search voice profiles by query
pub async fn search_voice_profiles(query: String) -> Result<Vec<VoiceProfile>, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
    }
    invoke("search_voice_profiles", &Args { query }).await
}

/// Get voice profiles by gender
pub async fn get_voice_profiles_by_gender(gender: String) -> Result<Vec<VoiceProfile>, String> {
    #[derive(Serialize)]
    struct Args {
        gender: String,
    }
    invoke("get_voice_profiles_by_gender", &Args { gender }).await
}

/// Get voice profiles by age range
pub async fn get_voice_profiles_by_age(age_range: String) -> Result<Vec<VoiceProfile>, String> {
    #[derive(Serialize)]
    struct Args {
        age_range: String,
    }
    invoke("get_voice_profiles_by_age", &Args { age_range }).await
}

// ============================================================================
// Audio Cache Commands (TASK-005)
// ============================================================================

/// Audio cache statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoiceCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub entry_count: usize,
    pub current_size_bytes: u64,
    pub max_size_bytes: u64,
    pub entries_by_format: std::collections::HashMap<String, usize>,
    pub hit_rate: f64,
    pub oldest_entry_age_secs: i64,
    pub avg_entry_size_bytes: u64,
}

/// Get audio cache statistics
pub async fn get_audio_cache_stats() -> Result<VoiceCacheStats, String> {
    invoke_no_args("get_audio_cache_stats").await
}

/// Clear audio cache entries by tag
pub async fn clear_audio_cache_by_tag(tag: String) -> Result<usize, String> {
    #[derive(Serialize)]
    struct Args {
        tag: String,
    }
    invoke("clear_audio_cache_by_tag", &Args { tag }).await
}

/// Clear all audio cache entries
pub async fn clear_audio_cache() -> Result<(), String> {
    invoke_void_no_args("clear_audio_cache").await
}

/// Prune old audio cache entries
pub async fn prune_audio_cache(max_age_seconds: i64) -> Result<usize, String> {
    #[derive(Serialize)]
    struct Args {
        max_age_seconds: i64,
    }
    invoke("prune_audio_cache", &Args { max_age_seconds }).await
}

/// Cache entry information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioCacheEntry {
    pub key: String,
    pub path: String,
    pub size: u64,
    pub created_at: String,
    pub last_accessed: String,
    pub access_count: u32,
    pub tags: Vec<String>,
    pub profile_id: Option<String>,
    pub duration_ms: Option<u64>,
    pub format: String,
}

/// List all cached audio entries
pub async fn list_audio_cache_entries() -> Result<Vec<AudioCacheEntry>, String> {
    invoke_no_args("list_audio_cache_entries").await
}

/// Audio cache size information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioCacheSizeInfo {
    pub current_size_bytes: u64,
    pub max_size_bytes: u64,
    pub entry_count: usize,
    pub usage_percent: f64,
}

/// Get audio cache size information
pub async fn get_audio_cache_size() -> Result<AudioCacheSizeInfo, String> {
    invoke_no_args("get_audio_cache_size").await
}

/// Format bytes to human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

// ============================================================================
// Personality Application Bindings (TASK-021)
// ============================================================================

/// Scene mood that modifies personality application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneMood {
    pub tone: String,
    pub intensity: u8,
    pub description: String,
}

/// Personality settings for narrative style
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersonalitySettings {
    pub tone: String,
    pub vocabulary: String,
    pub narrative_style: String,
    pub verbosity: String,
    pub genre: String,
    pub custom_patterns: Vec<String>,
    pub use_dialect: bool,
    pub dialect: Option<String>,
}

/// Active personality context for a campaign or session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePersonalityContext {
    pub campaign_id: String,
    pub session_id: Option<String>,
    pub narrator_personality_id: Option<String>,
    pub npc_personalities: std::collections::HashMap<String, String>,
    pub location_personalities: std::collections::HashMap<String, String>,
    pub scene_mood: Option<SceneMood>,
    pub active: bool,
    pub settings: PersonalitySettings,
    pub updated_at: String,
}

/// Personality preview for selector UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityPreview {
    pub personality_id: String,
    pub personality_name: String,
    pub sample_greetings: Vec<String>,
    pub sample_responses: std::collections::HashMap<String, String>,
    pub characteristics: Vec<String>,
}

/// Extended personality preview with full details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedPersonalityPreview {
    pub basic: PersonalityPreview,
    pub system_prompt_preview: String,
    pub example_phrases: Vec<String>,
    pub knowledge_areas: Vec<String>,
    pub tags: Vec<String>,
    pub source: Option<String>,
}

/// Quick preview response for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewResponse {
    pub personality_id: String,
    pub personality_name: String,
    pub sample_greeting: String,
    pub formality_level: u8,
    pub key_traits: Vec<String>,
}

/// Styled content result from personality application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyledContent {
    pub content: String,
    pub personality_id: Option<String>,
    pub style_notes: Vec<String>,
}

/// Request to set active personality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetActivePersonalityRequest {
    pub session_id: String,
    pub personality_id: Option<String>,
    pub campaign_id: String,
}

/// Request to update personality settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalitySettingsRequest {
    pub campaign_id: String,
    pub tone: Option<String>,
    pub vocabulary: Option<String>,
    pub narrative_style: Option<String>,
    pub verbosity: Option<String>,
    pub genre: Option<String>,
    pub custom_patterns: Option<Vec<String>>,
    pub use_dialect: Option<bool>,
    pub dialect: Option<String>,
}

/// Set the active personality for a session
pub async fn set_active_personality(request: SetActivePersonalityRequest) -> Result<(), String> {
    invoke_void("set_active_personality", &json!({
        "request": request
    })).await
}

/// Get the active personality ID for a session
pub async fn get_active_personality(session_id: String, campaign_id: String) -> Result<Option<String>, String> {
    invoke("get_active_personality", &json!({
        "session_id": session_id,
        "campaign_id": campaign_id
    })).await
}

/// Get the system prompt for a personality
pub async fn get_personality_prompt(personality_id: String) -> Result<String, String> {
    invoke("get_personality_prompt", &json!({
        "personality_id": personality_id
    })).await
}

/// Apply personality styling to text using LLM transformation
pub async fn apply_personality_to_text(text: String, personality_id: String) -> Result<String, String> {
    invoke("apply_personality_to_text", &json!({
        "text": text,
        "personality_id": personality_id
    })).await
}

/// Get personality context for a campaign
pub async fn get_personality_context(campaign_id: String) -> Result<ActivePersonalityContext, String> {
    invoke("get_personality_context", &json!({
        "campaign_id": campaign_id
    })).await
}

/// Get personality context for a session
pub async fn get_session_personality_context(session_id: String, campaign_id: String) -> Result<ActivePersonalityContext, String> {
    invoke("get_session_personality_context", &json!({
        "session_id": session_id,
        "campaign_id": campaign_id
    })).await
}

/// Update personality context for a campaign
pub async fn set_personality_context(context: ActivePersonalityContext) -> Result<(), String> {
    invoke_void("set_personality_context", &json!({
        "context": context
    })).await
}

/// Set the narrator personality for a campaign
pub async fn set_narrator_personality(campaign_id: String, personality_id: Option<String>) -> Result<(), String> {
    invoke_void("set_narrator_personality", &json!({
        "campaign_id": campaign_id,
        "personality_id": personality_id
    })).await
}

/// Assign a personality to an NPC
pub async fn assign_npc_personality(campaign_id: String, npc_id: String, personality_id: String) -> Result<(), String> {
    invoke_void("assign_npc_personality", &json!({
        "campaign_id": campaign_id,
        "npc_id": npc_id,
        "personality_id": personality_id
    })).await
}

/// Unassign personality from an NPC
pub async fn unassign_npc_personality(campaign_id: String, npc_id: String) -> Result<(), String> {
    invoke_void("unassign_npc_personality", &json!({
        "campaign_id": campaign_id,
        "npc_id": npc_id
    })).await
}

/// Set scene mood for a campaign
pub async fn set_scene_mood(campaign_id: String, mood: Option<SceneMood>) -> Result<(), String> {
    invoke_void("set_scene_mood", &json!({
        "campaign_id": campaign_id,
        "mood": mood
    })).await
}

/// Update personality settings for a campaign
pub async fn set_personality_settings(request: PersonalitySettingsRequest) -> Result<(), String> {
    invoke_void("set_personality_settings", &json!({
        "request": request
    })).await
}

/// Toggle personality application on/off
pub async fn set_personality_active(campaign_id: String, active: bool) -> Result<(), String> {
    invoke_void("set_personality_active", &json!({
        "campaign_id": campaign_id,
        "active": active
    })).await
}

/// Preview a personality
pub async fn preview_personality(personality_id: String) -> Result<PersonalityPreview, String> {
    invoke("preview_personality", &json!({
        "personality_id": personality_id
    })).await
}

/// Get extended personality preview with full details
pub async fn preview_personality_extended(personality_id: String) -> Result<ExtendedPersonalityPreview, String> {
    invoke("preview_personality_extended", &json!({
        "personality_id": personality_id
    })).await
}

/// Generate a preview response for personality selection UI
pub async fn generate_personality_preview(personality_id: String) -> Result<PreviewResponse, String> {
    invoke("generate_personality_preview", &json!({
        "personality_id": personality_id
    })).await
}

/// Test a personality by generating a response
pub async fn test_personality(personality_id: String, test_prompt: String) -> Result<String, String> {
    invoke("test_personality", &json!({
        "personality_id": personality_id,
        "test_prompt": test_prompt
    })).await
}

/// Get the session system prompt with personality applied
pub async fn get_session_system_prompt(session_id: String, campaign_id: String, content_type: String) -> Result<String, String> {
    invoke("get_session_system_prompt", &json!({
        "session_id": session_id,
        "campaign_id": campaign_id,
        "content_type": content_type
    })).await
}

/// Style NPC dialogue with personality
pub async fn style_npc_dialogue(npc_id: String, campaign_id: String, raw_dialogue: String) -> Result<StyledContent, String> {
    invoke("style_npc_dialogue", &json!({
        "npc_id": npc_id,
        "campaign_id": campaign_id,
        "raw_dialogue": raw_dialogue
    })).await
}

/// Build NPC system prompt with personality
pub async fn build_npc_system_prompt(npc_id: String, campaign_id: String, additional_context: Option<String>) -> Result<String, String> {
    invoke("build_npc_system_prompt", &json!({
        "npc_id": npc_id,
        "campaign_id": campaign_id,
        "additional_context": additional_context
    })).await
}

/// Build narration prompt with personality
pub async fn build_narration_prompt(campaign_id: String, narration_type: String) -> Result<String, String> {
    invoke("build_narration_prompt", &json!({
        "campaign_id": campaign_id,
        "narration_type": narration_type
    })).await
}

/// List all available personalities from the store
pub async fn list_personalities() -> Result<Vec<PersonalityPreview>, String> {
    invoke_no_args("list_personalities").await
}

/// Clear session-specific personality context
pub async fn clear_session_personality_context(session_id: String) -> Result<(), String> {
    invoke_void("clear_session_personality_context", &json!({
        "session_id": session_id
    })).await
}

// ============================================================================
// Gemini CLI Status & Extension
// ============================================================================

/// Status of Gemini CLI installation and authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiCliStatus {
    pub is_installed: bool,
    pub is_authenticated: bool,
    pub message: String,
}

/// Status of Gemini CLI extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiCliExtensionStatus {
    pub is_installed: bool,
    pub message: String,
}

/// Check Gemini CLI installation and authentication status
pub async fn check_gemini_cli_status() -> Result<GeminiCliStatus, String> {
    invoke_no_args("check_gemini_cli_status").await
}

/// Launch Gemini CLI for authentication
pub async fn launch_gemini_cli_login() -> Result<(), String> {
    invoke_no_args("launch_gemini_cli_login").await
}

/// Check if the Sidecar DM extension is installed
pub async fn check_gemini_cli_extension() -> Result<GeminiCliExtensionStatus, String> {
    invoke_no_args("check_gemini_cli_extension").await
}

/// Install the Sidecar DM extension from a source
pub async fn install_gemini_cli_extension(source: String) -> Result<String, String> {
    invoke("install_gemini_cli_extension", &json!({ "source": source })).await
}

/// Link a local extension directory for development
pub async fn link_gemini_cli_extension(path: String) -> Result<String, String> {
    invoke("link_gemini_cli_extension", &json!({ "path": path })).await
}

/// Uninstall the Sidecar DM extension
pub async fn uninstall_gemini_cli_extension() -> Result<String, String> {
    invoke_no_args("uninstall_gemini_cli_extension").await
}

// ============================================================================
// LLM Proxy / Meilisearch Chat Configuration
// ============================================================================

/// Configure Meilisearch chat workspace with an LLM provider
///
/// This sets up the Meilisearch chat workspace to use the specified provider.
/// For non-OpenAI providers (Claude, Gemini, etc.), requests are routed through
/// the local LLM proxy service.
///
/// # Arguments
/// * `provider` - The LLM provider to use (e.g., "claude", "openai", "gemini", "ollama")
/// * `api_key` - Optional API key (if not already stored)
/// * `model` - Optional model override (uses provider default if not specified)
/// * `custom_system_prompt` - Optional custom system prompt for the DM
/// * `host` - Optional host URL for Ollama (defaults to http://localhost:11434)
pub async fn configure_meilisearch_chat(
    provider: String,
    api_key: Option<String>,
    model: Option<String>,
    custom_system_prompt: Option<String>,
    host: Option<String>,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        provider: String,
        api_key: Option<String>,
        model: Option<String>,
        custom_system_prompt: Option<String>,
        host: Option<String>,
    }
    invoke_void(
        "configure_meilisearch_chat",
        &Args {
            provider,
            api_key,
            model,
            custom_system_prompt,
            host,
        },
    )
    .await
}

/// Get the URL of the local LLM proxy service
///
/// Returns the URL that Meilisearch uses to communicate with non-native providers.
/// Typically `http://127.0.0.1:8787` or similar.
pub async fn get_llm_proxy_url() -> Result<String, String> {
    invoke_no_args("get_llm_proxy_url").await
}

/// Check if the LLM proxy service is running and healthy
///
/// Returns `true` if the proxy is active and can accept requests.
pub async fn get_llm_proxy_status() -> Result<bool, String> {
    invoke_no_args("get_llm_proxy_status").await
}

/// Check if the LLM proxy is running
///
/// Returns `true` if the proxy server is active.
pub async fn is_llm_proxy_running() -> Result<bool, String> {
    invoke_no_args("is_llm_proxy_running").await
}

/// List providers registered with the LLM proxy
///
/// Returns a list of provider names that are currently registered with the proxy.
pub async fn list_proxy_providers() -> Result<Vec<String>, String> {
    invoke_no_args("list_proxy_providers").await
}

// ============================================================================
// Model Selection (Claude Code Smart Model Selection)
// ============================================================================

/// Usage data from Anthropic API (rate limit utilization)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageData {
    /// 5-hour window utilization (0.0 - 1.0)
    pub five_hour_util: f64,
    /// 7-day window utilization (0.0 - 1.0)
    pub seven_day_util: f64,
    /// When the 5-hour window resets (ISO 8601)
    #[serde(default)]
    pub five_hour_resets_at: Option<String>,
    /// When the 7-day window resets (ISO 8601)
    #[serde(default)]
    pub seven_day_resets_at: Option<String>,
    /// Unix timestamp when this data was cached
    pub cached_at: u64,
}

/// Model selection result from the smart model selector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelection {
    /// Full model ID (e.g., "claude-opus-4-20250514")
    pub model: String,
    /// Short model name (e.g., "opus", "sonnet")
    pub model_short: String,
    /// Subscription plan (e.g., "max_5x", "pro", "free")
    pub plan: String,
    /// Auth type ("oauth", "api", "none")
    pub auth_type: String,
    /// Current usage data
    pub usage: UsageData,
    /// Detected task complexity ("light", "medium", "heavy")
    pub complexity: String,
    /// Human-readable selection reason
    pub selection_reason: String,
    /// Whether a manual override is active
    pub override_active: bool,
}

/// Get the current model selection (uses default complexity)
pub async fn get_model_selection() -> Result<ModelSelection, String> {
    invoke_no_args("get_model_selection").await
}

/// Get model selection for a specific prompt (analyzes complexity)
pub async fn get_model_selection_for_prompt(prompt: String) -> Result<ModelSelection, String> {
    #[derive(Serialize)]
    struct Args {
        prompt: String,
    }
    invoke("get_model_selection_for_prompt", &Args { prompt }).await
}

/// Set or clear a manual model override
///
/// Pass `Some(model_id)` to force a specific model, or `None` to clear the override.
pub async fn set_model_override(model: Option<String>) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        model: Option<String>,
    }
    invoke_void("set_model_override", &Args { model }).await
}
