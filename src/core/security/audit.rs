//! Security Audit Logger
//!
//! Comprehensive security audit logging with log rotation, export, and viewer support.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::RwLock;

// ============================================================================
// Types
// ============================================================================

/// Type of audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEventType {
    // Authentication & Credentials
    ApiKeyAdded { provider: String, masked_key: String },
    ApiKeyRemoved { provider: String },
    ApiKeyAccessed { provider: String },
    ApiKeyRotated { provider: String },

    // File Operations
    DocumentIngested { path: String, doc_type: String, size_bytes: u64 },
    DocumentDeleted { doc_id: String, name: String },
    DocumentExported { doc_id: String, export_path: String },

    // Campaign Operations
    CampaignCreated { campaign_id: String, name: String },
    CampaignDeleted { campaign_id: String, name: String },
    CampaignExported { campaign_id: String, export_path: String },
    CampaignImported { name: String, source_path: String },
    CampaignArchived { campaign_id: String },
    CampaignRestored { campaign_id: String },

    // Session Operations
    SessionStarted { session_id: String, campaign_id: String },
    SessionEnded { session_id: String, duration_minutes: i64 },
    SessionRestored { session_id: String },

    // LLM Operations
    LlmRequest {
        provider: String,
        model: String,
        input_tokens: u32,
        output_tokens: u32,
        cost_usd: f64
    },
    LlmError { provider: String, error: String, is_auth_error: bool },
    LlmProviderChanged { old_provider: Option<String>, new_provider: String },

    // Configuration Changes
    SettingChanged {
        setting: String,
        old_value_hash: Option<String>,
        new_value_hash: String,
        is_sensitive: bool
    },
    BudgetLimitSet { period: String, limit_usd: f64 },
    ThemeChanged { old_theme: String, new_theme: String },

    // Security Events
    ValidationFailed { input_type: String, reason: String, severity: String },
    RateLimitHit { endpoint: String, limit: u32, window_seconds: u32 },
    SuspiciousActivity { description: String, details: String },

    // Data Operations
    DataBackupCreated { backup_path: String, size_bytes: u64 },
    DataBackupRestored { backup_path: String },
    DataExported { format: String, export_path: String },

    // System Events
    ApplicationStarted { version: String },
    ApplicationShutdown { graceful: bool },
    DatabaseMigration { from_version: i32, to_version: i32 },
    SidecarStarted { name: String },
    SidecarStopped { name: String, exit_code: Option<i32> },

    // Voice Operations
    VoiceGenerated { text_length: usize, voice_id: String, cached: bool },
    VoiceQueueCleared { item_count: usize },

    // Search Operations
    SearchPerformed { query_hash: String, result_count: usize, source_type: Option<String> },

    // Custom Events
    Custom { category: String, action: String, details: String },
}

/// Audit event severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    Debug,
    Info,
    Warning,
    Security,
    Critical,
}

impl AuditSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditSeverity::Debug => "debug",
            AuditSeverity::Info => "info",
            AuditSeverity::Warning => "warning",
            AuditSeverity::Security => "security",
            AuditSeverity::Critical => "critical",
        }
    }
}

/// A single audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditEvent {
    /// Unique event ID
    pub id: String,
    /// Event type
    pub event_type: SecurityEventType,
    /// Severity level
    pub severity: AuditSeverity,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Session/user context
    pub context: Option<String>,
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

impl SecurityAuditEvent {
    /// Get a human-readable description of the event
    pub fn description(&self) -> String {
        match &self.event_type {
            SecurityEventType::ApiKeyAdded { provider, masked_key } => {
                format!("API key added for {} ({})", provider, masked_key)
            }
            SecurityEventType::ApiKeyRemoved { provider } => {
                format!("API key removed for {}", provider)
            }
            SecurityEventType::ApiKeyAccessed { provider } => {
                format!("API key accessed for {}", provider)
            }
            SecurityEventType::DocumentIngested { path, doc_type, size_bytes } => {
                format!("Document ingested: {} ({}, {} bytes)", path, doc_type, size_bytes)
            }
            SecurityEventType::LlmRequest { provider, model, input_tokens, output_tokens, cost_usd } => {
                format!("LLM request to {}/{}: {}in/{}out tokens (${:.4})",
                    provider, model, input_tokens, output_tokens, cost_usd)
            }
            SecurityEventType::SettingChanged { setting, is_sensitive, .. } => {
                if *is_sensitive {
                    format!("Sensitive setting changed: {}", setting)
                } else {
                    format!("Setting changed: {}", setting)
                }
            }
            SecurityEventType::ValidationFailed { input_type, reason, .. } => {
                format!("Validation failed for {}: {}", input_type, reason)
            }
            SecurityEventType::ApplicationStarted { version } => {
                format!("Application started (v{})", version)
            }
            SecurityEventType::ApplicationShutdown { graceful } => {
                format!("Application shutdown (graceful: {})", graceful)
            }
            SecurityEventType::Custom { category, action, details } => {
                format!("{}/{}: {}", category, action, details)
            }
            _ => format!("{:?}", self.event_type),
        }
    }
}

/// Query parameters for filtering audit logs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditLogQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub min_severity: Option<AuditSeverity>,
    pub event_types: Option<Vec<String>>,
    pub search_text: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Audit log export format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Json,
    Csv,
    Jsonl,
}

/// Log rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    /// Maximum log file size in bytes before rotation
    pub max_size_bytes: u64,
    /// Maximum age of logs to keep (in days)
    pub max_age_days: u32,
    /// Maximum number of rotated log files to keep
    pub max_files: u32,
    /// Whether to compress rotated logs
    pub compress: bool,
}

impl Default for LogRotationConfig {
    fn default() -> Self {
        Self {
            max_size_bytes: 10 * 1024 * 1024, // 10 MB
            max_age_days: 90,
            max_files: 10,
            compress: false, // Compression requires additional dependencies
        }
    }
}

// ============================================================================
// Security Audit Logger
// ============================================================================

/// Security audit logging system
pub struct SecurityAuditLogger {
    /// In-memory event buffer
    events: RwLock<VecDeque<SecurityAuditEvent>>,
    /// Maximum events to keep in memory
    max_events: usize,
    /// Log file path (optional)
    log_path: Option<PathBuf>,
    /// Log rotation config
    rotation_config: LogRotationConfig,
    /// Whether to also log to tracing
    log_to_tracing: bool,
}

impl SecurityAuditLogger {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(VecDeque::with_capacity(10000)),
            max_events: 10000,
            log_path: None,
            rotation_config: LogRotationConfig::default(),
            log_to_tracing: true,
        }
    }

    /// Create with file logging enabled
    pub fn with_file_logging(log_dir: PathBuf) -> Self {
        let log_path = log_dir.join("security_audit.jsonl");

        // Ensure directory exists
        if let Some(parent) = log_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        Self {
            events: RwLock::new(VecDeque::with_capacity(10000)),
            max_events: 10000,
            log_path: Some(log_path),
            rotation_config: LogRotationConfig::default(),
            log_to_tracing: true,
        }
    }

    /// Configure log rotation
    pub fn with_rotation(mut self, config: LogRotationConfig) -> Self {
        self.rotation_config = config;
        self
    }

    /// Log an audit event
    pub fn log(&self, event_type: SecurityEventType, severity: AuditSeverity) -> String {
        self.log_with_context(event_type, severity, None, None)
    }

    /// Log an audit event with context
    pub fn log_with_context(
        &self,
        event_type: SecurityEventType,
        severity: AuditSeverity,
        context: Option<String>,
        metadata: Option<serde_json::Value>,
    ) -> String {
        let event = SecurityAuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type,
            severity,
            timestamp: Utc::now(),
            context,
            metadata,
        };

        let event_id = event.id.clone();

        // Log to tracing if enabled
        if self.log_to_tracing {
            self.trace_event(&event);
        }

        // Write to file if configured
        if let Some(ref path) = self.log_path {
            let _ = self.write_to_file(path, &event);
        }

        // Store in memory buffer
        {
            let mut events = self.events.write().unwrap();
            events.push_back(event);

            // Rotate if needed
            while events.len() > self.max_events {
                events.pop_front();
            }
        }

        event_id
    }

    /// Write event to log file
    fn write_to_file(&self, path: &PathBuf, event: &SecurityAuditEvent) -> std::io::Result<()> {
        // Check if rotation is needed
        if let Ok(metadata) = fs::metadata(path) {
            if metadata.len() >= self.rotation_config.max_size_bytes {
                self.rotate_logs(path)?;
            }
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        let json = serde_json::to_string(event)?;
        writeln!(file, "{}", json)?;

        Ok(())
    }

    /// Rotate log files
    fn rotate_logs(&self, current_path: &PathBuf) -> std::io::Result<()> {
        let parent = current_path.parent().unwrap_or(std::path::Path::new("."));
        let stem = current_path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = current_path.extension().unwrap_or_default().to_string_lossy();

        // Shift existing rotated files
        for i in (1..self.rotation_config.max_files).rev() {
            let old_path = parent.join(format!("{}.{}.{}", stem, i, ext));
            let new_path = parent.join(format!("{}.{}.{}", stem, i + 1, ext));
            if old_path.exists() {
                if i + 1 >= self.rotation_config.max_files {
                    fs::remove_file(&old_path)?;
                } else {
                    fs::rename(&old_path, &new_path)?;
                }
            }
        }

        // Rotate current file
        let rotated_path = parent.join(format!("{}.1.{}", stem, ext));
        if current_path.exists() {
            fs::rename(current_path, rotated_path)?;
        }

        Ok(())
    }

    /// Log to tracing system
    fn trace_event(&self, event: &SecurityAuditEvent) {
        let desc = event.description();

        match event.severity {
            AuditSeverity::Debug => {
                tracing::debug!(
                    audit_id = %event.id,
                    event = %desc,
                    "Security audit event"
                );
            }
            AuditSeverity::Info => {
                tracing::info!(
                    audit_id = %event.id,
                    event = %desc,
                    "Security audit event"
                );
            }
            AuditSeverity::Warning => {
                tracing::warn!(
                    audit_id = %event.id,
                    event = %desc,
                    "Security audit event"
                );
            }
            AuditSeverity::Security => {
                tracing::warn!(
                    audit_id = %event.id,
                    event = %desc,
                    "SECURITY audit event"
                );
            }
            AuditSeverity::Critical => {
                tracing::error!(
                    audit_id = %event.id,
                    event = %desc,
                    "CRITICAL security audit event"
                );
            }
        }
    }

    /// Query audit events
    pub fn query(&self, params: AuditLogQuery) -> Vec<SecurityAuditEvent> {
        let events = self.events.read().unwrap();

        let filtered: Vec<SecurityAuditEvent> = events
            .iter()
            .filter(|e| {
                // Filter by time range
                if let Some(from) = params.from {
                    if e.timestamp < from {
                        return false;
                    }
                }
                if let Some(to) = params.to {
                    if e.timestamp > to {
                        return false;
                    }
                }

                // Filter by severity
                if let Some(min_severity) = params.min_severity {
                    if e.severity < min_severity {
                        return false;
                    }
                }

                // Filter by event type (string match)
                if let Some(ref types) = params.event_types {
                    let event_str = format!("{:?}", e.event_type);
                    if !types.iter().any(|t| event_str.to_lowercase().contains(&t.to_lowercase())) {
                        return false;
                    }
                }

                // Filter by search text
                if let Some(ref search) = params.search_text {
                    let desc = e.description().to_lowercase();
                    if !desc.contains(&search.to_lowercase()) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Apply offset and limit
        let start = params.offset.unwrap_or(0);
        let limit = params.limit.unwrap_or(1000);

        filtered
            .into_iter()
            .skip(start)
            .take(limit)
            .collect()
    }

    /// Get recent events
    pub fn get_recent(&self, count: usize) -> Vec<SecurityAuditEvent> {
        let events = self.events.read().unwrap();
        events.iter().rev().take(count).cloned().collect()
    }

    /// Get events by severity
    pub fn get_by_severity(&self, min_severity: AuditSeverity) -> Vec<SecurityAuditEvent> {
        let events = self.events.read().unwrap();
        events
            .iter()
            .filter(|e| e.severity >= min_severity)
            .cloned()
            .collect()
    }

    /// Get security events (last 24 hours)
    pub fn get_security_events(&self) -> Vec<SecurityAuditEvent> {
        let cutoff = Utc::now() - Duration::hours(24);
        let events = self.events.read().unwrap();
        events
            .iter()
            .filter(|e| e.severity >= AuditSeverity::Security && e.timestamp > cutoff)
            .cloned()
            .collect()
    }

    /// Export events to a file
    pub fn export(&self, params: AuditLogQuery, format: ExportFormat) -> Result<String, String> {
        let events = self.query(params);

        match format {
            ExportFormat::Json => {
                serde_json::to_string_pretty(&events)
                    .map_err(|e| format!("Failed to serialize to JSON: {}", e))
            }
            ExportFormat::Jsonl => {
                let lines: Vec<String> = events
                    .iter()
                    .filter_map(|e| serde_json::to_string(e).ok())
                    .collect();
                Ok(lines.join("\n"))
            }
            ExportFormat::Csv => {
                let mut csv = String::from("id,timestamp,severity,event_type,description\n");
                for event in events {
                    let event_type = format!("{:?}", event.event_type)
                        .split_whitespace()
                        .next()
                        .unwrap_or("Unknown")
                        .to_string();
                    let desc = event.description().replace(',', ";").replace('\n', " ");
                    csv.push_str(&format!(
                        "{},{},{},{},{}\n",
                        event.id,
                        event.timestamp.to_rfc3339(),
                        event.severity.as_str(),
                        event_type,
                        desc
                    ));
                }
                Ok(csv)
            }
        }
    }

    /// Export to file
    pub fn export_to_file(
        &self,
        path: &PathBuf,
        params: AuditLogQuery,
        format: ExportFormat,
    ) -> Result<(), String> {
        let content = self.export(params, format)?;

        fs::write(path, content)
            .map_err(|e| format!("Failed to write export file: {}", e))
    }

    /// Clear old events (older than days)
    pub fn cleanup(&self, days: i64) -> usize {
        let cutoff = Utc::now() - Duration::days(days);
        let mut events = self.events.write().unwrap();
        let before_len = events.len();
        events.retain(|e| e.timestamp > cutoff);
        before_len - events.len()
    }

    /// Get event count
    pub fn count(&self) -> usize {
        self.events.read().unwrap().len()
    }

    /// Get event count by severity
    pub fn count_by_severity(&self) -> std::collections::HashMap<String, usize> {
        let events = self.events.read().unwrap();
        let mut counts = std::collections::HashMap::new();

        for event in events.iter() {
            let key = event.severity.as_str().to_string();
            *counts.entry(key).or_insert(0) += 1;
        }

        counts
    }

    // ========================================================================
    // Convenience Methods
    // ========================================================================

    /// Mask an API key for logging (show only last 4 chars)
    pub fn mask_api_key(key: &str) -> String {
        if key.len() <= 4 {
            return "****".to_string();
        }
        format!("****{}", &key[key.len() - 4..])
    }

    /// Hash a value for logging (don't store actual value)
    pub fn hash_value(value: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Log API key added
    pub fn log_api_key_added(&self, provider: &str, key: &str) {
        self.log(
            SecurityEventType::ApiKeyAdded {
                provider: provider.to_string(),
                masked_key: Self::mask_api_key(key),
            },
            AuditSeverity::Security,
        );
    }

    /// Log API key removed
    pub fn log_api_key_removed(&self, provider: &str) {
        self.log(
            SecurityEventType::ApiKeyRemoved {
                provider: provider.to_string(),
            },
            AuditSeverity::Security,
        );
    }

    /// Log document ingestion
    pub fn log_document_ingested(&self, path: &str, doc_type: &str, size_bytes: u64) {
        self.log(
            SecurityEventType::DocumentIngested {
                path: path.to_string(),
                doc_type: doc_type.to_string(),
                size_bytes,
            },
            AuditSeverity::Info,
        );
    }

    /// Log LLM request
    pub fn log_llm_request(
        &self,
        provider: &str,
        model: &str,
        input_tokens: u32,
        output_tokens: u32,
        cost_usd: f64,
    ) {
        self.log(
            SecurityEventType::LlmRequest {
                provider: provider.to_string(),
                model: model.to_string(),
                input_tokens,
                output_tokens,
                cost_usd,
            },
            AuditSeverity::Info,
        );
    }

    /// Log setting change
    pub fn log_setting_changed(
        &self,
        setting: &str,
        old_value: Option<&str>,
        new_value: &str,
        is_sensitive: bool,
    ) {
        self.log(
            SecurityEventType::SettingChanged {
                setting: setting.to_string(),
                old_value_hash: old_value.map(Self::hash_value),
                new_value_hash: Self::hash_value(new_value),
                is_sensitive,
            },
            if is_sensitive {
                AuditSeverity::Security
            } else {
                AuditSeverity::Info
            },
        );
    }

    /// Log validation failure
    pub fn log_validation_failed(&self, input_type: &str, reason: &str) {
        self.log(
            SecurityEventType::ValidationFailed {
                input_type: input_type.to_string(),
                reason: reason.to_string(),
                severity: "warning".to_string(),
            },
            AuditSeverity::Warning,
        );
    }

    /// Log campaign created
    pub fn log_campaign_created(&self, campaign_id: &str, name: &str) {
        self.log(
            SecurityEventType::CampaignCreated {
                campaign_id: campaign_id.to_string(),
                name: name.to_string(),
            },
            AuditSeverity::Info,
        );
    }

    /// Log session started
    pub fn log_session_started(&self, session_id: &str, campaign_id: &str) {
        self.log(
            SecurityEventType::SessionStarted {
                session_id: session_id.to_string(),
                campaign_id: campaign_id.to_string(),
            },
            AuditSeverity::Info,
        );
    }

    /// Log application started
    pub fn log_app_started(&self, version: &str) {
        self.log(
            SecurityEventType::ApplicationStarted {
                version: version.to_string(),
            },
            AuditSeverity::Info,
        );
    }
}

impl Default for SecurityAuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_logging() {
        let logger = SecurityAuditLogger::new();

        let id = logger.log(
            SecurityEventType::ApiKeyAdded {
                provider: "openai".to_string(),
                masked_key: "****1234".to_string(),
            },
            AuditSeverity::Security,
        );

        assert!(!id.is_empty());
        assert_eq!(logger.count(), 1);
    }

    #[test]
    fn test_query() {
        let logger = SecurityAuditLogger::new();

        logger.log_api_key_added("openai", "sk-1234567890");
        logger.log_document_ingested("/path/to/doc.pdf", "pdf", 1024);
        logger.log_llm_request("claude", "claude-3-sonnet", 100, 50, 0.01);

        let security_events = logger.get_by_severity(AuditSeverity::Security);
        assert_eq!(security_events.len(), 1);

        let all_events = logger.get_recent(10);
        assert_eq!(all_events.len(), 3);
    }

    #[test]
    fn test_mask_api_key() {
        assert_eq!(SecurityAuditLogger::mask_api_key("sk-1234567890"), "****7890");
        assert_eq!(SecurityAuditLogger::mask_api_key("abc"), "****");
    }

    #[test]
    fn test_export_csv() {
        let logger = SecurityAuditLogger::new();
        logger.log_app_started("1.0.0");

        let csv = logger.export(AuditLogQuery::default(), ExportFormat::Csv).unwrap();
        assert!(csv.contains("id,timestamp,severity,event_type,description"));
        assert!(csv.contains("ApplicationStarted"));
    }

    #[test]
    fn test_cleanup() {
        let logger = SecurityAuditLogger::new();
        logger.log_app_started("1.0.0");

        // Cleanup with 0 days should remove all events
        let removed = logger.cleanup(0);
        assert_eq!(removed, 1);
        assert_eq!(logger.count(), 0);
    }
}
