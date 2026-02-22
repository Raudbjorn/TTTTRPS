//! GURPS Character Generator
//!
//! Generates characters for GURPS (Generic Universal RolePlaying System).

use crate::core::character_gen::{
    SystemGenerator, Character, GameSystem, GenerationOptions,
    AttributeValue, CharacterTrait, TraitType, Equipment, EquipmentCategory,
    CharacterBackground, Result, random_fantasy_name, random_modern_name,
};
use rand::Rng;
use uuid::Uuid;
use std::collections::HashMap;

pub struct GURPSGenerator;

impl GURPSGenerator {
    pub fn new() -> Self {
        Self
    }

    fn generate_attributes(rng: &mut impl Rng, point_buy: Option<u32>) -> HashMap<String, AttributeValue> {
        // GURPS base attributes start at 10
        // Points: +/-10 per +/-1 for ST/IQ, +/-20 per +/-1 for DX/HT

        let points = point_buy.unwrap_or(100) as i32;
        let mut remaining = points;

        let mut attrs = HashMap::new();

        // Simplified random distribution
        let st_bonus = rng.gen_range(-2..=3);
        remaining -= st_bonus * 10;
        attrs.insert("ST".to_string(), AttributeValue::new_raw(10 + st_bonus));

        let dx_bonus = rng.gen_range(-1..=2);
        remaining -= dx_bonus * 20;
        attrs.insert("DX".to_string(), AttributeValue::new_raw(10 + dx_bonus));

        let iq_bonus = rng.gen_range(-1..=2);
        remaining -= iq_bonus * 10;
        attrs.insert("IQ".to_string(), AttributeValue::new_raw(10 + iq_bonus));

        let ht_bonus = remaining.min(40).max(-20) / 20;
        attrs.insert("HT".to_string(), AttributeValue::new_raw(10 + ht_bonus));

        // Secondary characteristics
        let st = attrs.get("ST").map(|a| a.base).unwrap_or(10);
        let dx = attrs.get("DX").map(|a| a.base).unwrap_or(10);
        let iq = attrs.get("IQ").map(|a| a.base).unwrap_or(10);
        let ht = attrs.get("HT").map(|a| a.base).unwrap_or(10);

        attrs.insert("HP".to_string(), AttributeValue::new_raw(st));
        attrs.insert("Will".to_string(), AttributeValue::new_raw(iq));
        attrs.insert("Per".to_string(), AttributeValue::new_raw(iq));
        attrs.insert("FP".to_string(), AttributeValue::new_raw(ht));
        attrs.insert("Basic Speed".to_string(), AttributeValue::new_raw((ht + dx) / 4));
        attrs.insert("Basic Move".to_string(), AttributeValue::new_raw((ht + dx) / 4));

        attrs
    }

    fn get_skills() -> HashMap<String, i32> {
        // Common GURPS skills
        let skills = [
            "Guns/TL", "Brawling", "Knife", "Broadsword", "Shield",
            "Stealth", "Climbing", "Swimming", "Running", "Jumping",
            "First Aid", "Survival", "Tracking", "Navigation",
            "Diplomacy", "Fast-Talk", "Intimidation", "Streetwise",
            "Area Knowledge", "Current Affairs", "Research",
            "Computer Operation", "Driving", "Electronics Operation",
        ];
        skills.iter().map(|s| (s.to_string(), 0)).collect()
    }

    fn random_template(rng: &mut impl Rng, theme: Option<&str>) -> String {
        let templates = match theme {
            Some("fantasy") => vec!["Knight", "Wizard", "Thief", "Ranger", "Priest"],
            Some("modern") => vec!["Soldier", "Detective", "Scientist", "Hacker", "Medic"],
            Some("scifi") | Some("space") => vec!["Space Marine", "Pilot", "Engineer", "Scientist", "Diplomat"],
            _ => vec!["Adventurer", "Scholar", "Warrior", "Rogue", "Leader"],
        };
        templates[rng.gen_range(0..templates.len())].to_string()
    }

    fn get_template_traits(template: &str, rng: &mut impl Rng) -> Vec<CharacterTrait> {
        let mut traits = vec![];

        // Add advantages based on template
        match template.to_lowercase().as_str() {
            "knight" | "warrior" | "soldier" | "space marine" => {
                traits.push(CharacterTrait {
                    name: "Combat Reflexes".to_string(),
                    trait_type: TraitType::Advantage,
                    description: "+1 to all active defense rolls, never freeze in combat".to_string(),
                    mechanical_effect: Some("15 points".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "High Pain Threshold".to_string(),
                    trait_type: TraitType::Advantage,
                    description: "Never suffer shock penalties, +3 to resist torture".to_string(),
                    mechanical_effect: Some("10 points".to_string()),
                });
            }
            "wizard" | "scientist" | "scholar" => {
                traits.push(CharacterTrait {
                    name: "Magery 2".to_string(),
                    trait_type: TraitType::Advantage,
                    description: "Can cast spells, +2 to spell rolls".to_string(),
                    mechanical_effect: Some("25 points".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Eidetic Memory".to_string(),
                    trait_type: TraitType::Advantage,
                    description: "Never forget anything, +5 to learn mental skills".to_string(),
                    mechanical_effect: Some("5 points".to_string()),
                });
            }
            "thief" | "rogue" | "hacker" => {
                traits.push(CharacterTrait {
                    name: "Flexibility".to_string(),
                    trait_type: TraitType::Advantage,
                    description: "+3 to Escape, +1 to Climbing".to_string(),
                    mechanical_effect: Some("5 points".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Night Vision 5".to_string(),
                    trait_type: TraitType::Advantage,
                    description: "Reduce darkness penalties by 5".to_string(),
                    mechanical_effect: Some("5 points".to_string()),
                });
            }
            _ => {
                traits.push(CharacterTrait {
                    name: "Luck".to_string(),
                    trait_type: TraitType::Advantage,
                    description: "Reroll any roll once per hour of play".to_string(),
                    mechanical_effect: Some("15 points".to_string()),
                });
            }
        }

        // Add a random disadvantage
        let disadvantages = [
            ("Sense of Duty", "Strong loyalty to a group", "-5 to -20 points"),
            ("Code of Honor", "Must follow a code", "-5 to -15 points"),
            ("Overconfidence", "Believes in own abilities", "-5 points"),
            ("Curious", "Must investigate mysteries", "-5 points"),
            ("Impulsiveness", "Acts without thinking", "-10 points"),
            ("Stubborn", "Won't change mind easily", "-5 points"),
        ];

        let disadv = &disadvantages[rng.gen_range(0..disadvantages.len())];
        traits.push(CharacterTrait {
            name: disadv.0.to_string(),
            trait_type: TraitType::Disadvantage,
            description: disadv.1.to_string(),
            mechanical_effect: Some(disadv.2.to_string()),
        });

        traits
    }
}

impl Default for GURPSGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemGenerator for GURPSGenerator {
    fn system(&self) -> GameSystem {
        GameSystem::GURPS
    }

    fn generate(&self, options: &GenerationOptions) -> Result<Character> {
        let mut rng = rand::thread_rng();

        let theme = options.theme.as_deref();
        let name = options.name.clone()
            .unwrap_or_else(|| {
                if theme == Some("fantasy") {
                    random_fantasy_name(&mut rng)
                } else {
                    random_modern_name(&mut rng)
                }
            });

        let attributes = Self::generate_attributes(&mut rng, options.point_buy);
        let skills = Self::get_skills();

        let template = options.class.clone()
            .unwrap_or_else(|| Self::random_template(&mut rng, theme));

        let traits = Self::get_template_traits(&template, &mut rng);

        let equipment = if options.include_equipment {
            self.starting_equipment(Some(&template))
        } else {
            vec![]
        };

        let background = CharacterBackground {
            origin: options.background.clone().unwrap_or_else(|| "Unknown".to_string()),
            occupation: Some(template.clone()),
            motivation: "Personal goals".to_string(),
            connections: vec![],
            secrets: vec![],
            history: String::new(),
        };

        let hp = attributes.get("HP").map(|a| a.base).unwrap_or(10);
        let fp = attributes.get("FP").map(|a| a.base).unwrap_or(10);

        Ok(Character {
            id: Uuid::new_v4().to_string(),
            name,
            system: GameSystem::GURPS,
            concept: options.concept.clone().unwrap_or_else(|| template.clone()),
            race: options.race.clone(),
            class: Some(template),
            level: 1, // GURPS doesn't use levels
            attributes,
            skills,
            traits,
            equipment,
            background,
            backstory: None,
            notes: format!("HP: {}\nFP: {}\nPoint Value: {}", hp, fp, options.point_buy.unwrap_or(100)),
            portrait_prompt: None,
        })
    }

    fn available_races(&self) -> Vec<String> {
        vec![
            "Human".to_string(),
            "Elf".to_string(),
            "Dwarf".to_string(),
            "Halfling".to_string(),
            "Cat-Folk".to_string(),
            "Reptilian".to_string(),
        ]
    }

    fn available_classes(&self) -> Vec<String> {
        vec![
            "Adventurer".to_string(),
            "Knight".to_string(),
            "Wizard".to_string(),
            "Thief".to_string(),
            "Ranger".to_string(),
            "Priest".to_string(),
            "Soldier".to_string(),
            "Detective".to_string(),
            "Scientist".to_string(),
            "Hacker".to_string(),
        ]
    }

    fn available_backgrounds(&self) -> Vec<String> {
        vec![
            "Wealthy".to_string(),
            "Average".to_string(),
            "Struggling".to_string(),
            "Poor".to_string(),
            "Dead Broke".to_string(),
        ]
    }

    fn attribute_names(&self) -> Vec<String> {
        vec![
            "ST".to_string(), "DX".to_string(), "IQ".to_string(), "HT".to_string(),
            "HP".to_string(), "Will".to_string(), "Per".to_string(), "FP".to_string(),
            "Basic Speed".to_string(), "Basic Move".to_string(),
        ]
    }

    fn starting_equipment(&self, template: Option<&str>) -> Vec<Equipment> {
        let mut equipment = vec![
            Equipment {
                name: "Personal Basics".to_string(),
                category: EquipmentCategory::Tool,
                description: "Clothing, ID, pocket items".to_string(),
                stats: HashMap::new(),
            },
        ];

        match template.map(|s| s.to_lowercase()).as_deref() {
            Some("knight") | Some("warrior") => {
                equipment.push(Equipment {
                    name: "Broadsword".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "sw+1 cut, thr+2 imp".to_string(),
                    stats: [("Damage".to_string(), "sw+1/thr+2".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Medium Shield".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "DB +2".to_string(),
                    stats: [("DB".to_string(), "+2".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Mail Armor".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "DR 4/2*".to_string(),
                    stats: [("DR".to_string(), "4".to_string())].into(),
                });
            }
            Some("soldier") | Some("space marine") => {
                equipment.push(Equipment {
                    name: "Assault Rifle".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "5d pi".to_string(),
                    stats: [("Damage".to_string(), "5d pi".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Combat Armor".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "DR 12/5*".to_string(),
                    stats: [("DR".to_string(), "12".to_string())].into(),
                });
            }
            Some("wizard") => {
                equipment.push(Equipment {
                    name: "Staff".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "sw+2 cr, can be powerstone".to_string(),
                    stats: [("Damage".to_string(), "sw+2 cr".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Spell Components".to_string(),
                    category: EquipmentCategory::Consumable,
                    description: "Various magical materials".to_string(),
                    stats: HashMap::new(),
                });
            }
            Some("thief") | Some("rogue") => {
                equipment.push(Equipment {
                    name: "Dagger".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "thr-1 imp, can be thrown".to_string(),
                    stats: [("Damage".to_string(), "thr-1 imp".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Lockpicks".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "Basic quality".to_string(),
                    stats: HashMap::new(),
                });
            }
            _ => {
                equipment.push(Equipment {
                    name: "Large Knife".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "sw-2 cut, thr imp".to_string(),
                    stats: [("Damage".to_string(), "sw-2/thr".to_string())].into(),
                });
            }
        }

        equipment
    }
}
