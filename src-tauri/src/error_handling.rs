use serde::{Deserialize, Serialize};
use std::fmt;

/// Comprehensive error types for native features
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "details")]
pub enum NativeFeatureError {
    /// File dialog operation failed
    FileDialog {
        operation: String,
        reason: String,
        recoverable: bool,
    },
    /// System tray operation failed
    SystemTray {
        operation: String,
        reason: String,
        platform: String,
    },
    /// Notification system error
    Notification {
        reason: String,
        platform: String,
        fallback_available: bool,
    },
    /// File association registration failed
    FileAssociation {
        extension: String,
        reason: String,
        platform: String,
        requires_admin: bool,
    },
    /// Drag and drop operation failed
    DragDrop {
        operation: String,
        reason: String,
        file_count: usize,
    },
    /// Cross-platform compatibility issue
    Platform {
        feature: String,
        platform: String,
        reason: String,
        workaround: Option<String>,
    },
    /// Permission denied error
    Permission {
        operation: String,
        required_permission: String,
        reason: String,
    },
}

impl fmt::Display for NativeFeatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NativeFeatureError::FileDialog { operation, reason, .. } => {
                write!(f, "File dialog error during {}: {}", operation, reason)
            }
            NativeFeatureError::SystemTray { operation, reason, platform } => {
                write!(f, "System tray error on {} during {}: {}", platform, operation, reason)
            }
            NativeFeatureError::Notification { reason, platform, .. } => {
                write!(f, "Notification error on {}: {}", platform, reason)
            }
            NativeFeatureError::FileAssociation { extension, reason, platform, .. } => {
                write!(f, "File association error for .{} on {}: {}", extension, platform, reason)
            }
            NativeFeatureError::DragDrop { operation, reason, file_count } => {
                write!(f, "Drag & drop error during {} ({} files): {}", operation, file_count, reason)
            }
            NativeFeatureError::Platform { feature, platform, reason, .. } => {
                write!(f, "Platform compatibility error: {} not supported on {}: {}", feature, platform, reason)
            }
            NativeFeatureError::Permission { operation, required_permission, reason } => {
                write!(f, "Permission error during {}: {} required - {}", operation, required_permission, reason)
            }
        }
    }
}

impl std::error::Error for NativeFeatureError {}

/// Result type for native feature operations
pub type NativeResult<T> = Result<T, NativeFeatureError>;

/// Error recovery strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecovery {
    pub can_retry: bool,
    pub retry_delay_ms: Option<u64>,
    pub alternative_action: Option<String>,
    pub user_message: String,
    pub technical_details: Option<String>,
}

impl NativeFeatureError {
    /// Get platform-specific error recovery information
    pub fn recovery_info(&self) -> ErrorRecovery {
        match self {
            NativeFeatureError::FileDialog { recoverable, .. } => ErrorRecovery {
                can_retry: *recoverable,
                retry_delay_ms: None,
                alternative_action: Some("Try using drag & drop instead".to_string()),
                user_message: "File dialog failed. You can drag files directly onto the window.".to_string(),
                technical_details: Some(self.to_string()),
            },
            NativeFeatureError::SystemTray { platform, .. } => ErrorRecovery {
                can_retry: true,
                retry_delay_ms: Some(1000),
                alternative_action: Some("Use window menu instead".to_string()),
                user_message: match platform.as_str() {
                    "windows" => "System tray unavailable. Use the window menu for actions.".to_string(),
                    "macos" => "Menu bar icon unavailable. Use the dock menu for actions.".to_string(),
                    "linux" => "System tray unavailable. Check if notification daemon is running.".to_string(),
                    _ => "System tray unavailable. Use the window menu for actions.".to_string(),
                },
                technical_details: Some(self.to_string()),
            },
            NativeFeatureError::Notification { fallback_available, .. } => ErrorRecovery {
                can_retry: *fallback_available,
                retry_delay_ms: Some(500),
                alternative_action: Some("Show in-app notification instead".to_string()),
                user_message: "Desktop notifications unavailable. Using in-app notifications.".to_string(),
                technical_details: Some(self.to_string()),
            },
            NativeFeatureError::FileAssociation { requires_admin, .. } => ErrorRecovery {
                can_retry: false,
                retry_delay_ms: None,
                alternative_action: if *requires_admin {
                    Some("Run as administrator and try again".to_string())
                } else {
                    Some("Set file associations manually in system settings".to_string())
                },
                user_message: if *requires_admin {
                    "File associations require administrator privileges. Files can still be opened via drag & drop.".to_string()
                } else {
                    "Could not register file associations. Files can still be opened via drag & drop.".to_string()
                },
                technical_details: Some(self.to_string()),
            },
            NativeFeatureError::DragDrop { file_count, .. } => ErrorRecovery {
                can_retry: true,
                retry_delay_ms: None,
                alternative_action: Some("Use file dialog instead".to_string()),
                user_message: format!(
                    "Could not process {} dropped file{}. Use the import buttons instead.",
                    file_count,
                    if *file_count == 1 { "" } else { "s" }
                ),
                technical_details: Some(self.to_string()),
            },
            NativeFeatureError::Platform { workaround, .. } => ErrorRecovery {
                can_retry: false,
                retry_delay_ms: None,
                alternative_action: workaround.clone(),
                user_message: workaround
                    .as_ref()
                    .map(|w| format!("Feature not available on this platform. Try: {}", w))
                    .unwrap_or_else(|| "Feature not available on this platform.".to_string()),
                technical_details: Some(self.to_string()),
            },
            NativeFeatureError::Permission { required_permission, .. } => ErrorRecovery {
                can_retry: false,
                retry_delay_ms: None,
                alternative_action: Some(format!("Grant {} permission in system settings", required_permission)),
                user_message: format!("Permission required: {}. Please check system settings.", required_permission),
                technical_details: Some(self.to_string()),
            },
        }
    }

    /// Check if this error is critical (should stop the app) or recoverable
    pub fn is_critical(&self) -> bool {
        match self {
            NativeFeatureError::FileDialog { .. } => false,
            NativeFeatureError::SystemTray { .. } => false,
            NativeFeatureError::Notification { .. } => false,
            NativeFeatureError::FileAssociation { .. } => false,
            NativeFeatureError::DragDrop { .. } => false,
            NativeFeatureError::Platform { .. } => false,
            NativeFeatureError::Permission { .. } => false,
        }
    }

    /// Get platform-specific error code
    pub fn error_code(&self) -> String {
        match self {
            NativeFeatureError::FileDialog { .. } => "NF001".to_string(),
            NativeFeatureError::SystemTray { .. } => "NF002".to_string(),
            NativeFeatureError::Notification { .. } => "NF003".to_string(),
            NativeFeatureError::FileAssociation { .. } => "NF004".to_string(),
            NativeFeatureError::DragDrop { .. } => "NF005".to_string(),
            NativeFeatureError::Platform { .. } => "NF006".to_string(),
            NativeFeatureError::Permission { .. } => "NF007".to_string(),
        }
    }
}

/// Platform compatibility checker
pub struct PlatformChecker;

impl PlatformChecker {
    /// Check if a feature is supported on the current platform
    pub fn is_supported(feature: &str) -> bool {
        match feature {
            "system_tray" => cfg!(any(target_os = "windows", target_os = "macos", target_os = "linux")),
            "file_associations" => cfg!(any(target_os = "windows", target_os = "macos", target_os = "linux")),
            "native_notifications" => cfg!(any(target_os = "windows", target_os = "macos", target_os = "linux")),
            "drag_drop" => true, // Supported on all platforms through Tauri
            "file_dialogs" => true, // Supported on all platforms through Tauri
            _ => false,
        }
    }

    /// Get current platform name
    pub fn current_platform() -> String {
        if cfg!(target_os = "windows") {
            "windows".to_string()
        } else if cfg!(target_os = "macos") {
            "macos".to_string()
        } else if cfg!(target_os = "linux") {
            "linux".to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// Get platform-specific limitations
    pub fn get_limitations() -> Vec<String> {
        let mut limitations = Vec::new();

        if cfg!(target_os = "linux") {
            limitations.push("System tray requires compatible desktop environment".to_string());
            limitations.push("File associations may require manual desktop file installation".to_string());
        }

        if cfg!(target_os = "macos") {
            limitations.push("File associations require app bundle configuration".to_string());
            limitations.push("Some features may require additional permissions".to_string());
        }

        if cfg!(target_os = "windows") {
            limitations.push("File associations may require administrator privileges".to_string());
        }

        limitations
    }
}

/// Error reporting and telemetry
pub struct ErrorReporter;

impl ErrorReporter {
    /// Log error with appropriate severity
    pub fn log_error(error: &NativeFeatureError) {
        if error.is_critical() {
            log::error!("Critical native feature error [{}]: {}", error.error_code(), error);
        } else {
            log::warn!("Native feature error [{}]: {}", error.error_code(), error);
        }
    }

    /// Create user-friendly error message
    pub fn user_message(error: &NativeFeatureError) -> String {
        error.recovery_info().user_message
    }

    /// Check if error should be reported to the user
    pub fn should_show_to_user(error: &NativeFeatureError) -> bool {
        match error {
            NativeFeatureError::Platform { .. } => true, // Always inform about platform limitations
            NativeFeatureError::Permission { .. } => true, // Always inform about permission issues
            _ => !error.is_critical(), // Show non-critical errors as warnings
        }
    }
}

/// Utility macros for error handling
#[macro_export]
macro_rules! native_error {
    (file_dialog, $operation:expr, $reason:expr) => {
        NativeFeatureError::FileDialog {
            operation: $operation.to_string(),
            reason: $reason.to_string(),
            recoverable: true,
        }
    };
    
    (system_tray, $operation:expr, $reason:expr) => {
        NativeFeatureError::SystemTray {
            operation: $operation.to_string(),
            reason: $reason.to_string(),
            platform: PlatformChecker::current_platform(),
        }
    };
    
    (notification, $reason:expr) => {
        NativeFeatureError::Notification {
            reason: $reason.to_string(),
            platform: PlatformChecker::current_platform(),
            fallback_available: true,
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_recovery_info() {
        let error = NativeFeatureError::FileDialog {
            operation: "open".to_string(),
            reason: "user cancelled".to_string(),
            recoverable: true,
        };
        
        let recovery = error.recovery_info();
        assert!(recovery.can_retry);
        assert!(recovery.alternative_action.is_some());
    }

    #[test]
    fn test_platform_checker() {
        assert!(PlatformChecker::is_supported("drag_drop"));
        assert!(PlatformChecker::is_supported("file_dialogs"));
        assert!(!PlatformChecker::is_supported("nonexistent_feature"));
    }

    #[test]
    fn test_error_codes() {
        let error = NativeFeatureError::FileDialog {
            operation: "test".to_string(),
            reason: "test".to_string(),
            recoverable: true,
        };
        
        assert_eq!(error.error_code(), "NF001");
        assert!(!error.is_critical());
    }
}