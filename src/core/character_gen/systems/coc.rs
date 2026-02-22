//! Call of Cthulhu Character Generator
//!
//! Generates investigators for Call of Cthulhu 7th Edition.

use crate::core::character_gen::{
    SystemGenerator, Character, GameSystem, GenerationOptions,
    AttributeValue, CharacterTrait, TraitType, Equipment, EquipmentCategory,
    CharacterBackground, Result, random_1920s_name,
};
use rand::Rng;
use uuid::Uuid;
use std::collections::HashMap;

pub struct CallOfCthulhuGenerator;

impl CallOfCthulhuGenerator {
    pub fn new() -> Self {
        Self
    }

    fn roll_characteristics(rng: &mut impl Rng) -> HashMap<String, AttributeValue> {
        let mut attrs = HashMap::new();

        // Roll 3d6*5 for STR, CON, DEX, APP, POW
        for name in ["STR", "CON", "DEX", "APP", "POW"] {
            let total: i32 = (0..3).map(|_| rng.gen_range(1..=6)).sum::<i32>() * 5;
            attrs.insert(name.to_string(), AttributeValue::new_raw(total));
        }

        // Roll (2d6+6)*5 for SIZ, INT, EDU
        for name in ["SIZ", "INT", "EDU"] {
            let total: i32 = ((0..2).map(|_| rng.gen_range(1..=6)).sum::<i32>() + 6) * 5;
            attrs.insert(name.to_string(), AttributeValue::new_raw(total));
        }

        // Derive Luck
        let luck: i32 = (0..3).map(|_| rng.gen_range(1..=6)).sum::<i32>() * 5;
        attrs.insert("Luck".to_string(), AttributeValue::new_raw(luck));

        attrs
    }

    fn get_skills() -> HashMap<String, i32> {
        let skills = [
            ("Accounting", 5), ("Anthropology", 1), ("Appraise", 5),
            ("Archaeology", 1), ("Art/Craft", 5), ("Charm", 15),
            ("Climb", 20), ("Credit Rating", 0), ("Cthulhu Mythos", 0),
            ("Disguise", 5), ("Dodge", 0), ("Drive Auto", 20),
            ("Electrical Repair", 10), ("Fast Talk", 5), ("Fighting (Brawl)", 25),
            ("Firearms (Handgun)", 20), ("Firearms (Rifle)", 25),
            ("First Aid", 30), ("History", 5), ("Intimidate", 15),
            ("Jump", 20), ("Law", 5), ("Library Use", 20),
            ("Listen", 20), ("Locksmith", 1), ("Mechanical Repair", 10),
            ("Medicine", 1), ("Natural World", 10), ("Navigate", 10),
            ("Occult", 5), ("Operate Heavy Machinery", 1), ("Persuade", 10),
            ("Photography", 5), ("Pilot", 1), ("Psychology", 10),
            ("Psychoanalysis", 1), ("Ride", 5), ("Science", 1),
            ("Sleight of Hand", 10), ("Spot Hidden", 25), ("Stealth", 20),
            ("Survival", 10), ("Swim", 20), ("Throw", 20),
            ("Track", 10),
        ];

        skills.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    fn random_occupation(rng: &mut impl Rng) -> String {
        let occupations = [
            "Antiquarian", "Author", "Dilettante", "Doctor",
            "Journalist", "Librarian", "Police Detective", "Professor",
            "Private Investigator", "Parapsychologist", "Nurse",
            "Clergy", "Archaeologist", "Artist", "Lawyer",
            "Military Officer", "Pilot", "Engineer", "Scientist",
        ];
        occupations[rng.gen_range(0..occupations.len())].to_string()
    }
}

impl Default for CallOfCthulhuGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemGenerator for CallOfCthulhuGenerator {
    fn system(&self) -> GameSystem {
        GameSystem::CallOfCthulhu
    }

    fn generate(&self, options: &GenerationOptions) -> Result<Character> {
        let mut rng = rand::thread_rng();

        let name = options.name.clone()
            .unwrap_or_else(|| random_1920s_name(&mut rng));

        let attributes = Self::roll_characteristics(&mut rng);
        let skills = Self::get_skills();

        let occupation = options.class.clone()
            .unwrap_or_else(|| Self::random_occupation(&mut rng));

        let traits = vec![
            CharacterTrait {
                name: "Investigator".to_string(),
                trait_type: TraitType::Background,
                description: "Driven to uncover the truth, no matter the cost to sanity".to_string(),
                mechanical_effect: None,
            },
            CharacterTrait {
                name: occupation.clone(),
                trait_type: TraitType::Class,
                description: format!("Professional {} with relevant skills", occupation),
                mechanical_effect: Some("Occupation skill points based on EDU".to_string()),
            },
        ];

        let equipment = if options.include_equipment {
            self.starting_equipment(Some(&occupation))
        } else {
            vec![]
        };

        // Calculate derived stats
        let hp = (attributes.get("CON").map(|a| a.base).unwrap_or(50)
            + attributes.get("SIZ").map(|a| a.base).unwrap_or(50)) / 10;
        let san = attributes.get("POW").map(|a| a.base).unwrap_or(50);
        let mp = attributes.get("POW").map(|a| a.base).unwrap_or(50) / 5;

        let background = CharacterBackground {
            origin: "United States".to_string(),
            occupation: Some(occupation.clone()),
            motivation: "Seeking forbidden knowledge".to_string(),
            connections: vec!["Local library".to_string(), "University contact".to_string()],
            secrets: vec!["Witnessed something unexplainable".to_string()],
            history: String::new(),
        };

        Ok(Character {
            id: Uuid::new_v4().to_string(),
            name,
            system: GameSystem::CallOfCthulhu,
            concept: options.concept.clone().unwrap_or(occupation),
            race: None, // CoC doesn't use races in the traditional sense
            class: Some("Investigator".to_string()),
            level: 1, // CoC doesn't have levels
            attributes,
            skills,
            traits,
            equipment,
            background,
            backstory: None,
            notes: format!("HP: {}\nSanity: {}\nMagic Points: {}", hp, san, mp),
            portrait_prompt: None,
        })
    }

    fn available_races(&self) -> Vec<String> {
        // CoC doesn't use races - return empty or "Human"
        vec!["Human".to_string()]
    }

    fn available_classes(&self) -> Vec<String> {
        vec![
            "Antiquarian".to_string(),
            "Archaeologist".to_string(),
            "Artist".to_string(),
            "Author".to_string(),
            "Clergy".to_string(),
            "Dilettante".to_string(),
            "Doctor".to_string(),
            "Engineer".to_string(),
            "Entertainer".to_string(),
            "Journalist".to_string(),
            "Lawyer".to_string(),
            "Librarian".to_string(),
            "Military Officer".to_string(),
            "Nurse".to_string(),
            "Parapsychologist".to_string(),
            "Pilot".to_string(),
            "Police Detective".to_string(),
            "Private Investigator".to_string(),
            "Professor".to_string(),
            "Scientist".to_string(),
        ]
    }

    fn available_backgrounds(&self) -> Vec<String> {
        vec![
            "Wealthy".to_string(),
            "Middle Class".to_string(),
            "Poor".to_string(),
            "Academic".to_string(),
            "Criminal".to_string(),
            "Military".to_string(),
            "Religious".to_string(),
        ]
    }

    fn attribute_names(&self) -> Vec<String> {
        vec![
            "STR".to_string(),
            "CON".to_string(),
            "SIZ".to_string(),
            "DEX".to_string(),
            "APP".to_string(),
            "INT".to_string(),
            "POW".to_string(),
            "EDU".to_string(),
            "Luck".to_string(),
        ]
    }

    fn starting_equipment(&self, occupation: Option<&str>) -> Vec<Equipment> {
        let mut equipment = vec![
            Equipment {
                name: "Flashlight".to_string(),
                category: EquipmentCategory::Tool,
                description: "Essential for exploring dark places".to_string(),
                stats: HashMap::new(),
            },
            Equipment {
                name: "Notebook and Pen".to_string(),
                category: EquipmentCategory::Tool,
                description: "For recording observations".to_string(),
                stats: HashMap::new(),
            },
            Equipment {
                name: "Pocket Knife".to_string(),
                category: EquipmentCategory::Tool,
                description: "A handy utility tool".to_string(),
                stats: HashMap::new(),
            },
        ];

        match occupation.map(|s| s.to_lowercase()).as_deref() {
            Some("doctor") | Some("nurse") => {
                equipment.push(Equipment {
                    name: "Medical Bag".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "Contains first aid supplies and basic medical tools".to_string(),
                    stats: [("Bonus".to_string(), "+20% to First Aid".to_string())].into(),
                });
            }
            Some("police detective") | Some("private investigator") => {
                equipment.push(Equipment {
                    name: ".38 Revolver".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Standard police sidearm".to_string(),
                    stats: [("Damage".to_string(), "1d10".to_string()), ("Range".to_string(), "15 yards".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Handcuffs".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "For restraining suspects".to_string(),
                    stats: HashMap::new(),
                });
            }
            Some("professor") | Some("librarian") | Some("antiquarian") => {
                equipment.push(Equipment {
                    name: "Research Notes".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "Years of accumulated research".to_string(),
                    stats: HashMap::new(),
                });
                equipment.push(Equipment {
                    name: "Magnifying Glass".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "For examining fine details".to_string(),
                    stats: HashMap::new(),
                });
            }
            Some("journalist") | Some("author") => {
                equipment.push(Equipment {
                    name: "Camera".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "For documenting evidence".to_string(),
                    stats: HashMap::new(),
                });
                equipment.push(Equipment {
                    name: "Press Credentials".to_string(),
                    category: EquipmentCategory::Other,
                    description: "Opens doors".to_string(),
                    stats: HashMap::new(),
                });
            }
            _ => {}
        }

        equipment
    }
}
