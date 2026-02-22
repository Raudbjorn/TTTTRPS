//! Call of Cthulhu Character Generator Unit Tests
//!
//! Tests for Call of Cthulhu 7th Edition character generation including:
//! - Occupation selection
//! - Skill point allocation
//! - Sanity and luck calculation
//! - Backstory generation
//! - Characteristic rolling

use crate::core::character_gen::{
    systems::coc::CallOfCthulhuGenerator,
    AttributeValue,
    GameSystem, GenerationOptions, SystemGenerator, TraitType,
};

// ============================================================================
// Test Helpers
// ============================================================================

fn create_test_generator() -> CallOfCthulhuGenerator {
    CallOfCthulhuGenerator::new()
}

fn create_default_options() -> GenerationOptions {
    GenerationOptions {
        system: Some("coc".to_string()),
        ..Default::default()
    }
}

// ============================================================================
// Character Creation Tests
// ============================================================================

#[cfg(test)]
mod character_creation {
    use super::*;

    #[test]
    fn test_create_investigator_with_defaults() {
        let generator = create_test_generator();
        let options = create_default_options();

        let result = generator.generate(&options);
        assert!(result.is_ok());

        let character = result.unwrap();
        assert_eq!(character.system, GameSystem::CallOfCthulhu);
        assert!(!character.name.is_empty());
        // CoC doesn't use traditional levels
        assert_eq!(character.level, 1);
        // CoC doesn't use races
        assert!(character.race.is_none());
        // Class is "Investigator"
        assert_eq!(character.class.as_deref(), Some("Investigator"));
    }

    #[test]
    fn test_create_investigator_with_custom_name() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            name: Some("Harvey Walters".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        assert_eq!(character.name, "Harvey Walters");
    }

    #[test]
    fn test_create_investigator_with_occupation() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            class: Some("Professor".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        // Occupation is stored in concept or background
        assert!(
            character.concept == "Professor" ||
            character.background.occupation == Some("Professor".to_string())
        );
    }

    #[test]
    fn test_character_has_valid_uuid() {
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();

        assert_eq!(character.id.len(), 36);
        assert!(character.id.contains('-'));
    }

    #[test]
    fn test_multiple_investigators_have_unique_ids() {
        let generator = create_test_generator();
        let options = create_default_options();

        let char1 = generator.generate(&options).unwrap();
        let char2 = generator.generate(&options).unwrap();

        assert_ne!(char1.id, char2.id);
    }

    #[test]
    fn test_investigator_name_format() {
        let generator = create_test_generator();
        let options = create_default_options();

        // Generate multiple characters and check name format
        for _ in 0..5 {
            let character = generator.generate(&options).unwrap();
            // Names should have first and last name (contain a space)
            assert!(
                character.name.contains(' '),
                "Name '{}' should be first + last name",
                character.name
            );
        }
    }
}

// ============================================================================
// Characteristic Tests (CoC uses different attributes)
// ============================================================================

#[cfg(test)]
mod characteristic_tests {
    use super::*;

    #[test]
    fn test_all_characteristics_present() {
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();
        let attrs = &character.attributes;

        // CoC 7e characteristics
        let expected_characteristics = [
            "STR", "CON", "SIZ", "DEX", "APP", "INT", "POW", "EDU", "Luck"
        ];

        for char in expected_characteristics {
            assert!(
                attrs.contains_key(char),
                "Missing characteristic: {}",
                char
            );
        }
    }

    #[test]
    fn test_characteristic_count() {
        let generator = create_test_generator();
        let attrs = generator.attribute_names();

        // CoC has 9 characteristics including Luck
        assert_eq!(attrs.len(), 9);
    }

    #[test]
    fn test_3d6x5_characteristics_in_range() {
        let generator = create_test_generator();

        // Test multiple times to verify randomness is within bounds
        for _ in 0..20 {
            let options = create_default_options();
            let character = generator.generate(&options).unwrap();

            // STR, CON, DEX, APP, POW use 3d6*5 (range: 15-90)
            let three_d6_chars = ["STR", "CON", "DEX", "APP", "POW"];
            for char_name in three_d6_chars {
                let value = character.attributes.get(char_name)
                    .expect(&format!("Missing {}", char_name))
                    .base;

                assert!(
                    value >= 15 && value <= 90,
                    "{} = {} should be in range [15, 90] (3d6*5)",
                    char_name, value
                );
            }
        }
    }

    #[test]
    fn test_2d6plus6x5_characteristics_in_range() {
        let generator = create_test_generator();

        for _ in 0..20 {
            let options = create_default_options();
            let character = generator.generate(&options).unwrap();

            // SIZ, INT, EDU use (2d6+6)*5 (range: 40-90)
            let two_d6_plus_6_chars = ["SIZ", "INT", "EDU"];
            for char_name in two_d6_plus_6_chars {
                let value = character.attributes.get(char_name)
                    .expect(&format!("Missing {}", char_name))
                    .base;

                assert!(
                    value >= 40 && value <= 90,
                    "{} = {} should be in range [40, 90] ((2d6+6)*5)",
                    char_name, value
                );
            }
        }
    }

    #[test]
    fn test_luck_characteristic_in_range() {
        let generator = create_test_generator();

        for _ in 0..20 {
            let options = create_default_options();
            let character = generator.generate(&options).unwrap();

            let luck = character.attributes.get("Luck")
                .expect("Missing Luck")
                .base;

            // Luck is 3d6*5 (range: 15-90)
            assert!(
                luck >= 15 && luck <= 90,
                "Luck = {} should be in range [15, 90] (3d6*5)",
                luck
            );
        }
    }

    #[test]
    fn test_characteristics_are_multiples_of_5() {
        let generator = create_test_generator();

        for _ in 0..10 {
            let options = create_default_options();
            let character = generator.generate(&options).unwrap();

            for (name, attr) in &character.attributes {
                assert!(
                    attr.base % 5 == 0,
                    "{} = {} should be a multiple of 5",
                    name, attr.base
                );
            }
        }
    }

    #[test]
    fn test_attribute_value_raw_has_no_modifier() {
        // CoC uses AttributeValue::new_raw which doesn't calculate modifiers
        let attr = AttributeValue::new_raw(50);
        assert_eq!(attr.base, 50);
        assert_eq!(attr.modifier, 0);
        assert_eq!(attr.temp_bonus, 0);
    }
}

// ============================================================================
// Derived Statistics Tests
// ============================================================================

#[cfg(test)]
mod derived_stats_tests {
    use super::*;

    #[test]
    fn test_derived_stats_in_notes() {
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();

        // Derived stats should be in notes
        assert!(character.notes.contains("HP:"), "Notes should contain HP");
        assert!(character.notes.contains("Sanity:"), "Notes should contain Sanity");
        assert!(character.notes.contains("Magic Points:"), "Notes should contain Magic Points");
    }

    #[test]
    fn test_hp_calculation() {
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();

        let con = character.attributes.get("CON").unwrap().base;
        let siz = character.attributes.get("SIZ").unwrap().base;

        // HP = (CON + SIZ) / 10
        let expected_hp = (con + siz) / 10;

        assert!(
            character.notes.contains(&format!("HP: {}", expected_hp)),
            "Notes should contain calculated HP: {} (CON {} + SIZ {} / 10)",
            expected_hp, con, siz
        );
    }

    #[test]
    fn test_sanity_equals_pow() {
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();

        let pow = character.attributes.get("POW").unwrap().base;

        // Starting Sanity = POW
        assert!(
            character.notes.contains(&format!("Sanity: {}", pow)),
            "Sanity should equal POW ({})",
            pow
        );
    }

    #[test]
    fn test_magic_points_calculation() {
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();

        let pow = character.attributes.get("POW").unwrap().base;

        // Magic Points = POW / 5
        let expected_mp = pow / 5;

        assert!(
            character.notes.contains(&format!("Magic Points: {}", expected_mp)),
            "Magic Points should be POW/5: {} (POW {})",
            expected_mp, pow
        );
    }
}

// ============================================================================
// Occupation Tests
// ============================================================================

#[cfg(test)]
mod occupation_tests {
    use super::*;

    #[test]
    fn test_available_occupations() {
        let generator = create_test_generator();
        let occupations = generator.available_classes();

        let expected_occupations = [
            "Antiquarian", "Archaeologist", "Author", "Dilettante",
            "Doctor", "Journalist", "Lawyer", "Librarian",
            "Nurse", "Police Detective", "Private Investigator",
            "Professor", "Scientist",
        ];

        for occ in expected_occupations {
            assert!(
                occupations.iter().any(|o| o == occ),
                "Expected occupation '{}' not found",
                occ
            );
        }
    }

    #[test]
    fn test_occupation_count() {
        let generator = create_test_generator();
        let occupations = generator.available_classes();

        // CoC should have at least 15 occupations
        assert!(
            occupations.len() >= 15,
            "Should have at least 15 occupations, found {}",
            occupations.len()
        );
    }

    #[test]
    fn test_custom_occupation_creates_trait() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            class: Some("Private Investigator".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();

        let has_occupation_trait = character.traits.iter().any(|t| {
            t.name == "Private Investigator" && t.trait_type == TraitType::Class
        });

        assert!(has_occupation_trait, "Should have occupation as trait");
    }

    #[test]
    fn test_investigator_trait_always_present() {
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();

        let has_investigator_trait = character.traits.iter().any(|t| {
            t.name == "Investigator" && t.trait_type == TraitType::Background
        });

        assert!(has_investigator_trait, "Should always have Investigator trait");
    }

    #[test]
    fn test_random_occupation_when_not_specified() {
        let generator = create_test_generator();
        let available_occupations = generator.available_classes();

        // Generate multiple characters and verify occupations are from the list
        for _ in 0..10 {
            let options = create_default_options();
            let character = generator.generate(&options).unwrap();

            // Find occupation trait
            let occupation_trait = character.traits.iter()
                .find(|t| t.trait_type == TraitType::Class);

            assert!(occupation_trait.is_some(), "Should have occupation trait");

            let occupation = &occupation_trait.unwrap().name;
            assert!(
                available_occupations.contains(occupation),
                "Occupation '{}' should be in available list",
                occupation
            );
        }
    }
}

// ============================================================================
// Skill Tests
// ============================================================================

#[cfg(test)]
mod skill_tests {
    use super::*;

    #[test]
    fn test_all_coc_skills_present() {
        let generator = create_test_generator();
        let options = create_default_options();
        let character = generator.generate(&options).unwrap();

        let expected_skills = [
            "Accounting", "Anthropology", "Appraise", "Archaeology",
            "Art/Craft", "Charm", "Climb", "Credit Rating",
            "Cthulhu Mythos", "Disguise", "Dodge", "Drive Auto",
            "Electrical Repair", "Fast Talk", "Fighting (Brawl)",
            "Firearms (Handgun)", "Firearms (Rifle)", "First Aid",
            "History", "Intimidate", "Jump", "Law", "Library Use",
            "Listen", "Locksmith", "Mechanical Repair", "Medicine",
            "Natural World", "Navigate", "Occult", "Persuade",
            "Photography", "Pilot", "Psychology", "Psychoanalysis",
            "Ride", "Science", "Sleight of Hand", "Spot Hidden",
            "Stealth", "Survival", "Swim", "Throw", "Track",
        ];

        for skill in expected_skills {
            assert!(
                character.skills.contains_key(skill),
                "Missing skill: {}",
                skill
            );
        }
    }

    #[test]
    fn test_cthulhu_mythos_starts_at_zero() {
        let generator = create_test_generator();
        let options = create_default_options();
        let character = generator.generate(&options).unwrap();

        let mythos = character.skills.get("Cthulhu Mythos").unwrap();
        assert_eq!(*mythos, 0, "Cthulhu Mythos should always start at 0");
    }

    #[test]
    fn test_credit_rating_starts_at_zero() {
        let generator = create_test_generator();
        let options = create_default_options();
        let character = generator.generate(&options).unwrap();

        let credit = character.skills.get("Credit Rating").unwrap();
        assert_eq!(*credit, 0, "Credit Rating should start at 0 (set by occupation)");
    }

    #[test]
    fn test_combat_skills_have_base_values() {
        let generator = create_test_generator();
        let options = create_default_options();
        let character = generator.generate(&options).unwrap();

        // Fighting (Brawl) base is 25
        let brawl = character.skills.get("Fighting (Brawl)").unwrap();
        assert_eq!(*brawl, 25, "Fighting (Brawl) base should be 25");

        // Firearms (Handgun) base is 20
        let handgun = character.skills.get("Firearms (Handgun)").unwrap();
        assert_eq!(*handgun, 20, "Firearms (Handgun) base should be 20");

        // Firearms (Rifle) base is 25
        let rifle = character.skills.get("Firearms (Rifle)").unwrap();
        assert_eq!(*rifle, 25, "Firearms (Rifle) base should be 25");
    }

    #[test]
    fn test_physical_skills_have_base_values() {
        let generator = create_test_generator();
        let options = create_default_options();
        let character = generator.generate(&options).unwrap();

        // Climb base is 20
        let climb = character.skills.get("Climb").unwrap();
        assert_eq!(*climb, 20, "Climb base should be 20");

        // Jump base is 20
        let jump = character.skills.get("Jump").unwrap();
        assert_eq!(*jump, 20, "Jump base should be 20");

        // Swim base is 20
        let swim = character.skills.get("Swim").unwrap();
        assert_eq!(*swim, 20, "Swim base should be 20");
    }

    #[test]
    fn test_perception_skills_have_base_values() {
        let generator = create_test_generator();
        let options = create_default_options();
        let character = generator.generate(&options).unwrap();

        // Spot Hidden base is 25
        let spot = character.skills.get("Spot Hidden").unwrap();
        assert_eq!(*spot, 25, "Spot Hidden base should be 25");

        // Listen base is 20
        let listen = character.skills.get("Listen").unwrap();
        assert_eq!(*listen, 20, "Listen base should be 20");
    }

    #[test]
    fn test_research_skills_have_base_values() {
        let generator = create_test_generator();
        let options = create_default_options();
        let character = generator.generate(&options).unwrap();

        // Library Use base is 20
        let library = character.skills.get("Library Use").unwrap();
        assert_eq!(*library, 20, "Library Use base should be 20");
    }

    #[test]
    fn test_first_aid_base_value() {
        let generator = create_test_generator();
        let options = create_default_options();
        let character = generator.generate(&options).unwrap();

        // First Aid base is 30
        let first_aid = character.skills.get("First Aid").unwrap();
        assert_eq!(*first_aid, 30, "First Aid base should be 30");
    }

    #[test]
    fn test_social_skills_have_base_values() {
        let generator = create_test_generator();
        let options = create_default_options();
        let character = generator.generate(&options).unwrap();

        // Charm base is 15
        let charm = character.skills.get("Charm").unwrap();
        assert_eq!(*charm, 15, "Charm base should be 15");

        // Intimidate base is 15
        let intimidate = character.skills.get("Intimidate").unwrap();
        assert_eq!(*intimidate, 15, "Intimidate base should be 15");

        // Persuade base is 10
        let persuade = character.skills.get("Persuade").unwrap();
        assert_eq!(*persuade, 10, "Persuade base should be 10");
    }
}

// ============================================================================
// Equipment Tests
// ============================================================================

#[cfg(test)]
mod equipment_tests {
    use super::*;

    #[test]
    fn test_equipment_included_when_requested() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            include_equipment: true,
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        assert!(!character.equipment.is_empty());
    }

    #[test]
    fn test_no_equipment_when_not_requested() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            include_equipment: false,
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        assert!(character.equipment.is_empty());
    }

    #[test]
    fn test_common_investigator_gear() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("Professor"));

        let has_flashlight = equipment.iter().any(|e| e.name == "Flashlight");
        let has_notebook = equipment.iter().any(|e| e.name == "Notebook and Pen");
        let has_knife = equipment.iter().any(|e| e.name == "Pocket Knife");

        assert!(has_flashlight, "Should have Flashlight");
        assert!(has_notebook, "Should have Notebook and Pen");
        assert!(has_knife, "Should have Pocket Knife");
    }

    #[test]
    fn test_doctor_equipment() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("Doctor"));

        let has_medical_bag = equipment.iter().any(|e| e.name == "Medical Bag");
        assert!(has_medical_bag, "Doctor should have Medical Bag");

        // Medical bag should have First Aid bonus
        let medical_bag = equipment.iter().find(|e| e.name == "Medical Bag").unwrap();
        assert!(
            medical_bag.stats.values().any(|v| v.contains("First Aid")),
            "Medical Bag should boost First Aid"
        );
    }

    #[test]
    fn test_detective_equipment() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("Police Detective"));

        let has_revolver = equipment.iter().any(|e| e.name == ".38 Revolver");
        let has_handcuffs = equipment.iter().any(|e| e.name == "Handcuffs");

        assert!(has_revolver, "Police Detective should have .38 Revolver");
        assert!(has_handcuffs, "Police Detective should have Handcuffs");
    }

    #[test]
    fn test_private_investigator_equipment() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("Private Investigator"));

        let has_revolver = equipment.iter().any(|e| e.name == ".38 Revolver");
        assert!(has_revolver, "Private Investigator should have .38 Revolver");
    }

    #[test]
    fn test_academic_equipment() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("Professor"));

        let has_research_notes = equipment.iter().any(|e| e.name == "Research Notes");
        let has_magnifying_glass = equipment.iter().any(|e| e.name == "Magnifying Glass");

        assert!(has_research_notes, "Professor should have Research Notes");
        assert!(has_magnifying_glass, "Professor should have Magnifying Glass");
    }

    #[test]
    fn test_journalist_equipment() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("Journalist"));

        let has_camera = equipment.iter().any(|e| e.name == "Camera");
        let has_credentials = equipment.iter().any(|e| e.name == "Press Credentials");

        assert!(has_camera, "Journalist should have Camera");
        assert!(has_credentials, "Journalist should have Press Credentials");
    }

    #[test]
    fn test_revolver_has_damage_stats() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("Police Detective"));

        let revolver = equipment.iter().find(|e| e.name == ".38 Revolver").unwrap();

        assert!(revolver.stats.contains_key("Damage"), "Revolver should have Damage stat");
        assert!(revolver.stats.contains_key("Range"), "Revolver should have Range stat");
    }
}

// ============================================================================
// Background Tests
// ============================================================================

#[cfg(test)]
mod background_tests {
    use super::*;

    #[test]
    fn test_available_backgrounds() {
        let generator = create_test_generator();
        let backgrounds = generator.available_backgrounds();

        let expected_backgrounds = [
            "Wealthy", "Middle Class", "Poor",
            "Academic", "Criminal", "Military", "Religious",
        ];

        for bg in expected_backgrounds {
            assert!(
                backgrounds.iter().any(|b| b == bg),
                "Expected background '{}' not found",
                bg
            );
        }
    }

    #[test]
    fn test_default_origin() {
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();
        assert_eq!(character.background.origin, "United States");
    }

    #[test]
    fn test_background_has_connections() {
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();
        assert!(!character.background.connections.is_empty(), "Should have connections");
    }

    #[test]
    fn test_background_has_secrets() {
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();
        assert!(!character.background.secrets.is_empty(), "Should have secrets");
    }

    #[test]
    fn test_background_motivation() {
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();
        assert!(!character.background.motivation.is_empty(), "Should have motivation");
    }
}

// ============================================================================
// Sanity and Luck Specific Tests
// ============================================================================

#[cfg(test)]
mod sanity_luck_tests {
    use super::*;

    #[test]
    fn test_sanity_value_calculation() {
        let generator = create_test_generator();

        // Test multiple times to verify calculation
        for _ in 0..10 {
            let options = create_default_options();
            let character = generator.generate(&options).unwrap();

            let pow = character.attributes.get("POW").unwrap().base;

            // Parse sanity from notes
            let sanity_str = character.notes
                .lines()
                .find(|l| l.starts_with("Sanity:"))
                .expect("Should have Sanity in notes");

            let sanity: i32 = sanity_str
                .split(':')
                .last()
                .unwrap()
                .trim()
                .parse()
                .expect("Sanity should be a number");

            assert_eq!(sanity, pow, "Sanity ({}) should equal POW ({})", sanity, pow);
        }
    }

    #[test]
    fn test_luck_is_characteristic() {
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();

        assert!(
            character.attributes.contains_key("Luck"),
            "Luck should be a characteristic"
        );
    }

    #[test]
    fn test_max_sanity_is_99_minus_mythos() {
        // Max Sanity = 99 - Cthulhu Mythos skill
        // Since Mythos starts at 0, max sanity is 99
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();

        let mythos = character.skills.get("Cthulhu Mythos").unwrap();
        let max_sanity = 99 - mythos;

        assert_eq!(max_sanity, 99, "Max sanity should be 99 (with 0 Mythos)");
    }
}

// ============================================================================
// Backstory Generation Tests
// ============================================================================

#[cfg(test)]
mod backstory_tests {
    use super::*;

    #[test]
    fn test_investigator_has_background_elements() {
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();

        // All investigators should have these background elements
        assert!(!character.background.origin.is_empty());
        assert!(!character.background.motivation.is_empty());
        assert!(!character.background.connections.is_empty());
        assert!(!character.background.secrets.is_empty());
    }

    #[test]
    fn test_secrets_contain_unexplainable() {
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();

        // Default secret mentions witnessing something unexplainable
        let has_witness_secret = character.background.secrets.iter()
            .any(|s| s.to_lowercase().contains("unexplainable") || s.to_lowercase().contains("witness"));

        assert!(has_witness_secret, "Should have a secret about witnessing something unexplainable");
    }

    #[test]
    fn test_connections_include_useful_contacts() {
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();

        // Should have at least 2 connections
        assert!(
            character.background.connections.len() >= 2,
            "Should have at least 2 connections"
        );
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_investigator_creation() {
        let generator = create_test_generator();

        let options = GenerationOptions {
            system: Some("coc".to_string()),
            name: Some("Roland Banks".to_string()),
            concept: Some("Federal agent investigating the occult".to_string()),
            class: Some("Police Detective".to_string()),
            include_equipment: true,
            ..Default::default()
        };

        let character = generator.generate(&options).unwrap();

        assert_eq!(character.name, "Roland Banks");
        assert_eq!(character.system, GameSystem::CallOfCthulhu);

        // Has all characteristics
        assert_eq!(character.attributes.len(), 9);

        // Has skills
        assert!(character.skills.len() >= 40);

        // Has equipment
        assert!(!character.equipment.is_empty());

        // Has derived stats in notes
        assert!(character.notes.contains("HP:"));
        assert!(character.notes.contains("Sanity:"));
        assert!(character.notes.contains("Magic Points:"));
    }

    #[test]
    fn test_multiple_investigator_generations() {
        let generator = create_test_generator();
        let occupations = ["Doctor", "Professor", "Journalist", "Police Detective", "Dilettante"];

        for occupation in occupations {
            let options = GenerationOptions {
                class: Some(occupation.to_string()),
                include_equipment: true,
                ..create_default_options()
            };

            let result = generator.generate(&options);
            assert!(
                result.is_ok(),
                "Failed to generate {}: {:?}",
                occupation, result.err()
            );
        }
    }

    #[test]
    fn test_system_returns_correct_game_system() {
        let generator = create_test_generator();
        assert_eq!(generator.system(), GameSystem::CallOfCthulhu);
    }

    #[test]
    fn test_races_returns_only_human() {
        let generator = create_test_generator();
        let races = generator.available_races();

        // CoC doesn't use fantasy races
        assert_eq!(races.len(), 1);
        assert_eq!(races[0], "Human");
    }
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[cfg(test)]
mod edge_cases {
    use super::*;

    #[test]
    fn test_empty_name_generates_random() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            name: None,
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        assert!(!character.name.is_empty());
    }

    #[test]
    fn test_unknown_occupation_still_works() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            class: Some("Space Marine".to_string()),
            ..create_default_options()
        };

        // Should not panic, just use the provided occupation
        let result = generator.generate(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_consistent_characteristic_calculation() {
        let generator = create_test_generator();

        // Verify that HP, Sanity, and MP are always calculated correctly
        for _ in 0..20 {
            let options = create_default_options();
            let character = generator.generate(&options).unwrap();

            let con = character.attributes.get("CON").unwrap().base;
            let siz = character.attributes.get("SIZ").unwrap().base;
            let pow = character.attributes.get("POW").unwrap().base;

            let expected_hp = (con + siz) / 10;
            let expected_san = pow;
            let expected_mp = pow / 5;

            assert!(
                character.notes.contains(&format!("HP: {}", expected_hp)),
                "HP calculation incorrect"
            );
            assert!(
                character.notes.contains(&format!("Sanity: {}", expected_san)),
                "Sanity calculation incorrect"
            );
            assert!(
                character.notes.contains(&format!("Magic Points: {}", expected_mp)),
                "MP calculation incorrect"
            );
        }
    }
}
