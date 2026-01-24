//! Copilot OAuth State and Commands
//!
//! Handles OAuth for the Copilot Gate provider (GitHub Copilot using Device Code Flow).
//! Provides type-erased storage backend support and runtime backend switching.

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::RwLock as AsyncRwLock;

// Copilot Gate OAuth client - for Device Code flow
use crate::gate::copilot::{
    CopilotClient, DeviceFlowPending, GateStorageAdapter as CopilotGateStorageAdapter,
    ModelInfo as CopilotModelInfo, ModelsResponse as CopilotModelsResponse,
    PollResult as CopilotPollResult, QuotaInfo as CopilotQuotaInfo,
    UsageResponse as CopilotUsageResponse,
    storage::CopilotTokenStorage,
};
use crate::gate::storage::FileTokenStorage as GateFileTokenStorage;

// Import AppState - will be available via commands_legacy re-export
use crate::commands::AppState;

// ============================================================================
// Storage Backend Enum
// ============================================================================

/// Storage backend type for Copilot Gate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum CopilotGateStorageBackend {
    /// File-based storage (~/.config/gate/copilot/auth.json)
    File,
    /// System keyring storage (not yet implemented for Copilot)
    Keyring,
    /// Auto-select (file for now, keyring when available)
    #[default]
    Auto,
}


impl std::fmt::Display for CopilotGateStorageBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File => write!(f, "file"),
            Self::Keyring => write!(f, "keyring"),
            Self::Auto => write!(f, "auto"),
        }
    }
}

impl std::str::FromStr for CopilotGateStorageBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "file" => Ok(Self::File),
            "keyring" => Ok(Self::Keyring),
            "auto" => Ok(Self::Auto),
            _ => Err(format!(
                "Unknown storage backend: {}. Valid options: file, keyring, auto",
                s
            )),
        }
    }
}

// ============================================================================
// Copilot Gate Client Trait (for type-erased storage backend support)
// ============================================================================

/// Trait for Copilot Gate client operations, allowing type-erased storage backends.
#[async_trait::async_trait]
trait CopilotGateClientOps: Send + Sync {
    async fn is_authenticated(&self) -> Result<bool, String>;
    async fn get_token_info(
        &self,
    ) -> Result<Option<crate::gate::copilot::models::TokenInfo>, String>;
    async fn start_device_flow(&self) -> Result<DeviceFlowPending, String>;
    async fn poll_for_token(&self, pending: &DeviceFlowPending) -> Result<CopilotPollResult, String>;
    async fn complete_auth(&self, github_token: String) -> Result<(), String>;
    async fn sign_out(&self) -> Result<(), String>;
    async fn get_models(&self) -> Result<CopilotModelsResponse, String>;
    async fn get_usage(&self) -> Result<CopilotUsageResponse, String>;
    fn storage_name(&self) -> &'static str;
}

/// File storage client wrapper for Copilot
struct CopilotFileStorageClientWrapper {
    client: CopilotClient<CopilotGateStorageAdapter<GateFileTokenStorage>>,
}

#[async_trait::async_trait]
impl CopilotGateClientOps for CopilotFileStorageClientWrapper {
    async fn is_authenticated(&self) -> Result<bool, String> {
        Ok(self.client.is_authenticated().await)
    }

    async fn get_token_info(
        &self,
    ) -> Result<Option<crate::gate::copilot::models::TokenInfo>, String> {
        self.client
            .storage()
            .load()
            .await
            .map_err(|e| e.to_string())
    }

    async fn start_device_flow(&self) -> Result<DeviceFlowPending, String> {
        self.client
            .start_device_flow()
            .await
            .map_err(|e| e.to_string())
    }

    async fn poll_for_token(&self, pending: &DeviceFlowPending) -> Result<CopilotPollResult, String> {
        self.client
            .poll_for_token(pending)
            .await
            .map_err(|e| e.to_string())
    }

    async fn complete_auth(&self, github_token: String) -> Result<(), String> {
        self.client
            .complete_auth(github_token)
            .await
            .map_err(|e| e.to_string())
    }

    async fn sign_out(&self) -> Result<(), String> {
        self.client.sign_out().await.map_err(|e| e.to_string())
    }

    async fn get_models(&self) -> Result<CopilotModelsResponse, String> {
        self.client.models().await.map_err(|e| e.to_string())
    }

    async fn get_usage(&self) -> Result<CopilotUsageResponse, String> {
        self.client.usage().await.map_err(|e| e.to_string())
    }

    fn storage_name(&self) -> &'static str {
        "file"
    }
}

// ============================================================================
// Copilot Gate State
// ============================================================================

/// Type-erased Copilot Gate client wrapper.
/// This allows storing the client in AppState regardless of storage backend.
pub struct CopilotGateState {
    /// The active client (type-erased)
    client: AsyncRwLock<Option<Box<dyn CopilotGateClientOps>>>,
    /// In-memory pending device flow state
    pending_device_flow: AsyncRwLock<Option<DeviceFlowPending>>,
    /// Current storage backend
    storage_backend: AsyncRwLock<CopilotGateStorageBackend>,
}

impl CopilotGateState {
    /// Create a client for the specified backend
    fn create_client(
        backend: CopilotGateStorageBackend,
    ) -> Result<Box<dyn CopilotGateClientOps>, String> {
        match backend {
            CopilotGateStorageBackend::File | CopilotGateStorageBackend::Auto => {
                // Create file-based storage for Copilot tokens
                let storage_path = dirs::config_dir()
                    .ok_or("Could not determine config directory")?
                    .join("gate")
                    .join("copilot");

                // Ensure the directory exists
                std::fs::create_dir_all(&storage_path)
                    .map_err(|e| format!("Failed to create storage directory: {}", e))?;

                let file_storage = GateFileTokenStorage::new(storage_path.join("auth.json"))
                    .map_err(|e| format!("Failed to create file storage: {}", e))?;
                let adapter = CopilotGateStorageAdapter::new(file_storage);

                let client = CopilotClient::builder()
                    .with_storage(adapter)
                    .build()
                    .map_err(|e| format!("Failed to create Copilot client: {}", e))?;

                Ok(Box::new(CopilotFileStorageClientWrapper { client }))
            }
            CopilotGateStorageBackend::Keyring => {
                // Keyring support for Copilot is not yet implemented
                // Fall back to file storage
                log::warn!(
                    "Keyring storage for Copilot is not yet implemented, using file storage"
                );
                Self::create_client(CopilotGateStorageBackend::File)
            }
        }
    }

    /// Create a new CopilotGateState with the specified backend.
    pub fn new(backend: CopilotGateStorageBackend) -> Result<Self, String> {
        let client = Self::create_client(backend)?;
        Ok(Self {
            client: AsyncRwLock::new(Some(client)),
            pending_device_flow: AsyncRwLock::new(None),
            storage_backend: AsyncRwLock::new(backend),
        })
    }

    /// Create with default (Auto) backend
    pub fn with_defaults() -> Result<Self, String> {
        Self::new(CopilotGateStorageBackend::Auto)
    }

    /// Check if authenticated
    pub async fn is_authenticated(&self) -> Result<bool, String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Copilot Gate client not initialized")?;
        client.is_authenticated().await
    }

    /// Get token info
    pub async fn get_token_info(
        &self,
    ) -> Result<Option<crate::gate::copilot::models::TokenInfo>, String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Copilot Gate client not initialized")?;
        client.get_token_info().await
    }

    /// Start device code flow
    pub async fn start_device_flow(&self) -> Result<DeviceFlowPending, String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Copilot Gate client not initialized")?;
        let pending = client.start_device_flow().await?;

        // Store pending state for later polling
        *self.pending_device_flow.write().await = Some(pending.clone());

        Ok(pending)
    }

    /// Poll for token
    pub async fn poll_for_token(&self, device_code: &str) -> Result<CopilotPollResult, String> {
        let pending = self.pending_device_flow.read().await;
        let pending = pending
            .as_ref()
            .ok_or("No pending device flow. Call start_device_flow first.")?;

        // Verify the device code matches
        if pending.device_code != device_code {
            return Err("Device code mismatch".to_string());
        }

        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Copilot Gate client not initialized")?;
        client.poll_for_token(pending).await
    }

    /// Complete authentication with GitHub token
    pub async fn complete_auth(&self, github_token: String) -> Result<(), String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Copilot Gate client not initialized")?;
        client.complete_auth(github_token).await?;

        // Clear pending state
        *self.pending_device_flow.write().await = None;

        Ok(())
    }

    /// Sign out
    pub async fn sign_out(&self) -> Result<(), String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Copilot Gate client not initialized")?;
        client.sign_out().await?;

        // Clear pending state
        *self.pending_device_flow.write().await = None;

        Ok(())
    }

    /// Get available models
    pub async fn get_models(&self) -> Result<CopilotModelsResponse, String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Copilot Gate client not initialized")?;
        client.get_models().await
    }

    /// Get usage information
    pub async fn get_usage(&self) -> Result<CopilotUsageResponse, String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Copilot Gate client not initialized")?;
        client.get_usage().await
    }

    /// Get current storage backend name
    pub async fn storage_backend_name(&self) -> String {
        let client = self.client.read().await;
        if let Some(c) = client.as_ref() {
            c.storage_name().to_string()
        } else {
            self.storage_backend.read().await.to_string()
        }
    }

    /// Switch to a different storage backend
    pub async fn switch_backend(
        &self,
        new_backend: CopilotGateStorageBackend,
    ) -> Result<String, String> {
        let new_client = Self::create_client(new_backend)?;
        let backend_name = new_client.storage_name();

        {
            let mut client_lock = self.client.write().await;
            *client_lock = Some(new_client);
        }

        {
            let mut backend_lock = self.storage_backend.write().await;
            *backend_lock = new_backend;
        }

        // Clear any pending auth state when switching backends
        {
            let mut state_lock = self.pending_device_flow.write().await;
            *state_lock = None;
        }

        log::info!(
            "Switched Copilot Gate storage backend to: {}",
            backend_name
        );
        Ok(backend_name.to_string())
    }
}

// ============================================================================
// Command Response Types
// ============================================================================

/// Response for copilot_gate_start_auth command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotDeviceCodeResponse {
    /// The device verification code (internal, for polling).
    pub device_code: String,
    /// The user-facing code to enter at the verification URL.
    pub user_code: String,
    /// URL where the user should enter the code.
    pub verification_uri: String,
    /// Seconds until the device code expires.
    pub expires_in: u64,
    /// Minimum seconds between polling attempts.
    pub interval: u64,
}

impl From<DeviceFlowPending> for CopilotDeviceCodeResponse {
    fn from(pending: DeviceFlowPending) -> Self {
        Self {
            device_code: pending.device_code,
            user_code: pending.user_code,
            verification_uri: pending.verification_uri,
            expires_in: pending.expires_in,
            interval: pending.interval,
        }
    }
}

/// Response for copilot_gate_poll_auth command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotAuthPollResult {
    /// Status: "success", "pending", "slow_down", "expired", or "denied"
    pub status: String,
    /// Whether authentication is complete
    pub authenticated: bool,
    /// Error message if status is "expired" or "denied"
    pub error: Option<String>,
}

/// Response for copilot_gate_get_status command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotAuthStatus {
    /// Whether the user is authenticated with valid tokens
    pub authenticated: bool,
    /// Current storage backend being used (file, keyring, auto)
    pub storage_backend: String,
    /// Unix timestamp when the Copilot token expires (short-lived)
    pub copilot_token_expires_at: Option<i64>,
    /// Whether the user has a valid GitHub token (long-lived)
    pub has_github_token: bool,
}

/// Usage information for Copilot quotas
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Detail about a single quota category
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl From<CopilotQuotaInfo> for CopilotQuotaDetail {
    fn from(quota: CopilotQuotaInfo) -> Self {
        Self {
            used: quota.used,
            limit: quota.limit,
            unlimited: quota.unlimited,
            remaining: quota.remaining(),
            is_exhausted: quota.is_exhausted(),
        }
    }
}

/// Model information from Copilot API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopilotGateModelInfo {
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

impl From<CopilotModelInfo> for CopilotGateModelInfo {
    fn from(model: CopilotModelInfo) -> Self {
        let (supports_chat, supports_tools, supports_vision, max_context, max_output) =
            if let Some(caps) = &model.capabilities {
                let supports = caps.supports.as_ref();
                (
                    caps.supports_chat(),
                    supports.map(|s| s.tool_calls).unwrap_or(false),
                    supports.map(|s| s.vision).unwrap_or(false),
                    caps.limits
                        .as_ref()
                        .map(|l| l.max_context_window_tokens as u64),
                    caps.limits.as_ref().map(|l| l.max_output_tokens as u64),
                )
            } else {
                (false, false, false, None, None)
            };

        Self {
            id: model.id,
            owned_by: model.owned_by,
            created: model.created,
            supports_chat,
            supports_tools,
            supports_vision,
            max_context_tokens: max_context,
            max_output_tokens: max_output,
            preview: model.preview.unwrap_or(false),
        }
    }
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Start Copilot authentication using Device Code flow
///
/// Returns device code information including the user code and verification URL.
/// The user should visit the verification URL and enter the user code.
#[tauri::command]
pub async fn start_copilot_auth(
    state: State<'_, AppState>,
) -> Result<CopilotDeviceCodeResponse, String> {
    log::info!("Starting Copilot Device Code authentication flow");
    let pending = state.copilot_gate.start_device_flow().await?;
    Ok(pending.into())
}

/// Poll for Copilot authorization completion
///
/// Should be called repeatedly with appropriate delays (as specified by interval)
/// until status is "success", "expired", or "denied".
#[tauri::command]
pub async fn poll_copilot_auth(
    state: State<'_, AppState>,
    device_code: String,
) -> Result<CopilotAuthPollResult, String> {
    match state.copilot_gate.poll_for_token(&device_code).await {
        Ok(CopilotPollResult::Pending) => Ok(CopilotAuthPollResult {
            status: "pending".to_string(),
            authenticated: false,
            error: None,
        }),
        Ok(CopilotPollResult::SlowDown) => Ok(CopilotAuthPollResult {
            status: "slow_down".to_string(),
            authenticated: false,
            error: None,
        }),
        Ok(CopilotPollResult::Complete(github_token)) => {
            // Complete authentication by exchanging for Copilot token
            state.copilot_gate.complete_auth(github_token).await?;
            log::info!("Copilot authentication completed successfully");
            Ok(CopilotAuthPollResult {
                status: "success".to_string(),
                authenticated: true,
                error: None,
            })
        }
        Err(e) => {
            let error_msg = e.to_string();
            // TODO: Refactor to propagate structured copilot::error::Error through the trait
            // instead of string matching. This would involve changing CopilotGateClientOps
            // to return CopilotResult<T> and matching on Error variants directly.
            // Match against specific error messages from copilot::error::Error variants:
            // - DeviceCodeExpired: "Device code expired - please try again"
            // - AuthorizationDenied: "Authorization denied by user"
            let status = if error_msg.contains("Device code expired") {
                "expired"
            } else if error_msg.contains("Authorization denied") {
                "denied"
            } else {
                "error"
            };
            Ok(CopilotAuthPollResult {
                status: status.to_string(),
                authenticated: false,
                error: Some(error_msg),
            })
        }
    }
}

/// Check current Copilot authentication status
#[tauri::command]
pub async fn check_copilot_auth(state: State<'_, AppState>) -> Result<CopilotAuthStatus, String> {
    let authenticated = state.copilot_gate.is_authenticated().await?;
    let storage_backend = state.copilot_gate.storage_backend_name().await;

    let (copilot_token_expires_at, has_github_token) = if authenticated {
        match state.copilot_gate.get_token_info().await? {
            Some(token_info) => (token_info.copilot_expires_at, token_info.has_github_token()),
            None => (None, false),
        }
    } else {
        (None, false)
    };

    Ok(CopilotAuthStatus {
        authenticated,
        storage_backend,
        copilot_token_expires_at,
        has_github_token,
    })
}

/// Logout from Copilot and remove stored tokens
#[tauri::command]
pub async fn logout_copilot(state: State<'_, AppState>) -> Result<(), String> {
    state.copilot_gate.sign_out().await?;
    log::info!("Copilot logout completed");
    Ok(())
}

/// Get Copilot usage and quota information
///
/// Requires authentication. Returns current usage against quotas.
#[tauri::command]
pub async fn get_copilot_usage(state: State<'_, AppState>) -> Result<CopilotUsageInfo, String> {
    if !state.copilot_gate.is_authenticated().await? {
        return Err("Not authenticated. Please log in first.".to_string());
    }

    let usage = state.copilot_gate.get_usage().await?;

    Ok(CopilotUsageInfo {
        copilot_plan: usage.copilot_plan,
        quota_reset_date: usage.quota_reset_date,
        completions: usage
            .quota_snapshots
            .completions
            .map(CopilotQuotaDetail::from),
        premium_requests: usage
            .quota_snapshots
            .premium_requests
            .map(CopilotQuotaDetail::from),
        code_completions: usage
            .quota_snapshots
            .code_completions
            .map(CopilotQuotaDetail::from),
    })
}

/// List available models from Copilot API
///
/// Requires authentication. Returns list of models the user can access.
#[tauri::command]
pub async fn get_copilot_models(
    state: State<'_, AppState>,
) -> Result<Vec<CopilotGateModelInfo>, String> {
    if !state.copilot_gate.is_authenticated().await? {
        return Err("Not authenticated. Please log in first.".to_string());
    }

    let models = state.copilot_gate.get_models().await?;

    let model_infos: Vec<CopilotGateModelInfo> = models
        .data
        .into_iter()
        .map(CopilotGateModelInfo::from)
        .collect();

    log::info!("Copilot Gate: Listed {} models", model_infos.len());
    Ok(model_infos)
}
