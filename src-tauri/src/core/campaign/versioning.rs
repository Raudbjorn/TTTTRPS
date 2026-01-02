//! Campaign Versioning Module (TASK-006)
//!
//! Provides granular versioning for campaign data with diff tracking,
//! rollback capabilities, and version history management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;
use uuid::Uuid;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum VersionError {
    #[error("Version not found: {0}")]
    NotFound(String),

    #[error("Campaign not found: {0}")]
    CampaignNotFound(String),

    #[error("Invalid version range")]
    InvalidRange,

    #[error("Cannot rollback to current version")]
    RollbackToCurrentVersion,

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Maximum versions reached")]
    MaxVersionsReached,
}

pub type Result<T> = std::result::Result<T, VersionError>;

// ============================================================================
// Data Models
// ============================================================================

/// Type of version/snapshot
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum VersionType {
    /// Manually created by user
    #[default]
    Manual,
    /// Auto-created before significant changes
    Auto,
    /// Created before a rollback operation
    PreRollback,
    /// Milestone version (user-marked as important)
    Milestone,
    /// Import from external source
    Import,
}

/// A campaign version representing a point-in-time snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignVersion {
    /// Unique version identifier
    pub id: String,
    /// Associated campaign ID
    pub campaign_id: String,
    /// Sequential version number within the campaign
    pub version_number: u64,
    /// Human-readable description
    pub description: String,
    /// Type of version
    pub version_type: VersionType,
    /// When this version was created
    pub created_at: DateTime<Utc>,
    /// User who created this version (optional)
    pub created_by: Option<String>,
    /// Serialized campaign data at this version
    pub data_snapshot: String,
    /// Hash of the data for integrity checking
    pub data_hash: String,
    /// Parent version ID (for version tree)
    pub parent_version_id: Option<String>,
    /// Tags for organization
    pub tags: Vec<String>,
    /// Size of the snapshot in bytes
    pub size_bytes: usize,
}

impl CampaignVersion {
    /// Create a new version from campaign data
    pub fn new(
        campaign_id: &str,
        version_number: u64,
        description: &str,
        version_type: VersionType,
        data_snapshot: &str,
        parent_version_id: Option<String>,
    ) -> Self {
        let data_hash = Self::compute_hash(data_snapshot);
        Self {
            id: Uuid::new_v4().to_string(),
            campaign_id: campaign_id.to_string(),
            version_number,
            description: description.to_string(),
            version_type,
            created_at: Utc::now(),
            created_by: None,
            data_snapshot: data_snapshot.to_string(),
            data_hash,
            parent_version_id,
            tags: vec![],
            size_bytes: data_snapshot.len(),
        }
    }

    /// Compute a simple hash for integrity checking
    fn compute_hash(data: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Verify data integrity
    pub fn verify_integrity(&self) -> bool {
        Self::compute_hash(&self.data_snapshot) == self.data_hash
    }
}

/// Summary of a version for listing (without full data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionSummary {
    pub id: String,
    pub campaign_id: String,
    pub version_number: u64,
    pub description: String,
    pub version_type: VersionType,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub tags: Vec<String>,
    pub size_bytes: usize,
}

impl From<&CampaignVersion> for VersionSummary {
    fn from(v: &CampaignVersion) -> Self {
        Self {
            id: v.id.clone(),
            campaign_id: v.campaign_id.clone(),
            version_number: v.version_number,
            description: v.description.clone(),
            version_type: v.version_type.clone(),
            created_at: v.created_at,
            created_by: v.created_by.clone(),
            tags: v.tags.clone(),
            size_bytes: v.size_bytes,
        }
    }
}

// ============================================================================
// Diff Types
// ============================================================================

/// Type of change in a diff
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiffOperation {
    Added,
    Removed,
    Modified,
}

/// A single entry in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    /// JSON path to the changed field (e.g., "settings.theme")
    pub path: String,
    /// Type of operation
    pub operation: DiffOperation,
    /// Old value (for Modified/Removed)
    pub old_value: Option<serde_json::Value>,
    /// New value (for Modified/Added)
    pub new_value: Option<serde_json::Value>,
}

/// Comparison result between two versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignDiff {
    /// Source version ID
    pub from_version_id: String,
    /// Target version ID
    pub to_version_id: String,
    /// Source version number
    pub from_version_number: u64,
    /// Target version number
    pub to_version_number: u64,
    /// List of changes
    pub changes: Vec<DiffEntry>,
    /// Summary statistics
    pub stats: DiffStats,
}

/// Statistics about a diff
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiffStats {
    pub added_count: usize,
    pub removed_count: usize,
    pub modified_count: usize,
    pub total_changes: usize,
}

impl CampaignDiff {
    /// Create a diff between two JSON values
    pub fn compute(
        from_version: &CampaignVersion,
        to_version: &CampaignVersion,
    ) -> Result<Self> {
        let from_json: serde_json::Value = serde_json::from_str(&from_version.data_snapshot)
            .map_err(|e| VersionError::SerializationError(e.to_string()))?;
        let to_json: serde_json::Value = serde_json::from_str(&to_version.data_snapshot)
            .map_err(|e| VersionError::SerializationError(e.to_string()))?;

        let changes = Self::diff_values(&from_json, &to_json, "");
        let stats = DiffStats {
            added_count: changes.iter().filter(|c| c.operation == DiffOperation::Added).count(),
            removed_count: changes.iter().filter(|c| c.operation == DiffOperation::Removed).count(),
            modified_count: changes.iter().filter(|c| c.operation == DiffOperation::Modified).count(),
            total_changes: changes.len(),
        };

        Ok(Self {
            from_version_id: from_version.id.clone(),
            to_version_id: to_version.id.clone(),
            from_version_number: from_version.version_number,
            to_version_number: to_version.version_number,
            changes,
            stats,
        })
    }

    /// Recursively diff two JSON values
    fn diff_values(from: &serde_json::Value, to: &serde_json::Value, path: &str) -> Vec<DiffEntry> {
        let mut changes = Vec::new();

        match (from, to) {
            (serde_json::Value::Object(from_map), serde_json::Value::Object(to_map)) => {
                // Check for removed/modified keys
                for (key, from_val) in from_map {
                    let new_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };

                    if let Some(to_val) = to_map.get(key) {
                        if from_val != to_val {
                            // Recurse for nested objects
                            if from_val.is_object() && to_val.is_object() {
                                changes.extend(Self::diff_values(from_val, to_val, &new_path));
                            } else {
                                changes.push(DiffEntry {
                                    path: new_path,
                                    operation: DiffOperation::Modified,
                                    old_value: Some(from_val.clone()),
                                    new_value: Some(to_val.clone()),
                                });
                            }
                        }
                    } else {
                        changes.push(DiffEntry {
                            path: new_path,
                            operation: DiffOperation::Removed,
                            old_value: Some(from_val.clone()),
                            new_value: None,
                        });
                    }
                }

                // Check for added keys
                for (key, to_val) in to_map {
                    if !from_map.contains_key(key) {
                        let new_path = if path.is_empty() {
                            key.clone()
                        } else {
                            format!("{}.{}", path, key)
                        };
                        changes.push(DiffEntry {
                            path: new_path,
                            operation: DiffOperation::Added,
                            old_value: None,
                            new_value: Some(to_val.clone()),
                        });
                    }
                }
            }
            (serde_json::Value::Array(from_arr), serde_json::Value::Array(to_arr)) => {
                // Simple array comparison - just note if different
                if from_arr != to_arr {
                    changes.push(DiffEntry {
                        path: path.to_string(),
                        operation: DiffOperation::Modified,
                        old_value: Some(serde_json::Value::Array(from_arr.clone())),
                        new_value: Some(serde_json::Value::Array(to_arr.clone())),
                    });
                }
            }
            _ => {
                if from != to {
                    changes.push(DiffEntry {
                        path: path.to_string(),
                        operation: DiffOperation::Modified,
                        old_value: Some(from.clone()),
                        new_value: Some(to.clone()),
                    });
                }
            }
        }

        changes
    }
}

// ============================================================================
// Version Manager
// ============================================================================

/// Configuration for the version manager
#[derive(Debug, Clone)]
pub struct VersionManagerConfig {
    /// Maximum versions per campaign (0 = unlimited)
    pub max_versions_per_campaign: usize,
    /// Maximum auto-versions (older auto-versions are pruned)
    pub max_auto_versions: usize,
    /// Whether to compress version data
    pub compress_data: bool,
}

impl Default for VersionManagerConfig {
    fn default() -> Self {
        Self {
            max_versions_per_campaign: 100,
            max_auto_versions: 20,
            compress_data: false,
        }
    }
}

/// Manages campaign versions with CRUD operations
pub struct VersionManager {
    /// Campaign ID -> Vec<Version> (ordered by version_number)
    versions: RwLock<HashMap<String, Vec<CampaignVersion>>>,
    /// Campaign ID -> current version number counter
    version_counters: RwLock<HashMap<String, u64>>,
    /// Configuration
    config: VersionManagerConfig,
}

impl Default for VersionManager {
    fn default() -> Self {
        Self::new(VersionManagerConfig::default())
    }
}

impl VersionManager {
    /// Create a new version manager
    pub fn new(config: VersionManagerConfig) -> Self {
        Self {
            versions: RwLock::new(HashMap::new()),
            version_counters: RwLock::new(HashMap::new()),
            config,
        }
    }

    // ========================================================================
    // Version CRUD
    // ========================================================================

    /// Create a new version for a campaign
    pub fn create_version(
        &self,
        campaign_id: &str,
        description: &str,
        version_type: VersionType,
        data_snapshot: &str,
    ) -> Result<CampaignVersion> {
        // Get next version number
        let version_number = {
            let mut counters = self.version_counters.write().unwrap();
            let counter = counters.entry(campaign_id.to_string()).or_insert(0);
            *counter += 1;
            *counter
        };

        // Get parent version
        let parent_version_id = self
            .get_latest_version(campaign_id)
            .map(|v| v.id.clone());

        let version = CampaignVersion::new(
            campaign_id,
            version_number,
            description,
            version_type.clone(),
            data_snapshot,
            parent_version_id,
        );

        // Store version
        {
            let mut versions = self.versions.write().unwrap();
            let campaign_versions = versions.entry(campaign_id.to_string()).or_default();

            // Check max versions
            if self.config.max_versions_per_campaign > 0
                && campaign_versions.len() >= self.config.max_versions_per_campaign
            {
                return Err(VersionError::MaxVersionsReached);
            }

            // Prune old auto-versions if needed
            if version_type == VersionType::Auto {
                let auto_count = campaign_versions
                    .iter()
                    .filter(|v| v.version_type == VersionType::Auto)
                    .count();

                if auto_count >= self.config.max_auto_versions {
                    // Remove oldest auto-version
                    if let Some(pos) = campaign_versions
                        .iter()
                        .position(|v| v.version_type == VersionType::Auto)
                    {
                        campaign_versions.remove(pos);
                    }
                }
            }

            campaign_versions.push(version.clone());
        }

        Ok(version)
    }

    /// Get a specific version by ID
    pub fn get_version(&self, campaign_id: &str, version_id: &str) -> Option<CampaignVersion> {
        self.versions
            .read()
            .unwrap()
            .get(campaign_id)
            .and_then(|versions| versions.iter().find(|v| v.id == version_id).cloned())
    }

    /// Get the latest version for a campaign
    pub fn get_latest_version(&self, campaign_id: &str) -> Option<CampaignVersion> {
        self.versions
            .read()
            .unwrap()
            .get(campaign_id)
            .and_then(|versions| versions.last().cloned())
    }

    /// Get a version by version number
    pub fn get_version_by_number(&self, campaign_id: &str, version_number: u64) -> Option<CampaignVersion> {
        self.versions
            .read()
            .unwrap()
            .get(campaign_id)
            .and_then(|versions| {
                versions
                    .iter()
                    .find(|v| v.version_number == version_number)
                    .cloned()
            })
    }

    /// List all versions for a campaign (returns summaries)
    pub fn list_versions(&self, campaign_id: &str) -> Vec<VersionSummary> {
        self.versions
            .read()
            .unwrap()
            .get(campaign_id)
            .map(|versions| versions.iter().map(VersionSummary::from).collect())
            .unwrap_or_default()
    }

    /// Delete a specific version
    pub fn delete_version(&self, campaign_id: &str, version_id: &str) -> Result<()> {
        let mut versions = self.versions.write().unwrap();
        let campaign_versions = versions
            .get_mut(campaign_id)
            .ok_or_else(|| VersionError::CampaignNotFound(campaign_id.to_string()))?;

        let pos = campaign_versions
            .iter()
            .position(|v| v.id == version_id)
            .ok_or_else(|| VersionError::NotFound(version_id.to_string()))?;

        campaign_versions.remove(pos);
        Ok(())
    }

    /// Delete all versions for a campaign
    pub fn delete_all_versions(&self, campaign_id: &str) {
        self.versions.write().unwrap().remove(campaign_id);
        self.version_counters.write().unwrap().remove(campaign_id);
    }

    // ========================================================================
    // Comparison and Diff
    // ========================================================================

    /// Compare two versions and return the diff
    pub fn compare_versions(
        &self,
        campaign_id: &str,
        from_version_id: &str,
        to_version_id: &str,
    ) -> Result<CampaignDiff> {
        let from_version = self
            .get_version(campaign_id, from_version_id)
            .ok_or_else(|| VersionError::NotFound(from_version_id.to_string()))?;

        let to_version = self
            .get_version(campaign_id, to_version_id)
            .ok_or_else(|| VersionError::NotFound(to_version_id.to_string()))?;

        CampaignDiff::compute(&from_version, &to_version)
    }

    /// Get changes between the current state and a specific version
    pub fn diff_from_current(
        &self,
        campaign_id: &str,
        version_id: &str,
        current_data: &str,
    ) -> Result<CampaignDiff> {
        let version = self
            .get_version(campaign_id, version_id)
            .ok_or_else(|| VersionError::NotFound(version_id.to_string()))?;

        // Create a temporary "current" version for comparison
        let current_version = CampaignVersion::new(
            campaign_id,
            u64::MAX, // Placeholder
            "Current",
            VersionType::Manual,
            current_data,
            None,
        );

        CampaignDiff::compute(&version, &current_version)
    }

    // ========================================================================
    // Rollback Operations
    // ========================================================================

    /// Prepare a rollback by creating a pre-rollback version and returning the target data
    pub fn prepare_rollback(
        &self,
        campaign_id: &str,
        target_version_id: &str,
        current_data: &str,
    ) -> Result<String> {
        // Get target version
        let target = self
            .get_version(campaign_id, target_version_id)
            .ok_or_else(|| VersionError::NotFound(target_version_id.to_string()))?;

        // Create pre-rollback snapshot
        self.create_version(
            campaign_id,
            &format!("Pre-rollback to version {}", target.version_number),
            VersionType::PreRollback,
            current_data,
        )?;

        Ok(target.data_snapshot)
    }

    /// Get the version count for a campaign
    pub fn version_count(&self, campaign_id: &str) -> usize {
        self.versions
            .read()
            .unwrap()
            .get(campaign_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    // ========================================================================
    // Tagging
    // ========================================================================

    /// Add a tag to a version
    pub fn add_tag(&self, campaign_id: &str, version_id: &str, tag: &str) -> Result<()> {
        let mut versions = self.versions.write().unwrap();
        let campaign_versions = versions
            .get_mut(campaign_id)
            .ok_or_else(|| VersionError::CampaignNotFound(campaign_id.to_string()))?;

        let version = campaign_versions
            .iter_mut()
            .find(|v| v.id == version_id)
            .ok_or_else(|| VersionError::NotFound(version_id.to_string()))?;

        if !version.tags.contains(&tag.to_string()) {
            version.tags.push(tag.to_string());
        }

        Ok(())
    }

    /// Remove a tag from a version
    pub fn remove_tag(&self, campaign_id: &str, version_id: &str, tag: &str) -> Result<()> {
        let mut versions = self.versions.write().unwrap();
        let campaign_versions = versions
            .get_mut(campaign_id)
            .ok_or_else(|| VersionError::CampaignNotFound(campaign_id.to_string()))?;

        let version = campaign_versions
            .iter_mut()
            .find(|v| v.id == version_id)
            .ok_or_else(|| VersionError::NotFound(version_id.to_string()))?;

        version.tags.retain(|t| t != tag);
        Ok(())
    }

    /// Mark a version as a milestone
    pub fn mark_as_milestone(&self, campaign_id: &str, version_id: &str) -> Result<()> {
        let mut versions = self.versions.write().unwrap();
        let campaign_versions = versions
            .get_mut(campaign_id)
            .ok_or_else(|| VersionError::CampaignNotFound(campaign_id.to_string()))?;

        let version = campaign_versions
            .iter_mut()
            .find(|v| v.id == version_id)
            .ok_or_else(|| VersionError::NotFound(version_id.to_string()))?;

        version.version_type = VersionType::Milestone;
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_campaign_data(name: &str) -> String {
        format!(r#"{{"name":"{}","system":"D&D 5e","description":"Test"}}"#, name)
    }

    #[test]
    fn test_create_and_get_version() {
        let manager = VersionManager::default();

        let version = manager
            .create_version("camp-1", "Initial version", VersionType::Manual, &sample_campaign_data("Test"))
            .unwrap();

        assert_eq!(version.version_number, 1);
        assert!(version.verify_integrity());

        let retrieved = manager.get_version("camp-1", &version.id).unwrap();
        assert_eq!(retrieved.id, version.id);
    }

    #[test]
    fn test_version_numbering() {
        let manager = VersionManager::default();

        let v1 = manager
            .create_version("camp-1", "v1", VersionType::Manual, &sample_campaign_data("v1"))
            .unwrap();
        let v2 = manager
            .create_version("camp-1", "v2", VersionType::Manual, &sample_campaign_data("v2"))
            .unwrap();
        let v3 = manager
            .create_version("camp-1", "v3", VersionType::Manual, &sample_campaign_data("v3"))
            .unwrap();

        assert_eq!(v1.version_number, 1);
        assert_eq!(v2.version_number, 2);
        assert_eq!(v3.version_number, 3);
        assert_eq!(v2.parent_version_id, Some(v1.id));
        assert_eq!(v3.parent_version_id, Some(v2.id));
    }

    #[test]
    fn test_list_versions() {
        let manager = VersionManager::default();

        manager
            .create_version("camp-1", "v1", VersionType::Manual, &sample_campaign_data("v1"))
            .unwrap();
        manager
            .create_version("camp-1", "v2", VersionType::Auto, &sample_campaign_data("v2"))
            .unwrap();

        let summaries = manager.list_versions("camp-1");
        assert_eq!(summaries.len(), 2);
    }

    #[test]
    fn test_compare_versions() {
        let manager = VersionManager::default();

        let v1 = manager
            .create_version(
                "camp-1",
                "v1",
                VersionType::Manual,
                r#"{"name":"Original","level":1}"#,
            )
            .unwrap();

        let v2 = manager
            .create_version(
                "camp-1",
                "v2",
                VersionType::Manual,
                r#"{"name":"Modified","level":2,"new_field":"added"}"#,
            )
            .unwrap();

        let diff = manager.compare_versions("camp-1", &v1.id, &v2.id).unwrap();

        assert_eq!(diff.stats.modified_count, 2); // name and level
        assert_eq!(diff.stats.added_count, 1); // new_field
        assert_eq!(diff.stats.total_changes, 3);
    }

    #[test]
    fn test_auto_version_pruning() {
        let config = VersionManagerConfig {
            max_auto_versions: 2,
            ..Default::default()
        };
        let manager = VersionManager::new(config);

        for i in 0..5 {
            manager
                .create_version(
                    "camp-1",
                    &format!("auto-{}", i),
                    VersionType::Auto,
                    &sample_campaign_data(&format!("auto-{}", i)),
                )
                .unwrap();
        }

        let versions = manager.list_versions("camp-1");
        let auto_count = versions
            .iter()
            .filter(|v| v.version_type == VersionType::Auto)
            .count();

        assert_eq!(auto_count, 2);
    }

    #[test]
    fn test_tagging() {
        let manager = VersionManager::default();

        let v = manager
            .create_version("camp-1", "v1", VersionType::Manual, &sample_campaign_data("v1"))
            .unwrap();

        manager.add_tag("camp-1", &v.id, "important").unwrap();
        manager.add_tag("camp-1", &v.id, "session-5").unwrap();

        let retrieved = manager.get_version("camp-1", &v.id).unwrap();
        assert!(retrieved.tags.contains(&"important".to_string()));
        assert!(retrieved.tags.contains(&"session-5".to_string()));

        manager.remove_tag("camp-1", &v.id, "important").unwrap();
        let retrieved = manager.get_version("camp-1", &v.id).unwrap();
        assert!(!retrieved.tags.contains(&"important".to_string()));
    }
}
