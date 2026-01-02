//! Dungeon World Character Generator
//!
//! Generates characters for Dungeon World (Powered by the Apocalypse).

use crate::core::character_gen::{
    SystemGenerator, Character, GameSystem, GenerationOptions,
    AttributeValue, CharacterTrait, TraitType, Equipment, EquipmentCategory,
    CharacterBackground, Result, random_fantasy_name,
};
use rand::Rng;
use uuid::Uuid;
use std::collections::HashMap;

pub struct DungeonWorldGenerator;

impl DungeonWorldGenerator {
    pub fn new() -> Self {
        Self
    }

    fn generate_stats(rng: &mut impl Rng) -> HashMap<String, AttributeValue> {
        // Standard array: 16, 15, 13, 12, 9, 8
        let mut values = vec![16, 15, 13, 12, 9, 8];
        let stats = ["STR", "DEX", "CON", "INT", "WIS", "CHA"];

        // Shuffle
        for i in (1..6).rev() {
            let j = rng.gen_range(0..=i);
            values.swap(i, j);
        }

        stats.iter().zip(values.iter()).map(|(&stat, &val)| {
            let modifier = Self::stat_modifier(val);
            (stat.to_string(), AttributeValue { base: val, modifier, temp_bonus: 0 })
        }).collect()
    }

    fn stat_modifier(stat: i32) -> i32 {
        match stat {
            1..=3 => -3,
            4..=5 => -2,
            6..=8 => -1,
            9..=12 => 0,
            13..=15 => 1,
            16..=17 => 2,
            _ => 3,
        }
    }

    fn get_skills() -> HashMap<String, i32> {
        // Dungeon World doesn't have traditional skills, but we'll track bonds
        let skills = [
            "Hack and Slash", "Volley", "Defy Danger", "Defend",
            "Spout Lore", "Discern Realities", "Parley", "Aid or Interfere",
        ];
        skills.iter().map(|s| (s.to_string(), 0)).collect()
    }

    fn random_playbook(rng: &mut impl Rng) -> String {
        let playbooks = [
            "Fighter", "Wizard", "Cleric", "Thief", "Ranger", "Bard",
            "Druid", "Paladin",
        ];
        playbooks[rng.gen_range(0..playbooks.len())].to_string()
    }

    fn get_playbook_moves(playbook: &str, level: u32) -> Vec<CharacterTrait> {
        let mut moves = vec![];

        match playbook.to_lowercase().as_str() {
            "fighter" => {
                moves.push(CharacterTrait {
                    name: "Bend Bars, Lift Gates".to_string(),
                    trait_type: TraitType::Move,
                    description: "When you use pure strength to destroy an inanimate obstacle, roll+STR".to_string(),
                    mechanical_effect: Some("10+: choose 3, 7-9: choose 2".to_string()),
                });
                moves.push(CharacterTrait {
                    name: "Signature Weapon".to_string(),
                    trait_type: TraitType::Move,
                    description: "You've honed your skill with your weapon of choice".to_string(),
                    mechanical_effect: Some("Choose weapon and enhancements".to_string()),
                });
                if level >= 2 {
                    moves.push(CharacterTrait {
                        name: "Armored".to_string(),
                        trait_type: TraitType::Move,
                        description: "You ignore the clumsy tag on armor".to_string(),
                        mechanical_effect: Some("Ignore clumsy tag".to_string()),
                    });
                }
            }
            "wizard" => {
                moves.push(CharacterTrait {
                    name: "Spellbook".to_string(),
                    trait_type: TraitType::Move,
                    description: "You have mastered several spells and inscribed them in your spellbook".to_string(),
                    mechanical_effect: Some("Cast Wizard spells".to_string()),
                });
                moves.push(CharacterTrait {
                    name: "Prepare Spells".to_string(),
                    trait_type: TraitType::Move,
                    description: "When you spend uninterrupted time studying your spellbook, hold a number of spells".to_string(),
                    mechanical_effect: Some("Level + INT spells prepared".to_string()),
                });
                moves.push(CharacterTrait {
                    name: "Ritual".to_string(),
                    trait_type: TraitType::Move,
                    description: "You can create magical effects given time and components".to_string(),
                    mechanical_effect: Some("Cast ritual magic".to_string()),
                });
            }
            "cleric" => {
                moves.push(CharacterTrait {
                    name: "Deity".to_string(),
                    trait_type: TraitType::Move,
                    description: "You serve and worship a deity who grants you spells".to_string(),
                    mechanical_effect: Some("Choose domain and precepts".to_string()),
                });
                moves.push(CharacterTrait {
                    name: "Divine Guidance".to_string(),
                    trait_type: TraitType::Move,
                    description: "When you petition your deity according to your precepts, gain their aid".to_string(),
                    mechanical_effect: Some("Ask your deity a question".to_string()),
                });
                moves.push(CharacterTrait {
                    name: "Turn Undead".to_string(),
                    trait_type: TraitType::Move,
                    description: "When you hold your holy symbol and call on your deity for protection, roll+WIS".to_string(),
                    mechanical_effect: Some("Repel the undead".to_string()),
                });
            }
            "thief" => {
                moves.push(CharacterTrait {
                    name: "Trap Expert".to_string(),
                    trait_type: TraitType::Move,
                    description: "When you spend a moment surveying a dangerous area, roll+DEX".to_string(),
                    mechanical_effect: Some("Detect and disable traps".to_string()),
                });
                moves.push(CharacterTrait {
                    name: "Tricks of the Trade".to_string(),
                    trait_type: TraitType::Move,
                    description: "When you pick locks or pockets, roll+DEX".to_string(),
                    mechanical_effect: Some("Succeed at thievery".to_string()),
                });
                moves.push(CharacterTrait {
                    name: "Backstab".to_string(),
                    trait_type: TraitType::Move,
                    description: "When you attack a surprised or defenseless enemy with a melee weapon".to_string(),
                    mechanical_effect: Some("+level damage, or choose from list".to_string()),
                });
            }
            "ranger" => {
                moves.push(CharacterTrait {
                    name: "Hunt and Track".to_string(),
                    trait_type: TraitType::Move,
                    description: "When you follow a trail of clues, roll+WIS".to_string(),
                    mechanical_effect: Some("Follow your prey".to_string()),
                });
                moves.push(CharacterTrait {
                    name: "Called Shot".to_string(),
                    trait_type: TraitType::Move,
                    description: "When you attack a defenseless or surprised enemy at range".to_string(),
                    mechanical_effect: Some("Choose location and effect".to_string()),
                });
                moves.push(CharacterTrait {
                    name: "Animal Companion".to_string(),
                    trait_type: TraitType::Move,
                    description: "You have a supernatural connection with a loyal animal".to_string(),
                    mechanical_effect: Some("Choose your animal companion".to_string()),
                });
            }
            "bard" => {
                moves.push(CharacterTrait {
                    name: "Arcane Art".to_string(),
                    trait_type: TraitType::Move,
                    description: "When you weave a performance into a spell, roll+CHA".to_string(),
                    mechanical_effect: Some("Choose magical effects".to_string()),
                });
                moves.push(CharacterTrait {
                    name: "Bardic Lore".to_string(),
                    trait_type: TraitType::Move,
                    description: "Choose an area of expertise".to_string(),
                    mechanical_effect: Some("Take +1 to Spout Lore about your area".to_string()),
                });
                moves.push(CharacterTrait {
                    name: "Charming and Open".to_string(),
                    trait_type: TraitType::Move,
                    description: "When you speak frankly with someone, roll+CHA".to_string(),
                    mechanical_effect: Some("They tell you something useful".to_string()),
                });
            }
            _ => {
                moves.push(CharacterTrait {
                    name: format!("{} Training", playbook),
                    trait_type: TraitType::Move,
                    description: format!("Training in the ways of the {}", playbook),
                    mechanical_effect: None,
                });
            }
        }

        moves
    }

    fn random_race(rng: &mut impl Rng, playbook: &str) -> String {
        let races = match playbook.to_lowercase().as_str() {
            "fighter" => vec!["Human", "Dwarf", "Elf", "Halfling"],
            "wizard" => vec!["Human", "Elf"],
            "cleric" => vec!["Human", "Dwarf"],
            "thief" => vec!["Human", "Halfling"],
            "ranger" => vec!["Human", "Elf"],
            "bard" => vec!["Human", "Elf"],
            _ => vec!["Human", "Elf", "Dwarf", "Halfling"],
        };
        races[rng.gen_range(0..races.len())].to_string()
    }
}

impl Default for DungeonWorldGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemGenerator for DungeonWorldGenerator {
    fn system(&self) -> GameSystem {
        GameSystem::DungeonWorld
    }

    fn generate(&self, options: &GenerationOptions) -> Result<Character> {
        let mut rng = rand::thread_rng();

        let name = options.name.clone()
            .unwrap_or_else(|| random_fantasy_name(&mut rng));

        let playbook = options.class.clone()
            .unwrap_or_else(|| Self::random_playbook(&mut rng));

        let race = options.race.clone()
            .unwrap_or_else(|| Self::random_race(&mut rng, &playbook));

        let level = options.level.unwrap_or(1);

        let attributes = Self::generate_stats(&mut rng);
        let skills = Self::get_skills();

        let mut traits = Self::get_playbook_moves(&playbook, level);

        // Add racial move
        let racial_move = match race.to_lowercase().as_str() {
            "elf" => "Once per day, cast a wizard spell as if you had prepared it",
            "dwarf" => "When you share a drink with someone, you may parley using CON instead of CHA",
            "halfling" => "When you attack with a ranged weapon, deal +2 damage",
            _ => "When you Defy Danger, on a 10+ you may choose to either act without drawing attention or take +1 forward",
        };

        traits.push(CharacterTrait {
            name: format!("{} Move", race),
            trait_type: TraitType::Racial,
            description: racial_move.to_string(),
            mechanical_effect: Some("Racial ability".to_string()),
        });

        let equipment = if options.include_equipment {
            self.starting_equipment(Some(&playbook))
        } else {
            vec![]
        };

        // Calculate HP
        let hp_base = match playbook.to_lowercase().as_str() {
            "fighter" | "paladin" => 10,
            "cleric" | "ranger" | "bard" => 8,
            "thief" | "druid" => 6,
            "wizard" => 4,
            _ => 8,
        };
        let con_mod = attributes.get("CON").map(|a| a.modifier).unwrap_or(0);
        let hp = hp_base + con_mod;

        let background = CharacterBackground {
            origin: options.background.clone().unwrap_or_else(|| "Unknown".to_string()),
            occupation: Some(playbook.clone()),
            motivation: "Adventure awaits".to_string(),
            connections: vec!["Fellow adventurers".to_string()],
            secrets: vec![],
            history: String::new(),
        };

        Ok(Character {
            id: Uuid::new_v4().to_string(),
            name,
            system: GameSystem::DungeonWorld,
            concept: options.concept.clone().unwrap_or_else(|| format!("{} {}", race, playbook)),
            race: Some(race),
            class: Some(playbook),
            level,
            attributes,
            skills,
            traits,
            equipment,
            background,
            backstory: None,
            notes: format!("HP: {}\nArmor: 0\nLoad: 9\nXP: 0/{}", hp, level + 7),
            portrait_prompt: None,
        })
    }

    fn available_races(&self) -> Vec<String> {
        vec![
            "Human".to_string(),
            "Elf".to_string(),
            "Dwarf".to_string(),
            "Halfling".to_string(),
        ]
    }

    fn available_classes(&self) -> Vec<String> {
        vec![
            "Fighter".to_string(),
            "Wizard".to_string(),
            "Cleric".to_string(),
            "Thief".to_string(),
            "Ranger".to_string(),
            "Bard".to_string(),
            "Druid".to_string(),
            "Paladin".to_string(),
        ]
    }

    fn available_backgrounds(&self) -> Vec<String> {
        // Dungeon World uses Bonds instead of backgrounds
        vec![]
    }

    fn attribute_names(&self) -> Vec<String> {
        vec![
            "STR".to_string(),
            "DEX".to_string(),
            "CON".to_string(),
            "INT".to_string(),
            "WIS".to_string(),
            "CHA".to_string(),
        ]
    }

    fn starting_equipment(&self, playbook: Option<&str>) -> Vec<Equipment> {
        let mut equipment = vec![
            Equipment {
                name: "Dungeon Rations".to_string(),
                category: EquipmentCategory::Consumable,
                description: "5 uses, ration tag".to_string(),
                stats: [("Weight".to_string(), "1".to_string())].into(),
            },
        ];

        match playbook.map(|s| s.to_lowercase()).as_deref() {
            Some("fighter") => {
                equipment.push(Equipment {
                    name: "Signature Weapon".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Your personal weapon".to_string(),
                    stats: [("Damage".to_string(), "d10".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Scale Armor".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "2 armor, worn, clumsy".to_string(),
                    stats: [("Armor".to_string(), "2".to_string())].into(),
                });
            }
            Some("wizard") => {
                equipment.push(Equipment {
                    name: "Spellbook".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "Contains your spells".to_string(),
                    stats: HashMap::new(),
                });
                equipment.push(Equipment {
                    name: "Staff".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Close, two-handed".to_string(),
                    stats: [("Damage".to_string(), "d4".to_string())].into(),
                });
            }
            Some("cleric") => {
                equipment.push(Equipment {
                    name: "Holy Symbol".to_string(),
                    category: EquipmentCategory::Magic,
                    description: "Symbol of your deity".to_string(),
                    stats: HashMap::new(),
                });
                equipment.push(Equipment {
                    name: "Warhammer".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Close".to_string(),
                    stats: [("Damage".to_string(), "d8".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Chainmail".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "1 armor, worn".to_string(),
                    stats: [("Armor".to_string(), "1".to_string())].into(),
                });
            }
            Some("thief") => {
                equipment.push(Equipment {
                    name: "Dagger".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Hand, precise".to_string(),
                    stats: [("Damage".to_string(), "d4".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Thieves' Tools".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "Required for traps and locks".to_string(),
                    stats: HashMap::new(),
                });
                equipment.push(Equipment {
                    name: "Leather Armor".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "1 armor, worn".to_string(),
                    stats: [("Armor".to_string(), "1".to_string())].into(),
                });
            }
            Some("ranger") => {
                equipment.push(Equipment {
                    name: "Hunter's Bow".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Near, far".to_string(),
                    stats: [("Damage".to_string(), "d8".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Bundle of Arrows".to_string(),
                    category: EquipmentCategory::Consumable,
                    description: "3 ammo".to_string(),
                    stats: HashMap::new(),
                });
                equipment.push(Equipment {
                    name: "Leather Armor".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "1 armor, worn".to_string(),
                    stats: [("Armor".to_string(), "1".to_string())].into(),
                });
            }
            _ => {
                equipment.push(Equipment {
                    name: "Short Sword".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Close".to_string(),
                    stats: [("Damage".to_string(), "d6".to_string())].into(),
                });
            }
        }

        equipment
    }
}
