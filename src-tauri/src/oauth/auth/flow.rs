//! OAuth flow orchestrator.
//!
//! This module provides the [`OAuthFlow`] struct which orchestrates the complete
//! OAuth authentication lifecycle including:
//!
//! - Starting authorization (generating PKCE and authorization URL)
//! - Exchanging authorization codes for tokens
//! - Refreshing access tokens automatically
//! - Token storage and retrieval
//! - Logout functionality
//!
//! # Example
//!
//! ```rust,ignore
//! use gate::auth::{OAuthFlow, OAuthConfig};
//! use gate::storage::MemoryTokenStorage;
//!
//! # async fn example() -> gate::Result<()> {
//! let storage = MemoryTokenStorage::new();
//! let config = OAuthConfig::claude();
//! let flow = OAuthFlow::new(storage, config, "anthropic");
//!
//! // Check if already authenticated
//! if !flow.is_authenticated().await? {
//!     // Start OAuth flow
//!     let (url, state) = flow.start_authorization()?;
//!     println!("Open: {}", url);
//!
//!     // After user authorizes, exchange the code
//!     // let code = "..."; // From callback
//!     // flow.exchange_code(code, Some(&state.state)).await?;
//! }
//!
//! // Get access token (auto-refreshes if needed)
//! let token = flow.get_access_token().await?;
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};

use super::state::OAuthFlowState;
use crate::oauth::error::{AuthError, Error, Result};
use crate::oauth::providers::OAuthProvider;
use crate::oauth::storage::TokenStorage;
use crate::oauth::token::TokenInfo;

/// OAuth flow orchestrator.
///
/// Manages the complete OAuth lifecycle including authorization,
/// token exchange, refresh, and storage. The flow is generic over
/// the storage backend and OAuth provider.
///
/// # Thread Safety
///
/// `OAuthFlow` is `Send + Sync` when the storage backend and provider are `Send + Sync`.
///
/// # Example
///
/// ```rust,ignore
/// use gate::auth::OAuthFlow;
/// use gate::providers::ClaudeProvider;
/// use gate::storage::FileTokenStorage;
///
/// # async fn example() -> gate::Result<()> {
/// let storage = FileTokenStorage::default_path()?;
/// let provider = ClaudeProvider::new();
/// let flow = OAuthFlow::new(storage, provider);
///
/// // Use from multiple tasks
/// let flow = std::sync::Arc::new(flow);
/// let flow_clone = flow.clone();
///
/// tokio::spawn(async move {
///     let token = flow_clone.get_access_token().await;
/// });
/// # Ok(())
/// # }
/// ```
pub struct OAuthFlow<S: TokenStorage, P: OAuthProvider> {
    /// Token storage backend.
    storage: S,
    /// OAuth provider implementation.
    provider: P,
    /// Pending OAuth flow state (PKCE verifier, challenge, state).
    ///
    /// This is set when `start_authorization()` is called and cleared
    /// after `exchange_code()` completes.
    pending_state: Arc<RwLock<Option<OAuthFlowState>>>,
}

impl<S: TokenStorage, P: OAuthProvider> OAuthFlow<S, P> {
    /// Create a new OAuthFlow with the specified storage and provider.
    ///
    /// # Arguments
    ///
    /// * `storage` - Token storage backend for persisting credentials
    /// * `provider` - OAuth provider implementation
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use gate::auth::OAuthFlow;
    /// use gate::providers::ClaudeProvider;
    /// use gate::storage::MemoryTokenStorage;
    ///
    /// let storage = MemoryTokenStorage::new();
    /// let provider = ClaudeProvider::new();
    /// let flow = OAuthFlow::new(storage, provider);
    /// ```
    pub fn new(storage: S, provider: P) -> Self {
        Self {
            storage,
            provider,
            pending_state: Arc::new(RwLock::new(None)),
        }
    }

    /// Get a reference to the provider.
    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// Get a reference to the storage backend.
    pub fn storage(&self) -> &S {
        &self.storage
    }

    /// Start a new authorization flow.
    ///
    /// Generates PKCE verifier/challenge and state, then returns the
    /// authorization URL for the user to visit along with the flow state.
    ///
    /// The flow state should be stored temporarily and the state parameter
    /// should be validated when the callback is received.
    ///
    /// # Returns
    ///
    /// A tuple of `(authorization_url, flow_state)` where:
    /// - `authorization_url`: URL for user to visit to authorize
    /// - `flow_state`: Contains verifier and state for later validation
    #[instrument(skip(self))]
    pub fn start_authorization(&self) -> Result<(String, OAuthFlowState)> {
        let flow_state = OAuthFlowState::new();

        let url = self.provider.build_auth_url(
            &crate::oauth::auth::Pkce {
                verifier: flow_state.code_verifier.clone(),
                challenge: flow_state.code_challenge.clone(),
                method: "S256",
            },
            &flow_state.state,
        );

        debug!(state = %flow_state.state, "Started OAuth authorization flow");

        // Store the pending state
        // Note: We use try_write to avoid blocking. If lock is held, spawn a task.
        let pending_clone = flow_state.clone();

        // Try non-blocking write first
        if let Ok(mut pending) = self.pending_state.try_write() {
            *pending = Some(pending_clone);
        } else {
            warn!("Failed to acquire lock for auth state in sync method");
            return Err(Error::Config("Failed to acquire lock for auth state - use start_authorization_async".into()));
        }

        Ok((url, flow_state))
    }

    /// Start authorization without blocking.
    ///
    /// Async version of `start_authorization()` that doesn't use try_write.
    /// Preferred when calling from an async context.
    #[instrument(skip(self))]
    pub async fn start_authorization_async(&self) -> Result<(String, OAuthFlowState)> {
        let flow_state = OAuthFlowState::new();

        let url = self.provider.build_auth_url(
            &crate::oauth::auth::Pkce {
                verifier: flow_state.code_verifier.clone(),
                challenge: flow_state.code_challenge.clone(),
                method: "S256",
            },
            &flow_state.state,
        );

        debug!(state = %flow_state.state, "Started OAuth authorization flow");

        // Store the pending state
        {
            let mut pending = self.pending_state.write().await;
            *pending = Some(flow_state.clone());
        }

        Ok((url, flow_state))
    }

    /// Exchange an authorization code for tokens.
    ///
    /// Completes the OAuth flow by exchanging the authorization code
    /// for access and refresh tokens. Optionally validates the state
    /// parameter to protect against CSRF attacks.
    ///
    /// # Arguments
    ///
    /// * `code` - Authorization code from the OAuth callback
    /// * `state` - Optional state parameter to validate (recommended)
    #[instrument(skip(self, code, state))]
    pub async fn exchange_code(&self, code: &str, state: Option<&str>) -> Result<TokenInfo> {
        // Get and clear pending state
        let pending_state = {
            let mut pending = self.pending_state.write().await;
            pending.take()
        };

        // Validate state if provided
        if let Some(expected_state) = state {
            match &pending_state {
                Some(flow_state) if flow_state.state != expected_state => {
                    warn!(
                        expected = %flow_state.state,
                        received = %expected_state,
                        "OAuth state mismatch"
                    );
                    return Err(Error::Auth(AuthError::StateMismatch));
                }
                None => {
                    warn!("OAuth state provided but no pending flow state found");
                    return Err(Error::Auth(AuthError::NotAuthenticated));
                }
                _ => {
                    debug!("OAuth state validated successfully");
                }
            }
        }

        // Get the verifier from pending state
        let verifier = match &pending_state {
            Some(flow_state) => flow_state.code_verifier.clone(),
            None => {
                // If no pending state, we can't proceed without a verifier
                warn!("No pending flow state, cannot exchange code");
                return Err(Error::Auth(AuthError::StateMismatch));
            }
        };

        // Exchange the code for tokens via provider
        let token = self.provider.exchange_code(code, &verifier).await?;

        // Save the token
        self.storage.save(self.provider.provider_id(), &token).await?;

        info!("OAuth flow completed successfully");

        Ok(token)
    }

    /// Exchange code using an externally-provided verifier.
    ///
    /// Use this when you've stored the verifier externally rather than
    /// relying on the pending flow state.
    ///
    /// # Arguments
    ///
    /// * `code` - Authorization code from callback
    /// * `verifier` - PKCE code verifier
    /// * `expected_state` - Optional state to validate against
    /// * `received_state` - State received in callback
    #[instrument(skip(self, code, verifier))]
    pub async fn exchange_code_with_verifier(
        &self,
        code: &str,
        verifier: &str,
        expected_state: Option<&str>,
        received_state: Option<&str>,
    ) -> Result<TokenInfo> {
        // Validate state if both are provided
        if let (Some(expected), Some(received)) = (expected_state, received_state) {
            if expected != received {
                warn!(
                    expected = %expected,
                    received = %received,
                    "OAuth state mismatch"
                );
                return Err(Error::Auth(AuthError::StateMismatch));
            }
            debug!("OAuth state validated successfully");
        }

        // Exchange the code for tokens via provider
        let token = self.provider.exchange_code(code, verifier).await?;

        // Save the token
        self.storage.save(self.provider.provider_id(), &token).await?;

        info!("OAuth flow completed successfully");

        Ok(token)
    }

    /// Get a valid access token, refreshing if necessary.
    ///
    /// If the stored access token is expired or about to expire (within 5 minutes),
    /// automatically refreshes it using the refresh token.
    #[instrument(skip(self))]
    pub async fn get_access_token(&self) -> Result<String> {
        let token = self
            .storage
            .load(self.provider.provider_id())
            .await?
            .ok_or(Error::Auth(AuthError::NotAuthenticated))?;

        // Check if token needs refresh (expired or within 5-minute window)
        if token.needs_refresh() {
            debug!("Access token expired or expiring soon, refreshing");
            let new_token = self.provider.refresh_token(&token.refresh_token).await?;
            self.storage.save(self.provider.provider_id(), &new_token).await?;
            return Ok(new_token.access_token);
        }

        Ok(token.access_token)
    }

    /// Get the full TokenInfo, refreshing if necessary.
    #[instrument(skip(self))]
    pub async fn get_token(&self) -> Result<TokenInfo> {
        let token = self
            .storage
            .load(self.provider.provider_id())
            .await?
            .ok_or(Error::Auth(AuthError::NotAuthenticated))?;

        // Check if token needs refresh
        if token.needs_refresh() {
            debug!("Access token expired or expiring soon, refreshing");
            let new_token = self.provider.refresh_token(&token.refresh_token).await?;
            self.storage.save(self.provider.provider_id(), &new_token).await?;
            return Ok(new_token);
        }

        Ok(token)
    }

    /// Check if the user is currently authenticated.
    #[instrument(skip(self))]
    pub async fn is_authenticated(&self) -> Result<bool> {
        self.storage.exists(self.provider.provider_id()).await
    }

    /// Log out by removing stored tokens.
    #[instrument(skip(self))]
    pub async fn logout(&self) -> Result<()> {
        // Clear pending state
        {
            let mut pending = self.pending_state.write().await;
            *pending = None;
        }

        // Remove stored token
        self.storage.remove(self.provider.provider_id()).await?;

        info!("Logged out successfully");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oauth::storage::MemoryTokenStorage;
    use crate::oauth::providers::ClaudeProvider;

    #[tokio::test]
    async fn test_new_flow_not_authenticated() {
        let storage = MemoryTokenStorage::new();
        let provider = ClaudeProvider::new();
        let flow = OAuthFlow::new(storage, provider);

        assert!(!flow.is_authenticated().await.unwrap());
    }

    #[tokio::test]
    async fn test_start_authorization_returns_url_and_state() {
        let storage = MemoryTokenStorage::new();
        let provider = ClaudeProvider::new();
        let flow = OAuthFlow::new(storage, provider);

        let (url, state) = flow.start_authorization_async().await.unwrap();

        assert!(url.contains("claude.ai") || url.contains("oauth"));
        assert!(!state.state.is_empty());
        assert!(!state.code_verifier.is_empty());
        assert!(!state.code_challenge.is_empty());
    }

    #[tokio::test]
    async fn test_start_authorization_stores_pending_state() {
        let storage = MemoryTokenStorage::new();
        let provider = ClaudeProvider::new();
        let flow = OAuthFlow::new(storage, provider);

        let (_, state) = flow.start_authorization_async().await.unwrap();

        // Verify pending state is stored
        let pending = flow.pending_state.read().await;
        assert!(pending.is_some());
        assert_eq!(pending.as_ref().unwrap().state, state.state);
    }

    #[tokio::test]
    async fn test_exchange_code_validates_state_mismatch() {
        let storage = MemoryTokenStorage::new();
        let provider = ClaudeProvider::new();
        let flow = OAuthFlow::new(storage, provider);

        // Start flow to set pending state
        let (_, _state) = flow.start_authorization_async().await.unwrap();

        // Try to exchange with wrong state
        let result = flow.exchange_code("code", Some("wrong_state")).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Auth(AuthError::StateMismatch) => {}
            e => panic!("Expected StateMismatch, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_provider_accessor() {
        let storage = MemoryTokenStorage::new();
        let provider = ClaudeProvider::new();
        let flow = OAuthFlow::new(storage, provider);

        // Note: "claude" is used for OAuth-based auth; "anthropic" is for API key auth
        assert_eq!(flow.provider().provider_id(), "claude");
    }

    #[tokio::test]
    async fn test_storage_accessor() {
        let storage = MemoryTokenStorage::new();
        let provider = ClaudeProvider::new();
        let flow = OAuthFlow::new(storage, provider);

        assert_eq!(flow.storage().name(), "memory");
    }
}