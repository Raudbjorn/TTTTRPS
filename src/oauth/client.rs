//! Generic Gate client.
//!
//! This module provides [`GateClient`], a generic interface for making API calls
//! to LLM providers using the unified OAuth flow and token storage.

use std::marker::PhantomData;
use std::sync::Arc;

use reqwest::{Method, RequestBuilder};
use tokio::sync::RwLock;
use tracing::instrument;

use crate::oauth::auth::{OAuthFlow, OAuthFlowState};
use crate::oauth::error::Result;
use crate::oauth::providers::OAuthProvider;
use crate::oauth::storage::TokenStorage;
use crate::oauth::TokenInfo;

/// Generic Gate client.
///
/// Combines an OAuth flow with a provider implementation to handle
/// authentication and request signing.
///
/// # Type Parameters
///
/// * `P`: The OAuth provider implementation (e.g., [`ClaudeProvider`], [`GeminiProvider`])
/// * `S`: The token storage backend (e.g., [`FileTokenStorage`], [`MemoryTokenStorage`])
///
/// # Example
///
/// ```rust,ignore
/// use crate::oauth::GateClient;
/// use crate::oauth::providers::ClaudeProvider;
/// use crate::oauth::storage::FileTokenStorage;
///
/// let storage = FileTokenStorage::default_path()?;
/// let provider = ClaudeProvider::new();
/// let client = GateClient::new(storage, provider);
///
/// // Start OAuth flow
/// let (url, state) = client.start_authorization().await?;
/// ```
#[derive(Clone)]
pub struct GateClient<P: OAuthProvider, S: TokenStorage> {
    flow: Arc<RwLock<OAuthFlow<S, P>>>,
    http_client: reqwest::Client,
    _storage: PhantomData<S>,
    _provider: PhantomData<P>,
}

impl<P: OAuthProvider + Clone + 'static, S: TokenStorage + 'static> GateClient<P, S> {
    /// Create a new Gate client.
    ///
    /// # Arguments
    ///
    /// * `storage` - Token storage backend
    /// * `provider` - OAuth provider implementation
    pub fn new(storage: S, provider: P) -> Self {
        let flow = OAuthFlow::new(storage, provider);

        Self {
            flow: Arc::new(RwLock::new(flow)),
            http_client: reqwest::Client::new(),
            _storage: PhantomData,
            _provider: PhantomData,
        }
    }

    /// Start the OAuth authorization flow.
    ///
    /// Returns the authorization URL that the user should open in their browser.
    /// The returned state should be preserved and passed to `exchange_code`.
    pub async fn start_authorization(&self) -> Result<(String, OAuthFlowState)> {
        self.flow.read().await.start_authorization_async().await
    }

    /// Complete the OAuth flow by exchanging the authorization code.
    ///
    /// # Arguments
    ///
    /// * `code` - The authorization code from the OAuth callback
    /// * `state` - Optional state parameter for CSRF verification
    pub async fn exchange_code(
        &self,
        code: &str,
        state: Option<&str>,
    ) -> Result<TokenInfo> {
        self.flow.write().await.exchange_code(code, state).await
    }

    /// Get a valid access token, refreshing if necessary.
    pub async fn get_access_token(&self) -> Result<String> {
        self.flow.read().await.get_access_token().await
    }

    /// Check if the client is authenticated.
    pub async fn is_authenticated(&self) -> Result<bool> {
        self.flow.read().await.is_authenticated().await
    }

    /// Log out and remove stored credentials.
    pub async fn logout(&self) -> Result<()> {
        self.flow.write().await.logout().await
    }

    /// Make an authenticated API request.
    ///
    /// Automatically adds the Authorization header with a valid access token.
    ///
    /// # Arguments
    ///
    /// * `method` - HTTP method
    /// * `path` - Full URL for the API request
    #[instrument(skip(self))]
    pub async fn request(&self, method: Method, path: &str) -> Result<RequestBuilder> {
        let access_token = self.get_access_token().await?;
        
        Ok(self.http_client
            .request(method, path)
            .bearer_auth(access_token))
    }
    
    /// Get a clone of the underlying provider.
    pub async fn provider(&self) -> P {
        self.flow.read().await.provider().clone()
    }
}