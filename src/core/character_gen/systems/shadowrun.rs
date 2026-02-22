//! Shadowrun 6th Edition Character Generator
//!
//! Generates shadowrunners for Shadowrun 6e - where magic meets megacorporations.

use crate::core::character_gen::{
    SystemGenerator, Character, GameSystem, GenerationOptions,
    AttributeValue, CharacterTrait, TraitType, Equipment, EquipmentCategory,
    CharacterBackground, Result, random_cyberpunk_handle,
};
use rand::Rng;
use uuid::Uuid;
use std::collections::HashMap;

pub struct ShadowrunGenerator;

impl ShadowrunGenerator {
    pub fn new() -> Self {
        Self
    }

    fn roll_attributes(rng: &mut impl Rng, metatype: &str) -> HashMap<String, AttributeValue> {
        let base_attrs = ["Body", "Agility", "Reaction", "Strength", "Willpower", "Logic", "Intuition", "Charisma", "Edge"];

        // Metatype adjustments (simplified)
        let (str_mod, body_mod, agi_mod, cha_mod) = match metatype.to_lowercase().as_str() {
            "ork" => (2, 1, 0, -1),
            "troll" => (4, 2, -1, -2),
            "elf" => (0, 0, 1, 1),
            "dwarf" => (1, 1, 0, 0),
            _ => (0, 0, 0, 0), // Human
        };

        let mods: HashMap<&str, i32> = [
            ("Strength", str_mod), ("Body", body_mod),
            ("Agility", agi_mod), ("Charisma", cha_mod),
        ].into();

        base_attrs.iter().map(|&attr| {
            let base = rng.gen_range(1..=6);
            let modifier = *mods.get(attr).unwrap_or(&0);
            (attr.to_string(), AttributeValue::new_raw((base + modifier).max(1)))
        }).collect()
    }

    fn get_skills() -> HashMap<String, i32> {
        let skills = [
            "Athletics", "Biotech", "Close Combat", "Con", "Conjuring",
            "Cracking", "Electronics", "Enchanting", "Engineering", "Exotic Weapons",
            "Firearms", "Influence", "Outdoors", "Perception", "Piloting",
            "Sorcery", "Stealth", "Tasking",
        ];
        skills.iter().map(|s| (s.to_string(), 0)).collect()
    }

    fn random_archetype(rng: &mut impl Rng) -> String {
        let archetypes = [
            "Street Samurai", "Decker", "Rigger", "Mage", "Adept",
            "Face", "Technomancer", "Shaman", "Infiltrator", "Combat Medic",
        ];
        archetypes[rng.gen_range(0..archetypes.len())].to_string()
    }

    fn get_archetype_traits(archetype: &str) -> Vec<CharacterTrait> {
        match archetype.to_lowercase().as_str() {
            "street samurai" => vec![
                CharacterTrait {
                    name: "Wired Reflexes".to_string(),
                    trait_type: TraitType::Cyberware,
                    description: "Enhanced reaction time".to_string(),
                    mechanical_effect: Some("+1 Reaction, +1d6 Initiative".to_string()),
                },
                CharacterTrait {
                    name: "Combat Specialist".to_string(),
                    trait_type: TraitType::Class,
                    description: "Trained in various combat techniques".to_string(),
                    mechanical_effect: Some("Combat pool bonus".to_string()),
                },
            ],
            "decker" => vec![
                CharacterTrait {
                    name: "Cyberdeck".to_string(),
                    trait_type: TraitType::Cyberware,
                    description: "Matrix interface device".to_string(),
                    mechanical_effect: Some("Can enter the Matrix".to_string()),
                },
                CharacterTrait {
                    name: "Matrix Initiative".to_string(),
                    trait_type: TraitType::Class,
                    description: "Enhanced digital reflexes".to_string(),
                    mechanical_effect: Some("Matrix Initiative = Data Processing + Intuition".to_string()),
                },
            ],
            "mage" | "shaman" => vec![
                CharacterTrait {
                    name: "Spellcasting".to_string(),
                    trait_type: TraitType::Class,
                    description: "Awakened magic user".to_string(),
                    mechanical_effect: Some("Can cast spells".to_string()),
                },
                CharacterTrait {
                    name: "Astral Perception".to_string(),
                    trait_type: TraitType::Class,
                    description: "See the astral plane".to_string(),
                    mechanical_effect: Some("Can perceive astral space".to_string()),
                },
            ],
            "adept" => vec![
                CharacterTrait {
                    name: "Adept Powers".to_string(),
                    trait_type: TraitType::Class,
                    description: "Channel magic through body".to_string(),
                    mechanical_effect: Some("Magic rating in power points".to_string()),
                },
            ],
            "technomancer" => vec![
                CharacterTrait {
                    name: "Living Persona".to_string(),
                    trait_type: TraitType::Class,
                    description: "Natural Matrix connection".to_string(),
                    mechanical_effect: Some("No deck required".to_string()),
                },
                CharacterTrait {
                    name: "Sprites".to_string(),
                    trait_type: TraitType::Class,
                    description: "Compile digital entities".to_string(),
                    mechanical_effect: Some("Can compile sprites".to_string()),
                },
            ],
            "rigger" => vec![
                CharacterTrait {
                    name: "Control Rig".to_string(),
                    trait_type: TraitType::Cyberware,
                    description: "Direct vehicle/drone control".to_string(),
                    mechanical_effect: Some("Jump into vehicles".to_string()),
                },
            ],
            "face" => vec![
                CharacterTrait {
                    name: "Silver Tongue".to_string(),
                    trait_type: TraitType::Class,
                    description: "Master of social manipulation".to_string(),
                    mechanical_effect: Some("Social limit bonus".to_string()),
                },
            ],
            _ => vec![CharacterTrait {
                name: archetype.to_string(),
                trait_type: TraitType::Class,
                description: format!("Trained as a {}", archetype),
                mechanical_effect: None,
            }],
        }
    }
}

impl Default for ShadowrunGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemGenerator for ShadowrunGenerator {
    fn system(&self) -> GameSystem {
        GameSystem::Shadowrun
    }

    fn generate(&self, options: &GenerationOptions) -> Result<Character> {
        let mut rng = rand::thread_rng();

        let name = options.name.clone()
            .unwrap_or_else(|| random_cyberpunk_handle(&mut rng));

        let metatype = options.race.clone().unwrap_or_else(|| "Human".to_string());
        let attributes = Self::roll_attributes(&mut rng, &metatype);
        let skills = Self::get_skills();

        let archetype = options.class.clone()
            .unwrap_or_else(|| Self::random_archetype(&mut rng));

        let mut traits = Self::get_archetype_traits(&archetype);

        // Add metatype trait
        traits.push(CharacterTrait {
            name: metatype.clone(),
            trait_type: TraitType::Racial,
            description: format!("{} metatype traits", metatype),
            mechanical_effect: Some(match metatype.to_lowercase().as_str() {
                "ork" => "Low-Light Vision, +2 STR, +1 BODY".to_string(),
                "troll" => "Thermographic Vision, +4 STR, +2 BODY, Dermal Deposits".to_string(),
                "elf" => "Low-Light Vision, +1 AGI, +1 CHA".to_string(),
                "dwarf" => "Thermographic Vision, +1 STR, +1 BODY, +2 to resist toxins".to_string(),
                _ => "No special abilities".to_string(),
            }),
        });

        let equipment = if options.include_equipment {
            self.starting_equipment(Some(&archetype))
        } else {
            vec![]
        };

        let background = CharacterBackground {
            origin: "Seattle Sprawl".to_string(),
            occupation: Some(archetype.clone()),
            motivation: "Survive the shadows".to_string(),
            connections: vec!["Fixer".to_string(), "Street contact".to_string()],
            secrets: vec!["Corp SIN".to_string()],
            history: String::new(),
        };

        // Calculate essence (assuming some cyberware)
        let essence = if archetype.to_lowercase().contains("samurai") || archetype.to_lowercase().contains("decker") || archetype.to_lowercase().contains("rigger") {
            4.0
        } else {
            6.0
        };

        Ok(Character {
            id: Uuid::new_v4().to_string(),
            name,
            system: GameSystem::Shadowrun,
            concept: options.concept.clone().unwrap_or_else(|| format!("{} {}", metatype, archetype)),
            race: Some(metatype),
            class: Some(archetype),
            level: 1,
            attributes,
            skills,
            traits,
            equipment,
            background,
            backstory: None,
            notes: format!("Essence: {}\nNuyen: 6000", essence),
            portrait_prompt: None,
        })
    }

    fn available_races(&self) -> Vec<String> {
        vec![
            "Human".to_string(),
            "Elf".to_string(),
            "Dwarf".to_string(),
            "Ork".to_string(),
            "Troll".to_string(),
        ]
    }

    fn available_classes(&self) -> Vec<String> {
        vec![
            "Street Samurai".to_string(),
            "Decker".to_string(),
            "Rigger".to_string(),
            "Mage".to_string(),
            "Shaman".to_string(),
            "Adept".to_string(),
            "Technomancer".to_string(),
            "Face".to_string(),
            "Infiltrator".to_string(),
            "Combat Medic".to_string(),
        ]
    }

    fn available_backgrounds(&self) -> Vec<String> {
        vec![
            "Corporate SINner".to_string(),
            "Street Kid".to_string(),
            "Tribal".to_string(),
            "Military Veteran".to_string(),
            "Ganger".to_string(),
            "Wage Slave".to_string(),
        ]
    }

    fn attribute_names(&self) -> Vec<String> {
        vec![
            "Body".to_string(), "Agility".to_string(), "Reaction".to_string(),
            "Strength".to_string(), "Willpower".to_string(), "Logic".to_string(),
            "Intuition".to_string(), "Charisma".to_string(), "Edge".to_string(),
        ]
    }

    fn starting_equipment(&self, archetype: Option<&str>) -> Vec<Equipment> {
        let mut equipment = vec![
            Equipment {
                name: "Commlink".to_string(),
                category: EquipmentCategory::Tech,
                description: "AR-enabled smartphone".to_string(),
                stats: HashMap::new(),
            },
        ];

        match archetype.map(|s| s.to_lowercase()).as_deref() {
            Some("street samurai") => {
                equipment.push(Equipment {
                    name: "Ares Predator VI".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Heavy pistol, standard street chrome".to_string(),
                    stats: [("DV".to_string(), "3P".to_string()), ("AR".to_string(), "10/10/8/-/-".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Katana".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Traditional blade".to_string(),
                    stats: [("DV".to_string(), "4P".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Armor Jacket".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "Armored street wear".to_string(),
                    stats: [("Defense".to_string(), "4".to_string())].into(),
                });
            }
            Some("decker") => {
                equipment.push(Equipment {
                    name: "Cyberdeck".to_string(),
                    category: EquipmentCategory::Tech,
                    description: "Matrix interface".to_string(),
                    stats: [("Attack".to_string(), "3".to_string()), ("Sleaze".to_string(), "4".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Light Pistol".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Backup weapon".to_string(),
                    stats: [("DV".to_string(), "2P".to_string())].into(),
                });
            }
            Some("mage") | Some("shaman") => {
                equipment.push(Equipment {
                    name: "Spell Focus".to_string(),
                    category: EquipmentCategory::Magic,
                    description: "Magical focus item".to_string(),
                    stats: [("Rating".to_string(), "2".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Reagents".to_string(),
                    category: EquipmentCategory::Consumable,
                    description: "Magical components".to_string(),
                    stats: [("Drams".to_string(), "10".to_string())].into(),
                });
            }
            Some("rigger") => {
                equipment.push(Equipment {
                    name: "MCT Rotodrone".to_string(),
                    category: EquipmentCategory::Vehicle,
                    description: "Combat drone".to_string(),
                    stats: [("Handling".to_string(), "4".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Control Rig".to_string(),
                    category: EquipmentCategory::Tech,
                    description: "Cyberware for vehicle control".to_string(),
                    stats: [("Rating".to_string(), "1".to_string())].into(),
                });
            }
            _ => {
                equipment.push(Equipment {
                    name: "Light Pistol".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Basic sidearm".to_string(),
                    stats: [("DV".to_string(), "2P".to_string())].into(),
                });
            }
        }

        equipment
    }
}
