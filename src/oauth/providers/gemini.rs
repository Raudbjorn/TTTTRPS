//! Gemini (Google Cloud Code) OAuth provider implementation.
//!
//! This module implements the [`OAuthProvider`] trait for Google's OAuth 2.0 flow
//! targeting the Cloud Code API (used for Gemini access).
//!
//! # Key Characteristics
//!
//! - **Token Request Format**: Form-encoded (standard OAuth)
//! - **Client Secret**: Required (even with PKCE)
//! - **Auth URL Parameters**: Requires `access_type=offline` and `prompt=consent`
//! - **Redirect**: Uses local callback server on port 51121
//!
//! # OAuth Endpoints
//!
//! | Endpoint | URL |
//! |----------|-----|
//! | Authorization | `https://accounts.google.com/o/oauth2/v2/auth` |
//! | Token | `https://oauth2.googleapis.com/token` |
//! | Redirect | `http://127.0.0.1:51121/callback` |
//!
//! # Scopes
//!
//! - `https://www.googleapis.com/auth/cloud-platform` - Cloud platform access
//! - `https://www.googleapis.com/auth/userinfo.email` - User email
//! - `https://www.googleapis.com/auth/userinfo.profile` - User profile
//! - `https://www.googleapis.com/auth/cclog` - Cloud Code logging
//! - `https://www.googleapis.com/auth/experimentsandconfigs` - Experiments
//!
//! # Example
//!
//! ```rust,ignore
//! use gate::providers::{OAuthProvider, GeminiProvider};
//! use gate::auth::Pkce;
//!
//! let provider = GeminiProvider::new();
//!
//! // Build authorization URL
//! let pkce = Pkce::generate();
//! let state = "random_state";
//! let url = provider.build_auth_url(&pkce, state);
//!
//! // Google's URL includes offline access and consent prompt
//! assert!(url.contains("access_type=offline"));
//! assert!(url.contains("prompt=consent"));
//!
//! // Exchange code for tokens (uses form-encoded body with client_secret)
//! let token = provider.exchange_code("auth_code", &pkce.verifier).await?;
//! ```

use async_trait::async_trait;
use tracing::{debug, warn};

use super::{parse_composite_token, OAuthProvider, TokenErrorResponse, TokenResponse};
use crate::oauth::auth::{OAuthConfig, Pkce};
use crate::oauth::error::{AuthError, Error, Result};
use crate::oauth::token::TokenInfo;

/// Provider identifier for Gemini/Google.
pub const PROVIDER_ID: &str = "gemini";

/// Human-readable provider name.
pub const PROVIDER_NAME: &str = "Gemini (Google)";

/// Gemini OAuth provider.
///
/// Implements OAuth 2.0 with PKCE for Google's Cloud Code API. Unlike Claude,
/// Google requires both a client secret and PKCE for native applications.
///
/// # Callback Server
///
/// Gemini authentication requires a local HTTP server to receive the OAuth
/// callback. The default port is 51121. The callback URL must match exactly
/// what is registered with Google Cloud Console.
///
/// # Example
///
/// ```rust,ignore
/// use gate::providers::GeminiProvider;
///
/// // Create with default configuration
/// let provider = GeminiProvider::new();
///
/// // Check the callback port
/// assert_eq!(provider.callback_port(), Some(51121));
///
/// // Or create with custom HTTP client
/// let client = reqwest::Client::builder()
///     .timeout(std::time::Duration::from_secs(30))
///     .build()?;
/// let provider = GeminiProvider::with_http_client(client);
/// ```
#[derive(Clone)]
pub struct GeminiProvider {
    config: OAuthConfig,
    http_client: reqwest::Client,
}

impl GeminiProvider {
    /// Create a new GeminiProvider with default configuration.
    ///
    /// Uses the standard Gemini/Cloud Code OAuth configuration from
    /// [`OAuthConfig::gemini()`].
    #[must_use]
    pub fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            config: OAuthConfig::gemini(),
            http_client,
        }
    }

    /// Create a GeminiProvider with a custom HTTP client.
    ///
    /// Useful for configuring timeouts, proxies, or custom TLS settings.
    ///
    /// # Arguments
    ///
    /// * `http_client` - Pre-configured reqwest client
    #[must_use]
    pub fn with_http_client(http_client: reqwest::Client) -> Self {
        Self {
            config: OAuthConfig::gemini(),
            http_client,
        }
    }

    /// Create a GeminiProvider with custom configuration and HTTP client.
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

impl Default for GeminiProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OAuthProvider for GeminiProvider {
    fn provider_id(&self) -> &str {
        PROVIDER_ID
    }

    fn name(&self) -> &str {
        PROVIDER_NAME
    }

    fn oauth_config(&self) -> &OAuthConfig {
        &self.config
    }

    /// Build Google's authorization URL.
    ///
    /// Google OAuth requires additional parameters for offline access:
    /// - `access_type=offline` - Required to receive a refresh token
    /// - `prompt=consent` - Forces consent screen, ensuring refresh token is returned
    ///
    /// Without these parameters, Google may not return a refresh token on
    /// subsequent authorizations.
    fn build_auth_url(&self, pkce: &Pkce, state: &str) -> String {
        let scopes = self.config.scopes.join(" ");

        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={}&code_challenge_method=S256&state={}&access_type=offline&prompt=consent",
            self.config.auth_url,
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(&self.config.redirect_uri),
            urlencoding::encode(&scopes),
            urlencoding::encode(&pkce.challenge),
            urlencoding::encode(state),
        )
    }

    /// Exchange authorization code for tokens using form-encoded body.
    ///
    /// Google's token endpoint expects standard form-encoded requests.
    /// This includes the client_secret which is required for native apps.
    async fn exchange_code(&self, code: &str, verifier: &str) -> Result<TokenInfo> {
        debug!("Exchanging authorization code for Gemini tokens");

        // Build form data
        let mut form_data = vec![
            ("code", code.to_string()),
            ("code_verifier", verifier.to_string()),
            ("grant_type", "authorization_code".to_string()),
            ("redirect_uri", self.config.redirect_uri.clone()),
            ("client_id", self.config.client_id.clone()),
        ];

        // Add client_secret (required for Google)
        if let Some(ref secret) = self.config.client_secret {
            form_data.push(("client_secret", secret.clone()));
        }

        let response = self
            .http_client
            .post(&self.config.token_url)
            .form(&form_data)
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
                    "Gemini token exchange failed"
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
                "No refresh token in response - ensure access_type=offline and prompt=consent"
                    .to_string(),
            ))
        })?;

        debug!("Gemini token exchange successful");

        Ok(TokenInfo::new(
            token_response.access_token,
            refresh_token,
            token_response.expires_in,
        )
        .with_provider(PROVIDER_ID))
    }

    /// Refresh Gemini access token using form-encoded body.
    ///
    /// Preserves composite token format for project IDs used in Cloud Code API.
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenInfo> {
        // Parse composite token format if present
        let (base_refresh, project_id, managed_project_id) = parse_composite_token(refresh_token);

        debug!("Refreshing Gemini access token");

        // Build form data
        let mut form_data = vec![
            ("refresh_token", base_refresh.clone()),
            ("grant_type", "refresh_token".to_string()),
            ("client_id", self.config.client_id.clone()),
        ];

        // Add client_secret (required for Google)
        if let Some(ref secret) = self.config.client_secret {
            form_data.push(("client_secret", secret.clone()));
        }

        let response = self
            .http_client
            .post(&self.config.token_url)
            .form(&form_data)
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
                    "Gemini token refresh failed"
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

        debug!("Gemini token refresh successful");

        // Google typically doesn't return a new refresh token on refresh
        // Use the old one if not provided
        let new_refresh = token_response
            .refresh_token
            .unwrap_or_else(|| base_refresh.clone());

        let mut token = TokenInfo::new(
            token_response.access_token,
            new_refresh,
            token_response.expires_in,
        )
        .with_provider(PROVIDER_ID);

        // Preserve project IDs from composite token (important for Cloud Code API)
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
        let provider = GeminiProvider::new();
        assert_eq!(provider.provider_id(), "gemini");
    }

    #[test]
    fn test_provider_name() {
        let provider = GeminiProvider::new();
        assert_eq!(provider.name(), "Gemini (Google)");
    }

    #[test]
    fn test_oauth_config() {
        let provider = GeminiProvider::new();
        let config = provider.oauth_config();

        assert!(!config.client_id.is_empty());
        assert!(config.client_secret.is_some());
        assert!(config.auth_url.contains("google.com"));
        assert!(config.token_url.contains("googleapis.com"));
        assert!(config.scopes.iter().any(|s| s.contains("cloud-platform")));
    }

    #[test]
    fn test_requires_client_secret() {
        let provider = GeminiProvider::new();
        assert!(provider.requires_client_secret());
    }

    #[test]
    fn test_callback_port() {
        let provider = GeminiProvider::new();
        assert_eq!(provider.callback_port(), Some(51121));
    }

    #[test]
    fn test_build_auth_url_contains_offline_access() {
        let provider = GeminiProvider::new();
        let pkce = Pkce::generate();
        let state = "test_state";

        let url = provider.build_auth_url(&pkce, state);

        // Must contain offline access parameters
        assert!(
            url.contains("access_type=offline"),
            "URL must contain access_type=offline"
        );
        assert!(
            url.contains("prompt=consent"),
            "URL must contain prompt=consent"
        );
    }

    #[test]
    fn test_build_auth_url_contains_standard_oauth_params() {
        let provider = GeminiProvider::new();
        let pkce = Pkce::generate();
        let state = "test_state";

        let url = provider.build_auth_url(&pkce, state);

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
        let provider = GeminiProvider::new();
        let pkce = Pkce::generate();
        let state = "test_state";

        let url = provider.build_auth_url(&pkce, state);

        assert!(
            url.contains(&pkce.challenge),
            "URL should contain the PKCE challenge"
        );
    }

    #[test]
    fn test_build_auth_url_starts_with_google() {
        let provider = GeminiProvider::new();
        let pkce = Pkce::generate();
        let state = "test_state";

        let url = provider.build_auth_url(&pkce, state);

        assert!(
            url.starts_with("https://accounts.google.com/"),
            "URL should start with Google's auth endpoint"
        );
    }

    #[test]
    fn test_default_trait() {
        let provider = GeminiProvider::default();
        assert_eq!(provider.provider_id(), "gemini");
    }

    #[test]
    fn test_with_http_client() {
        let client = reqwest::Client::new();
        let provider = GeminiProvider::with_http_client(client);
        assert_eq!(provider.provider_id(), "gemini");
    }

    #[test]
    fn test_with_config() {
        let config = OAuthConfig::builder()
            .client_id("custom-client-id")
            .client_secret("custom-secret")
            .auth_url("https://custom.auth.url")
            .token_url("https://custom.token.url")
            .redirect_uri("https://custom.redirect.uri")
            .scopes(vec!["custom:scope"])
            .callback_port(8080)
            .build();

        let client = reqwest::Client::new();
        let provider = GeminiProvider::with_config(config, client);

        assert_eq!(provider.oauth_config().client_id, "custom-client-id");
        assert_eq!(provider.callback_port(), Some(8080));
    }

    #[test]
    fn test_redirect_uri_is_localhost() {
        let provider = GeminiProvider::new();
        let config = provider.oauth_config();

        assert!(
            config.redirect_uri.contains("127.0.0.1"),
            "Redirect URI should use localhost"
        );
        assert!(
            config.redirect_uri.contains("51121"),
            "Redirect URI should use port 51121"
        );
    }
}
