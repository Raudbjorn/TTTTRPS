//! OAuth provider implementations.
//!
//! This module provides provider-specific OAuth implementations for different
//! LLM providers (Claude/Anthropic, Gemini/Google Cloud Code, GitHub Copilot).
//!
//! # Architecture
//!
//! The [`OAuthProvider`] trait defines the interface that all OAuth providers must
//! implement. Each provider can customize:
//!
//! - Authorization URL construction
//! - Token exchange request format (JSON vs form-encoded)
//! - Token refresh request format
//! - Provider-specific parameters
//!
//! # Providers
//!
//! - [`ClaudeProvider`] - Anthropic OAuth with JSON-encoded token requests (PKCE)
//! - [`GeminiProvider`] - Google OAuth with form-encoded token requests (PKCE)
//! - [`CopilotProvider`] - GitHub OAuth with Device Code flow (RFC 8628)
//!
//! # Example
//!
//! ```rust,ignore
//! use gate::providers::{OAuthProvider, ClaudeProvider, GeminiProvider};
//! use gate::auth::Pkce;
//!
//! // Create providers
//! let claude = ClaudeProvider::new();
//! let gemini = GeminiProvider::new();
//!
//! // Get provider info
//! println!("Claude ID: {}", claude.provider_id());
//! println!("Gemini ID: {}", gemini.provider_id());
//!
//! // Build authorization URL
//! let pkce = Pkce::generate();
//! let state = "random_state_value";
//! let url = claude.build_auth_url(&pkce, state);
//! ```

pub mod claude;
pub mod copilot;
pub mod gemini;

use async_trait::async_trait;

use crate::oauth::auth::{OAuthConfig, Pkce};
use crate::oauth::error::Result;
use crate::oauth::token::TokenInfo;

// Re-export providers
pub use claude::ClaudeProvider;
pub use copilot::CopilotProvider;
pub use gemini::GeminiProvider;

/// OAuth provider trait for LLM authentication.
///
/// This trait defines the interface for OAuth providers, allowing each provider
/// to customize their OAuth flow while sharing common infrastructure.
///
/// # Key Differences Between Providers
///
/// | Provider | Token Request Format | Client Secret | Extra Auth URL Params |
/// |----------|---------------------|---------------|----------------------|
/// | Claude   | JSON                | None (PKCE)   | `code=true`          |
/// | Gemini   | Form-encoded        | Required      | `access_type=offline`, `prompt=consent` |
///
/// # Implementing a New Provider
///
/// ```rust,ignore
/// use gate::providers::OAuthProvider;
/// use gate::auth::{OAuthConfig, Pkce};
/// use gate::error::Result;
/// use gate::token::TokenInfo;
///
/// struct MyProvider {
///     config: OAuthConfig,
///     http_client: reqwest::Client,
/// }
///
/// #[async_trait::async_trait]
/// impl OAuthProvider for MyProvider {
///     fn provider_id(&self) -> &str { "my_provider" }
///     fn name(&self) -> &str { "My Provider" }
///     fn oauth_config(&self) -> &OAuthConfig { &self.config }
///
///     async fn exchange_code(&self, code: &str, verifier: &str) -> Result<TokenInfo> {
///         // Custom token exchange logic
///     }
///
///     async fn refresh_token(&self, refresh_token: &str) -> Result<TokenInfo> {
///         // Custom token refresh logic
///     }
/// }
/// ```
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    /// Get the unique provider identifier.
    ///
    /// This is used for storage namespacing and logging.
    /// Examples: "anthropic", "gemini"
    fn provider_id(&self) -> &str;

    /// Get the human-readable provider name.
    ///
    /// Used for display purposes and error messages.
    /// Examples: "Claude (Anthropic)", "Gemini (Google)"
    fn name(&self) -> &str;

    /// Get the OAuth configuration for this provider.
    ///
    /// Contains client ID, endpoints, scopes, and optional client secret.
    fn oauth_config(&self) -> &OAuthConfig;

    /// Build the authorization URL for initiating OAuth flow.
    ///
    /// The default implementation constructs a standard OAuth authorization URL
    /// with PKCE support. Providers can override this to add custom parameters.
    ///
    /// # Arguments
    ///
    /// * `pkce` - PKCE data containing the code challenge
    /// * `state` - State parameter for CSRF protection
    ///
    /// # Returns
    ///
    /// The complete authorization URL for the user to visit.
    ///
    /// # Default Implementation
    ///
    /// Constructs URL with parameters:
    /// - `client_id`
    /// - `redirect_uri`
    /// - `response_type=code`
    /// - `scope`
    /// - `code_challenge` (from PKCE)
    /// - `code_challenge_method=S256`
    /// - `state`
    fn build_auth_url(&self, pkce: &Pkce, state: &str) -> String {
        let config = self.oauth_config();
        let scopes = config.scopes.join(" ");

        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
            config.auth_url,
            urlencoding::encode(&config.client_id),
            urlencoding::encode(&config.redirect_uri),
            urlencoding::encode(&scopes),
            urlencoding::encode(&pkce.challenge),
            urlencoding::encode(state),
        )
    }

    /// Exchange an authorization code for tokens.
    ///
    /// Completes the OAuth flow by exchanging the authorization code
    /// (received via callback) for access and refresh tokens.
    ///
    /// # Arguments
    ///
    /// * `code` - Authorization code from the OAuth callback
    /// * `verifier` - PKCE code verifier
    ///
    /// # Provider-Specific Behavior
    ///
    /// - **Claude**: Sends JSON-encoded request body
    /// - **Gemini**: Sends form-encoded request body with client_secret
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The code is invalid or expired
    /// - The verifier doesn't match the challenge
    /// - Network error occurs
    /// - Response cannot be parsed
    async fn exchange_code(&self, code: &str, verifier: &str) -> Result<TokenInfo>;

    /// Refresh an access token using a refresh token.
    ///
    /// Exchanges the refresh token for a new access token. Note that
    /// some providers may not return a new refresh token on refresh.
    ///
    /// # Arguments
    ///
    /// * `refresh_token` - The refresh token (may be in composite format)
    ///
    /// # Composite Token Handling
    ///
    /// If the refresh token is in composite format (`refresh|project|managed`),
    /// implementations should:
    /// 1. Extract the base refresh token for the API call
    /// 2. Preserve and re-attach project IDs to the resulting TokenInfo
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The refresh token is invalid or revoked (`AuthError::InvalidGrant`)
    /// - Network error occurs
    /// - Response cannot be parsed
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenInfo>;

    /// Check if this provider requires a client secret.
    ///
    /// Claude uses PKCE-only authentication without a client secret.
    /// Google requires both PKCE and client secret.
    fn requires_client_secret(&self) -> bool {
        self.oauth_config().client_secret.is_some()
    }

    /// Get the callback port for local OAuth redirect server.
    ///
    /// Returns `None` if the provider uses a remote redirect (e.g., Claude).
    fn callback_port(&self) -> Option<u16> {
        self.oauth_config().callback_port
    }
}

/// Token response from OAuth token endpoint.
///
/// Shared between providers since most OAuth endpoints return similar responses.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    pub expires_in: i64,
    #[serde(default)]
    #[allow(dead_code)]
    pub token_type: Option<String>,
}

/// Error response from OAuth token endpoint.
///
/// Standard OAuth error format used by most providers.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct TokenErrorResponse {
    pub error: String,
    #[serde(default)]
    pub error_description: Option<String>,
}

/// Parse a composite refresh token into its parts.
///
/// Format: `base_refresh|project_id|managed_project_id`
///
/// # Returns
///
/// A tuple of `(base_refresh, project_id, managed_project_id)` where:
/// - `base_refresh` - The actual refresh token to send to the OAuth endpoint
/// - `project_id` - Optional Cloud Code project ID
/// - `managed_project_id` - Optional managed project ID
pub(crate) fn parse_composite_token(token: &str) -> (String, Option<String>, Option<String>) {
    let parts: Vec<&str> = token.split('|').collect();
    let base = parts[0].to_string();
    let project = parts
        .get(1)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let managed = parts
        .get(2)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    (base, project, managed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_composite_token_simple() {
        let (base, project, managed) = parse_composite_token("refresh_token_here");
        assert_eq!(base, "refresh_token_here");
        assert!(project.is_none());
        assert!(managed.is_none());
    }

    #[test]
    fn test_parse_composite_token_with_project() {
        let (base, project, managed) = parse_composite_token("refresh|proj-123");
        assert_eq!(base, "refresh");
        assert_eq!(project.as_deref(), Some("proj-123"));
        assert!(managed.is_none());
    }

    #[test]
    fn test_parse_composite_token_with_both() {
        let (base, project, managed) = parse_composite_token("refresh|proj-123|managed-456");
        assert_eq!(base, "refresh");
        assert_eq!(project.as_deref(), Some("proj-123"));
        assert_eq!(managed.as_deref(), Some("managed-456"));
    }

    #[test]
    fn test_parse_composite_token_with_empty_parts() {
        // Empty project ID
        let (base, project, managed) = parse_composite_token("refresh||managed-456");
        assert_eq!(base, "refresh");
        assert!(project.is_none());
        assert_eq!(managed.as_deref(), Some("managed-456"));

        // Empty managed project ID
        let (base, project, managed) = parse_composite_token("refresh|proj-123|");
        assert_eq!(base, "refresh");
        assert_eq!(project.as_deref(), Some("proj-123"));
        assert!(managed.is_none());
    }
}
