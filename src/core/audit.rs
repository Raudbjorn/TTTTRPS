//! Audit Logger Module
//!
//! Provides security audit logging for tracking important system events.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::RwLock;

// ============================================================================
// Types
// ============================================================================

/// Type of audit event
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    // Authentication & Credentials
    ApiKeyAdded { provider: String },
    ApiKeyRemoved { provider: String },
    ApiKeyAccessed { provider: String },

    // Document Operations
    DocumentIngested { path: String, doc_type: String },
    DocumentDeleted { doc_id: String },
    DocumentSearched { query: String },

    // Campaign Operations
    CampaignCreated { campaign_id: String, name: String },
    CampaignDeleted { campaign_id: String },
    CampaignExported { campaign_id: String },
    CampaignImported { name: String },

    // Session Operations
    SessionStarted { session_id: String, campaign_id: String },
    SessionEnded { session_id: String },

    // LLM Operations
    LlmRequest { provider: String, model: String, tokens: u32 },
    LlmError { provider: String, error: String },

    // Settings Changes
    SettingsChanged { setting: String, old_value: String, new_value: String },

    // Security Events
    ValidationFailed { input_type: String, reason: String },
    RateLimitHit { endpoint: String },

    // System Events
    ApplicationStarted,
    ApplicationShutdown,
    BackupCreated { path: String },
    BackupRestored { path: String },

    // Custom
    Custom { category: String, action: String, details: String },
}

/// Audit event severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    Info,
    Warning,
    Security,
    Critical,
}

impl std::str::FromStr for AuditSeverity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "info" => Ok(Self::Info),
            "warning" | "warn" => Ok(Self::Warning),
            "security" | "sec" => Ok(Self::Security),
            "critical" | "crit" => Ok(Self::Critical),
            _ => Err(format!("Unknown audit severity: {}", s)),
        }
    }
}

impl std::fmt::Display for AuditSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Security => write!(f, "security"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// A single audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID
    pub id: String,
    /// Event type
    pub event_type: AuditEventType,
    /// Severity level
    pub severity: AuditSeverity,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Optional user/session context
    pub context: Option<String>,
    /// IP address or source (if applicable)
    pub source: Option<String>,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Audit log query parameters
#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub severity: Option<AuditSeverity>,
    pub event_types: Option<Vec<String>>,
    pub limit: Option<usize>,
}

// ============================================================================
// Audit Logger
// ============================================================================

/// Audit logging system
pub struct AuditLogger {
    /// In-memory event buffer
    events: RwLock<VecDeque<AuditEvent>>,
    /// Maximum events to keep in memory
    max_events: usize,
    /// Whether to also log to tracing
    log_to_tracing: bool,
}

impl AuditLogger {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(VecDeque::with_capacity(10000)),
            max_events: 10000,
            log_to_tracing: true,
        }
    }

    /// Log an audit event
    pub fn log(&self, event_type: AuditEventType, severity: AuditSeverity) -> String {
        self.log_with_context(event_type, severity, None, None)
    }

    /// Log an audit event with context
    pub fn log_with_context(
        &self,
        event_type: AuditEventType,
        severity: AuditSeverity,
        context: Option<String>,
        source: Option<String>,
    ) -> String {
        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: event_type.clone(),
            severity,
            timestamp: Utc::now(),
            context,
            source,
            metadata: serde_json::Value::Null,
        };

        let event_id = event.id.clone();

        // Log to tracing if enabled
        if self.log_to_tracing {
            self.trace_event(&event);
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

    /// Log to tracing system
    fn trace_event(&self, event: &AuditEvent) {
        let event_desc = format!("{:?}", event.event_type);

        match event.severity {
            AuditSeverity::Info => {
                tracing::info!(
                    audit_id = %event.id,
                    event = %event_desc,
                    "Audit event"
                );
            }
            AuditSeverity::Warning => {
                tracing::warn!(
                    audit_id = %event.id,
                    event = %event_desc,
                    "Audit event"
                );
            }
            AuditSeverity::Security => {
                tracing::warn!(
                    audit_id = %event.id,
                    event = %event_desc,
                    "SECURITY audit event"
                );
            }
            AuditSeverity::Critical => {
                tracing::error!(
                    audit_id = %event.id,
                    event = %event_desc,
                    "CRITICAL audit event"
                );
            }
        }
    }

    /// Query audit events
    pub fn query(&self, params: AuditQuery) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();

        let filtered: Vec<AuditEvent> = events
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
                if let Some(severity) = params.severity {
                    if e.severity < severity {
                        return false;
                    }
                }

                // Filter by event type (string match)
                if let Some(ref types) = params.event_types {
                    let event_str = format!("{:?}", e.event_type);
                    if !types.iter().any(|t| event_str.contains(t)) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Apply limit
        if let Some(limit) = params.limit {
            filtered.into_iter().rev().take(limit).collect()
        } else {
            filtered
        }
    }

    /// Get recent events
    pub fn get_recent(&self, count: usize) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();
        events.iter().rev().take(count).cloned().collect()
    }

    /// Get events by severity
    pub fn get_by_severity(&self, severity: AuditSeverity) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();
        events
            .iter()
            .filter(|e| e.severity >= severity)
            .cloned()
            .collect()
    }

    /// Get security events (last 24 hours)
    pub fn get_security_events(&self) -> Vec<AuditEvent> {
        let cutoff = Utc::now() - Duration::hours(24);
        let events = self.events.read().unwrap();
        events
            .iter()
            .filter(|e| e.severity >= AuditSeverity::Security && e.timestamp > cutoff)
            .cloned()
            .collect()
    }

    /// Export events to JSON
    pub fn export_json(&self, params: AuditQuery) -> String {
        let events = self.query(params);
        serde_json::to_string_pretty(&events).unwrap_or_default()
    }

    /// Clear old events (older than days)
    pub fn cleanup(&self, days: i64) {
        let cutoff = Utc::now() - Duration::days(days);
        let mut events = self.events.write().unwrap();
        events.retain(|e| e.timestamp > cutoff);
    }

    /// Get event count
    pub fn count(&self) -> usize {
        self.events.read().unwrap().len()
    }

    // ========================================================================
    // Convenience Methods
    // ========================================================================

    /// Log API key added
    pub fn log_api_key_added(&self, provider: &str) {
        self.log(
            AuditEventType::ApiKeyAdded {
                provider: provider.to_string(),
            },
            AuditSeverity::Security,
        );
    }

    /// Log API key removed
    pub fn log_api_key_removed(&self, provider: &str) {
        self.log(
            AuditEventType::ApiKeyRemoved {
                provider: provider.to_string(),
            },
            AuditSeverity::Security,
        );
    }

    /// Log document ingestion
    pub fn log_document_ingested(&self, path: &str, doc_type: &str) {
        self.log(
            AuditEventType::DocumentIngested {
                path: path.to_string(),
                doc_type: doc_type.to_string(),
            },
            AuditSeverity::Info,
        );
    }

    /// Log LLM request
    pub fn log_llm_request(&self, provider: &str, model: &str, tokens: u32) {
        self.log(
            AuditEventType::LlmRequest {
                provider: provider.to_string(),
                model: model.to_string(),
                tokens,
            },
            AuditSeverity::Info,
        );
    }

    /// Log validation failure
    pub fn log_validation_failed(&self, input_type: &str, reason: &str) {
        self.log(
            AuditEventType::ValidationFailed {
                input_type: input_type.to_string(),
                reason: reason.to_string(),
            },
            AuditSeverity::Security,
        );
    }

    /// Log campaign created
    pub fn log_campaign_created(&self, campaign_id: &str, name: &str) {
        self.log(
            AuditEventType::CampaignCreated {
                campaign_id: campaign_id.to_string(),
                name: name.to_string(),
            },
            AuditSeverity::Info,
        );
    }

    /// Log session started
    pub fn log_session_started(&self, session_id: &str, campaign_id: &str) {
        self.log(
            AuditEventType::SessionStarted {
                session_id: session_id.to_string(),
                campaign_id: campaign_id.to_string(),
            },
            AuditSeverity::Info,
        );
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_logging() {
        let logger = AuditLogger::new();

        let id = logger.log(
            AuditEventType::ApiKeyAdded {
                provider: "openai".to_string(),
            },
            AuditSeverity::Security,
        );

        assert!(!id.is_empty());
        assert_eq!(logger.count(), 1);
    }

    #[test]
    fn test_query() {
        let logger = AuditLogger::new();

        logger.log_api_key_added("openai");
        logger.log_document_ingested("/path/to/doc.pdf", "pdf");
        logger.log_llm_request("claude", "claude-3-sonnet", 100);

        let security_events = logger.get_by_severity(AuditSeverity::Security);
        assert_eq!(security_events.len(), 1);

        let all_events = logger.get_recent(10);
        assert_eq!(all_events.len(), 3);
    }

    #[test]
    fn test_rotation() {
        let logger = AuditLogger {
            events: RwLock::new(VecDeque::new()),
            max_events: 5,
            log_to_tracing: false,
        };

        for i in 0..10 {
            logger.log(
                AuditEventType::Custom {
                    category: "test".to_string(),
                    action: format!("action_{}", i),
                    details: "".to_string(),
                },
                AuditSeverity::Info,
            );
        }

        assert_eq!(logger.count(), 5);
    }
}
