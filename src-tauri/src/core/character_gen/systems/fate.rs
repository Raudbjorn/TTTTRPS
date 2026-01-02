//! Fate Core Character Generator
//!
//! Generates characters for Fate Core and Fate Accelerated Edition (FAE).

use crate::core::character_gen::{
    SystemGenerator, Character, GameSystem, GenerationOptions,
    AttributeValue, CharacterTrait, TraitType, Equipment, EquipmentCategory,
    CharacterBackground, Result, random_fantasy_name, random_modern_name,
};
use rand::Rng;
use uuid::Uuid;
use std::collections::HashMap;

pub struct FateCoreGenerator;

impl FateCoreGenerator {
    pub fn new() -> Self {
        Self
    }

    fn generate_approaches(rng: &mut impl Rng) -> HashMap<String, AttributeValue> {
        // FAE approaches with ratings from +0 to +3
        let ratings = [3, 2, 2, 1, 1, 0];
        let approaches = ["Careful", "Clever", "Flashy", "Forceful", "Quick", "Sneaky"];

        let mut indices: Vec<usize> = (0..6).collect();
        for i in (1..6).rev() {
            let j = rng.gen_range(0..=i);
            indices.swap(i, j);
        }

        approaches.iter().enumerate().map(|(i, &approach)| {
            let rating = ratings[indices[i]];
            (approach.to_string(), AttributeValue::new_raw(rating))
        }).collect()
    }

    fn generate_skills() -> HashMap<String, i32> {
        // Fate Core skill list with pyramid placeholder
        let skills = [
            "Athletics", "Burglary", "Contacts", "Crafts", "Deceive",
            "Drive", "Empathy", "Fight", "Investigate", "Lore",
            "Notice", "Physique", "Provoke", "Rapport", "Resources",
            "Shoot", "Stealth", "Will",
        ];
        skills.iter().map(|s| (s.to_string(), 0)).collect()
    }

    fn random_high_concept(rng: &mut impl Rng) -> String {
        let concepts = [
            "Retired Special Forces Operative",
            "Curious Academic Occultist",
            "Reformed Cat Burglar",
            "Reckless Stunt Driver",
            "Hard-Boiled Private Eye",
            "Wandering Sword Master",
            "Charismatic Con Artist",
            "Reluctant Chosen One",
            "Cynical War Veteran",
            "Idealistic Young Mage",
        ];
        concepts[rng.gen_range(0..concepts.len())].to_string()
    }

    fn random_trouble(rng: &mut impl Rng) -> String {
        let troubles = [
            "The Past Always Catches Up",
            "Can't Resist a Puzzle",
            "Old Enemies, New Problems",
            "My Word Is My Bond",
            "Short Fuse, Big Bang",
            "Haunted by Ghosts",
            "Debt to the Wrong People",
            "Trust Issues",
            "Glory Seeker",
            "The Weight of Responsibility",
        ];
        troubles[rng.gen_range(0..troubles.len())].to_string()
    }

    fn generate_aspects(options: &GenerationOptions, rng: &mut impl Rng) -> Vec<CharacterTrait> {
        let high_concept = options.concept.clone()
            .unwrap_or_else(|| Self::random_high_concept(rng));

        let trouble = Self::random_trouble(rng);

        vec![
            CharacterTrait {
                name: "High Concept".to_string(),
                trait_type: TraitType::Aspect,
                description: high_concept,
                mechanical_effect: Some("Invoke for +2 or reroll, compel for fate point".to_string()),
            },
            CharacterTrait {
                name: "Trouble".to_string(),
                trait_type: TraitType::Aspect,
                description: trouble,
                mechanical_effect: Some("Compel for complications, earn fate points".to_string()),
            },
            CharacterTrait {
                name: "Background Aspect".to_string(),
                trait_type: TraitType::Aspect,
                description: "Where you came from shapes who you are".to_string(),
                mechanical_effect: Some("Define during play".to_string()),
            },
            CharacterTrait {
                name: "Rising Conflict Aspect".to_string(),
                trait_type: TraitType::Aspect,
                description: "Your current story arc".to_string(),
                mechanical_effect: Some("Define during play".to_string()),
            },
            CharacterTrait {
                name: "Guest Star Aspect".to_string(),
                trait_type: TraitType::Aspect,
                description: "Connection to another PC".to_string(),
                mechanical_effect: Some("Define during play".to_string()),
            },
        ]
    }

    fn generate_stunts(rng: &mut impl Rng) -> Vec<CharacterTrait> {
        let stunt_templates = [
            ("Combat Veteran", "Because I'm a Combat Veteran, I get +2 to Fight when defending against multiple opponents"),
            ("Quick Reflexes", "Because of my Quick Reflexes, I get +2 to Quick when avoiding sudden danger"),
            ("Silver Tongue", "Because I have a Silver Tongue, I get +2 to Deceive when first meeting someone"),
            ("Academic Expert", "Because I'm an Academic Expert, I get +2 to Lore when researching occult topics"),
            ("Street Smart", "Because I'm Street Smart, I get +2 to Contacts when seeking information in urban areas"),
            ("Danger Sense", "Because of my Danger Sense, I can use Notice to defend against ambushes"),
            ("Tough as Nails", "Because I'm Tough as Nails, once per session I can reduce a physical consequence by one severity"),
        ];

        // Pick 3 random stunts
        let mut indices: Vec<usize> = (0..stunt_templates.len()).collect();
        for i in (1..indices.len()).rev() {
            let j = rng.gen_range(0..=i);
            indices.swap(i, j);
        }

        indices.iter().take(3).map(|&i| {
            let (name, desc) = stunt_templates[i];
            CharacterTrait {
                name: name.to_string(),
                trait_type: TraitType::Stunt,
                description: desc.to_string(),
                mechanical_effect: Some("+2 bonus or special effect".to_string()),
            }
        }).collect()
    }
}

impl Default for FateCoreGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemGenerator for FateCoreGenerator {
    fn system(&self) -> GameSystem {
        GameSystem::FateCore
    }

    fn generate(&self, options: &GenerationOptions) -> Result<Character> {
        let mut rng = rand::thread_rng();

        let name = options.name.clone()
            .unwrap_or_else(|| {
                if options.theme.as_deref() == Some("fantasy") {
                    random_fantasy_name(&mut rng)
                } else {
                    random_modern_name(&mut rng)
                }
            });

        let attributes = Self::generate_approaches(&mut rng);
        let skills = Self::generate_skills();

        let mut traits = Self::generate_aspects(options, &mut rng);
        traits.extend(Self::generate_stunts(&mut rng));

        let background = CharacterBackground {
            origin: options.background.clone().unwrap_or_else(|| "Unknown".to_string()),
            occupation: options.class.clone(),
            motivation: "Discover your story".to_string(),
            connections: vec![],
            secrets: vec![],
            history: String::new(),
        };

        // Fate doesn't really have equipment in the traditional sense
        let equipment = vec![];

        Ok(Character {
            id: Uuid::new_v4().to_string(),
            name,
            system: GameSystem::FateCore,
            concept: traits.first()
                .map(|t| t.description.clone())
                .unwrap_or_else(|| "Fate Character".to_string()),
            race: None,
            class: None,
            level: 1,
            attributes,
            skills,
            traits,
            equipment,
            background,
            backstory: None,
            notes: "Fate Points: 3\nRefresh: 3\nStress: [1][2][3]".to_string(),
            portrait_prompt: None,
        })
    }

    fn available_races(&self) -> Vec<String> {
        // Fate is genre-agnostic, no fixed races
        vec!["Any".to_string()]
    }

    fn available_classes(&self) -> Vec<String> {
        // Fate uses High Concepts instead of classes
        vec![
            "Warrior".to_string(),
            "Mage".to_string(),
            "Rogue".to_string(),
            "Scholar".to_string(),
            "Diplomat".to_string(),
            "Explorer".to_string(),
        ]
    }

    fn available_backgrounds(&self) -> Vec<String> {
        vec![
            "Noble".to_string(),
            "Street Urchin".to_string(),
            "Academic".to_string(),
            "Military".to_string(),
            "Criminal".to_string(),
            "Merchant".to_string(),
            "Traveler".to_string(),
        ]
    }

    fn attribute_names(&self) -> Vec<String> {
        // FAE Approaches
        vec![
            "Careful".to_string(),
            "Clever".to_string(),
            "Flashy".to_string(),
            "Forceful".to_string(),
            "Quick".to_string(),
            "Sneaky".to_string(),
        ]
    }

    fn starting_equipment(&self, _class: Option<&str>) -> Vec<Equipment> {
        // Fate doesn't track equipment mechanically
        vec![
            Equipment {
                name: "Signature Item".to_string(),
                category: EquipmentCategory::Other,
                description: "An item that defines your character".to_string(),
                stats: [("Aspect".to_string(), "Can be invoked".to_string())].into(),
            },
        ]
    }
}
