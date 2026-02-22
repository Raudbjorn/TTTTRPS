//! Copilot token exchange functionality.
//!
//! This module handles exchanging a GitHub OAuth access token for a
//! Copilot API token. The Copilot token is short-lived and needs to
//! be refreshed periodically.

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use tracing::{debug, info, warn};

use crate::oauth::copilot::auth::constants::{API_VERSION, COPILOT_TOKEN_URL, EDITOR_PLUGIN_VERSION};
use crate::oauth::copilot::auth::device_flow::mask_token;
use crate::oauth::copilot::error::{Error, Result};
use crate::oauth::copilot::models::auth::CopilotTokenResponse;

/// Configuration for token exchange.
#[derive(Debug, Clone)]
pub struct TokenExchangeConfig {
    /// URL for the Copilot token endpoint.
    pub token_url: String,
    /// VS Code version to report.
    pub vs_code_version: String,
}

impl Default for TokenExchangeConfig {
    fn default() -> Self {
        Self {
            token_url: COPILOT_TOKEN_URL.to_string(),
            vs_code_version: "vscode/1.96.2".to_string(),
        }
    }
}

impl TokenExchangeConfig {
    /// Sets a custom VS Code version to report.
    #[must_use]
    pub fn with_vs_code_version(mut self, version: impl Into<String>) -> Self {
        self.vs_code_version = version.into();
        self
    }

    /// Sets a custom token URL for testing.
    #[must_use]
    pub fn with_token_url(mut self, url: impl Into<String>) -> Self {
        self.token_url = url.into();
        self
    }
}

/// Exchanges a GitHub access token for a Copilot API token.
///
/// The Copilot token is required to access the Copilot API endpoints.
/// It is derived from the GitHub OAuth token and has a shorter lifetime
/// (typically ~30 minutes).
///
/// # Arguments
///
/// * `client` - HTTP client for making the request
/// * `github_token` - The GitHub OAuth access token
/// * `config` - Configuration for the exchange
///
/// # Returns
///
/// A `CopilotTokenResponse` containing the Copilot token and expiration time.
///
/// # Errors
///
/// Returns an error if:
/// - The GitHub token is invalid or doesn't have Copilot access
/// - Network error occurs
/// - Response cannot be parsed
///
/// # Example
///
/// ```no_run
/// use crate::oauth::copilot::auth::token_exchange::{exchange_for_copilot_token, TokenExchangeConfig};
///
/// # async fn example(github_token: &str) -> crate::oauth::copilot::Result<()> {
/// let client = reqwest::Client::new();
/// let config = TokenExchangeConfig::default();
///
/// let copilot_token = exchange_for_copilot_token(&client, github_token, &config).await?;
/// println!("Token expires at: {}", copilot_token.expires_at);
/// # Ok(())
/// # }
/// ```
pub async fn exchange_for_copilot_token(
    client: &reqwest::Client,
    github_token: &str,
    config: &TokenExchangeConfig,
) -> Result<CopilotTokenResponse> {
    debug!(
        token_preview = %mask_token(github_token),
        "Exchanging GitHub token for Copilot token"
    );

    let headers = build_exchange_headers(github_token, &config.vs_code_version)?;

    let response = client
        .get(&config.token_url)
        .headers(headers)
        .send()
        .await?;

    let status = response.status();

    if status == reqwest::StatusCode::UNAUTHORIZED {
        warn!("GitHub token is invalid or expired");
        return Err(Error::NotAuthenticated);
    }

    if status == reqwest::StatusCode::FORBIDDEN {
        let body = response.text().await.unwrap_or_default();
        warn!(status = %status, body = %body, "No Copilot access");
        return Err(Error::Api {
            status: 403,
            message: "GitHub account does not have Copilot access".to_string(),
        });
    }

    if !status.is_success() {
        let message = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(Error::Api {
            status: status.as_u16(),
            message,
        });
    }

    let token_response: CopilotTokenResponse = response.json().await?;

    info!(
        expires_at = token_response.expires_at,
        refresh_in = token_response.refresh_in,
        token_preview = %mask_token(&token_response.token),
        "Copilot token obtained successfully"
    );

    Ok(token_response)
}

/// Builds the headers required for the token exchange request.
fn build_exchange_headers(github_token: &str, vs_code_version: &str) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();

    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("token {github_token}"))
            .map_err(|e| Error::Config(format!("Invalid token header: {e}")))?,
    );

    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(crate::oauth::copilot::auth::constants::USER_AGENT),
    );

    headers.insert(
        "editor-version",
        HeaderValue::from_str(vs_code_version)
            .map_err(|e| Error::Config(format!("Invalid editor version: {e}")))?,
    );

    headers.insert(
        "editor-plugin-version",
        HeaderValue::from_static(EDITOR_PLUGIN_VERSION),
    );

    headers.insert("x-github-api-version", HeaderValue::from_static(API_VERSION));

    Ok(headers)
}

/// Validates that a GitHub token looks correct.
///
/// GitHub tokens typically start with `gho_` (OAuth), `ghp_` (personal access),
/// or `ghu_` (user-to-server).
#[must_use]
pub fn is_valid_github_token_format(token: &str) -> bool {
    if token.len() < 10 {
        return false;
    }

    // Check for known prefixes
    let known_prefixes = ["gho_", "ghp_", "ghu_", "github_pat_"];
    known_prefixes.iter().any(|p| token.starts_with(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_exchange_config_default() {
        let config = TokenExchangeConfig::default();
        assert!(config.token_url.contains("copilot_internal"));
        assert!(config.vs_code_version.contains("vscode"));
    }

    #[test]
    fn test_token_exchange_config_custom() {
        let config = TokenExchangeConfig::default().with_vs_code_version("vscode/1.80.0");
        assert_eq!(config.vs_code_version, "vscode/1.80.0");
    }

    #[test]
    fn test_is_valid_github_token_format() {
        assert!(is_valid_github_token_format("gho_abc123456789"));
        assert!(is_valid_github_token_format("ghp_xyz987654321"));
        assert!(is_valid_github_token_format("ghu_test12345678"));
        assert!(is_valid_github_token_format(
            "github_pat_11ABCD_somethinglong"
        ));

        assert!(!is_valid_github_token_format("short"));
        assert!(!is_valid_github_token_format("invalid_token_format"));
        assert!(!is_valid_github_token_format(""));
    }

    #[test]
    fn test_build_exchange_headers() {
        let headers =
            build_exchange_headers("gho_test123", "vscode/1.96.2").expect("headers should build");

        assert!(headers.contains_key(AUTHORIZATION));
        assert!(headers.contains_key(USER_AGENT));
        assert!(headers.contains_key("editor-version"));
        assert!(headers.contains_key("x-github-api-version"));

        let auth = headers.get(AUTHORIZATION).unwrap().to_str().unwrap();
        assert!(auth.starts_with("token "));
    }
}
