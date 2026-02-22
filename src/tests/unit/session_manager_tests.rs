//! Session Manager Unit Tests
//!
//! Comprehensive tests for SessionManager covering:
//! - Session creation with valid/invalid campaign
//! - Combat start with empty/multiple combatants
//! - Initiative ordering (descending, tie-breaking)
//! - Turn advancement (next combatant, wrap around)
//! - HP modification (damage, healing, temp HP)
//! - HP bounds (no negative, no exceeding max)
//! - Condition application (single, multiple)
//! - Condition duration countdown
//! - Condition removal (manual, expiry)
//! - Condition stacking rules
//! - Combatant death and removal
//! - Combat end scenarios
//! - Session notes (creation, categorization, search)
//! - Timeline events (ordering, filtering)
//! - Session snapshot creation/restoration


use crate::core::session_manager::{
    CombatEventType, CombatState, CombatStatus, Combatant, CombatantType,
    GameSession, LogEntryType, SessionError, SessionManager,
    SessionStatus,
};

use crate::core::session::conditions::{
    AdvancedCondition, ConditionDuration as AdvancedConditionDuration,
    ConditionTemplates, ConditionTracker, SaveTiming,
};
use crate::core::session::timeline::{
    EventSeverity, TimelineEvent, TimelineEventType,
};
use crate::core::session::notes::{
    EntityType as NoteEntityType, NoteCategory, SessionNote,
};

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a test session manager (in-memory, no database required)
fn create_test_manager() -> SessionManager {
    SessionManager::new()
}

/// Create a basic test combatant
fn create_test_combatant(name: &str, initiative: i32, hp: Option<i32>) -> Combatant {
    let mut combatant = Combatant::new(name, initiative, CombatantType::Player);
    combatant.initiative_modifier = initiative % 5; // Simple modifier based on initiative
    combatant.current_hp = hp;
    combatant.max_hp = hp;
    combatant.armor_class = Some(15);
    combatant
}

/// Create a combatant with full HP configuration
fn create_combatant_with_hp(name: &str, initiative: i32, current_hp: i32, max_hp: i32, temp_hp: Option<i32>) -> Combatant {
    let mut combatant = Combatant::new(name, initiative, CombatantType::Player);
    combatant.current_hp = Some(current_hp);
    combatant.max_hp = Some(max_hp);
    combatant.temp_hp = temp_hp;
    combatant.armor_class = Some(15);
    combatant
}

/// Create a monster combatant
fn create_monster(name: &str, initiative: i32, hp: i32) -> Combatant {
    let mut combatant = Combatant::new(name, initiative, CombatantType::Monster);
    combatant.initiative_modifier = 2;
    combatant.current_hp = Some(hp);
    combatant.max_hp = Some(hp);
    combatant.armor_class = Some(13);
    combatant
}

// ============================================================================
// Session Creation Tests
// ============================================================================

#[cfg(test)]
mod session_creation_tests {
    use super::*;

    #[test]
    fn test_start_session_creates_valid_session() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        assert_eq!(session.campaign_id, "campaign-001");
        assert_eq!(session.session_number, 1);
        assert_eq!(session.status, SessionStatus::Active);
        assert!(session.combat.is_none());
        assert!(session.ended_at.is_none());
        assert!(!session.id.is_empty());
    }

    #[test]
    fn test_start_session_assigns_unique_ids() {
        let manager = create_test_manager();
        let session1 = manager.start_session("campaign-001", 1);
        let session2 = manager.start_session("campaign-001", 2);

        assert_ne!(session1.id, session2.id);
    }

    #[test]
    fn test_start_session_with_different_campaigns() {
        let manager = create_test_manager();
        let session1 = manager.start_session("campaign-001", 1);
        let session2 = manager.start_session("campaign-002", 1);

        assert_eq!(session1.campaign_id, "campaign-001");
        assert_eq!(session2.campaign_id, "campaign-002");
    }

    #[test]
    fn test_get_session_returns_correct_session() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let retrieved = manager.get_session(&session.id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, session.id);
    }

    #[test]
    fn test_get_session_returns_none_for_invalid_id() {
        let manager = create_test_manager();
        let retrieved = manager.get_session("nonexistent-id");
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_get_active_session_returns_active_session() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let active = manager.get_active_session("campaign-001");
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, session.id);
    }

    #[test]
    fn test_get_active_session_returns_none_for_nonexistent_campaign() {
        let manager = create_test_manager();
        let active = manager.get_active_session("nonexistent-campaign");
        assert!(active.is_none());
    }

    #[test]
    fn test_create_planned_session() {
        let manager = create_test_manager();
        let session = manager.create_planned_session("campaign-001", Some("Session 1: The Beginning".to_string()));

        assert_eq!(session.status, SessionStatus::Planned);
        assert_eq!(session.title, Some("Session 1: The Beginning".to_string()));
    }

    #[test]
    fn test_start_planned_session() {
        let manager = create_test_manager();
        let planned = manager.create_planned_session("campaign-001", None);

        let started = manager.start_planned_session(&planned.id);
        assert!(started.is_ok());

        let session = started.unwrap();
        assert_eq!(session.status, SessionStatus::Active);
    }

    #[test]
    fn test_start_planned_session_with_invalid_id() {
        let manager = create_test_manager();
        let result = manager.start_planned_session("nonexistent");

        assert!(result.is_err());
        match result.unwrap_err() {
            SessionError::SessionNotFound(_) => {},
            _ => panic!("Expected SessionNotFound error"),
        }
    }

    #[test]
    fn test_session_number_auto_increment() {
        let manager = create_test_manager();

        // Create first session
        manager.start_session("campaign-001", 1);

        // Create planned session - should auto-increment
        let planned = manager.create_planned_session("campaign-001", None);
        assert_eq!(planned.session_number, 2);
    }

    #[test]
    fn test_list_sessions() {
        let manager = create_test_manager();

        manager.start_session("campaign-001", 1);
        manager.start_session("campaign-001", 2);
        manager.start_session("campaign-002", 1);

        let sessions = manager.list_sessions("campaign-001");
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_list_sessions_empty_campaign() {
        let manager = create_test_manager();
        let sessions = manager.list_sessions("nonexistent");
        assert!(sessions.is_empty());
    }
}

// ============================================================================
// Session Lifecycle Tests
// ============================================================================

#[cfg(test)]
mod session_lifecycle_tests {
    use super::*;

    #[test]
    fn test_pause_session() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let result = manager.pause_session(&session.id);
        assert!(result.is_ok());

        let paused = manager.get_session(&session.id).unwrap();
        assert_eq!(paused.status, SessionStatus::Paused);
    }

    #[test]
    fn test_pause_nonexistent_session() {
        let manager = create_test_manager();
        let result = manager.pause_session("nonexistent");

        assert!(result.is_err());
    }

    #[test]
    fn test_resume_session() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        manager.pause_session(&session.id).unwrap();
        let result = manager.resume_session(&session.id);

        assert!(result.is_ok());
        let resumed = manager.get_session(&session.id).unwrap();
        assert_eq!(resumed.status, SessionStatus::Active);
    }

    #[test]
    fn test_end_session() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let result = manager.end_session(&session.id);
        assert!(result.is_ok());

        let summary = result.unwrap();
        assert_eq!(summary.status, SessionStatus::Ended);
        assert!(summary.ended_at.is_some());
    }

    #[test]
    fn test_end_session_ends_active_combat() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        manager.start_combat(&session.id).unwrap();
        manager.end_session(&session.id).unwrap();

        let ended = manager.get_session(&session.id).unwrap();
        assert!(ended.combat.is_some(), "Expected combat to exist after ending session");
        let combat = ended.combat.expect("Combat should be present");
        assert_eq!(combat.status, CombatStatus::Ended, "Combat should have Ended status");
    }

    #[test]
    fn test_end_session_duration_calculated() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let summary = manager.end_session(&session.id).unwrap();
        assert!(summary.duration_minutes.is_some());
    }

    #[test]
    fn test_session_log_entry() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let entry = manager.add_log_entry(
            &session.id,
            LogEntryType::Narrative,
            "The party entered the dungeon".to_string(),
            None,
        );

        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert!(entry.content.contains("dungeon"));
    }

    #[test]
    fn test_set_active_scene() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let result = manager.set_active_scene(&session.id, Some("Tavern".to_string()));
        assert!(result.is_ok());

        let updated = manager.get_session(&session.id).unwrap();
        assert_eq!(updated.active_scene, Some("Tavern".to_string()));
    }

    #[test]
    fn test_reorder_session() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let result = manager.reorder_session(&session.id, 5);
        assert!(result.is_ok());

        let updated = manager.get_session(&session.id).unwrap();
        assert_eq!(updated.order_index, 5);
    }
}

// ============================================================================
// Combat Start Tests
// ============================================================================

#[cfg(test)]
mod combat_start_tests {
    use super::*;

    #[test]
    fn test_start_combat_creates_combat_state() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let result = manager.start_combat(&session.id);
        assert!(result.is_ok());

        let combat = result.unwrap();
        assert_eq!(combat.round, 1);
        assert_eq!(combat.current_turn, 0);
        assert_eq!(combat.status, CombatStatus::Active);
        assert!(combat.combatants.is_empty());
    }

    #[test]
    fn test_start_combat_with_invalid_session() {
        let manager = create_test_manager();
        let result = manager.start_combat("nonexistent");

        assert!(result.is_err());
        match result.unwrap_err() {
            SessionError::SessionNotFound(_) => {},
            _ => panic!("Expected SessionNotFound error"),
        }
    }

    #[test]
    fn test_start_combat_when_combat_already_active() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        manager.start_combat(&session.id).unwrap();
        let result = manager.start_combat(&session.id);

        assert!(result.is_err());
        match result.unwrap_err() {
            SessionError::CombatAlreadyActive => {},
            _ => panic!("Expected CombatAlreadyActive error"),
        }
    }

    #[test]
    fn test_end_combat() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        manager.start_combat(&session.id).unwrap();
        let result = manager.end_combat(&session.id);

        assert!(result.is_ok());

        let combat = manager.get_combat(&session.id).unwrap();
        assert_eq!(combat.status, CombatStatus::Ended);
    }

    #[test]
    fn test_end_combat_no_active_combat() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let result = manager.end_combat(&session.id);

        assert!(result.is_err());
        match result.unwrap_err() {
            SessionError::NoCombatActive => {},
            _ => panic!("Expected NoCombatActive error"),
        }
    }

    #[test]
    fn test_get_combat() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        // No combat yet
        assert!(manager.get_combat(&session.id).is_none());

        manager.start_combat(&session.id).unwrap();

        // Combat exists
        assert!(manager.get_combat(&session.id).is_some());
    }
}

// ============================================================================
// Combatant Management Tests
// ============================================================================

#[cfg(test)]
mod combatant_management_tests {
    use super::*;

    #[test]
    fn test_add_combatant() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = create_test_combatant("Fighter", 18, Some(50));
        let result = manager.add_combatant(&session.id, combatant);

        assert!(result.is_ok());

        let combat = manager.get_combat(&session.id).unwrap();
        assert_eq!(combat.combatants.len(), 1);
        assert_eq!(combat.combatants[0].name, "Fighter");
    }

    #[test]
    fn test_add_combatant_quick() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let result = manager.add_combatant_quick(
            &session.id,
            "Goblin",
            15,
            CombatantType::Monster,
        );

        assert!(result.is_ok());
        let combatant = result.unwrap();
        assert_eq!(combatant.name, "Goblin");
        assert_eq!(combatant.initiative, 15);
    }

    #[test]
    fn test_add_multiple_combatants() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        manager.add_combatant_quick(&session.id, "Fighter", 18, CombatantType::Player).unwrap();
        manager.add_combatant_quick(&session.id, "Wizard", 12, CombatantType::Player).unwrap();
        manager.add_combatant_quick(&session.id, "Goblin", 15, CombatantType::Monster).unwrap();

        let combat = manager.get_combat(&session.id).unwrap();
        assert_eq!(combat.combatants.len(), 3);
    }

    #[test]
    fn test_add_combatant_no_combat() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let combatant = create_test_combatant("Fighter", 18, Some(50));
        let result = manager.add_combatant(&session.id, combatant);

        assert!(result.is_err());
        match result.unwrap_err() {
            SessionError::NoCombatActive => {},
            _ => panic!("Expected NoCombatActive error"),
        }
    }

    #[test]
    fn test_remove_combatant() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = manager.add_combatant_quick(&session.id, "Fighter", 18, CombatantType::Player).unwrap();

        let result = manager.remove_combatant(&session.id, &combatant.id);
        assert!(result.is_ok());

        let combat = manager.get_combat(&session.id).unwrap();
        assert!(combat.combatants.is_empty());
    }

    #[test]
    fn test_remove_nonexistent_combatant() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let result = manager.remove_combatant(&session.id, "nonexistent");

        assert!(result.is_err());
        match result.unwrap_err() {
            SessionError::CombatantNotFound(_) => {},
            _ => panic!("Expected CombatantNotFound error"),
        }
    }

    #[test]
    fn test_update_combatant() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let mut combatant = create_test_combatant("Fighter", 18, Some(50));
        let combatant_id = combatant.id.clone();
        manager.add_combatant(&session.id, combatant.clone()).unwrap();

        combatant.current_hp = Some(30);
        let result = manager.update_combatant(&session.id, combatant);

        assert!(result.is_ok());

        let combat = manager.get_combat(&session.id).unwrap();
        let updated = combat.combatants.iter().find(|c| c.id == combatant_id).unwrap();
        assert_eq!(updated.current_hp, Some(30));
    }
}

// ============================================================================
// Initiative Ordering Tests
// ============================================================================

#[cfg(test)]
mod initiative_ordering_tests {
    use super::*;

    #[test]
    fn test_initiative_order_descending() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        manager.add_combatant_quick(&session.id, "Fighter", 18, CombatantType::Player).unwrap();
        manager.add_combatant_quick(&session.id, "Wizard", 12, CombatantType::Player).unwrap();
        manager.add_combatant_quick(&session.id, "Rogue", 20, CombatantType::Player).unwrap();

        let combat = manager.get_combat(&session.id).unwrap();

        // Should be sorted highest to lowest
        assert_eq!(combat.combatants[0].name, "Rogue");      // 20
        assert_eq!(combat.combatants[1].name, "Fighter");    // 18
        assert_eq!(combat.combatants[2].name, "Wizard");     // 12
    }

    #[test]
    fn test_initiative_tie_breaking_by_modifier() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        // Create combatants with same initiative but different modifiers
        let mut fighter = create_test_combatant("Fighter", 15, Some(50));
        fighter.initiative_modifier = 2;

        let mut rogue = create_test_combatant("Rogue", 15, Some(35));
        rogue.initiative_modifier = 5;

        manager.add_combatant(&session.id, fighter).unwrap();
        manager.add_combatant(&session.id, rogue).unwrap();

        let combat = manager.get_combat(&session.id).unwrap();

        // Rogue has higher modifier, should go first
        assert_eq!(combat.combatants[0].name, "Rogue");
        assert_eq!(combat.combatants[1].name, "Fighter");
    }

    #[test]
    fn test_set_initiative() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let fighter = manager.add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player).unwrap();
        manager.add_combatant_quick(&session.id, "Wizard", 18, CombatantType::Player).unwrap();

        // Fighter starts second
        let combat = manager.get_combat(&session.id).unwrap();
        assert_eq!(combat.combatants[0].name, "Wizard");

        // Update Fighter's initiative to be higher
        manager.set_initiative(&session.id, &fighter.id, 20).unwrap();

        // Now Fighter should be first
        let combat = manager.get_combat(&session.id).unwrap();
        assert_eq!(combat.combatants[0].name, "Fighter");
    }

    #[test]
    fn test_initiative_resorts_on_add() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        manager.add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player).unwrap();
        manager.add_combatant_quick(&session.id, "Wizard", 18, CombatantType::Player).unwrap();

        // Add someone in between
        manager.add_combatant_quick(&session.id, "Rogue", 17, CombatantType::Player).unwrap();

        let combat = manager.get_combat(&session.id).unwrap();
        assert_eq!(combat.combatants[0].name, "Wizard");     // 18
        assert_eq!(combat.combatants[1].name, "Rogue");      // 17
        assert_eq!(combat.combatants[2].name, "Fighter");    // 15
    }
}

// ============================================================================
// Turn Advancement Tests
// ============================================================================

#[cfg(test)]
mod turn_advancement_tests {
    use super::*;

    #[test]
    fn test_next_turn_advances_correctly() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        manager.add_combatant_quick(&session.id, "Fighter", 20, CombatantType::Player).unwrap();
        manager.add_combatant_quick(&session.id, "Wizard", 15, CombatantType::Player).unwrap();
        manager.add_combatant_quick(&session.id, "Rogue", 10, CombatantType::Player).unwrap();

        // First combatant (Fighter)
        let current = manager.get_current_combatant(&session.id).unwrap();
        assert_eq!(current.name, "Fighter");

        // Advance to Wizard
        manager.next_turn(&session.id).unwrap();
        let current = manager.get_current_combatant(&session.id).unwrap();
        assert_eq!(current.name, "Wizard");

        // Advance to Rogue
        manager.next_turn(&session.id).unwrap();
        let current = manager.get_current_combatant(&session.id).unwrap();
        assert_eq!(current.name, "Rogue");
    }

    #[test]
    fn test_turn_wrap_around_increments_round() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        manager.add_combatant_quick(&session.id, "Fighter", 20, CombatantType::Player).unwrap();
        manager.add_combatant_quick(&session.id, "Wizard", 15, CombatantType::Player).unwrap();

        let combat = manager.get_combat(&session.id).unwrap();
        assert_eq!(combat.round, 1);

        // Advance through all combatants
        manager.next_turn(&session.id).unwrap(); // Fighter -> Wizard
        manager.next_turn(&session.id).unwrap(); // Wizard -> Fighter (round 2)

        let combat = manager.get_combat(&session.id).unwrap();
        assert_eq!(combat.round, 2);
        assert_eq!(combat.combatants[combat.current_turn].name, "Fighter");
    }

    #[test]
    fn test_next_turn_skips_inactive_combatants() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        manager.add_combatant_quick(&session.id, "Fighter", 20, CombatantType::Player).unwrap();

        let mut wizard = create_test_combatant("Wizard", 15, Some(30));
        wizard.is_active = false; // Wizard is inactive
        manager.add_combatant(&session.id, wizard).unwrap();

        manager.add_combatant_quick(&session.id, "Rogue", 10, CombatantType::Player).unwrap();

        // Advance from Fighter
        manager.next_turn(&session.id).unwrap();

        // Should skip Wizard and go to Rogue
        let current = manager.get_current_combatant(&session.id).unwrap();
        assert_eq!(current.name, "Rogue");
    }

    #[test]
    fn test_next_turn_empty_combatants() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let result = manager.next_turn(&session.id);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_previous_turn() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        manager.add_combatant_quick(&session.id, "Fighter", 20, CombatantType::Player).unwrap();
        manager.add_combatant_quick(&session.id, "Wizard", 15, CombatantType::Player).unwrap();

        // Advance to Wizard
        manager.next_turn(&session.id).unwrap();
        let current = manager.get_current_combatant(&session.id).unwrap();
        assert_eq!(current.name, "Wizard");

        // Go back to Fighter
        manager.previous_turn(&session.id).unwrap();
        let current = manager.get_current_combatant(&session.id).unwrap();
        assert_eq!(current.name, "Fighter");
    }

    #[test]
    fn test_previous_turn_decrements_round() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        manager.add_combatant_quick(&session.id, "Fighter", 20, CombatantType::Player).unwrap();
        manager.add_combatant_quick(&session.id, "Wizard", 15, CombatantType::Player).unwrap();

        // Advance to round 2
        manager.next_turn(&session.id).unwrap();
        manager.next_turn(&session.id).unwrap();

        let combat = manager.get_combat(&session.id).unwrap();
        assert_eq!(combat.round, 2);

        // Go back
        manager.previous_turn(&session.id).unwrap();

        let combat = manager.get_combat(&session.id).unwrap();
        assert_eq!(combat.round, 1);
    }

    #[test]
    fn test_remove_combatant_adjusts_current_turn() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let fighter = manager.add_combatant_quick(&session.id, "Fighter", 20, CombatantType::Player).unwrap();
        manager.add_combatant_quick(&session.id, "Wizard", 15, CombatantType::Player).unwrap();
        manager.add_combatant_quick(&session.id, "Rogue", 10, CombatantType::Player).unwrap();

        // Advance to Wizard (index 1)
        manager.next_turn(&session.id).unwrap();

        // Remove Fighter (index 0)
        manager.remove_combatant(&session.id, &fighter.id).unwrap();

        // Current turn should adjust
        let _combat = manager.get_combat(&session.id).unwrap();
        let current = manager.get_current_combatant(&session.id).unwrap();

        // Should still be on Wizard
        assert_eq!(current.name, "Wizard");
    }
}

// ============================================================================
// HP Modification Tests
// ============================================================================

#[cfg(test)]
mod hp_modification_tests {
    use super::*;

    #[test]
    fn test_damage_combatant() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = create_combatant_with_hp("Fighter", 15, 50, 50, None);
        let combatant_id = combatant.id.clone();
        manager.add_combatant(&session.id, combatant).unwrap();

        let remaining = manager.damage_combatant(&session.id, &combatant_id, 15).unwrap();

        assert_eq!(remaining, 35);
    }

    #[test]
    fn test_damage_absorbs_temp_hp_first() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = create_combatant_with_hp("Fighter", 15, 50, 50, Some(10));
        let combatant_id = combatant.id.clone();
        manager.add_combatant(&session.id, combatant).unwrap();

        // 15 damage: 10 absorbed by temp HP, 5 to current HP
        let remaining = manager.damage_combatant(&session.id, &combatant_id, 15).unwrap();

        assert_eq!(remaining, 45);
    }

    #[test]
    fn test_damage_cannot_go_negative() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = create_combatant_with_hp("Fighter", 15, 10, 50, None);
        let combatant_id = combatant.id.clone();
        manager.add_combatant(&session.id, combatant).unwrap();

        let remaining = manager.damage_combatant(&session.id, &combatant_id, 100).unwrap();

        assert_eq!(remaining, 0);
    }

    #[test]
    fn test_heal_combatant() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = create_combatant_with_hp("Fighter", 15, 30, 50, None);
        let combatant_id = combatant.id.clone();
        manager.add_combatant(&session.id, combatant).unwrap();

        let new_hp = manager.heal_combatant(&session.id, &combatant_id, 15).unwrap();

        assert_eq!(new_hp, 45);
    }

    #[test]
    fn test_heal_cannot_exceed_max() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = create_combatant_with_hp("Fighter", 15, 45, 50, None);
        let combatant_id = combatant.id.clone();
        manager.add_combatant(&session.id, combatant).unwrap();

        let new_hp = manager.heal_combatant(&session.id, &combatant_id, 100).unwrap();

        assert_eq!(new_hp, 50);
    }

    #[test]
    fn test_add_temp_hp() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = create_combatant_with_hp("Fighter", 15, 50, 50, None);
        let combatant_id = combatant.id.clone();
        manager.add_combatant(&session.id, combatant).unwrap();

        manager.add_temp_hp(&session.id, &combatant_id, 15).unwrap();

        let combat = manager.get_combat(&session.id).unwrap();
        let fighter = combat.combatants.iter().find(|c| c.id == combatant_id).unwrap();
        assert_eq!(fighter.temp_hp, Some(15));
    }

    #[test]
    fn test_temp_hp_uses_higher_value() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = create_combatant_with_hp("Fighter", 15, 50, 50, Some(10));
        let combatant_id = combatant.id.clone();
        manager.add_combatant(&session.id, combatant).unwrap();

        // Apply 15 temp HP when already have 10
        manager.add_temp_hp(&session.id, &combatant_id, 15).unwrap();

        let combat = manager.get_combat(&session.id).unwrap();
        let fighter = combat.combatants.iter().find(|c| c.id == combatant_id).unwrap();
        assert_eq!(fighter.temp_hp, Some(15));
    }

    #[test]
    fn test_temp_hp_lower_value_ignored() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = create_combatant_with_hp("Fighter", 15, 50, 50, Some(20));
        let combatant_id = combatant.id.clone();
        manager.add_combatant(&session.id, combatant).unwrap();

        // Try to apply 10 temp HP when already have 20
        manager.add_temp_hp(&session.id, &combatant_id, 10).unwrap();

        let combat = manager.get_combat(&session.id).unwrap();
        let fighter = combat.combatants.iter().find(|c| c.id == combatant_id).unwrap();
        assert_eq!(fighter.temp_hp, Some(20));
    }

    #[test]
    fn test_damage_logs_combat_event() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = create_combatant_with_hp("Fighter", 15, 50, 50, None);
        let combatant_id = combatant.id.clone();
        manager.add_combatant(&session.id, combatant).unwrap();

        manager.damage_combatant(&session.id, &combatant_id, 15).unwrap();

        let events = manager.get_combat_log(&session.id);
        assert!(!events.is_empty());

        let damage_event = events.iter().find(|e| matches!(e.event_type, CombatEventType::Damage));
        assert!(damage_event.is_some());
    }

    #[test]
    fn test_heal_logs_combat_event() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = create_combatant_with_hp("Fighter", 15, 30, 50, None);
        let combatant_id = combatant.id.clone();
        manager.add_combatant(&session.id, combatant).unwrap();

        manager.heal_combatant(&session.id, &combatant_id, 15).unwrap();

        let events = manager.get_combat_log(&session.id);
        let heal_event = events.iter().find(|e| matches!(e.event_type, CombatEventType::Healing));
        assert!(heal_event.is_some());
    }
}

// ============================================================================
// Condition Application Tests
// ============================================================================

#[cfg(test)]
mod condition_tests {
    use super::*;

    #[test]
    fn test_add_single_condition() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = manager.add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player).unwrap();

        manager.add_condition_by_name(&session.id, &combatant.id, "Stunned", None, None, None).unwrap();

        let conditions = manager.get_combatant_conditions(&session.id, &combatant.id).unwrap();
        assert_eq!(conditions.len(), 1);
        assert_eq!(conditions[0].name, "Stunned");
    }

    #[test]
    fn test_add_multiple_conditions() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = manager.add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player).unwrap();

        manager.add_condition_by_name(&session.id, &combatant.id, "Stunned", None, None, None).unwrap();
        manager.add_condition_by_name(&session.id, &combatant.id, "Poisoned", None, None, None).unwrap();

        let conditions = manager.get_combatant_conditions(&session.id, &combatant.id).unwrap();
        assert_eq!(conditions.len(), 2);
    }

    #[test]
    fn test_remove_condition() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = manager.add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player).unwrap();

        manager.add_condition_by_name(&session.id, &combatant.id, "Stunned", None, None, None).unwrap();
        manager.remove_advanced_condition_by_name(&session.id, &combatant.id, "Stunned").unwrap();

        let conditions = manager.get_combatant_conditions(&session.id, &combatant.id).unwrap();
        assert!(conditions.is_empty());
    }

    #[test]
    fn test_condition_logs_combat_event() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = manager.add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player).unwrap();

        manager.add_condition_by_name(&session.id, &combatant.id, "Stunned", None, None, None).unwrap();

        let events = manager.get_combat_log(&session.id);
        let condition_event = events.iter().find(|e| matches!(e.event_type, CombatEventType::ConditionApplied));
        assert!(condition_event.is_some());
    }

    #[test]
    fn test_common_conditions_exist() {
        let conditions = vec![
            "Blinded", "Charmed", "Frightened", "Grappled", "Incapacitated",
            "Invisible", "Paralyzed", "Poisoned", "Prone", "Restrained",
            "Stunned", "Unconscious", "Concentrating",
        ];

        for name in conditions {
            let condition = ConditionTemplates::by_name(name);
            assert!(condition.is_some(), "Condition '{}' should exist", name);
        }
    }

    #[test]
    fn test_unknown_condition_returns_none() {
        let condition = ConditionTemplates::by_name("made-up-condition");
        assert!(condition.is_none());
    }
}

// ============================================================================
// Advanced Condition Tests
// ============================================================================

#[cfg(test)]
mod advanced_condition_tests {
    use super::*;

    #[test]
    fn test_add_advanced_condition() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = manager.add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player).unwrap();

        let condition = ConditionTemplates::stunned();
        manager.add_advanced_condition(&session.id, &combatant.id, condition).unwrap();

        let conditions = manager.get_combatant_conditions(&session.id, &combatant.id).unwrap();
        assert_eq!(conditions.len(), 1);
        assert_eq!(conditions[0].name, "Stunned");
    }

    #[test]
    fn test_add_condition_by_name() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = manager.add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player).unwrap();

        manager.add_condition_by_name(
            &session.id,
            &combatant.id,
            "Poisoned",
            None,
            None,
            None,
        ).unwrap();

        let conditions = manager.get_combatant_conditions(&session.id, &combatant.id).unwrap();
        assert_eq!(conditions.len(), 1);
        assert_eq!(conditions[0].name, "Poisoned");
    }

    #[test]
    fn test_add_condition_with_duration() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = manager.add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player).unwrap();

        manager.add_condition_by_name(
            &session.id,
            &combatant.id,
            "Poisoned",
            Some(AdvancedConditionDuration::Rounds(3)),
            None,
            None,
        ).unwrap();

        let conditions = manager.get_combatant_conditions(&session.id, &combatant.id).unwrap();
        assert_eq!(conditions[0].remaining, Some(3));
    }

    #[test]
    fn test_condition_immunity() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = manager.add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player).unwrap();

        // Add immunity
        manager.add_condition_immunity(&session.id, &combatant.id, "Frightened").unwrap();

        // Try to apply Frightened condition
        let condition = ConditionTemplates::frightened();
        manager.add_advanced_condition(&session.id, &combatant.id, condition).unwrap();

        // Should not have the condition
        let conditions = manager.get_combatant_conditions(&session.id, &combatant.id).unwrap();
        assert!(conditions.is_empty());
    }

    #[test]
    fn test_remove_advanced_condition_by_name() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = manager.add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player).unwrap();

        let condition = ConditionTemplates::poisoned();
        manager.add_advanced_condition(&session.id, &combatant.id, condition).unwrap();

        let removed = manager.remove_advanced_condition_by_name(&session.id, &combatant.id, "Poisoned").unwrap();
        assert_eq!(removed.len(), 1);

        let conditions = manager.get_combatant_conditions(&session.id, &combatant.id).unwrap();
        assert!(conditions.is_empty());
    }

    #[test]
    fn test_condition_stacking_no_stack() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = manager.add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player).unwrap();

        // Exhaustion has NoStack rule
        let exhaustion1 = ConditionTemplates::exhaustion(1);
        let exhaustion2 = ConditionTemplates::exhaustion(1);

        manager.add_advanced_condition(&session.id, &combatant.id, exhaustion1).unwrap();
        manager.add_advanced_condition(&session.id, &combatant.id, exhaustion2).unwrap();

        // Should only have one exhaustion
        let conditions = manager.get_combatant_conditions(&session.id, &combatant.id).unwrap();
        assert_eq!(conditions.len(), 1);
    }

    #[test]
    fn test_condition_save_attempt() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = manager.add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player).unwrap();

        // Create a condition that requires a save
        let condition = AdvancedCondition::new(
            "Web",
            "Restrained by webs",
            AdvancedConditionDuration::UntilSave {
                save_type: "STR".to_string(),
                dc: 12,
                timing: SaveTiming::EndOfTurn,
            },
        );

        manager.add_advanced_condition(&session.id, &combatant.id, condition.clone()).unwrap();

        let conditions = manager.get_combatant_conditions(&session.id, &combatant.id).unwrap();
        let condition_id = conditions[0].id.clone();

        // Failed save (roll 10)
        let result = manager.attempt_condition_save(&session.id, &combatant.id, &condition_id, 10).unwrap();
        assert!(!result);

        // Condition should still be there
        let conditions = manager.get_combatant_conditions(&session.id, &combatant.id).unwrap();
        assert_eq!(conditions.len(), 1);
    }

    #[test]
    fn test_condition_save_success_removes_condition() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = manager.add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player).unwrap();

        let condition = AdvancedCondition::new(
            "Web",
            "Restrained by webs",
            AdvancedConditionDuration::UntilSave {
                save_type: "STR".to_string(),
                dc: 12,
                timing: SaveTiming::EndOfTurn,
            },
        );

        manager.add_advanced_condition(&session.id, &combatant.id, condition.clone()).unwrap();

        let conditions = manager.get_combatant_conditions(&session.id, &combatant.id).unwrap();
        let condition_id = conditions[0].id.clone();

        // Successful save (roll 15)
        let result = manager.attempt_condition_save(&session.id, &combatant.id, &condition_id, 15).unwrap();
        assert!(result);

        // Condition should be removed
        let conditions = manager.get_combatant_conditions(&session.id, &combatant.id).unwrap();
        assert!(conditions.is_empty());
    }

    #[test]
    fn test_list_condition_templates() {
        let names = SessionManager::list_condition_templates();

        assert!(names.contains(&"Blinded"));
        assert!(names.contains(&"Stunned"));
        assert!(names.contains(&"Poisoned"));
        assert!(names.contains(&"Exhaustion 1"));
    }
}

// ============================================================================
// Condition Duration Tests
// ============================================================================

#[cfg(test)]
mod condition_duration_tests {
    use super::*;

    #[test]
    fn test_condition_tick_on_turn_end() {
        let mut condition = AdvancedCondition::new(
            "Stunned",
            "Cannot act",
            AdvancedConditionDuration::Turns(2),
        );

        assert_eq!(condition.remaining, Some(2));

        // Tick at end of own turn
        let expired = condition.tick_end_of_turn(true);
        assert!(!expired);
        assert_eq!(condition.remaining, Some(1));

        // Second tick - should expire
        let expired = condition.tick_end_of_turn(true);
        assert!(expired);
    }

    #[test]
    fn test_condition_tick_not_own_turn() {
        let mut condition = AdvancedCondition::new(
            "Stunned",
            "Cannot act",
            AdvancedConditionDuration::Turns(2),
        );

        // Tick but not own turn - should not decrement
        let expired = condition.tick_end_of_turn(false);
        assert!(!expired);
        assert_eq!(condition.remaining, Some(2));
    }

    #[test]
    fn test_condition_end_of_next_turn() {
        let mut condition = AdvancedCondition::new(
            "Staggered",
            "Off balance",
            AdvancedConditionDuration::EndOfNextTurn,
        );

        // On own turn end - should expire
        let expired = condition.tick_end_of_turn(true);
        assert!(expired);
    }

    #[test]
    fn test_condition_start_of_next_turn() {
        let mut condition = AdvancedCondition::new(
            "Hesitant",
            "Uncertain",
            AdvancedConditionDuration::StartOfNextTurn,
        );

        // End of turn - should not expire
        let expired = condition.tick_end_of_turn(true);
        assert!(!expired);

        // Start of own turn - should expire
        let expired = condition.tick_start_of_turn(true);
        assert!(expired);
    }

    #[test]
    fn test_condition_round_based_duration() {
        let mut condition = AdvancedCondition::new(
            "Blessing",
            "Divine protection",
            AdvancedConditionDuration::Rounds(3),
        );

        assert_eq!(condition.remaining, Some(3));

        // Round tick
        let expired = condition.tick_round();
        assert!(!expired);
        assert_eq!(condition.remaining, Some(2));
    }

    #[test]
    fn test_condition_tracker_tick_end_of_turn() {
        let mut tracker = ConditionTracker::new();

        let condition = AdvancedCondition::new(
            "Stunned",
            "Cannot act",
            AdvancedConditionDuration::Turns(1),
        );

        tracker.add_condition(condition).unwrap();

        let expired = tracker.tick_end_of_turn(true);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].name, "Stunned");
        assert!(tracker.conditions().is_empty());
    }

    #[test]
    fn test_condition_tracker_tick_round() {
        let mut tracker = ConditionTracker::new();

        let condition = AdvancedCondition::new(
            "Bless",
            "+1d4 to attacks and saves",
            AdvancedConditionDuration::Rounds(1),
        );

        tracker.add_condition(condition).unwrap();

        let expired = tracker.tick_round();
        assert_eq!(expired.len(), 1);
        assert!(tracker.conditions().is_empty());
    }
}

// ============================================================================
// Combat Death and Removal Tests
// ============================================================================

#[cfg(test)]
mod combat_death_tests {
    use super::*;

    #[test]
    fn test_damage_to_zero_hp() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = create_monster("Goblin", 15, 10);
        let combatant_id = combatant.id.clone();
        manager.add_combatant(&session.id, combatant).unwrap();

        let remaining = manager.damage_combatant(&session.id, &combatant_id, 10).unwrap();
        assert_eq!(remaining, 0);
    }

    #[test]
    fn test_overkill_damage() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = create_monster("Goblin", 15, 10);
        let combatant_id = combatant.id.clone();
        manager.add_combatant(&session.id, combatant).unwrap();

        // 50 damage to 10 HP creature
        let remaining = manager.damage_combatant(&session.id, &combatant_id, 50).unwrap();
        assert_eq!(remaining, 0);
    }

    #[test]
    fn test_remove_dead_combatant() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let goblin = create_monster("Goblin", 15, 10);
        let goblin_id = goblin.id.clone();
        manager.add_combatant(&session.id, goblin).unwrap();
        manager.add_combatant_quick(&session.id, "Fighter", 18, CombatantType::Player).unwrap();

        // Kill and remove goblin
        manager.damage_combatant(&session.id, &goblin_id, 10).unwrap();
        manager.remove_combatant(&session.id, &goblin_id).unwrap();

        let combat = manager.get_combat(&session.id).unwrap();
        assert_eq!(combat.combatants.len(), 1);
        assert_eq!(combat.combatants[0].name, "Fighter");
    }

    #[test]
    fn test_combat_with_all_monsters_dead() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let goblin = create_monster("Goblin", 15, 10);
        let goblin_id = goblin.id.clone();
        manager.add_combatant(&session.id, goblin).unwrap();
        manager.add_combatant_quick(&session.id, "Fighter", 18, CombatantType::Player).unwrap();

        manager.damage_combatant(&session.id, &goblin_id, 10).unwrap();
        manager.remove_combatant(&session.id, &goblin_id).unwrap();

        // Combat can be ended
        let result = manager.end_combat(&session.id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_log_death_event() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let _combatant = manager.add_combatant_quick(&session.id, "Fighter", 18, CombatantType::Player).unwrap();

        manager.log_combat_event(
            &session.id,
            "Fighter",
            CombatEventType::Death,
            "Fighter has fallen!",
        ).unwrap();

        let events = manager.get_combat_log(&session.id);
        let death_event = events.iter().find(|e| matches!(e.event_type, CombatEventType::Death));
        assert!(death_event.is_some());
    }
}

// ============================================================================
// Session Notes Tests
// ============================================================================

#[cfg(test)]
mod session_notes_tests {
    use super::*;

    #[test]
    fn test_create_note() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let note = SessionNote::new(
            &session.id,
            "campaign-001",
            "Combat Summary",
            "The party defeated the goblins in the forest.",
        );

        manager.create_note(note).unwrap();

        let notes = manager.list_notes_for_session(&session.id);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "Combat Summary");
    }

    #[test]
    fn test_get_note() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let note = SessionNote::new(
            &session.id,
            "campaign-001",
            "Test Note",
            "Content here",
        );
        let note_id = note.id.clone();
        manager.create_note(note).unwrap();

        let retrieved = manager.get_note(&note_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test Note");
    }

    #[test]
    fn test_update_note() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let mut note = SessionNote::new(
            &session.id,
            "campaign-001",
            "Original Title",
            "Original content",
        );
        let _note_id = note.id.clone();
        manager.create_note(note.clone()).unwrap();

        note.title = "Updated Title".to_string();
        let updated = manager.update_note(note).unwrap();

        assert_eq!(updated.title, "Updated Title");
    }

    #[test]
    fn test_delete_note() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let note = SessionNote::new(
            &session.id,
            "campaign-001",
            "To Delete",
            "Will be deleted",
        );
        let note_id = note.id.clone();
        manager.create_note(note).unwrap();

        manager.delete_note(&note_id).unwrap();

        let notes = manager.list_notes_for_session(&session.id);
        assert!(notes.is_empty());
    }

    #[test]
    fn test_note_with_category() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let note = SessionNote::new(
            &session.id,
            "campaign-001",
            "Battle Notes",
            "Combat details here",
        ).with_category(NoteCategory::Combat);

        manager.create_note(note).unwrap();

        let combat_notes = manager.get_notes_by_category(&NoteCategory::Combat, Some(&session.id));
        assert_eq!(combat_notes.len(), 1);
    }

    #[test]
    fn test_search_notes() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let note1 = SessionNote::new(
            &session.id,
            "campaign-001",
            "Goblin Fight",
            "The party fought goblins",
        );

        let note2 = SessionNote::new(
            &session.id,
            "campaign-001",
            "Town Visit",
            "The party visited the town",
        );

        manager.create_note(note1).unwrap();
        manager.create_note(note2).unwrap();

        let results = manager.search_notes("goblin", Some(&session.id));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Goblin Fight");
    }

    #[test]
    fn test_search_notes_by_tag() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let note = SessionNote::new(
            &session.id,
            "campaign-001",
            "Important Event",
            "Something important happened",
        ).with_tags(["important", "milestone"]);

        manager.create_note(note).unwrap();

        let results = manager.get_notes_by_tag("important");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_link_entity_to_note() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let note = SessionNote::new(
            &session.id,
            "campaign-001",
            "NPC Interaction",
            "Met Bartender Bob",
        );
        let note_id = note.id.clone();
        manager.create_note(note).unwrap();

        manager.link_entity_to_note(
            &note_id,
            NoteEntityType::NPC,
            "npc-001",
            "Bartender Bob",
        ).unwrap();

        let note = manager.get_note(&note_id).unwrap();
        assert_eq!(note.entity_links.len(), 1);
        assert_eq!(note.entity_links[0].display_name, "Bartender Bob");
    }

    #[test]
    fn test_unlink_entity_from_note() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let note = SessionNote::new(
            &session.id,
            "campaign-001",
            "NPC Interaction",
            "Met Bartender Bob",
        ).with_entity_link(NoteEntityType::NPC, "npc-001", "Bartender Bob");

        let note_id = note.id.clone();
        manager.create_note(note).unwrap();

        manager.unlink_entity_from_note(&note_id, "npc-001").unwrap();

        let note = manager.get_note(&note_id).unwrap();
        assert!(note.entity_links.is_empty());
    }
}

// ============================================================================
// Timeline Events Tests
// ============================================================================

#[cfg(test)]
mod timeline_tests {
    use super::*;

    #[test]
    fn test_session_start_creates_timeline_event() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let events = manager.get_timeline_events(&session.id);

        let start_event = events.iter().find(|e| matches!(e.event_type, TimelineEventType::SessionStart));
        assert!(start_event.is_some());
    }

    #[test]
    fn test_combat_start_creates_timeline_event() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let events = manager.get_timeline_events(&session.id);

        let combat_event = events.iter().find(|e| matches!(e.event_type, TimelineEventType::CombatStart));
        assert!(combat_event.is_some());
    }

    #[test]
    fn test_combat_end_creates_timeline_event() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();
        manager.end_combat(&session.id).unwrap();

        let events = manager.get_timeline_events(&session.id);

        let combat_end = events.iter().find(|e| matches!(e.event_type, TimelineEventType::CombatEnd));
        assert!(combat_end.is_some());
    }

    #[test]
    fn test_session_end_creates_timeline_event() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.end_session(&session.id).unwrap();

        let events = manager.get_timeline_events(&session.id);

        let end_event = events.iter().find(|e| matches!(e.event_type, TimelineEventType::SessionEnd));
        assert!(end_event.is_some());
    }

    #[test]
    fn test_add_custom_timeline_event() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let event = TimelineEvent::new(
            &session.id,
            TimelineEventType::Custom("discovery".to_string()),
            "Hidden Treasure Found",
            "The party discovered a hidden chest in the tavern",
        );

        manager.add_timeline_event(&session.id, event).unwrap();

        let events = manager.get_timeline_events(&session.id);
        assert!(events.len() >= 2); // At least session start + custom
    }

    #[test]
    fn test_filter_timeline_by_type() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();
        manager.end_combat(&session.id).unwrap();

        let combat_events = manager.get_timeline_events_by_type(&session.id, &TimelineEventType::CombatStart);
        assert_eq!(combat_events.len(), 1);
    }

    #[test]
    fn test_filter_timeline_by_severity() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        // Add events with different severity
        manager.log_session_event(
            &session.id,
            TimelineEventType::Custom("minor".to_string()),
            "Minor Event",
            "Something minor happened",
        ).unwrap();

        manager.log_combat_timeline_event(
            &session.id,
            TimelineEventType::CombatDeath,
            "Party Member Down!",
            "The fighter has fallen!",
            EventSeverity::Critical,
        ).unwrap();

        let critical_events = manager.get_timeline_events_by_severity(&session.id, EventSeverity::Critical);
        assert!(!critical_events.is_empty());
    }

    #[test]
    fn test_filter_timeline_by_entity() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let event = TimelineEvent::new(
            &session.id,
            TimelineEventType::NPCInteraction,
            "Met the Blacksmith",
            "The party spoke with Torgin the Blacksmith",
        ).with_entity("npc", "torgin-001", "Torgin");

        manager.add_timeline_event(&session.id, event).unwrap();

        let entity_events = manager.get_timeline_events_for_entity(&session.id, "torgin-001");
        assert_eq!(entity_events.len(), 1);
    }

    #[test]
    fn test_get_recent_timeline_events() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        // Add multiple events
        for i in 0..5 {
            manager.log_session_event(
                &session.id,
                TimelineEventType::Custom(format!("event-{}", i)),
                &format!("Event {}", i),
                "Description",
            ).unwrap();
        }

        let recent = manager.get_recent_timeline_events(&session.id, 3);
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_timeline_summary() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();
        manager.end_combat(&session.id).unwrap();

        let summary = manager.get_timeline_summary(&session.id);
        assert!(summary.is_ok());

        let summary = summary.unwrap();
        assert_eq!(summary.combat.encounters, 1);
    }

    #[test]
    fn test_timeline_narrative() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();
        manager.end_combat(&session.id).unwrap();
        manager.end_session(&session.id).unwrap();

        let narrative = manager.get_timeline_narrative(&session.id);
        assert!(narrative.is_some());

        let narrative = narrative.unwrap();
        assert!(!narrative.is_empty());
        assert!(narrative.contains("Session"));
    }

    #[test]
    fn test_timeline_ordering() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        // Events should be in chronological order
        manager.log_session_event(&session.id, TimelineEventType::Custom("first".to_string()), "First", "").unwrap();
        manager.log_session_event(&session.id, TimelineEventType::Custom("second".to_string()), "Second", "").unwrap();
        manager.log_session_event(&session.id, TimelineEventType::Custom("third".to_string()), "Third", "").unwrap();

        let events = manager.get_timeline_events(&session.id);

        // Verify ordering (SessionStart should be first, then our custom events)
        for i in 1..events.len() {
            assert!(events[i].timestamp >= events[i-1].timestamp);
        }
    }
}

// ============================================================================
// Session Snapshot Tests (Conceptual - based on available functionality)
// ============================================================================

#[cfg(test)]
mod session_snapshot_tests {
    use super::*;

    #[test]
    fn test_session_state_can_be_captured() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        // Add some state
        manager.start_combat(&session.id).unwrap();
        manager.add_combatant_quick(&session.id, "Fighter", 18, CombatantType::Player).unwrap();
        manager.add_combatant_quick(&session.id, "Goblin", 15, CombatantType::Monster).unwrap();

        // Get current state
        let current_session = manager.get_session(&session.id).unwrap();
        let combat = manager.get_combat(&session.id).unwrap();

        // Verify state can be retrieved
        assert!(current_session.combat.is_some());
        assert_eq!(combat.combatants.len(), 2);
    }

    #[test]
    fn test_combat_state_serializable() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();
        manager.add_combatant_quick(&session.id, "Fighter", 18, CombatantType::Player).unwrap();

        let combat = manager.get_combat(&session.id).unwrap();

        // Should be serializable
        let json = serde_json::to_string(&combat);
        assert!(json.is_ok());

        // Should be deserializable
        let deserialized: Result<CombatState, _> = serde_json::from_str(&json.unwrap());
        assert!(deserialized.is_ok());
    }

    #[test]
    fn test_session_serializable() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        // Should be serializable
        let json = serde_json::to_string(&session);
        assert!(json.is_ok());

        // Should be deserializable
        let deserialized: Result<GameSession, _> = serde_json::from_str(&json.unwrap());
        assert!(deserialized.is_ok());
    }

    #[test]
    fn test_notes_export() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let note = SessionNote::new(
            &session.id,
            "campaign-001",
            "Test Note",
            "Test content",
        );
        manager.create_note(note).unwrap();

        // Notes can be listed for export
        let notes = manager.list_notes_for_session(&session.id);
        assert_eq!(notes.len(), 1);

        // Notes are serializable
        let json = serde_json::to_string(&notes);
        assert!(json.is_ok());
    }

    #[test]
    fn test_timeline_export() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let events = manager.get_timeline_events(&session.id);

        // Timeline events are serializable
        let json = serde_json::to_string(&events);
        assert!(json.is_ok());
    }
}

// ============================================================================
// Combat Event Logging Tests
// ============================================================================

#[cfg(test)]
mod combat_event_logging_tests {
    use super::*;

    #[test]
    fn test_log_attack_event() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        manager.log_combat_event(
            &session.id,
            "Fighter",
            CombatEventType::Attack,
            "Fighter attacks Goblin with longsword",
        ).unwrap();

        let events = manager.get_combat_log(&session.id);
        let attack = events.iter().find(|e| matches!(e.event_type, CombatEventType::Attack));
        assert!(attack.is_some());
    }

    #[test]
    fn test_log_movement_event() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        manager.log_combat_event(
            &session.id,
            "Rogue",
            CombatEventType::Movement,
            "Rogue moves 30 feet to flank",
        ).unwrap();

        let events = manager.get_combat_log(&session.id);
        let movement = events.iter().find(|e| matches!(e.event_type, CombatEventType::Movement));
        assert!(movement.is_some());
    }

    #[test]
    fn test_combat_events_have_round_and_turn() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        manager.add_combatant_quick(&session.id, "Fighter", 18, CombatantType::Player).unwrap();
        manager.add_combatant_quick(&session.id, "Wizard", 15, CombatantType::Player).unwrap();

        // Advance to round 2
        manager.next_turn(&session.id).unwrap();
        manager.next_turn(&session.id).unwrap();

        manager.log_combat_event(
            &session.id,
            "Fighter",
            CombatEventType::Action,
            "Fighter takes action",
        ).unwrap();

        let events = manager.get_combat_log(&session.id);
        let action_event = events.iter()
            .find(|e| matches!(e.event_type, CombatEventType::Action))
            .unwrap();

        assert_eq!(action_event.round, 2);
    }

    #[test]
    fn test_get_empty_combat_log() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        // No combat started
        let events = manager.get_combat_log(&session.id);
        assert!(events.is_empty());
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_damage_combatant_no_hp_set() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        // Combatant without HP
        let combatant = create_test_combatant("Environment Hazard", 0, None);
        let combatant_id = combatant.id.clone();
        manager.add_combatant(&session.id, combatant).unwrap();

        // Should return 0 (no HP to damage)
        let result = manager.damage_combatant(&session.id, &combatant_id, 10);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_heal_combatant_no_hp_set() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        // Combatant without HP
        let combatant = create_test_combatant("Spirit", 0, None);
        let combatant_id = combatant.id.clone();
        manager.add_combatant(&session.id, combatant).unwrap();

        // Should return 0 (no HP to heal)
        let result = manager.heal_combatant(&session.id, &combatant_id, 10);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_zero_damage() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = create_combatant_with_hp("Fighter", 15, 50, 50, None);
        let combatant_id = combatant.id.clone();
        manager.add_combatant(&session.id, combatant).unwrap();

        let remaining = manager.damage_combatant(&session.id, &combatant_id, 0).unwrap();
        assert_eq!(remaining, 50);
    }

    #[test]
    fn test_zero_healing() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = create_combatant_with_hp("Fighter", 15, 30, 50, None);
        let combatant_id = combatant.id.clone();
        manager.add_combatant(&session.id, combatant).unwrap();

        let new_hp = manager.heal_combatant(&session.id, &combatant_id, 0).unwrap();
        assert_eq!(new_hp, 30);
    }

    #[test]
    fn test_same_initiative_multiple_combatants() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        // Add 5 combatants with same initiative
        for i in 0..5 {
            let mut combatant = create_test_combatant(&format!("Goblin {}", i), 15, Some(10));
            combatant.initiative_modifier = i; // Different modifiers for tie-breaking
            manager.add_combatant(&session.id, combatant).unwrap();
        }

        let combat = manager.get_combat(&session.id).unwrap();
        assert_eq!(combat.combatants.len(), 5);

        // Verify they're sorted by modifier (highest first)
        for i in 1..combat.combatants.len() {
            assert!(combat.combatants[i-1].initiative_modifier >= combat.combatants[i].initiative_modifier);
        }
    }

    #[test]
    fn test_very_long_session_notes() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let long_content = "Lorem ipsum ".repeat(1000);
        let note = SessionNote::new(
            &session.id,
            "campaign-001",
            "Long Note",
            &long_content,
        );

        manager.create_note(note).unwrap();

        let notes = manager.list_notes_for_session(&session.id);
        assert_eq!(notes.len(), 1);
        assert!(notes[0].content.len() > 10000);
    }

    #[test]
    fn test_special_characters_in_names() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);
        manager.start_combat(&session.id).unwrap();

        let combatant = manager.add_combatant_quick(
            &session.id,
            "Xal'atath, Blade of the Black Empire",
            20,
            CombatantType::Monster,
        ).unwrap();

        assert_eq!(combatant.name, "Xal'atath, Blade of the Black Empire");
    }

    #[test]
    fn test_unicode_in_notes() {
        let manager = create_test_manager();
        let session = manager.start_session("campaign-001", 1);

        let note = SessionNote::new(
            &session.id,
            "campaign-001",
            "Dragon Language",
            "The dragon spoke: 'Dovahkiin!'",
        );

        manager.create_note(note).unwrap();

        let results = manager.search_notes("Dovahkiin", None);
        assert_eq!(results.len(), 1);
    }
}

// ============================================================================
// Concurrency Safety Tests (Basic)
// ============================================================================

#[cfg(test)]
mod concurrency_tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_concurrent_read_access() {
        let manager = Arc::new(create_test_manager());
        let session = manager.start_session("campaign-001", 1);

        let mut handles = vec![];

        for _ in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let session_id = session.id.clone();

            handles.push(thread::spawn(move || {
                let result = manager_clone.get_session(&session_id);
                assert!(result.is_some());
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_concurrent_session_creation() {
        let manager = Arc::new(create_test_manager());
        let mut handles = vec![];

        for i in 0..10 {
            let manager_clone = Arc::clone(&manager);

            handles.push(thread::spawn(move || {
                manager_clone.start_session(&format!("campaign-{}", i), 1);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all sessions were created
        for i in 0..10 {
            let sessions = manager.list_sessions(&format!("campaign-{}", i));
            assert_eq!(sessions.len(), 1);
        }
    }
}
