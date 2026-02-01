//! Combat Mechanics Tests
//!
//! Tests for combat system including:
//! - Combat start/end
//! - Combatant management (add, remove, update)
//! - Initiative ordering and tie-breaking
//! - Turn advancement and wrap-around
//! - HP modification (damage, healing, temp HP)
//! - HP bounds (no negative, no exceeding max)
//! - Combat event logging

use crate::core::session_manager::{
    CombatEventType, CombatStatus, Combatant, CombatantType, SessionError, SessionManager,
};
use crate::tests::common::fixtures::{
    create_combatant_with_hp, create_monster, create_test_combatant,
};

// =============================================================================
// Test Helpers
// =============================================================================

fn create_test_manager() -> SessionManager {
    SessionManager::new()
}

// =============================================================================
// Combat Start Tests
// =============================================================================

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
        SessionError::SessionNotFound(_) => {}
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
        SessionError::CombatAlreadyActive => {}
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
        SessionError::NoCombatActive => {}
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

// =============================================================================
// Combatant Management Tests
// =============================================================================

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

    let result =
        manager.add_combatant_quick(&session.id, "Goblin", 15, CombatantType::Monster);

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

    manager
        .add_combatant_quick(&session.id, "Fighter", 18, CombatantType::Player)
        .unwrap();
    manager
        .add_combatant_quick(&session.id, "Wizard", 12, CombatantType::Player)
        .unwrap();
    manager
        .add_combatant_quick(&session.id, "Goblin", 15, CombatantType::Monster)
        .unwrap();

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
        SessionError::NoCombatActive => {}
        _ => panic!("Expected NoCombatActive error"),
    }
}

#[test]
fn test_remove_combatant() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);
    manager.start_combat(&session.id).unwrap();

    let combatant = manager
        .add_combatant_quick(&session.id, "Fighter", 18, CombatantType::Player)
        .unwrap();

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
        SessionError::CombatantNotFound(_) => {}
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
    manager
        .add_combatant(&session.id, combatant.clone())
        .unwrap();

    combatant.current_hp = Some(30);
    let result = manager.update_combatant(&session.id, combatant);

    assert!(result.is_ok());

    let combat = manager.get_combat(&session.id).unwrap();
    let updated = combat
        .combatants
        .iter()
        .find(|c| c.id == combatant_id)
        .unwrap();
    assert_eq!(updated.current_hp, Some(30));
}

// =============================================================================
// Initiative Ordering Tests
// =============================================================================

#[test]
fn test_initiative_order_descending() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);
    manager.start_combat(&session.id).unwrap();

    manager
        .add_combatant_quick(&session.id, "Fighter", 18, CombatantType::Player)
        .unwrap();
    manager
        .add_combatant_quick(&session.id, "Wizard", 12, CombatantType::Player)
        .unwrap();
    manager
        .add_combatant_quick(&session.id, "Rogue", 20, CombatantType::Player)
        .unwrap();

    let combat = manager.get_combat(&session.id).unwrap();

    // Should be sorted highest to lowest
    assert_eq!(combat.combatants[0].name, "Rogue"); // 20
    assert_eq!(combat.combatants[1].name, "Fighter"); // 18
    assert_eq!(combat.combatants[2].name, "Wizard"); // 12
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

    let fighter = manager
        .add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player)
        .unwrap();
    manager
        .add_combatant_quick(&session.id, "Wizard", 18, CombatantType::Player)
        .unwrap();

    // Fighter starts second
    let combat = manager.get_combat(&session.id).unwrap();
    assert_eq!(combat.combatants[0].name, "Wizard");

    // Update Fighter's initiative to be higher
    manager
        .set_initiative(&session.id, &fighter.id, 20)
        .unwrap();

    // Now Fighter should be first
    let combat = manager.get_combat(&session.id).unwrap();
    assert_eq!(combat.combatants[0].name, "Fighter");
}

#[test]
fn test_initiative_resorts_on_add() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);
    manager.start_combat(&session.id).unwrap();

    manager
        .add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player)
        .unwrap();
    manager
        .add_combatant_quick(&session.id, "Wizard", 18, CombatantType::Player)
        .unwrap();

    // Add someone in between
    manager
        .add_combatant_quick(&session.id, "Rogue", 17, CombatantType::Player)
        .unwrap();

    let combat = manager.get_combat(&session.id).unwrap();
    assert_eq!(combat.combatants[0].name, "Wizard"); // 18
    assert_eq!(combat.combatants[1].name, "Rogue"); // 17
    assert_eq!(combat.combatants[2].name, "Fighter"); // 15
}

// =============================================================================
// Turn Advancement Tests
// =============================================================================

#[test]
fn test_next_turn_advances_correctly() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);
    manager.start_combat(&session.id).unwrap();

    manager
        .add_combatant_quick(&session.id, "Fighter", 20, CombatantType::Player)
        .unwrap();
    manager
        .add_combatant_quick(&session.id, "Wizard", 15, CombatantType::Player)
        .unwrap();
    manager
        .add_combatant_quick(&session.id, "Rogue", 10, CombatantType::Player)
        .unwrap();

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

    manager
        .add_combatant_quick(&session.id, "Fighter", 20, CombatantType::Player)
        .unwrap();
    manager
        .add_combatant_quick(&session.id, "Wizard", 15, CombatantType::Player)
        .unwrap();

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

    manager
        .add_combatant_quick(&session.id, "Fighter", 20, CombatantType::Player)
        .unwrap();

    let mut wizard = create_test_combatant("Wizard", 15, Some(30));
    wizard.is_active = false; // Wizard is inactive
    manager.add_combatant(&session.id, wizard).unwrap();

    manager
        .add_combatant_quick(&session.id, "Rogue", 10, CombatantType::Player)
        .unwrap();

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

    manager
        .add_combatant_quick(&session.id, "Fighter", 20, CombatantType::Player)
        .unwrap();
    manager
        .add_combatant_quick(&session.id, "Wizard", 15, CombatantType::Player)
        .unwrap();

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

    manager
        .add_combatant_quick(&session.id, "Fighter", 20, CombatantType::Player)
        .unwrap();
    manager
        .add_combatant_quick(&session.id, "Wizard", 15, CombatantType::Player)
        .unwrap();

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

    let fighter = manager
        .add_combatant_quick(&session.id, "Fighter", 20, CombatantType::Player)
        .unwrap();
    manager
        .add_combatant_quick(&session.id, "Wizard", 15, CombatantType::Player)
        .unwrap();
    manager
        .add_combatant_quick(&session.id, "Rogue", 10, CombatantType::Player)
        .unwrap();

    // Advance to Wizard (index 1)
    manager.next_turn(&session.id).unwrap();

    // Remove Fighter (index 0)
    manager.remove_combatant(&session.id, &fighter.id).unwrap();

    // Current turn should adjust
    let current = manager.get_current_combatant(&session.id).unwrap();

    // Should still be on Wizard
    assert_eq!(current.name, "Wizard");
}

// =============================================================================
// HP Modification Tests
// =============================================================================

#[test]
fn test_damage_combatant() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);
    manager.start_combat(&session.id).unwrap();

    let combatant = create_combatant_with_hp("Fighter", 15, 50, 50, None);
    let combatant_id = combatant.id.clone();
    manager.add_combatant(&session.id, combatant).unwrap();

    let remaining = manager
        .damage_combatant(&session.id, &combatant_id, 15)
        .unwrap();

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
    let remaining = manager
        .damage_combatant(&session.id, &combatant_id, 15)
        .unwrap();

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

    let remaining = manager
        .damage_combatant(&session.id, &combatant_id, 100)
        .unwrap();

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

    let new_hp = manager
        .heal_combatant(&session.id, &combatant_id, 15)
        .unwrap();

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

    let new_hp = manager
        .heal_combatant(&session.id, &combatant_id, 100)
        .unwrap();

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

    manager
        .add_temp_hp(&session.id, &combatant_id, 15)
        .unwrap();

    let combat = manager.get_combat(&session.id).unwrap();
    let fighter = combat
        .combatants
        .iter()
        .find(|c| c.id == combatant_id)
        .unwrap();
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
    manager
        .add_temp_hp(&session.id, &combatant_id, 15)
        .unwrap();

    let combat = manager.get_combat(&session.id).unwrap();
    let fighter = combat
        .combatants
        .iter()
        .find(|c| c.id == combatant_id)
        .unwrap();
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
    manager
        .add_temp_hp(&session.id, &combatant_id, 10)
        .unwrap();

    let combat = manager.get_combat(&session.id).unwrap();
    let fighter = combat
        .combatants
        .iter()
        .find(|c| c.id == combatant_id)
        .unwrap();
    assert_eq!(fighter.temp_hp, Some(20));
}

// =============================================================================
// Combat Event Logging Tests
// =============================================================================

#[test]
fn test_damage_logs_combat_event() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);
    manager.start_combat(&session.id).unwrap();

    let combatant = create_combatant_with_hp("Fighter", 15, 50, 50, None);
    let combatant_id = combatant.id.clone();
    manager.add_combatant(&session.id, combatant).unwrap();

    manager
        .damage_combatant(&session.id, &combatant_id, 15)
        .unwrap();

    let events = manager.get_combat_log(&session.id);
    assert!(!events.is_empty());

    let damage_event = events
        .iter()
        .find(|e| matches!(e.event_type, CombatEventType::Damage));
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

    manager
        .heal_combatant(&session.id, &combatant_id, 15)
        .unwrap();

    let events = manager.get_combat_log(&session.id);
    let heal_event = events
        .iter()
        .find(|e| matches!(e.event_type, CombatEventType::Healing));
    assert!(heal_event.is_some());
}

// =============================================================================
// Combat Death Tests
// =============================================================================

#[test]
fn test_damage_to_zero_hp() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);
    manager.start_combat(&session.id).unwrap();

    let combatant = create_monster("Goblin", 15, 10);
    let combatant_id = combatant.id.clone();
    manager.add_combatant(&session.id, combatant).unwrap();

    let remaining = manager
        .damage_combatant(&session.id, &combatant_id, 10)
        .unwrap();

    assert_eq!(remaining, 0);
}

#[test]
fn test_overkill_damage_stops_at_zero() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);
    manager.start_combat(&session.id).unwrap();

    let combatant = create_monster("Goblin", 15, 10);
    let combatant_id = combatant.id.clone();
    manager.add_combatant(&session.id, combatant).unwrap();

    // 100 damage to a 10 HP goblin
    let remaining = manager
        .damage_combatant(&session.id, &combatant_id, 100)
        .unwrap();

    assert_eq!(remaining, 0);

    let combat = manager.get_combat(&session.id).unwrap();
    let goblin = combat
        .combatants
        .iter()
        .find(|c| c.id == combatant_id)
        .unwrap();
    assert_eq!(goblin.current_hp, Some(0));
}
