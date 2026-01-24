//! Gemini OAuth State and Commands
//!
//! Handles OAuth for the Gemini Gate provider (Google Cloud Code).
//! Provides type-erased storage backend support and runtime backend switching.

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::RwLock as AsyncRwLock;

// Unified Gate OAuth types
use crate::gate::{OAuthFlowState as GateOAuthFlowState, TokenInfo as GateTokenInfo};

// Gemini Gate OAuth client
#[allow(deprecated)]
use crate::gate::gemini::{
    CloudCodeClient as GeminiCloudCodeClient, FileTokenStorage as GeminiFileTokenStorage,
};
#[cfg(feature = "keyring")]
#[allow(deprecated)]
use crate::gate::gemini::KeyringTokenStorage as GeminiKeyringTokenStorage;

// Import AppState - will be available via commands_legacy re-export
use crate::commands::AppState;

// ============================================================================
// Storage Backend Enum
// ============================================================================

/// Storage backend type for Gemini Gate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum GeminiGateStorageBackend {
    /// File-based storage (~/.config/antigravity/auth.json)
    File,
    /// System keyring storage
    Keyring,
    /// Auto-select (keyring if available, else file)
    #[default]
    Auto,
}


impl std::fmt::Display for GeminiGateStorageBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File => write!(f, "file"),
            Self::Keyring => write!(f, "keyring"),
            Self::Auto => write!(f, "auto"),
        }
    }
}

impl std::str::FromStr for GeminiGateStorageBackend {
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
// Gemini Gate Client Trait (for type-erased storage backend support)
// ============================================================================

/// Trait for Gemini Gate client operations, allowing type-erased storage backends.
///
/// This trait uses unified gate types for OAuth flow state while internally
/// using gemini_gate types for API operations.
#[async_trait::async_trait]
#[allow(deprecated)]
trait GeminiGateClientOps: Send + Sync {
    async fn is_authenticated(&self) -> Result<bool, String>;
    async fn get_token_info(&self) -> Result<Option<GateTokenInfo>, String>;
    async fn start_oauth_flow_with_state(&self) -> Result<(String, GateOAuthFlowState), String>;
    async fn complete_oauth_flow(
        &self,
        code: &str,
        state: Option<&str>,
    ) -> Result<GateTokenInfo, String>;
    async fn logout(&self) -> Result<(), String>;
    fn storage_name(&self) -> &'static str;
}

/// File storage client wrapper for Gemini Gate
#[allow(deprecated)]
struct GeminiFileStorageClientWrapper {
    client: std::sync::Arc<GeminiCloudCodeClient<GeminiFileTokenStorage>>,
}

#[allow(deprecated)]
#[async_trait::async_trait]
impl GeminiGateClientOps for GeminiFileStorageClientWrapper {
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
            .map_err(|e| e.to_string())
    }
    async fn start_oauth_flow_with_state(&self) -> Result<(String, GateOAuthFlowState), String> {
        self.client
            .start_oauth_flow()
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
            .await.map_err(|e| e.to_string())
    }
    async fn logout(&self) -> Result<(), String> {
        self.client.logout().await.map_err(|e| e.to_string())
    }
    fn storage_name(&self) -> &'static str {
        "file"
    }
}

/// Keyring storage client wrapper for Gemini Gate
#[cfg(feature = "keyring")]
#[allow(deprecated)]
struct GeminiKeyringStorageClientWrapper {
    client: std::sync::Arc<GeminiCloudCodeClient<GeminiKeyringTokenStorage>>,
}

#[cfg(feature = "keyring")]
#[allow(deprecated)]
#[async_trait::async_trait]
impl GeminiGateClientOps for GeminiKeyringStorageClientWrapper {
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
            .map_err(|e| e.to_string())
    }
    async fn start_oauth_flow_with_state(&self) -> Result<(String, GateOAuthFlowState), String> {
        self.client
            .start_oauth_flow()
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
            .await.map_err(|e| e.to_string())
    }
    async fn logout(&self) -> Result<(), String> {
        self.client.logout().await.map_err(|e| e.to_string())
    }
    fn storage_name(&self) -> &'static str {
        "keyring"
    }
}

// ============================================================================
// Gemini Gate State
// ============================================================================

/// Type-erased Gemini Gate client wrapper.
/// This allows storing the client in AppState regardless of storage backend
/// and supports runtime backend switching.
#[allow(deprecated)]
pub struct GeminiGateState {
    /// The active client (type-erased)
    client: AsyncRwLock<Option<Box<dyn GeminiGateClientOps>>>,
    /// In-memory flow state for OAuth (needed for state verification)
    pending_oauth_state: AsyncRwLock<Option<String>>,
    /// Current storage backend
    storage_backend: AsyncRwLock<GeminiGateStorageBackend>,
}

#[allow(deprecated)]
impl GeminiGateState {
    /// Create a client for the specified backend
    fn create_client(
        backend: GeminiGateStorageBackend,
    ) -> Result<Box<dyn GeminiGateClientOps>, String> {
        match backend {
            GeminiGateStorageBackend::File => {
                let storage = GeminiFileTokenStorage::default_path()
                    .map_err(|e| format!("Failed to create file storage: {}", e))?;
                let client = GeminiCloudCodeClient::builder()
                    .with_storage(storage)
                    .build();
                Ok(Box::new(GeminiFileStorageClientWrapper {
                    client: std::sync::Arc::new(client),
                }))
            }
            #[cfg(feature = "keyring")]
            GeminiGateStorageBackend::Keyring => {
                let storage = GeminiKeyringTokenStorage::new();
                let client = GeminiCloudCodeClient::builder()
                    .with_storage(storage)
                    .build();
                Ok(Box::new(GeminiKeyringStorageClientWrapper {
                    client: std::sync::Arc::new(client),
                }))
            }
            #[cfg(not(feature = "keyring"))]
            GeminiGateStorageBackend::Keyring => {
                Err("Keyring storage is not available (keyring feature disabled)".to_string())
            }
            GeminiGateStorageBackend::Auto => {
                // Try keyring first, fall back to file
                #[cfg(feature = "keyring")]
                {
                    match Self::create_client(GeminiGateStorageBackend::Keyring) {
                        Ok(client) => {
                            log::info!("Gemini Gate: Auto-selected keyring storage backend");
                            return Ok(client);
                        }
                        Err(e) => {
                            log::warn!(
                                "Gemini Gate: Keyring storage failed, falling back to file: {}",
                                e
                            );
                        }
                    }
                }
                log::info!("Gemini Gate: Using file storage backend");
                Self::create_client(GeminiGateStorageBackend::File)
            }
        }
    }

    /// Create a new GeminiGateState with the specified backend.
    pub fn new(backend: GeminiGateStorageBackend) -> Result<Self, String> {
        let client = Self::create_client(backend)?;
        Ok(Self {
            client: AsyncRwLock::new(Some(client)),
            pending_oauth_state: AsyncRwLock::new(None),
            storage_backend: AsyncRwLock::new(backend),
        })
    }

    /// Create with default (Auto) backend
    pub fn with_defaults() -> Result<Self, String> {
        Self::new(GeminiGateStorageBackend::Auto)
    }

    /// Switch to a different storage backend.
    /// This recreates the client with the new backend.
    /// Note: Any existing tokens will not be migrated.
    pub async fn switch_backend(
        &self,
        new_backend: GeminiGateStorageBackend,
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

        log::info!("Gemini Gate storage backend switched to: {}", backend_name);
        Ok(backend_name.to_string())
    }

    /// Check if authenticated
    pub async fn is_authenticated(&self) -> Result<bool, String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Gemini Gate client not initialized")?;
        client.is_authenticated().await
    }

    /// Get token info using unified gate types
    pub async fn get_token_info(&self) -> Result<Option<GateTokenInfo>, String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Gemini Gate client not initialized")?;
        client.get_token_info().await
    }

    /// Start OAuth flow
    pub async fn start_oauth_flow(&self) -> Result<(String, String), String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Gemini Gate client not initialized")?;
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
        // Verify state - CSRF protection requires a pending OAuth flow
        // Use write lock for atomic check-and-clear to prevent TOCTOU race
        {
            let mut pending = self.pending_oauth_state.write().await;
            match pending.take() {
                Some(expected_state) => {
                    match state {
                        Some(received_state) if received_state == expected_state => {
                            // State matches - pending already cleared by take()
                        }
                        Some(_received_state) => {
                            // Note: Don't expose expected/received state in error to prevent info leakage
                            log::warn!("CSRF state mismatch during OAuth callback");
                            return Err("OAuth state mismatch - possible CSRF attack".to_string());
                        }
                        None => {
                            log::warn!("Missing CSRF state parameter in OAuth callback");
                            return Err("Missing state parameter for CSRF verification".to_string());
                        }
                    }
                }
                None => {
                    // No pending OAuth flow - reject callback entirely
                    log::warn!("OAuth callback received but no OAuth flow was initiated");
                    return Err("No pending OAuth flow - callback rejected".to_string());
                }
            }
        } // Write lock released here

        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Gemini Gate client not initialized")?;
        let token = client.complete_oauth_flow(code, state).await?;

        Ok(token)
    }

    /// Logout
    pub async fn logout(&self) -> Result<(), String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Gemini Gate client not initialized")?;
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
}

// ============================================================================
// Command Response Types
// ============================================================================

/// Response for gemini_gate_get_status command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiGateStatusResponse {
    /// Whether the user is authenticated with valid tokens
    pub authenticated: bool,
    /// Current storage backend being used (file, keyring, auto)
    pub storage_backend: String,
    /// Unix timestamp when token expires, if authenticated
    pub token_expires_at: Option<i64>,
    /// Whether keyring (secret service) is available on this system
    pub keyring_available: bool,
}

/// Response for gemini_gate_start_oauth command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiGateOAuthStartResponse {
    /// URL to open in user's browser for OAuth authorization
    pub auth_url: String,
    /// State parameter for CSRF protection (pass back to complete_oauth)
    pub state: String,
}

/// Response for gemini_gate_complete_oauth command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiGateOAuthCompleteResponse {
    /// Whether the OAuth flow completed successfully
    pub success: bool,
    /// Error message if the flow failed
    pub error: Option<String>,
}

/// Response for gemini_gate_logout command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiGateLogoutResponse {
    /// Whether the logout was successful
    pub success: bool,
}

/// Response for gemini_gate_set_storage_backend command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiGateSetStorageResponse {
    /// Whether the storage backend was changed successfully
    pub success: bool,
    /// The currently active storage backend after the change
    pub active_backend: String,
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Get Gemini Gate OAuth status
///
/// Returns authentication status, storage backend, token expiration, and keyring availability.
#[tauri::command]
pub async fn gemini_gate_get_status(
    state: State<'_, AppState>,
) -> Result<GeminiGateStatusResponse, String> {
    let authenticated = state.gemini_gate.is_authenticated().await?;
    let storage_backend = state.gemini_gate.storage_backend_name().await;

    let token_expires_at = if authenticated {
        state
            .gemini_gate
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

    Ok(GeminiGateStatusResponse {
        authenticated,
        storage_backend,
        token_expires_at,
        keyring_available,
    })
}

/// Start Gemini Gate OAuth flow
///
/// Returns the authorization URL that the user should open in their browser,
/// along with a state parameter for CSRF verification.
#[tauri::command]
pub async fn gemini_gate_start_oauth(
    state: State<'_, AppState>,
) -> Result<GeminiGateOAuthStartResponse, String> {
    let (auth_url, oauth_state) = state.gemini_gate.start_oauth_flow().await?;

    log::info!("Gemini Gate OAuth flow started");

    Ok(GeminiGateOAuthStartResponse {
        auth_url,
        state: oauth_state,
    })
}

/// Complete Gemini Gate OAuth flow
///
/// Exchange the authorization code for tokens and store them.
///
/// # Arguments
/// * `code` - The authorization code from the OAuth callback. May also be in
///   `code#state` format where the state is embedded after a `#` character.
/// * `oauth_state` - Optional state parameter for CSRF verification (if not embedded in code)
#[tauri::command]
pub async fn gemini_gate_complete_oauth(
    code: String,
    oauth_state: Option<String>,
    state: State<'_, AppState>,
) -> Result<GeminiGateOAuthCompleteResponse, String> {
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
        "Gemini Gate OAuth complete: code_len={}, state_provided={}",
        actual_code.len(),
        final_state.is_some()
    );

    match state
        .gemini_gate
        .complete_oauth_flow(&actual_code, final_state.as_deref())
        .await
    {
        Ok(_token) => {
            log::info!("Gemini Gate OAuth flow completed successfully");
            Ok(GeminiGateOAuthCompleteResponse {
                success: true,
                error: None,
            })
        }
        Err(e) => {
            log::error!("Gemini Gate OAuth flow failed: {}", e);
            Ok(GeminiGateOAuthCompleteResponse {
                success: false,
                error: Some(e),
            })
        }
    }
}

/// Logout from Gemini Gate and remove stored tokens
#[tauri::command]
pub async fn gemini_gate_logout(
    state: State<'_, AppState>,
) -> Result<GeminiGateLogoutResponse, String> {
    state.gemini_gate.logout().await?;
    log::info!("Gemini Gate logout completed");

    Ok(GeminiGateLogoutResponse { success: true })
}

/// Change Gemini Gate storage backend
///
/// Note: Changing the storage backend requires re-authentication as tokens
/// are not automatically migrated between backends.
///
/// # Arguments
/// * `backend` - Storage backend to use: "file", "keyring", or "auto"
#[tauri::command]
pub async fn gemini_gate_set_storage_backend(
    backend: String,
    state: State<'_, AppState>,
) -> Result<GeminiGateSetStorageResponse, String> {
    // Parse and validate the backend string
    let new_backend: GeminiGateStorageBackend = backend.parse()?;

    // Switch to the new backend - this recreates the client
    let active = state.gemini_gate.switch_backend(new_backend).await?;
    log::info!("Gemini Gate storage backend switched to: {}", active);

    Ok(GeminiGateSetStorageResponse {
        success: true,
        active_backend: active,
    })
}
