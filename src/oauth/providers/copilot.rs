//! GitHub Copilot OAuth provider using Device Code flow (RFC 8628).
//!
//! This module implements authentication for GitHub Copilot using the OAuth 2.0
//! Device Authorization Grant. Unlike PKCE-based flows (Claude, Gemini), Device
//! Code flow works without a browser redirect:
//!
//! 1. Application requests a device code from GitHub
//! 2. User visits a verification URL and enters the displayed code
//! 3. Application polls GitHub until the user completes authorization
//! 4. GitHub access token is exchanged for a Copilot API token
//!
//! ## Flow Overview
//!
//! ```text
//! +----------+                                +----------------+
//! |          |---(A)--- Device Code Request -->|                |
//! |          |<--(B)--- Device Code + URI -----|                |
//! |          |                                 |                |
//! |  Client  |         User enters code at     |   GitHub       |
//! |          |         verification_uri        |   Auth Server  |
//! |          |                                 |                |
//! |          |---(C)--- Poll Token Endpoint -->|                |
//! |          |<--(D)--- Access Token ----------|                |
//! +----------+                                +----------------+
//!           |
//!           |---(E)--- Exchange for Copilot Token --->
//!           |<--(F)--- Copilot Token -----------------
//! ```
//!
//! ## Why Not Use OAuthProvider Trait?
//!
//! The existing `OAuthProvider` trait is designed for PKCE Authorization Code
//! flow, which expects:
//! - `build_auth_url(pkce, state)` - Device Code doesn't use PKCE or redirect URLs
//! - `exchange_code(code, verifier)` - Device Code uses device_code, not auth code
//! - `refresh_token(token)` - Copilot uses GitHub token exchange, not OAuth refresh
//!
//! This provider implements a parallel interface suited to Device Code flow.
//!
//! ## Example
//!
//! ```no_run
//! use crate::oauth::providers::CopilotProvider;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let provider = CopilotProvider::new();
//!
//! // Start device flow
//! let pending = provider.initiate_device_flow().await?;
//! println!("Visit: {}", pending.verification_uri);
//! println!("Enter code: {}", pending.user_code);
//!
//! // Poll until authorized (with callback for progress)
//! let github_token = provider.poll_until_complete(&pending, None).await?;
//!
//! // Exchange for Copilot token
//! let copilot_token = provider.exchange_for_copilot_token(&github_token).await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use tracing::{debug, info};

use super::OAuthProvider;
use crate::oauth::auth::{OAuthConfig, Pkce};
use crate::oauth::copilot::auth::constants::{
    GITHUB_ACCESS_TOKEN_URL, GITHUB_CLIENT_ID, GITHUB_DEVICE_CODE_URL, GITHUB_OAUTH_SCOPE,
};
use crate::oauth::copilot::auth::device_flow::{
    poll_for_token, poll_until_complete, start_device_flow, DeviceFlowPending, PollResult,
};
use crate::oauth::copilot::auth::token_exchange::{
    exchange_for_copilot_token, TokenExchangeConfig,
};
use crate::oauth::copilot::error::{Error as CopilotError, Result as CopilotResult};
use crate::oauth::copilot::models::auth::CopilotTokenResponse;
use crate::oauth::error::{AuthError, Error, Result};
use crate::oauth::token::TokenInfo;

// =============================================================================
// Constants
// =============================================================================

/// Provider identifier for GitHub Copilot.
pub const PROVIDER_ID: &str = "copilot";

/// Human-readable provider name.
pub const PROVIDER_NAME: &str = "GitHub Copilot";

/// HTTP request timeout in seconds.
const HTTP_TIMEOUT_SECS: u64 = 30;

// =============================================================================
// Provider Implementation
// =============================================================================

/// GitHub Copilot OAuth provider using Device Code flow.
///
/// This provider handles the complete authentication flow for GitHub Copilot:
/// 1. Device code initiation with GitHub OAuth
/// 2. Polling for user authorization
/// 3. Token exchange from GitHub to Copilot
///
/// # Example
///
/// ```rust,ignore
/// use gate::providers::CopilotProvider;
///
/// let provider = CopilotProvider::new();
///
/// // Check provider info
/// assert_eq!(provider.provider_id(), "copilot");
/// assert_eq!(provider.name(), "GitHub Copilot");
/// ```
#[derive(Clone)]
pub struct CopilotProvider {
    /// OAuth configuration (adapted for Device Code flow).
    config: OAuthConfig,
    /// HTTP client for API requests.
    http_client: reqwest::Client,
    /// Configuration for GitHub -> Copilot token exchange.
    exchange_config: TokenExchangeConfig,
}

impl CopilotProvider {
    /// Create a new CopilotProvider with default configuration.
    ///
    /// Uses the standard GitHub Device Code OAuth configuration for Copilot.
    #[must_use]
    pub fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
            .build()
            .unwrap_or_default();

        Self {
            config: Self::build_oauth_config(),
            http_client,
            exchange_config: TokenExchangeConfig::default(),
        }
    }

    /// Create a CopilotProvider with a custom HTTP client.
    ///
    /// Useful for configuring timeouts, proxies, or custom TLS settings.
    ///
    /// # Arguments
    ///
    /// * `http_client` - Pre-configured reqwest client
    #[must_use]
    pub fn with_http_client(http_client: reqwest::Client) -> Self {
        Self {
            config: Self::build_oauth_config(),
            http_client,
            exchange_config: TokenExchangeConfig::default(),
        }
    }

    /// Create a CopilotProvider with custom exchange configuration.
    ///
    /// # Arguments
    ///
    /// * `exchange_config` - Custom Copilot token exchange configuration
    #[must_use]
    pub fn with_exchange_config(mut self, exchange_config: TokenExchangeConfig) -> Self {
        self.exchange_config = exchange_config;
        self
    }

    /// Build the OAuth configuration for GitHub Device Code flow.
    ///
    /// Note: For Device Code flow, auth_url and redirect_uri are not used
    /// in the traditional sense. We store the device code endpoint in auth_url
    /// and the token polling endpoint in token_url.
    fn build_oauth_config() -> OAuthConfig {
        OAuthConfig {
            client_id: GITHUB_CLIENT_ID.to_string(),
            client_secret: None, // Device Code flow doesn't use client secret
            auth_url: GITHUB_DEVICE_CODE_URL.to_string(),
            token_url: GITHUB_ACCESS_TOKEN_URL.to_string(),
            redirect_uri: String::new(), // Not used in Device Code flow
            scopes: vec![GITHUB_OAUTH_SCOPE.to_string()],
            callback_port: None, // No local callback server needed
        }
    }

    /// Get reference to the HTTP client.
    #[must_use]
    pub fn http_client(&self) -> &reqwest::Client {
        &self.http_client
    }

    // =========================================================================
    // Device Code Flow Methods
    // =========================================================================

    /// Initiate the GitHub Device Code flow.
    ///
    /// This starts the authentication process by requesting a device code
    /// and user code from GitHub. The user must visit the verification URL
    /// and enter the user code to authorize the application.
    ///
    /// # Returns
    ///
    /// A `DeviceFlowPending` containing:
    /// - `device_code` - Internal code for polling
    /// - `user_code` - Code for user to enter at verification URL
    /// - `verification_uri` - URL where user enters the code
    /// - `expires_in` - Seconds until the device code expires
    /// - `interval` - Minimum seconds between poll attempts
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Network error occurs
    /// - GitHub returns an error response
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crate::oauth::providers::CopilotProvider;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = CopilotProvider::new();
    /// let pending = provider.initiate_device_flow().await?;
    ///
    /// println!("Go to: {}", pending.verification_uri);
    /// println!("Enter: {}", pending.user_code);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn initiate_device_flow(&self) -> CopilotResult<DeviceFlowPending> {
        info!("Initiating GitHub device code flow for Copilot");
        start_device_flow(&self.http_client).await
    }

    /// Poll for device flow completion (single poll attempt).
    ///
    /// This performs a single poll to check if the user has completed
    /// authorization. Call this in a loop with appropriate delays.
    ///
    /// # Arguments
    ///
    /// * `pending` - The pending device flow state
    ///
    /// # Returns
    ///
    /// - `PollResult::Pending` - User hasn't completed yet, keep polling
    /// - `PollResult::SlowDown` - Increase poll interval
    /// - `PollResult::Complete(token)` - Got the GitHub access token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User denied authorization
    /// - Device code expired
    /// - Network error occurs
    pub async fn poll_device_flow(&self, pending: &DeviceFlowPending) -> CopilotResult<PollResult> {
        poll_for_token(&self.http_client, &pending.device_code).await
    }

    /// Poll until the device flow completes (full polling loop).
    ///
    /// This handles the complete polling loop, respecting the server-specified
    /// interval and backing off on `slow_down` responses.
    ///
    /// # Arguments
    ///
    /// * `pending` - The pending device flow state
    /// * `on_pending` - Optional callback called on each pending poll (receives attempt count)
    ///
    /// # Returns
    ///
    /// The GitHub access token on success.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User denies authorization
    /// - Device code expires
    /// - Maximum poll attempts exceeded
    /// - Network error occurs
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crate::oauth::providers::CopilotProvider;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = CopilotProvider::new();
    /// let pending = provider.initiate_device_flow().await?;
    ///
    /// // Poll with progress callback
    /// let token = provider.poll_until_complete(&pending, Some(&mut |attempt| {
    ///     println!("Polling attempt {}...", attempt);
    /// })).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn poll_until_complete(
        &self,
        pending: &DeviceFlowPending,
        on_pending: Option<&mut dyn FnMut(u32)>,
    ) -> CopilotResult<String> {
        poll_until_complete(&self.http_client, pending, on_pending).await
    }

    // =========================================================================
    // Token Exchange Methods
    // =========================================================================

    /// Exchange a GitHub access token for a Copilot API token.
    ///
    /// The Copilot token is required to access the Copilot API. It has a
    /// shorter lifetime than the GitHub token (typically ~30 minutes).
    ///
    /// # Arguments
    ///
    /// * `github_token` - The GitHub OAuth access token
    ///
    /// # Returns
    ///
    /// A `CopilotTokenResponse` containing the Copilot token and metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The GitHub token is invalid or expired
    /// - The GitHub account doesn't have Copilot access
    /// - Network error occurs
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crate::oauth::providers::CopilotProvider;
    ///
    /// # async fn example(github_token: &str) -> Result<(), Box<dyn std::error::Error>> {
    /// let provider = CopilotProvider::new();
    /// let copilot = provider.exchange_for_copilot_token(github_token).await?;
    /// println!("Token expires at: {}", copilot.expires_at);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn exchange_for_copilot_token(
        &self,
        github_token: &str,
    ) -> CopilotResult<CopilotTokenResponse> {
        exchange_for_copilot_token(&self.http_client, github_token, &self.exchange_config).await
    }

    /// Perform the complete authentication flow and return unified TokenInfo.
    ///
    /// This is a convenience method that:
    /// 1. Starts the device code flow
    /// 2. Returns the pending state for UI display
    ///
    /// The caller should:
    /// 1. Display the verification URL and user code to the user
    /// 2. Call `poll_until_complete()` to wait for authorization
    /// 3. Call `exchange_for_copilot_token()` to get the Copilot token
    /// 4. Use `create_token_info()` to create unified token storage
    ///
    /// # Returns
    ///
    /// The pending device flow state for display to the user.
    pub async fn start_auth_flow(&self) -> CopilotResult<DeviceFlowPending> {
        self.initiate_device_flow().await
    }

    /// Create a unified TokenInfo from GitHub and Copilot tokens.
    ///
    /// This creates a TokenInfo suitable for storage that contains both
    /// the long-lived GitHub token and the short-lived Copilot token.
    ///
    /// # Arguments
    ///
    /// * `github_token` - The GitHub OAuth access token
    /// * `copilot_response` - The Copilot token exchange response
    ///
    /// # Returns
    ///
    /// A TokenInfo with the provider set to "copilot".
    #[must_use]
    pub fn create_token_info(
        github_token: &str,
        copilot_response: &CopilotTokenResponse,
    ) -> TokenInfo {
        // Calculate expires_in from the absolute timestamp
        let now = chrono::Utc::now().timestamp();
        let expires_in = (copilot_response.expires_at - now).max(0);

        TokenInfo::new(
            copilot_response.token.clone(),
            github_token.to_string(), // Store GitHub token in refresh_token field
            expires_in,
        )
        .with_provider(PROVIDER_ID)
    }
}

impl Default for CopilotProvider {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// OAuthProvider Trait Implementation
// =============================================================================

/// OAuthProvider implementation for compatibility.
///
/// **Important**: GitHub Copilot uses Device Code flow, not PKCE Authorization
/// Code flow. The `build_auth_url`, `exchange_code`, and `refresh_token` methods
/// have limited functionality. Use the Device Code specific methods instead:
///
/// - `initiate_device_flow()` instead of `build_auth_url()`
/// - `poll_until_complete()` for polling
/// - `exchange_for_copilot_token()` for token exchange
#[async_trait]
impl OAuthProvider for CopilotProvider {
    fn provider_id(&self) -> &str {
        PROVIDER_ID
    }

    fn name(&self) -> &str {
        PROVIDER_NAME
    }

    fn oauth_config(&self) -> &OAuthConfig {
        &self.config
    }

    /// Build auth URL - for Device Code flow, returns the verification URI.
    ///
    /// **Note**: This method is not typically used for Device Code flow.
    /// Use `initiate_device_flow()` instead, which returns the verification
    /// URI along with the user code.
    fn build_auth_url(&self, _pkce: &Pkce, _state: &str) -> String {
        // For Device Code flow, return the verification URL base.
        // The actual verification URL with user code comes from start_device_flow().
        "https://github.com/login/device".to_string()
    }

    /// Exchange code for tokens - not used in Device Code flow.
    ///
    /// **Note**: Device Code flow uses `poll_until_complete()` and
    /// `exchange_for_copilot_token()` instead of this method.
    ///
    /// This implementation returns an error indicating to use the Device Code
    /// specific methods.
    async fn exchange_code(&self, _code: &str, _verifier: &str) -> Result<TokenInfo> {
        Err(Error::config(
            "CopilotProvider uses Device Code flow. Use initiate_device_flow() and \
             poll_until_complete() instead of exchange_code().",
        ))
    }

    /// Refresh token - re-exchanges GitHub token for new Copilot token.
    ///
    /// For Copilot, "refreshing" means exchanging the long-lived GitHub token
    /// for a new short-lived Copilot token. The GitHub token itself doesn't
    /// expire in the traditional OAuth sense.
    ///
    /// # Arguments
    ///
    /// * `github_token` - The GitHub OAuth access token (stored in refresh_token field)
    ///
    /// # Returns
    ///
    /// New TokenInfo with a fresh Copilot token.
    async fn refresh_token(&self, github_token: &str) -> Result<TokenInfo> {
        debug!("Refreshing Copilot token via GitHub token exchange");

        let copilot_response = self
            .exchange_for_copilot_token(github_token)
            .await
            .map_err(|e| match e {
                CopilotError::NotAuthenticated => Error::Auth(AuthError::NotAuthenticated),
                CopilotError::Api { status: 401, .. } => Error::Auth(AuthError::InvalidGrant),
                CopilotError::Api { status: 403, .. } => {
                    Error::Auth(AuthError::RefreshFailed("No Copilot access".to_string()))
                }
                CopilotError::Http(e) => Error::Network(e),
                other => Error::config(other.to_string()),
            })?;

        info!("Copilot token refreshed successfully");
        Ok(Self::create_token_info(github_token, &copilot_response))
    }

    /// Copilot doesn't require a client secret.
    fn requires_client_secret(&self) -> bool {
        false
    }

    /// No callback port - Device Code flow doesn't use redirects.
    fn callback_port(&self) -> Option<u16> {
        None
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_id() {
        let provider = CopilotProvider::new();
        assert_eq!(provider.provider_id(), "copilot");
    }

    #[test]
    fn test_provider_name() {
        let provider = CopilotProvider::new();
        assert_eq!(provider.name(), "GitHub Copilot");
    }

    #[test]
    fn test_oauth_config() {
        let provider = CopilotProvider::new();
        let config = provider.oauth_config();

        assert_eq!(config.client_id, GITHUB_CLIENT_ID);
        assert!(config.client_secret.is_none());
        assert!(config.auth_url.contains("github.com"));
        assert!(config.token_url.contains("github.com"));
        assert!(config.scopes.contains(&GITHUB_OAUTH_SCOPE.to_string()));
    }

    #[test]
    fn test_does_not_require_client_secret() {
        let provider = CopilotProvider::new();
        assert!(!provider.requires_client_secret());
    }

    #[test]
    fn test_no_callback_port() {
        let provider = CopilotProvider::new();
        assert!(provider.callback_port().is_none());
    }

    #[test]
    fn test_build_auth_url_returns_verification_url() {
        let provider = CopilotProvider::new();
        let pkce = Pkce::generate();
        let url = provider.build_auth_url(&pkce, "state");

        // Should return the GitHub device verification URL
        assert!(url.contains("github.com/login/device"));
    }

    #[test]
    fn test_default_trait() {
        let provider = CopilotProvider::default();
        assert_eq!(provider.provider_id(), "copilot");
    }

    #[test]
    fn test_with_http_client() {
        let client = reqwest::Client::new();
        let provider = CopilotProvider::with_http_client(client);
        assert_eq!(provider.provider_id(), "copilot");
    }

    #[test]
    fn test_with_exchange_config() {
        let config = TokenExchangeConfig::default().with_vs_code_version("vscode/1.80.0");
        let provider = CopilotProvider::new().with_exchange_config(config);
        assert_eq!(provider.exchange_config.vs_code_version, "vscode/1.80.0");
    }

    #[test]
    fn test_create_token_info() {
        let copilot_response = CopilotTokenResponse {
            token: "copilot_token_here".to_string(),
            expires_at: chrono::Utc::now().timestamp() + 1800, // 30 minutes
            refresh_in: 1500,
            annotations_enabled: false,
            chat_enabled: true,
            organization_id: None,
            enterprise_id: None,
            sku: Some("pro".to_string()),
            telemetry: None,
            tracking_id: None,
        };

        let token_info = CopilotProvider::create_token_info("gho_github_token", &copilot_response);

        assert_eq!(token_info.access_token, "copilot_token_here");
        assert_eq!(token_info.refresh_token, "gho_github_token");
        assert_eq!(token_info.provider.as_deref(), Some("copilot"));
        assert!(!token_info.is_expired());
    }

    #[tokio::test]
    async fn test_exchange_code_returns_error() {
        let provider = CopilotProvider::new();
        let result = provider.exchange_code("code", "verifier").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Config(_)));
    }
}
