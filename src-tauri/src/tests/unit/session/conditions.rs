//! Condition Tests
//!
//! Tests for condition application, duration, stacking, and immunities.
//!
//! TODO: Migrate remaining tests from session_manager_tests.rs

use crate::core::session_manager::{CombatantType, SessionManager};
use crate::core::session::conditions::{
    AdvancedCondition, ConditionDuration as AdvancedConditionDuration,
    ConditionTemplates, ConditionTracker,
};
use crate::tests::common::fixtures::create_test_manager;

// =============================================================================
// Basic Condition Tests
// =============================================================================

#[test]
fn test_add_single_condition() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);
    manager.start_combat(&session.id).unwrap();

    let combatant = manager
        .add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player)
        .unwrap();

    manager
        .add_condition_by_name(&session.id, &combatant.id, "Stunned", None, None, None)
        .unwrap();

    let conditions = manager
        .get_combatant_conditions(&session.id, &combatant.id)
        .unwrap();
    assert_eq!(conditions.len(), 1);
    assert_eq!(conditions[0].name, "Stunned");
}

#[test]
fn test_add_multiple_conditions() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);
    manager.start_combat(&session.id).unwrap();

    let combatant = manager
        .add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player)
        .unwrap();

    manager
        .add_condition_by_name(&session.id, &combatant.id, "Stunned", None, None, None)
        .unwrap();
    manager
        .add_condition_by_name(&session.id, &combatant.id, "Poisoned", None, None, None)
        .unwrap();

    let conditions = manager
        .get_combatant_conditions(&session.id, &combatant.id)
        .unwrap();
    assert_eq!(conditions.len(), 2);
}

#[test]
fn test_remove_condition() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);
    manager.start_combat(&session.id).unwrap();

    let combatant = manager
        .add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player)
        .unwrap();

    manager
        .add_condition_by_name(&session.id, &combatant.id, "Stunned", None, None, None)
        .unwrap();
    manager
        .remove_advanced_condition_by_name(&session.id, &combatant.id, "Stunned")
        .unwrap();

    let conditions = manager
        .get_combatant_conditions(&session.id, &combatant.id)
        .unwrap();
    assert!(conditions.is_empty());
}

// =============================================================================
// Condition Template Tests
// =============================================================================

#[test]
fn test_common_conditions_exist() {
    let conditions = vec![
        "Blinded",
        "Charmed",
        "Frightened",
        "Grappled",
        "Incapacitated",
        "Invisible",
        "Paralyzed",
        "Poisoned",
        "Prone",
        "Restrained",
        "Stunned",
        "Unconscious",
        "Concentrating",
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

#[test]
fn test_list_condition_templates() {
    let names = SessionManager::list_condition_templates();

    assert!(names.contains(&"Blinded"));
    assert!(names.contains(&"Stunned"));
    assert!(names.contains(&"Poisoned"));
    assert!(names.contains(&"Exhaustion 1"));
}

// =============================================================================
// Condition Duration Tests
// =============================================================================

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

// =============================================================================
// Condition Immunity Tests
// =============================================================================

#[test]
fn test_condition_immunity() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);
    manager.start_combat(&session.id).unwrap();

    let combatant = manager
        .add_combatant_quick(&session.id, "Fighter", 15, CombatantType::Player)
        .unwrap();

    // Add immunity
    manager
        .add_condition_immunity(&session.id, &combatant.id, "Frightened")
        .unwrap();

    // Try to apply Frightened condition
    let condition = ConditionTemplates::frightened();
    manager
        .add_advanced_condition(&session.id, &combatant.id, condition)
        .unwrap();

    // Should not have the condition
    let conditions = manager
        .get_combatant_conditions(&session.id, &combatant.id)
        .unwrap();
    assert!(conditions.is_empty());
}
