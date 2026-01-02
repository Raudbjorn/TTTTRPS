//! Warhammer Fantasy Roleplay Character Generator
//!
//! Generates characters for WFRP 4th Edition - grimdark fantasy in the Old World.

use crate::core::character_gen::{
    SystemGenerator, Character, GameSystem, GenerationOptions,
    AttributeValue, CharacterTrait, TraitType, Equipment, EquipmentCategory,
    CharacterBackground, Result, random_fantasy_name,
};
use rand::Rng;
use uuid::Uuid;
use std::collections::HashMap;

pub struct WarhammerGenerator;

impl WarhammerGenerator {
    pub fn new() -> Self {
        Self
    }

    fn roll_characteristics(rng: &mut impl Rng, species: &str) -> HashMap<String, AttributeValue> {
        // Base roll is 2d10+20 for most characteristics
        let chars = ["WS", "BS", "S", "T", "I", "Ag", "Dex", "Int", "WP", "Fel"];

        let mut attrs: HashMap<String, AttributeValue> = chars.iter().map(|&c| {
            let roll: i32 = rng.gen_range(1..=10) + rng.gen_range(1..=10) + 20;
            (c.to_string(), AttributeValue::new_raw(roll))
        }).collect();

        // Species modifiers
        match species.to_lowercase().as_str() {
            "dwarf" => {
                attrs.entry("WS".to_string()).and_modify(|a| a.base += 10);
                attrs.entry("T".to_string()).and_modify(|a| a.base += 10);
                attrs.entry("WP".to_string()).and_modify(|a| a.base += 20);
                attrs.entry("Ag".to_string()).and_modify(|a| a.base -= 10);
                attrs.entry("Fel".to_string()).and_modify(|a| a.base -= 10);
            }
            "halfling" => {
                attrs.entry("BS".to_string()).and_modify(|a| a.base += 10);
                attrs.entry("Ag".to_string()).and_modify(|a| a.base += 10);
                attrs.entry("Fel".to_string()).and_modify(|a| a.base += 10);
                attrs.entry("WS".to_string()).and_modify(|a| a.base -= 10);
                attrs.entry("S".to_string()).and_modify(|a| a.base -= 10);
            }
            "high elf" | "wood elf" | "elf" => {
                attrs.entry("WS".to_string()).and_modify(|a| a.base += 10);
                attrs.entry("BS".to_string()).and_modify(|a| a.base += 10);
                attrs.entry("I".to_string()).and_modify(|a| a.base += 20);
                attrs.entry("Ag".to_string()).and_modify(|a| a.base += 10);
                attrs.entry("Dex".to_string()).and_modify(|a| a.base += 10);
                // Elves have lower starting wounds
            }
            _ => {} // Human - no modifiers
        }

        attrs
    }

    fn get_skills() -> HashMap<String, i32> {
        let skills = [
            "Athletics", "Bribery", "Charm", "Climb", "Cool",
            "Consume Alcohol", "Dodge", "Drive", "Endurance", "Entertain",
            "Gamble", "Gossip", "Haggle", "Intimidate", "Intuition",
            "Leadership", "Lore (Various)", "Melee (Basic)", "Melee (Brawling)",
            "Navigation", "Outdoor Survival", "Perception", "Ranged (Bow)",
            "Ranged (Crossbow)", "Ride (Horse)", "Row", "Stealth (Rural)",
            "Stealth (Urban)", "Swim", "Track",
        ];
        skills.iter().map(|s| (s.to_string(), 0)).collect()
    }

    fn random_career(rng: &mut impl Rng) -> String {
        let careers = [
            // Human careers
            "Apothecary", "Beggar", "Boatman", "Bounty Hunter", "Coachman",
            "Entertainer", "Envoy", "Flagellant", "Grave Robber", "Hedge Witch",
            "Herbalist", "Huntsman", "Lawyer", "Messenger", "Miner",
            "Noble", "Nun/Monk", "Outlaw", "Peasant", "Physician",
            "Pit Fighter", "Rat Catcher", "Riverwoman", "Road Warden",
            "Scholar", "Scribe", "Servant", "Soldier", "Spy",
            "Thief", "Village Elder", "Watchman", "Witch Hunter",
        ];
        careers[rng.gen_range(0..careers.len())].to_string()
    }

    fn get_career_talents(career: &str) -> Vec<CharacterTrait> {
        match career.to_lowercase().as_str() {
            "soldier" | "pit fighter" => vec![
                CharacterTrait {
                    name: "Combat Reflexes".to_string(),
                    trait_type: TraitType::Talent,
                    description: "+10 to Initiative".to_string(),
                    mechanical_effect: Some("+10 Initiative".to_string()),
                },
                CharacterTrait {
                    name: "Strike Mighty Blow".to_string(),
                    trait_type: TraitType::Talent,
                    description: "+SB to melee damage".to_string(),
                    mechanical_effect: Some("+SB damage".to_string()),
                },
            ],
            "thief" | "grave robber" | "spy" => vec![
                CharacterTrait {
                    name: "Flee!".to_string(),
                    trait_type: TraitType::Talent,
                    description: "+1 Movement when running away".to_string(),
                    mechanical_effect: Some("+1 Move fleeing".to_string()),
                },
                CharacterTrait {
                    name: "Criminal".to_string(),
                    trait_type: TraitType::Talent,
                    description: "Know the underworld".to_string(),
                    mechanical_effect: Some("Access to criminal contacts".to_string()),
                },
            ],
            "witch hunter" | "road warden" | "bounty hunter" => vec![
                CharacterTrait {
                    name: "Coolheaded".to_string(),
                    trait_type: TraitType::Talent,
                    description: "+5 Willpower".to_string(),
                    mechanical_effect: Some("+5 WP".to_string()),
                },
                CharacterTrait {
                    name: "Marksman".to_string(),
                    trait_type: TraitType::Talent,
                    description: "+5 Ballistic Skill".to_string(),
                    mechanical_effect: Some("+5 BS".to_string()),
                },
            ],
            "scholar" | "scribe" | "lawyer" => vec![
                CharacterTrait {
                    name: "Read/Write".to_string(),
                    trait_type: TraitType::Talent,
                    description: "Can read and write".to_string(),
                    mechanical_effect: Some("Literacy".to_string()),
                },
                CharacterTrait {
                    name: "Savant".to_string(),
                    trait_type: TraitType::Talent,
                    description: "+10 to one Lore skill".to_string(),
                    mechanical_effect: Some("+10 Lore".to_string()),
                },
            ],
            "physician" | "apothecary" | "herbalist" => vec![
                CharacterTrait {
                    name: "Pharmacist".to_string(),
                    trait_type: TraitType::Talent,
                    description: "Can prepare medicines".to_string(),
                    mechanical_effect: Some("Prepare drugs and medicines".to_string()),
                },
                CharacterTrait {
                    name: "Surgery".to_string(),
                    trait_type: TraitType::Talent,
                    description: "Can perform surgery".to_string(),
                    mechanical_effect: Some("Perform operations".to_string()),
                },
            ],
            _ => vec![
                CharacterTrait {
                    name: format!("{} Training", career),
                    trait_type: TraitType::Talent,
                    description: format!("Trained in the {} career.", career),
                    mechanical_effect: None,
                },
            ],
        }
    }

    fn random_species(rng: &mut impl Rng) -> String {
        let roll: i32 = rng.gen_range(1..=100);
        match roll {
            1..=90 => "Human",
            91..=94 => "Halfling",
            95..=98 => "Dwarf",
            _ => "Elf",
        }.to_string()
    }
}

impl Default for WarhammerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemGenerator for WarhammerGenerator {
    fn system(&self) -> GameSystem {
        GameSystem::Warhammer
    }

    fn generate(&self, options: &GenerationOptions) -> Result<Character> {
        let mut rng = rand::thread_rng();

        let name = options.name.clone()
            .unwrap_or_else(|| random_fantasy_name(&mut rng));

        let species = options.race.clone()
            .unwrap_or_else(|| Self::random_species(&mut rng));

        let attributes = Self::roll_characteristics(&mut rng, &species);
        let skills = Self::get_skills();

        let career = options.class.clone()
            .unwrap_or_else(|| Self::random_career(&mut rng));

        let rank = options.level.unwrap_or(1);

        let mut traits = Self::get_career_talents(&career);

        // Add species traits
        let species_trait = match species.to_lowercase().as_str() {
            "dwarf" => CharacterTrait {
                name: "Night Vision".to_string(),
                trait_type: TraitType::Racial,
                description: "Can see in low light".to_string(),
                mechanical_effect: Some("See in low light conditions".to_string()),
            },
            "halfling" => CharacterTrait {
                name: "Small".to_string(),
                trait_type: TraitType::Racial,
                description: "Smaller than humans".to_string(),
                mechanical_effect: Some("-10 to hit in melee, +10 to hide".to_string()),
            },
            "elf" | "high elf" | "wood elf" => CharacterTrait {
                name: "Acute Sense (Sight)".to_string(),
                trait_type: TraitType::Racial,
                description: "Superior eyesight".to_string(),
                mechanical_effect: Some("+20 to sight-based Perception".to_string()),
            },
            _ => CharacterTrait {
                name: "Doomed".to_string(),
                trait_type: TraitType::Racial,
                description: "Humans have a doom - a prophecy of their death".to_string(),
                mechanical_effect: Some("GM determines doom".to_string()),
            },
        };
        traits.push(species_trait);

        let equipment = if options.include_equipment {
            self.starting_equipment(Some(&career))
        } else {
            vec![]
        };

        // Calculate wounds
        let s = attributes.get("S").map(|a| a.base / 10).unwrap_or(3);
        let t = attributes.get("T").map(|a| a.base / 10).unwrap_or(3);
        let wp = attributes.get("WP").map(|a| a.base / 10).unwrap_or(3);
        let wounds = s + 2 * t + wp;

        let background = CharacterBackground {
            origin: "The Empire".to_string(),
            occupation: Some(career.clone()),
            motivation: "Survival in a grim world".to_string(),
            connections: vec![],
            secrets: vec!["Something dark in your past".to_string()],
            history: String::new(),
        };

        Ok(Character {
            id: Uuid::new_v4().to_string(),
            name,
            system: GameSystem::Warhammer,
            concept: options.concept.clone().unwrap_or_else(|| format!("{} {}", species, career)),
            race: Some(species),
            class: Some(career),
            level: rank,
            attributes,
            skills,
            traits,
            equipment,
            background,
            backstory: None,
            notes: format!("Wounds: {}\nCareer Rank: {}\nFate: 2\nResilience: 1", wounds, rank),
            portrait_prompt: None,
        })
    }

    fn available_races(&self) -> Vec<String> {
        vec![
            "Human".to_string(),
            "Dwarf".to_string(),
            "Halfling".to_string(),
            "High Elf".to_string(),
            "Wood Elf".to_string(),
        ]
    }

    fn available_classes(&self) -> Vec<String> {
        vec![
            "Apothecary".to_string(),
            "Bounty Hunter".to_string(),
            "Entertainer".to_string(),
            "Grave Robber".to_string(),
            "Herbalist".to_string(),
            "Huntsman".to_string(),
            "Messenger".to_string(),
            "Noble".to_string(),
            "Outlaw".to_string(),
            "Peasant".to_string(),
            "Physician".to_string(),
            "Pit Fighter".to_string(),
            "Rat Catcher".to_string(),
            "Road Warden".to_string(),
            "Scholar".to_string(),
            "Soldier".to_string(),
            "Thief".to_string(),
            "Watchman".to_string(),
            "Witch Hunter".to_string(),
        ]
    }

    fn available_backgrounds(&self) -> Vec<String> {
        vec![
            "Reiklander".to_string(),
            "Middenheimer".to_string(),
            "Averlander".to_string(),
            "Stirlander".to_string(),
            "Nordlander".to_string(),
            "Ostermarker".to_string(),
            "Wissenlander".to_string(),
            "Talabeclander".to_string(),
        ]
    }

    fn attribute_names(&self) -> Vec<String> {
        vec![
            "WS".to_string(), "BS".to_string(), "S".to_string(), "T".to_string(),
            "I".to_string(), "Ag".to_string(), "Dex".to_string(), "Int".to_string(),
            "WP".to_string(), "Fel".to_string(),
        ]
    }

    fn starting_equipment(&self, career: Option<&str>) -> Vec<Equipment> {
        let mut equipment = vec![
            Equipment {
                name: "Common Clothes".to_string(),
                category: EquipmentCategory::Other,
                description: "Simple, worn clothing".to_string(),
                stats: HashMap::new(),
            },
            Equipment {
                name: "Pouch".to_string(),
                category: EquipmentCategory::Tool,
                description: "For carrying coins and small items".to_string(),
                stats: HashMap::new(),
            },
        ];

        match career.map(|s| s.to_lowercase()).as_deref() {
            Some("soldier") | Some("road warden") | Some("pit fighter") => {
                equipment.push(Equipment {
                    name: "Hand Weapon".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Sword or axe".to_string(),
                    stats: [("Damage".to_string(), "+SB".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Shield".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "Wooden shield".to_string(),
                    stats: [("AP".to_string(), "1".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Leather Jack".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "Light leather armor".to_string(),
                    stats: [("AP".to_string(), "1".to_string())].into(),
                });
            }
            Some("thief") | Some("grave robber") => {
                equipment.push(Equipment {
                    name: "Dagger".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Concealable blade".to_string(),
                    stats: [("Damage".to_string(), "+SB-2".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Lockpicks".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "For opening locks".to_string(),
                    stats: HashMap::new(),
                });
            }
            Some("witch hunter") | Some("bounty hunter") => {
                equipment.push(Equipment {
                    name: "Crossbow".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Ranged weapon".to_string(),
                    stats: [("Damage".to_string(), "+8".to_string()), ("Range".to_string(), "60".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Rapier".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Elegant blade".to_string(),
                    stats: [("Damage".to_string(), "+SB".to_string()), ("Quality".to_string(), "Fast".to_string())].into(),
                });
            }
            Some("rat catcher") => {
                equipment.push(Equipment {
                    name: "Small But Vicious Dog".to_string(),
                    category: EquipmentCategory::Other,
                    description: "Your faithful companion".to_string(),
                    stats: HashMap::new(),
                });
                equipment.push(Equipment {
                    name: "Sling".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "For hunting rats".to_string(),
                    stats: [("Damage".to_string(), "+4".to_string())].into(),
                });
            }
            _ => {
                equipment.push(Equipment {
                    name: "Knife".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Basic knife".to_string(),
                    stats: [("Damage".to_string(), "+SB-3".to_string())].into(),
                });
            }
        }

        equipment
    }
}
