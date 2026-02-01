//! Wizard State Service Tests
//!
//! Tests for campaign wizard step navigation, pacing options, experience levels,
//! arc templates, party roles, and narrative styles.

use wasm_bindgen_test::*;
use ttrpg_assistant_frontend::services::wizard_state::{
    WizardStep, CampaignPacing, ExperienceLevel, ArcTemplate,
    PartyRole, NarrativeStyle,
};

wasm_bindgen_test_configure!(run_in_browser);

// ============================================================================
// WizardStep Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_wizard_step_default() {
    assert_eq!(WizardStep::default(), WizardStep::Basics);
}

#[wasm_bindgen_test]
fn test_wizard_step_all_returns_correct_count() {
    let steps = WizardStep::all();
    assert_eq!(steps.len(), 8);
}

#[wasm_bindgen_test]
fn test_wizard_step_all_order() {
    let steps = WizardStep::all();
    assert_eq!(steps[0], WizardStep::Basics);
    assert_eq!(steps[1], WizardStep::Intent);
    assert_eq!(steps[2], WizardStep::Scope);
    assert_eq!(steps[3], WizardStep::Players);
    assert_eq!(steps[4], WizardStep::PartyComposition);
    assert_eq!(steps[5], WizardStep::ArcStructure);
    assert_eq!(steps[6], WizardStep::InitialContent);
    assert_eq!(steps[7], WizardStep::Review);
}

#[wasm_bindgen_test]
fn test_wizard_step_labels() {
    assert_eq!(WizardStep::Basics.label(), "Basics");
    assert_eq!(WizardStep::Intent.label(), "Creative Vision");
    assert_eq!(WizardStep::Scope.label(), "Campaign Scope");
    assert_eq!(WizardStep::Players.label(), "Players");
    assert_eq!(WizardStep::PartyComposition.label(), "Party Composition");
    assert_eq!(WizardStep::ArcStructure.label(), "Story Arc");
    assert_eq!(WizardStep::InitialContent.label(), "Initial Content");
    assert_eq!(WizardStep::Review.label(), "Review");
}

#[wasm_bindgen_test]
fn test_wizard_step_descriptions() {
    // All steps should have non-empty descriptions
    for step in WizardStep::all() {
        let desc = step.description();
        assert!(!desc.is_empty(), "Step {:?} should have description", step);
    }
}

#[wasm_bindgen_test]
fn test_wizard_step_indices() {
    assert_eq!(WizardStep::Basics.index(), 0);
    assert_eq!(WizardStep::Intent.index(), 1);
    assert_eq!(WizardStep::Scope.index(), 2);
    assert_eq!(WizardStep::Players.index(), 3);
    assert_eq!(WizardStep::PartyComposition.index(), 4);
    assert_eq!(WizardStep::ArcStructure.index(), 5);
    assert_eq!(WizardStep::InitialContent.index(), 6);
    assert_eq!(WizardStep::Review.index(), 7);
}

#[wasm_bindgen_test]
fn test_wizard_step_indices_sequential() {
    let steps = WizardStep::all();
    for (i, step) in steps.iter().enumerate() {
        assert_eq!(step.index(), i, "Step {:?} index mismatch", step);
    }
}

#[wasm_bindgen_test]
fn test_wizard_step_next_navigation() {
    assert_eq!(WizardStep::Basics.next(), Some(WizardStep::Intent));
    assert_eq!(WizardStep::Intent.next(), Some(WizardStep::Scope));
    assert_eq!(WizardStep::Scope.next(), Some(WizardStep::Players));
    assert_eq!(WizardStep::Players.next(), Some(WizardStep::PartyComposition));
    assert_eq!(WizardStep::PartyComposition.next(), Some(WizardStep::ArcStructure));
    assert_eq!(WizardStep::ArcStructure.next(), Some(WizardStep::InitialContent));
    assert_eq!(WizardStep::InitialContent.next(), Some(WizardStep::Review));
    assert_eq!(WizardStep::Review.next(), None);
}

#[wasm_bindgen_test]
fn test_wizard_step_previous_navigation() {
    assert_eq!(WizardStep::Basics.previous(), None);
    assert_eq!(WizardStep::Intent.previous(), Some(WizardStep::Basics));
    assert_eq!(WizardStep::Scope.previous(), Some(WizardStep::Intent));
    assert_eq!(WizardStep::Players.previous(), Some(WizardStep::Scope));
    assert_eq!(WizardStep::PartyComposition.previous(), Some(WizardStep::Players));
    assert_eq!(WizardStep::ArcStructure.previous(), Some(WizardStep::PartyComposition));
    assert_eq!(WizardStep::InitialContent.previous(), Some(WizardStep::ArcStructure));
    assert_eq!(WizardStep::Review.previous(), Some(WizardStep::InitialContent));
}

#[wasm_bindgen_test]
fn test_wizard_step_navigation_roundtrip() {
    // next then previous should return to original (except at boundaries)
    for step in WizardStep::all() {
        if let Some(next) = step.next() {
            assert_eq!(next.previous(), Some(step), "Roundtrip failed for {:?}", step);
        }
    }
}

#[wasm_bindgen_test]
fn test_wizard_step_skippable() {
    // Required steps
    assert!(!WizardStep::Basics.is_skippable());
    assert!(!WizardStep::Scope.is_skippable());
    assert!(!WizardStep::Players.is_skippable());
    assert!(!WizardStep::Review.is_skippable());

    // Optional steps
    assert!(WizardStep::Intent.is_skippable());
    assert!(WizardStep::PartyComposition.is_skippable());
    assert!(WizardStep::ArcStructure.is_skippable());
    assert!(WizardStep::InitialContent.is_skippable());
}

#[wasm_bindgen_test]
fn test_wizard_step_equality() {
    assert_eq!(WizardStep::Basics, WizardStep::Basics);
    assert_ne!(WizardStep::Basics, WizardStep::Review);
}

#[wasm_bindgen_test]
fn test_wizard_step_clone() {
    let step = WizardStep::Intent;
    let cloned = step.clone();
    assert_eq!(step, cloned);
}

#[wasm_bindgen_test]
fn test_wizard_step_copy() {
    let step = WizardStep::Scope;
    let copied: WizardStep = step;
    assert_eq!(step, copied);
}

// ============================================================================
// CampaignPacing Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_campaign_pacing_default() {
    assert_eq!(CampaignPacing::default(), CampaignPacing::Balanced);
}

#[wasm_bindgen_test]
fn test_campaign_pacing_all() {
    let pacing = CampaignPacing::all();
    assert_eq!(pacing.len(), 4);
    assert!(pacing.contains(&CampaignPacing::Fast));
    assert!(pacing.contains(&CampaignPacing::Balanced));
    assert!(pacing.contains(&CampaignPacing::Slow));
    assert!(pacing.contains(&CampaignPacing::Sandbox));
}

#[wasm_bindgen_test]
fn test_campaign_pacing_labels() {
    assert_eq!(CampaignPacing::Fast.label(), "Fast-Paced");
    assert_eq!(CampaignPacing::Balanced.label(), "Balanced");
    assert_eq!(CampaignPacing::Slow.label(), "Slow & Deliberate");
    assert_eq!(CampaignPacing::Sandbox.label(), "Sandbox");
}

#[wasm_bindgen_test]
fn test_campaign_pacing_descriptions() {
    for pacing in CampaignPacing::all() {
        let desc = pacing.description();
        assert!(!desc.is_empty(), "Pacing {:?} should have description", pacing);
    }
}

#[wasm_bindgen_test]
fn test_campaign_pacing_equality() {
    assert_eq!(CampaignPacing::Fast, CampaignPacing::Fast);
    assert_ne!(CampaignPacing::Fast, CampaignPacing::Slow);
}

// ============================================================================
// ExperienceLevel Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_experience_level_default() {
    assert_eq!(ExperienceLevel::default(), ExperienceLevel::Mixed);
}

#[wasm_bindgen_test]
fn test_experience_level_all() {
    let levels = ExperienceLevel::all();
    assert_eq!(levels.len(), 4);
    assert!(levels.contains(&ExperienceLevel::Beginner));
    assert!(levels.contains(&ExperienceLevel::Intermediate));
    assert!(levels.contains(&ExperienceLevel::Experienced));
    assert!(levels.contains(&ExperienceLevel::Mixed));
}

#[wasm_bindgen_test]
fn test_experience_level_labels() {
    assert_eq!(ExperienceLevel::Beginner.label(), "New Players");
    assert_eq!(ExperienceLevel::Intermediate.label(), "Some Experience");
    assert_eq!(ExperienceLevel::Experienced.label(), "Veterans");
    assert_eq!(ExperienceLevel::Mixed.label(), "Mixed Group");
}

#[wasm_bindgen_test]
fn test_experience_level_equality() {
    assert_eq!(ExperienceLevel::Beginner, ExperienceLevel::Beginner);
    assert_ne!(ExperienceLevel::Beginner, ExperienceLevel::Experienced);
}

// ============================================================================
// ArcTemplate Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_arc_template_default() {
    assert_eq!(ArcTemplate::default(), ArcTemplate::HerosJourney);
}

#[wasm_bindgen_test]
fn test_arc_template_all() {
    let templates = ArcTemplate::all();
    assert_eq!(templates.len(), 8);
    assert!(templates.contains(&ArcTemplate::HerosJourney));
    assert!(templates.contains(&ArcTemplate::ThreeAct));
    assert!(templates.contains(&ArcTemplate::FiveAct));
    assert!(templates.contains(&ArcTemplate::Mystery));
    assert!(templates.contains(&ArcTemplate::PoliticalIntrigue));
    assert!(templates.contains(&ArcTemplate::DungeonDelve));
    assert!(templates.contains(&ArcTemplate::Sandbox));
    assert!(templates.contains(&ArcTemplate::Custom));
}

#[wasm_bindgen_test]
fn test_arc_template_labels() {
    assert_eq!(ArcTemplate::HerosJourney.label(), "Hero's Journey");
    assert_eq!(ArcTemplate::ThreeAct.label(), "Three-Act Structure");
    assert_eq!(ArcTemplate::FiveAct.label(), "Five-Act Structure");
    assert_eq!(ArcTemplate::Mystery.label(), "Mystery/Investigation");
    assert_eq!(ArcTemplate::PoliticalIntrigue.label(), "Political Intrigue");
    assert_eq!(ArcTemplate::DungeonDelve.label(), "Dungeon Delve");
    assert_eq!(ArcTemplate::Sandbox.label(), "Sandbox");
    assert_eq!(ArcTemplate::Custom.label(), "Custom");
}

#[wasm_bindgen_test]
fn test_arc_template_descriptions() {
    for template in ArcTemplate::all() {
        let desc = template.description();
        assert!(!desc.is_empty(), "Template {:?} should have description", template);
    }
}

#[wasm_bindgen_test]
fn test_arc_template_equality() {
    assert_eq!(ArcTemplate::HerosJourney, ArcTemplate::HerosJourney);
    assert_ne!(ArcTemplate::HerosJourney, ArcTemplate::Mystery);
}

// ============================================================================
// PartyRole Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_party_role_all() {
    let roles = PartyRole::all();
    assert_eq!(roles.len(), 8);
    assert!(roles.contains(&PartyRole::Tank));
    assert!(roles.contains(&PartyRole::Healer));
    assert!(roles.contains(&PartyRole::DamageDealer));
    assert!(roles.contains(&PartyRole::Support));
    assert!(roles.contains(&PartyRole::Controller));
    assert!(roles.contains(&PartyRole::Utility));
    assert!(roles.contains(&PartyRole::Face));
    assert!(roles.contains(&PartyRole::Scout));
}

#[wasm_bindgen_test]
fn test_party_role_labels() {
    assert_eq!(PartyRole::Tank.label(), "Tank");
    assert_eq!(PartyRole::Healer.label(), "Healer");
    assert_eq!(PartyRole::DamageDealer.label(), "Damage Dealer");
    assert_eq!(PartyRole::Support.label(), "Support");
    assert_eq!(PartyRole::Controller.label(), "Controller");
    assert_eq!(PartyRole::Utility.label(), "Utility");
    assert_eq!(PartyRole::Face.label(), "Face");
    assert_eq!(PartyRole::Scout.label(), "Scout");
}

#[wasm_bindgen_test]
fn test_party_role_equality() {
    assert_eq!(PartyRole::Tank, PartyRole::Tank);
    assert_ne!(PartyRole::Tank, PartyRole::Healer);
}

#[wasm_bindgen_test]
fn test_party_role_hash() {
    // PartyRole derives Hash, so it can be used in HashSets
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(PartyRole::Tank);
    set.insert(PartyRole::Healer);
    set.insert(PartyRole::Tank); // Duplicate

    assert_eq!(set.len(), 2);
    assert!(set.contains(&PartyRole::Tank));
    assert!(set.contains(&PartyRole::Healer));
}

// ============================================================================
// NarrativeStyle Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_narrative_style_default() {
    assert_eq!(NarrativeStyle::default(), NarrativeStyle::Linear);
}

#[wasm_bindgen_test]
fn test_narrative_style_all() {
    let styles = NarrativeStyle::all();
    assert_eq!(styles.len(), 4);
    assert!(styles.contains(&NarrativeStyle::Linear));
    assert!(styles.contains(&NarrativeStyle::Branching));
    assert!(styles.contains(&NarrativeStyle::Sandbox));
    assert!(styles.contains(&NarrativeStyle::Episodic));
}

#[wasm_bindgen_test]
fn test_narrative_style_labels() {
    assert_eq!(NarrativeStyle::Linear.label(), "Linear");
    assert_eq!(NarrativeStyle::Branching.label(), "Branching");
    assert_eq!(NarrativeStyle::Sandbox.label(), "Sandbox");
    assert_eq!(NarrativeStyle::Episodic.label(), "Episodic");
}

#[wasm_bindgen_test]
fn test_narrative_style_equality() {
    assert_eq!(NarrativeStyle::Linear, NarrativeStyle::Linear);
    assert_ne!(NarrativeStyle::Linear, NarrativeStyle::Branching);
}

// ============================================================================
// Integration-style Tests
// ============================================================================

#[wasm_bindgen_test]
fn test_wizard_full_navigation_forward() {
    let mut step = WizardStep::Basics;
    let mut count = 0;

    while let Some(next) = step.next() {
        step = next;
        count += 1;
    }

    assert_eq!(step, WizardStep::Review);
    assert_eq!(count, 7); // 8 steps total, 7 transitions
}

#[wasm_bindgen_test]
fn test_wizard_full_navigation_backward() {
    let mut step = WizardStep::Review;
    let mut count = 0;

    while let Some(prev) = step.previous() {
        step = prev;
        count += 1;
    }

    assert_eq!(step, WizardStep::Basics);
    assert_eq!(count, 7);
}

/// Helper to check for duplicates without requiring Hash
fn has_no_duplicates<T: PartialEq>(items: &[T]) -> bool {
    for (i, item) in items.iter().enumerate() {
        for other in items.iter().skip(i + 1) {
            if item == other {
                return false;
            }
        }
    }
    true
}

#[wasm_bindgen_test]
fn test_wizard_step_all_unique() {
    let steps = WizardStep::all();
    assert!(has_no_duplicates(&steps), "WizardStep::all() has duplicates");
}

#[wasm_bindgen_test]
fn test_campaign_pacing_all_unique() {
    let pacing = CampaignPacing::all();
    assert!(has_no_duplicates(&pacing), "CampaignPacing::all() has duplicates");
}

#[wasm_bindgen_test]
fn test_experience_level_all_unique() {
    let levels = ExperienceLevel::all();
    assert!(has_no_duplicates(&levels), "ExperienceLevel::all() has duplicates");
}

#[wasm_bindgen_test]
fn test_arc_template_all_unique() {
    let templates = ArcTemplate::all();
    assert!(has_no_duplicates(&templates), "ArcTemplate::all() has duplicates");
}

#[wasm_bindgen_test]
fn test_party_role_all_unique() {
    // PartyRole derives Hash, so we can use HashSet
    use std::collections::HashSet;
    let roles = PartyRole::all();
    let unique: HashSet<_> = roles.iter().collect();
    assert_eq!(roles.len(), unique.len(), "PartyRole::all() has duplicates");
}

#[wasm_bindgen_test]
fn test_narrative_style_all_unique() {
    let styles = NarrativeStyle::all();
    assert!(has_no_duplicates(&styles), "NarrativeStyle::all() has duplicates");
}
