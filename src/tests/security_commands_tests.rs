#[cfg(test)]
mod tests {
    use crate::security_commands::*;
    use std::collections::HashMap;
    use std::sync::{Arc, atomic::{AtomicBool, AtomicU32, Ordering}};
    use std::time::{SystemTime, Duration};
    use tokio::sync::RwLock;
    use uuid::Uuid;
    use serde_json::json;

    // Mock security structures for testing
    #[derive(Debug, Clone)]
    struct MockSecurityManager {
        initialized: Arc<AtomicBool>,
        sessions: Arc<RwLock<HashMap<Uuid, MockSession>>>,
        credentials: Arc<RwLock<HashMap<String, MockCredential>>>,
        security_events: Arc<RwLock<Vec<MockSecurityEvent>>>,
        alerts: Arc<RwLock<Vec<MockSecurityAlert>>>,
        validation_count: Arc<AtomicU32>,
        process_count: Arc<AtomicU32>,
    }

    #[derive(Debug, Clone)]
    struct MockSession {
        id: Uuid,
        permissions: Vec<String>,
        created_at: SystemTime,
        expires_at: SystemTime,
    }

    #[derive(Debug, Clone)]
    struct MockCredential {
        service: String,
        account: String,
        secret: String,
        additional_data: HashMap<String, String>,
        expires_at: Option<SystemTime>,
        created_at: SystemTime,
    }

    #[derive(Debug, Clone)]
    struct MockSecurityEvent {
        event_type: String,
        severity: String,
        message: String,
        details: serde_json::Value,
        session_id: Option<Uuid>,
        timestamp: SystemTime,
    }

    #[derive(Debug, Clone)]
    struct MockSecurityAlert {
        alert_type: String,
        severity: String,
        message: String,
        timestamp: SystemTime,
    }

    #[derive(Debug, Clone)]
    struct MockValidationResult {
        valid: bool,
        sanitized_value: serde_json::Value,
        errors: Vec<MockValidationError>,
        warnings: Vec<String>,
    }

    #[derive(Debug, Clone)]
    struct MockValidationError {
        message: String,
        field: String,
    }

    #[derive(Debug, Clone)]
    struct MockPermissionResult {
        allowed: bool,
        reason: Option<String>,
        conditions: Vec<String>,
    }

    #[derive(Debug, Clone)]
    struct MockSecurityStats {
        total_sessions: u32,
        active_sessions: u32,
        total_events: u32,
        failed_authentications: u32,
        blocked_operations: u32,
    }

    impl MockSecurityManager {
        fn new() -> Self {
            Self {
                initialized: Arc::new(AtomicBool::new(false)),
                sessions: Arc::new(RwLock::new(HashMap::new())),
                credentials: Arc::new(RwLock::new(HashMap::new())),
                security_events: Arc::new(RwLock::new(Vec::new())),
                alerts: Arc::new(RwLock::new(Vec::new())),
                validation_count: Arc::new(AtomicU32::new(0)),
                process_count: Arc::new(AtomicU32::new(0)),
            }
        }

        async fn initialize(&self) -> Result<(), String> {
            self.initialized.store(true, Ordering::Relaxed);
            Ok(())
        }

        async fn create_session(&self, permissions: Vec<String>) -> Result<Uuid, String> {
            if !self.initialized.load(Ordering::Relaxed) {
                return Err("Security manager not initialized".to_string());
            }

            let session_id = Uuid::new_v4();
            let session = MockSession {
                id: session_id,
                permissions,
                created_at: SystemTime::now(),
                expires_at: SystemTime::now() + Duration::from_secs(3600),
            };

            self.sessions.write().await.insert(session_id, session);
            Ok(session_id)
        }

        async fn validate_session_permission(&self, session_id: Uuid, permission: &str) -> Result<bool, String> {
            let sessions = self.sessions.read().await;
            if let Some(session) = sessions.get(&session_id) {
                Ok(session.permissions.contains(&permission.to_string()))
            } else {
                Err("Session not found".to_string())
            }
        }

        async fn validate_input(&self, request: &MockValidationRequest) -> Result<MockValidationResult, String> {
            self.validation_count.fetch_add(1, Ordering::Relaxed);

            // Mock validation logic
            let mut errors = Vec::new();
            let mut warnings = Vec::new();

            let sanitized_value = if request.field_name == "email" {
                if let Some(value_str) = request.value.as_str() {
                    if !value_str.contains('@') {
                        errors.push(MockValidationError {
                            message: "Invalid email format".to_string(),
                            field: request.field_name.clone(),
                        });
                    }
                    json!(value_str.to_lowercase())
                } else {
                    json!(null)
                }
            } else if request.field_name == "password" {
                if let Some(value_str) = request.value.as_str() {
                    if value_str.len() < 8 {
                        errors.push(MockValidationError {
                            message: "Password too short".to_string(),
                            field: request.field_name.clone(),
                        });
                    }
                    if !value_str.chars().any(|c| c.is_numeric()) {
                        warnings.push("Password should contain numbers".to_string());
                    }
                }
                json!("***redacted***")
            } else {
                request.value.clone()
            };

            Ok(MockValidationResult {
                valid: errors.is_empty(),
                sanitized_value,
                errors,
                warnings,
            })
        }

        async fn sanitize_string(&self, input: &str) -> String {
            // Simple sanitization - remove HTML tags and trim
            input.replace('<', "&lt;").replace('>', "&gt;").trim().to_string()
        }

        async fn validate_path(&self, path: &str) -> Result<MockValidationResult, String> {
            let mut errors = Vec::new();
            let mut warnings = Vec::new();

            // Check for dangerous patterns
            if path.contains("..") {
                errors.push(MockValidationError {
                    message: "Path traversal detected".to_string(),
                    field: "path".to_string(),
                });
            }

            if path.starts_with('/') && !path.starts_with("/safe/") {
                warnings.push("Path outside safe directory".to_string());
            }

            let sanitized_path = path.replace("..", "").replace("//", "/");

            Ok(MockValidationResult {
                valid: errors.is_empty(),
                sanitized_value: json!(sanitized_path),
                errors,
                warnings,
            })
        }

        async fn validate_command(&self, command: &str, args: &[String]) -> Result<MockValidationResult, String> {
            let mut errors = Vec::new();
            let mut warnings = Vec::new();

            // Blacklist dangerous commands
            let dangerous_commands = ["rm", "del", "format", "sudo", "su"];
            if dangerous_commands.contains(&command) {
                errors.push(MockValidationError {
                    message: "Command not allowed".to_string(),
                    field: "command".to_string(),
                });
            }

            // Check for dangerous arguments
            for arg in args {
                if arg.contains("--force") || arg.contains("-rf") {
                    warnings.push("Potentially dangerous argument detected".to_string());
                }
            }

            Ok(MockValidationResult {
                valid: errors.is_empty(),
                sanitized_value: json!({
                    "command": command,
                    "args": args
                }),
                errors,
                warnings,
            })
        }

        async fn store_credential(&self, service: &str, account: &str, secret: &str, additional_data: HashMap<String, String>, expires_at: Option<SystemTime>) -> Result<String, String> {
            let key = format!("{}:{}", service, account);
            let credential = MockCredential {
                service: service.to_string(),
                account: account.to_string(),
                secret: secret.to_string(),
                additional_data,
                expires_at,
                created_at: SystemTime::now(),
            };

            self.credentials.write().await.insert(key.clone(), credential);
            Ok(key)
        }

        async fn retrieve_credential(&self, service: &str, account: &str) -> Result<MockCredential, String> {
            let key = format!("{}:{}", service, account);
            let credentials = self.credentials.read().await;
            credentials.get(&key)
                .cloned()
                .ok_or_else(|| "Credential not found".to_string())
        }

        async fn delete_credential(&self, service: &str, account: &str) -> Result<(), String> {
            let key = format!("{}:{}", service, account);
            let mut credentials = self.credentials.write().await;
            credentials.remove(&key)
                .ok_or_else(|| "Credential not found".to_string())?;
            Ok(())
        }

        async fn check_permission(&self, request: &MockPermissionRequest) -> Result<MockPermissionResult, String> {
            // Mock permission logic
            let allowed = match request.action.as_str() {
                "read" => true,
                "write" => request.user_id == "admin",
                "delete" => request.user_id == "admin" && request.context.get("confirmed").is_some(),
                _ => false,
            };

            Ok(MockPermissionResult {
                allowed,
                reason: if allowed { None } else { Some("Insufficient permissions".to_string()) },
                conditions: vec!["authenticated".to_string()],
            })
        }

        async fn create_sandboxed_process(&self, command: &str, args: &[String], _working_dir: Option<&str>) -> Result<Uuid, String> {
            // Simple validation
            if command.is_empty() {
                return Err("Command cannot be empty".to_string());
            }

            let process_id = Uuid::new_v4();
            self.process_count.fetch_add(1, Ordering::Relaxed);
            Ok(process_id)
        }

        async fn terminate_process(&self, _process_id: Uuid) -> Result<(), String> {
            self.process_count.fetch_sub(1, Ordering::Relaxed);
            Ok(())
        }

        async fn get_process_status(&self, _process_id: Uuid) -> Result<MockProcessStatus, String> {
            Ok(MockProcessStatus {
                running: true,
                exit_code: None,
                cpu_usage: 15.5,
                memory_usage: 64.0,
            })
        }

        async fn log_security_event(&self, event_type: &str, severity: &str, message: String, details: serde_json::Value, session_id: Option<Uuid>) {
            let event = MockSecurityEvent {
                event_type: event_type.to_string(),
                severity: severity.to_string(),
                message,
                details,
                session_id,
                timestamp: SystemTime::now(),
            };
            self.security_events.write().await.push(event);
        }

        async fn get_security_stats(&self) -> MockSecurityStats {
            let sessions = self.sessions.read().await;
            let events = self.security_events.read().await;

            MockSecurityStats {
                total_sessions: sessions.len() as u32,
                active_sessions: sessions.len() as u32, // Mock - all are active
                total_events: events.len() as u32,
                failed_authentications: 0,
                blocked_operations: 0,
            }
        }

        async fn get_recent_alerts(&self, limit: usize) -> Vec<MockSecurityAlert> {
            let alerts = self.alerts.read().await;
            let len = alerts.len();
            let start = if len > limit { len - limit } else { 0 };
            alerts[start..].to_vec()
        }

        async fn generate_random_string(&self, length: usize) -> String {
            "a".repeat(length) // Mock implementation
        }

        async fn hash_sha256(&self, data: &[u8]) -> String {
            format!("sha256:{}", data.len()) // Mock hash
        }

        async fn hash_sha512(&self, data: &[u8]) -> String {
            format!("sha512:{}", data.len()) // Mock hash
        }

        async fn hash_blake3(&self, data: &[u8]) -> String {
            format!("blake3:{}", data.len()) // Mock hash
        }

        async fn cleanup_expired_sessions(&self) -> Result<u32, String> {
            let mut sessions = self.sessions.write().await;
            let now = SystemTime::now();
            let initial_count = sessions.len();
            
            sessions.retain(|_, session| session.expires_at > now);
            
            Ok((initial_count - sessions.len()) as u32)
        }

        async fn cleanup_expired_credentials(&self) -> Result<u32, String> {
            let mut credentials = self.credentials.write().await;
            let now = SystemTime::now();
            let initial_count = credentials.len();
            
            credentials.retain(|_, credential| {
                credential.expires_at.map_or(true, |expires| expires > now)
            });
            
            Ok((initial_count - credentials.len()) as u32)
        }
    }

    #[derive(Debug, Clone)]
    struct MockValidationRequest {
        field_name: String,
        value: serde_json::Value,
        context: HashMap<String, String>,
    }

    #[derive(Debug, Clone)]
    struct MockPermissionRequest {
        user_id: String,
        resource_id: String,
        action: String,
        context: HashMap<String, serde_json::Value>,
    }

    #[derive(Debug, Clone)]
    struct MockProcessStatus {
        running: bool,
        exit_code: Option<i32>,
        cpu_usage: f64,
        memory_usage: f64,
    }

    // Mock state wrapper for testing
    struct MockSecurityManagerState {
        security_manager: Arc<RwLock<Option<MockSecurityManager>>>,
    }

    impl MockSecurityManagerState {
        fn new() -> Self {
            Self {
                security_manager: Arc::new(RwLock::new(None)),
            }
        }

        async fn initialize(&self) -> Result<(), String> {
            let manager = MockSecurityManager::new();
            manager.initialize().await?;
            *self.security_manager.write().await = Some(manager);
            Ok(())
        }

        async fn get_manager(&self) -> Option<MockSecurityManager> {
            self.security_manager.read().await.clone()
        }
    }

    #[tokio::test]
    async fn test_security_manager_state_creation() {
        let state = MockSecurityManagerState::new();
        assert!(state.get_manager().await.is_none());
    }

    #[tokio::test]
    async fn test_security_manager_initialization() {
        let state = MockSecurityManagerState::new();
        
        let result = state.initialize().await;
        assert!(result.is_ok());
        
        let manager = state.get_manager().await;
        assert!(manager.is_some());
        assert!(manager.unwrap().initialized.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn test_session_management() {
        let state = MockSecurityManagerState::new();
        state.initialize().await.unwrap();
        let manager = state.get_manager().await.unwrap();

        // Create session
        let permissions = vec!["read".to_string(), "write".to_string()];
        let session_id = manager.create_session(permissions.clone()).await.unwrap();
        assert_ne!(session_id, Uuid::nil());

        // Validate permissions
        let has_read = manager.validate_session_permission(session_id, "read").await.unwrap();
        assert!(has_read);

        let has_write = manager.validate_session_permission(session_id, "write").await.unwrap();
        assert!(has_write);

        let has_delete = manager.validate_session_permission(session_id, "delete").await.unwrap();
        assert!(!has_delete);

        // Test invalid session
        let invalid_session = Uuid::new_v4();
        let result = manager.validate_session_permission(invalid_session, "read").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Session not found");
    }

    #[tokio::test]
    async fn test_input_validation() {
        let state = MockSecurityManagerState::new();
        state.initialize().await.unwrap();
        let manager = state.get_manager().await.unwrap();

        // Test email validation
        let email_request = MockValidationRequest {
            field_name: "email".to_string(),
            value: json!("test@example.com"),
            context: HashMap::new(),
        };
        let result = manager.validate_input(&email_request).await.unwrap();
        assert!(result.valid);
        assert_eq!(result.sanitized_value, json!("test@example.com"));

        // Test invalid email
        let invalid_email_request = MockValidationRequest {
            field_name: "email".to_string(),
            value: json!("invalid-email"),
            context: HashMap::new(),
        };
        let result = manager.validate_input(&invalid_email_request).await.unwrap();
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].message, "Invalid email format");

        // Test password validation
        let password_request = MockValidationRequest {
            field_name: "password".to_string(),
            value: json!("short"),
            context: HashMap::new(),
        };
        let result = manager.validate_input(&password_request).await.unwrap();
        assert!(!result.valid);
        assert_eq!(result.sanitized_value, json!("***redacted***"));
        assert!(!result.errors.is_empty());

        // Test strong password
        let strong_password_request = MockValidationRequest {
            field_name: "password".to_string(),
            value: json!("strongpassword123"),
            context: HashMap::new(),
        };
        let result = manager.validate_input(&strong_password_request).await.unwrap();
        assert!(result.valid);
        assert_eq!(result.sanitized_value, json!("***redacted***"));
    }

    #[tokio::test]
    async fn test_string_sanitization() {
        let state = MockSecurityManagerState::new();
        state.initialize().await.unwrap();
        let manager = state.get_manager().await.unwrap();

        // Test HTML escaping
        let dangerous_input = "<script>alert('xss')</script>";
        let sanitized = manager.sanitize_string(dangerous_input).await;
        assert_eq!(sanitized, "&lt;script&gt;alert('xss')&lt;/script&gt;");

        // Test whitespace trimming
        let whitespace_input = "  normal text  ";
        let sanitized = manager.sanitize_string(whitespace_input).await;
        assert_eq!(sanitized, "normal text");

        // Test empty string
        let sanitized = manager.sanitize_string("").await;
        assert_eq!(sanitized, "");
    }

    #[tokio::test]
    async fn test_path_validation() {
        let state = MockSecurityManagerState::new();
        state.initialize().await.unwrap();
        let manager = state.get_manager().await.unwrap();

        // Test safe path
        let safe_path = "/safe/documents/file.txt";
        let result = manager.validate_path(safe_path).await.unwrap();
        assert!(result.valid);
        assert_eq!(result.sanitized_value, json!(safe_path));
        assert!(result.warnings.is_empty());

        // Test path traversal
        let dangerous_path = "/safe/../etc/passwd";
        let result = manager.validate_path(dangerous_path).await.unwrap();
        assert!(!result.valid);
        assert_eq!(result.errors[0].message, "Path traversal detected");
        assert_eq!(result.sanitized_value, json!("/safe/etc/passwd"));

        // Test unsafe directory
        let unsafe_path = "/etc/passwd";
        let result = manager.validate_path(unsafe_path).await.unwrap();
        assert!(result.valid); // Valid but has warning
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("outside safe directory"));
    }

    #[tokio::test]
    async fn test_command_validation() {
        let state = MockSecurityManagerState::new();
        state.initialize().await.unwrap();
        let manager = state.get_manager().await.unwrap();

        // Test safe command
        let safe_args = vec!["--version".to_string()];
        let result = manager.validate_command("ls", &safe_args).await.unwrap();
        assert!(result.valid);
        assert!(result.warnings.is_empty());

        // Test dangerous command
        let result = manager.validate_command("rm", &[]).await.unwrap();
        assert!(!result.valid);
        assert_eq!(result.errors[0].message, "Command not allowed");

        // Test dangerous arguments
        let dangerous_args = vec!["-rf".to_string(), "/tmp".to_string()];
        let result = manager.validate_command("rm", &dangerous_args).await.unwrap();
        assert!(!result.valid); // Command itself is blocked
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].contains("dangerous argument"));
    }

    #[tokio::test]
    async fn test_credential_management() {
        let state = MockSecurityManagerState::new();
        state.initialize().await.unwrap();
        let manager = state.get_manager().await.unwrap();

        let service = "test_service";
        let account = "test_user";
        let secret = "secret_password";
        let mut additional_data = HashMap::new();
        additional_data.insert("note".to_string(), "test credential".to_string());

        // Store credential
        let entry_id = manager.store_credential(
            service,
            account,
            secret,
            additional_data.clone(),
            None,
        ).await.unwrap();
        assert!(!entry_id.is_empty());

        // Retrieve credential
        let retrieved = manager.retrieve_credential(service, account).await.unwrap();
        assert_eq!(retrieved.service, service);
        assert_eq!(retrieved.account, account);
        assert_eq!(retrieved.secret, secret);
        assert_eq!(retrieved.additional_data, additional_data);
        assert!(retrieved.expires_at.is_none());

        // Delete credential
        let result = manager.delete_credential(service, account).await;
        assert!(result.is_ok());

        // Verify deletion
        let result = manager.retrieve_credential(service, account).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Credential not found");
    }

    #[tokio::test]
    async fn test_credential_with_expiration() {
        let state = MockSecurityManagerState::new();
        state.initialize().await.unwrap();
        let manager = state.get_manager().await.unwrap();

        let expires_at = SystemTime::now() + Duration::from_secs(3600);
        let _entry_id = manager.store_credential(
            "temp_service",
            "temp_user",
            "temp_secret",
            HashMap::new(),
            Some(expires_at),
        ).await.unwrap();

        let retrieved = manager.retrieve_credential("temp_service", "temp_user").await.unwrap();
        assert!(retrieved.expires_at.is_some());
        assert!(retrieved.expires_at.unwrap() > SystemTime::now());
    }

    #[tokio::test]
    async fn test_permission_checking() {
        let state = MockSecurityManagerState::new();
        state.initialize().await.unwrap();
        let manager = state.get_manager().await.unwrap();

        // Test read permission (should be allowed)
        let read_request = MockPermissionRequest {
            user_id: "user123".to_string(),
            resource_id: "document1".to_string(),
            action: "read".to_string(),
            context: HashMap::new(),
        };
        let result = manager.check_permission(&read_request).await.unwrap();
        assert!(result.allowed);
        assert!(result.reason.is_none());

        // Test write permission for non-admin (should be denied)
        let write_request = MockPermissionRequest {
            user_id: "user123".to_string(),
            resource_id: "document1".to_string(),
            action: "write".to_string(),
            context: HashMap::new(),
        };
        let result = manager.check_permission(&write_request).await.unwrap();
        assert!(!result.allowed);
        assert!(result.reason.is_some());

        // Test write permission for admin (should be allowed)
        let admin_write_request = MockPermissionRequest {
            user_id: "admin".to_string(),
            resource_id: "document1".to_string(),
            action: "write".to_string(),
            context: HashMap::new(),
        };
        let result = manager.check_permission(&admin_write_request).await.unwrap();
        assert!(result.allowed);

        // Test delete with confirmation
        let mut delete_context = HashMap::new();
        delete_context.insert("confirmed".to_string(), json!(true));
        let delete_request = MockPermissionRequest {
            user_id: "admin".to_string(),
            resource_id: "document1".to_string(),
            action: "delete".to_string(),
            context: delete_context,
        };
        let result = manager.check_permission(&delete_request).await.unwrap();
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_sandboxed_process_management() {
        let state = MockSecurityManagerState::new();
        state.initialize().await.unwrap();
        let manager = state.get_manager().await.unwrap();

        // Create process
        let process_id = manager.create_sandboxed_process(
            "echo",
            &["hello".to_string()],
            Some("/tmp"),
        ).await.unwrap();
        assert_ne!(process_id, Uuid::nil());
        assert_eq!(manager.process_count.load(Ordering::Relaxed), 1);

        // Get process status
        let status = manager.get_process_status(process_id).await.unwrap();
        assert!(status.running);
        assert!(status.exit_code.is_none());
        assert!(status.cpu_usage > 0.0);

        // Terminate process
        let result = manager.terminate_process(process_id).await;
        assert!(result.is_ok());
        assert_eq!(manager.process_count.load(Ordering::Relaxed), 0);

        // Test invalid command
        let result = manager.create_sandboxed_process("", &[], None).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Command cannot be empty");
    }

    #[tokio::test]
    async fn test_security_event_logging() {
        let state = MockSecurityManagerState::new();
        state.initialize().await.unwrap();
        let manager = state.get_manager().await.unwrap();

        let session_id = manager.create_session(vec!["read".to_string()]).await.unwrap();

        // Log various types of events
        manager.log_security_event(
            "Authentication",
            "Medium",
            "User logged in".to_string(),
            json!({"user_id": "test_user"}),
            Some(session_id),
        ).await;

        manager.log_security_event(
            "Authorization",
            "High",
            "Permission denied".to_string(),
            json!({"resource": "sensitive_data"}),
            None,
        ).await;

        // Verify events were logged
        let events = manager.security_events.read().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "Authentication");
        assert_eq!(events[0].severity, "Medium");
        assert_eq!(events[0].session_id, Some(session_id));
        assert_eq!(events[1].event_type, "Authorization");
        assert_eq!(events[1].severity, "High");
        assert!(events[1].session_id.is_none());
    }

    #[tokio::test]
    async fn test_security_statistics() {
        let state = MockSecurityManagerState::new();
        state.initialize().await.unwrap();
        let manager = state.get_manager().await.unwrap();

        // Create some sessions and events
        let _session1 = manager.create_session(vec!["read".to_string()]).await.unwrap();
        let _session2 = manager.create_session(vec!["write".to_string()]).await.unwrap();

        manager.log_security_event(
            "Test",
            "Low",
            "Test event".to_string(),
            json!({}),
            None,
        ).await;

        let stats = manager.get_security_stats().await;
        assert_eq!(stats.total_sessions, 2);
        assert_eq!(stats.active_sessions, 2);
        assert_eq!(stats.total_events, 1);
    }

    #[tokio::test]
    async fn test_cryptographic_operations() {
        let state = MockSecurityManagerState::new();
        state.initialize().await.unwrap();
        let manager = state.get_manager().await.unwrap();

        // Test random string generation
        let random_string = manager.generate_random_string(16).await;
        assert_eq!(random_string.len(), 16);
        assert_eq!(random_string, "a".repeat(16)); // Mock implementation

        // Test hashing
        let data = b"test data";
        let sha256_hash = manager.hash_sha256(data).await;
        assert!(sha256_hash.starts_with("sha256:"));
        assert!(sha256_hash.contains(&data.len().to_string()));

        let sha512_hash = manager.hash_sha512(data).await;
        assert!(sha512_hash.starts_with("sha512:"));

        let blake3_hash = manager.hash_blake3(data).await;
        assert!(blake3_hash.starts_with("blake3:"));
    }

    #[tokio::test]
    async fn test_session_and_credential_cleanup() {
        let state = MockSecurityManagerState::new();
        state.initialize().await.unwrap();
        let manager = state.get_manager().await.unwrap();

        // Create sessions (all current sessions are valid in mock)
        let _session1 = manager.create_session(vec!["read".to_string()]).await.unwrap();
        let _session2 = manager.create_session(vec!["write".to_string()]).await.unwrap();

        // Create credentials with and without expiration
        let _cred1 = manager.store_credential(
            "service1",
            "user1",
            "secret1",
            HashMap::new(),
            None, // No expiration
        ).await.unwrap();

        let _cred2 = manager.store_credential(
            "service2",
            "user2",
            "secret2",
            HashMap::new(),
            Some(SystemTime::now() + Duration::from_secs(3600)), // Future expiration
        ).await.unwrap();

        // Cleanup (no expired items in this test)
        let expired_sessions = manager.cleanup_expired_sessions().await.unwrap();
        let expired_credentials = manager.cleanup_expired_credentials().await.unwrap();

        assert_eq!(expired_sessions, 0); // No expired sessions
        assert_eq!(expired_credentials, 0); // No expired credentials
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let state = Arc::new(MockSecurityManagerState::new());
        state.initialize().await.unwrap();

        let mut handles = Vec::new();

        // Create multiple sessions concurrently
        for i in 0..10 {
            let state_clone = state.clone();
            let handle = tokio::spawn(async move {
                let manager = state_clone.get_manager().await.unwrap();
                manager.create_session(vec![format!("permission_{}", i)]).await
            });
            handles.push(handle);
        }

        // Wait for all sessions to be created
        let mut success_count = 0;
        for handle in handles {
            if let Ok(result) = handle.await {
                if result.is_ok() {
                    success_count += 1;
                }
            }
        }

        assert_eq!(success_count, 10);

        // Verify all sessions were created
        let manager = state.get_manager().await.unwrap();
        let stats = manager.get_security_stats().await;
        assert_eq!(stats.total_sessions, 10);
    }

    #[tokio::test]
    async fn test_validation_counter() {
        let state = MockSecurityManagerState::new();
        state.initialize().await.unwrap();
        let manager = state.get_manager().await.unwrap();

        assert_eq!(manager.validation_count.load(Ordering::Relaxed), 0);

        // Perform some validations
        let request = MockValidationRequest {
            field_name: "test".to_string(),
            value: json!("value"),
            context: HashMap::new(),
        };

        for _ in 0..5 {
            let _ = manager.validate_input(&request).await;
        }

        assert_eq!(manager.validation_count.load(Ordering::Relaxed), 5);
    }

    #[tokio::test]
    async fn test_error_conditions() {
        let state = MockSecurityManagerState::new();

        // Test operations without initialization
        let manager = MockSecurityManager::new();
        let result = manager.create_session(vec!["test".to_string()]).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Security manager not initialized");

        // Initialize and test edge cases
        state.initialize().await.unwrap();
        let manager = state.get_manager().await.unwrap();

        // Test empty credentials
        let result = manager.retrieve_credential("nonexistent", "service").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Credential not found");

        // Test deleting non-existent credential
        let result = manager.delete_credential("nonexistent", "service").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Credential not found");
    }

    #[tokio::test]
    async fn test_large_data_handling() {
        let state = MockSecurityManagerState::new();
        state.initialize().await.unwrap();
        let manager = state.get_manager().await.unwrap();

        // Test validation with large data
        let large_string = "x".repeat(10000);
        let request = MockValidationRequest {
            field_name: "large_field".to_string(),
            value: json!(large_string),
            context: HashMap::new(),
        };

        let result = manager.validate_input(&request).await;
        assert!(result.is_ok());

        // Test sanitization of large string
        let large_html = format!("<script>{}</script>", "x".repeat(5000));
        let sanitized = manager.sanitize_string(&large_html).await;
        assert!(sanitized.starts_with("&lt;script&gt;"));
        assert!(sanitized.ends_with("&lt;/script&gt;"));
    }

    #[tokio::test]
    async fn test_boundary_conditions() {
        let state = MockSecurityManagerState::new();
        state.initialize().await.unwrap();
        let manager = state.get_manager().await.unwrap();

        // Test zero-length random string
        let random = manager.generate_random_string(0).await;
        assert_eq!(random.len(), 0);

        // Test maximum length random string (within reason)
        let random = manager.generate_random_string(1024).await;
        assert_eq!(random.len(), 1024);

        // Test empty data hashing
        let hash = manager.hash_sha256(&[]).await;
        assert!(hash.contains("0"));

        // Test path validation with empty path
        let result = manager.validate_path("").await;
        assert!(result.is_ok());
    }
}
