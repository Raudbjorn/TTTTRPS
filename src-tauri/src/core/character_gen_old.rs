//! Character Generation Module
//!
//! Supports character creation for multiple TTRPG systems including:
//! - D&D 5e / Pathfinder (Fantasy)
//! - Call of Cthulhu (Horror)
//! - Cyberpunk / Shadowrun (Sci-Fi)
//! - Fate Core (Generic)
//! - World of Darkness (Urban Fantasy)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rand::Rng;
use uuid::Uuid;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum CharacterGenError {
    #[error("Unsupported game system: {0}")]
    UnsupportedSystem(String),

    #[error("Invalid attribute: {0}")]
    InvalidAttribute(String),

    #[error("Invalid option: {0}")]
    InvalidOption(String),

    #[error("LLM error: {0}")]
    LLMError(String),
}

pub type Result<T> = std::result::Result<T, CharacterGenError>;

// ============================================================================
// Core Character Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub system: GameSystem,
    pub concept: String,
    pub attributes: HashMap<String, AttributeValue>,
    pub skills: HashMap<String, i32>,
    pub traits: Vec<CharacterTrait>,
    pub equipment: Vec<Equipment>,
    pub background: CharacterBackground,
    pub notes: String,
    pub portrait_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GameSystem {
    DnD5e,
    Pathfinder2e,
    CallOfCthulhu,
    Cyberpunk,
    Shadowrun,
    FateCore,
    WorldOfDarkness,
    Custom(String),
}

impl GameSystem {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "dnd5e" | "d&d 5e" | "d&d" | "5e" => Self::DnD5e,
            "pathfinder" | "pf2e" | "pathfinder 2e" => Self::Pathfinder2e,
            "coc" | "call of cthulhu" | "cthulhu" => Self::CallOfCthulhu,
            "cyberpunk" | "cp2077" | "cyberpunk red" => Self::Cyberpunk,
            "shadowrun" | "sr" => Self::Shadowrun,
            "fate" | "fate core" | "fae" => Self::FateCore,
            "wod" | "world of darkness" | "vtm" | "vampire" => Self::WorldOfDarkness,
            other => Self::Custom(other.to_string()),
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::DnD5e => "D&D 5th Edition",
            Self::Pathfinder2e => "Pathfinder 2nd Edition",
            Self::CallOfCthulhu => "Call of Cthulhu",
            Self::Cyberpunk => "Cyberpunk",
            Self::Shadowrun => "Shadowrun",
            Self::FateCore => "Fate Core",
            Self::WorldOfDarkness => "World of Darkness",
            Self::Custom(name) => name,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeValue {
    pub base: i32,
    pub modifier: i32,
    pub temp_bonus: i32,
}

impl AttributeValue {
    pub fn new(base: i32) -> Self {
        Self {
            base,
            modifier: Self::calculate_modifier(base),
            temp_bonus: 0,
        }
    }

    fn calculate_modifier(base: i32) -> i32 {
        (base - 10) / 2
    }

    pub fn total(&self) -> i32 {
        self.base + self.temp_bonus
    }

    pub fn total_modifier(&self) -> i32 {
        Self::calculate_modifier(self.total())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterTrait {
    pub name: String,
    pub trait_type: TraitType,
    pub description: String,
    pub mechanical_effect: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TraitType {
    Personality,
    Background,
    Racial,
    Class,
    Feat,
    Flaw,
    Bond,
    Ideal,
    Aspect, // Fate
    Stunt,  // Fate
    Merit,  // WoD
    Edge,   // Shadowrun
    Cyberware, // Cyberpunk
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Equipment {
    pub name: String,
    pub category: EquipmentCategory,
    pub description: String,
    pub stats: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EquipmentCategory {
    Weapon,
    Armor,
    Tool,
    Consumable,
    Magic,
    Tech,
    Vehicle,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterBackground {
    pub origin: String,
    pub occupation: Option<String>,
    pub motivation: String,
    pub connections: Vec<String>,
    pub secrets: Vec<String>,
    pub history: String,
}

// ============================================================================
// Generation Options
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenerationOptions {
    pub system: Option<String>,
    pub name: Option<String>,
    pub concept: Option<String>,
    pub race: Option<String>,
    pub class: Option<String>,
    pub background: Option<String>,
    pub level: Option<u32>,
    pub point_buy: Option<u32>,
    pub random_stats: bool,
    pub include_equipment: bool,
    pub include_backstory: bool,
    pub theme: Option<String>,
}

// ============================================================================
// System-Specific Generators
// ============================================================================

pub struct CharacterGenerator;

impl CharacterGenerator {
    /// Generate a character for the specified system
    pub fn generate(options: &GenerationOptions) -> Result<Character> {
        let system = options.system.as_deref()
            .map(GameSystem::from_str)
            .unwrap_or(GameSystem::DnD5e);

        match system {
            GameSystem::DnD5e => Self::generate_dnd5e(options),
            GameSystem::Pathfinder2e => Self::generate_pathfinder(options),
            GameSystem::CallOfCthulhu => Self::generate_coc(options),
            GameSystem::Cyberpunk => Self::generate_cyberpunk(options),
            GameSystem::Shadowrun => Self::generate_shadowrun(options),
            GameSystem::FateCore => Self::generate_fate(options),
            GameSystem::WorldOfDarkness => Self::generate_wod(options),
            GameSystem::Custom(ref name) => Err(CharacterGenError::UnsupportedSystem(name.clone())),
        }
    }

    // ========================================================================
    // D&D 5e Generator
    // ========================================================================

    fn generate_dnd5e(options: &GenerationOptions) -> Result<Character> {
        let mut rng = rand::thread_rng();

        // Generate or use provided name
        let name = options.name.clone()
            .unwrap_or_else(|| Self::random_fantasy_name(&mut rng));

        // Generate attributes
        let attributes = if options.random_stats {
            Self::roll_dnd_stats(&mut rng)
        } else {
            Self::standard_array_dnd()
        };

        // Basic skills (would be expanded based on class/background)
        let skills = Self::default_dnd_skills();

        // Generate traits based on class/race
        let traits = Self::generate_dnd_traits(options);

        // Equipment
        let equipment = if options.include_equipment {
            Self::starting_equipment_dnd(options.class.as_deref())
        } else {
            vec![]
        };

        // Background
        let background = CharacterBackground {
            origin: options.background.clone().unwrap_or_else(|| "Unknown".to_string()),
            occupation: options.class.clone(),
            motivation: "Adventure and glory".to_string(),
            connections: vec![],
            secrets: vec![],
            history: String::new(),
        };

        Ok(Character {
            id: Uuid::new_v4().to_string(),
            name,
            system: GameSystem::DnD5e,
            concept: options.concept.clone().unwrap_or_else(|| {
                format!("{} {}",
                    options.race.as_deref().unwrap_or("Human"),
                    options.class.as_deref().unwrap_or("Adventurer")
                )
            }),
            attributes,
            skills,
            traits,
            equipment,
            background,
            notes: String::new(),
            portrait_prompt: None,
        })
    }

    fn roll_dnd_stats(rng: &mut impl Rng) -> HashMap<String, AttributeValue> {
        let mut attributes = HashMap::new();
        let attrs = ["Strength", "Dexterity", "Constitution", "Intelligence", "Wisdom", "Charisma"];

        for attr in attrs {
            // 4d6 drop lowest
            let mut rolls: Vec<i32> = (0..4).map(|_| rng.gen_range(1..=6)).collect();
            rolls.sort();
            let total: i32 = rolls[1..].iter().sum();
            attributes.insert(attr.to_string(), AttributeValue::new(total));
        }

        attributes
    }

    fn standard_array_dnd() -> HashMap<String, AttributeValue> {
        let mut attributes = HashMap::new();
        let standard = [15, 14, 13, 12, 10, 8];
        let attrs = ["Strength", "Dexterity", "Constitution", "Intelligence", "Wisdom", "Charisma"];

        for (attr, &val) in attrs.iter().zip(standard.iter()) {
            attributes.insert(attr.to_string(), AttributeValue::new(val));
        }

        attributes
    }

    fn default_dnd_skills() -> HashMap<String, i32> {
        let skills = [
            "Acrobatics", "Animal Handling", "Arcana", "Athletics",
            "Deception", "History", "Insight", "Intimidation",
            "Investigation", "Medicine", "Nature", "Perception",
            "Performance", "Persuasion", "Religion", "Sleight of Hand",
            "Stealth", "Survival",
        ];

        skills.iter().map(|s| (s.to_string(), 0)).collect()
    }

    fn generate_dnd_traits(options: &GenerationOptions) -> Vec<CharacterTrait> {
        let mut traits = vec![];

        // Add racial trait
        if let Some(race) = &options.race {
            traits.push(CharacterTrait {
                name: format!("{} Heritage", race),
                trait_type: TraitType::Racial,
                description: format!("The cultural and biological traits of the {} people.", race),
                mechanical_effect: None,
            });
        }

        // Add class feature
        if let Some(class) = &options.class {
            traits.push(CharacterTrait {
                name: format!("{} Training", class),
                trait_type: TraitType::Class,
                description: format!("Trained in the ways of the {}.", class),
                mechanical_effect: None,
            });
        }

        traits
    }

    fn starting_equipment_dnd(class: Option<&str>) -> Vec<Equipment> {
        let mut equipment = vec![];

        // Common adventuring gear
        equipment.push(Equipment {
            name: "Backpack".to_string(),
            category: EquipmentCategory::Tool,
            description: "A sturdy leather backpack".to_string(),
            stats: HashMap::new(),
        });

        equipment.push(Equipment {
            name: "Bedroll".to_string(),
            category: EquipmentCategory::Tool,
            description: "Warm bedroll for camping".to_string(),
            stats: HashMap::new(),
        });

        // Class-specific equipment
        match class {
            Some("Fighter") | Some("Paladin") => {
                equipment.push(Equipment {
                    name: "Longsword".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A versatile martial weapon".to_string(),
                    stats: [("Damage".to_string(), "1d8 slashing".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Chain Mail".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "Heavy armor made of interlocking rings".to_string(),
                    stats: [("AC".to_string(), "16".to_string())].into(),
                });
            }
            Some("Rogue") => {
                equipment.push(Equipment {
                    name: "Shortsword".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A finesse weapon perfect for quick strikes".to_string(),
                    stats: [("Damage".to_string(), "1d6 piercing".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Thieves' Tools".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "Lockpicks and other tools of the trade".to_string(),
                    stats: HashMap::new(),
                });
            }
            Some("Wizard") | Some("Sorcerer") => {
                equipment.push(Equipment {
                    name: "Spellbook".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "A leather-bound tome of arcane knowledge".to_string(),
                    stats: HashMap::new(),
                });
                equipment.push(Equipment {
                    name: "Arcane Focus".to_string(),
                    category: EquipmentCategory::Magic,
                    description: "A crystal orb for channeling magic".to_string(),
                    stats: HashMap::new(),
                });
            }
            _ => {
                equipment.push(Equipment {
                    name: "Dagger".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A simple but effective weapon".to_string(),
                    stats: [("Damage".to_string(), "1d4 piercing".to_string())].into(),
                });
            }
        }

        equipment
    }

    // ========================================================================
    // Call of Cthulhu Generator
    // ========================================================================

    fn generate_coc(options: &GenerationOptions) -> Result<Character> {
        let mut rng = rand::thread_rng();

        let name = options.name.clone()
            .unwrap_or_else(|| Self::random_1920s_name(&mut rng));

        // CoC uses different attributes
        let attributes = Self::roll_coc_stats(&mut rng);
        let skills = Self::default_coc_skills();

        let occupation = options.class.clone()
            .unwrap_or_else(|| Self::random_coc_occupation(&mut rng));

        let background = CharacterBackground {
            origin: "Boston, Massachusetts".to_string(),
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
            attributes,
            skills,
            traits: vec![
                CharacterTrait {
                    name: "Investigator".to_string(),
                    trait_type: TraitType::Background,
                    description: "Driven to uncover the truth, no matter the cost".to_string(),
                    mechanical_effect: None,
                }
            ],
            equipment: vec![
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
            ],
            background,
            notes: String::new(),
            portrait_prompt: None,
        })
    }

    fn roll_coc_stats(rng: &mut impl Rng) -> HashMap<String, AttributeValue> {
        let mut attributes = HashMap::new();

        // CoC 7e characteristics - helper functions
        fn roll_3d6(rng: &mut impl Rng) -> i32 {
            (0..3).map(|_| rng.gen_range(1..=6)).sum::<i32>() * 5
        }

        fn roll_2d6_plus_6(rng: &mut impl Rng) -> i32 {
            ((0..2).map(|_| rng.gen_range(1..=6)).sum::<i32>() + 6) * 5
        }

        attributes.insert("STR".to_string(), AttributeValue::new(roll_3d6(rng)));
        attributes.insert("CON".to_string(), AttributeValue::new(roll_3d6(rng)));
        attributes.insert("SIZ".to_string(), AttributeValue::new(roll_2d6_plus_6(rng)));
        attributes.insert("DEX".to_string(), AttributeValue::new(roll_3d6(rng)));
        attributes.insert("APP".to_string(), AttributeValue::new(roll_3d6(rng)));
        attributes.insert("INT".to_string(), AttributeValue::new(roll_2d6_plus_6(rng)));
        attributes.insert("POW".to_string(), AttributeValue::new(roll_3d6(rng)));
        attributes.insert("EDU".to_string(), AttributeValue::new(roll_2d6_plus_6(rng)));

        // Derived stats
        let luck = roll_3d6(rng);
        attributes.insert("Luck".to_string(), AttributeValue::new(luck));

        attributes
    }

    fn default_coc_skills() -> HashMap<String, i32> {
        let skills = [
            ("Library Use", 20),
            ("Spot Hidden", 25),
            ("Listen", 20),
            ("Psychology", 10),
            ("Occult", 5),
            ("History", 5),
            ("First Aid", 30),
            ("Dodge", 0), // DEX/2
            ("Fighting (Brawl)", 25),
            ("Firearms (Handgun)", 20),
            ("Drive Auto", 20),
            ("Persuade", 10),
            ("Credit Rating", 0),
        ];

        skills.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    fn random_coc_occupation(rng: &mut impl Rng) -> String {
        let occupations = [
            "Antiquarian", "Author", "Dilettante", "Doctor",
            "Journalist", "Librarian", "Police Detective", "Professor",
            "Private Investigator", "Parapsychologist",
        ];
        occupations[rng.gen_range(0..occupations.len())].to_string()
    }

    // ========================================================================
    // Cyberpunk Generator
    // ========================================================================

    fn generate_cyberpunk(options: &GenerationOptions) -> Result<Character> {
        let mut rng = rand::thread_rng();

        let name = options.name.clone()
            .unwrap_or_else(|| Self::random_cyberpunk_name(&mut rng));

        let attributes = Self::roll_cyberpunk_stats(&mut rng);
        let skills = Self::default_cyberpunk_skills();

        let role = options.class.clone()
            .unwrap_or_else(|| Self::random_cyberpunk_role(&mut rng));

        let mut traits = vec![
            CharacterTrait {
                name: format!("{} Role Ability", role),
                trait_type: TraitType::Class,
                description: format!("Special ability of the {} role", role),
                mechanical_effect: Some("See rulebook for details".to_string()),
            },
        ];

        // Add some cyberware
        traits.push(CharacterTrait {
            name: "Neural Interface".to_string(),
            trait_type: TraitType::Cyberware,
            description: "Basic neural processor for interfacing with tech".to_string(),
            mechanical_effect: Some("+2 to Interface checks".to_string()),
        });

        let background = CharacterBackground {
            origin: "Night City".to_string(),
            occupation: Some(role.clone()),
            motivation: "Make it to the top".to_string(),
            connections: vec!["Fixer contact".to_string()],
            secrets: vec!["Corporate ties".to_string()],
            history: String::new(),
        };

        Ok(Character {
            id: Uuid::new_v4().to_string(),
            name,
            system: GameSystem::Cyberpunk,
            concept: options.concept.clone().unwrap_or(role),
            attributes,
            skills,
            traits,
            equipment: vec![
                Equipment {
                    name: "Agent".to_string(),
                    category: EquipmentCategory::Tech,
                    description: "Personal smartphone/computer".to_string(),
                    stats: HashMap::new(),
                },
                Equipment {
                    name: "Light Pistol".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Standard sidearm".to_string(),
                    stats: [("Damage".to_string(), "2d6".to_string())].into(),
                },
            ],
            background,
            notes: String::new(),
            portrait_prompt: None,
        })
    }

    fn roll_cyberpunk_stats(rng: &mut impl Rng) -> HashMap<String, AttributeValue> {
        let mut attributes = HashMap::new();
        let attrs = ["INT", "REF", "DEX", "TECH", "COOL", "WILL", "LUCK", "MOVE", "BODY", "EMP"];

        for attr in attrs {
            let value = rng.gen_range(2..=8);
            attributes.insert(attr.to_string(), AttributeValue::new(value));
        }

        attributes
    }

    fn default_cyberpunk_skills() -> HashMap<String, i32> {
        let skills = [
            "Athletics", "Brawling", "Concentration", "Conversation",
            "Education", "Evasion", "First Aid", "Human Perception",
            "Interface", "Stealth", "Handgun", "Melee", "Streetwise",
        ];

        skills.iter().map(|s| (s.to_string(), 2)).collect()
    }

    fn random_cyberpunk_role(rng: &mut impl Rng) -> String {
        let roles = [
            "Solo", "Netrunner", "Tech", "Media", "Exec",
            "Lawman", "Fixer", "Nomad", "Rockerboy",
        ];
        roles[rng.gen_range(0..roles.len())].to_string()
    }

    // ========================================================================
    // Fate Core Generator
    // ========================================================================

    fn generate_fate(options: &GenerationOptions) -> Result<Character> {
        let mut rng = rand::thread_rng();

        let name = options.name.clone()
            .unwrap_or_else(|| Self::random_fantasy_name(&mut rng));

        // Fate uses approaches or skills
        let skills = Self::default_fate_skills();

        // Fate characters have 5 aspects instead of traditional attributes
        let mut traits = vec![];

        // High Concept
        traits.push(CharacterTrait {
            name: "High Concept".to_string(),
            trait_type: TraitType::Aspect,
            description: options.concept.clone()
                .unwrap_or_else(|| "Mysterious Wanderer".to_string()),
            mechanical_effect: None,
        });

        // Trouble
        traits.push(CharacterTrait {
            name: "Trouble".to_string(),
            trait_type: TraitType::Aspect,
            description: "Something always goes wrong".to_string(),
            mechanical_effect: None,
        });

        // Add a stunt
        traits.push(CharacterTrait {
            name: "Lucky Break".to_string(),
            trait_type: TraitType::Stunt,
            description: "Once per session, turn a failed roll into a success".to_string(),
            mechanical_effect: Some("Spend a Fate Point to succeed with style".to_string()),
        });

        let background = CharacterBackground {
            origin: "The frontier".to_string(),
            occupation: options.class.clone(),
            motivation: "Find purpose".to_string(),
            connections: vec![],
            secrets: vec![],
            history: String::new(),
        };

        Ok(Character {
            id: Uuid::new_v4().to_string(),
            name,
            system: GameSystem::FateCore,
            concept: options.concept.clone().unwrap_or_else(|| "Fate Character".to_string()),
            attributes: HashMap::new(), // Fate doesn't use traditional attributes
            skills,
            traits,
            equipment: vec![],
            background,
            notes: "Refresh: 3\nFate Points: 3".to_string(),
            portrait_prompt: None,
        })
    }

    fn default_fate_skills() -> HashMap<String, i32> {
        // Default skill pyramid: +4, +3, +3, +2, +2, +2, +1, +1, +1, +1
        let skills = [
            ("Athletics", 2),
            ("Burglary", 1),
            ("Contacts", 1),
            ("Crafts", 0),
            ("Deceive", 1),
            ("Drive", 0),
            ("Empathy", 2),
            ("Fight", 3),
            ("Investigate", 2),
            ("Lore", 1),
            ("Notice", 3),
            ("Physique", 2),
            ("Provoke", 0),
            ("Rapport", 1),
            ("Resources", 0),
            ("Shoot", 4),
            ("Stealth", 1),
            ("Will", 2),
        ];

        skills.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    // ========================================================================
    // Shadowrun Generator
    // ========================================================================

    fn generate_shadowrun(options: &GenerationOptions) -> Result<Character> {
        let mut rng = rand::thread_rng();

        let name = options.name.clone()
            .unwrap_or_else(|| Self::random_shadowrun_name(&mut rng));

        let attributes = Self::roll_shadowrun_stats(&mut rng);
        let skills = Self::default_shadowrun_skills();

        let archetype = options.class.clone()
            .unwrap_or_else(|| Self::random_shadowrun_archetype(&mut rng));

        let metatype = options.race.clone()
            .unwrap_or_else(|| "Human".to_string());

        let mut traits = vec![
            CharacterTrait {
                name: format!("{}", metatype),
                trait_type: TraitType::Racial,
                description: format!("{} metatype traits", metatype),
                mechanical_effect: None,
            },
        ];

        // Add edge based on archetype
        traits.push(CharacterTrait {
            name: "Street Cred".to_string(),
            trait_type: TraitType::Edge,
            description: "Known in the shadows".to_string(),
            mechanical_effect: Some("+1 to social tests in the shadows".to_string()),
        });

        let background = CharacterBackground {
            origin: "Seattle Sprawl".to_string(),
            occupation: Some(archetype.clone()),
            motivation: "Survive and thrive".to_string(),
            connections: vec!["Mr. Johnson".to_string(), "Street Doc".to_string()],
            secrets: vec![],
            history: String::new(),
        };

        Ok(Character {
            id: Uuid::new_v4().to_string(),
            name,
            system: GameSystem::Shadowrun,
            concept: options.concept.clone().unwrap_or(archetype),
            attributes,
            skills,
            traits,
            equipment: vec![
                Equipment {
                    name: "Commlink".to_string(),
                    category: EquipmentCategory::Tech,
                    description: "Standard issue communication device".to_string(),
                    stats: HashMap::new(),
                },
                Equipment {
                    name: "Fake SIN".to_string(),
                    category: EquipmentCategory::Other,
                    description: "Rating 3 fake identity".to_string(),
                    stats: [("Rating".to_string(), "3".to_string())].into(),
                },
            ],
            background,
            notes: "Essence: 6.0\nEdge: 3".to_string(),
            portrait_prompt: None,
        })
    }

    fn roll_shadowrun_stats(rng: &mut impl Rng) -> HashMap<String, AttributeValue> {
        let mut attributes = HashMap::new();
        let attrs = ["Body", "Agility", "Reaction", "Strength", "Willpower", "Logic", "Intuition", "Charisma", "Edge"];

        for attr in attrs {
            let value = rng.gen_range(2..=6);
            attributes.insert(attr.to_string(), AttributeValue::new(value));
        }

        attributes
    }

    fn default_shadowrun_skills() -> HashMap<String, i32> {
        let skills = [
            "Athletics", "Close Combat", "Firearms", "Stealth",
            "Con", "Intimidation", "Negotiation", "Perception",
            "Electronics", "Hacking", "Biotech", "Engineering",
        ];

        skills.iter().map(|s| (s.to_string(), 2)).collect()
    }

    fn random_shadowrun_archetype(rng: &mut impl Rng) -> String {
        let archetypes = [
            "Street Samurai", "Decker", "Rigger", "Face",
            "Mage", "Shaman", "Adept", "Technomancer",
        ];
        archetypes[rng.gen_range(0..archetypes.len())].to_string()
    }

    // ========================================================================
    // World of Darkness Generator
    // ========================================================================

    fn generate_wod(options: &GenerationOptions) -> Result<Character> {
        let mut rng = rand::thread_rng();

        let name = options.name.clone()
            .unwrap_or_else(|| Self::random_modern_name(&mut rng));

        let attributes = Self::roll_wod_stats(&mut rng);
        let skills = Self::default_wod_skills();

        let creature_type = options.race.clone()
            .unwrap_or_else(|| "Mortal".to_string());

        let mut traits = vec![];

        // Add merits
        traits.push(CharacterTrait {
            name: "Iron Will".to_string(),
            trait_type: TraitType::Merit,
            description: "Exceptional resistance to mental influence".to_string(),
            mechanical_effect: Some("+2 to resist mind-affecting powers".to_string()),
        });

        let background = CharacterBackground {
            origin: "Modern city".to_string(),
            occupation: options.class.clone(),
            motivation: "Survive the darkness".to_string(),
            connections: vec![],
            secrets: vec!["Knows the truth about the world".to_string()],
            history: String::new(),
        };

        Ok(Character {
            id: Uuid::new_v4().to_string(),
            name,
            system: GameSystem::WorldOfDarkness,
            concept: options.concept.clone().unwrap_or(creature_type),
            attributes,
            skills,
            traits,
            equipment: vec![],
            background,
            notes: "Willpower: 5\nIntegrity: 7".to_string(),
            portrait_prompt: None,
        })
    }

    fn roll_wod_stats(rng: &mut impl Rng) -> HashMap<String, AttributeValue> {
        let mut attributes = HashMap::new();

        // Physical
        for attr in ["Strength", "Dexterity", "Stamina"] {
            attributes.insert(attr.to_string(), AttributeValue::new(rng.gen_range(1..=4)));
        }

        // Social
        for attr in ["Presence", "Manipulation", "Composure"] {
            attributes.insert(attr.to_string(), AttributeValue::new(rng.gen_range(1..=4)));
        }

        // Mental
        for attr in ["Intelligence", "Wits", "Resolve"] {
            attributes.insert(attr.to_string(), AttributeValue::new(rng.gen_range(1..=4)));
        }

        attributes
    }

    fn default_wod_skills() -> HashMap<String, i32> {
        let skills = [
            // Mental
            "Academics", "Computer", "Crafts", "Investigation", "Medicine",
            "Occult", "Politics", "Science",
            // Physical
            "Athletics", "Brawl", "Drive", "Firearms", "Larceny",
            "Stealth", "Survival", "Weaponry",
            // Social
            "Animal Ken", "Empathy", "Expression", "Intimidation",
            "Persuasion", "Socialize", "Streetwise", "Subterfuge",
        ];

        skills.iter().map(|s| (s.to_string(), 0)).collect()
    }

    // ========================================================================
    // Pathfinder 2e Generator
    // ========================================================================

    fn generate_pathfinder(options: &GenerationOptions) -> Result<Character> {
        // Very similar to D&D 5e but with some differences
        let mut rng = rand::thread_rng();

        let name = options.name.clone()
            .unwrap_or_else(|| Self::random_fantasy_name(&mut rng));

        let attributes = if options.random_stats {
            Self::roll_dnd_stats(&mut rng) // Same method works
        } else {
            Self::standard_array_dnd()
        };

        let skills = Self::default_pathfinder_skills();
        let traits = Self::generate_dnd_traits(options);

        let equipment = if options.include_equipment {
            Self::starting_equipment_dnd(options.class.as_deref())
        } else {
            vec![]
        };

        let background = CharacterBackground {
            origin: options.background.clone().unwrap_or_else(|| "Unknown".to_string()),
            occupation: options.class.clone(),
            motivation: "Adventure awaits".to_string(),
            connections: vec![],
            secrets: vec![],
            history: String::new(),
        };

        Ok(Character {
            id: Uuid::new_v4().to_string(),
            name,
            system: GameSystem::Pathfinder2e,
            concept: options.concept.clone().unwrap_or_else(|| {
                format!("{} {}",
                    options.race.as_deref().unwrap_or("Human"),
                    options.class.as_deref().unwrap_or("Adventurer")
                )
            }),
            attributes,
            skills,
            traits,
            equipment,
            background,
            notes: String::new(),
            portrait_prompt: None,
        })
    }

    fn default_pathfinder_skills() -> HashMap<String, i32> {
        let skills = [
            "Acrobatics", "Arcana", "Athletics", "Crafting",
            "Deception", "Diplomacy", "Intimidation", "Lore",
            "Medicine", "Nature", "Occultism", "Performance",
            "Religion", "Society", "Stealth", "Survival", "Thievery",
        ];

        skills.iter().map(|s| (s.to_string(), 0)).collect()
    }

    // ========================================================================
    // Name Generators
    // ========================================================================

    fn random_fantasy_name(rng: &mut impl Rng) -> String {
        let first = ["Aldric", "Branwen", "Caden", "Dara", "Elara",
                     "Finn", "Gwyn", "Hadrian", "Isolde", "Kael"];
        let last = ["Blackwood", "Ironforge", "Silverleaf", "Stormwind",
                    "Thornwood", "Winterborne", "Ravencrest", "Shadowmere"];

        format!("{} {}",
            first[rng.gen_range(0..first.len())],
            last[rng.gen_range(0..last.len())]
        )
    }

    fn random_1920s_name(rng: &mut impl Rng) -> String {
        let first = ["Arthur", "Dorothy", "Edward", "Florence", "George",
                     "Helen", "James", "Margaret", "Robert", "Virginia"];
        let last = ["Blackwell", "Crawford", "Fitzgerald", "Harrison",
                    "Montgomery", "Patterson", "Sinclair", "Thornton"];

        format!("{} {}",
            first[rng.gen_range(0..first.len())],
            last[rng.gen_range(0..last.len())]
        )
    }

    fn random_cyberpunk_name(rng: &mut impl Rng) -> String {
        let handles = ["Razor", "Chrome", "Neon", "Ghost", "Virus",
                       "Zero", "Glitch", "Spike", "Nova", "Crash"];

        handles[rng.gen_range(0..handles.len())].to_string()
    }

    fn random_shadowrun_name(rng: &mut impl Rng) -> String {
        let handles = ["Razor", "Ghost", "Demon", "Shadow", "Ice",
                       "Storm", "Viper", "Phoenix", "Wraith", "Chrome"];

        handles[rng.gen_range(0..handles.len())].to_string()
    }

    fn random_modern_name(rng: &mut impl Rng) -> String {
        let first = ["Alex", "Casey", "Jordan", "Morgan", "Quinn",
                     "Riley", "Sam", "Taylor", "Blake", "Cameron"];
        let last = ["Anderson", "Brooks", "Chen", "Davis", "Evans",
                    "Foster", "Garcia", "Hayes", "Kim", "Lewis"];

        format!("{} {}",
            first[rng.gen_range(0..first.len())],
            last[rng.gen_range(0..last.len())]
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dnd5e_generation() {
        let options = GenerationOptions {
            system: Some("dnd5e".to_string()),
            name: Some("Test Fighter".to_string()),
            class: Some("Fighter".to_string()),
            race: Some("Human".to_string()),
            random_stats: true,
            include_equipment: true,
            ..Default::default()
        };

        let character = CharacterGenerator::generate(&options).unwrap();
        assert_eq!(character.name, "Test Fighter");
        assert_eq!(character.system, GameSystem::DnD5e);
        assert!(!character.equipment.is_empty());

        // Check attributes
        assert!(character.attributes.contains_key("Strength"));
        assert!(character.attributes.contains_key("Dexterity"));
    }

    #[test]
    fn test_coc_generation() {
        let options = GenerationOptions {
            system: Some("call of cthulhu".to_string()),
            ..Default::default()
        };

        let character = CharacterGenerator::generate(&options).unwrap();
        assert_eq!(character.system, GameSystem::CallOfCthulhu);
        assert!(character.attributes.contains_key("SIZ"));
        assert!(character.attributes.contains_key("POW"));
    }

    #[test]
    fn test_cyberpunk_generation() {
        let options = GenerationOptions {
            system: Some("cyberpunk".to_string()),
            ..Default::default()
        };

        let character = CharacterGenerator::generate(&options).unwrap();
        assert_eq!(character.system, GameSystem::Cyberpunk);
        assert!(character.attributes.contains_key("REF"));
        assert!(character.attributes.contains_key("COOL"));
    }

    #[test]
    fn test_fate_generation() {
        let options = GenerationOptions {
            system: Some("fate".to_string()),
            concept: Some("Mysterious Stranger".to_string()),
            ..Default::default()
        };

        let character = CharacterGenerator::generate(&options).unwrap();
        assert_eq!(character.system, GameSystem::FateCore);
        assert!(character.traits.iter().any(|t| t.trait_type == TraitType::Aspect));
    }

    #[test]
    fn test_game_system_parsing() {
        assert_eq!(GameSystem::from_str("dnd5e"), GameSystem::DnD5e);
        assert_eq!(GameSystem::from_str("D&D"), GameSystem::DnD5e);
        assert_eq!(GameSystem::from_str("call of cthulhu"), GameSystem::CallOfCthulhu);
        assert_eq!(GameSystem::from_str("cyberpunk"), GameSystem::Cyberpunk);
        assert_eq!(GameSystem::from_str("fate"), GameSystem::FateCore);
    }
}
