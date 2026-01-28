//! Acceptance Manager - Draft lifecycle management
//!
//! Phase 4, Task 4.10: Implement AcceptanceManager
//!
//! Manages the lifecycle of generated drafts from Draft -> Approved -> Canonical,
//! with full audit trail logging.
//!
//! Note: Database persistence methods are stubbed pending implementation of
//! generation_drafts, canon_status_log, and acceptance_events tables.

use crate::core::campaign::pipeline::PipelineError;
use crate::database::{CanonStatus, Database};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during acceptance operations
#[derive(Debug, thiserror::Error)]
pub enum AcceptanceError {
    #[error("Draft not found: {0}")]
    DraftNotFound(String),

    #[error("Invalid status transition: cannot move from {from} to {to}")]
    InvalidTransition { from: String, to: String },

    #[error("Draft is locked and cannot be modified")]
    DraftLocked,

    #[error("Database error: {0}")]
    Database(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Entity creation failed: {0}")]
    EntityCreation(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

impl From<sqlx::Error> for AcceptanceError {
    fn from(e: sqlx::Error) -> Self {
        AcceptanceError::Database(e.to_string())
    }
}

impl From<PipelineError> for AcceptanceError {
    fn from(e: PipelineError) -> Self {
        match e {
            PipelineError::Database(e) => AcceptanceError::Database(e.to_string()),
            PipelineError::InvalidTransition { from, to } => {
                AcceptanceError::InvalidTransition { from, to }
            }
            PipelineError::DraftLocked => AcceptanceError::DraftLocked,
            _ => AcceptanceError::Validation(e.to_string()),
        }
    }
}

// ============================================================================
// Types
// ============================================================================

/// Action to take on a draft
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum DraftAction {
    /// Approve the draft as-is
    Approve,
    /// Reject the draft
    Reject { reason: String },
    /// Modify the draft with changes
    Modify { modifications: serde_json::Value },
    /// Apply the draft to the campaign as a real entity
    Apply { entity_type: String },
}

/// Result of applying a draft to the campaign
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedEntity {
    /// The new entity's ID
    pub entity_id: String,
    /// The entity type (npc, location, etc.)
    pub entity_type: String,
    /// The draft that was applied
    pub draft_id: String,
    /// Campaign the entity was added to
    pub campaign_id: String,
    /// Timestamp of application
    pub applied_at: String,
}

/// In-memory draft record (used until database persistence is implemented)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InMemoryDraft {
    pub id: String,
    pub entity_type: String,
    pub data: serde_json::Value,
    pub status: CanonStatus,
    pub campaign_id: Option<String>,
    pub wizard_id: Option<String>,
    pub trust_level: String,
    pub trust_confidence: f32,
    pub citations: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub applied_entity_id: Option<String>,
}

impl InMemoryDraft {
    pub fn new(id: String, entity_type: String, data: serde_json::Value) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            entity_type,
            data,
            status: CanonStatus::Draft,
            campaign_id: None,
            wizard_id: None,
            trust_level: "creative".to_string(),
            trust_confidence: 0.5,
            citations: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
            applied_entity_id: None,
        }
    }

    pub fn with_campaign(mut self, campaign_id: String) -> Self {
        self.campaign_id = Some(campaign_id);
        self
    }

    pub fn with_wizard(mut self, wizard_id: String) -> Self {
        self.wizard_id = Some(wizard_id);
        self
    }

    pub fn is_editable(&self) -> bool {
        matches!(self.status, CanonStatus::Draft | CanonStatus::Approved)
    }
}

// ============================================================================
// Acceptance Manager
// ============================================================================

/// Manages the lifecycle of generated drafts
///
/// Note: Currently uses in-memory storage. Database persistence will be
/// implemented when the generation_drafts table is added.
pub struct AcceptanceManager {
    #[allow(dead_code)]
    database: Database,
    /// In-memory draft storage (temporary until database methods are implemented)
    drafts: Arc<RwLock<HashMap<String, InMemoryDraft>>>,
}

impl AcceptanceManager {
    /// Create a new acceptance manager
    pub fn new(database: Database) -> Self {
        Self {
            database,
            drafts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get a draft by ID
    pub async fn get_draft(&self, draft_id: &str) -> Result<InMemoryDraft, AcceptanceError> {
        let drafts = self.drafts.read().await;
        drafts
            .get(draft_id)
            .cloned()
            .ok_or_else(|| AcceptanceError::DraftNotFound(draft_id.to_string()))
    }

    /// Create a new draft
    pub async fn create_draft(
        &self,
        entity_type: &str,
        data: serde_json::Value,
        campaign_id: Option<&str>,
        wizard_id: Option<&str>,
    ) -> Result<String, AcceptanceError> {
        let draft_id = uuid::Uuid::new_v4().to_string();

        let mut draft = InMemoryDraft::new(draft_id.clone(), entity_type.to_string(), data);

        if let Some(cid) = campaign_id {
            draft = draft.with_campaign(cid.to_string());
        }
        if let Some(wid) = wizard_id {
            draft = draft.with_wizard(wid.to_string());
        }

        let mut drafts = self.drafts.write().await;
        drafts.insert(draft_id.clone(), draft);

        tracing::debug!(draft_id = %draft_id, entity_type, "Draft created (in-memory)");

        Ok(draft_id)
    }

    /// Approve a draft
    pub async fn approve_draft(
        &self,
        draft_id: &str,
        _reason: Option<&str>,
    ) -> Result<InMemoryDraft, AcceptanceError> {
        let mut drafts = self.drafts.write().await;

        let draft = drafts
            .get_mut(draft_id)
            .ok_or_else(|| AcceptanceError::DraftNotFound(draft_id.to_string()))?;

        // Validate transition
        if draft.status != CanonStatus::Draft {
            return Err(AcceptanceError::InvalidTransition {
                from: format!("{:?}", draft.status),
                to: "approved".to_string(),
            });
        }

        draft.status = CanonStatus::Approved;
        draft.updated_at = chrono::Utc::now().to_rfc3339();

        tracing::info!(draft_id, "Draft approved (in-memory)");

        Ok(draft.clone())
    }

    /// Reject a draft
    pub async fn reject_draft(
        &self,
        draft_id: &str,
        _reason: &str,
    ) -> Result<InMemoryDraft, AcceptanceError> {
        let mut drafts = self.drafts.write().await;

        let draft = drafts
            .get(draft_id)
            .ok_or_else(|| AcceptanceError::DraftNotFound(draft_id.to_string()))?
            .clone();

        // Only drafts can be rejected
        if draft.status != CanonStatus::Draft {
            return Err(AcceptanceError::InvalidTransition {
                from: format!("{:?}", draft.status),
                to: "rejected".to_string(),
            });
        }

        // Remove from in-memory storage (rejection = deletion)
        drafts.remove(draft_id);

        tracing::info!(draft_id, "Draft rejected and removed (in-memory)");

        Ok(draft)
    }

    /// Modify a draft with changes
    pub async fn modify_draft(
        &self,
        draft_id: &str,
        modifications: serde_json::Value,
        _reason: Option<&str>,
    ) -> Result<InMemoryDraft, AcceptanceError> {
        let mut drafts = self.drafts.write().await;

        let draft = drafts
            .get_mut(draft_id)
            .ok_or_else(|| AcceptanceError::DraftNotFound(draft_id.to_string()))?;

        // Check if draft is editable
        if !draft.is_editable() {
            return Err(AcceptanceError::DraftLocked);
        }

        // Merge modifications into existing data
        match (draft.data.as_object_mut(), modifications.as_object()) {
            (Some(base), Some(mods)) => {
                for (key, value) in mods {
                    base.insert(key.clone(), value.clone());
                }
            }
            (None, _) => {
                tracing::warn!(
                    draft_id,
                    "Cannot merge modifications: draft.data is not a JSON object"
                );
                return Err(AcceptanceError::InvalidData(
                    "Draft data must be a JSON object for merge".to_string(),
                ));
            }
            (_, None) => {
                tracing::warn!(
                    draft_id,
                    "Cannot merge modifications: modifications is not a JSON object"
                );
                return Err(AcceptanceError::InvalidData(
                    "Modifications must be a JSON object for merge".to_string(),
                ));
            }
        }

        draft.updated_at = chrono::Utc::now().to_rfc3339();

        tracing::debug!(draft_id, "Draft modified (in-memory)");

        Ok(draft.clone())
    }

    /// Apply an approved draft to the campaign as a real entity
    pub async fn apply_to_campaign(
        &self,
        draft_id: &str,
    ) -> Result<AppliedEntity, AcceptanceError> {
        let mut drafts = self.drafts.write().await;

        let draft = drafts
            .get_mut(draft_id)
            .ok_or_else(|| AcceptanceError::DraftNotFound(draft_id.to_string()))?;

        // Must be approved to apply
        if draft.status != CanonStatus::Approved {
            return Err(AcceptanceError::InvalidTransition {
                from: format!("{:?}", draft.status),
                to: "applied (must be approved first)".to_string(),
            });
        }

        let campaign_id = draft
            .campaign_id
            .clone()
            .ok_or_else(|| AcceptanceError::Validation("Draft has no campaign_id".to_string()))?;

        // Generate entity ID (actual entity creation is deferred to database implementation)
        let entity_id = uuid::Uuid::new_v4().to_string();

        draft.status = CanonStatus::Canonical;
        draft.applied_entity_id = Some(entity_id.clone());
        draft.updated_at = chrono::Utc::now().to_rfc3339();

        let applied_at = draft.updated_at.clone();
        let entity_type = draft.entity_type.clone();

        tracing::info!(
            draft_id,
            entity_id = %entity_id,
            entity_type = %entity_type,
            "Draft applied to campaign (in-memory)"
        );

        // TODO: Actually create the entity in the database when methods are available

        Ok(AppliedEntity {
            entity_id,
            entity_type,
            draft_id: draft_id.to_string(),
            campaign_id,
            applied_at,
        })
    }

    /// Revert an applied draft back to approved status
    pub async fn revert_to_approved(
        &self,
        draft_id: &str,
        _reason: &str,
    ) -> Result<InMemoryDraft, AcceptanceError> {
        let mut drafts = self.drafts.write().await;

        let draft = drafts
            .get_mut(draft_id)
            .ok_or_else(|| AcceptanceError::DraftNotFound(draft_id.to_string()))?;

        // Must be canonical to revert
        if draft.status != CanonStatus::Canonical {
            return Err(AcceptanceError::InvalidTransition {
                from: format!("{:?}", draft.status),
                to: "approved (revert)".to_string(),
            });
        }

        // TODO: Delete the created entity when database methods are available

        draft.status = CanonStatus::Approved;
        draft.applied_entity_id = None;
        draft.updated_at = chrono::Utc::now().to_rfc3339();

        tracing::info!(draft_id, "Draft reverted to approved (in-memory)");

        Ok(draft.clone())
    }

    /// Deprecate a canonical entity
    pub async fn deprecate(
        &self,
        draft_id: &str,
        _reason: &str,
    ) -> Result<(), AcceptanceError> {
        let mut drafts = self.drafts.write().await;

        let draft = drafts
            .get_mut(draft_id)
            .ok_or_else(|| AcceptanceError::DraftNotFound(draft_id.to_string()))?;

        // Only canonical can be deprecated
        if draft.status != CanonStatus::Canonical {
            return Err(AcceptanceError::InvalidTransition {
                from: format!("{:?}", draft.status),
                to: "deprecated".to_string(),
            });
        }

        draft.status = CanonStatus::Deprecated;
        draft.updated_at = chrono::Utc::now().to_rfc3339();

        tracing::info!(draft_id, "Draft deprecated (in-memory)");

        Ok(())
    }

    /// List drafts for a campaign
    pub async fn list_campaign_drafts(
        &self,
        campaign_id: &str,
        status_filter: Option<CanonStatus>,
    ) -> Result<Vec<InMemoryDraft>, AcceptanceError> {
        let drafts = self.drafts.read().await;

        let filtered: Vec<InMemoryDraft> = drafts
            .values()
            .filter(|d| d.campaign_id.as_deref() == Some(campaign_id))
            .filter(|d| {
                if let Some(status) = &status_filter {
                    d.status == *status
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        Ok(filtered)
    }

    /// Delete a draft (only draft/rejected status)
    pub async fn delete_draft(&self, draft_id: &str) -> Result<(), AcceptanceError> {
        let mut drafts = self.drafts.write().await;

        let draft = drafts
            .get(draft_id)
            .ok_or_else(|| AcceptanceError::DraftNotFound(draft_id.to_string()))?;

        // Only allow deletion of draft status
        if draft.status != CanonStatus::Draft {
            return Err(AcceptanceError::DraftLocked);
        }

        drafts.remove(draft_id);

        tracing::debug!(draft_id, "Draft deleted (in-memory)");

        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draft_action_serialization() {
        let approve = DraftAction::Approve;
        let json = serde_json::to_string(&approve).unwrap();
        assert!(json.contains("approve"));

        let reject = DraftAction::Reject {
            reason: "Not suitable".to_string(),
        };
        let json = serde_json::to_string(&reject).unwrap();
        assert!(json.contains("reject"));
        assert!(json.contains("Not suitable"));
    }

    #[test]
    fn test_applied_entity() {
        let applied = AppliedEntity {
            entity_id: "entity-1".to_string(),
            entity_type: "npc".to_string(),
            draft_id: "draft-1".to_string(),
            campaign_id: "camp-1".to_string(),
            applied_at: "2024-01-01T00:00:00Z".to_string(),
        };

        assert_eq!(applied.entity_type, "npc");
    }

    #[test]
    fn test_in_memory_draft_creation() {
        let draft = InMemoryDraft::new(
            "draft-1".to_string(),
            "npc".to_string(),
            serde_json::json!({"name": "Test NPC"}),
        );

        assert_eq!(draft.status, CanonStatus::Draft);
        assert!(draft.is_editable());
    }

    #[test]
    fn test_in_memory_draft_with_campaign() {
        let draft = InMemoryDraft::new(
            "draft-1".to_string(),
            "npc".to_string(),
            serde_json::json!({}),
        )
        .with_campaign("camp-1".to_string())
        .with_wizard("wizard-1".to_string());

        assert_eq!(draft.campaign_id, Some("camp-1".to_string()));
        assert_eq!(draft.wizard_id, Some("wizard-1".to_string()));
    }
}
