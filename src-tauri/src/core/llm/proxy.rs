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
    #[serde(default)]
    pub tools: Option<Vec<OpenAITool>>,
    #[serde(default)]
    pub tool_choice: Option<serde_json::Value>,
}

/// OpenAI-compatible tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAITool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: OpenAIFunction,
}

/// OpenAI-compatible function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIFunction {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

/// OpenAI-compatible tool call (in assistant messages)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: OpenAIFunctionCall,
}

/// OpenAI-compatible function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIFunctionCall {
    pub name: String,
    pub arguments: String,
}

/// OpenAI-compatible message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIMessage {
    pub role: String,
    /// Content can be None for assistant messages with tool_calls
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Tool calls made by the assistant
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAIToolCall>>,
    /// Tool call ID for tool response messages
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl From<OpenAIMessage> for ChatMessage {
    fn from(msg: OpenAIMessage) -> Self {
        let role = match msg.role.as_str() {
            "system" => MessageRole::System,
            "assistant" => MessageRole::Assistant,
            "tool" => MessageRole::User, // Tool results are sent as user role in internal format
            _ => MessageRole::User,
        };

        // Convert OpenAI tool calls to internal JSON format
        let tool_calls = msg.tool_calls.map(|calls| {
            calls
                .into_iter()
                .map(|tc| {
                    serde_json::json!({
                        "id": tc.id,
                        "type": tc.tool_type,
                        "function": {
                            "name": tc.function.name,
                            "arguments": tc.function.arguments
                        }
                    })
                })
                .collect()
        });

        ChatMessage {
            role,
            content: msg.content.unwrap_or_default(),
            images: None,
            name: None,
            tool_calls,
            tool_call_id: msg.tool_call_id,
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
    pub message: OpenAIChoiceMessage,
    pub finish_reason: Option<String>,
}

/// Message in a choice (separate from request message for cleaner serialization)
#[derive(Debug, Clone, Serialize)]
pub struct OpenAIChoiceMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAIToolCall>>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAIDeltaToolCall>>,
}

/// Tool call delta for streaming (may have partial data)
#[derive(Debug, Clone, Serialize)]
pub struct OpenAIDeltaToolCall {
    pub index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<OpenAIDeltaFunctionCall>,
}

/// Function call delta for streaming
#[derive(Debug, Clone, Serialize)]
pub struct OpenAIDeltaFunctionCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
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
// OpenAI-Compatible Embedding Types
// ============================================================================

/// OpenAI-compatible embedding request
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIEmbeddingRequest {
    /// Model to use for embeddings
    pub model: String,
    /// Input text(s) to embed
    pub input: EmbeddingInputType,
    /// Number of dimensions for the embedding (optional)
    #[serde(default)]
    pub dimensions: Option<u32>,
    /// Encoding format (optional, default: float)
    #[serde(default)]
    pub encoding_format: Option<String>,
}

/// Input type for embeddings - single string or array
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInputType {
    Single(String),
    Multiple(Vec<String>),
}

impl EmbeddingInputType {
    pub fn into_vec(self) -> Vec<String> {
        match self {
            Self::Single(s) => vec![s],
            Self::Multiple(v) => v,
        }
    }
}

/// OpenAI-compatible embedding response
#[derive(Debug, Clone, Serialize)]
pub struct OpenAIEmbeddingResponse {
    pub object: String,
    pub model: String,
    pub data: Vec<OpenAIEmbeddingData>,
    pub usage: OpenAIEmbeddingUsage,
}

/// Individual embedding data
#[derive(Debug, Clone, Serialize)]
pub struct OpenAIEmbeddingData {
    pub object: String,
    pub index: u32,
    pub embedding: Vec<f32>,
}

/// Usage info for embeddings
#[derive(Debug, Clone, Serialize)]
pub struct OpenAIEmbeddingUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

/// Callback type for embedding requests
pub type EmbeddingCallback = Arc<
    dyn Fn(String, Vec<String>, Option<u32>) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<OpenAIEmbeddingResponse, String>> + Send>
    > + Send + Sync
>;

// ============================================================================
// Proxy Service State
// ============================================================================

/// Shared state for the proxy service
pub struct ProxyState {
    /// Registered providers keyed by provider ID
    pub providers: RwLock<HashMap<String, Arc<dyn LLMProvider>>>,
    /// Default provider ID (used when no prefix in model name)
    pub default_provider: RwLock<Option<String>>,
    /// Embedding callback for handling /v1/embeddings requests
    pub embedding_callback: RwLock<Option<EmbeddingCallback>>,
    /// Default embedding model
    pub default_embedding_model: RwLock<Option<String>>,
}

impl ProxyState {
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            default_provider: RwLock::new(None),
            embedding_callback: RwLock::new(None),
            default_embedding_model: RwLock::new(None),
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

    /// Create with default port (18787)
    pub fn with_defaults() -> Self {
        Self::new(18787)
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

    /// Set the embedding callback for handling /v1/embeddings requests
    pub async fn set_embedding_callback(&self, callback: EmbeddingCallback) {
        let mut cb = self.state.embedding_callback.write().await;
        *cb = Some(callback);
        log::info!("Registered embedding callback");
    }

    /// Set the default embedding model
    pub async fn set_default_embedding_model(&self, model: &str) {
        let mut default = self.state.default_embedding_model.write().await;
        *default = Some(model.to_string());
        log::info!("Set default embedding model: {}", model);
    }

    /// Check if embeddings are available
    pub async fn has_embeddings(&self) -> bool {
        self.state.embedding_callback.read().await.is_some()
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
            .route("/v1/embeddings", post(embeddings))
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

            // HTTP is intentional for localhost (127.0.0.1) - no TLS needed for local connections
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

/// Embeddings endpoint
async fn embeddings(
    State(state): State<Arc<ProxyState>>,
    Json(request): Json<OpenAIEmbeddingRequest>,
) -> Response {
    // Get the embedding callback
    let callback = {
        let guard = state.embedding_callback.read().await;
        match guard.as_ref() {
            Some(cb) => cb.clone(),
            None => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({
                        "error": {
                            "message": "Embeddings not configured",
                            "type": "service_unavailable"
                        }
                    })),
                )
                    .into_response();
            }
        }
    };

    // Get default model if not specified or empty
    let model = if request.model.is_empty() {
        let default = state.default_embedding_model.read().await;
        match default.as_ref() {
            Some(m) => m.clone(),
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": {
                            "message": "Model is required",
                            "type": "invalid_request_error"
                        }
                    })),
                )
                    .into_response();
            }
        }
    } else {
        request.model
    };

    let input = request.input.into_vec();
    let dimensions = request.dimensions;

    // Call the embedding callback
    match callback(model, input, dimensions).await {
        Ok(response) => Json(response).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": {
                    "message": e,
                    "type": "internal_error"
                }
            })),
        )
            .into_response(),
    }
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

    // Extract system messages and combine them into a single system prompt
    // Claude API handles system prompts differently - they should be passed
    // as a separate `system` parameter, not as a message with role "system"
    let system_prompt: Option<String> = {
        let combined: String = request
            .messages
            .iter()
            .filter(|m| m.role == "system")
            .filter_map(|m| m.content.as_deref())
            .collect::<Vec<_>>()
            .join("\n\n");
        if combined.is_empty() {
            None
        } else {
            Some(combined)
        }
    };

    // Convert non-system messages
    let messages: Vec<ChatMessage> = request
        .messages
        .into_iter()
        .filter(|m| m.role != "system")
        .map(|m| m.into())
        .collect();

    // Convert tools to internal JSON format
    let tools = request.tools.map(convert_openai_tools);

    // Build internal request
    let chat_request = ChatRequest {
        messages,
        temperature: request.temperature,
        max_tokens: request.max_tokens,
        system_prompt,
        provider: None,
        tools,
        tool_choice: request.tool_choice,
    };

    if request.stream {
        // Streaming response
        handle_streaming(provider, chat_request, request.model).await
    } else {
        // Non-streaming response
        handle_non_streaming(provider, chat_request, request.model).await
    }
}

/// Convert OpenAI tools to internal JSON format
fn convert_openai_tools(tools: Vec<OpenAITool>) -> Vec<serde_json::Value> {
    tools
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "type": t.tool_type,
                "function": {
                    "name": t.function.name,
                    "description": t.function.description,
                    "parameters": t.function.parameters
                }
            })
        })
        .collect()
}

/// Convert internal tool_calls JSON to OpenAI format
fn convert_tool_calls_to_openai(tool_calls: Option<Vec<serde_json::Value>>) -> Option<Vec<OpenAIToolCall>> {
    tool_calls.map(|calls| {
        calls
            .into_iter()
            .filter_map(|tc| {
                let id = tc.get("id")?.as_str()?.to_string();
                let tool_type = tc.get("type").and_then(|t| t.as_str()).unwrap_or("function").to_string();
                let function = tc.get("function")?;
                let name = function.get("name")?.as_str()?.to_string();
                let arguments = function.get("arguments").and_then(|a| a.as_str()).unwrap_or("{}").to_string();

                Some(OpenAIToolCall {
                    id,
                    tool_type,
                    function: OpenAIFunctionCall { name, arguments },
                })
            })
            .collect()
    })
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

            // Convert tool_calls from internal format to OpenAI format
            let tool_calls = convert_tool_calls_to_openai(response.tool_calls);

            // Determine content and finish_reason based on whether there are tool calls
            let (content, finish_reason) = if tool_calls.is_some() {
                // When there are tool calls, content may be empty and finish_reason is "tool_calls"
                let content = if response.content.is_empty() {
                    None
                } else {
                    Some(response.content)
                };
                (content, Some("tool_calls".to_string()))
            } else {
                (Some(response.content), response.finish_reason)
            };

            let openai_response = OpenAIChatResponse {
                id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                object: "chat.completion".to_string(),
                created: now,
                model,
                choices: vec![OpenAIChoice {
                    index: 0,
                    message: OpenAIChoiceMessage {
                        role: "assistant".to_string(),
                        content,
                        tool_calls,
                    },
                    finish_reason,
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
    match provider.stream_chat(request.clone()).await {
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
                                    content: if chunk.content.is_empty() { None } else { Some(chunk.content) },
                                    tool_calls: None, // Tool calls would be streamed separately if supported
                                }
                            } else {
                                OpenAIDelta {
                                    role: None,
                                    content: if chunk.content.is_empty() { None } else { Some(chunk.content) },
                                    tool_calls: None,
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
        Err(LLMError::StreamingNotSupported(_)) => {
            // Fall back to non-streaming for providers that don't support it
            log::info!("Provider doesn't support streaming, falling back to non-streaming");
            handle_streaming_fallback(provider, request, model).await
        }
        Err(e) => error_response(e),
    }
}

/// Fallback for providers that don't support streaming - emit full response as SSE
async fn handle_streaming_fallback(
    provider: Arc<dyn LLMProvider>,
    request: ChatRequest,
    model: String,
) -> Response {
    let stream_id = format!("chatcmpl-{}", uuid::Uuid::new_v4());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Call non-streaming chat
    match provider.chat(request).await {
        Ok(response) => {
            // Convert tool_calls if present for streaming delta format
            let tool_calls_delta = response.tool_calls.as_ref().map(|calls| {
                calls
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, tc)| {
                        let id = tc.get("id")?.as_str()?.to_string();
                        let tool_type = tc.get("type").and_then(|t| t.as_str()).unwrap_or("function").to_string();
                        let function = tc.get("function")?;
                        let name = function.get("name")?.as_str()?.to_string();
                        let arguments = function.get("arguments").and_then(|a| a.as_str()).unwrap_or("{}").to_string();

                        Some(OpenAIDeltaToolCall {
                            index: idx as u32,
                            id: Some(id),
                            tool_type: Some(tool_type),
                            function: Some(OpenAIDeltaFunctionCall {
                                name: Some(name),
                                arguments: Some(arguments),
                            }),
                        })
                    })
                    .collect::<Vec<_>>()
            }).filter(|v| !v.is_empty());

            // Determine finish_reason
            let finish_reason = if tool_calls_delta.is_some() {
                "tool_calls".to_string()
            } else {
                "stop".to_string()
            };

            // Emit the full response as a single SSE chunk
            let stream = async_stream::stream! {
                // First chunk with role
                let delta = OpenAIDelta {
                    role: Some("assistant".to_string()),
                    content: if response.content.is_empty() { None } else { Some(response.content) },
                    tool_calls: tool_calls_delta,
                };

                let stream_chunk = OpenAIStreamChunk {
                    id: stream_id.clone(),
                    object: "chat.completion.chunk".to_string(),
                    created: now,
                    model: model.clone(),
                    choices: vec![OpenAIStreamChoice {
                        index: 0,
                        delta,
                        finish_reason: Some(finish_reason),
                    }],
                };

                let json = serde_json::to_string(&stream_chunk).unwrap();
                yield Ok::<_, Infallible>(Event::default().data(json));
                yield Ok(Event::default().data("[DONE]"));
            };

            Sse::new(stream)
                .keep_alive(axum::response::sse::KeepAlive::new())
                .into_response()
        }
        Err(e) => {
            // Return error as SSE event so client can handle it properly
            let stream = async_stream::stream! {
                let error_chunk = OpenAIStreamChunk {
                    id: stream_id.clone(),
                    object: "chat.completion.chunk".to_string(),
                    created: now,
                    model: model.clone(),
                    choices: vec![OpenAIStreamChoice {
                        index: 0,
                        delta: OpenAIDelta {
                            role: Some("assistant".to_string()),
                            content: Some(format!("Error: {}", e)),
                            tool_calls: None,
                        },
                        // Use "stop" for spec compliance; "error" is not a valid OpenAI finish_reason
                        finish_reason: Some("stop".to_string()),
                    }],
                };

                let json = serde_json::to_string(&error_chunk).unwrap();
                yield Ok::<_, Infallible>(Event::default().data(json));
                yield Ok(Event::default().data("[DONE]"));
            };

            Sse::new(stream)
                .keep_alive(axum::response::sse::KeepAlive::new())
                .into_response()
        }
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
            content: Some("Hello".to_string()),
            tool_calls: None,
            tool_call_id: None,
        };
        let chat_msg: ChatMessage = msg.into();
        assert_eq!(chat_msg.role, MessageRole::User);
        assert_eq!(chat_msg.content, "Hello");
    }

    #[test]
    fn test_openai_message_with_tool_calls() {
        let msg = OpenAIMessage {
            role: "assistant".to_string(),
            content: None,
            tool_calls: Some(vec![OpenAIToolCall {
                id: "call_123".to_string(),
                tool_type: "function".to_string(),
                function: OpenAIFunctionCall {
                    name: "_meiliSearchInIndex".to_string(),
                    arguments: r#"{"query": "test"}"#.to_string(),
                },
            }]),
            tool_call_id: None,
        };
        let chat_msg: ChatMessage = msg.into();
        assert_eq!(chat_msg.role, MessageRole::Assistant);
        assert!(chat_msg.tool_calls.is_some());
        let tool_calls = chat_msg.tool_calls.unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0]["id"], "call_123");
        assert_eq!(tool_calls[0]["function"]["name"], "_meiliSearchInIndex");
    }

    #[test]
    fn test_openai_tool_message() {
        let msg = OpenAIMessage {
            role: "tool".to_string(),
            content: Some(r#"{"results": []}"#.to_string()),
            tool_calls: None,
            tool_call_id: Some("call_123".to_string()),
        };
        let chat_msg: ChatMessage = msg.into();
        // Tool messages are mapped to User role internally
        assert_eq!(chat_msg.role, MessageRole::User);
        assert_eq!(chat_msg.tool_call_id, Some("call_123".to_string()));
        assert_eq!(chat_msg.content, r#"{"results": []}"#);
    }

    #[test]
    fn test_convert_openai_tools() {
        let tools = vec![OpenAITool {
            tool_type: "function".to_string(),
            function: OpenAIFunction {
                name: "_meiliSearchInIndex".to_string(),
                description: Some("Search in a Meilisearch index".to_string()),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    }
                })),
            },
        }];

        let converted = convert_openai_tools(tools);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0]["type"], "function");
        assert_eq!(converted[0]["function"]["name"], "_meiliSearchInIndex");
    }

    #[test]
    fn test_convert_tool_calls_to_openai() {
        let tool_calls = Some(vec![serde_json::json!({
            "id": "call_456",
            "type": "function",
            "function": {
                "name": "_meiliSearchProgress",
                "arguments": "{}"
            }
        })]);

        let openai_calls = convert_tool_calls_to_openai(tool_calls);
        assert!(openai_calls.is_some());
        let calls = openai_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_456");
        assert_eq!(calls[0].tool_type, "function");
        assert_eq!(calls[0].function.name, "_meiliSearchProgress");
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
