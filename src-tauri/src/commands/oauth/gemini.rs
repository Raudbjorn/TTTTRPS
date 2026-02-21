//! Gemini OAuth State and Commands
//!
//! Handles OAuth for the Gemini provider (Google Cloud Code).
//! Provides type-erased storage backend support and runtime backend switching.

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::RwLock as AsyncRwLock;

// Unified OAuth types
use crate::oauth::{OAuthFlowState as GateOAuthFlowState, TokenInfo as GateTokenInfo};

// Gemini OAuth client
#[allow(deprecated)]
use crate::oauth::gemini::{
    CloudCodeClient as GeminiCloudCodeClient, FileTokenStorage as GeminiFileTokenStorage,
};
#[cfg(feature = "keyring")]
#[allow(deprecated)]
use crate::oauth::gemini::KeyringTokenStorage as GeminiKeyringTokenStorage;

// Import AppState - will be available via commands_legacy re-export
use crate::commands::AppState;

// ============================================================================
// Storage Backend Enum
// ============================================================================

/// Storage backend type for Gemini
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum GeminiStorageBackend {
    /// File-based storage (~/.config/antigravity/auth.json)
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
            Self::File => write!(f, "file"),
            Self::Keyring => write!(f, "keyring"),
            Self::Auto => write!(f, "auto"),
        }
    }
}

impl std::str::FromStr for GeminiStorageBackend {
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
// Gemini Client Trait (for type-erased storage backend support)
// ============================================================================

/// Trait for Gemini client operations, allowing type-erased storage backends.
///
/// This trait uses unified gate types for OAuth flow state while internally
/// using gemini types for API operations.
#[async_trait::async_trait]
#[allow(deprecated)]
trait GeminiClientOps: Send + Sync {
    async fn is_authenticated(&self) -> Result<bool, String>;
    async fn get_token_info(&self) -> Result<Option<GateTokenInfo>, String>;
    async fn start_oauth_flow_with_state(&self) -> Result<(String, GateOAuthFlowState), String>;
    async fn complete_oauth_flow(
        &self,
        code: &str,
        state: Option<&str>,
    ) -> Result<GateTokenInfo, String>;
    async fn logout(&self) -> Result<(), String>;
    async fn list_models(&self) -> Result<Vec<crate::oauth::gemini::GeminiApiModel>, String>;
    fn storage_name(&self) -> &'static str;
}

/// File storage client wrapper for Gemini
#[allow(deprecated)]
struct GeminiFileStorageClientWrapper {
    client: std::sync::Arc<GeminiCloudCodeClient<GeminiFileTokenStorage>>,
}

#[allow(deprecated)]
#[async_trait::async_trait]
impl GeminiClientOps for GeminiFileStorageClientWrapper {
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
    async fn list_models(&self) -> Result<Vec<crate::oauth::gemini::GeminiApiModel>, String> {
        self.client.list_models().await.map_err(|e| e.to_string())
    }
    fn storage_name(&self) -> &'static str {
        "file"
    }
}

/// Keyring storage client wrapper for Gemini
#[cfg(feature = "keyring")]
#[allow(deprecated)]
struct GeminiKeyringStorageClientWrapper {
    client: std::sync::Arc<GeminiCloudCodeClient<GeminiKeyringTokenStorage>>,
}

#[cfg(feature = "keyring")]
#[allow(deprecated)]
#[async_trait::async_trait]
impl GeminiClientOps for GeminiKeyringStorageClientWrapper {
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
    async fn list_models(&self) -> Result<Vec<crate::oauth::gemini::GeminiApiModel>, String> {
        self.client.list_models().await.map_err(|e| e.to_string())
    }
    fn storage_name(&self) -> &'static str {
        "keyring"
    }
}

// ============================================================================
// Gemini State
// ============================================================================

/// Type-erased Gemini client wrapper.
/// This allows storing the client in AppState regardless of storage backend
/// and supports runtime backend switching.
#[allow(deprecated)]
pub struct GeminiState {
    /// The active client (type-erased)
    client: AsyncRwLock<Option<Box<dyn GeminiClientOps>>>,
    /// In-memory flow state for OAuth (needed for state verification)
    pending_oauth_state: AsyncRwLock<Option<String>>,
    /// Current storage backend
    storage_backend: AsyncRwLock<GeminiStorageBackend>,
}

#[allow(deprecated)]
impl GeminiState {
    /// Create a client for the specified backend
    fn create_client(
        backend: GeminiStorageBackend,
    ) -> Result<Box<dyn GeminiClientOps>, String> {
        match backend {
            GeminiStorageBackend::File => {
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
            GeminiStorageBackend::Keyring => {
                let storage = GeminiKeyringTokenStorage::new();
                let client = GeminiCloudCodeClient::builder()
                    .with_storage(storage)
                    .build();
                Ok(Box::new(GeminiKeyringStorageClientWrapper {
                    client: std::sync::Arc::new(client),
                }))
            }
            #[cfg(not(feature = "keyring"))]
            GeminiStorageBackend::Keyring => {
                Err("Keyring storage is not available (keyring feature disabled)".to_string())
            }
            GeminiStorageBackend::Auto => {
                // Try keyring first, fall back to file
                #[cfg(feature = "keyring")]
                {
                    match Self::create_client(GeminiStorageBackend::Keyring) {
                        Ok(client) => {
                            log::info!("Gemini: Auto-selected keyring storage backend");
                            return Ok(client);
                        }
                        Err(e) => {
                            log::warn!(
                                "Gemini: Keyring storage failed, falling back to file: {}",
                                e
                            );
                        }
                    }
                }
                log::info!("Gemini: Using file storage backend");
                Self::create_client(GeminiStorageBackend::File)
            }
        }
    }

    /// Create a new GeminiState with the specified backend.
    pub fn new(backend: GeminiStorageBackend) -> Result<Self, String> {
        let client = Self::create_client(backend)?;
        Ok(Self {
            client: AsyncRwLock::new(Some(client)),
            pending_oauth_state: AsyncRwLock::new(None),
            storage_backend: AsyncRwLock::new(backend),
        })
    }

    /// Create with default (Auto) backend
    pub fn with_defaults() -> Result<Self, String> {
        Self::new(GeminiStorageBackend::Auto)
    }

    /// Switch to a different storage backend.
    /// This recreates the client with the new backend.
    /// Note: Any existing tokens will not be migrated.
    pub async fn switch_backend(
        &self,
        new_backend: GeminiStorageBackend,
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

        log::info!("Gemini storage backend switched to: {}", backend_name);
        Ok(backend_name.to_string())
    }

    /// Check if authenticated
    pub async fn is_authenticated(&self) -> Result<bool, String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Gemini client not initialized")?;
        client.is_authenticated().await
    }

    /// Get token info using unified gate types
    pub async fn get_token_info(&self) -> Result<Option<GateTokenInfo>, String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Gemini client not initialized")?;
        client.get_token_info().await
    }

    /// Start OAuth flow
    pub async fn start_oauth_flow(&self) -> Result<(String, String), String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Gemini client not initialized")?;
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
            .ok_or("Gemini client not initialized")?;
        let token = client.complete_oauth_flow(code, state).await?;

        Ok(token)
    }

    /// Logout
    pub async fn logout(&self) -> Result<(), String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Gemini client not initialized")?;
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

    /// List available models from the Cloud Code API.
    pub async fn list_models(&self) -> Result<Vec<crate::oauth::gemini::GeminiApiModel>, String> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Gemini client not initialized")?;
        client.list_models().await
    }
}

// ============================================================================
// Command Response Types
// ============================================================================

/// Response for gemini_get_status command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiStatusResponse {
    /// Whether the user is authenticated with valid tokens
    pub authenticated: bool,
    /// Current storage backend being used (file, keyring, auto)
    pub storage_backend: String,
    /// Unix timestamp when token expires, if authenticated
    pub token_expires_at: Option<i64>,
    /// Whether keyring (secret service) is available on this system
    pub keyring_available: bool,
}

/// Response for gemini_start_oauth command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiOAuthStartResponse {
    /// URL to open in user's browser for OAuth authorization
    pub auth_url: String,
    /// State parameter for CSRF protection (pass back to complete_oauth)
    pub state: String,
}

/// Response for gemini_complete_oauth command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiOAuthCompleteResponse {
    /// Whether the OAuth flow completed successfully
    pub success: bool,
    /// Error message if the flow failed
    pub error: Option<String>,
}

/// Response for gemini_logout command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiLogoutResponse {
    /// Whether the logout was successful
    pub success: bool,
}

/// Response for gemini_set_storage_backend command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiSetStorageResponse {
    /// Whether the storage backend was changed successfully
    pub success: bool,
    /// The currently active storage backend after the change
    pub active_backend: String,
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Get Gemini OAuth status
///
/// Returns authentication status, storage backend, token expiration, and keyring availability.
#[tauri::command]
pub async fn gemini_get_status(
    state: State<'_, AppState>,
) -> Result<GeminiStatusResponse, String> {
    let authenticated = state.gemini.is_authenticated().await?;
    let storage_backend = state.gemini.storage_backend_name().await;

    let token_expires_at = if authenticated {
        state
            .gemini
            .get_token_info()
            .await?
            .map(|t| t.expires_at)
    } else {
        None
    };

    // Check if keyring is available on this system (using unified gate)
    #[cfg(feature = "keyring")]
    let keyring_available = crate::oauth::KeyringTokenStorage::is_available();
    #[cfg(not(feature = "keyring"))]
    let keyring_available = false;

    Ok(GeminiStatusResponse {
        authenticated,
        storage_backend,
        token_expires_at,
        keyring_available,
    })
}

/// Start Gemini OAuth flow
///
/// Returns the authorization URL that the user should open in their browser,
/// along with a state parameter for CSRF verification.
#[tauri::command]
pub async fn gemini_start_oauth(
    state: State<'_, AppState>,
) -> Result<GeminiOAuthStartResponse, String> {
    let (auth_url, oauth_state) = state.gemini.start_oauth_flow().await?;

    log::info!("Gemini OAuth flow started");

    Ok(GeminiOAuthStartResponse {
        auth_url,
        state: oauth_state,
    })
}

/// Complete Gemini OAuth flow
///
/// Exchange the authorization code for tokens and store them.
///
/// # Arguments
/// * `code` - The authorization code from the OAuth callback. May also be in
///   `code#state` format where the state is embedded after a `#` character.
/// * `oauth_state` - Optional state parameter for CSRF verification (if not embedded in code)
#[tauri::command]
pub async fn gemini_complete_oauth(
    code: String,
    oauth_state: Option<String>,
    state: State<'_, AppState>,
) -> Result<GeminiOAuthCompleteResponse, String> {
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
        "Gemini OAuth complete: code_len={}, state_provided={}",
        actual_code.len(),
        final_state.is_some()
    );

    match state
        .gemini
        .complete_oauth_flow(&actual_code, final_state.as_deref())
        .await
    {
        Ok(_token) => {
            log::info!("Gemini OAuth flow completed successfully");
            Ok(GeminiOAuthCompleteResponse {
                success: true,
                error: None,
            })
        }
        Err(e) => {
            log::error!("Gemini OAuth flow failed: {}", e);
            Ok(GeminiOAuthCompleteResponse {
                success: false,
                error: Some(e),
            })
        }
    }
}

/// Logout from Gemini and remove stored tokens
#[tauri::command]
pub async fn gemini_logout(
    state: State<'_, AppState>,
) -> Result<GeminiLogoutResponse, String> {
    state.gemini.logout().await?;
    log::info!("Gemini logout completed");

    Ok(GeminiLogoutResponse { success: true })
}

/// Change Gemini storage backend
///
/// Note: Changing the storage backend requires re-authentication as tokens
/// are not automatically migrated between backends.
///
/// # Arguments
/// * `backend` - Storage backend to use: "file", "keyring", or "auto"
#[tauri::command]
pub async fn gemini_set_storage_backend(
    backend: String,
    state: State<'_, AppState>,
) -> Result<GeminiSetStorageResponse, String> {
    // Parse and validate the backend string
    let new_backend: GeminiStorageBackend = backend.parse()?;

    // Switch to the new backend - this recreates the client
    let active = state.gemini.switch_backend(new_backend).await?;
    log::info!("Gemini storage backend switched to: {}", active);

    Ok(GeminiSetStorageResponse {
        success: true,
        active_backend: active,
    })
}

// ============================================================================
// Callback Server Integration
// ============================================================================

use crate::oauth::callback_server::{CallbackConfig, CallbackServer};
use std::time::Duration;

/// Response for gemini_oauth_with_callback command
#[derive(Debug, Clone, Serialize, Deserialize)]
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
/// 2. Generates the OAuth authorization URL
/// 3. Opens the URL in the user's default browser
/// 4. Waits for the OAuth callback (with timeout)
/// 5. Completes the OAuth flow automatically
///
/// This provides a seamless "one-click" authentication experience.
///
/// # Arguments
/// * `timeout_secs` - Optional timeout in seconds (default: 300 = 5 minutes)
/// * `open_browser` - Whether to automatically open the browser (default: true)
#[tauri::command]
pub async fn gemini_oauth_with_callback(
    timeout_secs: Option<u64>,
    open_browser: Option<bool>,
    state: State<'_, AppState>,
) -> Result<GeminiOAuthCallbackResponse, String> {
    let timeout = Duration::from_secs(timeout_secs.unwrap_or(300));
    let should_open_browser = open_browser.unwrap_or(true);

    // Start the OAuth flow to get the auth URL and state
    let (auth_url, oauth_state) = state.gemini.start_oauth_flow().await?;

    log::info!("Gemini OAuth: Starting callback server on port 51121");

    // Create and start the callback server
    let server = CallbackServer::new(CallbackConfig::gemini());
    let handle = match server.start().await {
        Ok(h) => h,
        Err(e) => {
            log::error!("Failed to start callback server: {}", e);
            return Ok(GeminiOAuthCallbackResponse {
                success: false,
                error: Some(format!("Failed to start callback server: {}", e)),
                auth_url: Some(auth_url),
            });
        }
    };

    // Open the browser if requested
    if should_open_browser {
        log::info!("Opening browser for Gemini OAuth");
        if let Err(e) = open::that(&auth_url) {
            log::warn!("Failed to open browser: {}. User can manually visit: {}", e, auth_url);
        }
    }

    // Wait for the callback
    log::info!("Waiting for OAuth callback (timeout: {}s)", timeout.as_secs());
    let callback_result = match handle.wait(timeout).await {
        Ok(result) => result,
        Err(e) => {
            log::error!("OAuth callback failed: {}", e);
            return Ok(GeminiOAuthCallbackResponse {
                success: false,
                error: Some(format!("OAuth callback failed: {}", e)),
                auth_url: Some(auth_url),
            });
        }
    };

    log::info!("OAuth callback received, completing flow");

    // Verify the state matches
    let callback_state = callback_result.state.as_deref();
    if callback_state != Some(oauth_state.as_str()) {
        log::error!(
            "OAuth state mismatch: expected '{}', got '{:?}'",
            oauth_state,
            callback_state
        );
        return Ok(GeminiOAuthCallbackResponse {
            success: false,
            error: Some("OAuth state mismatch - possible CSRF attack".to_string()),
            auth_url: None,
        });
    }

    // Complete the OAuth flow
    match state
        .gemini
        .complete_oauth_flow(&callback_result.code, callback_state)
        .await
    {
        Ok(_token) => {
            log::info!("Gemini OAuth completed successfully");
            Ok(GeminiOAuthCallbackResponse {
                success: true,
                error: None,
                auth_url: None,
            })
        }
        Err(e) => {
            log::error!("Gemini OAuth completion failed: {}", e);
            Ok(GeminiOAuthCallbackResponse {
                success: false,
                error: Some(e),
                auth_url: None,
            })
        }
    }
}

// ============================================================================
// Model Listing
// ============================================================================

/// A model available via the Gemini Cloud Code API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiModel {
    /// Unique identifier for the model.
    pub id: String,
    /// Display name for the model.
    pub name: String,
    /// Description of the model (optional).
    pub description: Option<String>,
}

impl From<crate::oauth::gemini::GeminiApiModel> for GeminiModel {
    fn from(m: crate::oauth::gemini::GeminiApiModel) -> Self {
        Self {
            id: m.id.clone(),
            name: m.display_name,
            description: m.description,
        }
    }
}

/// List available models from the Gemini Cloud Code API.
///
/// Returns models available for use with the authenticated account.
/// Requires successful OAuth authentication first.
#[tauri::command]
pub async fn gemini_list_models(
    state: State<'_, AppState>,
) -> Result<Vec<GeminiModel>, String> {
    // Check if authenticated
    if !state.gemini.is_authenticated().await? {
        return Err("Not authenticated. Please complete OAuth login first.".to_string());
    }

    let models = state.gemini.list_models().await?;
    Ok(models.into_iter().map(GeminiModel::from).collect())
}
