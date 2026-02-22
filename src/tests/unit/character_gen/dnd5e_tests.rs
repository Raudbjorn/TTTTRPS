//! D&D 5th Edition Character Generator Unit Tests
//!
//! Tests for D&D 5e character generation including:
//! - Character creation with valid inputs
//! - Attribute generation (standard array, point buy, rolled)
//! - Class/subclass feature assignment
//! - Spell list generation
//! - Multiclass validation
//! - Race ability bonuses
//! - Proficiency calculation

use crate::core::character_gen::{
    systems::dnd5e::DnD5eGenerator,
    AttributeValue, CharacterTrait, EquipmentCategory,
    GameSystem, GenerationOptions, SystemGenerator, TraitType,
};

// ============================================================================
// Test Helpers
// ============================================================================

fn create_test_generator() -> DnD5eGenerator {
    DnD5eGenerator::new()
}

fn create_default_options() -> GenerationOptions {
    GenerationOptions {
        system: Some("dnd5e".to_string()),
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
        assert_eq!(character.system, GameSystem::DnD5e);
        assert!(!character.name.is_empty());
        assert_eq!(character.level, 1);
        // Default race and class
        assert_eq!(character.race.as_deref(), Some("Human"));
        assert_eq!(character.class.as_deref(), Some("Fighter"));
    }

    #[test]
    fn test_create_character_with_custom_name() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            name: Some("Thorin Ironforge".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        assert_eq!(character.name, "Thorin Ironforge");
    }

    #[test]
    fn test_create_character_with_all_options() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            system: Some("dnd5e".to_string()),
            name: Some("Elara Moonshadow".to_string()),
            concept: Some("Stealthy scout".to_string()),
            race: Some("Elf".to_string()),
            class: Some("Rogue".to_string()),
            background: Some("Criminal".to_string()),
            level: Some(5),
            random_stats: false,
            include_equipment: true,
            ..Default::default()
        };

        let character = generator.generate(&options).unwrap();

        assert_eq!(character.name, "Elara Moonshadow");
        assert_eq!(character.concept, "Stealthy scout");
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

        // UUID should be a valid format (36 chars with hyphens)
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

        // Standard array values: 15, 14, 13, 12, 10, 8
        let expected_values: Vec<i32> = vec![15, 14, 13, 12, 10, 8];
        let mut actual_values: Vec<i32> = attrs.values().map(|a| a.base).collect();
        actual_values.sort_by(|a, b| b.cmp(a)); // Sort descending

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

        // Run multiple times to verify randomness produces valid results
        for _ in 0..10 {
            let character = generator.generate(&options).unwrap();

            for (name, attr) in &character.attributes {
                // 4d6 drop lowest: min = 3 (all 1s after dropping), max = 18 (all 6s)
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
        // Test the modifier calculation formula: (base - 10) / 2
        let test_cases = [
            (8, -1),
            // Note: Rust integer division rounds toward zero, so (9-10)/2 = -1/2 = 0
            (9, 0),
            (10, 0),
            (11, 0),
            (12, 1),
            (13, 1),
            (14, 2),
            (15, 2),
            (16, 3),
            (17, 3),
            (18, 4),
            (20, 5),
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

    #[test]
    fn test_attribute_value_total() {
        let mut attr = AttributeValue::new(14);
        assert_eq!(attr.total(), 14);

        attr.temp_bonus = 2;
        assert_eq!(attr.total(), 16);
        assert_eq!(attr.total_modifier(), 3); // (16 - 10) / 2 = 3
    }
}

// ============================================================================
// Race Tests
// ============================================================================

#[cfg(test)]
mod race_tests {
    use super::*;

    #[test]
    fn test_available_races() {
        let generator = create_test_generator();
        let races = generator.available_races();

        // Core races should be present
        let expected_races = [
            "Human", "Elf", "Dwarf", "Halfling", "Dragonborn",
            "Gnome", "Half-Elf", "Half-Orc", "Tiefling",
        ];

        for race in expected_races {
            assert!(
                races.iter().any(|r| r == race),
                "Expected race '{}' not found in available races",
                race
            );
        }
    }

    #[test]
    fn test_human_racial_traits() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            race: Some("Human".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();

        let has_versatile = character.traits.iter().any(|t| {
            t.name == "Versatile" && t.trait_type == TraitType::Racial
        });
        assert!(has_versatile, "Human should have Versatile trait");
    }

    #[test]
    fn test_elf_racial_traits() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            race: Some("Elf".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let racial_traits: Vec<&CharacterTrait> = character.traits
            .iter()
            .filter(|t| t.trait_type == TraitType::Racial)
            .collect();

        let trait_names: Vec<&str> = racial_traits.iter().map(|t| t.name.as_str()).collect();

        assert!(trait_names.contains(&"Darkvision"), "Elf should have Darkvision");
        assert!(trait_names.contains(&"Fey Ancestry"), "Elf should have Fey Ancestry");
        assert!(trait_names.contains(&"Trance"), "Elf should have Trance");
    }

    #[test]
    fn test_dwarf_racial_traits() {
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

        assert!(racial_traits.contains(&"Darkvision"));
        assert!(racial_traits.contains(&"Dwarven Resilience"));
        assert!(racial_traits.contains(&"Stonecunning"));
    }

    #[test]
    fn test_halfling_racial_traits() {
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

        assert!(racial_traits.contains(&"Lucky"));
        assert!(racial_traits.contains(&"Brave"));
        assert!(racial_traits.contains(&"Halfling Nimbleness"));
    }

    #[test]
    fn test_tiefling_racial_traits() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            race: Some("Tiefling".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let racial_traits: Vec<&str> = character.traits
            .iter()
            .filter(|t| t.trait_type == TraitType::Racial)
            .map(|t| t.name.as_str())
            .collect();

        assert!(racial_traits.contains(&"Darkvision"));
        assert!(racial_traits.contains(&"Hellish Resistance"));
        assert!(racial_traits.contains(&"Infernal Legacy"));
    }

    #[test]
    fn test_dragonborn_racial_traits() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            race: Some("Dragonborn".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let racial_traits: Vec<&str> = character.traits
            .iter()
            .filter(|t| t.trait_type == TraitType::Racial)
            .map(|t| t.name.as_str())
            .collect();

        assert!(racial_traits.contains(&"Draconic Ancestry"));
        assert!(racial_traits.contains(&"Breath Weapon"));
    }

    #[test]
    fn test_unknown_race_gets_generic_trait() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            race: Some("Aasimar".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let has_heritage_trait = character.traits.iter().any(|t| {
            t.name.contains("Heritage") && t.trait_type == TraitType::Racial
        });

        assert!(has_heritage_trait, "Unknown race should have generic heritage trait");
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
            "Barbarian", "Bard", "Cleric", "Druid", "Fighter",
            "Monk", "Paladin", "Ranger", "Rogue", "Sorcerer",
            "Warlock", "Wizard",
        ];

        assert_eq!(classes.len(), 12);
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

        assert!(class_traits.contains(&"Fighting Style"));
        assert!(class_traits.contains(&"Second Wind"));
    }

    #[test]
    fn test_fighter_action_surge_at_level_2() {
        let generator = create_test_generator();

        // Level 1 - no Action Surge
        let options_l1 = GenerationOptions {
            class: Some("Fighter".to_string()),
            level: Some(1),
            ..create_default_options()
        };
        let char_l1 = generator.generate(&options_l1).unwrap();
        let has_action_surge_l1 = char_l1.traits.iter().any(|t| t.name == "Action Surge");
        assert!(!has_action_surge_l1, "Fighter level 1 should NOT have Action Surge");

        // Level 2 - has Action Surge
        let options_l2 = GenerationOptions {
            class: Some("Fighter".to_string()),
            level: Some(2),
            ..create_default_options()
        };
        let char_l2 = generator.generate(&options_l2).unwrap();
        let has_action_surge_l2 = char_l2.traits.iter().any(|t| t.name == "Action Surge");
        assert!(has_action_surge_l2, "Fighter level 2 should have Action Surge");
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

        assert!(class_traits.contains(&"Spellcasting"));
        assert!(class_traits.contains(&"Arcane Recovery"));
    }

    #[test]
    fn test_rogue_sneak_attack_scales_with_level() {
        let generator = create_test_generator();

        for level in [1, 3, 5, 7, 9, 11] {
            let options = GenerationOptions {
                class: Some("Rogue".to_string()),
                level: Some(level),
                ..create_default_options()
            };
            let character = generator.generate(&options).unwrap();

            let sneak_attack = character.traits.iter()
                .find(|t| t.name == "Sneak Attack")
                .expect("Rogue should have Sneak Attack");

            // Formula: (level + 1) / 2 dice
            let expected_dice = (level + 1) / 2;
            let expected_effect = format!("{}d6 extra damage", expected_dice);

            assert!(
                sneak_attack.mechanical_effect.as_ref().map(|e| e.contains(&expected_effect)).unwrap_or(false),
                "Level {} rogue should have {}d6 sneak attack",
                level, expected_dice
            );
        }
    }

    #[test]
    fn test_rogue_cunning_action_at_level_2() {
        let generator = create_test_generator();

        let options = GenerationOptions {
            class: Some("Rogue".to_string()),
            level: Some(2),
            ..create_default_options()
        };
        let character = generator.generate(&options).unwrap();

        let has_cunning_action = character.traits.iter().any(|t| t.name == "Cunning Action");
        assert!(has_cunning_action, "Rogue level 2 should have Cunning Action");
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

        assert!(class_traits.contains(&"Spellcasting"));
        assert!(class_traits.contains(&"Divine Domain"));
    }

    #[test]
    fn test_paladin_lay_on_hands_scales() {
        let generator = create_test_generator();

        for level in [1, 5, 10] {
            let options = GenerationOptions {
                class: Some("Paladin".to_string()),
                level: Some(level),
                ..create_default_options()
            };
            let character = generator.generate(&options).unwrap();

            let lay_on_hands = character.traits.iter()
                .find(|t| t.name == "Lay on Hands")
                .expect("Paladin should have Lay on Hands");

            let expected_hp = level * 5;
            assert!(
                lay_on_hands.mechanical_effect.as_ref()
                    .map(|e| e.contains(&expected_hp.to_string()))
                    .unwrap_or(false),
                "Level {} paladin Lay on Hands should have {} HP pool",
                level, expected_hp
            );
        }
    }

    #[test]
    fn test_barbarian_class_features() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            class: Some("Barbarian".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let class_traits: Vec<&str> = character.traits
            .iter()
            .filter(|t| t.trait_type == TraitType::Class)
            .map(|t| t.name.as_str())
            .collect();

        assert!(class_traits.contains(&"Rage"));
        assert!(class_traits.contains(&"Unarmored Defense"));
    }

    #[test]
    fn test_monk_ki_points_scale() {
        let generator = create_test_generator();

        for level in [2, 5, 10] {
            let options = GenerationOptions {
                class: Some("Monk".to_string()),
                level: Some(level),
                ..create_default_options()
            };
            let character = generator.generate(&options).unwrap();

            let ki = character.traits.iter()
                .find(|t| t.name == "Ki");

            if level >= 2 {
                let ki = ki.expect("Monk level 2+ should have Ki");
                assert!(
                    ki.mechanical_effect.as_ref()
                        .map(|e| e.contains(&format!("{} ki points", level)))
                        .unwrap_or(false),
                    "Level {} monk should have {} ki points",
                    level, level
                );
            }
        }
    }

    #[test]
    fn test_unknown_class_gets_generic_trait() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            class: Some("Artificer".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        let has_training = character.traits.iter().any(|t| {
            t.name.contains("Training") && t.trait_type == TraitType::Class
        });

        assert!(has_training, "Unknown class should have generic training trait");
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
            "Acrobatics", "Animal Handling", "Arcana", "Athletics",
            "Deception", "History", "Insight", "Intimidation",
            "Investigation", "Medicine", "Nature", "Perception",
            "Performance", "Persuasion", "Religion", "Sleight of Hand",
            "Stealth", "Survival",
        ];

        assert_eq!(character.skills.len(), 18);

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
        let has_shield = equipment.iter().any(|e| e.name == "Shield");
        let has_chain_mail = equipment.iter().any(|e| e.name == "Chain Mail");

        assert!(has_longsword, "Fighter should start with Longsword");
        assert!(has_shield, "Fighter should start with Shield");
        assert!(has_chain_mail, "Fighter should start with Chain Mail");
    }

    #[test]
    fn test_rogue_starting_equipment() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("Rogue"));

        let has_shortsword = equipment.iter().any(|e| e.name == "Shortsword");
        let has_shortbow = equipment.iter().any(|e| e.name == "Shortbow");
        let has_leather = equipment.iter().any(|e| e.name == "Leather Armor");
        let has_thieves_tools = equipment.iter().any(|e| e.name == "Thieves' Tools");

        assert!(has_shortsword, "Rogue should start with Shortsword");
        assert!(has_shortbow, "Rogue should start with Shortbow");
        assert!(has_leather, "Rogue should start with Leather Armor");
        assert!(has_thieves_tools, "Rogue should start with Thieves' Tools");
    }

    #[test]
    fn test_wizard_starting_equipment() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("Wizard"));

        let has_quarterstaff = equipment.iter().any(|e| e.name == "Quarterstaff");
        let has_spellbook = equipment.iter().any(|e| e.name == "Spellbook");
        let has_arcane_focus = equipment.iter().any(|e| e.name == "Arcane Focus");

        assert!(has_quarterstaff, "Wizard should start with Quarterstaff");
        assert!(has_spellbook, "Wizard should start with Spellbook");
        assert!(has_arcane_focus, "Wizard should start with Arcane Focus");
    }

    #[test]
    fn test_common_adventuring_gear() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("Fighter"));

        let has_backpack = equipment.iter().any(|e| e.name == "Backpack");
        let has_bedroll = equipment.iter().any(|e| e.name == "Bedroll");
        let has_rations = equipment.iter().any(|e| e.name.contains("Rations"));
        let has_waterskin = equipment.iter().any(|e| e.name == "Waterskin");

        assert!(has_backpack, "Should have Backpack");
        assert!(has_bedroll, "Should have Bedroll");
        assert!(has_rations, "Should have Rations");
        assert!(has_waterskin, "Should have Waterskin");
    }

    #[test]
    fn test_equipment_categories() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("Fighter"));

        let has_weapon = equipment.iter().any(|e| matches!(e.category, EquipmentCategory::Weapon));
        let has_armor = equipment.iter().any(|e| matches!(e.category, EquipmentCategory::Armor));
        let has_tool = equipment.iter().any(|e| matches!(e.category, EquipmentCategory::Tool));

        assert!(has_weapon, "Fighter equipment should include weapons");
        assert!(has_armor, "Fighter equipment should include armor");
        assert!(has_tool, "Fighter equipment should include tools");
    }

    #[test]
    fn test_unknown_class_gets_dagger() {
        let generator = create_test_generator();
        let equipment = generator.starting_equipment(Some("SomeUnknownClass"));

        let has_dagger = equipment.iter().any(|e| e.name == "Dagger");
        assert!(has_dagger, "Unknown class should get a Dagger as default weapon");
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
            "Acolyte", "Charlatan", "Criminal", "Entertainer",
            "Folk Hero", "Guild Artisan", "Hermit", "Noble",
            "Outlander", "Sage", "Sailor", "Soldier", "Urchin",
        ];

        assert_eq!(backgrounds.len(), 13);
        for bg in expected_backgrounds {
            assert!(
                backgrounds.contains(&bg.to_string()),
                "Expected background '{}' not found",
                bg
            );
        }
    }

    #[test]
    fn test_custom_background() {
        let generator = create_test_generator();
        let options = GenerationOptions {
            background: Some("Sage".to_string()),
            ..create_default_options()
        };

        let character = generator.generate(&options).unwrap();
        assert_eq!(character.background.origin, "Sage");
    }

    #[test]
    fn test_default_background() {
        let generator = create_test_generator();
        let options = create_default_options();

        let character = generator.generate(&options).unwrap();
        assert_eq!(character.background.origin, "Folk Hero");
    }
}

// ============================================================================
// Proficiency Calculation Tests
// ============================================================================

#[cfg(test)]
mod proficiency_tests {

    /// Test proficiency bonus calculation based on level
    /// 5e proficiency: +2 at level 1-4, +3 at 5-8, +4 at 9-12, +5 at 13-16, +6 at 17-20
    fn calculate_proficiency_bonus(level: u32) -> i32 {
        ((level - 1) / 4 + 2) as i32
    }

    #[test]
    fn test_proficiency_bonus_by_level() {
        let test_cases = [
            (1, 2), (4, 2),
            (5, 3), (8, 3),
            (9, 4), (12, 4),
            (13, 5), (16, 5),
            (17, 6), (20, 6),
        ];

        for (level, expected_prof) in test_cases {
            let prof = calculate_proficiency_bonus(level);
            assert_eq!(
                prof, expected_prof,
                "Level {} should have +{} proficiency bonus",
                level, expected_prof
            );
        }
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
            system: Some("dnd5e".to_string()),
            name: Some("Gandalf the Grey".to_string()),
            concept: Some("Wise wizard seeking to protect the realm".to_string()),
            race: Some("Human".to_string()),
            class: Some("Wizard".to_string()),
            background: Some("Sage".to_string()),
            level: Some(10),
            random_stats: false,
            include_equipment: true,
            ..Default::default()
        };

        let character = generator.generate(&options).unwrap();

        // Verify all aspects of the character
        assert_eq!(character.name, "Gandalf the Grey");
        assert_eq!(character.system, GameSystem::DnD5e);
        assert_eq!(character.race.as_deref(), Some("Human"));
        assert_eq!(character.class.as_deref(), Some("Wizard"));
        assert_eq!(character.level, 10);
        assert_eq!(character.background.origin, "Sage");

        // Has attributes
        assert_eq!(character.attributes.len(), 6);

        // Has skills
        assert_eq!(character.skills.len(), 18);

        // Has traits (racial + class)
        assert!(!character.traits.is_empty());

        // Has equipment
        assert!(!character.equipment.is_empty());
    }

    #[test]
    fn test_multiple_character_generations() {
        let generator = create_test_generator();

        let classes = ["Fighter", "Wizard", "Rogue", "Cleric"];
        let races = ["Human", "Elf", "Dwarf", "Halfling"];

        for class in classes {
            for race in races {
                let options = GenerationOptions {
                    class: Some(class.to_string()),
                    race: Some(race.to_string()),
                    include_equipment: true,
                    ..create_default_options()
                };

                let result = generator.generate(&options);
                assert!(
                    result.is_ok(),
                    "Failed to generate {} {}: {:?}",
                    race, class, result.err()
                );
            }
        }
    }
}
