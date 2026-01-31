//! Wizard Commands Module
//!
//! Tauri IPC commands for the campaign creation wizard state machine.
//! Provides the frontend interface for wizard lifecycle, navigation, and completion.
//!
//! # Command Overview
//!
//! ## Lifecycle Commands
//! - [`start_campaign_wizard`]: Initialize a new wizard session
//! - [`get_wizard_state`]: Retrieve current wizard state by ID
//! - [`list_incomplete_wizards`]: Get all drafts for recovery UI
//! - [`delete_wizard`]: Permanently remove a wizard
//!
//! ## Navigation Commands
//! - [`advance_wizard_step`]: Move forward with step data
//! - [`wizard_go_back`]: Return to previous step (preserves data)
//! - [`wizard_skip_step`]: Skip optional steps
//! - [`update_wizard_draft`]: Direct draft updates (AI suggestions)
//!
//! ## Completion Commands
//! - [`complete_wizard`]: Finalize and create campaign
//! - [`cancel_wizard`]: Exit with optional draft preservation
//! - [`auto_save_wizard`]: Trigger auto-save checkpoint
//!
//! ## Conversation Commands
//! - [`link_wizard_conversation`]: Associate AI conversation thread
//!
//! # Frontend Integration
//!
//! These commands are invoked via Tauri's IPC mechanism from the Leptos frontend.
//! All commands use camelCase naming in JavaScript (e.g., `startCampaignWizard`).
//!
//! ```typescript
//! // Frontend example
//! const state = await invoke('start_campaign_wizard', { aiAssisted: true });
//! ```

use std::sync::Arc;
use tauri::State;
use tracing::{info, debug, error};

use crate::commands::AppState;
use crate::core::campaign::wizard::{
    WizardManager, WizardState, WizardSummary, WizardError,
    StepData, PartialCampaign,
};
use crate::database::CampaignRecord;

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a WizardManager from AppState
fn get_wizard_manager(state: &State<'_, AppState>) -> WizardManager {
    let pool = Arc::new(state.database.pool().clone());
    WizardManager::new(pool)
}

/// Convert WizardError to String for Tauri IPC
fn wizard_err_to_string(err: WizardError) -> String {
    error!(error = %err, "Wizard command error");
    err.to_string()
}

// ============================================================================
// Wizard Lifecycle Commands
// ============================================================================

/// Start a new campaign creation wizard.
///
/// Creates a new wizard state and returns it. The wizard starts at the Basics step.
///
/// # Arguments
/// * `ai_assisted` - Whether to enable AI assistance for suggestions
///
/// # Returns
/// The newly created wizard state
#[tauri::command]
pub async fn start_campaign_wizard(
    ai_assisted: bool,
    state: State<'_, AppState>,
) -> Result<WizardState, String> {
    info!(ai_assisted, "Starting campaign wizard");

    let manager = get_wizard_manager(&state);
    manager
        .start_wizard(ai_assisted)
        .await
        .map_err(wizard_err_to_string)
}

/// Get wizard state by ID.
///
/// Retrieves the current state of a wizard, including its draft campaign data.
///
/// # Arguments
/// * `wizard_id` - The wizard's unique identifier
///
/// # Returns
/// The wizard state if found
#[tauri::command]
pub async fn get_wizard_state(
    wizard_id: String,
    state: State<'_, AppState>,
) -> Result<Option<WizardState>, String> {
    debug!(wizard_id = %wizard_id, "Getting wizard state");

    let manager = get_wizard_manager(&state);
    manager
        .get_wizard(&wizard_id)
        .await
        .map_err(wizard_err_to_string)
}

/// List all incomplete wizards.
///
/// Returns summaries of all wizards that haven't been completed, useful for
/// showing "resume" options to the user.
///
/// # Returns
/// List of wizard summaries ordered by last update (most recent first)
#[tauri::command]
pub async fn list_incomplete_wizards(
    state: State<'_, AppState>,
) -> Result<Vec<WizardSummary>, String> {
    debug!("Listing incomplete wizards");

    let manager = get_wizard_manager(&state);
    manager
        .list_incomplete_wizards()
        .await
        .map_err(wizard_err_to_string)
}

/// Delete a wizard by ID.
///
/// Permanently removes the wizard state. This cannot be undone.
///
/// # Arguments
/// * `wizard_id` - The wizard's unique identifier
#[tauri::command]
pub async fn delete_wizard(
    wizard_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    info!(wizard_id = %wizard_id, "Deleting wizard");

    let manager = get_wizard_manager(&state);
    manager
        .delete_wizard(&wizard_id)
        .await
        .map_err(wizard_err_to_string)
}

// ============================================================================
// Wizard Navigation Commands
// ============================================================================

/// Advance to the next wizard step.
///
/// Validates the step data, applies it to the draft, and moves to the next step.
///
/// # Arguments
/// * `wizard_id` - The wizard's unique identifier
/// * `step_data` - Data collected at the current step (JSON)
///
/// # Returns
/// The updated wizard state
#[tauri::command]
pub async fn advance_wizard_step(
    wizard_id: String,
    step_data: StepData,
    state: State<'_, AppState>,
) -> Result<WizardState, String> {
    debug!(wizard_id = %wizard_id, step = ?step_data.step(), "Advancing wizard step");

    let manager = get_wizard_manager(&state);
    manager
        .advance_step(&wizard_id, step_data)
        .await
        .map_err(wizard_err_to_string)
}

/// Go back to the previous wizard step.
///
/// Navigates backward while preserving all collected data.
///
/// # Arguments
/// * `wizard_id` - The wizard's unique identifier
///
/// # Returns
/// The updated wizard state
#[tauri::command]
pub async fn wizard_go_back(
    wizard_id: String,
    state: State<'_, AppState>,
) -> Result<WizardState, String> {
    debug!(wizard_id = %wizard_id, "Going back in wizard");

    let manager = get_wizard_manager(&state);
    manager
        .go_back(&wizard_id)
        .await
        .map_err(wizard_err_to_string)
}

/// Skip the current wizard step.
///
/// Only works for skippable steps (Intent, PartyComposition, ArcStructure, InitialContent).
/// The step is not marked as completed when skipped.
///
/// # Arguments
/// * `wizard_id` - The wizard's unique identifier
///
/// # Returns
/// The updated wizard state
#[tauri::command]
pub async fn wizard_skip_step(
    wizard_id: String,
    state: State<'_, AppState>,
) -> Result<WizardState, String> {
    debug!(wizard_id = %wizard_id, "Skipping wizard step");

    let manager = get_wizard_manager(&state);
    manager
        .skip_step(&wizard_id)
        .await
        .map_err(wizard_err_to_string)
}

/// Update wizard draft without advancing step.
///
/// Used for partial saves, AI suggestion acceptance, or direct edits.
///
/// # Arguments
/// * `wizard_id` - The wizard's unique identifier
/// * `draft` - The updated partial campaign draft
///
/// # Returns
/// The updated wizard state
#[tauri::command]
pub async fn update_wizard_draft(
    wizard_id: String,
    draft: PartialCampaign,
    state: State<'_, AppState>,
) -> Result<WizardState, String> {
    debug!(wizard_id = %wizard_id, "Updating wizard draft");

    let manager = get_wizard_manager(&state);
    manager
        .update_draft(&wizard_id, draft)
        .await
        .map_err(wizard_err_to_string)
}

// ============================================================================
// Wizard Completion Commands
// ============================================================================

/// Complete the wizard and create a campaign.
///
/// Validates all required data, creates the campaign, and cleans up the wizard state.
/// The wizard must be at the Review step to complete.
///
/// # Arguments
/// * `wizard_id` - The wizard's unique identifier
///
/// # Returns
/// The created campaign record
#[tauri::command]
pub async fn complete_wizard(
    wizard_id: String,
    state: State<'_, AppState>,
) -> Result<CampaignRecord, String> {
    info!(wizard_id = %wizard_id, "Completing wizard");

    let manager = get_wizard_manager(&state);
    manager
        .complete_wizard(&wizard_id)
        .await
        .map_err(wizard_err_to_string)
}

/// Cancel the wizard with optional draft save.
///
/// If `save_draft` is true, the wizard state is preserved for later resumption.
/// Otherwise, the wizard state is permanently deleted.
///
/// # Arguments
/// * `wizard_id` - The wizard's unique identifier
/// * `save_draft` - If true, keeps the wizard state for later resumption
#[tauri::command]
pub async fn cancel_wizard(
    wizard_id: String,
    save_draft: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    info!(wizard_id = %wizard_id, save_draft, "Cancelling wizard");

    let manager = get_wizard_manager(&state);
    manager
        .cancel_wizard(&wizard_id, save_draft)
        .await
        .map_err(wizard_err_to_string)
}

/// Trigger an auto-save of the current wizard state.
///
/// Updates the auto_saved_at timestamp and persists any pending changes.
/// Called from frontend with debouncing (typically 30 second intervals).
///
/// # Arguments
/// * `wizard_id` - The wizard's unique identifier
/// * `partial_data` - Optional partial draft updates to apply
#[tauri::command]
pub async fn auto_save_wizard(
    wizard_id: String,
    partial_data: Option<PartialCampaign>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    debug!(wizard_id = %wizard_id, "Auto-saving wizard");

    let manager = get_wizard_manager(&state);
    manager
        .auto_save(&wizard_id, partial_data)
        .await
        .map_err(wizard_err_to_string)
}

// ============================================================================
// Conversation Thread Commands
// ============================================================================

/// Link a conversation thread to the wizard.
///
/// Associates a conversation thread for AI-assisted campaign creation.
///
/// # Arguments
/// * `wizard_id` - The wizard's unique identifier
/// * `thread_id` - The conversation thread ID to link
///
/// # Returns
/// The updated wizard state
#[tauri::command]
pub async fn link_wizard_conversation(
    wizard_id: String,
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<WizardState, String> {
    debug!(wizard_id = %wizard_id, thread_id = %thread_id, "Linking conversation to wizard");

    let manager = get_wizard_manager(&state);
    manager
        .link_conversation_thread(&wizard_id, thread_id)
        .await
        .map_err(wizard_err_to_string)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    // Integration tests would require a mock AppState with a test database.
    // Unit tests for command logic are in the wizard manager module.

    #[test]
    fn test_wizard_err_to_string() {
        use super::*;

        let err = WizardError::NotFound("test-id".to_string());
        let msg = wizard_err_to_string(err);
        assert!(msg.contains("test-id"));
    }
}
