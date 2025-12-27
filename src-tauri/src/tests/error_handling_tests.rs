#[cfg(test)]
mod tests {
    use crate::error_handling::*;
    use std::collections::HashMap;

    #[test]
    fn test_native_feature_error_display() {
        let file_dialog_error = NativeFeatureError::FileDialog {
            operation: "open".to_string(),
            reason: "user cancelled".to_string(),
            recoverable: true,
        };
        let display = format!("{}", file_dialog_error);
        assert!(display.contains("File dialog error during open"));
        assert!(display.contains("user cancelled"));

        let system_tray_error = NativeFeatureError::SystemTray {
            operation: "create".to_string(),
            reason: "service not available".to_string(),
            platform: "linux".to_string(),
        };
        let display = format!("{}", system_tray_error);
        assert!(display.contains("System tray error on linux during create"));
        assert!(display.contains("service not available"));

        let notification_error = NativeFeatureError::Notification {
            reason: "notification daemon not running".to_string(),
            platform: "linux".to_string(),
            fallback_available: true,
        };
        let display = format!("{}", notification_error);
        assert!(display.contains("Notification error on linux"));
        assert!(display.contains("notification daemon not running"));

        let file_association_error = NativeFeatureError::FileAssociation {
            extension: "pdf".to_string(),
            reason: "registry access denied".to_string(),
            platform: "windows".to_string(),
            requires_admin: true,
        };
        let display = format!("{}", file_association_error);
        assert!(display.contains("File association error for .pdf on windows"));
        assert!(display.contains("registry access denied"));

        let drag_drop_error = NativeFeatureError::DragDrop {
            operation: "process".to_string(),
            reason: "invalid file format".to_string(),
            file_count: 3,
        };
        let display = format!("{}", drag_drop_error);
        assert!(display.contains("Drag & drop error during process (3 files)"));
        assert!(display.contains("invalid file format"));

        let platform_error = NativeFeatureError::Platform {
            feature: "system_tray".to_string(),
            platform: "unsupported_os".to_string(),
            reason: "not implemented".to_string(),
            workaround: Some("use menu bar".to_string()),
        };
        let display = format!("{}", platform_error);
        assert!(display.contains("Platform compatibility error"));
        assert!(display.contains("system_tray not supported on unsupported_os"));
        assert!(display.contains("not implemented"));

        let permission_error = NativeFeatureError::Permission {
            operation: "write_file".to_string(),
            required_permission: "filesystem_write".to_string(),
            reason: "access denied".to_string(),
        };
        let display = format!("{}", permission_error);
        assert!(display.contains("Permission error during write_file"));
        assert!(display.contains("filesystem_write required"));
        assert!(display.contains("access denied"));
    }

    #[test]
    fn test_error_recovery_info_file_dialog() {
        let error = NativeFeatureError::FileDialog {
            operation: "save".to_string(),
            reason: "invalid path".to_string(),
            recoverable: true,
        };
        
        let recovery = error.recovery_info();
        assert!(recovery.can_retry);
        assert!(recovery.alternative_action.is_some());
        assert_eq!(recovery.alternative_action.unwrap(), "Try using drag & drop instead");
        assert!(recovery.user_message.contains("drag files directly"));
        assert!(recovery.technical_details.is_some());
    }

    #[test]
    fn test_error_recovery_info_file_dialog_not_recoverable() {
        let error = NativeFeatureError::FileDialog {
            operation: "save".to_string(),
            reason: "critical system error".to_string(),
            recoverable: false,
        };
        
        let recovery = error.recovery_info();
        assert!(!recovery.can_retry);
    }

    #[test]
    fn test_error_recovery_info_system_tray() {
        let platforms = vec!["windows", "macos", "linux", "unknown"];
        
        for platform in platforms {
            let error = NativeFeatureError::SystemTray {
                operation: "create".to_string(),
                reason: "service unavailable".to_string(),
                platform: platform.to_string(),
            };
            
            let recovery = error.recovery_info();
            assert!(recovery.can_retry);
            assert_eq!(recovery.retry_delay_ms, Some(1000));
            assert_eq!(recovery.alternative_action.unwrap(), "Use window menu instead");
            assert!(recovery.technical_details.is_some());
            
            match platform {
                "windows" => assert!(recovery.user_message.contains("window menu")),
                "macos" => assert!(recovery.user_message.contains("dock menu")),
                "linux" => assert!(recovery.user_message.contains("notification daemon")),
                _ => assert!(recovery.user_message.contains("window menu")),
            }
        }
    }

    #[test]
    fn test_error_recovery_info_notification() {
        let error_with_fallback = NativeFeatureError::Notification {
            reason: "system busy".to_string(),
            platform: "windows".to_string(),
            fallback_available: true,
        };
        
        let recovery = error_with_fallback.recovery_info();
        assert!(recovery.can_retry);
        assert_eq!(recovery.retry_delay_ms, Some(500));
        assert_eq!(recovery.alternative_action.unwrap(), "Show in-app notification instead");
        assert!(recovery.user_message.contains("in-app notifications"));

        let error_no_fallback = NativeFeatureError::Notification {
            reason: "system busy".to_string(),
            platform: "windows".to_string(),
            fallback_available: false,
        };
        
        let recovery_no_fallback = error_no_fallback.recovery_info();
        assert!(!recovery_no_fallback.can_retry);
    }

    #[test]
    fn test_error_recovery_info_file_association() {
        let error_requires_admin = NativeFeatureError::FileAssociation {
            extension: "json".to_string(),
            reason: "registry locked".to_string(),
            platform: "windows".to_string(),
            requires_admin: true,
        };
        
        let recovery = error_requires_admin.recovery_info();
        assert!(!recovery.can_retry);
        assert!(recovery.alternative_action.unwrap().contains("administrator"));
        assert!(recovery.user_message.contains("administrator privileges"));

        let error_no_admin = NativeFeatureError::FileAssociation {
            extension: "json".to_string(),
            reason: "registry corrupted".to_string(),
            platform: "windows".to_string(),
            requires_admin: false,
        };
        
        let recovery_no_admin = error_no_admin.recovery_info();
        assert!(!recovery_no_admin.can_retry);
        assert!(recovery_no_admin.alternative_action.unwrap().contains("manually"));
        assert!(recovery_no_admin.user_message.contains("drag & drop"));
    }

    #[test]
    fn test_error_recovery_info_drag_drop() {
        let single_file_error = NativeFeatureError::DragDrop {
            operation: "import".to_string(),
            reason: "unsupported format".to_string(),
            file_count: 1,
        };
        
        let recovery = single_file_error.recovery_info();
        assert!(recovery.can_retry);
        assert_eq!(recovery.alternative_action.unwrap(), "Use file dialog instead");
        assert!(recovery.user_message.contains("1 dropped file"));
        assert!(!recovery.user_message.contains("files"));

        let multiple_files_error = NativeFeatureError::DragDrop {
            operation: "import".to_string(),
            reason: "too large".to_string(),
            file_count: 5,
        };
        
        let recovery_multiple = multiple_files_error.recovery_info();
        assert!(recovery_multiple.can_retry);
        assert!(recovery_multiple.user_message.contains("5 dropped files"));
    }

    #[test]
    fn test_error_recovery_info_platform() {
        let error_with_workaround = NativeFeatureError::Platform {
            feature: "native_dialogs".to_string(),
            platform: "web".to_string(),
            reason: "not available in browser".to_string(),
            workaround: Some("use HTML file input".to_string()),
        };
        
        let recovery = error_with_workaround.recovery_info();
        assert!(!recovery.can_retry);
        assert_eq!(recovery.alternative_action, Some("use HTML file input".to_string()));
        assert!(recovery.user_message.contains("use HTML file input"));

        let error_no_workaround = NativeFeatureError::Platform {
            feature: "system_tray".to_string(),
            platform: "mobile".to_string(),
            reason: "not supported".to_string(),
            workaround: None,
        };
        
        let recovery_no_workaround = error_no_workaround.recovery_info();
        assert!(!recovery_no_workaround.can_retry);
        assert!(recovery_no_workaround.alternative_action.is_none());
        assert!(recovery_no_workaround.user_message.contains("not available"));
    }

    #[test]
    fn test_error_recovery_info_permission() {
        let error = NativeFeatureError::Permission {
            operation: "create_file".to_string(),
            required_permission: "storage_write".to_string(),
            reason: "user denied".to_string(),
        };
        
        let recovery = error.recovery_info();
        assert!(!recovery.can_retry);
        assert_eq!(recovery.alternative_action.unwrap(), "Grant storage_write permission in system settings");
        assert!(recovery.user_message.contains("Permission required: storage_write"));
    }

    #[test]
    fn test_error_criticality() {
        let errors = vec![
            NativeFeatureError::FileDialog {
                operation: "open".to_string(),
                reason: "cancelled".to_string(),
                recoverable: true,
            },
            NativeFeatureError::SystemTray {
                operation: "create".to_string(),
                reason: "unavailable".to_string(),
                platform: "linux".to_string(),
            },
            NativeFeatureError::Notification {
                reason: "blocked".to_string(),
                platform: "windows".to_string(),
                fallback_available: true,
            },
            NativeFeatureError::FileAssociation {
                extension: "txt".to_string(),
                reason: "denied".to_string(),
                platform: "macos".to_string(),
                requires_admin: false,
            },
            NativeFeatureError::DragDrop {
                operation: "drop".to_string(),
                reason: "invalid".to_string(),
                file_count: 2,
            },
            NativeFeatureError::Platform {
                feature: "test".to_string(),
                platform: "test".to_string(),
                reason: "test".to_string(),
                workaround: None,
            },
            NativeFeatureError::Permission {
                operation: "test".to_string(),
                required_permission: "test".to_string(),
                reason: "test".to_string(),
            },
        ];

        // All errors should be non-critical (recoverable)
        for error in errors {
            assert!(!error.is_critical());
        }
    }

    #[test]
    fn test_error_codes() {
        let test_cases = vec![
            (NativeFeatureError::FileDialog {
                operation: "test".to_string(),
                reason: "test".to_string(),
                recoverable: true,
            }, "NF001"),
            (NativeFeatureError::SystemTray {
                operation: "test".to_string(),
                reason: "test".to_string(),
                platform: "test".to_string(),
            }, "NF002"),
            (NativeFeatureError::Notification {
                reason: "test".to_string(),
                platform: "test".to_string(),
                fallback_available: true,
            }, "NF003"),
            (NativeFeatureError::FileAssociation {
                extension: "test".to_string(),
                reason: "test".to_string(),
                platform: "test".to_string(),
                requires_admin: false,
            }, "NF004"),
            (NativeFeatureError::DragDrop {
                operation: "test".to_string(),
                reason: "test".to_string(),
                file_count: 1,
            }, "NF005"),
            (NativeFeatureError::Platform {
                feature: "test".to_string(),
                platform: "test".to_string(),
                reason: "test".to_string(),
                workaround: None,
            }, "NF006"),
            (NativeFeatureError::Permission {
                operation: "test".to_string(),
                required_permission: "test".to_string(),
                reason: "test".to_string(),
            }, "NF007"),
        ];

        for (error, expected_code) in test_cases {
            assert_eq!(error.error_code(), expected_code);
        }
    }

    #[test]
    fn test_platform_checker_support() {
        // Test supported features
        assert!(PlatformChecker::is_supported("system_tray"));
        assert!(PlatformChecker::is_supported("file_associations"));
        assert!(PlatformChecker::is_supported("native_notifications"));
        assert!(PlatformChecker::is_supported("drag_drop"));
        assert!(PlatformChecker::is_supported("file_dialogs"));

        // Test unsupported features
        assert!(!PlatformChecker::is_supported("nonexistent_feature"));
        assert!(!PlatformChecker::is_supported(""));
        assert!(!PlatformChecker::is_supported("random_string"));
    }

    #[test]
    fn test_platform_checker_current_platform() {
        let platform = PlatformChecker::current_platform();
        
        // Should return one of the known platforms or "unknown"
        assert!(matches!(platform.as_str(), "windows" | "macos" | "linux" | "unknown"));
        assert!(!platform.is_empty());
    }

    #[test]
    fn test_platform_checker_limitations() {
        let limitations = PlatformChecker::get_limitations();
        
        // Should return a vector (may be empty on some platforms)
        assert!(limitations.len() >= 0);
        
        // All limitations should be non-empty strings
        for limitation in limitations {
            assert!(!limitation.is_empty());
        }
    }

    #[test]
    fn test_error_reporter_logging() {
        // Test that error logging doesn't panic
        let error = NativeFeatureError::FileDialog {
            operation: "test".to_string(),
            reason: "test error".to_string(),
            recoverable: true,
        };
        
        ErrorReporter::log_error(&error); // Should not panic
    }

    #[test]
    fn test_error_reporter_user_message() {
        let error = NativeFeatureError::Notification {
            reason: "service unavailable".to_string(),
            platform: "linux".to_string(),
            fallback_available: true,
        };
        
        let user_message = ErrorReporter::user_message(&error);
        assert!(!user_message.is_empty());
        assert!(user_message.contains("in-app"));
    }

    #[test]
    fn test_error_reporter_should_show_to_user() {
        let platform_error = NativeFeatureError::Platform {
            feature: "test".to_string(),
            platform: "test".to_string(),
            reason: "test".to_string(),
            workaround: None,
        };
        assert!(ErrorReporter::should_show_to_user(&platform_error));

        let permission_error = NativeFeatureError::Permission {
            operation: "test".to_string(),
            required_permission: "test".to_string(),
            reason: "test".to_string(),
        };
        assert!(ErrorReporter::should_show_to_user(&permission_error));

        let file_dialog_error = NativeFeatureError::FileDialog {
            operation: "test".to_string(),
            reason: "test".to_string(),
            recoverable: true,
        };
        assert!(ErrorReporter::should_show_to_user(&file_dialog_error)); // Non-critical
    }

    #[test]
    fn test_native_error_macro_file_dialog() {
        let error = native_error!(file_dialog, "open", "user cancelled");
        
        match error {
            NativeFeatureError::FileDialog { operation, reason, recoverable } => {
                assert_eq!(operation, "open");
                assert_eq!(reason, "user cancelled");
                assert!(recoverable);
            },
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_native_error_macro_system_tray() {
        let error = native_error!(system_tray, "create", "service not found");
        
        match error {
            NativeFeatureError::SystemTray { operation, reason, platform } => {
                assert_eq!(operation, "create");
                assert_eq!(reason, "service not found");
                assert!(!platform.is_empty());
            },
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_native_error_macro_notification() {
        let error = native_error!(notification, "permission denied");
        
        match error {
            NativeFeatureError::Notification { reason, platform, fallback_available } => {
                assert_eq!(reason, "permission denied");
                assert!(!platform.is_empty());
                assert!(fallback_available);
            },
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_error_serialization() {
        let errors = vec![
            NativeFeatureError::FileDialog {
                operation: "test".to_string(),
                reason: "test".to_string(),
                recoverable: true,
            },
            NativeFeatureError::SystemTray {
                operation: "test".to_string(),
                reason: "test".to_string(),
                platform: "test".to_string(),
            },
            NativeFeatureError::Notification {
                reason: "test".to_string(),
                platform: "test".to_string(),
                fallback_available: false,
            },
        ];

        for error in errors {
            // Test serialization
            let serialized = serde_json::to_string(&error).unwrap();
            assert!(!serialized.is_empty());
            
            // Test deserialization
            let deserialized: NativeFeatureError = serde_json::from_str(&serialized).unwrap();
            
            // Verify error codes match (simple way to verify they're the same type)
            assert_eq!(error.error_code(), deserialized.error_code());
        }
    }

    #[test]
    fn test_error_recovery_serialization() {
        let recovery = ErrorRecovery {
            can_retry: true,
            retry_delay_ms: Some(1000),
            alternative_action: Some("Try again".to_string()),
            user_message: "Please try again".to_string(),
            technical_details: Some("Error details".to_string()),
        };

        // Test serialization
        let serialized = serde_json::to_string(&recovery).unwrap();
        assert!(!serialized.is_empty());
        
        // Test deserialization
        let deserialized: ErrorRecovery = serde_json::from_str(&serialized).unwrap();
        assert_eq!(recovery.can_retry, deserialized.can_retry);
        assert_eq!(recovery.retry_delay_ms, deserialized.retry_delay_ms);
        assert_eq!(recovery.alternative_action, deserialized.alternative_action);
        assert_eq!(recovery.user_message, deserialized.user_message);
        assert_eq!(recovery.technical_details, deserialized.technical_details);
    }

    #[test]
    fn test_comprehensive_error_scenarios() {
        // Test file dialog scenarios
        let scenarios = vec![
            ("save", "disk full", true),
            ("open", "permission denied", false),
            ("select_folder", "network error", true),
        ];

        for (operation, reason, recoverable) in scenarios {
            let error = NativeFeatureError::FileDialog {
                operation: operation.to_string(),
                reason: reason.to_string(),
                recoverable,
            };
            
            let recovery = error.recovery_info();
            assert_eq!(recovery.can_retry, recoverable);
            assert!(!recovery.user_message.is_empty());
        }

        // Test drag and drop with different file counts
        for file_count in [0, 1, 2, 10, 100] {
            let error = NativeFeatureError::DragDrop {
                operation: "process".to_string(),
                reason: "invalid format".to_string(),
                file_count,
            };
            
            let recovery = error.recovery_info();
            if file_count == 1 {
                assert!(!recovery.user_message.contains("files"));
            } else {
                assert!(recovery.user_message.contains("files"));
            }
        }
    }

    #[test]
    fn test_error_chaining() {
        // Test that errors can be converted to strings for error chaining
        let error = NativeFeatureError::Platform {
            feature: "test_feature".to_string(),
            platform: "test_platform".to_string(),
            reason: "not supported".to_string(),
            workaround: Some("use alternative".to_string()),
        };

        let error_string = error.to_string();
        let result: Result<(), String> = Err(error_string);
        
        match result {
            Err(e) => assert!(e.contains("test_feature")),
            Ok(_) => panic!("Should be error"),
        }
    }

    #[test]
    fn test_native_result_type() {
        // Test successful result
        let success: NativeResult<i32> = Ok(42);
        assert_eq!(success.unwrap(), 42);

        // Test error result
        let error = NativeFeatureError::FileDialog {
            operation: "test".to_string(),
            reason: "test".to_string(),
            recoverable: true,
        };
        let failure: NativeResult<i32> = Err(error);
        assert!(failure.is_err());
    }
}
