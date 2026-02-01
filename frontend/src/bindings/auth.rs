use serde::{Deserialize, Serialize};
use super::core::{invoke, invoke_void, invoke_no_args};

// ============================================================================
// Claude Gate OAuth Commands
// ============================================================================

/// Storage backend options for Claude tokens
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ClaudeStorageBackend {
    /// Auto-select best available (keyring if available, else file)
    #[default]
    Auto,
    /// System keyring (GNOME Keyring, macOS Keychain, Windows Credential Manager)
    Keyring,
    /// File-based storage (~/.local/share/ttrpg-assistant/oauth-tokens.json)
    File,
}

impl std::fmt::Display for ClaudeStorageBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClaudeStorageBackend::Auto => write!(f, "Auto"),
            ClaudeStorageBackend::Keyring => write!(f, "Keyring"),
            ClaudeStorageBackend::File => write!(f, "File"),
        }
    }
}

/// Status of Claude OAuth authentication
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClaudeStatus {
    /// Whether the user is authenticated
    pub authenticated: bool,
    /// Storage backend being used
    pub storage_backend: String,
    /// Token expiration timestamp (Unix seconds)
    pub token_expires_at: Option<i64>,
    /// Human-readable time until expiry
    pub expiration_display: Option<String>,
    /// Error message if any
    pub error: Option<String>,
    /// Whether keyring (secret service) is available on this system
    #[serde(default)]
    pub keyring_available: bool,
}

/// Get Claude Gate OAuth status
pub async fn claude_get_status() -> Result<ClaudeStatus, String> {
    invoke_no_args("claude_get_status").await
}

/// Response from starting OAuth flow
#[derive(Debug, Clone, Deserialize)]
pub struct ClaudeOAuthStartResponse {
    /// URL to open in user's browser for OAuth authorization
    pub auth_url: String,
    /// State parameter for CSRF protection (pass back to complete_oauth)
    pub state: String,
}

/// Start OAuth flow - returns the authorization URL and CSRF state
pub async fn claude_start_oauth() -> Result<ClaudeOAuthStartResponse, String> {
    invoke_no_args("claude_start_oauth").await
}

/// Response from completing OAuth flow
#[derive(Debug, Clone, Deserialize)]
pub struct ClaudeOAuthCompleteResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Complete OAuth flow with authorization code
pub async fn claude_complete_oauth(code: String, oauth_state: Option<String>) -> Result<ClaudeOAuthCompleteResponse, String> {
    #[derive(Serialize)]
    struct Args {
        code: String,
        oauth_state: Option<String>,
    }
    invoke("claude_complete_oauth", &Args { code, oauth_state }).await
}

/// Logout from Claude (remove stored token)
pub async fn claude_logout() -> Result<(), String> {
    invoke_void("claude_logout", &()).await
}

/// Set storage backend for Claude tokens
pub async fn claude_set_storage_backend(backend: ClaudeStorageBackend) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        backend: ClaudeStorageBackend,
    }
    invoke_void("claude_set_storage_backend", &Args { backend }).await
}

/// Model info from Claude API
#[derive(Debug, Clone, Deserialize)]
pub struct ClaudeModelInfo {
    pub id: String,
    pub name: String,
}

/// List available models from Claude Gate API
///
/// Requires authentication. Returns list of models the user can access.
pub async fn claude_list_models() -> Result<Vec<ClaudeModelInfo>, String> {
    invoke_no_args("claude_list_models").await
}

// ============================================================================
// Copilot Gate OAuth Commands (GitHub Copilot Device Code Flow)
// ============================================================================

/// Response from starting Copilot Device Code authentication flow
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotDeviceCodeResponse {
    /// The device verification code (internal, for polling)
    pub device_code: String,
    /// The user-facing code to enter at the verification URL
    pub user_code: String,
    /// URL where the user should enter the code
    pub verification_uri: String,
    /// Seconds until the device code expires
    pub expires_in: u64,
    /// Minimum seconds between polling attempts
    pub interval: u64,
}

/// Start Copilot authentication using Device Code flow
///
/// Returns device code information. The user should visit verification_uri
/// and enter the user_code to authorize the application.
pub async fn start_copilot_auth() -> Result<CopilotDeviceCodeResponse, String> {
    invoke_no_args("start_copilot_auth").await
}

/// Result of polling for Copilot authorization
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotAuthPollResult {
    /// Status: "success", "pending", "slow_down", "expired", "denied", or "error"
    pub status: String,
    /// Whether authentication is complete
    pub authenticated: bool,
    /// Error message if status is "expired", "denied", or "error"
    pub error: Option<String>,
}

/// Poll for Copilot authorization completion
///
/// Should be called repeatedly with appropriate delays (as specified by interval)
/// until status is "success", "expired", or "denied".
pub async fn poll_copilot_auth(device_code: String) -> Result<CopilotAuthPollResult, String> {
    #[derive(Serialize)]
    struct Args {
        device_code: String,
    }
    invoke("poll_copilot_auth", &Args { device_code }).await
}

/// Storage backend options for Copilot tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CopilotStorageBackend {
    /// File-based storage
    File,
    /// System keyring storage
    Keyring,
    /// Auto-select (keyring if available, else file)
    #[default]
    Auto,
}

impl std::fmt::Display for CopilotStorageBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CopilotStorageBackend::Auto => write!(f, "Auto"),
            CopilotStorageBackend::Keyring => write!(f, "Keyring"),
            CopilotStorageBackend::File => write!(f, "File"),
        }
    }
}

/// Status of Copilot authentication
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CopilotAuthStatus {
    /// Whether the user is authenticated with valid tokens
    pub authenticated: bool,
    /// Current storage backend being used
    pub storage_backend: String,
    /// Unix timestamp when the Copilot token expires (short-lived)
    pub copilot_token_expires_at: Option<i64>,
    /// Whether the user has a valid GitHub token (long-lived)
    pub has_github_token: bool,
    /// Whether keyring (secret service) is available on this system
    #[serde(default)]
    pub keyring_available: bool,
}

/// Check current Copilot authentication status
pub async fn check_copilot_auth() -> Result<CopilotAuthStatus, String> {
    invoke_no_args("check_copilot_auth").await
}

/// Set storage backend for Copilot tokens
pub async fn copilot_set_storage_backend(backend: CopilotStorageBackend) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        backend: CopilotStorageBackend,
    }
    invoke_void("copilot_set_storage_backend", &Args { backend }).await
}

/// Logout from Copilot (remove stored tokens)
pub async fn logout_copilot() -> Result<(), String> {
    invoke_void("logout_copilot", &()).await
}

/// Detail about a single quota category
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotQuotaDetail {
    /// Number of units used in the current period
    pub used: u64,
    /// Maximum units allowed in the current period
    pub limit: u64,
    /// Whether the quota is unlimited
    pub unlimited: bool,
    /// Remaining quota
    pub remaining: u64,
    /// Whether the quota is exhausted
    pub is_exhausted: bool,
}

/// Usage information for Copilot quotas
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotUsageInfo {
    /// The user's Copilot plan type
    pub copilot_plan: String,
    /// The date when quotas reset (ISO 8601 format)
    pub quota_reset_date: String,
    /// Chat completions quota info
    pub completions: Option<CopilotQuotaDetail>,
    /// Premium requests quota info
    pub premium_requests: Option<CopilotQuotaDetail>,
    /// Code completions quota info
    pub code_completions: Option<CopilotQuotaDetail>,
}

/// Get Copilot usage and quota information
///
/// Requires authentication. Returns current usage against quotas.
pub async fn get_copilot_usage() -> Result<CopilotUsageInfo, String> {
    invoke_no_args("get_copilot_usage").await
}

/// Model information from Copilot API
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotModelInfo {
    /// The model ID (e.g., "gpt-4o", "claude-3.5-sonnet")
    pub id: String,
    /// Owner organization
    pub owned_by: String,
    /// Unix timestamp when created
    pub created: i64,
    /// Whether the model supports chat completions
    pub supports_chat: bool,
    /// Whether the model supports tool calls
    pub supports_tools: bool,
    /// Whether the model supports vision
    pub supports_vision: bool,
    /// Maximum context window in tokens
    pub max_context_tokens: Option<u64>,
    /// Maximum output tokens
    pub max_output_tokens: Option<u64>,
    /// Whether the model is in preview
    pub preview: bool,
}

/// List available models from Copilot API
///
/// Requires authentication. Returns list of models the user can access.
pub async fn get_copilot_models() -> Result<Vec<CopilotModelInfo>, String> {
    invoke_no_args("get_copilot_models").await
}

// ============================================================================
// Gemini OAuth Commands
// ============================================================================

/// Storage backend options for Gemini tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GeminiStorageBackend {
    /// File-based storage
    File,
    /// System keyring storage
    Keyring,
    /// Auto-select (keyring if available, else file)
    #[default]
    Auto,
}

impl std::fmt::Display for GeminiStorageBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeminiStorageBackend::Auto => write!(f, "Auto"),
            GeminiStorageBackend::Keyring => write!(f, "Keyring"),
            GeminiStorageBackend::File => write!(f, "File"),
        }
    }
}

/// Status of Gemini OAuth authentication
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeminiStatus {
    /// Whether the user is authenticated
    pub authenticated: bool,
    /// Storage backend being used
    pub storage_backend: String,
    /// Token expiration timestamp (Unix seconds)
    pub token_expires_at: Option<i64>,
    /// Whether keyring (secret service) is available on this system
    #[serde(default)]
    pub keyring_available: bool,
}

/// Get Gemini OAuth status
pub async fn gemini_get_status() -> Result<GeminiStatus, String> {
    invoke_no_args("gemini_get_status").await
}

/// Response from starting Gemini OAuth flow
#[derive(Debug, Clone, Deserialize)]
pub struct GeminiOAuthStartResponse {
    /// URL to open in user's browser for OAuth authorization
    pub auth_url: String,
    /// State parameter for CSRF protection (pass back to complete_oauth)
    pub state: String,
}

/// Start Gemini OAuth flow - returns the authorization URL and CSRF state
pub async fn gemini_start_oauth() -> Result<GeminiOAuthStartResponse, String> {
    invoke_no_args("gemini_start_oauth").await
}

/// Response from completing Gemini OAuth flow
#[derive(Debug, Clone, Deserialize)]
pub struct GeminiOAuthCompleteResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// Complete Gemini OAuth flow with authorization code
pub async fn gemini_complete_oauth(code: String, oauth_state: Option<String>) -> Result<GeminiOAuthCompleteResponse, String> {
    #[derive(Serialize)]
    struct Args {
        code: String,
        oauth_state: Option<String>,
    }
    invoke("gemini_complete_oauth", &Args { code, oauth_state }).await
}

/// Logout from Gemini (remove stored token)
pub async fn gemini_logout() -> Result<(), String> {
    invoke_void("gemini_logout", &()).await
}

/// Set storage backend for Gemini tokens
pub async fn gemini_set_storage_backend(backend: GeminiStorageBackend) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        backend: GeminiStorageBackend,
    }
    invoke_void("gemini_set_storage_backend", &Args { backend }).await
}

/// Response from Gemini OAuth flow with automatic callback server
#[derive(Debug, Clone, Deserialize)]
pub struct GeminiOAuthCallbackResponse {
    /// Whether the OAuth flow completed successfully
    pub success: bool,
    /// Error message if the flow failed
    pub error: Option<String>,
    /// The authorization URL (for display/manual fallback)
    pub auth_url: Option<String>,
}

/// Start Gemini OAuth flow with automatic callback server
///
/// This command:
/// 1. Starts a local HTTP server to receive the OAuth callback
/// 2. Opens the authorization URL in the user's browser
/// 3. Waits for the OAuth callback
/// 4. Completes the OAuth flow automatically
///
/// Returns the result of the OAuth flow, including any errors.
pub async fn gemini_oauth_with_callback(
    timeout_secs: Option<u64>,
    open_browser: Option<bool>,
) -> Result<GeminiOAuthCallbackResponse, String> {
    #[derive(Serialize)]
    struct Args {
        timeout_secs: Option<u64>,
        open_browser: Option<bool>,
    }
    invoke("gemini_oauth_with_callback", &Args { timeout_secs, open_browser }).await
}

/// A model available via the Gemini Cloud Code API.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeminiModel {
    /// Unique identifier for the model.
    pub id: String,
    /// Display name for the model.
    pub name: String,
    /// Description of the model (optional).
    pub description: Option<String>,
}

/// List available models from the Gemini Cloud Code API.
///
/// Returns models available for use with the authenticated account.
/// Requires successful OAuth authentication first.
pub async fn gemini_list_models() -> Result<Vec<GeminiModel>, String> {
    invoke_no_args("gemini_list_models").await
}
