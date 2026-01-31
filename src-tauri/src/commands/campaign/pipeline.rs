//! Pipeline Commands Module
//!
//! Tauri commands for the campaign intelligence pipeline.
//! Provides the frontend interface for draft management, trust assignment,
//! and content acceptance workflow.

use tauri::State;
use tracing::{debug, error, info, warn};

use crate::commands::AppState;
use crate::core::campaign::generation::{
    AcceptanceError, AcceptanceManager, AppliedEntity, InMemoryDraft,
    TrustAssigner, TrustAssignment,
};
use crate::core::campaign::pipeline::TrustLevel;
use crate::database::{CanonStatus, Citation};

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert pipeline errors to String for Tauri IPC
fn pipeline_err_to_string(err: impl std::fmt::Display) -> String {
    let msg = err.to_string();
    error!(error = %msg, "Pipeline command error");
    msg
}

/// Create an AcceptanceManager from AppState
fn get_acceptance_manager(state: &State<'_, AppState>) -> AcceptanceManager {
    AcceptanceManager::new(state.database.clone())
}

/// Create a TrustAssigner
fn get_trust_assigner(_state: &State<'_, AppState>) -> TrustAssigner {
    TrustAssigner::new()
}

// ============================================================================
// Draft CRUD Commands
// ============================================================================

/// Get a draft by ID.
///
/// # Arguments
/// * `draft_id` - The draft's unique identifier
///
/// # Returns
/// The draft record if found
#[tauri::command]
pub async fn get_draft(
    draft_id: String,
    state: State<'_, AppState>,
) -> Result<Option<InMemoryDraft>, String> {
    debug!(draft_id = %draft_id, "Getting draft");

    let manager = get_acceptance_manager(&state);
    match manager.get_draft(&draft_id).await {
        Ok(draft) => Ok(Some(draft)),
        Err(AcceptanceError::DraftNotFound(_)) => Ok(None),
        Err(e) => Err(pipeline_err_to_string(e)),
    }
}

/// List drafts for a campaign.
///
/// # Arguments
/// * `campaign_id` - The campaign's unique identifier
/// * `status_filter` - Optional status to filter by
///
/// # Returns
/// List of draft records
#[tauri::command]
pub async fn list_campaign_drafts(
    campaign_id: String,
    status_filter: Option<CanonStatus>,
    state: State<'_, AppState>,
) -> Result<Vec<InMemoryDraft>, String> {
    debug!(
        campaign_id = %campaign_id,
        status = ?status_filter,
        "Listing campaign drafts"
    );

    let manager = get_acceptance_manager(&state);
    manager
        .list_campaign_drafts(&campaign_id, status_filter)
        .await
        .map_err(pipeline_err_to_string)
}

/// List drafts for a wizard.
///
/// Note: Not yet fully implemented - would need wizard-specific filtering.
///
/// # Arguments
/// * `wizard_id` - The wizard's unique identifier
///
/// # Returns
/// List of draft records associated with the wizard
#[tauri::command]
pub async fn list_wizard_drafts(
    wizard_id: String,
    _state: State<'_, AppState>,
) -> Result<Vec<InMemoryDraft>, String> {
    debug!(wizard_id = %wizard_id, "Listing wizard drafts (not implemented)");

    // TODO: Implement wizard-specific draft filtering when in-memory storage
    // supports wizard_id queries
    Ok(Vec::new())
}

/// Create a new draft manually.
///
/// # Arguments
/// * `entity_type` - Type of entity (npc, location, etc.)
/// * `data` - Entity data as JSON
/// * `campaign_id` - Optional campaign ID
/// * `wizard_id` - Optional wizard ID
///
/// # Returns
/// The created draft ID
#[tauri::command]
pub async fn create_draft(
    entity_type: String,
    data: serde_json::Value,
    campaign_id: Option<String>,
    wizard_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    info!(
        entity_type = %entity_type,
        campaign_id = ?campaign_id,
        "Creating draft"
    );

    let manager = get_acceptance_manager(&state);
    manager
        .create_draft(
            &entity_type,
            data,
            campaign_id.as_deref(),
            wizard_id.as_deref(),
        )
        .await
        .map_err(pipeline_err_to_string)
}

/// Delete a draft.
///
/// Only drafts with status 'draft' can be deleted.
///
/// # Arguments
/// * `draft_id` - The draft's unique identifier
#[tauri::command]
pub async fn delete_draft(
    draft_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    info!(draft_id = %draft_id, "Deleting draft");

    let manager = get_acceptance_manager(&state);
    manager
        .delete_draft(&draft_id)
        .await
        .map_err(pipeline_err_to_string)
}

// ============================================================================
// Draft Lifecycle Commands
// ============================================================================

/// Approve a draft.
///
/// Transitions the draft from 'draft' to 'approved'.
///
/// # Arguments
/// * `draft_id` - The draft's unique identifier
/// * `reason` - Optional approval reason
///
/// # Returns
/// The updated draft record
#[tauri::command]
pub async fn approve_draft(
    draft_id: String,
    reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<InMemoryDraft, String> {
    info!(draft_id = %draft_id, "Approving draft");

    let manager = get_acceptance_manager(&state);
    manager
        .approve_draft(&draft_id, reason.as_deref())
        .await
        .map_err(pipeline_err_to_string)
}

/// Reject a draft.
///
/// Transitions the draft from 'draft' to 'rejected'.
///
/// # Arguments
/// * `draft_id` - The draft's unique identifier
/// * `reason` - Reason for rejection
///
/// # Returns
/// The updated draft record
#[tauri::command]
pub async fn reject_draft(
    draft_id: String,
    reason: String,
    state: State<'_, AppState>,
) -> Result<InMemoryDraft, String> {
    info!(draft_id = %draft_id, "Rejecting draft");

    let manager = get_acceptance_manager(&state);
    manager
        .reject_draft(&draft_id, &reason)
        .await
        .map_err(pipeline_err_to_string)
}

/// Modify a draft.
///
/// Updates the draft data while keeping it in 'draft' status.
///
/// # Arguments
/// * `draft_id` - The draft's unique identifier
/// * `modifications` - JSON modifications to apply
/// * `reason` - Optional reason for modification
///
/// # Returns
/// The updated draft record
#[tauri::command]
pub async fn modify_draft(
    draft_id: String,
    modifications: serde_json::Value,
    reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<InMemoryDraft, String> {
    info!(draft_id = %draft_id, "Modifying draft");

    let manager = get_acceptance_manager(&state);
    manager
        .modify_draft(&draft_id, modifications, reason.as_deref())
        .await
        .map_err(pipeline_err_to_string)
}

/// Apply a draft to the campaign.
///
/// Creates the actual entity from an approved draft and marks it as canonical.
///
/// # Arguments
/// * `draft_id` - The draft's unique identifier
///
/// # Returns
/// Information about the applied entity
#[tauri::command]
pub async fn apply_draft_to_campaign(
    draft_id: String,
    state: State<'_, AppState>,
) -> Result<AppliedEntity, String> {
    info!(draft_id = %draft_id, "Applying draft to campaign");

    let manager = get_acceptance_manager(&state);
    manager
        .apply_to_campaign(&draft_id)
        .await
        .map_err(pipeline_err_to_string)
}

/// Revert an applied draft.
///
/// Removes the created entity and returns the draft to 'approved' status.
///
/// # Arguments
/// * `draft_id` - The draft's unique identifier
/// * `reason` - Reason for reversion
///
/// # Returns
/// The updated draft record
#[tauri::command]
pub async fn revert_applied_draft(
    draft_id: String,
    reason: String,
    state: State<'_, AppState>,
) -> Result<InMemoryDraft, String> {
    info!(draft_id = %draft_id, "Reverting applied draft");

    let manager = get_acceptance_manager(&state);
    manager
        .revert_to_approved(&draft_id, &reason)
        .await
        .map_err(pipeline_err_to_string)
}

/// Deprecate a canonical entity.
///
/// Marks a canonical entity as deprecated (soft delete).
///
/// # Arguments
/// * `draft_id` - The draft's unique identifier
/// * `reason` - Reason for deprecation
#[tauri::command]
pub async fn deprecate_entity(
    draft_id: String,
    reason: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    info!(draft_id = %draft_id, "Deprecating entity");

    let manager = get_acceptance_manager(&state);
    manager
        .deprecate(&draft_id, &reason)
        .await
        .map_err(pipeline_err_to_string)
}

// ============================================================================
// Batch Operations
// ============================================================================

/// Batch approve multiple drafts.
///
/// # Arguments
/// * `draft_ids` - List of draft IDs to approve
/// * `reason` - Shared approval reason
///
/// # Returns
/// Results for each draft (success or error message)
#[tauri::command]
pub async fn batch_approve_drafts(
    draft_ids: Vec<String>,
    reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<BatchResult>, String> {
    info!(count = draft_ids.len(), "Batch approving drafts");

    let manager = get_acceptance_manager(&state);
    let mut results = Vec::with_capacity(draft_ids.len());

    for draft_id in draft_ids {
        let result = match manager.approve_draft(&draft_id, reason.as_deref()).await {
            Ok(_) => BatchResult {
                draft_id,
                success: true,
                error: None,
            },
            Err(e) => BatchResult {
                draft_id,
                success: false,
                error: Some(e.to_string()),
            },
        };
        results.push(result);
    }

    Ok(results)
}

/// Batch reject multiple drafts.
///
/// # Arguments
/// * `draft_ids` - List of draft IDs to reject
/// * `reason` - Shared rejection reason
///
/// # Returns
/// Results for each draft (success or error message)
#[tauri::command]
pub async fn batch_reject_drafts(
    draft_ids: Vec<String>,
    reason: String,
    state: State<'_, AppState>,
) -> Result<Vec<BatchResult>, String> {
    info!(count = draft_ids.len(), "Batch rejecting drafts");

    let manager = get_acceptance_manager(&state);
    let mut results = Vec::with_capacity(draft_ids.len());

    for draft_id in draft_ids {
        let result = match manager.reject_draft(&draft_id, &reason).await {
            Ok(_) => BatchResult {
                draft_id,
                success: true,
                error: None,
            },
            Err(e) => BatchResult {
                draft_id,
                success: false,
                error: Some(e.to_string()),
            },
        };
        results.push(result);
    }

    Ok(results)
}

/// Batch apply multiple approved drafts.
///
/// # Arguments
/// * `draft_ids` - List of approved draft IDs to apply
///
/// # Returns
/// Results for each draft including created entity info
#[tauri::command]
pub async fn batch_apply_drafts(
    draft_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<BatchApplyResult>, String> {
    info!(count = draft_ids.len(), "Batch applying drafts");

    let manager = get_acceptance_manager(&state);
    let mut results = Vec::with_capacity(draft_ids.len());

    for draft_id in draft_ids {
        let result = match manager.apply_to_campaign(&draft_id).await {
            Ok(applied) => BatchApplyResult {
                draft_id,
                success: true,
                applied_entity: Some(applied),
                error: None,
            },
            Err(e) => BatchApplyResult {
                draft_id,
                success: false,
                applied_entity: None,
                error: Some(e.to_string()),
            },
        };
        results.push(result);
    }

    Ok(results)
}

// ============================================================================
// Trust Assignment Commands
// ============================================================================

/// Assign trust level to content.
///
/// Analyzes citations and assigns appropriate trust level.
///
/// # Arguments
/// * `content` - The content to analyze
/// * `citations` - Citations supporting the content
/// * `parsed_content` - Optional parsed JSON content
///
/// # Returns
/// Trust assignment with level, confidence, and reasoning
#[tauri::command]
pub fn assign_trust_level(
    content: String,
    citations: Vec<Citation>,
    parsed_content: Option<serde_json::Value>,
    state: State<'_, AppState>,
) -> TrustAssignment {
    debug!(
        citation_count = citations.len(),
        "Assigning trust level"
    );

    let assigner = get_trust_assigner(&state);
    assigner.assign(&content, &citations, parsed_content.as_ref())
}

/// Override trust level for a draft.
///
/// Manually sets the trust level, overriding automatic assignment.
///
/// # Arguments
/// * `draft_id` - The draft's unique identifier
/// * `trust_level` - The trust level to set
/// * `reason` - Reason for the override
///
/// # Errors
/// Returns error because database persistence is not yet implemented.
/// This prevents callers from assuming the override was persisted.
#[tauri::command]
pub async fn override_trust_level(
    draft_id: String,
    trust_level: TrustLevel,
    reason: String,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    warn!(
        draft_id = %draft_id,
        trust_level = ?trust_level,
        reason = %reason,
        "Trust level override requested but persistence not implemented"
    );

    // Return error to prevent callers from assuming the override was applied
    Err("Trust level override persistence not yet implemented".to_string())
}

// ============================================================================
// Audit Trail Commands
// ============================================================================

/// Get status history for a draft.
///
/// Note: Database persistence not yet implemented - returns empty list.
///
/// # Arguments
/// * `draft_id` - The draft's unique identifier
///
/// # Returns
/// List of status change logs
#[tauri::command]
pub async fn get_draft_status_history(
    draft_id: String,
    _state: State<'_, AppState>,
) -> Result<Vec<StatusHistoryEntry>, String> {
    debug!(draft_id = %draft_id, "Getting draft status history (not implemented)");

    // TODO: Implement when database methods are available
    Ok(Vec::new())
}

/// Get acceptance events for a draft.
///
/// Note: Database persistence not yet implemented - returns empty list.
///
/// # Arguments
/// * `draft_id` - The draft's unique identifier
///
/// # Returns
/// List of acceptance events (approvals, rejections, modifications)
#[tauri::command]
pub async fn get_draft_acceptance_events(
    draft_id: String,
    _state: State<'_, AppState>,
) -> Result<Vec<AcceptanceEventEntry>, String> {
    debug!(draft_id = %draft_id, "Getting draft acceptance events (not implemented)");

    // TODO: Implement when database methods are available
    Ok(Vec::new())
}

// ============================================================================
// Stats and Summary Commands
// ============================================================================

/// Get draft statistics for a campaign.
///
/// # Arguments
/// * `campaign_id` - The campaign's unique identifier
///
/// # Returns
/// Statistics about drafts in the campaign
#[tauri::command]
pub async fn get_campaign_draft_stats(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<DraftStats, String> {
    debug!(campaign_id = %campaign_id, "Getting campaign draft stats");

    let manager = get_acceptance_manager(&state);

    // Get all drafts for the campaign
    let all_drafts = manager
        .list_campaign_drafts(&campaign_id, None)
        .await
        .map_err(pipeline_err_to_string)?;

    // Count by status
    let mut draft_count = 0;
    let mut approved_count = 0;
    let mut canonical_count = 0;
    let mut deprecated_count = 0;

    // Count by entity type
    let mut by_entity_type: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    for draft in &all_drafts {
        match draft.status {
            CanonStatus::Draft => draft_count += 1,
            CanonStatus::Approved => approved_count += 1,
            CanonStatus::Canonical => canonical_count += 1,
            CanonStatus::Deprecated => deprecated_count += 1,
        }

        *by_entity_type.entry(draft.entity_type.clone()).or_insert(0) += 1;
    }

    Ok(DraftStats {
        total: all_drafts.len() as u32,
        draft: draft_count,
        approved: approved_count,
        canonical: canonical_count,
        deprecated: deprecated_count,
        by_entity_type,
    })
}

// ============================================================================
// Result Types
// ============================================================================

/// Result of a batch operation on a single draft
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchResult {
    pub draft_id: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Result of a batch apply operation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchApplyResult {
    pub draft_id: String,
    pub success: bool,
    pub applied_entity: Option<AppliedEntity>,
    pub error: Option<String>,
}

/// Statistics about drafts in a campaign
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DraftStats {
    pub total: u32,
    pub draft: u32,
    pub approved: u32,
    pub canonical: u32,
    pub deprecated: u32,
    pub by_entity_type: std::collections::HashMap<String, u32>,
}

/// A status history entry (placeholder until database is implemented)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StatusHistoryEntry {
    pub from_status: String,
    pub to_status: String,
    pub reason: Option<String>,
    pub triggered_by: Option<String>,
    pub timestamp: String,
}

/// An acceptance event entry (placeholder until database is implemented)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AcceptanceEventEntry {
    pub decision: String,
    pub modifications: Option<serde_json::Value>,
    pub reason: Option<String>,
    pub timestamp: String,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_result_serialization() {
        let result = BatchResult {
            draft_id: "draft-1".to_string(),
            success: true,
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("draft-1"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_draft_stats_serialization() {
        let mut by_type = std::collections::HashMap::new();
        by_type.insert("npc".to_string(), 5);
        by_type.insert("location".to_string(), 3);

        let stats = DraftStats {
            total: 10,
            draft: 4,
            approved: 2,
            canonical: 2,
            deprecated: 1,
            by_entity_type: by_type,
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total\":10"));
        assert!(json.contains("\"npc\":5"));
    }
}
