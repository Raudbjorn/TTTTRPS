//! Data Migration Utilities
//!
//! Utilities to migrate existing plot points from legacy format to the enhanced
//! Meilisearch-first format.
//!
//! TASK-CAMP-006

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum MigrationError {
    #[error("Migration failed: {0}")]
    Failed(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Rollback error: {0}")]
    RollbackError(String),

    #[error("Data conversion error: {0}")]
    ConversionError(String),
}

pub type Result<T> = std::result::Result<T, MigrationError>;

// ============================================================================
// Migration Report
// ============================================================================

/// Report generated after migration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationReport {
    /// Migration ID
    pub id: String,
    /// When migration started
    pub started_at: DateTime<Utc>,
    /// When migration completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Migration status
    pub status: MigrationStatus,
    /// Number of entities migrated
    pub migrated_count: usize,
    /// Number of entities skipped (already migrated)
    pub skipped_count: usize,
    /// Number of entities that failed
    pub failed_count: usize,
    /// IDs of migrated entities
    pub migrated_ids: Vec<String>,
    /// IDs of skipped entities
    pub skipped_ids: Vec<String>,
    /// Failed entity IDs with error messages
    pub failed_entities: Vec<FailedEntity>,
    /// Warnings generated during migration
    pub warnings: Vec<String>,
}

/// Migration status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MigrationStatus {
    /// Migration in progress
    InProgress,
    /// Migration completed successfully
    Completed,
    /// Migration completed with some failures
    PartialSuccess,
    /// Migration failed
    Failed,
    /// Migration rolled back
    RolledBack,
}

/// A failed entity with error details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedEntity {
    pub id: String,
    pub title: Option<String>,
    pub error: String,
}

impl MigrationReport {
    /// Create a new migration report
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            started_at: Utc::now(),
            completed_at: None,
            status: MigrationStatus::InProgress,
            migrated_count: 0,
            skipped_count: 0,
            failed_count: 0,
            migrated_ids: Vec::new(),
            skipped_ids: Vec::new(),
            failed_entities: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Record a successful migration
    pub fn record_migrated(&mut self, id: &str) {
        self.migrated_count += 1;
        self.migrated_ids.push(id.to_string());
    }

    /// Record a skipped entity (already migrated)
    pub fn record_skipped(&mut self, id: &str) {
        self.skipped_count += 1;
        self.skipped_ids.push(id.to_string());
    }

    /// Record a failed migration
    pub fn record_failed(&mut self, id: &str, title: Option<&str>, error: &str) {
        self.failed_count += 1;
        self.failed_entities.push(FailedEntity {
            id: id.to_string(),
            title: title.map(|s| s.to_string()),
            error: error.to_string(),
        });
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: &str) {
        self.warnings.push(warning.to_string());
    }

    /// Finalize the report
    pub fn finalize(&mut self) {
        self.completed_at = Some(Utc::now());
        self.status = if self.failed_count == 0 {
            MigrationStatus::Completed
        } else if self.migrated_count > 0 {
            MigrationStatus::PartialSuccess
        } else {
            MigrationStatus::Failed
        };
    }

    /// Mark as rolled back
    pub fn mark_rolled_back(&mut self) {
        self.status = MigrationStatus::RolledBack;
    }

    /// Get summary string
    pub fn summary(&self) -> String {
        format!(
            "Migration {}: {} migrated, {} skipped, {} failed",
            self.status.as_str(),
            self.migrated_count,
            self.skipped_count,
            self.failed_count
        )
    }
}

impl Default for MigrationReport {
    fn default() -> Self {
        Self::new()
    }
}

impl MigrationStatus {
    /// Get status as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::PartialSuccess => "partial_success",
            Self::Failed => "failed",
            Self::RolledBack => "rolled_back",
        }
    }
}

// ============================================================================
// Migration Options
// ============================================================================

/// Options for migration
#[derive(Debug, Clone)]
pub struct MigrationOptions {
    /// Campaign IDs to migrate (None = all campaigns)
    pub campaign_ids: Option<Vec<String>>,
    /// Skip entities that have already been migrated
    pub skip_existing: bool,
    /// Dry run mode - don't actually write data
    pub dry_run: bool,
    /// Maximum entities to migrate (for testing)
    pub limit: Option<usize>,
}

impl Default for MigrationOptions {
    fn default() -> Self {
        Self {
            campaign_ids: None,
            skip_existing: true,
            dry_run: false,
            limit: None,
        }
    }
}

impl MigrationOptions {
    /// Create options for a specific campaign
    pub fn for_campaign(campaign_id: &str) -> Self {
        Self {
            campaign_ids: Some(vec![campaign_id.to_string()]),
            ..Default::default()
        }
    }

    /// Enable dry run mode
    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    /// Set a limit on entities to migrate
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

// ============================================================================
// Migration State Tracker
// ============================================================================

/// Tracks migration state for rollback support
#[derive(Debug, Clone)]
pub struct MigrationState {
    /// IDs of newly created documents that can be rolled back
    pub created_ids: HashSet<String>,
    /// Original documents that were updated (for rollback)
    pub original_documents: Vec<serde_json::Value>,
}

impl MigrationState {
    /// Create new migration state
    pub fn new() -> Self {
        Self {
            created_ids: HashSet::new(),
            original_documents: Vec::new(),
        }
    }

    /// Record a newly created document
    pub fn record_created(&mut self, id: &str) {
        self.created_ids.insert(id.to_string());
    }

    /// Record an original document for rollback
    pub fn record_original(&mut self, document: serde_json::Value) {
        self.original_documents.push(document);
    }

    /// Check if a document was created in this migration
    pub fn was_created(&self, id: &str) -> bool {
        self.created_ids.contains(id)
    }

    /// Get IDs for rollback (created documents that should be deleted)
    pub fn rollback_ids(&self) -> Vec<String> {
        self.created_ids.iter().cloned().collect()
    }
}

impl Default for MigrationState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Default Values for Migration
// ============================================================================

/// Default values applied during migration from legacy PlotPoint
pub mod defaults {
    /// Default plot type for migrated plot points
    pub const DEFAULT_PLOT_TYPE: &str = "hook";

    /// Default tension level (1-10 scale)
    pub const DEFAULT_TENSION_LEVEL: u8 = 5;

    /// Default urgency for migrated plot points
    pub const DEFAULT_URGENCY: &str = "background";

    /// Generate dramatic question from title
    pub fn generate_dramatic_question(title: &str) -> String {
        format!("Will the party {}?", title.to_lowercase())
    }

    /// Map legacy PlotStatus to ActivationState
    pub fn map_status_to_activation_state(status: &str) -> &'static str {
        match status.to_lowercase().as_str() {
            "pending" => "dormant",
            "active" => "active",
            "completed" | "failed" => "resolved",
            "paused" => "planted",
            _ => "dormant",
        }
    }

    /// Map legacy PlotPriority to Urgency
    pub fn map_priority_to_urgency(priority: &str) -> &'static str {
        match priority.to_lowercase().as_str() {
            "background" => "background",
            "side" => "background",
            "main" => "upcoming",
            "critical" => "critical",
            _ => "background",
        }
    }
}

// ============================================================================
// Validation
// ============================================================================

/// Validate a migrated entity
pub fn validate_migrated_data(data: &serde_json::Value) -> Result<()> {
    // Check required fields
    let required_fields = ["id", "campaign_id", "title"];
    for field in required_fields {
        if data.get(field).is_none() {
            return Err(MigrationError::ValidationError(format!(
                "Missing required field: {}",
                field
            )));
        }
    }

    // Validate tension_level if present
    if let Some(tension) = data.get("tension_level") {
        if let Some(level) = tension.as_u64() {
            if level > 10 {
                return Err(MigrationError::ValidationError(format!(
                    "tension_level {} exceeds maximum of 10",
                    level
                )));
            }
        }
    }

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_report_new() {
        let report = MigrationReport::new();
        assert_eq!(report.status, MigrationStatus::InProgress);
        assert_eq!(report.migrated_count, 0);
        assert!(report.completed_at.is_none());
    }

    #[test]
    fn test_migration_report_finalize() {
        let mut report = MigrationReport::new();
        report.record_migrated("id1");
        report.record_migrated("id2");
        report.finalize();

        assert_eq!(report.status, MigrationStatus::Completed);
        assert_eq!(report.migrated_count, 2);
        assert!(report.completed_at.is_some());
    }

    #[test]
    fn test_migration_report_partial_success() {
        let mut report = MigrationReport::new();
        report.record_migrated("id1");
        report.record_failed("id2", Some("Test"), "Error message");
        report.finalize();

        assert_eq!(report.status, MigrationStatus::PartialSuccess);
        assert_eq!(report.migrated_count, 1);
        assert_eq!(report.failed_count, 1);
    }

    #[test]
    fn test_defaults_status_mapping() {
        assert_eq!(defaults::map_status_to_activation_state("pending"), "dormant");
        assert_eq!(defaults::map_status_to_activation_state("active"), "active");
        assert_eq!(defaults::map_status_to_activation_state("completed"), "resolved");
        assert_eq!(defaults::map_status_to_activation_state("paused"), "planted");
    }

    #[test]
    fn test_defaults_priority_mapping() {
        assert_eq!(defaults::map_priority_to_urgency("background"), "background");
        assert_eq!(defaults::map_priority_to_urgency("main"), "upcoming");
        assert_eq!(defaults::map_priority_to_urgency("critical"), "critical");
    }

    #[test]
    fn test_defaults_dramatic_question() {
        let question = defaults::generate_dramatic_question("Save the Princess");
        assert_eq!(question, "Will the party save the princess?");
    }

    #[test]
    fn test_migration_state() {
        let mut state = MigrationState::new();
        state.record_created("id1");
        state.record_created("id2");

        assert!(state.was_created("id1"));
        assert!(state.was_created("id2"));
        assert!(!state.was_created("id3"));
        assert_eq!(state.rollback_ids().len(), 2);
    }

    #[test]
    fn test_migration_options_default() {
        let opts = MigrationOptions::default();
        assert!(opts.campaign_ids.is_none());
        assert!(opts.skip_existing);
        assert!(!opts.dry_run);
        assert!(opts.limit.is_none());
    }

    #[test]
    fn test_migration_options_for_campaign() {
        let opts = MigrationOptions::for_campaign("camp-123").dry_run().with_limit(100);
        assert_eq!(opts.campaign_ids, Some(vec!["camp-123".to_string()]));
        assert!(opts.dry_run);
        assert_eq!(opts.limit, Some(100));
    }

    #[test]
    fn test_validate_migrated_data() {
        // Valid data
        let valid = serde_json::json!({
            "id": "test-id",
            "campaign_id": "camp-1",
            "title": "Test Plot"
        });
        assert!(validate_migrated_data(&valid).is_ok());

        // Missing required field
        let invalid = serde_json::json!({
            "id": "test-id",
            "title": "Test Plot"
        });
        assert!(validate_migrated_data(&invalid).is_err());

        // Invalid tension level
        let invalid_tension = serde_json::json!({
            "id": "test-id",
            "campaign_id": "camp-1",
            "title": "Test Plot",
            "tension_level": 15
        });
        assert!(validate_migrated_data(&invalid_tension).is_err());
    }
}
