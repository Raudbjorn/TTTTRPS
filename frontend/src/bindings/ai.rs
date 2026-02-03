use super::core::{invoke, invoke_no_args, invoke_void, listen_event};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// ============================================================================
// LLM Types
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
    pub storage_backend: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub provider: String,
    pub healthy: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: Option<String>,
    pub parameter_size: Option<String>,
}

/// Generic model info for cloud providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
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

pub async fn list_ollama_models(host: String) -> Result<Vec<OllamaModel>, String> {
    #[derive(Serialize)]
    struct Args {
        host: String,
    }
    invoke("list_ollama_models", &Args { host }).await
}

pub async fn list_anthropic_models(api_key: Option<String>) -> Result<Vec<ModelInfo>, String> {
    #[derive(Serialize)]
    struct Args {
        api_key: Option<String>,
    }
    invoke("list_anthropic_models", &Args { api_key }).await
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

// ============================================================================
// LLM Proxy / Meilisearch Chat Configuration
// ============================================================================

/// Configure Meilisearch chat workspace with an LLM provider
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

pub async fn get_llm_proxy_url() -> Result<String, String> {
    invoke_no_args("get_llm_proxy_url").await
}

pub async fn get_llm_proxy_status() -> Result<bool, String> {
    invoke_no_args("get_llm_proxy_status").await
}

pub async fn is_llm_proxy_running() -> Result<bool, String> {
    invoke_no_args("is_llm_proxy_running").await
}

pub async fn list_proxy_providers() -> Result<Vec<String>, String> {
    invoke_no_args("list_proxy_providers").await
}

// ============================================================================
// Global Chat Session Commands (Persistent LLM Chat History)
// ============================================================================

/// Global chat session record (matching backend)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalChatSession {
    pub id: String,
    pub status: String,
    pub linked_game_session_id: Option<String>,
    pub linked_campaign_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Chat message record (matching backend)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageRecord {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub tokens_input: Option<i32>,
    pub tokens_output: Option<i32>,
    pub is_streaming: bool,
    pub metadata: Option<String>,
    pub created_at: String,
}

/// Get or create the active global chat session
pub async fn get_or_create_chat_session() -> Result<GlobalChatSession, String> {
    invoke_no_args("get_or_create_chat_session").await
}

/// Get the current active chat session (if any)
pub async fn get_active_chat_session() -> Result<Option<GlobalChatSession>, String> {
    invoke_no_args("get_active_chat_session").await
}

/// Get messages for a chat session
pub async fn get_chat_messages(
    session_id: String,
    limit: Option<i32>,
) -> Result<Vec<ChatMessageRecord>, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        limit: Option<i32>,
    }
    invoke("get_chat_messages", &Args { session_id, limit }).await
}

/// Add a message to the chat session
pub async fn add_chat_message(
    session_id: String,
    role: String,
    content: String,
    tokens: Option<(i32, i32)>,
) -> Result<ChatMessageRecord, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        role: String,
        content: String,
        tokens: Option<(i32, i32)>,
    }
    invoke(
        "add_chat_message",
        &Args {
            session_id,
            role,
            content,
            tokens,
        },
    )
    .await
}

/// Update a chat message (e.g., after streaming completes)
pub async fn update_chat_message(
    message_id: String,
    content: String,
    tokens: Option<(i32, i32)>,
    is_streaming: bool,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        message_id: String,
        content: String,
        tokens: Option<(i32, i32)>,
        is_streaming: bool,
    }
    invoke_void(
        "update_chat_message",
        &Args {
            message_id,
            content,
            tokens,
            is_streaming,
        },
    )
    .await
}

/// Link the current chat session to a game session
pub async fn link_chat_to_game_session(
    chat_session_id: String,
    game_session_id: String,
    campaign_id: Option<String>,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        chat_session_id: String,
        game_session_id: String,
        campaign_id: Option<String>,
    }
    invoke_void(
        "link_chat_to_game_session",
        &Args {
            chat_session_id,
            game_session_id,
            campaign_id,
        },
    )
    .await
}

/// Archive the current chat session and create a new one
pub async fn end_chat_session_and_spawn_new(
    chat_session_id: String,
) -> Result<GlobalChatSession, String> {
    #[derive(Serialize)]
    struct Args {
        chat_session_id: String,
    }
    invoke("end_chat_session_and_spawn_new", &Args { chat_session_id }).await
}

/// Clear all messages in a chat session
pub async fn clear_chat_messages(session_id: String) -> Result<u64, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke("clear_chat_messages", &Args { session_id }).await
}

/// List chat sessions (for history view)
pub async fn list_chat_sessions(limit: Option<i32>) -> Result<Vec<GlobalChatSession>, String> {
    #[derive(Serialize)]
    struct Args {
        limit: Option<i32>,
    }
    invoke("list_chat_sessions", &Args { limit }).await
}

/// Get chat sessions linked to a specific game session
pub async fn get_chat_sessions_for_game(
    game_session_id: String,
) -> Result<Vec<GlobalChatSession>, String> {
    #[derive(Serialize)]
    struct Args {
        game_session_id: String,
    }
    invoke("get_chat_sessions_for_game", &Args { game_session_id }).await
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
pub async fn set_model_override(model: Option<String>) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        model: Option<String>,
    }
    invoke_void("set_model_override", &Args { model }).await
}

// ============================================================================
// Streaming Chat Types and Commands
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
    invoke(
        "stream_chat",
        &Args {
            messages,
            system_prompt,
            temperature,
            max_tokens,
            provided_stream_id,
        },
    )
    .await
}

/// Start a streaming NPC chat session
/// Uses NPC personality for system prompt and persists to NPC conversation
///
/// # Arguments
/// * `npc_id` - The NPC to chat with
/// * `user_message` - The user's message
/// * `mode` - "about" for DM assistant mode, "voice" (default) for roleplay mode
/// * `provided_stream_id` - Optional stream ID for tracking
pub async fn stream_npc_chat(
    npc_id: String,
    user_message: String,
    mode: Option<String>,
    provided_stream_id: Option<String>,
) -> Result<String, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        npc_id: String,
        user_message: String,
        mode: Option<String>,
        provided_stream_id: Option<String>,
    }
    invoke(
        "stream_npc_chat",
        &Args {
            npc_id,
            user_message,
            mode,
            provided_stream_id,
        },
    )
    .await
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

/// Wrapper for Tauri event payload
#[derive(Debug, Clone, Deserialize)]
struct StreamEventWrapper {
    payload: ChatChunk,
}

/// Listen for streaming chat chunks (sync version - deprecated)
pub fn listen_chat_chunks<F>(callback: F) -> JsValue
where
    F: Fn(ChatChunk) + 'static,
{
    listen_event(
        "chat-chunk",
        move |event| match serde_wasm_bindgen::from_value::<StreamEventWrapper>(event.clone()) {
            Ok(wrapper) => callback(wrapper.payload),
            Err(e) => {
                let json_str =
                    js_sys::JSON::stringify(&event).unwrap_or(js_sys::JsString::from("?"));
                web_sys::console::error_2(
                    &JsValue::from_str("Failed to deserialize chat-chunk event:"),
                    &e.into(),
                );
                web_sys::console::log_2(&JsValue::from_str("Event data:"), &json_str);
            }
        },
    )
}

/// Listen for streaming chat chunks (async version for Tauri 2)
pub async fn listen_chat_chunks_async<F>(callback: F) -> JsValue
where
    F: Fn(ChatChunk) + 'static,
{
    use wasm_bindgen_futures::JsFuture;

    #[cfg(debug_assertions)]
    web_sys::console::log_1(&"[DEBUG] listen_chat_chunks_async: Setting up listener...".into());

    let promise = listen_event(
        "chat-chunk",
        move |event| match serde_wasm_bindgen::from_value::<StreamEventWrapper>(event.clone()) {
            Ok(wrapper) => {
                callback(wrapper.payload);
            }
            Err(e) => {
                let json_str = js_sys::JSON::stringify(&event).unwrap_or_else(|_| "?".into());
                web_sys::console::error_2(
                    &"Failed to deserialize chat-chunk event:".into(),
                    &e.into(),
                );
                web_sys::console::log_2(&"Event data:".into(), &json_str);
            }
        },
    );

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
