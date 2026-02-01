//! Copilot API client.
//!
//! This module provides the main client for interacting with the GitHub Copilot API,
//! including authentication, chat completions, embeddings, and model listing.

use futures_util::Stream;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use reqwest::Method;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, instrument, warn};

use crate::oauth::copilot::api::chat::ChatRequestBuilder;
use crate::oauth::copilot::api::embeddings::EmbeddingsRequestBuilder;
use crate::oauth::copilot::auth::constants::{API_VERSION, EDITOR_PLUGIN_VERSION};
use crate::oauth::copilot::auth::token_exchange::TokenExchangeConfig;
use crate::oauth::copilot::auth::{ensure_valid_copilot_token, DeviceFlowPending, PollResult};
use crate::oauth::copilot::config::CopilotConfig;
use crate::oauth::copilot::error::{Error, Result};
use crate::oauth::copilot::models::{ModelsResponse, TokenInfo};
use crate::oauth::copilot::storage::{CopilotTokenStorage, MemoryTokenStorage};

/// Builder for constructing a [`CopilotClient`].
#[derive(Debug)]
pub struct CopilotClientBuilder<S: CopilotTokenStorage = MemoryTokenStorage> {
    config: CopilotConfig,
    storage: S,
    http_client: Option<reqwest::Client>,
}

impl Default for CopilotClientBuilder<MemoryTokenStorage> {
    fn default() -> Self {
        Self {
            config: CopilotConfig::default(),
            storage: MemoryTokenStorage::new(),
            http_client: None,
        }
    }
}

impl CopilotClientBuilder<MemoryTokenStorage> {
    /// Creates a new builder with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<S: CopilotTokenStorage> CopilotClientBuilder<S> {
    /// Sets a custom configuration.
    #[must_use]
    pub fn with_config(mut self, config: CopilotConfig) -> Self {
        self.config = config;
        self
    }

    /// Sets a custom HTTP client.
    #[must_use]
    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Sets the VS Code version to report.
    #[must_use]
    pub fn with_vs_code_version(mut self, version: impl Into<String>) -> Self {
        self.config = self.config.with_vs_code_version(version);
        self
    }

    /// Sets the token storage backend.
    #[must_use]
    pub fn with_storage<T: CopilotTokenStorage>(self, storage: T) -> CopilotClientBuilder<T> {
        CopilotClientBuilder {
            config: self.config,
            storage,
            http_client: self.http_client,
        }
    }

    /// Builds the client.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be constructed.
    pub fn build(self) -> Result<CopilotClient<S>> {
        let http_client = match self.http_client {
            Some(client) => client,
            None => reqwest::Client::builder()
                .timeout(self.config.request_timeout)
                .connect_timeout(self.config.connect_timeout)
                .build()
                .map_err(|e| Error::Config(format!("Failed to build HTTP client: {e}")))?,
        };

        Ok(CopilotClient {
            config: self.config,
            storage: Arc::new(self.storage),
            http_client,
            models_cache: Arc::new(RwLock::new(None)),
        })
    }
}

/// Client for the GitHub Copilot API.
///
/// The client handles authentication, token refresh, and API requests.
///
/// # Example
///
/// ```no_run
/// use crate::oauth::copilot::CopilotClient;
///
/// # async fn example() -> crate::oauth::copilot::Result<()> {
/// let client = CopilotClient::builder().build()?;
///
/// // Start device flow authentication
/// let pending = client.start_device_flow().await?;
/// println!("Visit {} and enter {}", pending.verification_uri, pending.user_code);
///
/// // Poll for completion
/// let token = client.poll_for_token(&pending).await?;
///
/// // Make API requests
/// let response = client
///     .chat()
///     .user("Hello!")
///     .send()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct CopilotClient<S: CopilotTokenStorage = MemoryTokenStorage> {
    config: CopilotConfig,
    storage: Arc<S>,
    http_client: reqwest::Client,
    models_cache: Arc<RwLock<Option<ModelsResponse>>>,
}

impl CopilotClient<MemoryTokenStorage> {
    /// Creates a new builder.
    #[must_use]
    pub fn builder() -> CopilotClientBuilder<MemoryTokenStorage> {
        CopilotClientBuilder::new()
    }
}

impl<S: CopilotTokenStorage> CopilotClient<S> {
    // ─────────────────────────────────────────────────────────────────────────
    // Configuration & Accessors
    // ─────────────────────────────────────────────────────────────────────────

    /// Returns a reference to the client configuration.
    #[must_use]
    pub fn config(&self) -> &CopilotConfig {
        &self.config
    }

    /// Returns a reference to the HTTP client.
    #[must_use]
    pub fn http_client(&self) -> &reqwest::Client {
        &self.http_client
    }

    /// Returns a reference to the storage backend.
    pub fn storage(&self) -> &S {
        &self.storage
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Authentication
    // ─────────────────────────────────────────────────────────────────────────

    /// Starts the device code flow for authentication.
    ///
    /// Returns information about the device code flow, including the
    /// user code to display and the verification URL.
    #[instrument(skip(self))]
    pub async fn start_device_flow(&self) -> Result<DeviceFlowPending> {
        crate::oauth::copilot::auth::device_flow::start_device_flow(&self.http_client).await
    }

    /// Polls for device flow completion.
    ///
    /// Call this repeatedly after starting the device flow to check
    /// if the user has completed authorization.
    #[instrument(skip(self, pending))]
    pub async fn poll_for_token(&self, pending: &DeviceFlowPending) -> Result<PollResult> {
        crate::oauth::copilot::auth::device_flow::poll_for_token(
            &self.http_client,
            &pending.device_code,
        )
        .await
    }

    /// Completes authentication by exchanging the GitHub token for a Copilot token.
    ///
    /// Call this after `poll_for_token` returns `PollResult::Complete`.
    #[instrument(skip(self, github_token))]
    pub async fn complete_auth(&self, github_token: impl Into<String>) -> Result<()> {
        let github_token = github_token.into();
        let mut token_info = TokenInfo::new(&github_token);

        // Exchange for Copilot token
        let exchange_config = TokenExchangeConfig::default()
            .with_token_url(&self.config.copilot_token_url)
            .with_vs_code_version(&self.config.vs_code_version);

        ensure_valid_copilot_token(&self.http_client, &mut token_info, &exchange_config).await?;

        // Save to storage
        self.storage.save(&token_info).await?;

        debug!("Authentication completed successfully");
        Ok(())
    }

    /// Checks if the client is authenticated.
    #[must_use]
    pub async fn is_authenticated(&self) -> bool {
        match self.storage.load().await {
            Ok(Some(token)) => token.has_github_token(),
            _ => false,
        }
    }

    /// Signs out by removing stored tokens.
    #[instrument(skip(self))]
    pub async fn sign_out(&self) -> Result<()> {
        self.storage.remove().await?;
        debug!("Signed out successfully");
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // API Builders
    // ─────────────────────────────────────────────────────────────────────────

    /// Creates a chat completion request builder.
    #[must_use]
    pub fn chat(&self) -> ChatRequestBuilder<'_, S> {
        ChatRequestBuilder::new(self)
    }

    /// Creates an embeddings request builder.
    #[must_use]
    pub fn embeddings(&self) -> EmbeddingsRequestBuilder<'_, S> {
        EmbeddingsRequestBuilder::new(self)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Cache Management
    // ─────────────────────────────────────────────────────────────────────────

    /// Returns cached models if available.
    pub(crate) async fn get_cached_models(&self) -> Option<ModelsResponse> {
        let cache = self.models_cache.read().await;
        cache.clone()
    }

    /// Updates the models cache.
    pub(crate) async fn cache_models(&self, models: ModelsResponse) {
        let mut cache = self.models_cache.write().await;
        *cache = Some(models);
    }

    /// Clears the models cache.
    pub async fn clear_models_cache(&self) {
        let mut cache = self.models_cache.write().await;
        *cache = None;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // HTTP Helpers
    // ─────────────────────────────────────────────────────────────────────────

    /// Makes an authenticated request to the Copilot API.
    pub(crate) async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T> {
        let token = self.ensure_valid_token().await?;

        let url = self.config.endpoint(path);
        let mut request = self.http_client.request(method, &url);
        request = request.headers(self.build_headers(&token)?);

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;
        let status = response.status();

        if !status.is_success() {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if status.as_u16() == 429 {
                return Err(Error::rate_limited(None));
            }

            return Err(Error::api(status.as_u16(), message));
        }

        let result = response.json().await?;
        Ok(result)
    }

    /// Makes an authenticated streaming request.
    pub(crate) async fn request_stream(
        &self,
        method: Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<impl Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>>> {
        let token = self.ensure_valid_token().await?;

        let url = self.config.endpoint(path);
        let mut request = self.http_client.request(method, &url);
        request = request.headers(self.build_headers(&token)?);

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;
        let status = response.status();

        if !status.is_success() {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::api(status.as_u16(), message));
        }

        Ok(response.bytes_stream())
    }

    /// Ensures a valid Copilot token is available.
    async fn ensure_valid_token(&self) -> Result<TokenInfo> {
        let mut token = self
            .storage
            .load()
            .await?
            .ok_or(Error::NotAuthenticated)?;

        if token.needs_copilot_refresh() && self.config.auto_refresh {
            debug!("Token needs refresh, refreshing...");
            let exchange_config = TokenExchangeConfig::default()
                .with_token_url(&self.config.copilot_token_url)
                .with_vs_code_version(&self.config.vs_code_version);

            if let Err(e) =
                ensure_valid_copilot_token(&self.http_client, &mut token, &exchange_config).await
            {
                warn!(error = %e, "Token refresh failed");
                return Err(e);
            }

            // Save updated token
            self.storage.save(&token).await?;
        }

        // Verify we have a Copilot token
        if token.copilot_token.is_none() {
            return Err(Error::NotAuthenticated);
        }

        Ok(token)
    }

    /// Builds headers for API requests.
    fn build_headers(&self, token: &TokenInfo) -> Result<HeaderMap> {
        let copilot_token = token
            .copilot_token
            .as_ref()
            .ok_or(Error::NotAuthenticated)?;

        let mut headers = HeaderMap::new();

        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {copilot_token}"))
                .map_err(|e| Error::Config(format!("Invalid token: {e}")))?,
        );

        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(crate::oauth::copilot::auth::constants::USER_AGENT),
        );

        headers.insert(
            "editor-version",
            HeaderValue::from_str(&format!("vscode/{}", self.config.vs_code_version))
                .map_err(|e| Error::Config(format!("Invalid editor version: {e}")))?,
        );

        headers.insert(
            "editor-plugin-version",
            HeaderValue::from_static(EDITOR_PLUGIN_VERSION),
        );

        headers.insert(
            "copilot-integration-id",
            HeaderValue::from_static("vscode-chat"),
        );

        headers.insert(
            "openai-organization",
            HeaderValue::from_static("github-copilot"),
        );

        headers.insert(
            "openai-intent",
            HeaderValue::from_static("conversation-panel"),
        );

        headers.insert(
            "x-github-api-version",
            HeaderValue::from_static(API_VERSION),
        );

        Ok(headers)
    }
}

// Implement Clone manually since we use Arc
impl<S: CopilotTokenStorage> Clone for CopilotClient<S> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            storage: Arc::clone(&self.storage),
            http_client: self.http_client.clone(),
            models_cache: Arc::clone(&self.models_cache),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder() {
        let client = CopilotClient::builder()
            .with_vs_code_version("vscode/1.80.0")
            .build()
            .expect("should build");

        assert_eq!(client.config().vs_code_version, "vscode/1.80.0");
    }

    #[test]
    fn test_client_builder_with_config() {
        let config = CopilotConfig::default()
            .with_api_base_url("https://custom.api.com")
            .with_auto_refresh(false);

        let client = CopilotClient::builder()
            .with_config(config)
            .build()
            .expect("should build");

        assert_eq!(client.config().api_base_url, "https://custom.api.com");
        assert!(!client.config().auto_refresh);
    }

    #[tokio::test]
    async fn test_client_builder_with_storage() {
        let storage = MemoryTokenStorage::with_token(TokenInfo::new("gho_test"));

        let client = CopilotClient::builder()
            .with_storage(storage)
            .build()
            .expect("should build");

        // Verify we can access the token via public API
        assert!(client.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_client_not_authenticated() {
        let client = CopilotClient::builder().build().expect("should build");

        assert!(!client.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_client_sign_out() {
        let storage = MemoryTokenStorage::with_token(TokenInfo::new("gho_test"));
        let client = CopilotClient::builder()
            .with_storage(storage)
            .build()
            .expect("should build");

        client.sign_out().await.expect("should sign out");
        assert!(!client.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_models_cache() {
        let client = CopilotClient::builder().build().expect("should build");

        // Initially empty
        assert!(client.get_cached_models().await.is_none());

        // Add to cache
        let models = ModelsResponse {
            object: "list".to_string(),
            data: vec![],
        };
        client.cache_models(models).await;

        // Should be cached
        assert!(client.get_cached_models().await.is_some());

        // Clear cache
        client.clear_models_cache().await;
        assert!(client.get_cached_models().await.is_none());
    }

    #[test]
    fn test_client_clone() {
        let client = CopilotClient::builder().build().expect("should build");

        let cloned = client.clone();

        // Should share the same storage
        assert!(Arc::ptr_eq(&client.storage, &cloned.storage));
        assert!(Arc::ptr_eq(&client.models_cache, &cloned.models_cache));
    }
}
