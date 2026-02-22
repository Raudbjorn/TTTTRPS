//! Claude (Anthropic) OAuth provider implementation.
//!
//! This module implements the [`OAuthProvider`] trait for Anthropic's OAuth 2.0 flow.
//!
//! # Key Characteristics
//!
//! - **Token Request Format**: JSON-encoded (not form-encoded)
//! - **Client Secret**: Not required (PKCE-only authentication)
//! - **Auth URL Parameter**: Requires `code=true` parameter
//! - **Redirect**: Uses Anthropic's hosted callback URL
//!
//! # OAuth Endpoints
//!
//! | Endpoint | URL |
//! |----------|-----|
//! | Authorization | `https://claude.ai/oauth/authorize` |
//! | Token | `https://console.anthropic.com/v1/oauth/token` |
//! | Redirect | `https://console.anthropic.com/oauth/code/callback` |
//!
//! # Scopes
//!
//! - `org:create_api_key` - Create API keys for the organization
//! - `user:profile` - Access user profile information
//! - `user:inference` - Make inference requests
//!
//! # Example
//!
//! ```rust,ignore
//! use gate::providers::{OAuthProvider, ClaudeProvider};
//! use gate::auth::Pkce;
//!
//! let provider = ClaudeProvider::new();
//!
//! // Build authorization URL
//! let pkce = Pkce::generate();
//! let state = "random_state";
//! let url = provider.build_auth_url(&pkce, state);
//!
//! // Claude's URL includes the special `code=true` parameter
//! assert!(url.contains("code=true"));
//!
//! // Exchange code for tokens (uses JSON body)
//! let token = provider.exchange_code("auth_code", &pkce.verifier).await?;
//! ```

use async_trait::async_trait;
use tracing::{debug, warn};

use super::{parse_composite_token, OAuthProvider, TokenErrorResponse, TokenResponse};
use crate::oauth::auth::{OAuthConfig, Pkce};
use crate::oauth::error::{AuthError, Error, Result};
use crate::oauth::token::TokenInfo;

/// Provider identifier for Claude OAuth.
/// Note: "claude" is for OAuth-based auth; "anthropic" is for API key auth.
pub const PROVIDER_ID: &str = "claude";

/// Human-readable provider name.
pub const PROVIDER_NAME: &str = "Claude (Anthropic)";

/// Claude OAuth provider.
///
/// Implements OAuth 2.0 with PKCE for Anthropic's API. Claude uses JSON-encoded
/// token requests and does not require a client secret.
///
/// # Important: JSON Token Requests
///
/// Unlike most OAuth providers that use form-encoded requests, Anthropic's
/// token endpoint expects JSON-encoded request bodies. This provider handles
/// this automatically.
///
/// # Example
///
/// ```rust,ignore
/// use gate::providers::ClaudeProvider;
///
/// // Create with default configuration
/// let provider = ClaudeProvider::new();
///
/// // Or create with custom HTTP client
/// let client = reqwest::Client::builder()
///     .timeout(std::time::Duration::from_secs(30))
///     .build()?;
/// let provider = ClaudeProvider::with_http_client(client);
/// ```
#[derive(Clone)]
pub struct ClaudeProvider {
    config: OAuthConfig,
    http_client: reqwest::Client,
}

impl ClaudeProvider {
    /// Create a new ClaudeProvider with default configuration.
    ///
    /// Uses the standard Claude OAuth configuration from [`OAuthConfig::claude()`].
    #[must_use]
    pub fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            config: OAuthConfig::claude(),
            http_client,
        }
    }

    /// Create a ClaudeProvider with a custom HTTP client.
    ///
    /// Useful for configuring timeouts, proxies, or custom TLS settings.
    ///
    /// # Arguments
    ///
    /// * `http_client` - Pre-configured reqwest client
    #[must_use]
    pub fn with_http_client(http_client: reqwest::Client) -> Self {
        Self {
            config: OAuthConfig::claude(),
            http_client,
        }
    }

    /// Create a ClaudeProvider with custom configuration and HTTP client.
    ///
    /// # Arguments
    ///
    /// * `config` - Custom OAuth configuration
    /// * `http_client` - Pre-configured reqwest client
    #[must_use]
    pub fn with_config(config: OAuthConfig, http_client: reqwest::Client) -> Self {
        Self { config, http_client }
    }
}

impl Default for ClaudeProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OAuthProvider for ClaudeProvider {
    fn provider_id(&self) -> &str {
        PROVIDER_ID
    }

    fn name(&self) -> &str {
        PROVIDER_NAME
    }

    fn oauth_config(&self) -> &OAuthConfig {
        &self.config
    }

    /// Build Claude's authorization URL.
    ///
    /// Claude requires a special `code=true` parameter in addition to the
    /// standard OAuth parameters. This tells Anthropic's server to return
    /// the authorization code in a format suitable for copy-paste.
    fn build_auth_url(&self, pkce: &Pkce, state: &str) -> String {
        let scopes = self.config.scopes.join(" ");

        format!(
            "{}?code=true&response_type=code&client_id={}&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
            self.config.auth_url,
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(&self.config.redirect_uri),
            urlencoding::encode(&scopes),
            urlencoding::encode(&pkce.challenge),
            urlencoding::encode(state),
        )
    }

    /// Exchange authorization code for tokens using JSON body.
    ///
    /// Claude's token endpoint expects JSON-encoded requests, not form-encoded.
    /// This is different from most OAuth providers.
    async fn exchange_code(&self, code: &str, verifier: &str) -> Result<TokenInfo> {
        debug!("Exchanging authorization code for Claude tokens");

        // Claude uses JSON body for token exchange (NOT form-encoded)
        let request_body = serde_json::json!({
            "grant_type": "authorization_code",
            "client_id": self.config.client_id,
            "code": code,
            "code_verifier": verifier,
            "redirect_uri": self.config.redirect_uri,
        });

        let response = self
            .http_client
            .post(&self.config.token_url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            // Try to parse error response
            if let Ok(error) = serde_json::from_str::<TokenErrorResponse>(&body) {
                warn!(
                    error = %error.error,
                    description = ?error.error_description,
                    "Claude token exchange failed"
                );

                if error.error == "invalid_grant" {
                    return Err(Error::Auth(AuthError::InvalidGrant));
                }

                return Err(Error::api(
                    status.as_u16(),
                    error
                        .error_description
                        .unwrap_or_else(|| error.error.clone()),
                    None,
                ));
            }

            return Err(Error::api(status.as_u16(), body, None));
        }

        let token_response: TokenResponse = serde_json::from_str(&body)?;

        // Refresh token is required for initial exchange
        let refresh_token = token_response.refresh_token.ok_or_else(|| {
            Error::Auth(AuthError::RefreshFailed(
                "No refresh token in response".to_string(),
            ))
        })?;

        debug!("Claude token exchange successful");

        Ok(TokenInfo::new(
            token_response.access_token,
            refresh_token,
            token_response.expires_in,
        )
        .with_provider(PROVIDER_ID))
    }

    /// Refresh Claude access token using JSON body.
    ///
    /// Like token exchange, Claude's refresh endpoint expects JSON requests.
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenInfo> {
        // Parse composite token format if present
        let (base_refresh, project_id, managed_project_id) = parse_composite_token(refresh_token);

        debug!("Refreshing Claude access token");

        // Claude uses JSON body for token refresh (NOT form-encoded)
        let request_body = serde_json::json!({
            "grant_type": "refresh_token",
            "client_id": self.config.client_id,
            "refresh_token": base_refresh,
        });

        let response = self
            .http_client
            .post(&self.config.token_url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            // Try to parse error response
            if let Ok(error) = serde_json::from_str::<TokenErrorResponse>(&body) {
                warn!(
                    error = %error.error,
                    description = ?error.error_description,
                    "Claude token refresh failed"
                );

                if error.error == "invalid_grant" {
                    return Err(Error::Auth(AuthError::InvalidGrant));
                }

                return Err(Error::api(
                    status.as_u16(),
                    error
                        .error_description
                        .unwrap_or_else(|| error.error.clone()),
                    None,
                ));
            }

            return Err(Error::api(status.as_u16(), body, None));
        }

        let token_response: TokenResponse = serde_json::from_str(&body)?;

        debug!("Claude token refresh successful");

        // Use new refresh token if provided, otherwise preserve the old one
        let new_refresh = token_response
            .refresh_token
            .unwrap_or_else(|| base_refresh.clone());

        let mut token = TokenInfo::new(
            token_response.access_token,
            new_refresh,
            token_response.expires_in,
        )
        .with_provider(PROVIDER_ID);

        // Preserve project IDs from composite token (though Claude doesn't use them)
        if let Some(project) = project_id {
            token = token.with_project_ids(&project, managed_project_id.as_deref());
        }

        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_id() {
        let provider = ClaudeProvider::new();
        assert_eq!(provider.provider_id(), "claude");
    }

    #[test]
    fn test_provider_name() {
        let provider = ClaudeProvider::new();
        assert_eq!(provider.name(), "Claude (Anthropic)");
    }

    #[test]
    fn test_oauth_config() {
        let provider = ClaudeProvider::new();
        let config = provider.oauth_config();

        assert_eq!(config.client_id, "9d1c250a-e61b-44d9-88ed-5944d1962f5e");
        assert!(config.client_secret.is_none());
        assert!(config.auth_url.contains("claude.ai"));
        assert!(config.token_url.contains("anthropic.com"));
        assert!(config.scopes.contains(&"user:inference".to_string()));
    }

    #[test]
    fn test_does_not_require_client_secret() {
        let provider = ClaudeProvider::new();
        assert!(!provider.requires_client_secret());
    }

    #[test]
    fn test_no_callback_port() {
        let provider = ClaudeProvider::new();
        // Claude uses Anthropic's hosted redirect, not a local server
        assert!(provider.callback_port().is_none());
    }

    #[test]
    fn test_build_auth_url_contains_code_true() {
        let provider = ClaudeProvider::new();
        let pkce = Pkce::generate();
        let state = "test_state";

        let url = provider.build_auth_url(&pkce, state);

        // Must contain code=true (Claude-specific requirement)
        assert!(url.contains("code=true"), "URL must contain code=true");

        // Standard OAuth parameters
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id="));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("scope="));
        assert!(url.contains("code_challenge="));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=test_state"));
    }

    #[test]
    fn test_build_auth_url_contains_pkce_challenge() {
        let provider = ClaudeProvider::new();
        let pkce = Pkce::generate();
        let state = "test_state";

        let url = provider.build_auth_url(&pkce, state);

        // URL should contain the PKCE challenge
        assert!(
            url.contains(&pkce.challenge),
            "URL should contain the PKCE challenge"
        );
    }

    #[test]
    fn test_build_auth_url_url_encodes_special_chars() {
        let provider = ClaudeProvider::new();
        let pkce = Pkce::generate();
        let state = "state with spaces";

        let url = provider.build_auth_url(&pkce, state);

        // Spaces should be URL-encoded
        assert!(url.contains("state%20with%20spaces") || url.contains("state+with+spaces"));
    }

    #[test]
    fn test_default_trait() {
        let provider = ClaudeProvider::default();
        assert_eq!(provider.provider_id(), "claude");
    }

    #[test]
    fn test_with_http_client() {
        let client = reqwest::Client::new();
        let provider = ClaudeProvider::with_http_client(client);
        assert_eq!(provider.provider_id(), "claude");
    }

    #[test]
    fn test_with_config() {
        let config = OAuthConfig::builder()
            .client_id("custom-client-id")
            .auth_url("https://custom.auth.url")
            .token_url("https://custom.token.url")
            .redirect_uri("https://custom.redirect.uri")
            .scopes(vec!["custom:scope"])
            .build();

        let client = reqwest::Client::new();
        let provider = ClaudeProvider::with_config(config, client);

        assert_eq!(provider.oauth_config().client_id, "custom-client-id");
    }
}
