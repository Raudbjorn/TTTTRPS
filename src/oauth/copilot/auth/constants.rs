//! API constants for GitHub and Copilot authentication.
//!
//! These constants define the endpoints and identifiers used in the
//! OAuth device code flow and Copilot token exchange.

// =============================================================================
// GitHub OAuth Constants
// =============================================================================

/// GitHub OAuth Client ID for VS Code Copilot extension.
///
/// This is the public client ID used by the official VS Code Copilot extension.
pub const GITHUB_CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";

/// OAuth scopes requested during device code flow.
pub const GITHUB_OAUTH_SCOPE: &str = "read:user";

/// Grant type for device code token exchange (RFC 8628).
pub const DEVICE_CODE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

// =============================================================================
// GitHub API Endpoints
// =============================================================================

/// Base URL for GitHub web endpoints.
pub const GITHUB_BASE_URL: &str = "https://github.com";

/// Endpoint to initiate device code flow.
pub const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";

/// Endpoint to exchange device code for access token.
pub const GITHUB_ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

/// Base URL for GitHub API.
pub const GITHUB_API_BASE_URL: &str = "https://api.github.com";

/// Endpoint to exchange GitHub token for Copilot token.
pub const COPILOT_TOKEN_URL: &str = "https://api.github.com/copilot_internal/v2/token";

// =============================================================================
// Polling Constants
// =============================================================================

/// Default polling interval in seconds (per RFC 8628, minimum is 5).
pub const DEFAULT_POLL_INTERVAL_SECS: u64 = 5;

/// Additional buffer to add to polling interval to avoid rate limiting.
pub const POLL_INTERVAL_BUFFER_SECS: u64 = 1;

/// Maximum number of poll attempts before giving up.
/// With 5-second intervals, this allows for ~15 minutes of polling.
pub const MAX_POLL_ATTEMPTS: u32 = 180;

// =============================================================================
// Header Constants (mimicking VS Code Copilot extension)
// =============================================================================

/// Copilot extension version to report.
pub const COPILOT_VERSION: &str = "0.26.7";

/// Editor plugin version header value.
pub const EDITOR_PLUGIN_VERSION: &str = "copilot-chat/0.26.7";

/// User agent header value.
pub const USER_AGENT: &str = "GitHubCopilotChat/0.26.7";

/// GitHub API version header value.
pub const API_VERSION: &str = "2025-04-01";

/// Default VS Code version to report.
pub const DEFAULT_EDITOR_VERSION: &str = "vscode/1.96.2";

// =============================================================================
// GitHub OAuth Error Codes (per RFC 8628)
// =============================================================================

/// Error code: User has not yet authorized, keep polling.
pub const ERROR_AUTHORIZATION_PENDING: &str = "authorization_pending";

/// Error code: Polling too fast, increase interval.
pub const ERROR_SLOW_DOWN: &str = "slow_down";

/// Error code: User denied authorization.
pub const ERROR_ACCESS_DENIED: &str = "access_denied";

/// Error code: Device code has expired.
pub const ERROR_EXPIRED_TOKEN: &str = "expired_token";

// =============================================================================
// Token Refresh Constants
// =============================================================================

/// Seconds before expiry to trigger proactive token refresh.
pub const TOKEN_REFRESH_BUFFER_SECS: i64 = 60;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urls_are_https() {
        assert!(GITHUB_DEVICE_CODE_URL.starts_with("https://"));
        assert!(GITHUB_ACCESS_TOKEN_URL.starts_with("https://"));
        assert!(COPILOT_TOKEN_URL.starts_with("https://"));
    }

    #[test]
    fn test_client_id_format() {
        // VS Code Copilot client ID starts with "Iv1."
        assert!(GITHUB_CLIENT_ID.starts_with("Iv1."));
    }

    #[test]
    fn test_poll_interval_respects_rfc8628() {
        // RFC 8628 specifies minimum 5 second interval
        assert!(DEFAULT_POLL_INTERVAL_SECS >= 5);
    }
}
