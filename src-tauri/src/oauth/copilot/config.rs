//! Configuration for the Copilot client.
//!
//! This module provides configuration options for the Copilot API client,
//! including timeout settings, base URLs, and VS Code version simulation.

use std::time::Duration;

use crate::oauth::copilot::auth::constants::{
    COPILOT_TOKEN_URL, DEFAULT_EDITOR_VERSION, GITHUB_ACCESS_TOKEN_URL, GITHUB_DEVICE_CODE_URL,
};

/// Configuration for the Copilot client.
#[derive(Debug, Clone)]
pub struct CopilotConfig {
    /// Timeout for API requests.
    pub request_timeout: Duration,

    /// Timeout for connecting to the API.
    pub connect_timeout: Duration,

    /// Base URL for Copilot API.
    pub api_base_url: String,

    /// URL for device code flow initiation.
    pub device_code_url: String,

    /// URL for exchanging device code for access token.
    pub access_token_url: String,

    /// URL for exchanging GitHub token for Copilot token.
    pub copilot_token_url: String,

    /// VS Code version to report in headers.
    pub vs_code_version: String,

    /// Whether to automatically refresh tokens.
    pub auto_refresh: bool,
}

impl Default for CopilotConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(60),
            connect_timeout: Duration::from_secs(10),
            api_base_url: "https://api.githubcopilot.com".to_string(),
            device_code_url: GITHUB_DEVICE_CODE_URL.to_string(),
            access_token_url: GITHUB_ACCESS_TOKEN_URL.to_string(),
            copilot_token_url: COPILOT_TOKEN_URL.to_string(),
            vs_code_version: DEFAULT_EDITOR_VERSION.to_string(),
            auto_refresh: true,
        }
    }
}

impl CopilotConfig {
    /// Creates a new configuration with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the request timeout.
    #[must_use]
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Sets the connect timeout.
    #[must_use]
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Sets the API base URL.
    #[must_use]
    pub fn with_api_base_url(mut self, url: impl Into<String>) -> Self {
        self.api_base_url = url.into();
        self
    }

    /// Sets the VS Code version to report.
    #[must_use]
    pub fn with_vs_code_version(mut self, version: impl Into<String>) -> Self {
        self.vs_code_version = version.into();
        self
    }

    /// Sets a custom Copilot token URL for testing.
    #[must_use]
    pub fn with_copilot_token_url(mut self, url: impl Into<String>) -> Self {
        self.copilot_token_url = url.into();
        self
    }

    /// Sets whether to automatically refresh tokens.
    #[must_use]
    pub fn with_auto_refresh(mut self, enabled: bool) -> Self {
        self.auto_refresh = enabled;
        self
    }

    /// Returns the full URL for an API endpoint.
    #[must_use]
    pub fn endpoint(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        format!("{}/{}", self.api_base_url.trim_end_matches('/'), path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CopilotConfig::default();
        assert_eq!(config.request_timeout, Duration::from_secs(60));
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert!(config.api_base_url.contains("githubcopilot.com"));
        assert!(config.auto_refresh);
    }

    #[test]
    fn test_builder_methods() {
        let config = CopilotConfig::new()
            .with_request_timeout(Duration::from_secs(30))
            .with_connect_timeout(Duration::from_secs(5))
            .with_api_base_url("https://custom.api.com")
            .with_vs_code_version("vscode/1.80.0")
            .with_auto_refresh(false);

        assert_eq!(config.request_timeout, Duration::from_secs(30));
        assert_eq!(config.connect_timeout, Duration::from_secs(5));
        assert_eq!(config.api_base_url, "https://custom.api.com");
        assert_eq!(config.vs_code_version, "vscode/1.80.0");
        assert!(!config.auto_refresh);
    }

    #[test]
    fn test_endpoint() {
        let config = CopilotConfig::new().with_api_base_url("https://api.example.com");

        assert_eq!(
            config.endpoint("/chat/completions"),
            "https://api.example.com/chat/completions"
        );

        assert_eq!(
            config.endpoint("models"),
            "https://api.example.com/models"
        );
    }

    #[test]
    fn test_endpoint_trailing_slash() {
        let config = CopilotConfig::new().with_api_base_url("https://api.example.com/");

        assert_eq!(
            config.endpoint("/chat/completions"),
            "https://api.example.com/chat/completions"
        );
    }
}
