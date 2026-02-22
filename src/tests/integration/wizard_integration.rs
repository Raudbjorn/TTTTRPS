//! Campaign Wizard Integration Tests
//!
//! Comprehensive integration tests for the campaign creation wizard flow.
//!
//! # Test Categories
//!
//! ## Manual Mode Flow
//! - Complete wizard flow without AI
//! - Step validation and transitions
//! - Draft persistence and recovery
//!
//! ## AI-Assisted Mode Flow
//! - Wizard with AI assistance enabled
//! - Conversation thread linking
//! - Suggestion acceptance flow
//!
//! ## Recovery Scenarios
//! - Crash recovery from incomplete state
//! - Draft cleanup for old wizards
//!
//! Note: These tests use in-memory databases and mock the wizard manager
//! for fast, isolated testing.

#![allow(unused_imports, dead_code)]

use std::sync::Arc;
use tempfile::TempDir;

use crate::core::campaign::wizard::{
    BasicsData, CampaignPacing, ExperienceLevel, IntentData, PartialCampaign, PlayersData,
    ScopeData, StepData, WizardManager,
};
use crate::database::{Database, WizardStep, WizardStateRecord};

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a test database in a temporary directory
async fn create_test_db() -> (Database, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db = Database::new(temp_dir.path())
        .await
        .expect("Failed to create test database");
    (db, temp_dir)
}

/// Create a wizard manager with test database
async fn setup_wizard_manager() -> (WizardManager, Database, TempDir) {
    let (db, temp_dir) = create_test_db().await;
    let pool = Arc::new(db.pool().clone());
    let manager = WizardManager::new(pool);
    (manager, db, temp_dir)
}

// ============================================================================
// Manual Mode Flow Tests
// ============================================================================

#[tokio::test]
async fn test_wizard_complete_manual_flow() {
    let (manager, _db, _temp) = setup_wizard_manager().await;

    // 1. Start wizard in manual mode (no AI)
    let state = manager.start_wizard(false).await.expect("Failed to start wizard");
    assert_eq!(state.current_step, WizardStep::Basics);
    assert!(!state.ai_assisted);
    assert!(state.completed_steps.is_empty());

    let wizard_id = state.id.clone();

    // 2. Complete Basics step
    let basics_data = StepData::Basics(BasicsData {
        name: "Test Campaign".to_string(),
        system: "dnd5e".to_string(),
        description: Some("A test campaign for integration testing".to_string()),
    });

    let state = manager
        .advance_step(&wizard_id, basics_data)
        .await
        .expect("Failed to advance from Basics");
    assert_eq!(state.current_step, WizardStep::Intent);
    assert!(state.completed_steps.contains(&WizardStep::Basics));
    assert_eq!(state.campaign_draft.name, Some("Test Campaign".to_string()));

    // 3. Skip Intent (optional step)
    let state = manager
        .skip_step(&wizard_id)
        .await
        .expect("Failed to skip Intent");
    assert_eq!(state.current_step, WizardStep::Scope);

    // 4. Complete Scope step
    let scope_data = StepData::Scope(ScopeData {
        session_count: Some(12),
        session_duration_hours: Some(4.0),
        pacing: Some(CampaignPacing::Balanced),
        duration_months: Some(6),
    });

    let state = manager
        .advance_step(&wizard_id, scope_data)
        .await
        .expect("Failed to advance from Scope");
    assert_eq!(state.current_step, WizardStep::Players);

    // 5. Complete Players step
    let players_data = StepData::Players(PlayersData {
        player_count: 4,
        experience_level: Some(ExperienceLevel::Intermediate),
    });

    let state = manager
        .advance_step(&wizard_id, players_data)
        .await
        .expect("Failed to advance from Players");
    assert_eq!(state.current_step, WizardStep::PartyComposition);
    assert_eq!(state.campaign_draft.player_count, Some(4));

    // 6. Skip remaining optional steps
    let state = manager
        .skip_step(&wizard_id)
        .await
        .expect("Failed to skip PartyComposition");
    assert_eq!(state.current_step, WizardStep::ArcStructure);

    let state = manager
        .skip_step(&wizard_id)
        .await
        .expect("Failed to skip ArcStructure");
    assert_eq!(state.current_step, WizardStep::InitialContent);

    let state = manager
        .skip_step(&wizard_id)
        .await
        .expect("Failed to skip InitialContent");
    assert_eq!(state.current_step, WizardStep::Review);

    // 7. Complete wizard
    let campaign = manager
        .complete_wizard(&wizard_id)
        .await
        .expect("Failed to complete wizard");

    assert_eq!(campaign.name, "Test Campaign");
    assert_eq!(campaign.system, "dnd5e");

    // 8. Verify wizard is cleaned up
    let state = manager.get_wizard(&wizard_id).await.expect("Failed to check wizard");
    assert!(state.is_none(), "Wizard should be deleted after completion");
}

#[tokio::test]
async fn test_wizard_go_back_preserves_data() {
    let (manager, _db, _temp) = setup_wizard_manager().await;

    // Start wizard
    let state = manager.start_wizard(false).await.expect("Failed to start wizard");
    let wizard_id = state.id.clone();

    // Complete Basics
    let basics_data = StepData::Basics(BasicsData {
        name: "Test Campaign".to_string(),
        system: "pf2e".to_string(),
        description: None,
    });
    manager
        .advance_step(&wizard_id, basics_data)
        .await
        .expect("Failed to advance");

    // Complete Intent
    let intent_data = StepData::Intent(IntentData {
        fantasy: "Dark political intrigue".to_string(),
        player_experiences: vec!["mystery".to_string(), "betrayal".to_string()],
        constraints: vec![],
        themes: vec!["power".to_string()],
        tone_keywords: vec!["grim".to_string()],
        avoid: vec!["comedy".to_string()],
    });
    let state = manager
        .advance_step(&wizard_id, intent_data)
        .await
        .expect("Failed to advance");
    assert_eq!(state.current_step, WizardStep::Scope);

    // Go back to Intent
    let state = manager.go_back(&wizard_id).await.expect("Failed to go back");
    assert_eq!(state.current_step, WizardStep::Intent);

    // Verify data is preserved
    assert!(state.campaign_draft.intent.is_some());
    let intent = state.campaign_draft.intent.unwrap();
    assert_eq!(intent.fantasy, "Dark political intrigue");

    // Go back to Basics
    let state = manager.go_back(&wizard_id).await.expect("Failed to go back");
    assert_eq!(state.current_step, WizardStep::Basics);

    // Verify Basics data is preserved
    assert_eq!(state.campaign_draft.name, Some("Test Campaign".to_string()));
    assert_eq!(state.campaign_draft.system, Some("pf2e".to_string()));
}

#[tokio::test]
async fn test_wizard_cannot_skip_required_steps() {
    let (manager, _db, _temp) = setup_wizard_manager().await;

    let state = manager.start_wizard(false).await.expect("Failed to start wizard");
    let wizard_id = state.id.clone();

    // Try to skip Basics (required step)
    let result = manager.skip_step(&wizard_id).await;
    assert!(result.is_err(), "Should not be able to skip Basics");

    // Complete Basics first
    let basics_data = StepData::Basics(BasicsData {
        name: "Test".to_string(),
        system: "dnd5e".to_string(),
        description: None,
    });
    manager
        .advance_step(&wizard_id, basics_data)
        .await
        .expect("Failed to advance");

    // Skip Intent (optional)
    manager.skip_step(&wizard_id).await.expect("Should skip Intent");

    // Try to skip Scope (required)
    let result = manager.skip_step(&wizard_id).await;
    assert!(result.is_err(), "Should not be able to skip Scope");
}

// ============================================================================
// Draft Recovery Tests
// ============================================================================

#[tokio::test]
async fn test_wizard_draft_recovery() {
    let (manager, _db, _temp) = setup_wizard_manager().await;

    // Start wizard and partially complete it
    let state = manager.start_wizard(true).await.expect("Failed to start wizard");
    let wizard_id = state.id.clone();

    // Complete a few steps
    let basics_data = StepData::Basics(BasicsData {
        name: "Abandoned Campaign".to_string(),
        system: "coc7e".to_string(),
        description: Some("Left incomplete".to_string()),
    });
    manager
        .advance_step(&wizard_id, basics_data)
        .await
        .expect("Failed to advance");

    // Now simulate "crash" - just check that we can list incomplete wizards
    let incomplete = manager
        .list_incomplete_wizards()
        .await
        .expect("Failed to list incomplete wizards");

    assert_eq!(incomplete.len(), 1);
    assert_eq!(incomplete[0].id, wizard_id);
    assert_eq!(
        incomplete[0].campaign_name,
        Some("Abandoned Campaign".to_string())
    );
    assert!(incomplete[0].ai_assisted);

    // Resume the wizard
    let resumed = manager
        .get_wizard(&wizard_id)
        .await
        .expect("Failed to get wizard")
        .expect("Wizard should exist");

    assert_eq!(resumed.current_step, WizardStep::Intent);
    assert_eq!(
        resumed.campaign_draft.name,
        Some("Abandoned Campaign".to_string())
    );
}

#[tokio::test]
async fn test_wizard_cancel_with_save_draft() {
    let (manager, _db, _temp) = setup_wizard_manager().await;

    let state = manager.start_wizard(false).await.expect("Failed to start wizard");
    let wizard_id = state.id.clone();

    // Do some work
    let basics_data = StepData::Basics(BasicsData {
        name: "To Be Saved".to_string(),
        system: "vtm5e".to_string(),
        description: None,
    });
    manager
        .advance_step(&wizard_id, basics_data)
        .await
        .expect("Failed to advance");

    // Cancel but save draft
    manager
        .cancel_wizard(&wizard_id, true)
        .await
        .expect("Failed to cancel");

    // Wizard should still exist
    let state = manager.get_wizard(&wizard_id).await.expect("Failed to get wizard");
    assert!(state.is_some(), "Draft should be preserved");
}

#[tokio::test]
async fn test_wizard_cancel_without_save() {
    let (manager, _db, _temp) = setup_wizard_manager().await;

    let state = manager.start_wizard(false).await.expect("Failed to start wizard");
    let wizard_id = state.id.clone();

    // Do some work
    let basics_data = StepData::Basics(BasicsData {
        name: "To Be Discarded".to_string(),
        system: "bitd".to_string(),
        description: None,
    });
    manager
        .advance_step(&wizard_id, basics_data)
        .await
        .expect("Failed to advance");

    // Cancel without saving draft
    manager
        .cancel_wizard(&wizard_id, false)
        .await
        .expect("Failed to cancel");

    // Wizard should be deleted
    let state = manager.get_wizard(&wizard_id).await.expect("Failed to get wizard");
    assert!(state.is_none(), "Draft should be deleted");
}

// ============================================================================
// Auto-save Tests
// ============================================================================

#[tokio::test]
async fn test_wizard_auto_save() {
    let (manager, _db, _temp) = setup_wizard_manager().await;

    let state = manager.start_wizard(false).await.expect("Failed to start wizard");
    let wizard_id = state.id.clone();

    // Complete basics
    let basics_data = StepData::Basics(BasicsData {
        name: "Auto Save Test".to_string(),
        system: "dnd5e".to_string(),
        description: None,
    });
    manager
        .advance_step(&wizard_id, basics_data)
        .await
        .expect("Failed to advance");

    // Trigger auto-save with updated draft
    let mut updated_draft = manager
        .get_wizard(&wizard_id)
        .await
        .expect("Failed to get wizard")
        .expect("Wizard should exist")
        .campaign_draft;

    updated_draft.description = Some("Auto-saved description".to_string());

    manager
        .auto_save(&wizard_id, Some(updated_draft))
        .await
        .expect("Auto-save failed");

    // Verify auto-save timestamp is set
    let state = manager
        .get_wizard(&wizard_id)
        .await
        .expect("Failed to get wizard")
        .expect("Wizard should exist");

    assert!(state.auto_saved_at.is_some());
    assert_eq!(
        state.campaign_draft.description,
        Some("Auto-saved description".to_string())
    );
}

// ============================================================================
// AI-Assisted Mode Tests
// ============================================================================

#[tokio::test]
async fn test_wizard_ai_assisted_mode() {
    let (manager, _db, _temp) = setup_wizard_manager().await;

    // Start wizard with AI assistance
    let state = manager.start_wizard(true).await.expect("Failed to start wizard");
    assert!(state.ai_assisted);
    assert!(state.conversation_thread_id.is_none());

    let wizard_id = state.id.clone();

    // Link a conversation thread
    let state = manager
        .link_conversation_thread(&wizard_id, "thread-123".to_string())
        .await
        .expect("Failed to link conversation");

    assert_eq!(
        state.conversation_thread_id,
        Some("thread-123".to_string())
    );
}

#[tokio::test]
async fn test_wizard_update_draft_directly() {
    let (manager, _db, _temp) = setup_wizard_manager().await;

    let state = manager.start_wizard(true).await.expect("Failed to start wizard");
    let wizard_id = state.id.clone();

    // Update draft directly (simulating AI suggestion acceptance)
    let mut draft = PartialCampaign::default();
    draft.name = Some("AI Suggested Name".to_string());
    draft.system = Some("dnd5e".to_string());
    draft.description = Some("AI-generated description".to_string());

    let state = manager
        .update_draft(&wizard_id, draft)
        .await
        .expect("Failed to update draft");

    assert_eq!(
        state.campaign_draft.name,
        Some("AI Suggested Name".to_string())
    );
    assert_eq!(
        state.campaign_draft.description,
        Some("AI-generated description".to_string())
    );

    // Current step should be unchanged (still at Basics)
    assert_eq!(state.current_step, WizardStep::Basics);
}

// ============================================================================
// Multiple Wizards Tests
// ============================================================================

#[tokio::test]
async fn test_multiple_incomplete_wizards() {
    let (manager, _db, _temp) = setup_wizard_manager().await;

    // Create multiple wizards
    let state1 = manager.start_wizard(false).await.expect("Failed to start wizard 1");
    let state2 = manager.start_wizard(true).await.expect("Failed to start wizard 2");
    let _state3 = manager.start_wizard(false).await.expect("Failed to start wizard 3");

    // Advance each to different stages
    let basics1 = StepData::Basics(BasicsData {
        name: "Campaign 1".to_string(),
        system: "dnd5e".to_string(),
        description: None,
    });
    manager
        .advance_step(&state1.id, basics1)
        .await
        .expect("Failed to advance");

    let basics2 = StepData::Basics(BasicsData {
        name: "Campaign 2".to_string(),
        system: "pf2e".to_string(),
        description: None,
    });
    manager
        .advance_step(&state2.id, basics2)
        .await
        .expect("Failed to advance");
    manager.skip_step(&state2.id).await.expect("Failed to skip");

    // state3 left at Basics

    // List incomplete
    let incomplete = manager
        .list_incomplete_wizards()
        .await
        .expect("Failed to list");

    assert_eq!(incomplete.len(), 3);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn test_wizard_not_found() {
    let (manager, _db, _temp) = setup_wizard_manager().await;

    let result = manager
        .get_wizard("nonexistent-id")
        .await
        .expect("Get should not fail");
    assert!(result.is_none());

    let result = manager
        .advance_step(
            "nonexistent-id",
            StepData::Basics(BasicsData {
                name: "Test".to_string(),
                system: "dnd5e".to_string(),
                description: None,
            }),
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_wizard_delete_nonexistent() {
    let (manager, _db, _temp) = setup_wizard_manager().await;

    let result = manager.delete_wizard("nonexistent-id").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_wizard_complete_from_wrong_step() {
    let (manager, _db, _temp) = setup_wizard_manager().await;

    let state = manager.start_wizard(false).await.expect("Failed to start wizard");
    let wizard_id = state.id.clone();

    // Try to complete immediately (at Basics step)
    let result = manager.complete_wizard(&wizard_id).await;
    assert!(result.is_err(), "Should not complete from Basics step");
}
