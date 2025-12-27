//! Campaign Manager Module
//!
//! Handles TTRPG campaign lifecycle: creation, versioning, rollback, and notes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum CampaignError {
    #[error("Campaign not found: {0}")]
    NotFound(String),

    #[error("Snapshot not found: {0}")]
    SnapshotNotFound(String),

    #[error("Note not found: {0}")]
    NoteNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Maximum snapshots reached for campaign")]
    MaxSnapshotsReached,
}

pub type Result<T> = std::result::Result<T, CampaignError>;

// ============================================================================
// Data Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    pub id: String,
    pub name: String,
    pub system: String,
    pub description: Option<String>,
    pub current_date: String,
    pub notes: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub settings: CampaignSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CampaignSettings {
    /// Maximum number of auto-snapshots to keep
    pub max_auto_snapshots: usize,
    /// Whether to auto-snapshot before major changes
    pub auto_snapshot: bool,
    /// Custom genre/subgenre tags
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignSnapshot {
    pub id: String,
    pub campaign_id: String,
    pub timestamp: DateTime<Utc>,
    pub data: Campaign,
    pub description: String,
    #[serde(default)]
    pub snapshot_type: SnapshotType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum SnapshotType {
    #[default]
    Manual,
    Auto,
    PreRollback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionNote {
    pub id: String,
    pub campaign_id: String,
    pub timestamp: DateTime<Utc>,
    pub content: String,
    pub tags: Vec<String>,
    #[serde(default)]
    pub session_number: Option<u32>,
}

/// Summary of a snapshot for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotSummary {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub description: String,
    pub snapshot_type: SnapshotType,
}

/// Campaign export format for backup/sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignExport {
    pub version: String,
    pub exported_at: DateTime<Utc>,
    pub campaign: Campaign,
    pub snapshots: Vec<CampaignSnapshot>,
    pub notes: Vec<SessionNote>,
}

// ============================================================================
// Campaign Manager
// ============================================================================

const MAX_SNAPSHOTS_DEFAULT: usize = 50;

pub struct CampaignManager {
    campaigns: RwLock<HashMap<String, Campaign>>,
    snapshots: RwLock<HashMap<String, Vec<CampaignSnapshot>>>,
    notes: RwLock<HashMap<String, Vec<SessionNote>>>,
    data_dir: Option<std::path::PathBuf>,
}

impl Default for CampaignManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CampaignManager {
    pub fn new() -> Self {
        Self {
            campaigns: RwLock::new(HashMap::new()),
            snapshots: RwLock::new(HashMap::new()),
            notes: RwLock::new(HashMap::new()),
            data_dir: None,
        }
    }

    /// Create manager with persistent storage directory
    pub fn with_data_dir(data_dir: impl AsRef<Path>) -> Self {
        Self {
            campaigns: RwLock::new(HashMap::new()),
            snapshots: RwLock::new(HashMap::new()),
            notes: RwLock::new(HashMap::new()),
            data_dir: Some(data_dir.as_ref().to_path_buf()),
        }
    }

    // ========================================================================
    // Campaign CRUD
    // ========================================================================

    pub fn create_campaign(&self, name: &str, system: &str) -> Campaign {
        let campaign = Campaign {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            system: system.to_string(),
            description: None,
            current_date: "Session 1".to_string(),
            notes: vec![],
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            settings: CampaignSettings {
                max_auto_snapshots: MAX_SNAPSHOTS_DEFAULT,
                auto_snapshot: true,
                tags: vec![],
            },
        };

        self.campaigns.write().unwrap()
            .insert(campaign.id.clone(), campaign.clone());
        campaign
    }

    pub fn get_campaign(&self, id: &str) -> Option<Campaign> {
        self.campaigns.read().unwrap().get(id).cloned()
    }

    pub fn list_campaigns(&self) -> Vec<Campaign> {
        self.campaigns.read().unwrap().values().cloned().collect()
    }

    pub fn update_campaign(&self, mut campaign: Campaign, auto_snapshot: bool) -> Result<()> {
        let id = campaign.id.clone();

        // Auto-snapshot if enabled
        if auto_snapshot {
            if let Some(existing) = self.get_campaign(&id) {
                if existing.settings.auto_snapshot {
                    let _ = self.create_snapshot_internal(
                        &id,
                        "Auto-snapshot before update",
                        SnapshotType::Auto,
                    );
                }
            }
        }

        campaign.updated_at = Utc::now().to_rfc3339();
        self.campaigns.write().unwrap().insert(id, campaign);
        Ok(())
    }

    pub fn delete_campaign(&self, id: &str) -> Result<()> {
        let mut campaigns = self.campaigns.write().unwrap();
        if campaigns.remove(id).is_none() {
            return Err(CampaignError::NotFound(id.to_string()));
        }

        // Clean up associated data
        self.snapshots.write().unwrap().remove(id);
        self.notes.write().unwrap().remove(id);

        Ok(())
    }

    // ========================================================================
    // Versioning and Rollback
    // ========================================================================

    pub fn create_snapshot(&self, campaign_id: &str, description: &str) -> Result<String> {
        self.create_snapshot_internal(campaign_id, description, SnapshotType::Manual)
    }

    fn create_snapshot_internal(
        &self,
        campaign_id: &str,
        description: &str,
        snapshot_type: SnapshotType,
    ) -> Result<String> {
        let campaigns = self.campaigns.read().unwrap();
        let campaign = campaigns.get(campaign_id)
            .ok_or_else(|| CampaignError::NotFound(campaign_id.to_string()))?;

        let snapshot = CampaignSnapshot {
            id: Uuid::new_v4().to_string(),
            campaign_id: campaign_id.to_string(),
            timestamp: Utc::now(),
            data: campaign.clone(),
            description: description.to_string(),
            snapshot_type: snapshot_type.clone(),
        };

        let mut snapshots = self.snapshots.write().unwrap();
        let campaign_snapshots = snapshots.entry(campaign_id.to_string()).or_default();

        // Enforce max auto-snapshots
        if snapshot_type == SnapshotType::Auto {
            let max = campaign.settings.max_auto_snapshots;
            let auto_count = campaign_snapshots.iter()
                .filter(|s| s.snapshot_type == SnapshotType::Auto)
                .count();

            if auto_count >= max {
                // Remove oldest auto-snapshot
                if let Some(pos) = campaign_snapshots.iter()
                    .position(|s| s.snapshot_type == SnapshotType::Auto)
                {
                    campaign_snapshots.remove(pos);
                }
            }
        }

        let snapshot_id = snapshot.id.clone();
        campaign_snapshots.push(snapshot);

        Ok(snapshot_id)
    }

    pub fn list_snapshots(&self, campaign_id: &str) -> Vec<SnapshotSummary> {
        self.snapshots.read().unwrap()
            .get(campaign_id)
            .map(|snapshots| {
                snapshots.iter()
                    .map(|s| SnapshotSummary {
                        id: s.id.clone(),
                        timestamp: s.timestamp,
                        description: s.description.clone(),
                        snapshot_type: s.snapshot_type.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_snapshot(&self, campaign_id: &str, snapshot_id: &str) -> Option<CampaignSnapshot> {
        self.snapshots.read().unwrap()
            .get(campaign_id)
            .and_then(|snapshots| {
                snapshots.iter().find(|s| s.id == snapshot_id).cloned()
            })
    }

    pub fn restore_snapshot(&self, campaign_id: &str, snapshot_id: &str) -> Result<()> {
        // Create a pre-rollback snapshot first
        let _ = self.create_snapshot_internal(
            campaign_id,
            &format!("Pre-rollback to snapshot {}", snapshot_id),
            SnapshotType::PreRollback,
        );

        let snapshots = self.snapshots.read().unwrap();
        let campaign_snapshots = snapshots.get(campaign_id)
            .ok_or_else(|| CampaignError::NotFound(campaign_id.to_string()))?;

        let snapshot = campaign_snapshots.iter()
            .find(|s| s.id == snapshot_id)
            .ok_or_else(|| CampaignError::SnapshotNotFound(snapshot_id.to_string()))?;

        let mut restored = snapshot.data.clone();
        restored.updated_at = Utc::now().to_rfc3339();

        drop(snapshots); // Release read lock before write

        self.campaigns.write().unwrap()
            .insert(campaign_id.to_string(), restored);

        Ok(())
    }

    pub fn delete_snapshot(&self, campaign_id: &str, snapshot_id: &str) -> Result<()> {
        let mut snapshots = self.snapshots.write().unwrap();
        let campaign_snapshots = snapshots.get_mut(campaign_id)
            .ok_or_else(|| CampaignError::NotFound(campaign_id.to_string()))?;

        let pos = campaign_snapshots.iter()
            .position(|s| s.id == snapshot_id)
            .ok_or_else(|| CampaignError::SnapshotNotFound(snapshot_id.to_string()))?;

        campaign_snapshots.remove(pos);
        Ok(())
    }

    /// Compare two snapshots and return differences
    pub fn diff_snapshots(
        &self,
        campaign_id: &str,
        snapshot_id_a: &str,
        snapshot_id_b: &str,
    ) -> Result<SnapshotDiff> {
        let snapshots = self.snapshots.read().unwrap();
        let campaign_snapshots = snapshots.get(campaign_id)
            .ok_or_else(|| CampaignError::NotFound(campaign_id.to_string()))?;

        let snapshot_a = campaign_snapshots.iter()
            .find(|s| s.id == snapshot_id_a)
            .ok_or_else(|| CampaignError::SnapshotNotFound(snapshot_id_a.to_string()))?;

        let snapshot_b = campaign_snapshots.iter()
            .find(|s| s.id == snapshot_id_b)
            .ok_or_else(|| CampaignError::SnapshotNotFound(snapshot_id_b.to_string()))?;

        Ok(SnapshotDiff {
            snapshot_a_id: snapshot_id_a.to_string(),
            snapshot_b_id: snapshot_id_b.to_string(),
            name_changed: snapshot_a.data.name != snapshot_b.data.name,
            description_changed: snapshot_a.data.description != snapshot_b.data.description,
            current_date_changed: snapshot_a.data.current_date != snapshot_b.data.current_date,
            notes_added: snapshot_b.data.notes.len().saturating_sub(snapshot_a.data.notes.len()),
            notes_removed: snapshot_a.data.notes.len().saturating_sub(snapshot_b.data.notes.len()),
        })
    }

    // ========================================================================
    // Session Notes
    // ========================================================================

    pub fn add_note(
        &self,
        campaign_id: &str,
        content: &str,
        tags: Vec<String>,
        session_number: Option<u32>,
    ) -> SessionNote {
        let note = SessionNote {
            id: Uuid::new_v4().to_string(),
            campaign_id: campaign_id.to_string(),
            timestamp: Utc::now(),
            content: content.to_string(),
            tags,
            session_number,
        };

        self.notes.write().unwrap()
            .entry(campaign_id.to_string())
            .or_default()
            .push(note.clone());
        note
    }

    pub fn get_notes(&self, campaign_id: &str) -> Vec<SessionNote> {
        self.notes.read().unwrap()
            .get(campaign_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_note(&self, campaign_id: &str, note_id: &str) -> Option<SessionNote> {
        self.notes.read().unwrap()
            .get(campaign_id)
            .and_then(|notes| notes.iter().find(|n| n.id == note_id).cloned())
    }

    pub fn update_note(&self, campaign_id: &str, note: SessionNote) -> Result<()> {
        let mut notes = self.notes.write().unwrap();
        let campaign_notes = notes.get_mut(campaign_id)
            .ok_or_else(|| CampaignError::NotFound(campaign_id.to_string()))?;

        let pos = campaign_notes.iter()
            .position(|n| n.id == note.id)
            .ok_or_else(|| CampaignError::NoteNotFound(note.id.clone()))?;

        campaign_notes[pos] = note;
        Ok(())
    }

    pub fn delete_note(&self, campaign_id: &str, note_id: &str) -> Result<()> {
        let mut notes = self.notes.write().unwrap();
        let campaign_notes = notes.get_mut(campaign_id)
            .ok_or_else(|| CampaignError::NotFound(campaign_id.to_string()))?;

        let pos = campaign_notes.iter()
            .position(|n| n.id == note_id)
            .ok_or_else(|| CampaignError::NoteNotFound(note_id.to_string()))?;

        campaign_notes.remove(pos);
        Ok(())
    }

    pub fn search_notes(&self, campaign_id: &str, query: &str, tags: Option<&[String]>) -> Vec<SessionNote> {
        let notes = self.notes.read().unwrap();
        let campaign_notes = match notes.get(campaign_id) {
            Some(n) => n,
            None => return vec![],
        };

        let query_lower = query.to_lowercase();

        campaign_notes.iter()
            .filter(|note| {
                let content_match = query.is_empty() ||
                    note.content.to_lowercase().contains(&query_lower);

                let tag_match = tags.map(|t| {
                    t.iter().any(|tag| note.tags.contains(tag))
                }).unwrap_or(true);

                content_match && tag_match
            })
            .cloned()
            .collect()
    }

    pub fn get_notes_by_session(&self, campaign_id: &str, session_number: u32) -> Vec<SessionNote> {
        self.notes.read().unwrap()
            .get(campaign_id)
            .map(|notes| {
                notes.iter()
                    .filter(|n| n.session_number == Some(session_number))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    // ========================================================================
    // Export / Import
    // ========================================================================

    pub fn export_campaign(&self, campaign_id: &str) -> Result<CampaignExport> {
        let campaign = self.get_campaign(campaign_id)
            .ok_or_else(|| CampaignError::NotFound(campaign_id.to_string()))?;

        let snapshots = self.snapshots.read().unwrap()
            .get(campaign_id)
            .cloned()
            .unwrap_or_default();

        let notes = self.notes.read().unwrap()
            .get(campaign_id)
            .cloned()
            .unwrap_or_default();

        Ok(CampaignExport {
            version: "1.0".to_string(),
            exported_at: Utc::now(),
            campaign,
            snapshots,
            notes,
        })
    }

    pub fn import_campaign(&self, export: CampaignExport, new_id: bool) -> Result<String> {
        let mut campaign = export.campaign;
        let old_id = campaign.id.clone();

        if new_id {
            campaign.id = Uuid::new_v4().to_string();
        }

        let campaign_id = campaign.id.clone();

        // Import campaign
        self.campaigns.write().unwrap()
            .insert(campaign_id.clone(), campaign);

        // Import snapshots with updated campaign_id
        let mut snapshots: Vec<CampaignSnapshot> = export.snapshots.into_iter()
            .map(|mut s| {
                if new_id {
                    s.campaign_id = campaign_id.clone();
                    s.data.id = campaign_id.clone();
                }
                s
            })
            .collect();

        if !snapshots.is_empty() {
            self.snapshots.write().unwrap()
                .insert(campaign_id.clone(), snapshots);
        }

        // Import notes with updated campaign_id
        let notes: Vec<SessionNote> = export.notes.into_iter()
            .map(|mut n| {
                if new_id {
                    n.campaign_id = campaign_id.clone();
                }
                n
            })
            .collect();

        if !notes.is_empty() {
            self.notes.write().unwrap()
                .insert(campaign_id.clone(), notes);
        }

        Ok(campaign_id)
    }

    pub fn export_to_json(&self, campaign_id: &str) -> Result<String> {
        let export = self.export_campaign(campaign_id)?;
        serde_json::to_string_pretty(&export)
            .map_err(|e| CampaignError::SerializationError(e.to_string()))
    }

    pub fn import_from_json(&self, json: &str, new_id: bool) -> Result<String> {
        let export: CampaignExport = serde_json::from_str(json)
            .map_err(|e| CampaignError::SerializationError(e.to_string()))?;
        self.import_campaign(export, new_id)
    }
}

// ============================================================================
// Snapshot Diff
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    pub snapshot_a_id: String,
    pub snapshot_b_id: String,
    pub name_changed: bool,
    pub description_changed: bool,
    pub current_date_changed: bool,
    pub notes_added: usize,
    pub notes_removed: usize,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_campaign_crud() {
        let manager = CampaignManager::new();

        // Create
        let campaign = manager.create_campaign("Lost Mines", "D&D 5e");
        assert_eq!(campaign.name, "Lost Mines");
        assert_eq!(campaign.system, "D&D 5e");

        // Read
        let fetched = manager.get_campaign(&campaign.id).unwrap();
        assert_eq!(fetched.id, campaign.id);

        // List
        let all = manager.list_campaigns();
        assert_eq!(all.len(), 1);

        // Delete
        manager.delete_campaign(&campaign.id).unwrap();
        assert!(manager.get_campaign(&campaign.id).is_none());
    }

    #[test]
    fn test_snapshot_versioning() {
        let manager = CampaignManager::new();
        let campaign = manager.create_campaign("Test Campaign", "Pathfinder");

        // Create snapshot
        let snapshot_id = manager.create_snapshot(&campaign.id, "Before changes").unwrap();

        // Modify campaign
        let mut modified = manager.get_campaign(&campaign.id).unwrap();
        modified.current_date = "Session 5".to_string();
        manager.update_campaign(modified, false).unwrap();

        // Restore snapshot
        manager.restore_snapshot(&campaign.id, &snapshot_id).unwrap();

        // Verify restoration
        let restored = manager.get_campaign(&campaign.id).unwrap();
        assert_eq!(restored.current_date, "Session 1");
    }

    #[test]
    fn test_session_notes() {
        let manager = CampaignManager::new();
        let campaign = manager.create_campaign("Note Test", "Fate");

        // Add notes
        manager.add_note(&campaign.id, "Met the dragon", vec!["combat".into()], Some(1));
        manager.add_note(&campaign.id, "Found treasure", vec!["loot".into()], Some(1));
        manager.add_note(&campaign.id, "Town visit", vec!["roleplay".into()], Some(2));

        // Get all notes
        let notes = manager.get_notes(&campaign.id);
        assert_eq!(notes.len(), 3);

        // Search by content
        let dragon_notes = manager.search_notes(&campaign.id, "dragon", None);
        assert_eq!(dragon_notes.len(), 1);

        // Search by tag
        let combat_notes = manager.search_notes(&campaign.id, "", Some(&["combat".into()]));
        assert_eq!(combat_notes.len(), 1);

        // Get by session
        let session1_notes = manager.get_notes_by_session(&campaign.id, 1);
        assert_eq!(session1_notes.len(), 2);
    }

    #[test]
    fn test_export_import() {
        let manager = CampaignManager::new();
        let campaign = manager.create_campaign("Export Test", "GURPS");
        manager.create_snapshot(&campaign.id, "Snapshot 1").unwrap();
        manager.add_note(&campaign.id, "Test note", vec![], None);

        // Export
        let json = manager.export_to_json(&campaign.id).unwrap();

        // Import with new ID
        let new_id = manager.import_from_json(&json, true).unwrap();
        assert_ne!(new_id, campaign.id);

        // Verify imported data
        let imported = manager.get_campaign(&new_id).unwrap();
        assert_eq!(imported.name, "Export Test");

        let snapshots = manager.list_snapshots(&new_id);
        assert_eq!(snapshots.len(), 1);

        let notes = manager.get_notes(&new_id);
        assert_eq!(notes.len(), 1);
    }
}
