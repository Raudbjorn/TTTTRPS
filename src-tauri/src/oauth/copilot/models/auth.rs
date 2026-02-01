//! Authentication-related data models.
//!
//! This module contains the data structures for the OAuth device code flow
//! and token management.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::oauth::copilot::auth::refresh::REFRESH_BUFFER_SECS;

// =============================================================================
// Device Code Flow Types
// =============================================================================

/// Response from GitHub's device code endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCodeResponse {
    /// The device verification code (internal).
    pub device_code: String,
    /// The user-facing code to display.
    pub user_code: String,
    /// URL where the user should enter the code.
    pub verification_uri: String,
    /// Seconds until the device code expires.
    pub expires_in: u64,
    /// Minimum polling interval in seconds.
    #[serde(default = "default_interval")]
    pub interval: u64,
}

fn default_interval() -> u64 {
    5
}

/// Response from GitHub's token endpoint after device flow completion.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubTokenResponse {
    /// The OAuth access token.
    pub access_token: String,
    /// Token type (usually "bearer").
    pub token_type: String,
    /// OAuth scopes granted.
    pub scope: String,
}

// =============================================================================
// Copilot Token Types
// =============================================================================

/// Response from the Copilot token exchange endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotTokenResponse {
    /// The Copilot API token.
    pub token: String,

    /// Unix timestamp when the token expires.
    pub expires_at: i64,

    /// Recommended refresh interval in seconds.
    #[serde(default)]
    pub refresh_in: u64,

    /// Additional token annotations (e.g., SKU info).
    #[serde(default)]
    pub annotations_enabled: bool,

    /// Chat-enabled flag.
    #[serde(default)]
    pub chat_enabled: bool,

    /// Organization ID if using org-level access.
    #[serde(default)]
    pub organization_id: Option<String>,

    /// Enterprise ID if using enterprise-level access.
    #[serde(default)]
    pub enterprise_id: Option<String>,

    /// SKU (product tier).
    #[serde(default)]
    pub sku: Option<String>,

    /// Telemetry ID.
    #[serde(default)]
    pub telemetry: Option<String>,

    /// Tracking ID for this token issuance.
    #[serde(default)]
    pub tracking_id: Option<String>,
}

impl CopilotTokenResponse {
    /// Returns the time remaining until the token expires.
    #[must_use]
    pub fn expires_in(&self) -> std::time::Duration {
        let now = Utc::now().timestamp();
        let remaining = (self.expires_at - now).max(0);
        std::time::Duration::from_secs(remaining as u64)
    }

    /// Returns true if the token has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() >= self.expires_at
    }

    /// Returns true if the token should be refreshed (within buffer period).
    #[must_use]
    pub fn should_refresh(&self) -> bool {
        let now = Utc::now().timestamp();
        now >= self.expires_at - REFRESH_BUFFER_SECS
    }
}

// =============================================================================
// Token Storage Types
// =============================================================================

/// Complete token information for storage.
///
/// This struct contains both the long-lived GitHub token and the
/// short-lived Copilot token, along with expiration information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenInfo {
    /// Type identifier (always "github" for Copilot auth).
    #[serde(default = "default_token_type")]
    pub token_type: String,

    /// The GitHub OAuth access token (long-lived).
    pub github_token: String,

    /// The Copilot API token (short-lived, may be None).
    #[serde(default)]
    pub copilot_token: Option<String>,

    /// Unix timestamp when the Copilot token expires.
    #[serde(default)]
    pub copilot_expires_at: Option<i64>,
}

fn default_token_type() -> String {
    "github".to_string()
}

impl TokenInfo {
    /// Creates a new TokenInfo with just the GitHub token.
    #[must_use]
    pub fn new(github_token: impl Into<String>) -> Self {
        Self {
            token_type: "github".to_string(),
            github_token: github_token.into(),
            copilot_token: None,
            copilot_expires_at: None,
        }
    }

    /// Creates a TokenInfo with both GitHub and Copilot tokens.
    #[must_use]
    pub fn with_copilot(
        github_token: impl Into<String>,
        copilot_token: impl Into<String>,
        expires_at: i64,
    ) -> Self {
        Self {
            token_type: "github".to_string(),
            github_token: github_token.into(),
            copilot_token: Some(copilot_token.into()),
            copilot_expires_at: Some(expires_at),
        }
    }

    /// Returns true if there is no valid Copilot token.
    #[must_use]
    pub fn needs_copilot_refresh(&self) -> bool {
        match (&self.copilot_token, self.copilot_expires_at) {
            (None, _) => true,
            (Some(_), None) => true,
            (Some(_), Some(expires_at)) => {
                let now = Utc::now().timestamp();
                now >= expires_at - REFRESH_BUFFER_SECS
            }
        }
    }

    /// Returns the Copilot token if available and valid.
    #[must_use]
    pub fn valid_copilot_token(&self) -> Option<&str> {
        if self.needs_copilot_refresh() {
            None
        } else {
            self.copilot_token.as_deref()
        }
    }

    /// Updates the Copilot token information.
    pub fn update_copilot_token(&mut self, token: impl Into<String>, expires_at: i64) {
        self.copilot_token = Some(token.into());
        self.copilot_expires_at = Some(expires_at);
    }

    /// Returns true if this token info has a GitHub token.
    #[must_use]
    pub fn has_github_token(&self) -> bool {
        !self.github_token.is_empty()
    }

    /// Clears all token information.
    pub fn clear(&mut self) {
        self.github_token.clear();
        self.copilot_token = None;
        self.copilot_expires_at = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_code_response_deserialize() {
        let json = r#"{
            "device_code": "dc_12345",
            "user_code": "ABCD-1234",
            "verification_uri": "https://github.com/login/device",
            "expires_in": 900,
            "interval": 5
        }"#;

        let response: DeviceCodeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.device_code, "dc_12345");
        assert_eq!(response.user_code, "ABCD-1234");
        assert_eq!(response.expires_in, 900);
        assert_eq!(response.interval, 5);
    }

    #[test]
    fn test_device_code_response_default_interval() {
        let json = r#"{
            "device_code": "dc_12345",
            "user_code": "ABCD-1234",
            "verification_uri": "https://github.com/login/device",
            "expires_in": 900
        }"#;

        let response: DeviceCodeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.interval, 5); // Default value
    }

    #[test]
    fn test_github_token_response_deserialize() {
        let json = r#"{
            "access_token": "gho_test123",
            "token_type": "bearer",
            "scope": "read:user"
        }"#;

        let response: GitHubTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.access_token, "gho_test123");
        assert_eq!(response.token_type, "bearer");
        assert_eq!(response.scope, "read:user");
    }

    #[test]
    fn test_copilot_token_response_deserialize() {
        let json = r#"{
            "token": "tid=test;exp=12345;sku=pro;st=dotcom;ssc=1",
            "expires_at": 1700000000,
            "refresh_in": 1500,
            "chat_enabled": true,
            "sku": "pro"
        }"#;

        let response: CopilotTokenResponse = serde_json::from_str(json).unwrap();
        assert!(response.token.contains("tid=test"));
        assert_eq!(response.expires_at, 1700000000);
        assert_eq!(response.refresh_in, 1500);
        assert!(response.chat_enabled);
        assert_eq!(response.sku, Some("pro".to_string()));
    }

    #[test]
    fn test_copilot_token_response_minimal() {
        let json = r#"{
            "token": "test_token",
            "expires_at": 1700000000
        }"#;

        let response: CopilotTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.token, "test_token");
        assert_eq!(response.refresh_in, 0);
        assert!(!response.chat_enabled);
        assert!(response.sku.is_none());
    }

    #[test]
    fn test_token_info_new() {
        let info = TokenInfo::new("gho_test");
        assert_eq!(info.github_token, "gho_test");
        assert_eq!(info.token_type, "github");
        assert!(info.copilot_token.is_none());
        assert!(info.copilot_expires_at.is_none());
    }

    #[test]
    fn test_token_info_with_copilot() {
        let info = TokenInfo::with_copilot("gho_test", "cop_test", 1700000000);
        assert_eq!(info.github_token, "gho_test");
        assert_eq!(info.copilot_token.as_deref(), Some("cop_test"));
        assert_eq!(info.copilot_expires_at, Some(1700000000));
    }

    #[test]
    fn test_token_info_needs_refresh() {
        // No Copilot token
        let info = TokenInfo::new("gho_test");
        assert!(info.needs_copilot_refresh());

        // Expired token
        let info = TokenInfo::with_copilot("gho_test", "cop_test", 0);
        assert!(info.needs_copilot_refresh());

        // Valid token (far in future)
        let info = TokenInfo::with_copilot("gho_test", "cop_test", Utc::now().timestamp() + 3600);
        assert!(!info.needs_copilot_refresh());
    }

    #[test]
    fn test_token_info_update() {
        let mut info = TokenInfo::new("gho_test");
        info.update_copilot_token("cop_new", 1700000000);

        assert_eq!(info.copilot_token.as_deref(), Some("cop_new"));
        assert_eq!(info.copilot_expires_at, Some(1700000000));
    }

    #[test]
    fn test_token_info_clear() {
        let mut info = TokenInfo::with_copilot("gho_test", "cop_test", 1700000000);
        info.clear();

        assert!(info.github_token.is_empty());
        assert!(info.copilot_token.is_none());
        assert!(info.copilot_expires_at.is_none());
    }

    #[test]
    fn test_token_info_serialization() {
        let info = TokenInfo::with_copilot("gho_test", "cop_test", 1700000000);
        let json = serde_json::to_string(&info).unwrap();

        let restored: TokenInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.github_token, "gho_test");
        assert_eq!(restored.copilot_token.as_deref(), Some("cop_test"));
        assert_eq!(restored.copilot_expires_at, Some(1700000000));
    }
}
