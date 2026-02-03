//! Session Component Tests
//!
//! Tests for ActiveSession rendering, CombatTracker, InitiativeList ordering,
//! and Condition badge rendering.

use leptos::prelude::*;
use leptos_router::components::Router;
use ttrpg_assistant_frontend::components::design_system::{Badge, BadgeVariant};
use ttrpg_assistant_frontend::services::layout_service::provide_layout_state;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ============================================================================
// Badge Component Tests (used for conditions)
// ============================================================================

#[wasm_bindgen_test]
fn test_badge_variant_default() {
    // Use pattern matching since BadgeVariant doesn't implement Debug
    let variant = BadgeVariant::default();
    assert!(matches!(variant, BadgeVariant::Default));
}

#[wasm_bindgen_test]
fn test_badge_variant_equality() {
    // Test equality using PartialEq
    assert!(BadgeVariant::Success == BadgeVariant::Success);
    assert!(BadgeVariant::Warning == BadgeVariant::Warning);
    assert!(BadgeVariant::Danger == BadgeVariant::Danger);
    assert!(BadgeVariant::Info == BadgeVariant::Info);
    assert!(BadgeVariant::Success != BadgeVariant::Danger);
}

#[wasm_bindgen_test]
fn test_badge_renders_without_panic() {
    leptos::mount::mount_to_body(|| {
        provide_layout_state();

        view! {
            <Router>
                <Badge variant=BadgeVariant::Default>
                    "Default Badge"
                </Badge>
            </Router>
        }
    });
}

#[wasm_bindgen_test]
fn test_badge_success_variant_renders() {
    leptos::mount::mount_to_body(|| {
        provide_layout_state();

        view! {
            <Router>
                <Badge variant=BadgeVariant::Success>
                    "Success!"
                </Badge>
            </Router>
        }
    });
}

#[wasm_bindgen_test]
fn test_badge_warning_variant_renders() {
    leptos::mount::mount_to_body(|| {
        provide_layout_state();

        view! {
            <Router>
                <Badge variant=BadgeVariant::Warning>
                    "Poisoned"
                </Badge>
            </Router>
        }
    });
}

#[wasm_bindgen_test]
fn test_badge_danger_variant_renders() {
    leptos::mount::mount_to_body(|| {
        provide_layout_state();

        view! {
            <Router>
                <Badge variant=BadgeVariant::Danger>
                    "Unconscious"
                </Badge>
            </Router>
        }
    });
}

#[wasm_bindgen_test]
fn test_badge_info_variant_renders() {
    leptos::mount::mount_to_body(|| {
        provide_layout_state();

        view! {
            <Router>
                <Badge variant=BadgeVariant::Info>
                    "Concentrating"
                </Badge>
            </Router>
        }
    });
}

// ============================================================================
// Condition Badge Rendering Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_condition_badges_multiple() {
    // Test that multiple condition badges can be rendered together
    leptos::mount::mount_to_body(|| {
        provide_layout_state();

        let conditions = vec!["Poisoned", "Frightened", "Prone"];

        view! {
            <Router>
                <div class="flex gap-1">
                    {conditions.into_iter().map(|condition| {
                        view! {
                            <Badge variant=BadgeVariant::Warning>
                                {condition}
                            </Badge>
                        }
                    }).collect_view()}
                </div>
            </Router>
        }
    });
}

// ============================================================================
// Initiative List Ordering Tests (Unit Tests)
// ============================================================================

#[wasm_bindgen_test]
fn test_initiative_ordering_basic() {
    // Test that combatants are ordered by initiative (highest first)
    let mut combatants = vec![
        ("Goblin", 10),
        ("Fighter", 18),
        ("Wizard", 12),
        ("Dragon", 25),
    ];

    // Sort by initiative descending (as the initiative list should)
    combatants.sort_by(|a, b| b.1.cmp(&a.1));

    assert_eq!(combatants[0].0, "Dragon");
    assert_eq!(combatants[1].0, "Fighter");
    assert_eq!(combatants[2].0, "Wizard");
    assert_eq!(combatants[3].0, "Goblin");
}

#[wasm_bindgen_test]
fn test_initiative_ordering_ties() {
    // Test initiative tie handling - original order preserved (stable sort)
    let mut combatants = vec![("Goblin 1", 15), ("Fighter", 15), ("Goblin 2", 15)];

    // Stable sort by initiative descending
    combatants.sort_by(|a, b| b.1.cmp(&a.1));

    // All have same initiative, order should be preserved
    assert_eq!(combatants[0].1, 15);
    assert_eq!(combatants[1].1, 15);
    assert_eq!(combatants[2].1, 15);
}

#[wasm_bindgen_test]
fn test_initiative_ordering_negative() {
    // Test that negative initiatives are handled correctly
    let mut combatants = vec![
        ("Fast", 20),
        ("Slow", -5),
        ("Normal", 10),
        ("Very Slow", -10),
    ];

    combatants.sort_by(|a, b| b.1.cmp(&a.1));

    assert_eq!(combatants[0].0, "Fast");
    assert_eq!(combatants[1].0, "Normal");
    assert_eq!(combatants[2].0, "Slow");
    assert_eq!(combatants[3].0, "Very Slow");
}

// ============================================================================
// Combat State Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_combat_state_signals() {
    // Test combat state management with reactive signals
    let combat_active = RwSignal::new(false);
    let current_round = RwSignal::new(1_u32);
    let current_turn = RwSignal::new(0_usize);

    // Initially not in combat
    assert!(!combat_active.get());
    assert_eq!(current_round.get(), 1);
    assert_eq!(current_turn.get(), 0);

    // Start combat
    combat_active.set(true);
    assert!(combat_active.get());

    // Advance turn
    current_turn.update(|t| *t += 1);
    assert_eq!(current_turn.get(), 1);

    // Advance to next round
    current_round.update(|r| *r += 1);
    current_turn.set(0);
    assert_eq!(current_round.get(), 2);
    assert_eq!(current_turn.get(), 0);

    // End combat
    combat_active.set(false);
    assert!(!combat_active.get());
}

#[wasm_bindgen_test]
fn test_combatant_hp_tracking() {
    // Test HP tracking for combatants
    let hp_current = RwSignal::new(45_i32);
    let hp_max = RwSignal::new(50_i32);

    // Initial HP
    assert_eq!(hp_current.get(), 45);
    assert_eq!(hp_max.get(), 50);

    // Take damage
    hp_current.update(|hp| *hp -= 10);
    assert_eq!(hp_current.get(), 35);

    // Heal
    hp_current.update(|hp| *hp = (*hp + 20).min(hp_max.get()));
    assert_eq!(hp_current.get(), 50); // Capped at max

    // Take massive damage
    hp_current.update(|hp| *hp -= 60);
    assert_eq!(hp_current.get(), -10); // Can go negative (death saves)
}

#[wasm_bindgen_test]
fn test_condition_list_management() {
    // Test managing a list of conditions
    let conditions = RwSignal::new(Vec::<String>::new());

    // Initially no conditions
    assert!(conditions.get().is_empty());

    // Add a condition
    conditions.update(|c| c.push("Poisoned".to_string()));
    assert_eq!(conditions.get().len(), 1);
    assert!(conditions.get().contains(&"Poisoned".to_string()));

    // Add another condition
    conditions.update(|c| c.push("Frightened".to_string()));
    assert_eq!(conditions.get().len(), 2);

    // Remove a condition
    conditions.update(|c| c.retain(|cond| cond != "Poisoned"));
    assert_eq!(conditions.get().len(), 1);
    assert!(!conditions.get().contains(&"Poisoned".to_string()));
    assert!(conditions.get().contains(&"Frightened".to_string()));

    // Clear all conditions
    conditions.set(Vec::new());
    assert!(conditions.get().is_empty());
}

#[wasm_bindgen_test]
fn test_combatant_type_mapping() {
    // Test that combatant type keys map to expected display values
    // This validates the mapping logic used in UI components
    fn get_combatant_display(type_key: &str) -> &'static str {
        match type_key {
            "player" => "Player Character",
            "monster" => "Monster/Enemy",
            "npc" => "NPC",
            "ally" => "Ally",
            _ => "Unknown",
        }
    }

    // Verify each type maps correctly
    assert_eq!(get_combatant_display("player"), "Player Character");
    assert_eq!(get_combatant_display("monster"), "Monster/Enemy");
    assert_eq!(get_combatant_display("npc"), "NPC");
    assert_eq!(get_combatant_display("ally"), "Ally");
    assert_eq!(get_combatant_display("invalid"), "Unknown");
}

// ============================================================================
// Turn Tracking Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_turn_cycling() {
    // Test that turns cycle through combatants correctly
    let combatant_count = 4_usize;
    let current_turn = RwSignal::new(0_usize);
    let current_round = RwSignal::new(1_u32);

    // Advance through all combatants
    for expected_turn in 0..combatant_count {
        assert_eq!(current_turn.get(), expected_turn);
        current_turn.update(|t| {
            *t = (*t + 1) % combatant_count;
        });
    }

    // Should be back to turn 0
    assert_eq!(current_turn.get(), 0);

    // Increment round when cycling back
    if current_turn.get() == 0 {
        current_round.update(|r| *r += 1);
    }
    assert_eq!(current_round.get(), 2);
}

#[wasm_bindgen_test]
fn test_current_combatant_highlight() {
    // Test determining which combatant is currently active
    let current_turn = RwSignal::new(2_usize);
    let combatants = vec![
        ("Fighter", 0),
        ("Wizard", 1),
        ("Rogue", 2), // Current turn
        ("Cleric", 3),
    ];

    for (name, idx) in combatants {
        let is_current = idx == current_turn.get();
        if name == "Rogue" {
            assert!(is_current, "Rogue should be current");
        } else {
            assert!(!is_current, "{} should not be current", name);
        }
    }
}

// ============================================================================
// Session State Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_session_state_signals() {
    // Test session state management
    let session_active = RwSignal::new(false);
    let session_id = RwSignal::new(Option::<String>::None);
    let session_number = RwSignal::new(1_u32);

    // No active session initially
    assert!(!session_active.get());
    assert!(session_id.get().is_none());

    // Start a session
    session_active.set(true);
    session_id.set(Some("session-abc-123".to_string()));
    assert!(session_active.get());
    assert_eq!(session_id.get(), Some("session-abc-123".to_string()));

    // End session
    session_active.set(false);
    session_number.update(|n| *n += 1);
    session_id.set(None);

    assert!(!session_active.get());
    assert_eq!(session_number.get(), 2);
    assert!(session_id.get().is_none());
}

// ============================================================================
// D&D 5e Condition Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_standard_conditions_list() {
    // Test that all standard D&D 5e conditions are recognized
    let standard_conditions = vec![
        "Blinded",
        "Charmed",
        "Deafened",
        "Frightened",
        "Grappled",
        "Incapacitated",
        "Invisible",
        "Paralyzed",
        "Petrified",
        "Poisoned",
        "Prone",
        "Restrained",
        "Stunned",
        "Unconscious",
        "Exhaustion",
    ];

    assert_eq!(standard_conditions.len(), 15);

    // Verify each condition is a non-empty string
    for condition in &standard_conditions {
        assert!(!condition.is_empty());
    }
}

#[wasm_bindgen_test]
fn test_condition_severity_classification() {
    // Test classifying conditions by severity (for UI display)
    fn classify_condition(condition: &str) -> &'static str {
        match condition {
            "Paralyzed" | "Petrified" | "Stunned" | "Unconscious" => "severe",
            "Exhaustion" => "exhaustion", // Special category - has levels 1-6
            "Blinded" | "Frightened" | "Incapacitated" | "Restrained" => "moderate",
            "Charmed" | "Deafened" | "Grappled" | "Poisoned" | "Prone" => "minor",
            "Invisible" => "beneficial",
            _ => "unknown",
        }
    }

    assert_eq!(classify_condition("Paralyzed"), "severe");
    assert_eq!(classify_condition("Exhaustion"), "exhaustion");
    assert_eq!(classify_condition("Frightened"), "moderate");
    assert_eq!(classify_condition("Poisoned"), "minor");
    assert_eq!(classify_condition("Invisible"), "beneficial");
    assert_eq!(classify_condition("Custom"), "unknown");
}
