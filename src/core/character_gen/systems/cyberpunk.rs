//! Cyberpunk Red Character Generator
//!
//! Generates Edgerunners for Cyberpunk Red.

use crate::core::character_gen::{
    SystemGenerator, Character, GameSystem, GenerationOptions,
    AttributeValue, CharacterTrait, TraitType, Equipment, EquipmentCategory,
    CharacterBackground, Result, random_cyberpunk_handle,
};
use rand::Rng;
use uuid::Uuid;
use std::collections::HashMap;

pub struct CyberpunkGenerator;

impl CyberpunkGenerator {
    pub fn new() -> Self {
        Self
    }

    fn roll_stats(rng: &mut impl Rng) -> HashMap<String, AttributeValue> {
        let attrs = ["INT", "REF", "DEX", "TECH", "COOL", "WILL", "LUCK", "MOVE", "BODY", "EMP"];
        attrs.iter().map(|&attr| {
            let value = rng.gen_range(2..=8);
            (attr.to_string(), AttributeValue::new_raw(value))
        }).collect()
    }

    fn get_skills() -> HashMap<String, i32> {
        let skills = [
            "Athletics", "Brawling", "Concentration", "Conversation",
            "Education", "Evasion", "First Aid", "Human Perception",
            "Interface", "Stealth", "Handgun", "Melee", "Streetwise",
            "Perception", "Tracking", "Driving", "Shoulder Arms",
            "Cybertech", "Electronics", "Weaponstech", "Persuasion",
        ];
        skills.iter().map(|s| (s.to_string(), 2)).collect()
    }

    fn random_role(rng: &mut impl Rng) -> String {
        let roles = [
            "Solo", "Netrunner", "Tech", "Media", "Exec",
            "Lawman", "Fixer", "Nomad", "Rockerboy", "Medtech",
        ];
        roles[rng.gen_range(0..roles.len())].to_string()
    }

    fn get_role_ability(role: &str) -> CharacterTrait {
        let (name, desc, effect) = match role.to_lowercase().as_str() {
            "solo" => ("Combat Awareness", "Enhanced combat senses", "+2 Initiative per rank"),
            "netrunner" => ("Interface", "Jack into the Net", "Access NET architecture"),
            "tech" => ("Maker", "Create and repair tech", "Can craft/repair items"),
            "media" => ("Credibility", "Influence through media", "Sway public opinion"),
            "exec" => ("Teamwork", "Corporate connections", "Access to corp resources"),
            "lawman" => ("Backup", "Call for support", "Summon backup officers"),
            "fixer" => ("Operator", "Street connections", "Find items and info"),
            "nomad" => ("Moto", "Vehicle expertise", "Access to nomad vehicles"),
            "rockerboy" => ("Charismatic Impact", "Move crowds", "Influence audiences"),
            "medtech" => ("Medicine", "Advanced healing", "Perform medical procedures"),
            _ => ("Role Ability", "Special role power", "See rulebook"),
        };

        CharacterTrait {
            name: name.to_string(),
            trait_type: TraitType::Class,
            description: desc.to_string(),
            mechanical_effect: Some(effect.to_string()),
        }
    }
}

impl Default for CyberpunkGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemGenerator for CyberpunkGenerator {
    fn system(&self) -> GameSystem {
        GameSystem::Cyberpunk
    }

    fn generate(&self, options: &GenerationOptions) -> Result<Character> {
        let mut rng = rand::thread_rng();

        let name = options.name.clone()
            .unwrap_or_else(|| random_cyberpunk_handle(&mut rng));

        let attributes = Self::roll_stats(&mut rng);
        let skills = Self::get_skills();

        let role = options.class.clone()
            .unwrap_or_else(|| Self::random_role(&mut rng));

        let mut traits = vec![Self::get_role_ability(&role)];

        // Add basic cyberware
        traits.push(CharacterTrait {
            name: "Neural Interface".to_string(),
            trait_type: TraitType::Cyberware,
            description: "Basic neural processor for interfacing with tech".to_string(),
            mechanical_effect: Some("Required for most cyberware".to_string()),
        });

        if role.to_lowercase() == "netrunner" {
            traits.push(CharacterTrait {
                name: "Cyberdeck".to_string(),
                trait_type: TraitType::Cyberware,
                description: "Personal NET running device".to_string(),
                mechanical_effect: Some("Required for netrunning".to_string()),
            });
        }

        let equipment = if options.include_equipment {
            self.starting_equipment(Some(&role))
        } else {
            vec![]
        };

        let background = CharacterBackground {
            origin: "Night City".to_string(),
            occupation: Some(role.clone()),
            motivation: "Make it to the top".to_string(),
            connections: vec!["Fixer contact".to_string(), "Street crew".to_string()],
            secrets: vec!["Corporate ties".to_string()],
            history: String::new(),
        };

        Ok(Character {
            id: Uuid::new_v4().to_string(),
            name,
            system: GameSystem::Cyberpunk,
            concept: options.concept.clone().unwrap_or_else(|| format!("{} Edgerunner", role)),
            race: None,
            class: Some(role),
            level: 1,
            attributes,
            skills,
            traits,
            equipment,
            background,
            backstory: None,
            notes: "Humanity: 40\nEurodollars: 2550".to_string(),
            portrait_prompt: None,
        })
    }

    fn available_races(&self) -> Vec<String> {
        vec!["Human".to_string()]
    }

    fn available_classes(&self) -> Vec<String> {
        vec![
            "Solo".to_string(),
            "Netrunner".to_string(),
            "Tech".to_string(),
            "Medtech".to_string(),
            "Media".to_string(),
            "Exec".to_string(),
            "Lawman".to_string(),
            "Fixer".to_string(),
            "Nomad".to_string(),
            "Rockerboy".to_string(),
        ]
    }

    fn available_backgrounds(&self) -> Vec<String> {
        vec![
            "Corporate".to_string(),
            "Street Kid".to_string(),
            "Nomad".to_string(),
            "Corporate Exile".to_string(),
            "Gang Member".to_string(),
            "Military Veteran".to_string(),
        ]
    }

    fn attribute_names(&self) -> Vec<String> {
        vec![
            "INT".to_string(), "REF".to_string(), "DEX".to_string(),
            "TECH".to_string(), "COOL".to_string(), "WILL".to_string(),
            "LUCK".to_string(), "MOVE".to_string(), "BODY".to_string(),
            "EMP".to_string(),
        ]
    }

    fn starting_equipment(&self, role: Option<&str>) -> Vec<Equipment> {
        let mut equipment = vec![
            Equipment {
                name: "Agent".to_string(),
                category: EquipmentCategory::Tech,
                description: "Personal smartphone/computer".to_string(),
                stats: HashMap::new(),
            },
            Equipment {
                name: "Flashlight".to_string(),
                category: EquipmentCategory::Tool,
                description: "Handheld light source".to_string(),
                stats: HashMap::new(),
            },
        ];

        match role.map(|s| s.to_lowercase()).as_deref() {
            Some("solo") => {
                equipment.push(Equipment {
                    name: "Heavy Pistol".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "High-powered sidearm".to_string(),
                    stats: [("Damage".to_string(), "3d6".to_string()), ("ROF".to_string(), "2".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Light Armorjack".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "Light body armor".to_string(),
                    stats: [("SP".to_string(), "11".to_string())].into(),
                });
            }
            Some("netrunner") => {
                equipment.push(Equipment {
                    name: "Medium Pistol".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Standard sidearm".to_string(),
                    stats: [("Damage".to_string(), "2d6".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Cyberdeck".to_string(),
                    category: EquipmentCategory::Tech,
                    description: "Netrunning hardware".to_string(),
                    stats: [("Slots".to_string(), "7".to_string())].into(),
                });
            }
            Some("tech") | Some("medtech") => {
                equipment.push(Equipment {
                    name: "Tech Toolkit".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "Complete repair toolkit".to_string(),
                    stats: HashMap::new(),
                });
                equipment.push(Equipment {
                    name: "Medium Pistol".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Standard sidearm".to_string(),
                    stats: [("Damage".to_string(), "2d6".to_string())].into(),
                });
            }
            _ => {
                equipment.push(Equipment {
                    name: "Light Pistol".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Basic sidearm".to_string(),
                    stats: [("Damage".to_string(), "1d6".to_string())].into(),
                });
            }
        }

        equipment
    }
}
