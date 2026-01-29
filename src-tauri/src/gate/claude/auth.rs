//! OAuth 2.0 authentication with PKCE support.
//!
//! This module implements the OAuth 2.0 authorization code flow with PKCE
//! (Proof Key for Code Exchange) for secure authentication with Anthropic's API.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{debug, info, instrument, warn};
use url::Url;

use super::error::{Error, Result};
use crate::gate::storage::TokenStorage;
use crate::gate::token::TokenInfo;

/// Default OAuth client ID (public by design, security via PKCE).
pub const DEFAULT_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";

/// Default authorization URL.
pub const DEFAULT_AUTHORIZE_URL: &str = "https://claude.ai/oauth/authorize";

/// Default token URL.
pub const DEFAULT_TOKEN_URL: &str = "https://console.anthropic.com/v1/oauth/token";

/// Default redirect URI.
pub const DEFAULT_REDIRECT_URI: &str = "https://console.anthropic.com/oauth/code/callback";

/// Default OAuth scopes.
pub const DEFAULT_SCOPES: &[&str] = &["org:create_api_key", "user:profile", "user:inference"];

/// PKCE challenge method.
const PKCE_METHOD: &str = "S256";

/// PKCE verifier length in bytes.
const PKCE_VERIFIER_LENGTH: usize = 32;

/// PKCE (Proof Key for Code Exchange) data.
#[derive(Debug, Clone)]
pub struct Pkce {
    /// The code verifier (sent during token exchange).
    pub verifier: String,
    /// The code challenge (sent during authorization).
    pub challenge: String,
    /// The challenge method (always "S256").
    pub method: &'static str,
}

impl Pkce {
    /// Generate a new PKCE challenge/verifier pair.
    #[must_use]
    pub fn generate() -> Self {
        // Generate random bytes for verifier
        let mut rng = rand::thread_rng();
        let random_bytes: [u8; PKCE_VERIFIER_LENGTH] = rng.gen();

        // Base64url encode the verifier
        let verifier = URL_SAFE_NO_PAD.encode(random_bytes);

        // SHA256 hash the verifier and base64url encode for challenge
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        let challenge = URL_SAFE_NO_PAD.encode(hash);

        Self {
            verifier,
            challenge,
            method: PKCE_METHOD,
        }
    }

    /// Verify that a challenge matches a verifier.
    #[must_use]
    pub fn verify(verifier: &str, challenge: &str) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        let expected = URL_SAFE_NO_PAD.encode(hash);
        expected == challenge
    }
}

/// OAuth configuration.
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    /// OAuth client ID.
    pub client_id: String,
    /// Authorization endpoint URL.
    pub authorize_url: String,
    /// Token endpoint URL.
    pub token_url: String,
    /// Redirect URI.
    pub redirect_uri: String,
    /// Requested scopes.
    pub scopes: Vec<String>,
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            client_id: DEFAULT_CLIENT_ID.to_string(),
            authorize_url: DEFAULT_AUTHORIZE_URL.to_string(),
            token_url: DEFAULT_TOKEN_URL.to_string(),
            redirect_uri: DEFAULT_REDIRECT_URI.to_string(),
            scopes: DEFAULT_SCOPES.iter().map(|&s| s.to_string()).collect(),
        }
    }
}

impl OAuthConfig {
    /// Create a new OAuth config with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a custom client ID.
    #[must_use]
    pub fn with_client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = client_id.into();
        self
    }

    /// Set custom scopes.
    #[must_use]
    pub fn with_scopes(mut self, scopes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.scopes = scopes.into_iter().map(Into::into).collect();
        self
    }
}

/// State for an in-progress OAuth flow.
#[derive(Debug)]
pub struct OAuthFlowState {
    /// PKCE verifier (needed for token exchange).
    pub pkce: Pkce,
    /// State parameter for CSRF protection.
    pub state: String,
}

/// OAuth flow handler.
pub struct OAuthFlow<S: TokenStorage> {
    config: OAuthConfig,
    storage: S,
    http_client: reqwest::Client,
    /// Current flow state (set after starting authorization).
    flow_state: Option<OAuthFlowState>,
}

impl<S: TokenStorage> OAuthFlow<S> {
    /// Create a new OAuth flow handler.
    pub fn new(storage: S) -> Self {
        Self::with_config(storage, OAuthConfig::default())
    }

    /// Create a new OAuth flow handler with custom config.
    pub fn with_config(storage: S, config: OAuthConfig) -> Self {
        Self {
            config,
            storage,
            http_client: reqwest::Client::new(),
            flow_state: None,
        }
    }

    /// Get the storage backend.
    pub fn storage(&self) -> &S {
        &self.storage
    }

    /// Get mutable access to the storage backend.
    pub fn storage_mut(&mut self) -> &mut S {
        &mut self.storage
    }

    /// Check if a valid token exists.
    #[instrument(skip(self))]
    pub async fn is_authenticated(&self) -> Result<bool> {
        match self.storage.load("anthropic").await? {
            Some(token) => {
                if token.is_expired() {
                    debug!("Token exists but is expired");
                    Ok(false)
                } else {
                    debug!("Valid token exists");
                    Ok(true)
                }
            }
            None => {
                debug!("No token stored");
                Ok(false)
            }
        }
    }

    /// Start the OAuth authorization flow.
    ///
    /// Returns the authorization URL that the user should open in their browser.
    /// The returned state should be preserved and passed to `exchange_code`.
    #[instrument(skip(self))]
    pub fn start_authorization(&mut self) -> Result<(String, OAuthFlowState)> {
        let pkce = Pkce::generate();
        // Use PKCE verifier as state (43 chars = base64url of 32 bytes)
        let state = pkce.verifier.clone();

        let mut url = Url::parse(&self.config.authorize_url)?;
        url.query_pairs_mut()
            .append_pair("code", "true")
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", &self.config.redirect_uri)
            .append_pair("scope", &self.config.scopes.join(" "))
            .append_pair("code_challenge", &pkce.challenge)
            .append_pair("code_challenge_method", pkce.method)
            .append_pair("state", &state);

        let flow_state = OAuthFlowState {
            pkce,
            state: state.clone(),
        };

        // Store the flow state for later use
        self.flow_state = Some(OAuthFlowState {
            pkce: flow_state.pkce.clone(),
            state: flow_state.state.clone(),
        });

        info!(url = %url, "Started OAuth authorization flow");
        Ok((url.to_string(), flow_state))
    }

    /// Exchange an authorization code for tokens.
    ///
    /// # Arguments
    ///
    /// * `code` - The authorization code from the callback
    /// * `state` - The state parameter from the callback (for CSRF verification)
    #[instrument(skip(self, code))]
    pub async fn exchange_code(&mut self, code: &str, state: Option<&str>) -> Result<TokenInfo> {
        let flow_state = self
            .flow_state
            .take()
            .ok_or_else(|| Error::oauth("No active OAuth flow - call start_authorization first"))?;

        // Verify state
        let received_state = state.ok_or_else(|| Error::InvalidState {
            expected: flow_state.state.clone(),
            actual: "missing".to_string(),
        })?;

        if received_state != flow_state.state {
            return Err(Error::InvalidState {
                expected: flow_state.state,
                actual: received_state.to_string(),
            });
        }

        // Token exchange uses JSON body (NOT form-encoded)
        let response = self
            .http_client
            .post(&self.config.token_url)
            .header("Content-Type", "application/json")
            .json(&TokenRequest {
                code,
                grant_type: "authorization_code",
                client_id: &self.config.client_id,
                redirect_uri: &self.config.redirect_uri,
                code_verifier: &flow_state.pkce.verifier,
                state,
            })
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            warn!(status, body = %body, "Token exchange failed");
            return Err(Error::oauth(format!(
                "Token exchange failed ({}): {}",
                status, body
            )));
        }

        let token_response: TokenResponse = response.json().await?;
        let token = TokenInfo::new(
            token_response.access_token,
            token_response.refresh_token,
            token_response.expires_in,
        );

        // Save the token
        self.storage.save("anthropic", &token).await?;
        info!("Token exchange successful, token saved");

        Ok(token)
    }

    /// Refresh an existing token.
    #[instrument(skip(self))]
    pub async fn refresh_token(&self) -> Result<TokenInfo> {
        let current_token = self
            .storage
            .load("anthropic")
            .await?
            .ok_or(Error::NotAuthenticated)?;

        // Token refresh uses JSON body (NOT form-encoded)
        let response = self
            .http_client
            .post(&self.config.token_url)
            .header("Content-Type", "application/json")
            .json(&RefreshRequest {
                grant_type: "refresh_token",
                refresh_token: &current_token.refresh_token,
                client_id: &self.config.client_id,
            })
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            warn!(status, body = %body, "Token refresh failed");
            return Err(Error::RefreshFailed(format!(
                "Token refresh failed ({}): {}",
                status, body
            )));
        }

        let token_response: TokenResponse = response.json().await?;
        let token = TokenInfo::new(
            token_response.access_token,
            token_response.refresh_token,
            token_response.expires_in,
        );

        // Save the new token
        self.storage.save("anthropic", &token).await?;
        info!("Token refresh successful");

        Ok(token)
    }

    /// Get a valid access token, refreshing if necessary.
    #[instrument(skip(self))]
    pub async fn get_access_token(&self) -> Result<String> {
        let token = self
            .storage
            .load("anthropic")
            .await?
            .ok_or(Error::NotAuthenticated)?;

        if token.needs_refresh() {
            debug!("Token needs refresh");
            let refreshed = self.refresh_token().await?;
            Ok(refreshed.access_token)
        } else {
            Ok(token.access_token)
        }
    }

    /// Log out by removing the stored token.
    #[instrument(skip(self))]
    pub async fn logout(&self) -> Result<()> {
        self.storage.remove("anthropic").await?;
        info!("Logged out, token removed");
        Ok(())
    }
}


/// Token request payload (JSON format for Anthropic's OAuth endpoint).
#[derive(Serialize)]
struct TokenRequest<'a> {
    code: &'a str,
    grant_type: &'a str,
    client_id: &'a str,
    redirect_uri: &'a str,
    code_verifier: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<&'a str>,
}

/// Refresh request payload.
#[derive(Serialize)]
struct RefreshRequest<'a> {
    grant_type: &'a str,
    refresh_token: &'a str,
    client_id: &'a str,
}

/// Token response from OAuth endpoint.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    #[serde(rename = "token_type")]
    #[allow(dead_code)]
    token_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_generation() {
        let pkce = Pkce::generate();
        assert!(!pkce.verifier.is_empty());
        assert!(!pkce.challenge.is_empty());
        assert_eq!(pkce.method, "S256");

        // Verify that the challenge matches the verifier
        assert!(Pkce::verify(&pkce.verifier, &pkce.challenge));
    }

    #[test]
    fn test_pkce_verification() {
        let pkce = Pkce::generate();
        assert!(Pkce::verify(&pkce.verifier, &pkce.challenge));

        // Wrong verifier should fail
        assert!(!Pkce::verify("wrong_verifier", &pkce.challenge));
    }

    #[test]
    fn test_oauth_config_default() {
        let config = OAuthConfig::default();
        assert_eq!(config.client_id, DEFAULT_CLIENT_ID);
        assert_eq!(config.authorize_url, DEFAULT_AUTHORIZE_URL);
        assert_eq!(config.token_url, DEFAULT_TOKEN_URL);
    }

    #[tokio::test]
    async fn test_start_authorization() {
        use crate::gate::storage::MemoryTokenStorage;

        let storage = MemoryTokenStorage::new();
        let mut flow = OAuthFlow::new(storage);

        let (url, state) = flow.start_authorization().unwrap();

        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id="));
        assert!(url.contains("code_challenge="));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(!state.state.is_empty());
        assert!(!state.pkce.verifier.is_empty());
    }

    #[test]
    fn test_state_is_pkce_verifier() {
        use crate::gate::storage::MemoryTokenStorage;

        let storage = MemoryTokenStorage::new();
        let mut flow = OAuthFlow::new(storage);

        let (_url, state) = flow.start_authorization().unwrap();

        // State should be the PKCE verifier (43 chars = base64url of 32 bytes)
        assert_eq!(state.state, state.pkce.verifier);
        assert_eq!(state.state.len(), 43);
    }
}
