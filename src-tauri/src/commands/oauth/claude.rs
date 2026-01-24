//! Claude OAuth State and Commands
//!
//! Handles OAuth for the Claude Gate provider.
//! Provides type-erased storage backend support and runtime backend switching.

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::RwLock as AsyncRwLock;

// Unified Gate OAuth types
use crate::gate::{OAuthFlowState as GateOAuthFlowState, TokenInfo as GateTokenInfo};

// Claude Gate OAuth client
use crate::gate::claude::{ClaudeClient, FileTokenStorage};
#[cfg(feature = "keyring")]
use crate::gate::claude::KeyringTokenStorage;

// Import AppState - will be available via commands_legacy re-export
use crate::commands::AppState;

// ============================================================================
// Storage Backend Enum
// ============================================================================

/// Storage backend type for Claude Gate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ClaudeGateStorageBackend {
    /// File-based storage (~/.config/cld/auth.json)
    File,
    /// System keyring storage
    Keyring,
    /// Auto-select (keyring if available, else file)
    #[default]
    Auto,
}


impl std::fmt::Display for ClaudeGateStorageBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File => write!(f, "file"),
            Self::Keyring => write!(f, "keyring"),
            Self::Auto => write!(f, "auto"),
        }
    }
}

impl std::str::FromStr for ClaudeGateStorageBackend {
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
// Claude Gate Client Trait (for type-erased storage backend support)
// ============================================================================

/// Trait for Claude Gate client operations, allowing type-erased storage backends.
///
/// This trait uses unified gate types for OAuth flow state while internally
/// using claude_gate types for API operations.
#[async_trait::async_trait]
trait ClaudeGateClientOps: Send + Sync {
    async fn is_authenticated(&self) -> Result<bool, String>;
    async fn get_token_info(&self) -> Result<Option<GateTokenInfo>, String>;
    async fn start_oauth_flow_with_state(&self) -> Result<(String, GateOAuthFlowState), String>;
    async fn complete_oauth_flow(
        &self,
        code: &str,
        state: Option<&str>,
    ) -> Result<GateTokenInfo, String>;
    async fn logout(&self) -> Result<(), String>;
    async fn list_models(&self) -> Result<Vec<crate::gate::claude::ApiModel>, String>;
    fn storage_name(&self) -> &'static str;
}

/// File storage client wrapper
struct FileStorageClientWrapper {
    client: ClaudeClient<FileTokenStorage>,
}

#[async_trait::async_trait]
impl ClaudeGateClientOps for FileStorageClientWrapper {
    async fn is_authenticated(&self) -> Result<bool, String> {
        self.client
            .is_authenticated()
            .await
            .map_err(|e| e.to_string())
    }
    async fn get_token_info(&self) -> Result<Option<GateTokenInfo>, String> {
        self.client
            .get_token_info()
            .await
            .map(|opt| opt.map(GateTokenInfo::from))
            .map_err(|e| e.to_string())
    }
    async fn start_oauth_flow_with_state(&self) -> Result<(String, GateOAuthFlowState), String> {
        self.client
            .start_oauth_flow_with_state()
            .await
            .map(|(url, state)| (url, GateOAuthFlowState::from(state)))
            .map_err(|e| e.to_string())
    }
    async fn complete_oauth_flow(
        &self,
        code: &str,
        state: Option<&str>,
    ) -> Result<GateTokenInfo, String> {
        self.client
            .complete_oauth_flow(code, state)
            .await
            .map(GateTokenInfo::from)
            .map_err(|e| e.to_string())
    }
    async fn logout(&self) -> Result<(), String> {
        self.client.logout().await.map_err(|e| e.to_string())
    }
    async fn list_models(&self) -> Result<Vec<crate::gate::claude::ApiModel>, String> {
        self.client.list_models().await.map_err(|e| e.to_string())
    }
    fn storage_name(&self) -> &'static str {
        "file"
    }
}

/// Keyring storage client wrapper
#[cfg(feature = "keyring")]
struct KeyringStorageClientWrapper {
    client: ClaudeClient<KeyringTokenStorage>,
}

#[cfg(feature = "keyring")]
#[async_trait::async_trait]
impl ClaudeGateClientOps for KeyringStorageClientWrapper {
    async fn is_authenticated(&self) -> Result<bool, String> {
        self.client
            .is_authenticated()
            .await
            .map_err(|e| e.to_string())
    }
    async fn get_token_info(&self) -> Result<Option<GateTokenInfo>, String> {
        self.client
            .get_token_info()
            .await
            .map(|opt| opt.map(GateTokenInfo::from))
            .map_err(|e| e.to_string())
    }
    async fn start_oauth_flow_with_state(&self) -> Result<(String, GateOAuthFlowState), String> {
        self.client
            .start_oauth_flow_with_state()
            .await
            .map(|(url, state)| (url, GateOAuthFlowState::from(state)))
            .map_err(|e| e.to_string())
    }
    async fn complete_oauth_flow(
        &self,
        code: &str,
        state: Option<&str>,
    ) -> Result<GateTokenInfo, String> {
        self.client
            .complete_oauth_flow(code, state)
            .await
            .map(GateTokenInfo::from)
            .map_err(|e| e.to_string())
    }
    async fn logout(&self) -> Result<(), String> {
        self.client.logout().await.map_err(|e| e.to_string())
    }
    async fn list_models(&self) -> Result<Vec<crate::gate::claude::ApiModel>, String> {
        self.client.list_models().await.map_err(|e| e.to_string())
    }
    fn storage_name(&self) -> &'static str {
        "keyring"
    }
}

// ============================================================================
// Claude Gate State
// ============================================================================

/// Type-erased Claude Gate client wrapper.
/// This allows storing the client in AppState regardless of storage backend
/// and supports runtime backend switching.
pub struct ClaudeGateState {
    /// The active client (type-erased)
    client: AsyncRwLock<Option<Box<dyn ClaudeGateClientOps>>>,
    /// In-memory flow state for OAuth (needed for state verification)
    pending_oauth_state: AsyncRwLock<Option<String>>,
    /// Current storage backend
    storage_backend: AsyncRwLock<ClaudeGateStorageBackend>,
}

impl ClaudeGateState {
    /// Create a client for the specified backend
    fn create_client(
        backend: ClaudeGateStorageBackend,
    ) -> Result<Box<dyn ClaudeGateClientOps>, String> {
        match backend {
            ClaudeGateStorageBackend::File => {
                let storage = FileTokenStorage::default_path()
                    .map_err(|e| format!("Failed to create file storage: {}", e))?;
                let client = ClaudeClient::builder()
                    .with_storage(storage)
                    .build()
                    .map_err(|e| format!("Failed to create Claude client: {}", e))?;
                Ok(Box::new(FileStorageClientWrapper { client }))
            }
            #[cfg(feature = "keyring")]
            ClaudeGateStorageBackend::Keyring => {
                let storage = KeyringTokenStorage::new();
                let client = ClaudeClient::builder()
                    .with_storage(storage)
                    .build()
                    .map_err(|e| format!("Failed to create Claude client with keyring: {}", e))?;
                Ok(Box::new(KeyringStorageClientWrapper { client }))
            }
            #[cfg(not(feature = "keyring"))]
            ClaudeGateStorageBackend::Keyring => {
                Err("Keyring storage is not available (keyring feature disabled)".to_string())
            }
            ClaudeGateStorageBackend::Auto => {
                // Try keyring first, fall back to file
                #[cfg(feature = "keyring")]
                {
                    match Self::create_client(ClaudeGateStorageBackend::Keyring) {
                        Ok(client) => {
                            log::info!("Auto-selected keyring storage backend");
                            return Ok(client);
                        }
                        Err(e) => {
                            log::warn!("Keyring storage failed, falling back to file: {}", e);
                        }
                    }
                }
                log::info!("Using file storage backend");
                Self::create_client(ClaudeGateStorageBackend::File)
            }
        }
    }

    /// Create a new ClaudeGateState with the specified backend.
    pub fn new(backend: ClaudeGateStorageBackend) -> Result<Self, String> {
        let client = Self::create_client(backend)?;
        Ok(Self {
            client: AsyncRwLock::new(Some(client)),
            pending_oauth_state: AsyncRwLock::new(None),
            storage_backend: AsyncRwLock::new(backend),
        })
    }

    /// Create with default (Auto) backend
    pub fn with_defaults() -> Result<Self, String> {
        Self::new(ClaudeGateStorageBackend::Auto)
    }

    /// Switch to a different storage backend.
    /// This recreates the client with the new backend.
    /// Note: Any existing tokens will not be migrated.
    pub async fn switch_backend(
        &self,
        new_backend: ClaudeGateStorageBackend,
    ) -> Result<String, String> {
        let new_client = Self::create_client(new_backend)?;
        let backend_name = new_client.storage_name();

        // Replace the client
        {
            let mut client_lock = self.client.write().await;
            *client_lock = Some(new_client);
        }

        // Update the backend setting
        {
            let mut backend_lock = self.storage_backend.write().await;
            *backend_lock = new_backend;
        }

        // Clear any pending OAuth state
        {
            let mut state_lock = self.pending_oauth_state.write().await;
            *state_lock = None;
        }

        log::info!("Switched Claude Gate storage backend to: {}", backend_name);
        Ok(backend_name.to_string())
    }

    /// Check if authenticated
    pub async fn is_authenticated(&self) -> Result<bool, String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Claude Gate client not initialized")?;
        client.is_authenticated().await
    }

    /// Get token info using unified gate types
    pub async fn get_token_info(&self) -> Result<Option<GateTokenInfo>, String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Claude Gate client not initialized")?;
        client.get_token_info().await
    }

    /// Start OAuth flow
    pub async fn start_oauth_flow(&self) -> Result<(String, String), String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Claude Gate client not initialized")?;
        let (url, state) = client.start_oauth_flow_with_state().await?;

        // Store the state for verification
        *self.pending_oauth_state.write().await = Some(state.state.clone());

        Ok((url, state.state))
    }

    /// Complete OAuth flow using unified gate types
    pub async fn complete_oauth_flow(
        &self,
        code: &str,
        state: Option<&str>,
    ) -> Result<GateTokenInfo, String> {
        // Verify state if provided
        if let Some(received_state) = state {
            let pending = self.pending_oauth_state.read().await;
            if let Some(expected_state) = pending.as_ref() {
                if received_state != expected_state {
                    return Err(format!(
                        "State mismatch: expected {}, got {}",
                        expected_state, received_state
                    ));
                }
            }
        }

        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Claude Gate client not initialized")?;
        let token = client.complete_oauth_flow(code, state).await?;

        // Clear pending state
        *self.pending_oauth_state.write().await = None;

        Ok(token)
    }

    /// Logout
    pub async fn logout(&self) -> Result<(), String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Claude Gate client not initialized")?;
        client.logout().await
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

    /// List available models from Claude API
    pub async fn list_models(&self) -> Result<Vec<crate::gate::claude::ApiModel>, String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Claude Gate client not initialized")?;
        client.list_models().await
    }
}

// ============================================================================
// Command Response Types
// ============================================================================

/// Response for claude_gate_get_status command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeGateStatusResponse {
    /// Whether the user is authenticated with valid tokens
    pub authenticated: bool,
    /// Current storage backend being used (file, keyring, auto)
    pub storage_backend: String,
    /// Unix timestamp when token expires, if authenticated
    pub token_expires_at: Option<i64>,
    /// Whether keyring (secret service) is available on this system
    pub keyring_available: bool,
}

/// Response for claude_gate_start_oauth command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeGateOAuthStartResponse {
    /// URL to open in user's browser for OAuth authorization
    pub auth_url: String,
    /// State parameter for CSRF protection (pass back to complete_oauth)
    pub state: String,
}

/// Response for claude_gate_complete_oauth command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeGateOAuthCompleteResponse {
    /// Whether the OAuth flow completed successfully
    pub success: bool,
    /// Error message if the flow failed
    pub error: Option<String>,
}

/// Response for claude_gate_logout command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeGateLogoutResponse {
    /// Whether logout was successful
    pub success: bool,
}

/// Response for claude_gate_set_storage_backend command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeGateSetStorageResponse {
    /// Whether the storage backend was changed successfully
    pub success: bool,
    /// The currently active storage backend after the change
    pub active_backend: String,
}

/// Model info returned from Claude Gate API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeGateModelInfo {
    /// Model ID (e.g., "claude-sonnet-4-20250514")
    pub id: String,
    /// Display name (may be same as ID if not provided)
    pub name: String,
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Get Claude Gate OAuth status
///
/// Returns authentication status, storage backend, token expiration, and keyring availability.
#[tauri::command]
pub async fn claude_gate_get_status(
    state: State<'_, AppState>,
) -> Result<ClaudeGateStatusResponse, String> {
    let authenticated = state.claude_gate.is_authenticated().await?;
    let storage_backend = state.claude_gate.storage_backend_name().await;

    let token_expires_at = if authenticated {
        state
            .claude_gate
            .get_token_info()
            .await?
            .map(|t| t.expires_at)
    } else {
        None
    };

    // Check if keyring is available on this system (using unified gate)
    #[cfg(feature = "keyring")]
    let keyring_available = crate::gate::KeyringTokenStorage::is_available();
    #[cfg(not(feature = "keyring"))]
    let keyring_available = false;

    Ok(ClaudeGateStatusResponse {
        authenticated,
        storage_backend,
        token_expires_at,
        keyring_available,
    })
}

/// Start Claude Gate OAuth flow
///
/// Returns the authorization URL that the user should open in their browser,
/// along with a state parameter for CSRF verification.
#[tauri::command]
pub async fn claude_gate_start_oauth(
    state: State<'_, AppState>,
) -> Result<ClaudeGateOAuthStartResponse, String> {
    let (auth_url, oauth_state) = state.claude_gate.start_oauth_flow().await?;

    log::info!("Claude Gate OAuth flow started");

    Ok(ClaudeGateOAuthStartResponse {
        auth_url,
        state: oauth_state,
    })
}

/// Complete Claude Gate OAuth flow
///
/// Exchange the authorization code for tokens and store them.
///
/// # Arguments
/// * `code` - The authorization code from the OAuth callback. May also be in
///   `code#state` format where the state is embedded after a `#` character.
/// * `oauth_state` - Optional state parameter for CSRF verification (if not embedded in code)
#[tauri::command]
pub async fn claude_gate_complete_oauth(
    code: String,
    oauth_state: Option<String>,
    state: State<'_, AppState>,
) -> Result<ClaudeGateOAuthCompleteResponse, String> {
    // Parse code#state format if present
    let (actual_code, embedded_state) = if let Some(hash_pos) = code.find('#') {
        let (c, s) = code.split_at(hash_pos);
        // Only treat as embedded state if there is content after the '#' character
        let embedded = if s.len() > 1 {
            Some(s[1..].to_string())
        } else {
            None
        };
        (c.to_string(), embedded)
    } else {
        (code, None)
    };

    // Use embedded state if present, otherwise use the provided oauth_state
    let final_state = embedded_state.or(oauth_state);

    log::debug!(
        "OAuth complete: code_len={}, state_provided={}",
        actual_code.len(),
        final_state.is_some()
    );

    match state
        .claude_gate
        .complete_oauth_flow(&actual_code, final_state.as_deref())
        .await
    {
        Ok(_token) => {
            log::info!("Claude Gate OAuth flow completed successfully");
            Ok(ClaudeGateOAuthCompleteResponse {
                success: true,
                error: None,
            })
        }
        Err(e) => {
            log::error!("Claude Gate OAuth flow failed: {}", e);
            Ok(ClaudeGateOAuthCompleteResponse {
                success: false,
                error: Some(e),
            })
        }
    }
}

/// Logout from Claude Gate and remove stored tokens
#[tauri::command]
pub async fn claude_gate_logout(
    state: State<'_, AppState>,
) -> Result<ClaudeGateLogoutResponse, String> {
    state.claude_gate.logout().await?;
    log::info!("Claude Gate logout completed");

    Ok(ClaudeGateLogoutResponse { success: true })
}

/// Change Claude Gate storage backend
///
/// Note: Changing the storage backend requires re-authentication as tokens
/// are not automatically migrated between backends.
///
/// # Arguments
/// * `backend` - Storage backend to use: "file", "keyring", or "auto"
#[tauri::command]
pub async fn claude_gate_set_storage_backend(
    backend: String,
    state: State<'_, AppState>,
) -> Result<ClaudeGateSetStorageResponse, String> {
    // Parse and validate the backend string
    let new_backend: ClaudeGateStorageBackend = backend.parse()?;

    // Switch to the new backend - this recreates the client
    let active = state.claude_gate.switch_backend(new_backend).await?;
    log::info!("Claude Gate storage backend switched to: {}", active);

    Ok(ClaudeGateSetStorageResponse {
        success: true,
        active_backend: active,
    })
}

/// List available models from Claude Gate API
///
/// Requires authentication. Returns list of models the user can access.
#[tauri::command]
pub async fn claude_gate_list_models(
    state: State<'_, AppState>,
) -> Result<Vec<ClaudeGateModelInfo>, String> {
    // Check if authenticated
    if !state.claude_gate.is_authenticated().await? {
        return Err("Not authenticated. Please log in first.".to_string());
    }

    // Get models from API
    let models = state.claude_gate.list_models().await?;

    // Convert to response format
    let model_infos: Vec<ClaudeGateModelInfo> = models
        .into_iter()
        .map(|m| ClaudeGateModelInfo {
            id: m.id.clone(),
            name: if m.display_name.is_empty() {
                m.id
            } else {
                m.display_name
            },
        })
        .collect();

    log::info!("Claude Gate: Listed {} models", model_infos.len());
    Ok(model_infos)
}
