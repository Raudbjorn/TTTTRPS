//! LLM Proxy Service
//!
//! Provides an OpenAI-compatible HTTP endpoint that routes requests to
//! any of the project's LLM providers. This allows Meilisearch's chat
//! feature to use providers like Claude, Gemini, etc. via the VLlm source.
//!
//! ## Endpoints
//! - `POST /v1/chat/completions` - OpenAI-compatible chat endpoint
//! - `GET /v1/models` - List available models
//! - `GET /health` - Health check
//!
//! ## Routing
//! Provider selection via model name prefix: `{provider}:{model}`
//! Examples: `claude:claude-sonnet-4-20250514`, `gemini:gemini-pro`

use super::router::{ChatMessage, ChatRequest, LLMError, LLMProvider, MessageRole};
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{sse::Event, IntoResponse, Response, Sse},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};
use tower_http::cors::{Any, CorsLayer};

// ============================================================================
// OpenAI-Compatible Types
// ============================================================================

/// OpenAI-compatible chat completion request
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

/// OpenAI-compatible message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIMessage {
    pub role: String,
    pub content: String,
}

impl From<OpenAIMessage> for ChatMessage {
    fn from(msg: OpenAIMessage) -> Self {
        let role = match msg.role.as_str() {
            "system" => MessageRole::System,
            "assistant" => MessageRole::Assistant,
            _ => MessageRole::User,
        };
        ChatMessage {
            role,
            content: msg.content,
            images: None,
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

/// OpenAI-compatible chat completion response
#[derive(Debug, Clone, Serialize)]
pub struct OpenAIChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<OpenAIUsage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIChoice {
    pub index: u32,
    pub message: OpenAIMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// OpenAI-compatible streaming chunk
#[derive(Debug, Clone, Serialize)]
pub struct OpenAIStreamChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIStreamChoice>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIStreamChoice {
    pub index: u32,
    pub delta: OpenAIDelta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Model info for /v1/models endpoint
#[derive(Debug, Clone, Serialize)]
pub struct OpenAIModel {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIModelList {
    pub object: String,
    pub data: Vec<OpenAIModel>,
}

// ============================================================================
// Proxy Service State
// ============================================================================

/// Shared state for the proxy service
pub struct ProxyState {
    /// Registered providers keyed by provider ID
    pub providers: RwLock<HashMap<String, Arc<dyn LLMProvider>>>,
    /// Default provider ID (used when no prefix in model name)
    pub default_provider: RwLock<Option<String>>,
}

impl ProxyState {
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            default_provider: RwLock::new(None),
        }
    }

    /// Parse model name to extract provider and actual model
    /// Format: "provider:model" or just "model" (uses default)
    pub async fn parse_model(&self, model: &str) -> Option<(String, String)> {
        if let Some((provider, actual_model)) = model.split_once(':') {
            Some((provider.to_string(), actual_model.to_string()))
        } else {
            // Use default provider if set
            let default = self.default_provider.read().await;
            default.as_ref().map(|p| (p.clone(), model.to_string()))
        }
    }

    /// Get a provider by ID
    pub async fn get_provider(&self, provider_id: &str) -> Option<Arc<dyn LLMProvider>> {
        let providers = self.providers.read().await;
        providers.get(provider_id).cloned()
    }
}

impl Default for ProxyState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// LLM Proxy Service
// ============================================================================

/// OpenAI-compatible LLM proxy service
pub struct LLMProxyService {
    port: u16,
    state: Arc<ProxyState>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl LLMProxyService {
    /// Create a new proxy service on the specified port
    pub fn new(port: u16) -> Self {
        Self {
            port,
            state: Arc::new(ProxyState::new()),
            shutdown_tx: None,
        }
    }

    /// Create with default port (8787)
    pub fn with_defaults() -> Self {
        Self::new(8787)
    }

    /// Get the proxy URL
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Register a provider
    pub async fn register_provider(&self, id: &str, provider: Arc<dyn LLMProvider>) {
        let mut providers = self.state.providers.write().await;
        log::info!("Registered LLM proxy provider: {} (model: {})", id, provider.model());
        providers.insert(id.to_string(), provider);
    }

    /// Unregister a provider
    pub async fn unregister_provider(&self, id: &str) {
        let mut providers = self.state.providers.write().await;
        if providers.remove(id).is_some() {
            log::info!("Unregistered LLM proxy provider: {}", id);
        }
    }

    /// Set the default provider
    pub async fn set_default_provider(&self, id: &str) {
        let mut default = self.state.default_provider.write().await;
        *default = Some(id.to_string());
        log::info!("Set default LLM proxy provider: {}", id);
    }

    /// List registered providers
    pub async fn list_providers(&self) -> Vec<String> {
        let providers = self.state.providers.read().await;
        providers.keys().cloned().collect()
    }

    /// Start the proxy service
    pub async fn start(&mut self) -> Result<(), String> {
        if self.shutdown_tx.is_some() {
            return Err("Proxy already running".to_string());
        }

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let state = self.state.clone();
        let port = self.port;

        // Build router
        let app = Router::new()
            .route("/v1/chat/completions", post(chat_completions))
            .route("/v1/models", get(list_models))
            .route("/health", get(health_check))
            .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
            .with_state(state);

        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        // Spawn server task
        tokio::spawn(async move {
            let listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => l,
                Err(e) => {
                    log::error!("Failed to bind LLM proxy to {}: {}", addr, e);
                    return;
                }
            };

            log::info!("LLM proxy service started on http://{}", addr);

            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                    log::info!("LLM proxy service shutting down");
                })
                .await
                .ok();
        });

        self.shutdown_tx = Some(shutdown_tx);
        Ok(())
    }

    /// Stop the proxy service
    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
            log::info!("LLM proxy service stopped");
        }
    }

    /// Check if the service is running
    pub fn is_running(&self) -> bool {
        self.shutdown_tx.is_some()
    }
}

// ============================================================================
// HTTP Handlers
// ============================================================================

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

/// List models endpoint
async fn list_models(State(state): State<Arc<ProxyState>>) -> impl IntoResponse {
    let providers = state.providers.read().await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let models: Vec<OpenAIModel> = providers
        .iter()
        .map(|(id, provider)| OpenAIModel {
            id: format!("{}:{}", id, provider.model()),
            object: "model".to_string(),
            created: now,
            owned_by: provider.name().to_string(),
        })
        .collect();

    Json(OpenAIModelList {
        object: "list".to_string(),
        data: models,
    })
}

/// Chat completions endpoint
async fn chat_completions(
    State(state): State<Arc<ProxyState>>,
    Json(request): Json<OpenAIChatRequest>,
) -> Response {
    // Parse provider from model name
    let (provider_id, _actual_model) = match state.parse_model(&request.model).await {
        Some(parsed) => parsed,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": {
                        "message": "Invalid model format. Use 'provider:model' or set a default provider.",
                        "type": "invalid_request_error"
                    }
                })),
            )
                .into_response();
        }
    };

    // Get provider
    let provider = match state.get_provider(&provider_id).await {
        Some(p) => p,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": {
                        "message": format!("Provider '{}' not found", provider_id),
                        "type": "invalid_request_error"
                    }
                })),
            )
                .into_response();
        }
    };

    // Convert messages
    let messages: Vec<ChatMessage> = request.messages.into_iter().map(|m| m.into()).collect();

    // Build internal request
    let chat_request = ChatRequest {
        messages,
        temperature: request.temperature,
        max_tokens: request.max_tokens,
        system_prompt: None,
        provider: None,
        tools: None,
        tool_choice: None,
    };

    if request.stream {
        // Streaming response
        handle_streaming(provider, chat_request, request.model).await
    } else {
        // Non-streaming response
        handle_non_streaming(provider, chat_request, request.model).await
    }
}

/// Handle non-streaming chat request
async fn handle_non_streaming(
    provider: Arc<dyn LLMProvider>,
    request: ChatRequest,
    model: String,
) -> Response {
    match provider.chat(request).await {
        Ok(response) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let usage = response.usage.map(|u| OpenAIUsage {
                prompt_tokens: u.input_tokens,
                completion_tokens: u.output_tokens,
                total_tokens: u.input_tokens + u.output_tokens,
            });

            let openai_response = OpenAIChatResponse {
                id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                object: "chat.completion".to_string(),
                created: now,
                model,
                choices: vec![OpenAIChoice {
                    index: 0,
                    message: OpenAIMessage {
                        role: "assistant".to_string(),
                        content: response.content,
                    },
                    finish_reason: response.finish_reason,
                }],
                usage,
            };

            Json(openai_response).into_response()
        }
        Err(e) => error_response(e),
    }
}

/// Handle streaming chat request
async fn handle_streaming(
    provider: Arc<dyn LLMProvider>,
    request: ChatRequest,
    model: String,
) -> Response {
    match provider.stream_chat(request).await {
        Ok(mut rx) => {
            let stream_id = format!("chatcmpl-{}", uuid::Uuid::new_v4());
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let stream = async_stream::stream! {
                let mut is_first = true;

                while let Some(result) = rx.recv().await {
                    match result {
                        Ok(chunk) => {
                            let delta = if is_first {
                                is_first = false;
                                OpenAIDelta {
                                    role: Some("assistant".to_string()),
                                    content: Some(chunk.content),
                                }
                            } else {
                                OpenAIDelta {
                                    role: None,
                                    content: Some(chunk.content),
                                }
                            };

                            let stream_chunk = OpenAIStreamChunk {
                                id: stream_id.clone(),
                                object: "chat.completion.chunk".to_string(),
                                created: now,
                                model: model.clone(),
                                choices: vec![OpenAIStreamChoice {
                                    index: 0,
                                    delta,
                                    finish_reason: if chunk.is_final {
                                        Some(chunk.finish_reason.unwrap_or_else(|| "stop".to_string()))
                                    } else {
                                        None
                                    },
                                }],
                            };

                            let json = serde_json::to_string(&stream_chunk).unwrap();
                            yield Ok::<_, Infallible>(Event::default().data(json));

                            if chunk.is_final {
                                yield Ok(Event::default().data("[DONE]"));
                                break;
                            }
                        }
                        Err(e) => {
                            log::error!("Stream error: {}", e);
                            break;
                        }
                    }
                }
            };

            Sse::new(stream)
                .keep_alive(axum::response::sse::KeepAlive::new())
                .into_response()
        }
        Err(e) => error_response(e),
    }
}

/// Convert LLMError to HTTP error response
fn error_response(error: LLMError) -> Response {
    let (status, error_type) = match &error {
        LLMError::AuthError(_) => (StatusCode::UNAUTHORIZED, "authentication_error"),
        LLMError::RateLimited { .. } => (StatusCode::TOO_MANY_REQUESTS, "rate_limit_error"),
        LLMError::NotConfigured(_) => (StatusCode::SERVICE_UNAVAILABLE, "service_unavailable"),
        LLMError::Timeout => (StatusCode::GATEWAY_TIMEOUT, "timeout_error"),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
    };

    (
        status,
        Json(serde_json::json!({
            "error": {
                "message": error.to_string(),
                "type": error_type
            }
        })),
    )
        .into_response()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_message_conversion() {
        let msg = OpenAIMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        };
        let chat_msg: ChatMessage = msg.into();
        assert_eq!(chat_msg.role, MessageRole::User);
        assert_eq!(chat_msg.content, "Hello");
    }

    #[tokio::test]
    async fn test_model_parsing() {
        let state = ProxyState::new();

        // With prefix
        let result = state.parse_model("claude:claude-sonnet-4-20250514").await;
        assert_eq!(result, Some(("claude".to_string(), "claude-sonnet-4-20250514".to_string())));

        // Without prefix, no default
        let result = state.parse_model("gpt-4").await;
        assert_eq!(result, None);

        // Set default and try again
        {
            let mut default = state.default_provider.write().await;
            *default = Some("openai".to_string());
        }
        let result = state.parse_model("gpt-4").await;
        assert_eq!(result, Some(("openai".to_string(), "gpt-4".to_string())));
    }
}
