//! OAuth configuration for different providers.
//!
//! This module provides the [`OAuthConfig`] struct for configuring OAuth 2.0
//! authentication with different providers (Claude, Gemini, etc.).
//!
//! # Example
//!
//! ```rust,ignore
//! use gate::auth::config::OAuthConfig;
//!
//! // Use provider-specific defaults
//! let claude_config = OAuthConfig::claude();
//! let gemini_config = OAuthConfig::gemini();
//!
//! // Or build a custom configuration
//! let custom = OAuthConfig::builder()
//!     .client_id("my-client-id")
//!     .auth_url("https://example.com/oauth/authorize")
//!     .token_url("https://example.com/oauth/token")
//!     .redirect_uri("http://localhost:8080/callback")
//!     .scopes(vec!["openid", "profile"])
//!     .build();
//! ```

/// OAuth 2.0 configuration.
///
/// Contains all the settings needed to initiate and complete an OAuth flow
/// with a specific provider.
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    /// OAuth client ID.
    ///
    /// The public identifier for the application, provided by the OAuth provider.
    pub client_id: String,

    /// OAuth client secret (optional).
    ///
    /// Some providers (like Anthropic) don't require a client secret when using PKCE.
    /// Others (like Google) may require it even with PKCE.
    pub client_secret: Option<String>,

    /// Authorization URL for initiating OAuth flow.
    ///
    /// Users are redirected here to grant authorization.
    pub auth_url: String,

    /// Token URL for exchanging authorization code.
    ///
    /// The application sends the authorization code here to get tokens.
    pub token_url: String,

    /// Redirect URI for OAuth callback.
    ///
    /// Where the provider redirects after authorization. Must match
    /// what's registered with the provider.
    pub redirect_uri: String,

    /// OAuth scopes to request.
    ///
    /// Determines what permissions the application requests.
    pub scopes: Vec<String>,

    /// Local callback port for OAuth redirect (optional).
    ///
    /// If set, the redirect_uri will use this port for a local HTTP server
    /// to receive the callback.
    pub callback_port: Option<u16>,
}

impl OAuthConfig {
    /// Create a new OAuth config builder.
    #[must_use]
    pub fn builder() -> OAuthConfigBuilder {
        OAuthConfigBuilder::default()
    }

    /// Create OAuth configuration for Claude (Anthropic).
    ///
    /// Uses Anthropic's OAuth defaults with PKCE support.
    /// Client secret is not required for Anthropic OAuth with PKCE.
    #[must_use]
    pub fn claude() -> Self {
        Self {
            client_id: "9d1c250a-e61b-44d9-88ed-5944d1962f5e".to_string(),
            client_secret: None,
            auth_url: "https://claude.ai/oauth/authorize".to_string(),
            token_url: "https://console.anthropic.com/v1/oauth/token".to_string(),
            redirect_uri: "https://console.anthropic.com/oauth/code/callback".to_string(),
            scopes: vec![
                "org:create_api_key".to_string(),
                "user:profile".to_string(),
                "user:inference".to_string(),
            ],
            callback_port: None, // Anthropic uses their own redirect
        }
    }

    /// Create OAuth configuration for Gemini (Google Cloud Code).
    ///
    /// Uses Google's OAuth defaults for Cloud Code API access.
    ///
    /// # Security Note
    ///
    /// The client secret included here follows Google's "installed application"
    /// OAuth pattern. Per Google's documentation, installed app secrets are not
    /// truly confidential as they ship with the application. Google recommends
    /// PKCE for additional security, which this flow uses (S256 challenge).
    ///
    /// See: <https://developers.google.com/identity/protocols/oauth2/native-app>
    #[must_use]
    pub fn gemini() -> Self {
        // Note: This secret is for an "installed application" (native desktop app).
        // Google treats these secrets as non-confidential. PKCE provides additional
        // security for the authorization code exchange.
        Self {
            client_id: "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com"
                .to_string(),
            client_secret: Some("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".to_string()),
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_url: "https://oauth2.googleapis.com/token".to_string(),
            redirect_uri: "http://127.0.0.1:51121/callback".to_string(),
            scopes: vec![
                "https://www.googleapis.com/auth/cloud-platform".to_string(),
                "https://www.googleapis.com/auth/userinfo.email".to_string(),
                "https://www.googleapis.com/auth/userinfo.profile".to_string(),
                "https://www.googleapis.com/auth/cclog".to_string(),
                "https://www.googleapis.com/auth/experimentsandconfigs".to_string(),
            ],
            callback_port: Some(51121),
        }
    }

    /// Get the callback port, defaulting to extracting from redirect_uri.
    pub fn get_callback_port(&self) -> Option<u16> {
        if let Some(port) = self.callback_port {
            return Some(port);
        }

        // Try to extract from redirect_uri
        self.redirect_uri
            .parse::<url::Url>()
            .ok()
            .and_then(|u| u.port())
    }
}

/// Builder for OAuthConfig.
#[derive(Debug, Default)]
pub struct OAuthConfigBuilder {
    client_id: Option<String>,
    client_secret: Option<String>,
    auth_url: Option<String>,
    token_url: Option<String>,
    redirect_uri: Option<String>,
    scopes: Vec<String>,
    callback_port: Option<u16>,
}

impl OAuthConfigBuilder {
    /// Set the client ID.
    #[must_use]
    pub fn client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = Some(client_id.into());
        self
    }

    /// Set the client secret.
    #[must_use]
    pub fn client_secret(mut self, client_secret: impl Into<String>) -> Self {
        self.client_secret = Some(client_secret.into());
        self
    }

    /// Set the authorization URL.
    #[must_use]
    pub fn auth_url(mut self, auth_url: impl Into<String>) -> Self {
        self.auth_url = Some(auth_url.into());
        self
    }

    /// Set the token URL.
    #[must_use]
    pub fn token_url(mut self, token_url: impl Into<String>) -> Self {
        self.token_url = Some(token_url.into());
        self
    }

    /// Set the redirect URI.
    #[must_use]
    pub fn redirect_uri(mut self, redirect_uri: impl Into<String>) -> Self {
        self.redirect_uri = Some(redirect_uri.into());
        self
    }

    /// Set the OAuth scopes.
    #[must_use]
    pub fn scopes(mut self, scopes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.scopes = scopes.into_iter().map(Into::into).collect();
        self
    }

    /// Add a single scope.
    #[must_use]
    pub fn scope(mut self, scope: impl Into<String>) -> Self {
        self.scopes.push(scope.into());
        self
    }

    /// Set the callback port.
    #[must_use]
    pub fn callback_port(mut self, port: u16) -> Self {
        self.callback_port = Some(port);
        self
    }

    /// Build the OAuthConfig.
    ///
    /// # Panics
    ///
    /// Panics if required fields (client_id, auth_url, token_url, redirect_uri)
    /// are not set.
    #[must_use]
    pub fn build(self) -> OAuthConfig {
        OAuthConfig {
            client_id: self.client_id.expect("client_id is required"),
            client_secret: self.client_secret,
            auth_url: self.auth_url.expect("auth_url is required"),
            token_url: self.token_url.expect("token_url is required"),
            redirect_uri: self.redirect_uri.expect("redirect_uri is required"),
            scopes: self.scopes,
            callback_port: self.callback_port,
        }
    }

    /// Try to build the OAuthConfig, returning an error if required fields are missing.
    pub fn try_build(self) -> Result<OAuthConfig, &'static str> {
        Ok(OAuthConfig {
            client_id: self.client_id.ok_or("client_id is required")?,
            client_secret: self.client_secret,
            auth_url: self.auth_url.ok_or("auth_url is required")?,
            token_url: self.token_url.ok_or("token_url is required")?,
            redirect_uri: self.redirect_uri.ok_or("redirect_uri is required")?,
            scopes: self.scopes,
            callback_port: self.callback_port,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_config() {
        let config = OAuthConfig::claude();

        assert!(!config.client_id.is_empty());
        assert!(config.client_secret.is_none()); // Anthropic doesn't require secret with PKCE
        assert!(config.auth_url.contains("claude.ai"));
        assert!(config.token_url.contains("anthropic.com"));
        assert!(!config.scopes.is_empty());
        assert!(config.scopes.contains(&"user:inference".to_string()));
    }

    #[test]
    fn test_gemini_config() {
        let config = OAuthConfig::gemini();

        assert!(!config.client_id.is_empty());
        assert!(config.client_secret.is_some()); // Google requires secret
        assert!(config.auth_url.contains("google.com"));
        assert!(config.token_url.contains("googleapis.com"));
        assert!(!config.scopes.is_empty());
        assert!(config.scopes.iter().any(|s| s.contains("cloud-platform")));
        assert_eq!(config.callback_port, Some(51121));
    }

    #[test]
    fn test_builder() {
        let config = OAuthConfig::builder()
            .client_id("test-client")
            .client_secret("test-secret")
            .auth_url("https://example.com/auth")
            .token_url("https://example.com/token")
            .redirect_uri("http://localhost:8080/callback")
            .scopes(vec!["openid", "profile"])
            .callback_port(8080)
            .build();

        assert_eq!(config.client_id, "test-client");
        assert_eq!(config.client_secret, Some("test-secret".to_string()));
        assert_eq!(config.auth_url, "https://example.com/auth");
        assert_eq!(config.token_url, "https://example.com/token");
        assert_eq!(config.redirect_uri, "http://localhost:8080/callback");
        assert_eq!(config.scopes, vec!["openid", "profile"]);
        assert_eq!(config.callback_port, Some(8080));
    }

    #[test]
    fn test_builder_scope_accumulation() {
        let config = OAuthConfig::builder()
            .client_id("test")
            .auth_url("https://example.com/auth")
            .token_url("https://example.com/token")
            .redirect_uri("http://localhost/callback")
            .scope("openid")
            .scope("profile")
            .scope("email")
            .build();

        assert_eq!(config.scopes.len(), 3);
        assert!(config.scopes.contains(&"openid".to_string()));
        assert!(config.scopes.contains(&"profile".to_string()));
        assert!(config.scopes.contains(&"email".to_string()));
    }

    #[test]
    fn test_try_build_success() {
        let result = OAuthConfig::builder()
            .client_id("test")
            .auth_url("https://example.com/auth")
            .token_url("https://example.com/token")
            .redirect_uri("http://localhost/callback")
            .try_build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_try_build_missing_client_id() {
        let result = OAuthConfig::builder()
            .auth_url("https://example.com/auth")
            .token_url("https://example.com/token")
            .redirect_uri("http://localhost/callback")
            .try_build();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("client_id"));
    }

    #[test]
    fn test_get_callback_port_explicit() {
        let config = OAuthConfig::builder()
            .client_id("test")
            .auth_url("https://example.com/auth")
            .token_url("https://example.com/token")
            .redirect_uri("http://localhost:9999/callback")
            .callback_port(8080)
            .build();

        // Explicit port takes precedence
        assert_eq!(config.get_callback_port(), Some(8080));
    }

    #[test]
    fn test_get_callback_port_from_uri() {
        let config = OAuthConfig::builder()
            .client_id("test")
            .auth_url("https://example.com/auth")
            .token_url("https://example.com/token")
            .redirect_uri("http://localhost:9999/callback")
            .build();

        // Extract from redirect_uri when callback_port not set
        assert_eq!(config.get_callback_port(), Some(9999));
    }

    #[test]
    fn test_get_callback_port_none() {
        let config = OAuthConfig::builder()
            .client_id("test")
            .auth_url("https://example.com/auth")
            .token_url("https://example.com/token")
            .redirect_uri("https://example.com/callback") // No port in URL
            .build();

        assert_eq!(config.get_callback_port(), None);
    }

    #[test]
    fn test_clone() {
        let config = OAuthConfig::gemini();
        let cloned = config.clone();

        assert_eq!(config.client_id, cloned.client_id);
        assert_eq!(config.client_secret, cloned.client_secret);
        assert_eq!(config.auth_url, cloned.auth_url);
        assert_eq!(config.token_url, cloned.token_url);
        assert_eq!(config.redirect_uri, cloned.redirect_uri);
        assert_eq!(config.scopes, cloned.scopes);
        assert_eq!(config.callback_port, cloned.callback_port);
    }
}
