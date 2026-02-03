//! Game Mechanics Service Tests
//!
//! Tests for D&D 5e standard conditions and their descriptions.

use ttrpg_assistant_frontend::services::game_mechanics::{
    get_condition_description, STANDARD_CONDITIONS,
};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ============================================================================
// STANDARD_CONDITIONS Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_standard_conditions_count() {
    // D&D 5e has exactly 15 standard conditions
    assert_eq!(STANDARD_CONDITIONS.len(), 15);
}

#[wasm_bindgen_test]
fn test_standard_conditions_contains_all_dnd_conditions() {
    let expected = vec![
        "Blinded",
        "Charmed",
        "Deafened",
        "Exhaustion",
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
    ];

    for condition in expected {
        assert!(
            STANDARD_CONDITIONS.contains(&condition),
            "Missing condition: {}",
            condition
        );
    }
}

#[wasm_bindgen_test]
fn test_standard_conditions_alphabetical_order() {
    let mut sorted = STANDARD_CONDITIONS.to_vec();
    sorted.sort();
    assert_eq!(
        STANDARD_CONDITIONS.to_vec(),
        sorted,
        "Conditions should be alphabetically sorted"
    );
}

// ============================================================================
// get_condition_description Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_blinded_description() {
    let desc = get_condition_description("Blinded");
    assert!(desc.is_some());
    assert!(desc.unwrap().contains("Can't see"));
}

#[wasm_bindgen_test]
fn test_charmed_description() {
    let desc = get_condition_description("Charmed");
    assert!(desc.is_some());
    assert!(desc.unwrap().contains("charmer"));
}

#[wasm_bindgen_test]
fn test_deafened_description() {
    let desc = get_condition_description("Deafened");
    assert!(desc.is_some());
    assert!(desc.unwrap().contains("hear"));
}

#[wasm_bindgen_test]
fn test_exhaustion_description() {
    let desc = get_condition_description("Exhaustion");
    assert!(desc.is_some());
    assert!(desc.unwrap().contains("level"));
}

#[wasm_bindgen_test]
fn test_frightened_description() {
    let desc = get_condition_description("Frightened");
    assert!(desc.is_some());
    assert!(desc.unwrap().contains("Disadvantage"));
}

#[wasm_bindgen_test]
fn test_grappled_description() {
    let desc = get_condition_description("Grappled");
    assert!(desc.is_some());
    assert!(desc.unwrap().contains("Speed 0"));
}

#[wasm_bindgen_test]
fn test_incapacitated_description() {
    let desc = get_condition_description("Incapacitated");
    assert!(desc.is_some());
    assert!(desc.unwrap().contains("actions"));
}

#[wasm_bindgen_test]
fn test_invisible_description() {
    let desc = get_condition_description("Invisible");
    assert!(desc.is_some());
    assert!(desc.unwrap().contains("Can't be seen"));
}

#[wasm_bindgen_test]
fn test_paralyzed_description() {
    let desc = get_condition_description("Paralyzed");
    assert!(desc.is_some());
    assert!(desc.unwrap().contains("Incapacitated"));
    assert!(desc.unwrap().contains("STR/DEX"));
}

#[wasm_bindgen_test]
fn test_petrified_description() {
    let desc = get_condition_description("Petrified");
    assert!(desc.is_some());
    assert!(desc.unwrap().contains("stone"));
}

#[wasm_bindgen_test]
fn test_poisoned_description() {
    let desc = get_condition_description("Poisoned");
    assert!(desc.is_some());
    assert!(desc.unwrap().contains("Disadvantage"));
}

#[wasm_bindgen_test]
fn test_prone_description() {
    let desc = get_condition_description("Prone");
    assert!(desc.is_some());
    assert!(desc.unwrap().contains("crawl"));
}

#[wasm_bindgen_test]
fn test_restrained_description() {
    let desc = get_condition_description("Restrained");
    assert!(desc.is_some());
    assert!(desc.unwrap().contains("Speed 0"));
}

#[wasm_bindgen_test]
fn test_stunned_description() {
    let desc = get_condition_description("Stunned");
    assert!(desc.is_some());
    assert!(desc.unwrap().contains("Incapacitated"));
}

#[wasm_bindgen_test]
fn test_unconscious_description() {
    let desc = get_condition_description("Unconscious");
    assert!(desc.is_some());
    assert!(desc.unwrap().contains("prone"));
}

// ============================================================================
// Case Sensitivity Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_lowercase_condition_lookup() {
    // Function handles case-insensitive lookup
    assert!(get_condition_description("blinded").is_some());
    assert!(get_condition_description("charmed").is_some());
    assert!(get_condition_description("unconscious").is_some());
}

#[wasm_bindgen_test]
fn test_uppercase_condition_lookup() {
    assert!(get_condition_description("BLINDED").is_some());
    assert!(get_condition_description("CHARMED").is_some());
    assert!(get_condition_description("UNCONSCIOUS").is_some());
}

#[wasm_bindgen_test]
fn test_mixed_case_condition_lookup() {
    assert!(get_condition_description("bLiNdEd").is_some());
    assert!(get_condition_description("ChArMeD").is_some());
}

// ============================================================================
// Edge Cases
// ============================================================================

#[wasm_bindgen_test]
fn test_unknown_condition_returns_none() {
    assert!(get_condition_description("Flying").is_none());
    assert!(get_condition_description("Blessed").is_none());
    assert!(get_condition_description("Hasted").is_none());
}

#[wasm_bindgen_test]
fn test_empty_string_returns_none() {
    assert!(get_condition_description("").is_none());
}

#[wasm_bindgen_test]
fn test_whitespace_returns_none() {
    assert!(get_condition_description("   ").is_none());
    assert!(get_condition_description("\t").is_none());
}

#[wasm_bindgen_test]
fn test_partial_condition_name_returns_none() {
    // "Blind" is not "Blinded"
    assert!(get_condition_description("Blind").is_none());
    assert!(get_condition_description("Charm").is_none());
}

#[wasm_bindgen_test]
fn test_condition_with_extra_spaces_returns_none() {
    assert!(get_condition_description(" Blinded").is_none());
    assert!(get_condition_description("Blinded ").is_none());
    assert!(get_condition_description(" Blinded ").is_none());
}

// ============================================================================
// All Conditions Have Descriptions Test
// ============================================================================

#[wasm_bindgen_test]
fn test_all_standard_conditions_have_descriptions() {
    for condition in STANDARD_CONDITIONS {
        let desc = get_condition_description(condition);
        assert!(
            desc.is_some(),
            "Condition '{}' should have a description",
            condition
        );
        assert!(
            !desc.unwrap().is_empty(),
            "Condition '{}' should have non-empty description",
            condition
        );
    }
}

#[wasm_bindgen_test]
fn test_descriptions_are_concise() {
    // Descriptions should be summaries, not full rules text
    for condition in STANDARD_CONDITIONS {
        if let Some(desc) = get_condition_description(condition) {
            assert!(
                desc.len() < 200,
                "Condition '{}' description too long ({}): {}",
                condition,
                desc.len(),
                desc
            );
        }
    }
}
