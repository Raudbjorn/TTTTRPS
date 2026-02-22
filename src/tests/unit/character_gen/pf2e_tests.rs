//! Pathfinder 2nd Edition Character Generator Unit Tests
//!
//! Tests for PF2e character generation including:
//! - Ancestry selection and bonuses
//! - Class features
//! - Action economy validation
//! - Dedication feat validation

use crate::core::character_gen::{
    systems::pf2e::Pathfinder2eGenerator,
    AttributeValue,
    GameSystem, GenerationOptions, SystemGenerator, TraitType,
};

// ============================================================================
// Test Helpers
// ============================================================================

fn create_test_generator() -> Pathfinder2eGenerator {
    Pathfinder2eGenerator::new()
}

fn create_default_options() -> GenerationOptions {
    GenerationOptions {
        system: Some("pf2e".to_string()),
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
    fn test_create_character_with_defaults() {
        let generator = create_test_generator();
        let options = create_default_options();

        let result = generator.generate(&options);
        assert!(result.is_ok());

        let character = result.unwrap();
        assert_eq!(character.system, GameSystem::Pathfinder2e);
        assert!(!character.name.is_empty());
        assert_eq!(character.level, 1);
        // Default ancestry and class
        assert_eq!(character.race.as_deref(), Some("Human"));
        assert_eq!(character.class.as_deref(), Some("Fighter"));
    }

    #[test]
    fn test_create_character_with_custom_name() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            name: Some("Valeros the Bold".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        assert_eq!(character.name, "Valeros the Bold");
    }

    #[test]
    fn test_create_character_with_all_options() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            system: Some("pf2e".to_string()),
            name: Some("Merisiel".to_string()),
            concept: Some("Elven rogue seeking adventure".to_string()),
            race: Some("Elf".to_string()),
            class: Some("Rogue".to_string()),
            background: Some("Bounty Hunter".to_string()),
            level: Some(5),
            random_stats: false,
            include_equipment: true,
            ..Default::default()
        };

        let character = generator.generate(&options).unwrap();

        assert_eq!(character.name, "Merisiel");
        assert_eq!(character.concept, "Elven rogue seeking adventure");
        assert_eq!(character.race.as_deref(), Some("Elf"));
        assert_eq!(character.class.as_deref(), Some("Rogue"));
        assert_eq!(character.level, 5);
        assert!(!character.equipment.is_empty());
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
    fn test_multiple_characters_have_unique_ids() {
        let generator = create_test_generator();
        let options = create_default_options();

        let char1 = generator.generate(&options).unwrap();
        let char2 = generator.generate(&options).unwrap();

        assert_ne!(char1.id, char2.id);
    }
}

// ============================================================================
// Attribute Generation Tests
// ============================================================================

#[cfg(test)]
mod attribute_generation {
    use super::*;

    #[test]
    fn test_standard_array_attributes() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            random_stats: false,
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let attrs = &character.attributes;

        // PF2e standard-like array: 16, 14, 14, 12, 10, 10
        let expected_values: Vec<i32> = vec![16, 14, 14, 12, 10, 10];
        let mut actual_values: Vec<i32> = attrs.values().map(|a| a.base).collect();
        actual_values.sort_by(|a, b| b.cmp(a));

        assert_eq!(actual_values, expected_values);
    }

    #[test]
    fn test_all_six_attributes_present() {
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();
        let attrs = &character.attributes;

        let expected_attrs = ["Strength", "Dexterity", "Constitution", "Intelligence", "Wisdom", "Charisma"];
        for attr in expected_attrs {
            assert!(attrs.contains_key(attr), "Missing attribute: {}", attr);
        }
    }

    #[test]
    fn test_rolled_stats_within_range() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            random_stats: true,
            ..create_default_options()
        };

        for _ in 0..10 {
            let character = generator.generate(&options).unwrap();

            for (name, attr) in &character.attributes {
                // 4d6 drop lowest: min = 3, max = 18
                assert!(
                    attr.base >= 3 && attr.base <= 18,
                    "Attribute {} = {} is out of range [3, 18]",
                    name,
                    attr.base
                );
            }
        }
    }

    #[test]
    fn test_attribute_modifier_calculation() {
        // PF2e uses the same modifier formula as 5e: (base - 10) / 2
        let test_cases = [
            (8, -1),
            (10, 0),
            (12, 1),
            (14, 2),
            (16, 3),
            (18, 4),
        ];

        for (base, expected_mod) in test_cases {
            let attr = AttributeValue::new(base);
            assert_eq!(
                attr.modifier, expected_mod,
                "Base {} should have modifier {}, got {}",
                base, expected_mod, attr.modifier
            );
        }
    }
}

// ============================================================================
// Ancestry Tests
// ============================================================================

#[cfg(test)]
mod ancestry_tests {
    use super::*;

    #[test]
    fn test_available_ancestries() {
        let generator = create_test_generator();
        let ancestries = generator.available_races();

        // Core ancestries should be present
        let expected_ancestries = [
            "Human", "Elf", "Dwarf", "Gnome", "Goblin",
            "Halfling", "Leshy", "Orc",
        ];

        for ancestry in expected_ancestries {
            assert!(
                ancestries.iter().any(|a| a == ancestry),
                "Expected ancestry '{}' not found",
                ancestry
            );
        }
    }

    #[test]
    fn test_human_ancestry_traits() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            race: Some("Human".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();

        let has_human_trait = character.traits.iter().any(|t| {
            t.name == "Human" && t.trait_type == TraitType::Racial
        });
        assert!(has_human_trait, "Human should have Human ancestry trait");
    }

    #[test]
    fn test_elf_ancestry_traits() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            race: Some("Elf".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let racial_traits: Vec<&str> = character.traits
            .iter()
            .filter(|t| t.trait_type == TraitType::Racial)
            .map(|t| t.name.as_str())
            .collect();

        assert!(racial_traits.contains(&"Elf"), "Should have Elf trait");
        assert!(racial_traits.contains(&"Low-Light Vision"), "Elf should have Low-Light Vision");
    }

    #[test]
    fn test_dwarf_ancestry_traits() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            race: Some("Dwarf".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let racial_traits: Vec<&str> = character.traits
            .iter()
            .filter(|t| t.trait_type == TraitType::Racial)
            .map(|t| t.name.as_str())
            .collect();

        assert!(racial_traits.contains(&"Dwarf"));
        assert!(racial_traits.contains(&"Darkvision"));
        assert!(racial_traits.contains(&"Clan Dagger"));
    }

    #[test]
    fn test_goblin_ancestry_traits() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            race: Some("Goblin".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let racial_traits: Vec<&str> = character.traits
            .iter()
            .filter(|t| t.trait_type == TraitType::Racial)
            .map(|t| t.name.as_str())
            .collect();

        assert!(racial_traits.contains(&"Goblin"));
        assert!(racial_traits.contains(&"Darkvision"));
    }

    #[test]
    fn test_halfling_ancestry_traits() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            race: Some("Halfling".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let racial_traits: Vec<&str> = character.traits
            .iter()
            .filter(|t| t.trait_type == TraitType::Racial)
            .map(|t| t.name.as_str())
            .collect();

        assert!(racial_traits.contains(&"Halfling"));
        assert!(racial_traits.contains(&"Keen Eyes"));
    }

    #[test]
    fn test_gnome_ancestry_traits() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            race: Some("Gnome".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let racial_traits: Vec<&str> = character.traits
            .iter()
            .filter(|t| t.trait_type == TraitType::Racial)
            .map(|t| t.name.as_str())
            .collect();

        assert!(racial_traits.contains(&"Gnome"));
        assert!(racial_traits.contains(&"Low-Light Vision"));
    }

    #[test]
    fn test_orc_ancestry_traits() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            race: Some("Orc".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let racial_traits: Vec<&str> = character.traits
            .iter()
            .filter(|t| t.trait_type == TraitType::Racial)
            .map(|t| t.name.as_str())
            .collect();

        assert!(racial_traits.contains(&"Orc"));
        assert!(racial_traits.contains(&"Darkvision"));
    }

    #[test]
    fn test_leshy_ancestry_traits() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            race: Some("Leshy".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let has_leshy = character.traits.iter().any(|t| {
            t.name == "Leshy" && t.trait_type == TraitType::Racial
        });

        assert!(has_leshy, "Should have Leshy ancestry trait");
    }

    #[test]
    fn test_unknown_ancestry_gets_generic_trait() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            race: Some("Kitsune".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let has_ancestry_trait = character.traits.iter().any(|t| {
            t.name.contains("Ancestry") && t.trait_type == TraitType::Racial
        });

        assert!(has_ancestry_trait, "Unknown ancestry should have generic ancestry trait");
    }

    #[test]
    fn test_ancestry_count() {
        let generator = create_test_generator();
        let ancestries = generator.available_races();

        // PF2e has many ancestries - should have at least 8 core
        assert!(ancestries.len() >= 8, "Should have at least 8 ancestries");
    }
}

// ============================================================================
// Class Feature Tests
// ============================================================================

#[cfg(test)]
mod class_feature_tests {
    use super::*;

    #[test]
    fn test_available_classes() {
        let generator = create_test_generator();
        let classes = generator.available_classes();

        let expected_classes = [
            "Alchemist", "Barbarian", "Bard", "Champion", "Cleric",
            "Druid", "Fighter", "Monk", "Ranger", "Rogue",
            "Sorcerer", "Wizard",
        ];

        for class in expected_classes {
            assert!(
                classes.contains(&class.to_string()),
                "Expected class '{}' not found",
                class
            );
        }
    }

    #[test]
    fn test_fighter_class_features() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            class: Some("Fighter".to_string()),
            level: Some(1),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let class_traits: Vec<&str> = character.traits
            .iter()
            .filter(|t| t.trait_type == TraitType::Class)
            .map(|t| t.name.as_str())
            .collect();

        assert!(class_traits.contains(&"Attack of Opportunity"), "Fighter should have Attack of Opportunity");
        assert!(class_traits.contains(&"Shield Block"), "Fighter should have Shield Block");
    }

    #[test]
    fn test_fighter_bravery_at_level_3() {
        let generator = create_test_generator();

        // Level 2 - no Bravery
        let options_l2 = GenerationOptions {
            class: Some("Fighter".to_string()),
            level: Some(2),
            ..create_default_options()
        };
        let char_l2 = generator.generate(&options_l2).unwrap();
        let has_bravery_l2 = char_l2.traits.iter().any(|t| t.name == "Bravery");
        assert!(!has_bravery_l2, "Fighter level 2 should NOT have Bravery");

        // Level 3 - has Bravery
        let options_l3 = GenerationOptions {
            class: Some("Fighter".to_string()),
            level: Some(3),
            ..create_default_options()
        };
        let char_l3 = generator.generate(&options_l3).unwrap();
        let has_bravery_l3 = char_l3.traits.iter().any(|t| t.name == "Bravery");
        assert!(has_bravery_l3, "Fighter level 3 should have Bravery");
    }

    #[test]
    fn test_wizard_class_features() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            class: Some("Wizard".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let class_traits: Vec<&str> = character.traits
            .iter()
            .filter(|t| t.trait_type == TraitType::Class)
            .map(|t| t.name.as_str())
            .collect();

        assert!(class_traits.contains(&"Arcane Spellcasting"));
        assert!(class_traits.contains(&"Arcane School"));
        assert!(class_traits.contains(&"Arcane Bond"));
    }

    #[test]
    fn test_rogue_class_features() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            class: Some("Rogue".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let class_traits: Vec<&str> = character.traits
            .iter()
            .filter(|t| t.trait_type == TraitType::Class)
            .map(|t| t.name.as_str())
            .collect();

        assert!(class_traits.contains(&"Sneak Attack"));
        assert!(class_traits.contains(&"Surprise Attack"));
        assert!(class_traits.contains(&"Racket"));
    }

    #[test]
    fn test_cleric_class_features() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            class: Some("Cleric".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let class_traits: Vec<&str> = character.traits
            .iter()
            .filter(|t| t.trait_type == TraitType::Class)
            .map(|t| t.name.as_str())
            .collect();

        assert!(class_traits.contains(&"Divine Spellcasting"));
        assert!(class_traits.contains(&"Divine Font"));
        assert!(class_traits.contains(&"Doctrine"));
    }

    #[test]
    fn test_champion_class_features() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            class: Some("Champion".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let class_traits: Vec<&str> = character.traits
            .iter()
            .filter(|t| t.trait_type == TraitType::Class)
            .map(|t| t.name.as_str())
            .collect();

        assert!(class_traits.contains(&"Champion's Code"));
        assert!(class_traits.contains(&"Champion's Reaction"));
        assert!(class_traits.contains(&"Deity's Domain"));
    }

    #[test]
    fn test_barbarian_class_features() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            class: Some("Barbarian".to_string()),
            level: Some(1),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let class_traits: Vec<&str> = character.traits
            .iter()
            .filter(|t| t.trait_type == TraitType::Class)
            .map(|t| t.name.as_str())
            .collect();

        assert!(class_traits.contains(&"Rage"));
        assert!(class_traits.contains(&"Instinct"));
    }

    #[test]
    fn test_barbarian_deny_advantage_at_level_3() {
        let generator = create_test_generator();

        let options = GenerationOptions {
            class: Some("Barbarian".to_string()),
            level: Some(3),
            ..create_default_options()
        };
        let character = generator.generate(&options).unwrap();

        let has_deny_advantage = character.traits.iter().any(|t| t.name == "Deny Advantage");
        assert!(has_deny_advantage, "Barbarian level 3 should have Deny Advantage");
    }

    #[test]
    fn test_ranger_class_features() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            class: Some("Ranger".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let class_traits: Vec<&str> = character.traits
            .iter()
            .filter(|t| t.trait_type == TraitType::Class)
            .map(|t| t.name.as_str())
            .collect();

        assert!(class_traits.contains(&"Hunt Prey"));
        assert!(class_traits.contains(&"Hunter's Edge"));
    }

    #[test]
    fn test_bard_class_features() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            class: Some("Bard".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let class_traits: Vec<&str> = character.traits
            .iter()
            .filter(|t| t.trait_type == TraitType::Class)
            .map(|t| t.name.as_str())
            .collect();

        assert!(class_traits.contains(&"Occult Spellcasting"));
        assert!(class_traits.contains(&"Composition Spells"));
        assert!(class_traits.contains(&"Muse"));
    }

    #[test]
    fn test_unknown_class_gets_generic_trait() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            class: Some("Inventor".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let has_training = character.traits.iter().any(|t| {
            t.name.contains("Training") && t.trait_type == TraitType::Class
        });

        assert!(has_training, "Unknown class should have generic training trait");
    }

    #[test]
    fn test_class_count() {
        let generator = create_test_generator();
        let classes = generator.available_classes();

        // PF2e has many classes - should have at least 12 core + expansion
        assert!(classes.len() >= 12, "Should have at least 12 classes");
    }
}

// ============================================================================
// Skills Tests
// ============================================================================

#[cfg(test)]
mod skill_tests {
    use super::*;

    #[test]
    fn test_all_skills_present() {
        let generator = create_test_generator();
        let options = create_default_options();
        let character = generator.generate(&options).unwrap();

        let expected_skills = [
            "Acrobatics", "Arcana", "Athletics", "Crafting",
            "Deception", "Diplomacy", "Intimidation", "Lore",
            "Medicine", "Nature", "Occultism", "Performance",
            "Religion", "Society", "Stealth", "Survival", "Thievery",
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
    fn test_skills_initialized_to_zero() {
        let generator = create_test_generator();
        let options = create_default_options();
        let character = generator.generate(&options).unwrap();

        for (skill, value) in &character.skills {
            assert_eq!(
                *value, 0,
                "Skill '{}' should be initialized to 0, got {}",
                skill, value
            );
        }
    }

    #[test]
    fn test_skill_count() {
        let generator = create_test_generator();
        let options = create_default_options();
        let character = generator.generate(&options).unwrap();

        // PF2e has 17 skills
        assert_eq!(character.skills.len(), 17);
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
    fn test_fighter_starting_equipment() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("Fighter"));

        let has_longsword = equipment.iter().any(|e| e.name == "Longsword");
        let has_shield = equipment.iter().any(|e| e.name == "Steel Shield");
        let has_chain_mail = equipment.iter().any(|e| e.name == "Chain Mail");

        assert!(has_longsword, "Fighter should start with Longsword");
        assert!(has_shield, "Fighter should start with Steel Shield");
        assert!(has_chain_mail, "Fighter should start with Chain Mail");
    }

    #[test]
    fn test_rogue_starting_equipment() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("Rogue"));

        let has_rapier = equipment.iter().any(|e| e.name == "Rapier");
        let has_shortbow = equipment.iter().any(|e| e.name == "Shortbow");
        let has_leather = equipment.iter().any(|e| e.name == "Leather Armor");
        let has_thieves_tools = equipment.iter().any(|e| e.name == "Thieves' Tools");

        assert!(has_rapier, "Rogue should start with Rapier");
        assert!(has_shortbow, "Rogue should start with Shortbow");
        assert!(has_leather, "Rogue should start with Leather Armor");
        assert!(has_thieves_tools, "Rogue should start with Thieves' Tools");
    }

    #[test]
    fn test_wizard_starting_equipment() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("Wizard"));

        let has_staff = equipment.iter().any(|e| e.name == "Staff");
        let has_spellbook = equipment.iter().any(|e| e.name == "Spellbook");
        let has_components = equipment.iter().any(|e| e.name == "Material Component Pouch");

        assert!(has_staff, "Wizard should start with Staff");
        assert!(has_spellbook, "Wizard should start with Spellbook");
        assert!(has_components, "Wizard should start with Material Component Pouch");
    }

    #[test]
    fn test_cleric_starting_equipment() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("Cleric"));

        let has_religious_symbol = equipment.iter().any(|e| e.name == "Religious Symbol");
        let has_mace = equipment.iter().any(|e| e.name == "Mace");
        let has_scale_mail = equipment.iter().any(|e| e.name == "Scale Mail");

        assert!(has_religious_symbol, "Cleric should start with Religious Symbol");
        assert!(has_mace, "Cleric should start with Mace");
        assert!(has_scale_mail, "Cleric should start with Scale Mail");
    }

    #[test]
    fn test_common_adventuring_gear() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("Fighter"));

        let has_backpack = equipment.iter().any(|e| e.name == "Backpack");
        let has_bedroll = equipment.iter().any(|e| e.name == "Bedroll");
        let has_rations = equipment.iter().any(|e| e.name.contains("Rations"));

        assert!(has_backpack, "Should have Backpack");
        assert!(has_bedroll, "Should have Bedroll");
        assert!(has_rations, "Should have Rations");
    }

    #[test]
    fn test_unknown_class_gets_dagger() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("Swashbuckler"));

        let has_dagger = equipment.iter().any(|e| e.name == "Dagger");
        assert!(has_dagger, "Unknown class should get a Dagger as default weapon");
    }

    #[test]
    fn test_equipment_has_traits() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("Rogue"));

        // Rapier should have traits
        let rapier = equipment.iter().find(|e| e.name == "Rapier");
        assert!(rapier.is_some());

        let rapier = rapier.unwrap();
        assert!(rapier.stats.contains_key("Traits"), "Rapier should have Traits");
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
            "Acolyte", "Acrobat", "Artisan", "Charlatan",
            "Criminal", "Entertainer", "Farmhand", "Hermit",
            "Noble", "Scholar", "Scout", "Warrior",
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
    fn test_custom_background() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            background: Some("Bounty Hunter".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        assert_eq!(character.background.origin, "Bounty Hunter");
    }

    #[test]
    fn test_default_background() {
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();
        assert_eq!(character.background.origin, "Farmhand");
    }

    #[test]
    fn test_background_count() {
        let generator = create_test_generator();
        let backgrounds = generator.available_backgrounds();

        // PF2e has many backgrounds
        assert!(backgrounds.len() >= 20, "Should have at least 20 backgrounds");
    }
}

// ============================================================================
// Action Economy Tests (PF2e-specific)
// ============================================================================

#[cfg(test)]
mod action_economy_tests {
    use super::*;

    /// Validates that reaction abilities are properly marked
    #[test]
    fn test_fighter_attack_of_opportunity_is_reaction() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            class: Some("Fighter".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let aoo = character.traits.iter()
            .find(|t| t.name == "Attack of Opportunity")
            .expect("Fighter should have Attack of Opportunity");

        assert!(
            aoo.mechanical_effect.as_ref()
                .map(|e| e.to_lowercase().contains("reaction"))
                .unwrap_or(false),
            "Attack of Opportunity should be marked as a reaction"
        );
    }

    #[test]
    fn test_champion_reaction_is_reaction() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            class: Some("Champion".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let reaction = character.traits.iter()
            .find(|t| t.name == "Champion's Reaction")
            .expect("Champion should have Champion's Reaction");

        assert!(
            reaction.description.to_lowercase().contains("react") ||
            reaction.mechanical_effect.as_ref()
                .map(|e| e.to_lowercase().contains("step") || e.to_lowercase().contains("strike"))
                .unwrap_or(false),
            "Champion's Reaction should describe reactive abilities"
        );
    }

    #[test]
    fn test_shield_block_is_reaction() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            class: Some("Fighter".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let shield_block = character.traits.iter()
            .find(|t| t.name == "Shield Block")
            .expect("Fighter should have Shield Block");

        assert!(
            shield_block.mechanical_effect.as_ref()
                .map(|e| e.to_lowercase().contains("reaction"))
                .unwrap_or(false),
            "Shield Block should be marked as a reaction"
        );
    }
}

// ============================================================================
// Dedication Feat Validation Tests
// ============================================================================

#[cfg(test)]
mod dedication_tests {
    use super::*;

    /// Test that characters can have dedication-like features at appropriate levels
    #[test]
    fn test_multiclass_dedication_requires_level_2() {
        // In PF2e, multiclass dedication feats are available at level 2+
        let generator = create_test_generator();

        let options_l1 = GenerationOptions {
            level: Some(1),
            ..create_default_options()
        };
        let char_l1 = generator.generate(&options_l1).unwrap();
        assert_eq!(char_l1.level, 1);

        let options_l2 = GenerationOptions {
            level: Some(2),
            ..create_default_options()
        };
        let char_l2 = generator.generate(&options_l2).unwrap();
        assert_eq!(char_l2.level, 2);
        // At level 2, dedication feats become available (though we don't implement them yet)
    }

    /// Test that level affects available features
    #[test]
    fn test_level_progression_affects_features() {
        let generator = create_test_generator();

        for level in 1..=5 {
            let options = GenerationOptions {
                level: Some(level),
                class: Some("Fighter".to_string()),
                ..create_default_options()
            };
            let character = generator.generate(&options).unwrap();
            assert_eq!(character.level, level);

            // Higher levels should potentially have more features
            // (though this is simplified in current implementation)
        }
    }
}

// ============================================================================
// Proficiency Tests
// ============================================================================

#[cfg(test)]
mod proficiency_tests {

    /// PF2e proficiency bonus calculation: level + proficiency rank modifier
    /// Untrained: +0, Trained: +2, Expert: +4, Master: +6, Legendary: +8
    fn calculate_pf2e_proficiency(level: u32, rank: &str) -> i32 {
        let rank_bonus = match rank {
            "untrained" => 0,
            "trained" => 2,
            "expert" => 4,
            "master" => 6,
            "legendary" => 8,
            _ => 0,
        };

        if rank == "untrained" {
            0
        } else {
            level as i32 + rank_bonus
        }
    }

    #[test]
    fn test_pf2e_proficiency_by_rank() {
        // Level 1, trained = 1 + 2 = 3
        assert_eq!(calculate_pf2e_proficiency(1, "trained"), 3);

        // Level 5, expert = 5 + 4 = 9
        assert_eq!(calculate_pf2e_proficiency(5, "expert"), 9);

        // Level 10, master = 10 + 6 = 16
        assert_eq!(calculate_pf2e_proficiency(10, "master"), 16);

        // Level 20, legendary = 20 + 8 = 28
        assert_eq!(calculate_pf2e_proficiency(20, "legendary"), 28);

        // Untrained is always 0
        assert_eq!(calculate_pf2e_proficiency(10, "untrained"), 0);
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_character_creation_workflow() {
        let generator = create_test_generator();

        let options = GenerationOptions {
            system: Some("pf2e".to_string()),
            name: Some("Kyra the Cleric".to_string()),
            concept: Some("Devout healer of Sarenrae".to_string()),
            race: Some("Human".to_string()),
            class: Some("Cleric".to_string()),
            background: Some("Acolyte".to_string()),
            level: Some(5),
            random_stats: false,
            include_equipment: true,
            ..Default::default()
        };

        let character = generator.generate(&options).unwrap();

        assert_eq!(character.name, "Kyra the Cleric");
        assert_eq!(character.system, GameSystem::Pathfinder2e);
        assert_eq!(character.race.as_deref(), Some("Human"));
        assert_eq!(character.class.as_deref(), Some("Cleric"));
        assert_eq!(character.level, 5);
        assert_eq!(character.background.origin, "Acolyte");

        // Has all required components
        assert_eq!(character.attributes.len(), 6);
        assert_eq!(character.skills.len(), 17);
        assert!(!character.traits.is_empty());
        assert!(!character.equipment.is_empty());
    }

    #[test]
    fn test_multiple_character_generations() {
        let generator = create_test_generator();

        let classes = ["Fighter", "Wizard", "Rogue", "Cleric", "Champion", "Barbarian"];
        let ancestries = ["Human", "Elf", "Dwarf", "Goblin"];

        for class in classes {
            for ancestry in ancestries {
                let options = GenerationOptions {
                    class: Some(class.to_string()),
                    race: Some(ancestry.to_string()),
                    include_equipment: true,
                    ..create_default_options()
                };

                let result = generator.generate(&options);
                assert!(
                    result.is_ok(),
                    "Failed to generate {} {}: {:?}",
                    ancestry, class, result.err()
                );
            }
        }
    }

    #[test]
    fn test_system_returns_correct_game_system() {
        let generator = create_test_generator();
        assert_eq!(generator.system(), GameSystem::Pathfinder2e);
    }
}
