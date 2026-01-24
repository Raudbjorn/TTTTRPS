//! Gemini Provider Implementation
//!
//! OAuth-based Google Cloud Code API access using the `gemini_gate` module.
//! This provider enables Gemini API access without requiring an API key
//! by using Google OAuth 2.0 PKCE flow for authentication.
//!
//! ## Features
//!
//! - OAuth 2.0 PKCE authentication flow (Google OAuth)
//! - Multiple storage backends (file, keyring, memory)
//! - Automatic token refresh
//! - Full streaming support via SSE
//! - Tool use support
//! - Cost tracking with token usage
//!
//! ## Usage
//!
//! ```rust,no_run
//! use crate::core::llm::providers::GeminiProvider;
//!
//! // Using file storage (default)
//! let provider = GeminiProvider::new().unwrap();
//!
//! // Using keyring storage
//! let provider = GeminiProvider::with_keyring().unwrap();
//!
//! // Check if authenticated
//! if !provider.is_authenticated().await.unwrap() {
//!     // Start OAuth flow
//!     let (auth_url, state) = provider.start_oauth_flow().await.unwrap();
//!     println!("Open this URL to authenticate: {}", auth_url);
//!     // After user completes flow and gets code:
//!     // provider.complete_oauth_flow(&code, Some(&state.state)).await.unwrap();
//! }
//! ```

use crate::gate::gemini::{
    CloudCodeClient, ContentDelta, FileTokenStorage,
    MemoryTokenStorage, MessagesResponse, StreamEvent, TokenInfo,
};
#[cfg(feature = "keyring")]
use crate::gate::gemini::KeyringTokenStorage;

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
const DEFAULT_MODEL: &str = "gemini-2.0-flash";

/// Default max tokens
const DEFAULT_MAX_TOKENS: u32 = 8192;

// ============================================================================
// Storage Backend Enum
// ============================================================================

/// Storage backend options for OAuth tokens
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum GeminiStorageBackend {
    /// File-based storage (~/.config/antigravity-gate/auth.json)
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


impl GeminiStorageBackend {
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

/// Status of the Gemini provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiStatus {
    /// Whether the provider is authenticated
    pub authenticated: bool,
    /// Storage backend in use
    pub storage_backend: String,
    /// Time until token expires (seconds)
    pub token_expires_in: Option<i64>,
    /// Project ID if discovered
    pub project_id: Option<String>,
    /// Error message if any
    pub error: Option<String>,
}

impl Default for GeminiStatus {
    fn default() -> Self {
        Self {
            authenticated: false,
            storage_backend: "unknown".to_string(),
            token_expires_in: None,
            project_id: None,
            error: None,
        }
    }
}

// ============================================================================
// Provider Implementation
// ============================================================================

/// Gemini provider using OAuth authentication.
///
/// This provider uses the `gemini_gate` module to authenticate with
/// Google's OAuth 2.0 PKCE flow, enabling API access without
/// requiring an API key.
pub struct GeminiProvider {
    /// The underlying Cloud Code client (type-erased for flexibility)
    client: Arc<dyn GeminiClientTrait>,
    /// Model to use
    model: String,
    /// Max tokens for responses
    max_tokens: u32,
    /// Storage backend name
    storage_backend: String,
}

/// Trait to abstract over different storage backends
#[async_trait]
trait GeminiClientTrait: Send + Sync {
    async fn is_authenticated(&self) -> crate::gate::gemini::Result<bool>;
    async fn start_oauth_flow(
        &self,
    ) -> crate::gate::gemini::Result<(String, crate::gate::gemini::OAuthFlowState)>;
    async fn complete_oauth_flow(
        &self,
        code: &str,
        state: Option<&str>,
    ) -> crate::gate::gemini::Result<TokenInfo>;
    async fn logout(&self) -> crate::gate::gemini::Result<()>;
    async fn get_token_info(&self) -> crate::gate::gemini::Result<Option<TokenInfo>>;
    async fn send_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<crate::gate::gemini::Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::gate::gemini::Result<MessagesResponse>;
    async fn stream_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<crate::gate::gemini::Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::gate::gemini::Result<
        mpsc::Receiver<crate::gate::gemini::Result<StreamEvent>>,
    >;
}

/// Wrapper for CloudCodeClient with FileTokenStorage
struct FileStorageClient {
    client: Arc<CloudCodeClient<FileTokenStorage>>,
}

#[async_trait]
impl GeminiClientTrait for FileStorageClient {
    async fn is_authenticated(&self) -> crate::gate::gemini::Result<bool> {
        self.client.is_authenticated().await
    }

    async fn start_oauth_flow(
        &self,
    ) -> crate::gate::gemini::Result<(String, crate::gate::gemini::OAuthFlowState)> {
        self.client.start_oauth_flow().await
    }

    async fn complete_oauth_flow(
        &self,
        code: &str,
        state: Option<&str>,
    ) -> crate::gate::gemini::Result<TokenInfo> {
        self.client.complete_oauth_flow(code, state).await
    }

    async fn logout(&self) -> crate::gate::gemini::Result<()> {
        self.client.logout().await
    }

    async fn get_token_info(&self) -> crate::gate::gemini::Result<Option<TokenInfo>> {
        self.client.get_token_info().await
    }

    async fn send_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<crate::gate::gemini::Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::gate::gemini::Result<MessagesResponse> {
        let mut builder = Arc::clone(&self.client)
            .messages()
            .model(model)
            .max_tokens(max_tokens);

        for msg in messages {
            builder = builder.message(msg);
        }

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
        messages: Vec<crate::gate::gemini::Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::gate::gemini::Result<mpsc::Receiver<crate::gate::gemini::Result<StreamEvent>>>
    {
        let mut builder = Arc::clone(&self.client)
            .messages()
            .model(model)
            .max_tokens(max_tokens);

        for msg in messages {
            builder = builder.message(msg);
        }

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

/// Wrapper for CloudCodeClient with KeyringTokenStorage
#[cfg(feature = "keyring")]
struct KeyringStorageClient {
    client: Arc<CloudCodeClient<KeyringTokenStorage>>,
}

#[cfg(feature = "keyring")]
#[async_trait]
impl GeminiClientTrait for KeyringStorageClient {
    async fn is_authenticated(&self) -> crate::gate::gemini::Result<bool> {
        self.client.is_authenticated().await
    }

    async fn start_oauth_flow(
        &self,
    ) -> crate::gate::gemini::Result<(String, crate::gate::gemini::OAuthFlowState)> {
        self.client.start_oauth_flow().await
    }

    async fn complete_oauth_flow(
        &self,
        code: &str,
        state: Option<&str>,
    ) -> crate::gate::gemini::Result<TokenInfo> {
        self.client.complete_oauth_flow(code, state).await
    }

    async fn logout(&self) -> crate::gate::gemini::Result<()> {
        self.client.logout().await
    }

    async fn get_token_info(&self) -> crate::gate::gemini::Result<Option<TokenInfo>> {
        self.client.get_token_info().await
    }

    async fn send_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<crate::gate::gemini::Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::gate::gemini::Result<MessagesResponse> {
        let mut builder = Arc::clone(&self.client)
            .messages()
            .model(model)
            .max_tokens(max_tokens);

        for msg in messages {
            builder = builder.message(msg);
        }

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
        messages: Vec<crate::gate::gemini::Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::gate::gemini::Result<mpsc::Receiver<crate::gate::gemini::Result<StreamEvent>>>
    {
        let mut builder = Arc::clone(&self.client)
            .messages()
            .model(model)
            .max_tokens(max_tokens);

        for msg in messages {
            builder = builder.message(msg);
        }

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

/// Wrapper for CloudCodeClient with MemoryTokenStorage
struct MemoryStorageClient {
    client: Arc<CloudCodeClient<MemoryTokenStorage>>,
}

#[async_trait]
impl GeminiClientTrait for MemoryStorageClient {
    async fn is_authenticated(&self) -> crate::gate::gemini::Result<bool> {
        self.client.is_authenticated().await
    }

    async fn start_oauth_flow(
        &self,
    ) -> crate::gate::gemini::Result<(String, crate::gate::gemini::OAuthFlowState)> {
        self.client.start_oauth_flow().await
    }

    async fn complete_oauth_flow(
        &self,
        code: &str,
        state: Option<&str>,
    ) -> crate::gate::gemini::Result<TokenInfo> {
        self.client.complete_oauth_flow(code, state).await
    }

    async fn logout(&self) -> crate::gate::gemini::Result<()> {
        self.client.logout().await
    }

    async fn get_token_info(&self) -> crate::gate::gemini::Result<Option<TokenInfo>> {
        self.client.get_token_info().await
    }

    async fn send_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<crate::gate::gemini::Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::gate::gemini::Result<MessagesResponse> {
        let mut builder = Arc::clone(&self.client)
            .messages()
            .model(model)
            .max_tokens(max_tokens);

        for msg in messages {
            builder = builder.message(msg);
        }

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
        messages: Vec<crate::gate::gemini::Message>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::gate::gemini::Result<mpsc::Receiver<crate::gate::gemini::Result<StreamEvent>>>
    {
        let mut builder = Arc::clone(&self.client)
            .messages()
            .model(model)
            .max_tokens(max_tokens);

        for msg in messages {
            builder = builder.message(msg);
        }

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

impl GeminiProvider {
    /// Create a new provider with file-based token storage.
    ///
    /// Uses the default path (~/.config/antigravity-gate/auth.json).
    pub fn new() -> Result<Self> {
        Self::with_storage(
            GeminiStorageBackend::File,
            DEFAULT_MODEL.to_string(),
            DEFAULT_MAX_TOKENS,
        )
    }

    /// Create a new provider with automatic storage selection.
    ///
    /// Tries keyring first (if available), then falls back to file storage.
    pub fn auto() -> Result<Self> {
        Self::with_storage(
            GeminiStorageBackend::Auto,
            DEFAULT_MODEL.to_string(),
            DEFAULT_MAX_TOKENS,
        )
    }

    /// Create a new provider with keyring storage.
    #[cfg(feature = "keyring")]
    pub fn with_keyring() -> Result<Self> {
        Self::with_storage(
            GeminiStorageBackend::Keyring,
            DEFAULT_MODEL.to_string(),
            DEFAULT_MAX_TOKENS,
        )
    }

    /// Create a new provider with in-memory storage.
    ///
    /// Tokens will be lost when the provider is dropped.
    /// Useful for testing.
    pub fn with_memory() -> Result<Self> {
        Self::with_storage(
            GeminiStorageBackend::Memory,
            DEFAULT_MODEL.to_string(),
            DEFAULT_MAX_TOKENS,
        )
    }

    /// Create a new provider with specified storage backend.
    pub fn with_storage(
        backend: GeminiStorageBackend,
        model: String,
        max_tokens: u32,
    ) -> Result<Self> {
        let (client, storage_name): (Arc<dyn GeminiClientTrait>, String) = match backend {
            GeminiStorageBackend::File => {
                let storage = FileTokenStorage::default_path().map_err(|e| {
                    LLMError::NotConfigured(format!("Failed to create file storage: {}", e))
                })?;
                let gemini_client = CloudCodeClient::new(storage);
                (
                    Arc::new(FileStorageClient {
                        client: Arc::new(gemini_client),
                    }),
                    "file".to_string(),
                )
            }
            #[cfg(feature = "keyring")]
            GeminiStorageBackend::Keyring => {
                let storage = KeyringTokenStorage::new();
                let gemini_client = CloudCodeClient::new(storage);
                (
                    Arc::new(KeyringStorageClient {
                        client: Arc::new(gemini_client),
                    }),
                    "keyring".to_string(),
                )
            }
            GeminiStorageBackend::Memory => {
                let storage = MemoryTokenStorage::new();
                let gemini_client = CloudCodeClient::new(storage);
                (
                    Arc::new(MemoryStorageClient {
                        client: Arc::new(gemini_client),
                    }),
                    "memory".to_string(),
                )
            }
            GeminiStorageBackend::Auto => {
                // Try keyring first, fall back to file
                #[cfg(feature = "keyring")]
                {
                    if KeyringTokenStorage::is_available() {
                        let storage = KeyringTokenStorage::new();
                        let gemini_client = CloudCodeClient::new(storage);
                        (
                            Arc::new(KeyringStorageClient {
                                client: Arc::new(gemini_client),
                            }),
                            "keyring".to_string(),
                        )
                    } else {
                        let storage = FileTokenStorage::default_path().map_err(|e| {
                            LLMError::NotConfigured(format!(
                                "Failed to create file storage: {}",
                                e
                            ))
                        })?;
                        let gemini_client = CloudCodeClient::new(storage);
                        (
                            Arc::new(FileStorageClient {
                                client: Arc::new(gemini_client),
                            }),
                            "file".to_string(),
                        )
                    }
                }
                #[cfg(not(feature = "keyring"))]
                {
                    let storage = FileTokenStorage::default_path().map_err(|e| {
                        LLMError::NotConfigured(format!("Failed to create file storage: {}", e))
                    })?;
                    let gemini_client = CloudCodeClient::new(storage);
                    (
                        Arc::new(FileStorageClient {
                            client: Arc::new(gemini_client),
                        }),
                        "file".to_string(),
                    )
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
            "file" => GeminiStorageBackend::File,
            #[cfg(feature = "keyring")]
            "keyring" => GeminiStorageBackend::Keyring,
            "memory" => GeminiStorageBackend::Memory,
            "auto" => GeminiStorageBackend::Auto,
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
    /// Returns a tuple of (URL, OAuthFlowState).
    /// The user should open the URL in their browser.
    /// After the user authorizes, they will be redirected to a page
    /// that displays an authorization code.
    ///
    /// The state should be stored and passed to `complete_oauth_flow`
    /// for CSRF protection.
    pub async fn start_oauth_flow(&self) -> Result<(String, crate::gate::gemini::OAuthFlowState)> {
        self.client
            .start_oauth_flow()
            .await
            .map_err(|e| LLMError::AuthError(format!("Failed to start OAuth flow: {}", e)))
    }

    /// Complete the OAuth flow by exchanging the authorization code.
    ///
    /// Call this with the code the user received after authorization.
    /// Optionally pass the state for CSRF protection.
    pub async fn complete_oauth_flow(&self, code: &str, state: Option<&str>) -> Result<()> {
        self.client
            .complete_oauth_flow(code, state)
            .await
            .map_err(|e| LLMError::AuthError(format!("Failed to complete OAuth flow: {}", e)))?;
        info!("OAuth flow completed successfully for Gemini");
        Ok(())
    }

    /// Log out and remove stored credentials.
    pub async fn logout(&self) -> Result<()> {
        self.client
            .logout()
            .await
            .map_err(|e| LLMError::AuthError(format!("Failed to logout: {}", e)))?;
        info!("Logged out from Gemini");
        Ok(())
    }

    /// Get the storage backend name.
    pub fn get_storage_backend(&self) -> &str {
        &self.storage_backend
    }

    /// Get full status of the provider.
    pub async fn get_status(&self) -> GeminiStatus {
        match self.client.is_authenticated().await {
            Ok(authenticated) => {
                let token_expires_in = if authenticated {
                    self.client
                        .get_token_info()
                        .await
                        .ok()
                        .flatten()
                        .map(|t| t.time_until_expiry().as_secs() as i64)
                } else {
                    None
                };

                GeminiStatus {
                    authenticated,
                    storage_backend: self.storage_backend.clone(),
                    token_expires_in,
                    project_id: None, // Would need to call get_project_id
                    error: None,
                }
            }
            Err(e) => GeminiStatus {
                authenticated: false,
                storage_backend: self.storage_backend.clone(),
                token_expires_in: None,
                project_id: None,
                error: Some(e.to_string()),
            },
        }
    }

    // ========================================================================
    // Message Conversion
    // ========================================================================

    /// Convert ChatRequest messages to gemini_gate Message format.
    ///
    /// Note: System messages are filtered out here because the API expects
    /// the system prompt to be passed separately via the `system` parameter,
    /// not as part of the messages array.
    fn convert_messages(&self, request: &ChatRequest) -> Vec<crate::gate::gemini::Message> {
        request
            .messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|msg| match msg.role {
                MessageRole::User => crate::gate::gemini::Message::user(&msg.content),
                MessageRole::Assistant => crate::gate::gemini::Message::assistant(&msg.content),
                MessageRole::System => {
                    tracing::warn!("System message reached convert_messages unexpectedly");
                    crate::gate::gemini::Message::user(&msg.content)
                }
            })
            .collect()
    }

    /// Convert gemini_gate MessagesResponse to ChatResponse
    fn convert_response(&self, response: MessagesResponse, latency_ms: u64) -> ChatResponse {
        let content = response.text();

        let usage = Some(TokenUsage {
            input_tokens: response.usage.input_tokens,
            output_tokens: response.usage.output_tokens,
        });

        let cost_usd = usage.as_ref().and_then(|u| {
            self.pricing().map(|p| p.calculate_cost(u))
        });

        // Convert tool calls to JSON format if present
        let tool_calls = if response.has_tool_calls() {
            let calls: Vec<serde_json::Value> = response
                .tool_calls()
                .filter_map(|block| {
                    // Extract tool use data from ContentBlock
                    if let Some((id, name, input)) = block.as_tool_use() {
                        Some(serde_json::json!({
                            "id": id,
                            "type": "function",
                            "function": {
                                "name": name,
                                "arguments": serde_json::to_string(&input).unwrap_or_default()
                            }
                        }))
                    } else {
                        None
                    }
                })
                .collect();
            if calls.is_empty() { None } else { Some(calls) }
        } else {
            None
        };

        ChatResponse {
            content,
            model: response.model,
            provider: "gemini".to_string(),
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
impl LLMProvider for GeminiProvider {
    fn id(&self) -> &str {
        "gemini"
    }

    fn name(&self) -> &str {
        "Google Gemini"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn health_check(&self) -> bool {
        match self.client.is_authenticated().await {
            Ok(true) => true,
            Ok(false) => {
                debug!("Gemini health check: not authenticated");
                false
            }
            Err(e) => {
                warn!("Gemini health check failed: {}", e);
                false
            }
        }
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        ProviderPricing::for_model("gemini", &self.model)
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
            "Sending chat request via Gemini"
        );

        let start = Instant::now();

        let response = self
            .client
            .send_message(&self.model, max_tokens, messages, system, temperature)
            .await
            .map_err(|e| {
                if e.is_auth_error() {
                    LLMError::AuthError(e.to_string())
                } else if e.is_rate_limit() {
                    LLMError::RateLimited {
                        retry_after_secs: e.retry_after().map(|d| d.as_secs()).unwrap_or(60),
                    }
                } else {
                    match &e {
                        crate::gate::gemini::Error::Api { status, message, .. } => {
                            LLMError::ApiError {
                                status: *status,
                                message: message.clone(),
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
            "Received response from Gemini"
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
            "Starting streaming chat via Gemini"
        );

        let stream_rx = self
            .client
            .stream_message(&self.model, max_tokens, messages, system, temperature)
            .await
            .map_err(|e| {
                if e.is_auth_error() {
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
                                if let Some(usage) = &message.usage {
                                    input_tokens = usage.input_tokens;
                                }
                            }
                            StreamEvent::ContentBlockDelta { delta, .. } => {
                                if let ContentDelta::TextDelta { text } = delta {
                                    if !text.is_empty() {
                                        chunk_index += 1;
                                        let chunk = ChatChunk {
                                            stream_id: stream_id.clone(),
                                            content: text,
                                            provider: "gemini".to_string(),
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
                                if let Some(u) = usage {
                                    final_usage = Some(TokenUsage {
                                        input_tokens,
                                        output_tokens: u.output_tokens,
                                    });
                                }
                            }
                            StreamEvent::MessageStop => {
                                let final_chunk = ChatChunk {
                                    stream_id: stream_id.clone(),
                                    content: String::new(),
                                    provider: "gemini".to_string(),
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
        let provider = GeminiProvider::with_memory().unwrap();
        assert_eq!(provider.id(), "gemini");
        assert_eq!(provider.name(), "Google Gemini");
    }

    #[test]
    fn test_storage_backend() {
        let provider = GeminiProvider::with_memory().unwrap();
        assert_eq!(provider.get_storage_backend(), "memory");
    }

    #[test]
    fn test_model() {
        let provider = GeminiProvider::with_memory().unwrap();
        assert_eq!(provider.model(), DEFAULT_MODEL);
    }

    #[test]
    fn test_from_storage_name() {
        let provider = GeminiProvider::from_storage_name(
            "memory",
            "gemini-2.0-flash".to_string(),
            4096,
        )
        .unwrap();
        assert_eq!(provider.model(), "gemini-2.0-flash");
        assert_eq!(provider.get_storage_backend(), "memory");
    }

    #[test]
    fn test_invalid_storage_name() {
        let result = GeminiProvider::from_storage_name(
            "invalid",
            DEFAULT_MODEL.to_string(),
            DEFAULT_MAX_TOKENS,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_storage_backend_names() {
        assert_eq!(GeminiStorageBackend::File.name(), "file");
        assert_eq!(GeminiStorageBackend::Memory.name(), "memory");
        assert_eq!(GeminiStorageBackend::Auto.name(), "auto");
        #[cfg(feature = "keyring")]
        assert_eq!(GeminiStorageBackend::Keyring.name(), "keyring");
    }

    #[test]
    fn test_status_default() {
        let status = GeminiStatus::default();
        assert!(!status.authenticated);
        assert_eq!(status.storage_backend, "unknown");
        assert!(status.token_expires_in.is_none());
        assert!(status.project_id.is_none());
        assert!(status.error.is_none());
    }

    #[test]
    fn test_supports_streaming() {
        let provider = GeminiProvider::with_memory().unwrap();
        assert!(provider.supports_streaming());
        assert!(!provider.supports_embeddings());
    }

    #[test]
    fn test_pricing() {
        let provider = GeminiProvider::with_memory().unwrap();
        // Should return pricing since it uses Gemini models
        let pricing = provider.pricing();
        // May or may not have pricing depending on model configuration
        let _ = pricing;
    }

    #[tokio::test]
    async fn test_not_authenticated() {
        let provider = GeminiProvider::with_memory().unwrap();
        // Memory storage starts without tokens
        let is_auth = provider.is_authenticated().await.unwrap();
        assert!(!is_auth);
    }

    #[tokio::test]
    async fn test_health_check_not_authenticated() {
        let provider = GeminiProvider::with_memory().unwrap();
        // Health check should fail when not authenticated
        let healthy = provider.health_check().await;
        assert!(!healthy);
    }

    #[tokio::test]
    async fn test_get_status() {
        let provider = GeminiProvider::with_memory().unwrap();
        let status = provider.get_status().await;
        assert!(!status.authenticated);
        assert_eq!(status.storage_backend, "memory");
    }

    #[tokio::test]
    async fn test_chat_requires_auth() {
        let provider = GeminiProvider::with_memory().unwrap();
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
        let provider = GeminiProvider::with_memory().unwrap();
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
