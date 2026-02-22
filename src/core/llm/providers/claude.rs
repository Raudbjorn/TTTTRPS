//! Claude Provider Implementation
//!
//! OAuth-based Anthropic API access using the `claude` module.
//! This provider enables Claude API access without requiring an API key
//! by using OAuth 2.0 PKCE flow for authentication.
//!
//! ## Features
//!
//! - OAuth 2.0 PKCE authentication flow
//! - Multiple storage backends (file, keyring, memory)
//! - Automatic token refresh
//! - Full streaming support
//! - Tool use support
//! - Cost tracking with token usage
//!
//! ## Usage
//!
//! ```rust,no_run
//! use crate::core::llm::providers::ClaudeProvider;
//!
//! // Using file storage (default)
//! let provider = ClaudeProvider::new().unwrap();
//!
//! // Using keyring storage
//! let provider = ClaudeProvider::with_keyring().unwrap();
//!
//! // Check if authenticated
//! if !provider.is_authenticated().await.unwrap() {
//!     // Start OAuth flow
//!     let auth_url = provider.start_oauth_flow().await.unwrap();
//!     println!("Open this URL to authenticate: {}", auth_url);
//!     // After user completes flow and gets code:
//!     // provider.complete_oauth_flow(&code).await.unwrap();
//! }
//! ```

use crate::oauth::claude::{
    ClaudeClient, ContentBlock as GateContentBlock, FileTokenStorage, MemoryTokenStorage,
    MessagesResponse, Role as GateRole, StreamEvent,
};
use crate::oauth::claude::models::ContentDelta;
#[cfg(feature = "keyring")]
use crate::oauth::claude::KeyringTokenStorage;

use crate::core::llm::cost::{ProviderPricing, TokenUsage};
use crate::core::llm::router::{
    ChatChunk, ChatRequest, ChatResponse, LLMError, LLMProvider, MessageRole, Result,
};
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

// ============================================================================
// Constants
// ============================================================================

/// Default model to use
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

/// Default max tokens
const DEFAULT_MAX_TOKENS: u32 = 8192;

// ============================================================================
// Storage Backend Enum
// ============================================================================

/// Storage backend options for OAuth tokens
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum StorageBackend {
    /// File-based storage (~/.local/share/ttrpg-assistant/oauth-tokens.json)
    File,
    /// System keyring (GNOME Keyring, macOS Keychain, Windows Credential Manager)
    #[cfg(feature = "keyring")]
    Keyring,
    /// In-memory storage (tokens lost on restart)
    Memory,
    /// Automatic selection (keyring if available, else file)
    #[default]
    Auto,
}


impl StorageBackend {
    /// Get the display name of the storage backend
    pub fn name(&self) -> &str {
        match self {
            Self::File => "file",
            #[cfg(feature = "keyring")]
            Self::Keyring => "keyring",
            Self::Memory => "memory",
            Self::Auto => "auto",
        }
    }
}

// ============================================================================
// Status Types
// ============================================================================

/// Status of the Claude provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeStatus {
    /// Whether the provider is authenticated
    pub authenticated: bool,
    /// Storage backend in use
    pub storage_backend: String,
    /// Time until token expires (seconds)
    pub token_expires_in: Option<i64>,
    /// Error message if any
    pub error: Option<String>,
}

impl Default for ClaudeStatus {
    fn default() -> Self {
        Self {
            authenticated: false,
            storage_backend: "unknown".to_string(),
            token_expires_in: None,
            error: None,
        }
    }
}

// ============================================================================
// Provider Implementation
// ============================================================================

/// Claude provider using OAuth authentication.
///
/// This provider uses the `claude` module to authenticate with
/// Anthropic's OAuth 2.0 PKCE flow, enabling API access without
/// requiring an API key.
pub struct ClaudeProvider {
    /// The underlying Claude client (type-erased for flexibility)
    client: Arc<dyn ClaudeClientTrait>,
    /// Model to use
    model: String,
    /// Max tokens for responses
    max_tokens: u32,
    /// Storage backend name
    storage_backend: String,
}

/// Trait to abstract over different storage backends
#[async_trait]
trait ClaudeClientTrait: Send + Sync {
    async fn is_authenticated(&self) -> crate::oauth::claude::Result<bool>;
    async fn start_oauth_flow(&self) -> crate::oauth::claude::Result<String>;
    async fn complete_oauth_flow(
        &self,
        code: &str,
        state: Option<&str>,
    ) -> crate::oauth::claude::Result<crate::oauth::claude::TokenInfo>;
    async fn logout(&self) -> crate::oauth::claude::Result<()>;
    async fn get_token_info(&self) -> crate::oauth::claude::Result<Option<crate::oauth::claude::TokenInfo>>;
    async fn send_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<crate::oauth::claude::Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::oauth::claude::Result<MessagesResponse>;
    async fn stream_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<crate::oauth::claude::Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::oauth::claude::Result<mpsc::Receiver<crate::oauth::claude::Result<StreamEvent>>>;
}

/// Wrapper for ClaudeClient with FileTokenStorage
struct FileStorageClient {
    client: ClaudeClient<FileTokenStorage>,
}

#[async_trait]
impl ClaudeClientTrait for FileStorageClient {
    async fn is_authenticated(&self) -> crate::oauth::claude::Result<bool> {
        self.client.is_authenticated().await
    }

    async fn start_oauth_flow(&self) -> crate::oauth::claude::Result<String> {
        self.client.start_oauth_flow().await
    }

    async fn complete_oauth_flow(
        &self,
        code: &str,
        state: Option<&str>,
    ) -> crate::oauth::claude::Result<crate::oauth::claude::TokenInfo> {
        self.client.complete_oauth_flow(code, state).await
    }

    async fn logout(&self) -> crate::oauth::claude::Result<()> {
        self.client.logout().await
    }

    async fn get_token_info(&self) -> crate::oauth::claude::Result<Option<crate::oauth::claude::TokenInfo>> {
        self.client.get_token_info().await
    }

    async fn send_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<crate::oauth::claude::Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::oauth::claude::Result<MessagesResponse> {
        let mut builder = self.client.messages()
            .model(model)
            .max_tokens(max_tokens)
            .messages(messages);

        if let Some(sys) = system {
            builder = builder.system(sys);
        }
        if let Some(temp) = temperature {
            builder = builder.temperature(temp);
        }

        builder.send().await
    }

    async fn stream_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<crate::oauth::claude::Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::oauth::claude::Result<mpsc::Receiver<crate::oauth::claude::Result<StreamEvent>>> {
        let mut builder = self.client.messages()
            .model(model)
            .max_tokens(max_tokens)
            .messages(messages)
            .stream();

        if let Some(sys) = system {
            builder = builder.system(sys);
        }
        if let Some(temp) = temperature {
            builder = builder.temperature(temp);
        }

        let stream = builder.send_stream().await?;
        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            futures::pin_mut!(stream);
            while let Some(event) = stream.next().await {
                if tx.send(event).await.is_err() {
                    break;
                }
            }
        });

        Ok(rx)
    }
}

/// Wrapper for ClaudeClient with KeyringTokenStorage
#[cfg(feature = "keyring")]
struct KeyringStorageClient {
    client: ClaudeClient<KeyringTokenStorage>,
}

#[cfg(feature = "keyring")]
#[async_trait]
impl ClaudeClientTrait for KeyringStorageClient {
    async fn is_authenticated(&self) -> crate::oauth::claude::Result<bool> {
        self.client.is_authenticated().await
    }

    async fn start_oauth_flow(&self) -> crate::oauth::claude::Result<String> {
        self.client.start_oauth_flow().await
    }

    async fn complete_oauth_flow(
        &self,
        code: &str,
        state: Option<&str>,
    ) -> crate::oauth::claude::Result<crate::oauth::claude::TokenInfo> {
        self.client.complete_oauth_flow(code, state).await
    }

    async fn logout(&self) -> crate::oauth::claude::Result<()> {
        self.client.logout().await
    }

    async fn get_token_info(&self) -> crate::oauth::claude::Result<Option<crate::oauth::claude::TokenInfo>> {
        self.client.get_token_info().await
    }

    async fn send_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<crate::oauth::claude::Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::oauth::claude::Result<MessagesResponse> {
        let mut builder = self.client.messages()
            .model(model)
            .max_tokens(max_tokens)
            .messages(messages);

        if let Some(sys) = system {
            builder = builder.system(sys);
        }
        if let Some(temp) = temperature {
            builder = builder.temperature(temp);
        }

        builder.send().await
    }

    async fn stream_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<crate::oauth::claude::Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::oauth::claude::Result<mpsc::Receiver<crate::oauth::claude::Result<StreamEvent>>> {
        let mut builder = self.client.messages()
            .model(model)
            .max_tokens(max_tokens)
            .messages(messages)
            .stream();

        if let Some(sys) = system {
            builder = builder.system(sys);
        }
        if let Some(temp) = temperature {
            builder = builder.temperature(temp);
        }

        let stream = builder.send_stream().await?;
        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            futures::pin_mut!(stream);
            while let Some(event) = stream.next().await {
                if tx.send(event).await.is_err() {
                    break;
                }
            }
        });

        Ok(rx)
    }
}

/// Wrapper for ClaudeClient with MemoryTokenStorage
struct MemoryStorageClient {
    client: ClaudeClient<MemoryTokenStorage>,
}

#[async_trait]
impl ClaudeClientTrait for MemoryStorageClient {
    async fn is_authenticated(&self) -> crate::oauth::claude::Result<bool> {
        self.client.is_authenticated().await
    }

    async fn start_oauth_flow(&self) -> crate::oauth::claude::Result<String> {
        self.client.start_oauth_flow().await
    }

    async fn complete_oauth_flow(
        &self,
        code: &str,
        state: Option<&str>,
    ) -> crate::oauth::claude::Result<crate::oauth::claude::TokenInfo> {
        self.client.complete_oauth_flow(code, state).await
    }

    async fn logout(&self) -> crate::oauth::claude::Result<()> {
        self.client.logout().await
    }

    async fn get_token_info(&self) -> crate::oauth::claude::Result<Option<crate::oauth::claude::TokenInfo>> {
        self.client.get_token_info().await
    }

    async fn send_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<crate::oauth::claude::Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::oauth::claude::Result<MessagesResponse> {
        let mut builder = self.client.messages()
            .model(model)
            .max_tokens(max_tokens)
            .messages(messages);

        if let Some(sys) = system {
            builder = builder.system(sys);
        }
        if let Some(temp) = temperature {
            builder = builder.temperature(temp);
        }

        builder.send().await
    }

    async fn stream_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<crate::oauth::claude::Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::oauth::claude::Result<mpsc::Receiver<crate::oauth::claude::Result<StreamEvent>>> {
        let mut builder = self.client.messages()
            .model(model)
            .max_tokens(max_tokens)
            .messages(messages)
            .stream();

        if let Some(sys) = system {
            builder = builder.system(sys);
        }
        if let Some(temp) = temperature {
            builder = builder.temperature(temp);
        }

        let stream = builder.send_stream().await?;
        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            futures::pin_mut!(stream);
            while let Some(event) = stream.next().await {
                if tx.send(event).await.is_err() {
                    break;
                }
            }
        });

        Ok(rx)
    }
}

impl ClaudeProvider {
    /// Create a new provider with file-based token storage.
    ///
    /// Uses the app data path (~/.local/share/ttrpg-assistant/oauth-tokens.json).
    pub fn new() -> Result<Self> {
        Self::with_storage(StorageBackend::File, DEFAULT_MODEL.to_string(), DEFAULT_MAX_TOKENS)
    }

    /// Create a new provider with automatic storage selection.
    ///
    /// Tries keyring first (if available), then falls back to file storage.
    pub fn auto() -> Result<Self> {
        Self::with_storage(StorageBackend::Auto, DEFAULT_MODEL.to_string(), DEFAULT_MAX_TOKENS)
    }

    /// Create a new provider with keyring storage.
    #[cfg(feature = "keyring")]
    pub fn with_keyring() -> Result<Self> {
        Self::with_storage(StorageBackend::Keyring, DEFAULT_MODEL.to_string(), DEFAULT_MAX_TOKENS)
    }

    /// Create a new provider with in-memory storage.
    ///
    /// Tokens will be lost when the provider is dropped.
    /// Useful for testing.
    pub fn with_memory() -> Result<Self> {
        Self::with_storage(StorageBackend::Memory, DEFAULT_MODEL.to_string(), DEFAULT_MAX_TOKENS)
    }

    /// Create a new provider with specified storage backend.
    pub fn with_storage(backend: StorageBackend, model: String, max_tokens: u32) -> Result<Self> {
        let (client, storage_name): (Arc<dyn ClaudeClientTrait>, String) = match backend {
            StorageBackend::File => {
                let storage = FileTokenStorage::app_data_path()
                    .map_err(|e| LLMError::NotConfigured(format!("Failed to create file storage: {}", e)))?;
                let claude_client = ClaudeClient::builder()
                    .with_storage(storage)
                    .build()
                    .map_err(|e| LLMError::NotConfigured(format!("Failed to create client: {}", e)))?;
                (Arc::new(FileStorageClient { client: claude_client }), "file".to_string())
            }
            #[cfg(feature = "keyring")]
            StorageBackend::Keyring => {
                let storage = KeyringTokenStorage::new();
                let claude_client = ClaudeClient::builder()
                    .with_storage(storage)
                    .build()
                    .map_err(|e| LLMError::NotConfigured(format!("Failed to create client: {}", e)))?;
                (Arc::new(KeyringStorageClient { client: claude_client }), "keyring".to_string())
            }
            StorageBackend::Memory => {
                let storage = MemoryTokenStorage::new();
                let claude_client = ClaudeClient::builder()
                    .with_storage(storage)
                    .build()
                    .map_err(|e| LLMError::NotConfigured(format!("Failed to create client: {}", e)))?;
                (Arc::new(MemoryStorageClient { client: claude_client }), "memory".to_string())
            }
            StorageBackend::Auto => {
                // Try keyring first, fall back to file
                #[cfg(feature = "keyring")]
                {
                    if KeyringTokenStorage::is_available() {
                        let storage = KeyringTokenStorage::new();
                        let claude_client = ClaudeClient::builder()
                            .with_storage(storage)
                            .build()
                            .map_err(|e| LLMError::NotConfigured(format!("Failed to create client: {}", e)))?;
                        (Arc::new(KeyringStorageClient { client: claude_client }), "keyring".to_string())
                    } else {
                        let storage = FileTokenStorage::app_data_path()
                            .map_err(|e| LLMError::NotConfigured(format!("Failed to create file storage: {}", e)))?;
                        let claude_client = ClaudeClient::builder()
                            .with_storage(storage)
                            .build()
                            .map_err(|e| LLMError::NotConfigured(format!("Failed to create client: {}", e)))?;
                        (Arc::new(FileStorageClient { client: claude_client }), "file".to_string())
                    }
                }
                #[cfg(not(feature = "keyring"))]
                {
                    let storage = FileTokenStorage::app_data_path()
                        .map_err(|e| LLMError::NotConfigured(format!("Failed to create file storage: {}", e)))?;
                    let claude_client = ClaudeClient::builder()
                        .with_storage(storage)
                        .build()
                        .map_err(|e| LLMError::NotConfigured(format!("Failed to create client: {}", e)))?;
                    (Arc::new(FileStorageClient { client: claude_client }), "file".to_string())
                }
            }
        };

        Ok(Self {
            client,
            model,
            max_tokens,
            storage_backend: storage_name,
        })
    }

    /// Create a provider from a storage backend name string.
    ///
    /// Accepts: "file", "keyring", "memory", "auto"
    pub fn from_storage_name(name: &str, model: String, max_tokens: u32) -> Result<Self> {
        let backend = match name.to_lowercase().as_str() {
            "file" => StorageBackend::File,
            #[cfg(feature = "keyring")]
            "keyring" => StorageBackend::Keyring,
            "memory" => StorageBackend::Memory,
            "auto" => StorageBackend::Auto,
            _ => {
                #[cfg(feature = "keyring")]
                let valid_options = "file, keyring, memory, auto";
                #[cfg(not(feature = "keyring"))]
                let valid_options = "file, memory, auto";
                return Err(LLMError::NotConfigured(format!(
                    "Unknown storage backend: {}. Valid options: {}",
                    name, valid_options
                )));
            }
        };
        Self::with_storage(backend, model, max_tokens)
    }

    // ========================================================================
    // OAuth Flow Methods
    // ========================================================================

    /// Check if the provider is authenticated.
    ///
    /// Returns true if a valid (non-expired) token exists.
    pub async fn is_authenticated(&self) -> Result<bool> {
        self.client
            .is_authenticated()
            .await
            .map_err(|e| LLMError::AuthError(e.to_string()))
    }

    /// Start the OAuth authorization flow.
    ///
    /// Returns the URL that the user should open in their browser.
    /// After the user authorizes, they will be redirected to a page
    /// that displays an authorization code.
    pub async fn start_oauth_flow(&self) -> Result<String> {
        self.client
            .start_oauth_flow()
            .await
            .map_err(|e| LLMError::AuthError(format!("Failed to start OAuth flow: {}", e)))
    }

    /// Complete the OAuth flow by exchanging the authorization code.
    ///
    /// Call this with the code the user received after authorization.
    pub async fn complete_oauth_flow(&self, code: &str) -> Result<()> {
        self.client
            .complete_oauth_flow(code, None)
            .await
            .map_err(|e| LLMError::AuthError(format!("Failed to complete OAuth flow: {}", e)))?;
        info!("OAuth flow completed successfully");
        Ok(())
    }

    /// Log out and remove stored credentials.
    pub async fn logout(&self) -> Result<()> {
        self.client
            .logout()
            .await
            .map_err(|e| LLMError::AuthError(format!("Failed to logout: {}", e)))?;
        info!("Logged out from Claude");
        Ok(())
    }

    /// Get the storage backend name.
    pub fn get_storage_backend(&self) -> &str {
        &self.storage_backend
    }

    /// Get full status of the provider.
    pub async fn get_status(&self) -> ClaudeStatus {
        match self.client.is_authenticated().await {
            Ok(authenticated) => {
                let token_expires_in = if authenticated {
                    self.client
                        .get_token_info()
                        .await
                        .ok()
                        .flatten()
                        .map(|t| t.time_until_expiry())
                } else {
                    None
                };

                ClaudeStatus {
                    authenticated,
                    storage_backend: self.storage_backend.clone(),
                    token_expires_in,
                    error: None,
                }
            }
            Err(e) => ClaudeStatus {
                authenticated: false,
                storage_backend: self.storage_backend.clone(),
                token_expires_in: None,
                error: Some(e.to_string()),
            },
        }
    }

    // ========================================================================
    // Message Conversion
    // ========================================================================

    /// Convert ChatRequest messages to claude Message format
    /// Convert ChatRequest messages to claude Message format.
    ///
    /// Note: System messages are filtered out here because Claude API expects
    /// the system prompt to be passed separately via the `system` parameter,
    /// not as part of the messages array. The system prompt is extracted from
    /// `request.system_prompt` and passed directly to the API.
    fn convert_messages(&self, request: &ChatRequest) -> Vec<crate::oauth::claude::Message> {
        request
            .messages
            .iter()
            .filter(|m| m.role != MessageRole::System) // System messages go via separate API parameter
            .map(|msg| {
                let role = match msg.role {
                    MessageRole::User => GateRole::User,
                    MessageRole::Assistant => GateRole::Assistant,
                    // System messages are filtered above; this arm exists for exhaustiveness
                    MessageRole::System => {
                        tracing::warn!("System message reached convert_messages unexpectedly");
                        GateRole::User
                    }
                };

                crate::oauth::claude::Message::with_content(
                    role,
                    vec![GateContentBlock::text(&msg.content)],
                )
            })
            .collect()
    }

    /// Convert claude MessagesResponse to ChatResponse
    fn convert_response(&self, response: MessagesResponse, latency_ms: u64) -> ChatResponse {
        let content = response.text();

        let usage = Some(TokenUsage {
            input_tokens: response.usage.input_tokens,
            output_tokens: response.usage.output_tokens,
        });

        let cost_usd = usage.as_ref().and_then(|u| {
            self.pricing().map(|p| p.calculate_cost(u))
        });

        // Convert tool uses to tool_calls format if present
        let tool_calls = if !response.tool_uses().is_empty() {
            let calls: Vec<serde_json::Value> = response
                .tool_uses()
                .iter()
                .map(|(id, name, input)| {
                    serde_json::json!({
                        "id": id,
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": serde_json::to_string(input).unwrap_or_default()
                        }
                    })
                })
                .collect();
            if calls.is_empty() { None } else { Some(calls) }
        } else {
            None
        };

        ChatResponse {
            content,
            model: response.model,
            provider: "claude".to_string(),
            usage,
            finish_reason: response.stop_reason.map(|r| format!("{:?}", r).to_lowercase()),
            latency_ms,
            cost_usd,
            tool_calls,
        }
    }
}

// ============================================================================
// LLMProvider Implementation
// ============================================================================

#[async_trait]
impl LLMProvider for ClaudeProvider {
    fn id(&self) -> &str {
        "claude"
    }

    fn name(&self) -> &str {
        "Claude"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn health_check(&self) -> bool {
        match self.client.is_authenticated().await {
            Ok(true) => true,
            Ok(false) => {
                debug!("Claude health check: not authenticated");
                false
            }
            Err(e) => {
                warn!("Claude health check failed: {}", e);
                false
            }
        }
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        // Use the same pricing as the Claude provider
        ProviderPricing::for_model("claude", &self.model)
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        // Check authentication first
        if !self.is_authenticated().await? {
            return Err(LLMError::AuthError(
                "Not authenticated. Please complete OAuth flow first.".to_string(),
            ));
        }

        let messages = self.convert_messages(&request);
        let system = request.system_prompt.clone();
        let temperature = request.temperature;
        let max_tokens = request.max_tokens.unwrap_or(self.max_tokens);

        debug!(
            model = %self.model,
            message_count = messages.len(),
            max_tokens = max_tokens,
            "Sending chat request via Claude"
        );

        let start = Instant::now();

        let response = self.client
            .send_message(&self.model, max_tokens, messages, system, temperature)
            .await
            .map_err(|e| {
                if e.requires_reauth() {
                    LLMError::AuthError(e.to_string())
                } else {
                    match &e {
                        crate::oauth::claude::Error::Api { status, message, .. } => {
                            if *status == 429 {
                                LLMError::RateLimited { retry_after_secs: 60 }
                            } else {
                                LLMError::ApiError {
                                    status: *status,
                                    message: message.clone(),
                                }
                            }
                        }
                        _ => LLMError::ApiError {
                            status: 0,
                            message: e.to_string(),
                        },
                    }
                }
            })?;

        let latency_ms = start.elapsed().as_millis() as u64;

        info!(
            latency_ms = latency_ms,
            input_tokens = response.usage.input_tokens,
            output_tokens = response.usage.output_tokens,
            "Received response from Claude"
        );

        Ok(self.convert_response(response, latency_ms))
    }

    async fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        // Check authentication first
        if !self.is_authenticated().await? {
            return Err(LLMError::AuthError(
                "Not authenticated. Please complete OAuth flow first.".to_string(),
            ));
        }

        let messages = self.convert_messages(&request);
        let system = request.system_prompt.clone();
        let temperature = request.temperature;
        let max_tokens = request.max_tokens.unwrap_or(self.max_tokens);

        debug!(
            model = %self.model,
            message_count = messages.len(),
            max_tokens = max_tokens,
            "Starting streaming chat via Claude"
        );

        let stream_rx = self.client
            .stream_message(&self.model, max_tokens, messages, system, temperature)
            .await
            .map_err(|e| {
                if e.requires_reauth() {
                    LLMError::AuthError(e.to_string())
                } else {
                    LLMError::ApiError {
                        status: 0,
                        message: e.to_string(),
                    }
                }
            })?;

        let (tx, rx) = mpsc::channel::<Result<ChatChunk>>(100);
        let stream_id = uuid::Uuid::new_v4().to_string();
        let model = self.model.clone();

        tokio::spawn(async move {
            let mut stream_rx = stream_rx;
            let mut chunk_index: u32 = 0;
            let mut input_tokens = 0u32;
            let mut final_usage: Option<TokenUsage> = None;

            while let Some(event_result) = stream_rx.recv().await {
                match event_result {
                    Ok(event) => {
                        match event {
                            StreamEvent::MessageStart { message } => {
                                input_tokens = message.usage.input_tokens;
                            }
                            StreamEvent::ContentBlockDelta { delta, .. } => {
                                if let ContentDelta::TextDelta { text } = delta {
                                    if !text.is_empty() {
                                        chunk_index += 1;
                                        let chunk = ChatChunk {
                                            stream_id: stream_id.clone(),
                                            content: text,
                                            provider: "claude".to_string(),
                                            model: model.clone(),
                                            is_final: false,
                                            finish_reason: None,
                                            usage: None,
                                            index: chunk_index,
                                        };
                                        if tx.send(Ok(chunk)).await.is_err() {
                                            return;
                                        }
                                    }
                                }
                            }
                            StreamEvent::MessageDelta { usage, .. } => {
                                final_usage = Some(TokenUsage {
                                    input_tokens,
                                    output_tokens: usage.output_tokens,
                                });
                            }
                            StreamEvent::MessageStop => {
                                let final_chunk = ChatChunk {
                                    stream_id: stream_id.clone(),
                                    content: String::new(),
                                    provider: "claude".to_string(),
                                    model: model.clone(),
                                    is_final: true,
                                    finish_reason: Some("stop".to_string()),
                                    usage: final_usage.clone(),
                                    index: chunk_index + 1,
                                };
                                let _ = tx.send(Ok(final_chunk)).await;
                                return;
                            }
                            StreamEvent::Error { error } => {
                                let _ = tx
                                    .send(Err(LLMError::ApiError {
                                        status: 0,
                                        message: error.message,
                                    }))
                                    .await;
                                return;
                            }
                            _ => {
                                // Ping, ContentBlockStart, ContentBlockStop - ignore
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Err(LLMError::ApiError {
                                status: 0,
                                message: e.to_string(),
                            }))
                            .await;
                        return;
                    }
                }
            }
        });

        Ok(rx)
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_embeddings(&self) -> bool {
        false
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::llm::router::ChatMessage;

    #[test]
    fn test_provider_id() {
        let provider = ClaudeProvider::with_memory().unwrap();
        assert_eq!(provider.id(), "claude");
        assert_eq!(provider.name(), "Claude");
    }

    #[test]
    fn test_storage_backend() {
        let provider = ClaudeProvider::with_memory().unwrap();
        assert_eq!(provider.get_storage_backend(), "memory");
    }

    #[test]
    fn test_model() {
        let provider = ClaudeProvider::with_memory().unwrap();
        assert_eq!(provider.model(), DEFAULT_MODEL);
    }

    #[test]
    fn test_from_storage_name() {
        let provider = ClaudeProvider::from_storage_name(
            "memory",
            "claude-haiku-4-20250514".to_string(),
            4096,
        )
        .unwrap();
        assert_eq!(provider.model(), "claude-haiku-4-20250514");
        assert_eq!(provider.get_storage_backend(), "memory");
    }

    #[test]
    fn test_invalid_storage_name() {
        let result = ClaudeProvider::from_storage_name(
            "invalid",
            DEFAULT_MODEL.to_string(),
            DEFAULT_MAX_TOKENS,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_storage_backend_names() {
        assert_eq!(StorageBackend::File.name(), "file");
        assert_eq!(StorageBackend::Memory.name(), "memory");
        assert_eq!(StorageBackend::Auto.name(), "auto");
        #[cfg(feature = "keyring")]
        assert_eq!(StorageBackend::Keyring.name(), "keyring");
    }

    #[test]
    fn test_status_default() {
        let status = ClaudeStatus::default();
        assert!(!status.authenticated);
        assert_eq!(status.storage_backend, "unknown");
        assert!(status.token_expires_in.is_none());
        assert!(status.error.is_none());
    }

    #[test]
    fn test_supports_streaming() {
        let provider = ClaudeProvider::with_memory().unwrap();
        assert!(provider.supports_streaming());
        assert!(!provider.supports_embeddings());
    }

    #[test]
    fn test_pricing() {
        let provider = ClaudeProvider::with_memory().unwrap();
        // Should return pricing since it uses Claude models
        // The actual pricing depends on the model
        let pricing = provider.pricing();
        // May or may not have pricing depending on model configuration
        // Just ensure it doesn't panic
        let _ = pricing;
    }

    #[tokio::test]
    async fn test_not_authenticated() {
        let provider = ClaudeProvider::with_memory().unwrap();
        // Memory storage starts without tokens
        let is_auth = provider.is_authenticated().await.unwrap();
        assert!(!is_auth);
    }

    #[tokio::test]
    async fn test_health_check_not_authenticated() {
        let provider = ClaudeProvider::with_memory().unwrap();
        // Health check should fail when not authenticated
        let healthy = provider.health_check().await;
        assert!(!healthy);
    }

    #[tokio::test]
    async fn test_get_status() {
        let provider = ClaudeProvider::with_memory().unwrap();
        let status = provider.get_status().await;
        assert!(!status.authenticated);
        assert_eq!(status.storage_backend, "memory");
    }

    #[tokio::test]
    async fn test_chat_requires_auth() {
        let provider = ClaudeProvider::with_memory().unwrap();
        let request = ChatRequest {
            messages: vec![ChatMessage::user("Hello")],
            system_prompt: None,
            temperature: None,
            max_tokens: None,
            provider: None,
            tools: None,
            tool_choice: None,
        };

        let result = provider.chat(request).await;
        assert!(matches!(result, Err(LLMError::AuthError(_))));
    }

    #[tokio::test]
    async fn test_stream_chat_requires_auth() {
        let provider = ClaudeProvider::with_memory().unwrap();
        let request = ChatRequest {
            messages: vec![ChatMessage::user("Hello")],
            system_prompt: None,
            temperature: None,
            max_tokens: None,
            provider: None,
            tools: None,
            tool_choice: None,
        };

        let result = provider.stream_chat(request).await;
        assert!(matches!(result, Err(LLMError::AuthError(_))));
    }
}
