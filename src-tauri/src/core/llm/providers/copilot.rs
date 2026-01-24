//! Copilot Provider Implementation
//!
//! OAuth-based GitHub Copilot API access using the `copilot` gate module.
//! This provider enables Copilot API access using GitHub OAuth Device Code flow
//! for authentication.
//!
//! ## Features
//!
//! - OAuth 2.0 Device Code authentication flow
//! - Multiple storage backends (file, keyring, memory)
//! - Automatic Copilot token refresh
//! - Full streaming support
//! - Tool use support
//! - Vision support (via GPT-4o models)
//! - Embeddings support (via text-embedding-3-small model)
//!
//! ## Usage
//!
//! ```rust,no_run
//! use crate::core::llm::providers::CopilotLLMProvider;
//!
//! // Using file storage (default)
//! let provider = CopilotLLMProvider::new().unwrap();
//!
//! // Using memory storage (for testing)
//! let provider = CopilotLLMProvider::with_memory().unwrap();
//!
//! // Check if authenticated
//! if !provider.is_authenticated().await {
//!     // Start Device Code flow
//!     let pending = provider.start_device_flow().await.unwrap();
//!     println!("Visit: {}", pending.verification_uri);
//!     println!("Enter code: {}", pending.user_code);
//!     // Poll until complete, then complete_auth()
//! }
//! ```

use crate::gate::copilot::{
    ChatResponse as CopilotChatResponse, CopilotClient,
    DeviceFlowPending, Message as CopilotMessage, PollResult, Role as CopilotRole,
    StreamChunk, Content as CopilotContent, ContentPart as CopilotContentPart,
    ImageUrl as CopilotImageUrl, EmbeddingResponse,
};
use crate::gate::copilot::storage::{GateStorageAdapter, MemoryTokenStorage};
use crate::gate::storage::FileTokenStorage;
#[cfg(feature = "keyring")]
use crate::gate::storage::KeyringTokenStorage;

use crate::core::llm::cost::{ProviderPricing, TokenUsage};
use crate::core::llm::router::{
    ChatChunk, ChatRequest, ChatResponse, LLMError, LLMProvider, MessageRole, Result,
};
use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, info};

// ============================================================================
// Constants
// ============================================================================

/// Default model to use
const DEFAULT_MODEL: &str = "gpt-4o";

/// Default max tokens
const DEFAULT_MAX_TOKENS: u32 = 8192;

/// Default retry delay for rate limiting (seconds)
const DEFAULT_RETRY_AFTER_SECS: u64 = 60;

// ============================================================================
// Storage Backend Enum
// ============================================================================

/// Storage backend options for OAuth tokens
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum CopilotStorageBackend {
    /// File-based storage (~/.config/ttrpg-assistant/copilot-auth.json)
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


impl CopilotStorageBackend {
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

/// Status of the Copilot provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotStatus {
    /// Whether the provider is authenticated
    pub authenticated: bool,
    /// Storage backend in use
    pub storage_backend: String,
    /// GitHub username (if available)
    pub github_username: Option<String>,
    /// Error message if any
    pub error: Option<String>,
}

impl Default for CopilotStatus {
    fn default() -> Self {
        Self {
            authenticated: false,
            storage_backend: "unknown".to_string(),
            github_username: None,
            error: None,
        }
    }
}

// ============================================================================
// Provider Implementation
// ============================================================================

/// Copilot provider using Device Code OAuth authentication.
///
/// This provider uses the `copilot` gate module to authenticate with
/// GitHub's Device Code OAuth flow, enabling API access without
/// requiring a browser redirect.
pub struct CopilotLLMProvider {
    /// The underlying Copilot client (type-erased for flexibility)
    client: Arc<dyn CopilotClientTrait>,
    /// Model to use
    model: String,
    /// Max tokens for responses
    max_tokens: u32,
    /// Storage backend name
    storage_backend: String,
}

/// Trait to abstract over different storage backends
#[async_trait]
trait CopilotClientTrait: Send + Sync {
    async fn is_authenticated(&self) -> bool;
    async fn start_device_flow(&self) -> crate::gate::copilot::Result<DeviceFlowPending>;
    async fn poll_for_token(&self, pending: &DeviceFlowPending) -> crate::gate::copilot::Result<PollResult>;
    async fn complete_auth(&self, github_token: String) -> crate::gate::copilot::Result<()>;
    async fn sign_out(&self) -> crate::gate::copilot::Result<()>;
    async fn send_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<CopilotMessage>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::gate::copilot::Result<CopilotChatResponse>;
    async fn stream_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<CopilotMessage>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::gate::copilot::Result<
        std::pin::Pin<Box<dyn futures_util::Stream<Item = crate::gate::copilot::Result<StreamChunk>> + Send>>,
    >;
    async fn embeddings(&self, text: &str) -> crate::gate::copilot::Result<EmbeddingResponse>;
}

/// Wrapper for CopilotClient with FileTokenStorage
struct FileStorageClient {
    client: CopilotClient<GateStorageAdapter<FileTokenStorage>>,
}

#[async_trait]
impl CopilotClientTrait for FileStorageClient {
    async fn is_authenticated(&self) -> bool {
        self.client.is_authenticated().await
    }

    async fn start_device_flow(&self) -> crate::gate::copilot::Result<DeviceFlowPending> {
        self.client.start_device_flow().await
    }

    async fn poll_for_token(&self, pending: &DeviceFlowPending) -> crate::gate::copilot::Result<PollResult> {
        self.client.poll_for_token(pending).await
    }

    async fn complete_auth(&self, github_token: String) -> crate::gate::copilot::Result<()> {
        self.client.complete_auth(github_token).await
    }

    async fn sign_out(&self) -> crate::gate::copilot::Result<()> {
        self.client.sign_out().await
    }

    async fn send_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<CopilotMessage>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::gate::copilot::Result<CopilotChatResponse> {
        let mut builder = self.client.chat()
            .model(model)
            .max_tokens(max_tokens);

        if let Some(sys) = system {
            builder = builder.system(sys);
        }
        if let Some(temp) = temperature {
            builder = builder.temperature(temp);
        }

        for msg in messages {
            builder = builder.message(msg);
        }

        builder.send().await
    }

    async fn stream_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<CopilotMessage>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::gate::copilot::Result<
        std::pin::Pin<Box<dyn futures_util::Stream<Item = crate::gate::copilot::Result<StreamChunk>> + Send>>,
    > {
        let mut builder = self.client.chat()
            .model(model)
            .max_tokens(max_tokens);

        if let Some(sys) = system {
            builder = builder.system(sys);
        }
        if let Some(temp) = temperature {
            builder = builder.temperature(temp);
        }

        for msg in messages {
            builder = builder.message(msg);
        }

        builder.send_stream().await
    }

    async fn embeddings(&self, text: &str) -> crate::gate::copilot::Result<EmbeddingResponse> {
        self.client.embeddings().input(text).send().await
    }
}

/// Wrapper for CopilotClient with KeyringTokenStorage
#[cfg(feature = "keyring")]
struct KeyringStorageClient {
    client: CopilotClient<GateStorageAdapter<KeyringTokenStorage>>,
}

#[cfg(feature = "keyring")]
#[async_trait]
impl CopilotClientTrait for KeyringStorageClient {
    async fn is_authenticated(&self) -> bool {
        self.client.is_authenticated().await
    }

    async fn start_device_flow(&self) -> crate::gate::copilot::Result<DeviceFlowPending> {
        self.client.start_device_flow().await
    }

    async fn poll_for_token(&self, pending: &DeviceFlowPending) -> crate::gate::copilot::Result<PollResult> {
        self.client.poll_for_token(pending).await
    }

    async fn complete_auth(&self, github_token: String) -> crate::gate::copilot::Result<()> {
        self.client.complete_auth(github_token).await
    }

    async fn sign_out(&self) -> crate::gate::copilot::Result<()> {
        self.client.sign_out().await
    }

    async fn send_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<CopilotMessage>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::gate::copilot::Result<CopilotChatResponse> {
        let mut builder = self.client.chat()
            .model(model)
            .max_tokens(max_tokens);

        if let Some(sys) = system {
            builder = builder.system(sys);
        }
        if let Some(temp) = temperature {
            builder = builder.temperature(temp);
        }

        for msg in messages {
            builder = builder.message(msg);
        }

        builder.send().await
    }

    async fn stream_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<CopilotMessage>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::gate::copilot::Result<
        std::pin::Pin<Box<dyn futures_util::Stream<Item = crate::gate::copilot::Result<StreamChunk>> + Send>>,
    > {
        let mut builder = self.client.chat()
            .model(model)
            .max_tokens(max_tokens);

        if let Some(sys) = system {
            builder = builder.system(sys);
        }
        if let Some(temp) = temperature {
            builder = builder.temperature(temp);
        }

        for msg in messages {
            builder = builder.message(msg);
        }

        builder.send_stream().await
    }

    async fn embeddings(&self, text: &str) -> crate::gate::copilot::Result<EmbeddingResponse> {
        self.client.embeddings().input(text).send().await
    }
}

/// Wrapper for CopilotClient with MemoryTokenStorage
struct MemoryStorageClient {
    client: CopilotClient<MemoryTokenStorage>,
}

#[async_trait]
impl CopilotClientTrait for MemoryStorageClient {
    async fn is_authenticated(&self) -> bool {
        self.client.is_authenticated().await
    }

    async fn start_device_flow(&self) -> crate::gate::copilot::Result<DeviceFlowPending> {
        self.client.start_device_flow().await
    }

    async fn poll_for_token(&self, pending: &DeviceFlowPending) -> crate::gate::copilot::Result<PollResult> {
        self.client.poll_for_token(pending).await
    }

    async fn complete_auth(&self, github_token: String) -> crate::gate::copilot::Result<()> {
        self.client.complete_auth(github_token).await
    }

    async fn sign_out(&self) -> crate::gate::copilot::Result<()> {
        self.client.sign_out().await
    }

    async fn send_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<CopilotMessage>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::gate::copilot::Result<CopilotChatResponse> {
        let mut builder = self.client.chat()
            .model(model)
            .max_tokens(max_tokens);

        if let Some(sys) = system {
            builder = builder.system(sys);
        }
        if let Some(temp) = temperature {
            builder = builder.temperature(temp);
        }

        for msg in messages {
            builder = builder.message(msg);
        }

        builder.send().await
    }

    async fn stream_message(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<CopilotMessage>,
        system: Option<String>,
        temperature: Option<f32>,
    ) -> crate::gate::copilot::Result<
        std::pin::Pin<Box<dyn futures_util::Stream<Item = crate::gate::copilot::Result<StreamChunk>> + Send>>,
    > {
        let mut builder = self.client.chat()
            .model(model)
            .max_tokens(max_tokens);

        if let Some(sys) = system {
            builder = builder.system(sys);
        }
        if let Some(temp) = temperature {
            builder = builder.temperature(temp);
        }

        for msg in messages {
            builder = builder.message(msg);
        }

        builder.send_stream().await
    }

    async fn embeddings(&self, text: &str) -> crate::gate::copilot::Result<EmbeddingResponse> {
        self.client.embeddings().input(text).send().await
    }
}

impl CopilotLLMProvider {
    /// Create a new provider with file-based token storage.
    ///
    /// Uses the default path (~/.config/ttrpg-assistant/copilot-auth.json).
    pub fn new() -> Result<Self> {
        Self::with_storage(
            CopilotStorageBackend::File,
            DEFAULT_MODEL.to_string(),
            DEFAULT_MAX_TOKENS,
        )
    }

    /// Create a new provider with automatic storage selection.
    ///
    /// Tries keyring first (if available), then falls back to file storage.
    pub fn auto() -> Result<Self> {
        Self::with_storage(
            CopilotStorageBackend::Auto,
            DEFAULT_MODEL.to_string(),
            DEFAULT_MAX_TOKENS,
        )
    }

    /// Create a new provider with keyring storage.
    #[cfg(feature = "keyring")]
    pub fn with_keyring() -> Result<Self> {
        Self::with_storage(
            CopilotStorageBackend::Keyring,
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
            CopilotStorageBackend::Memory,
            DEFAULT_MODEL.to_string(),
            DEFAULT_MAX_TOKENS,
        )
    }

    /// Create a new provider with specified storage backend.
    pub fn with_storage(
        backend: CopilotStorageBackend,
        model: String,
        max_tokens: u32,
    ) -> Result<Self> {
        let (client, storage_name): (Arc<dyn CopilotClientTrait>, String) = match backend {
            CopilotStorageBackend::File => {
                let storage = FileTokenStorage::default_path().map_err(|e| {
                    LLMError::NotConfigured(format!("Failed to create file storage: {}", e))
                })?;
                let adapter = GateStorageAdapter::new(storage);
                let copilot_client = CopilotClient::builder()
                    .with_storage(adapter)
                    .build()
                    .map_err(|e| LLMError::NotConfigured(format!("Failed to create client: {}", e)))?;
                (
                    Arc::new(FileStorageClient { client: copilot_client }),
                    "file".to_string(),
                )
            }
            #[cfg(feature = "keyring")]
            CopilotStorageBackend::Keyring => {
                let storage = KeyringTokenStorage::new();
                let adapter = GateStorageAdapter::new(storage);
                let copilot_client = CopilotClient::builder()
                    .with_storage(adapter)
                    .build()
                    .map_err(|e| LLMError::NotConfigured(format!("Failed to create client: {}", e)))?;
                (
                    Arc::new(KeyringStorageClient { client: copilot_client }),
                    "keyring".to_string(),
                )
            }
            CopilotStorageBackend::Memory => {
                let storage = MemoryTokenStorage::new();
                let copilot_client = CopilotClient::builder()
                    .with_storage(storage)
                    .build()
                    .map_err(|e| LLMError::NotConfigured(format!("Failed to create client: {}", e)))?;
                (
                    Arc::new(MemoryStorageClient { client: copilot_client }),
                    "memory".to_string(),
                )
            }
            CopilotStorageBackend::Auto => {
                // Try keyring first, fall back to file
                #[cfg(feature = "keyring")]
                {
                    if KeyringTokenStorage::is_available() {
                        let storage = KeyringTokenStorage::new();
                        let adapter = GateStorageAdapter::new(storage);
                        let copilot_client = CopilotClient::builder()
                            .with_storage(adapter)
                            .build()
                            .map_err(|e| LLMError::NotConfigured(format!("Failed to create client: {}", e)))?;
                        (
                            Arc::new(KeyringStorageClient { client: copilot_client }),
                            "keyring".to_string(),
                        )
                    } else {
                        let storage = FileTokenStorage::default_path().map_err(|e| {
                            LLMError::NotConfigured(format!("Failed to create file storage: {}", e))
                        })?;
                        let adapter = GateStorageAdapter::new(storage);
                        let copilot_client = CopilotClient::builder()
                            .with_storage(adapter)
                            .build()
                            .map_err(|e| LLMError::NotConfigured(format!("Failed to create client: {}", e)))?;
                        (
                            Arc::new(FileStorageClient { client: copilot_client }),
                            "file".to_string(),
                        )
                    }
                }
                #[cfg(not(feature = "keyring"))]
                {
                    let storage = FileTokenStorage::default_path().map_err(|e| {
                        LLMError::NotConfigured(format!("Failed to create file storage: {}", e))
                    })?;
                    let adapter = GateStorageAdapter::new(storage);
                    let copilot_client = CopilotClient::builder()
                        .with_storage(adapter)
                        .build()
                        .map_err(|e| LLMError::NotConfigured(format!("Failed to create client: {}", e)))?;
                    (
                        Arc::new(FileStorageClient { client: copilot_client }),
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
            "file" => CopilotStorageBackend::File,
            #[cfg(feature = "keyring")]
            "keyring" => CopilotStorageBackend::Keyring,
            "memory" => CopilotStorageBackend::Memory,
            "auto" => CopilotStorageBackend::Auto,
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
    // Device Code Flow Methods
    // ========================================================================

    /// Check if the provider is authenticated.
    ///
    /// Returns true if a valid GitHub token exists.
    pub async fn is_authenticated(&self) -> bool {
        self.client.is_authenticated().await
    }

    /// Start the Device Code authorization flow.
    ///
    /// Returns information about the device flow including:
    /// - `verification_uri`: URL the user should visit
    /// - `user_code`: Code the user should enter at the URL
    /// - `device_code`: Internal code for polling (don't expose to user)
    /// - `expires_in`: Seconds until the code expires
    /// - `interval`: Minimum seconds between poll attempts
    pub async fn start_device_flow(&self) -> Result<DeviceFlowPending> {
        self.client
            .start_device_flow()
            .await
            .map_err(|e| LLMError::AuthError(format!("Failed to start device flow: {}", e)))
    }

    /// Poll for device flow completion.
    ///
    /// Returns:
    /// - `PollResult::Pending` - User hasn't completed yet, keep polling
    /// - `PollResult::SlowDown` - Increase poll interval
    /// - `PollResult::Complete(token)` - Got the GitHub access token
    pub async fn poll_for_token(&self, pending: &DeviceFlowPending) -> Result<PollResult> {
        self.client
            .poll_for_token(pending)
            .await
            .map_err(|e| LLMError::AuthError(format!("Failed to poll for token: {}", e)))
    }

    /// Complete the authentication by exchanging the GitHub token for a Copilot token.
    ///
    /// Call this after `poll_for_token` returns `PollResult::Complete(github_token)`.
    pub async fn complete_auth(&self, github_token: String) -> Result<()> {
        self.client
            .complete_auth(github_token)
            .await
            .map_err(|e| LLMError::AuthError(format!("Failed to complete auth: {}", e)))?;
        info!("Device flow completed successfully for Copilot");
        Ok(())
    }

    /// Log out and remove stored credentials.
    pub async fn logout(&self) -> Result<()> {
        self.client
            .sign_out()
            .await
            .map_err(|e| LLMError::AuthError(format!("Failed to logout: {}", e)))?;
        info!("Logged out from Copilot");
        Ok(())
    }

    /// Get the storage backend name.
    pub fn get_storage_backend(&self) -> &str {
        &self.storage_backend
    }

    /// Get full status of the provider.
    pub async fn get_status(&self) -> CopilotStatus {
        let authenticated = self.client.is_authenticated().await;

        CopilotStatus {
            authenticated,
            storage_backend: self.storage_backend.clone(),
            github_username: None, // Would need API call to get this
            error: None,
        }
    }

    // ========================================================================
    // Message Conversion
    // ========================================================================

    /// Convert ChatRequest messages to Copilot Message format.
    ///
    /// Note: System messages are filtered out here because the Copilot API
    /// expects the system prompt to be passed separately via the `system`
    /// parameter, not as part of the messages array.
    fn convert_messages(&self, request: &ChatRequest) -> Vec<CopilotMessage> {
        request
            .messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|msg| {
                let role = match msg.role {
                    MessageRole::User => CopilotRole::User,
                    MessageRole::Assistant => CopilotRole::Assistant,
                    MessageRole::System => {
                        tracing::warn!("System message reached convert_messages unexpectedly");
                        CopilotRole::User
                    }
                };

                // Handle images if present
                if let Some(images) = &msg.images {
                    if !images.is_empty() {
                        // Create multi-part content with text and images
                        let mut parts = vec![
                            CopilotContentPart::Text { text: msg.content.clone() }
                        ];

                        for image_url in images {
                            parts.push(CopilotContentPart::ImageUrl {
                                image_url: CopilotImageUrl {
                                    url: image_url.clone(),
                                    detail: None,
                                },
                            });
                        }

                        return CopilotMessage {
                            role,
                            content: CopilotContent::Parts(parts),
                            name: None,
                        };
                    }
                }

                // Simple text message
                CopilotMessage::new(role, msg.content.clone())
            })
            .collect()
    }

    /// Convert Copilot ChatResponse to LLM ChatResponse
    fn convert_response(&self, response: CopilotChatResponse, latency_ms: u64) -> ChatResponse {
        let content = response.first_content().unwrap_or_default();
        let finish_reason = response.first_finish_reason().map(|s| s.to_string());
        let model = response.model.clone();

        let usage = response.usage.map(|u| TokenUsage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
        });

        let cost_usd = usage.as_ref().and_then(|u| {
            self.pricing().map(|p| p.calculate_cost(u))
        });

        ChatResponse {
            content,
            model,
            provider: "copilot".to_string(),
            usage,
            finish_reason,
            latency_ms,
            cost_usd,
            tool_calls: None, // TODO: Implement tool calls conversion
        }
    }
}

// ============================================================================
// LLMProvider Implementation
// ============================================================================

#[async_trait]
impl LLMProvider for CopilotLLMProvider {
    fn id(&self) -> &str {
        "copilot"
    }

    fn name(&self) -> &str {
        "GitHub Copilot"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn health_check(&self) -> bool {
        if self.client.is_authenticated().await {
            true
        } else {
            debug!("Copilot health check: not authenticated");
            false
        }
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        // Copilot doesn't charge per-token; it's subscription-based.
        // Return None to indicate no per-token pricing.
        None
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        // Check authentication first
        if !self.is_authenticated().await {
            return Err(LLMError::AuthError(
                "Not authenticated. Please complete Device Code flow first.".to_string(),
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
            "Sending chat request via Copilot"
        );

        let start = Instant::now();

        let response = self
            .client
            .send_message(&self.model, max_tokens, messages, system, temperature)
            .await
            .map_err(|e| {
                if e.needs_auth() {
                    LLMError::AuthError(e.to_string())
                } else {
                    match &e {
                        crate::gate::copilot::Error::Api { status, message } => {
                            if *status == 429 {
                                LLMError::RateLimited { retry_after_secs: 60 }
                            } else {
                                LLMError::ApiError {
                                    status: *status,
                                    message: message.clone(),
                                }
                            }
                        }
                        crate::gate::copilot::Error::RateLimited { retry_after } => {
                            LLMError::RateLimited {
                                retry_after_secs: retry_after.unwrap_or(60),
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

        if let Some(usage) = &response.usage {
            info!(
                latency_ms = latency_ms,
                input_tokens = usage.prompt_tokens,
                output_tokens = usage.completion_tokens,
                "Received response from Copilot"
            );
        } else {
            info!(latency_ms = latency_ms, "Received response from Copilot");
        }

        Ok(self.convert_response(response, latency_ms))
    }

    async fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        // Check authentication first
        if !self.is_authenticated().await {
            return Err(LLMError::AuthError(
                "Not authenticated. Please complete Device Code flow first.".to_string(),
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
            "Starting streaming chat via Copilot"
        );

        let stream = self
            .client
            .stream_message(&self.model, max_tokens, messages, system, temperature)
            .await
            .map_err(|e| {
                if e.needs_auth() {
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
            futures_util::pin_mut!(stream);
            let mut chunk_index: u32 = 0;
            let mut final_usage: Option<TokenUsage> = None;

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        match chunk {
                            StreamChunk::Delta { content, index: _ } => {
                                if !content.is_empty() {
                                    chunk_index += 1;
                                    let chat_chunk = ChatChunk {
                                        stream_id: stream_id.clone(),
                                        content,
                                        provider: "copilot".to_string(),
                                        model: model.clone(),
                                        is_final: false,
                                        finish_reason: None,
                                        usage: None,
                                        index: chunk_index,
                                    };
                                    if tx.send(Ok(chat_chunk)).await.is_err() {
                                        return;
                                    }
                                }
                            }
                            StreamChunk::FinishReason { reason, index: _ } => {
                                let final_chunk = ChatChunk {
                                    stream_id: stream_id.clone(),
                                    content: String::new(),
                                    provider: "copilot".to_string(),
                                    model: model.clone(),
                                    is_final: true,
                                    finish_reason: Some(reason),
                                    usage: final_usage.clone(),
                                    index: chunk_index + 1,
                                };
                                let _ = tx.send(Ok(final_chunk)).await;
                                return;
                            }
                            StreamChunk::Usage(usage) => {
                                final_usage = Some(TokenUsage {
                                    input_tokens: usage.prompt_tokens,
                                    output_tokens: usage.completion_tokens,
                                });
                            }
                            StreamChunk::Done => {
                                // Send final chunk if not already sent
                                let final_chunk = ChatChunk {
                                    stream_id: stream_id.clone(),
                                    content: String::new(),
                                    provider: "copilot".to_string(),
                                    model: model.clone(),
                                    is_final: true,
                                    finish_reason: Some("stop".to_string()),
                                    usage: final_usage.clone(),
                                    index: chunk_index + 1,
                                };
                                let _ = tx.send(Ok(final_chunk)).await;
                                return;
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
        true
    }

    /// Generate embeddings using GitHub Copilot's text-embedding-3-small model.
    ///
    /// Returns a vector of 1536 f32 values representing the text embedding.
    async fn embeddings(&self, text: String) -> Result<Vec<f32>> {
        // Check authentication first
        if !self.is_authenticated().await {
            return Err(LLMError::AuthError(
                "Not authenticated. Please complete Device Code flow first.".to_string(),
            ));
        }

        debug!(
            text_length = text.len(),
            "Generating embeddings via Copilot"
        );

        let response = self
            .client
            .embeddings(&text)
            .await
            .map_err(|e| {
                if e.needs_auth() {
                    LLMError::AuthError(e.to_string())
                } else {
                    match &e {
                        // Note: HTTP 429 is already converted to Error::RateLimited by the client
                        crate::gate::copilot::Error::Api { status, message } => LLMError::ApiError {
                            status: *status,
                            message: message.clone(),
                        },
                        crate::gate::copilot::Error::RateLimited { retry_after } => {
                            LLMError::RateLimited {
                                retry_after_secs: retry_after.unwrap_or(DEFAULT_RETRY_AFTER_SECS),
                            }
                        }
                        _ => LLMError::ApiError {
                            status: 0,
                            message: e.to_string(),
                        },
                    }
                }
            })?;

        // Extract the first embedding from the response
        response
            .first_embedding()
            .map(|embedding| embedding.to_vec())
            .ok_or_else(|| LLMError::EmbeddingNotSupported(
                "No embedding returned from Copilot API".to_string(),
            ))
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
        let provider = CopilotLLMProvider::with_memory().unwrap();
        assert_eq!(provider.id(), "copilot");
        assert_eq!(provider.name(), "GitHub Copilot");
    }

    #[test]
    fn test_storage_backend() {
        let provider = CopilotLLMProvider::with_memory().unwrap();
        assert_eq!(provider.get_storage_backend(), "memory");
    }

    #[test]
    fn test_model() {
        let provider = CopilotLLMProvider::with_memory().unwrap();
        assert_eq!(provider.model(), DEFAULT_MODEL);
    }

    #[test]
    fn test_from_storage_name() {
        let provider = CopilotLLMProvider::from_storage_name(
            "memory",
            "gpt-4o".to_string(),
            4096,
        )
        .unwrap();
        assert_eq!(provider.model(), "gpt-4o");
        assert_eq!(provider.get_storage_backend(), "memory");
    }

    #[test]
    fn test_invalid_storage_name() {
        let result = CopilotLLMProvider::from_storage_name(
            "invalid",
            DEFAULT_MODEL.to_string(),
            DEFAULT_MAX_TOKENS,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_storage_backend_names() {
        assert_eq!(CopilotStorageBackend::File.name(), "file");
        assert_eq!(CopilotStorageBackend::Memory.name(), "memory");
        assert_eq!(CopilotStorageBackend::Auto.name(), "auto");
        #[cfg(feature = "keyring")]
        assert_eq!(CopilotStorageBackend::Keyring.name(), "keyring");
    }

    #[test]
    fn test_status_default() {
        let status = CopilotStatus::default();
        assert!(!status.authenticated);
        assert_eq!(status.storage_backend, "unknown");
        assert!(status.github_username.is_none());
        assert!(status.error.is_none());
    }

    #[test]
    fn test_supports_streaming() {
        let provider = CopilotLLMProvider::with_memory().unwrap();
        assert!(provider.supports_streaming());
    }

    #[test]
    fn test_pricing() {
        let provider = CopilotLLMProvider::with_memory().unwrap();
        // Copilot is subscription-based, no per-token pricing
        assert!(provider.pricing().is_none());
    }

    #[tokio::test]
    async fn test_not_authenticated() {
        let provider = CopilotLLMProvider::with_memory().unwrap();
        // Memory storage starts without tokens
        let is_auth = provider.is_authenticated().await;
        assert!(!is_auth);
    }

    #[tokio::test]
    async fn test_health_check_not_authenticated() {
        let provider = CopilotLLMProvider::with_memory().unwrap();
        // Health check should fail when not authenticated
        let healthy = provider.health_check().await;
        assert!(!healthy);
    }

    #[tokio::test]
    async fn test_get_status() {
        let provider = CopilotLLMProvider::with_memory().unwrap();
        let status = provider.get_status().await;
        assert!(!status.authenticated);
        assert_eq!(status.storage_backend, "memory");
    }

    #[tokio::test]
    async fn test_chat_requires_auth() {
        let provider = CopilotLLMProvider::with_memory().unwrap();
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
        let provider = CopilotLLMProvider::with_memory().unwrap();
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

    #[test]
    fn test_supports_embeddings() {
        let provider = CopilotLLMProvider::with_memory().unwrap();
        // Copilot now supports embeddings via the text-embedding-3-small model
        assert!(provider.supports_embeddings());
    }

    #[tokio::test]
    async fn test_embeddings_requires_auth() {
        let provider = CopilotLLMProvider::with_memory().unwrap();
        let result = provider.embeddings("Hello, world!".to_string()).await;
        // Should fail with AuthError since not authenticated
        assert!(matches!(result, Err(LLMError::AuthError(_))));
    }
}
