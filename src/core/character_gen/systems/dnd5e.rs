//! D&D 5th Edition Character Generator
//!
//! Generates characters for Dungeons & Dragons 5th Edition.

use crate::core::character_gen::{
    SystemGenerator, Character, GameSystem, GenerationOptions,
    AttributeValue, CharacterTrait, TraitType, Equipment, EquipmentCategory,
    CharacterBackground, Result, random_fantasy_name,
};
use rand::Rng;
use uuid::Uuid;
use std::collections::HashMap;

pub struct DnD5eGenerator;

impl DnD5eGenerator {
    pub fn new() -> Self {
        Self
    }

    fn roll_stats(rng: &mut impl Rng) -> HashMap<String, AttributeValue> {
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

    fn standard_array() -> HashMap<String, AttributeValue> {
        let mut attributes = HashMap::new();
        let standard = [15, 14, 13, 12, 10, 8];
        let attrs = ["Strength", "Dexterity", "Constitution", "Intelligence", "Wisdom", "Charisma"];

        for (attr, &val) in attrs.iter().zip(standard.iter()) {
            attributes.insert(attr.to_string(), AttributeValue::new(val));
        }

        attributes
    }

    fn get_skills() -> HashMap<String, i32> {
        let skills = [
            "Acrobatics", "Animal Handling", "Arcana", "Athletics",
            "Deception", "History", "Insight", "Intimidation",
            "Investigation", "Medicine", "Nature", "Perception",
            "Performance", "Persuasion", "Religion", "Sleight of Hand",
            "Stealth", "Survival",
        ];

        skills.iter().map(|s| (s.to_string(), 0)).collect()
    }

    fn generate_traits(options: &GenerationOptions) -> Vec<CharacterTrait> {
        let mut traits = vec![];

        // Add racial trait
        if let Some(race) = &options.race {
            let racial_traits = Self::get_racial_traits(race);
            traits.extend(racial_traits);
        }

        // Add class feature
        if let Some(class) = &options.class {
            let class_traits = Self::get_class_traits(class, options.level.unwrap_or(1));
            traits.extend(class_traits);
        }

        traits
    }

    fn get_racial_traits(race: &str) -> Vec<CharacterTrait> {
        match race.to_lowercase().as_str() {
            "human" => vec![
                CharacterTrait {
                    name: "Versatile".to_string(),
                    trait_type: TraitType::Racial,
                    description: "+1 to all ability scores".to_string(),
                    mechanical_effect: Some("+1 to all ability scores".to_string()),
                },
            ],
            "elf" | "high elf" | "wood elf" | "dark elf" | "drow" => vec![
                CharacterTrait {
                    name: "Darkvision".to_string(),
                    trait_type: TraitType::Racial,
                    description: "See in dim light within 60 feet as if it were bright light".to_string(),
                    mechanical_effect: Some("60 ft. darkvision".to_string()),
                },
                CharacterTrait {
                    name: "Fey Ancestry".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Advantage on saving throws against being charmed, immune to magical sleep".to_string(),
                    mechanical_effect: Some("Advantage vs. charm, immune to magical sleep".to_string()),
                },
                CharacterTrait {
                    name: "Trance".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Elves don't sleep. They meditate deeply for 4 hours a day".to_string(),
                    mechanical_effect: Some("4-hour trance instead of 8-hour sleep".to_string()),
                },
            ],
            "dwarf" | "hill dwarf" | "mountain dwarf" => vec![
                CharacterTrait {
                    name: "Darkvision".to_string(),
                    trait_type: TraitType::Racial,
                    description: "See in dim light within 60 feet as if it were bright light".to_string(),
                    mechanical_effect: Some("60 ft. darkvision".to_string()),
                },
                CharacterTrait {
                    name: "Dwarven Resilience".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Advantage on saving throws against poison, resistance to poison damage".to_string(),
                    mechanical_effect: Some("Advantage vs. poison, poison resistance".to_string()),
                },
                CharacterTrait {
                    name: "Stonecunning".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Double proficiency bonus on History checks related to stonework".to_string(),
                    mechanical_effect: Some("Expertise on stonework History checks".to_string()),
                },
            ],
            "halfling" | "lightfoot halfling" | "stout halfling" => vec![
                CharacterTrait {
                    name: "Lucky".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Reroll natural 1s on attack rolls, ability checks, and saving throws".to_string(),
                    mechanical_effect: Some("Reroll natural 1s".to_string()),
                },
                CharacterTrait {
                    name: "Brave".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Advantage on saving throws against being frightened".to_string(),
                    mechanical_effect: Some("Advantage vs. frightened".to_string()),
                },
                CharacterTrait {
                    name: "Halfling Nimbleness".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Move through the space of any creature larger than you".to_string(),
                    mechanical_effect: Some("Move through larger creatures' spaces".to_string()),
                },
            ],
            "dragonborn" => vec![
                CharacterTrait {
                    name: "Draconic Ancestry".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Choose a dragon type for breath weapon and damage resistance".to_string(),
                    mechanical_effect: Some("Breath weapon, damage resistance".to_string()),
                },
                CharacterTrait {
                    name: "Breath Weapon".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Exhale destructive energy based on draconic ancestry".to_string(),
                    mechanical_effect: Some("Use action to deal damage in area".to_string()),
                },
            ],
            "gnome" | "rock gnome" | "forest gnome" => vec![
                CharacterTrait {
                    name: "Darkvision".to_string(),
                    trait_type: TraitType::Racial,
                    description: "See in dim light within 60 feet as if it were bright light".to_string(),
                    mechanical_effect: Some("60 ft. darkvision".to_string()),
                },
                CharacterTrait {
                    name: "Gnome Cunning".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Advantage on all Intelligence, Wisdom, and Charisma saving throws against magic".to_string(),
                    mechanical_effect: Some("Advantage on mental saves vs. magic".to_string()),
                },
            ],
            "half-elf" => vec![
                CharacterTrait {
                    name: "Darkvision".to_string(),
                    trait_type: TraitType::Racial,
                    description: "See in dim light within 60 feet as if it were bright light".to_string(),
                    mechanical_effect: Some("60 ft. darkvision".to_string()),
                },
                CharacterTrait {
                    name: "Fey Ancestry".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Advantage on saving throws against being charmed, immune to magical sleep".to_string(),
                    mechanical_effect: Some("Advantage vs. charm, immune to magical sleep".to_string()),
                },
                CharacterTrait {
                    name: "Skill Versatility".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Proficiency in two skills of your choice".to_string(),
                    mechanical_effect: Some("+2 skill proficiencies".to_string()),
                },
            ],
            "half-orc" => vec![
                CharacterTrait {
                    name: "Darkvision".to_string(),
                    trait_type: TraitType::Racial,
                    description: "See in dim light within 60 feet as if it were bright light".to_string(),
                    mechanical_effect: Some("60 ft. darkvision".to_string()),
                },
                CharacterTrait {
                    name: "Relentless Endurance".to_string(),
                    trait_type: TraitType::Racial,
                    description: "When reduced to 0 HP but not killed, drop to 1 HP instead (once per long rest)".to_string(),
                    mechanical_effect: Some("Drop to 1 HP instead of 0 once per long rest".to_string()),
                },
                CharacterTrait {
                    name: "Savage Attacks".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Roll one additional damage die on critical hits with melee weapons".to_string(),
                    mechanical_effect: Some("Extra damage die on melee crits".to_string()),
                },
            ],
            "tiefling" => vec![
                CharacterTrait {
                    name: "Darkvision".to_string(),
                    trait_type: TraitType::Racial,
                    description: "See in dim light within 60 feet as if it were bright light".to_string(),
                    mechanical_effect: Some("60 ft. darkvision".to_string()),
                },
                CharacterTrait {
                    name: "Hellish Resistance".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Resistance to fire damage".to_string(),
                    mechanical_effect: Some("Fire resistance".to_string()),
                },
                CharacterTrait {
                    name: "Infernal Legacy".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Know the thaumaturgy cantrip; gain spells at higher levels".to_string(),
                    mechanical_effect: Some("Thaumaturgy, Hellish Rebuke at 3rd, Darkness at 5th".to_string()),
                },
            ],
            _ => vec![
                CharacterTrait {
                    name: format!("{} Heritage", race),
                    trait_type: TraitType::Racial,
                    description: format!("The cultural and biological traits of the {} people.", race),
                    mechanical_effect: None,
                },
            ],
        }
    }

    fn get_class_traits(class: &str, level: u32) -> Vec<CharacterTrait> {
        let mut traits = vec![];

        match class.to_lowercase().as_str() {
            "fighter" => {
                traits.push(CharacterTrait {
                    name: "Fighting Style".to_string(),
                    trait_type: TraitType::Class,
                    description: "Specialized combat technique".to_string(),
                    mechanical_effect: Some("Choose a fighting style".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Second Wind".to_string(),
                    trait_type: TraitType::Class,
                    description: "Regain hit points as a bonus action".to_string(),
                    mechanical_effect: Some("Bonus action: heal 1d10 + level HP".to_string()),
                });
                if level >= 2 {
                    traits.push(CharacterTrait {
                        name: "Action Surge".to_string(),
                        trait_type: TraitType::Class,
                        description: "Take one additional action on your turn".to_string(),
                        mechanical_effect: Some("One extra action per short rest".to_string()),
                    });
                }
            }
            "wizard" => {
                traits.push(CharacterTrait {
                    name: "Spellcasting".to_string(),
                    trait_type: TraitType::Class,
                    description: "Cast wizard spells using Intelligence".to_string(),
                    mechanical_effect: Some("Intelligence-based spellcasting".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Arcane Recovery".to_string(),
                    trait_type: TraitType::Class,
                    description: "Recover spell slots during a short rest".to_string(),
                    mechanical_effect: Some("Recover spell slots = half wizard level".to_string()),
                });
            }
            "rogue" => {
                traits.push(CharacterTrait {
                    name: "Sneak Attack".to_string(),
                    trait_type: TraitType::Class,
                    description: "Deal extra damage when you have advantage or an ally nearby".to_string(),
                    mechanical_effect: Some(format!("{}d6 extra damage", level.div_ceil(2))),
                });
                traits.push(CharacterTrait {
                    name: "Thieves' Cant".to_string(),
                    trait_type: TraitType::Class,
                    description: "Secret language of thieves and rogues".to_string(),
                    mechanical_effect: Some("Speak Thieves' Cant".to_string()),
                });
                if level >= 2 {
                    traits.push(CharacterTrait {
                        name: "Cunning Action".to_string(),
                        trait_type: TraitType::Class,
                        description: "Dash, Disengage, or Hide as a bonus action".to_string(),
                        mechanical_effect: Some("Bonus action: Dash/Disengage/Hide".to_string()),
                    });
                }
            }
            "cleric" => {
                traits.push(CharacterTrait {
                    name: "Spellcasting".to_string(),
                    trait_type: TraitType::Class,
                    description: "Cast cleric spells using Wisdom".to_string(),
                    mechanical_effect: Some("Wisdom-based spellcasting".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Divine Domain".to_string(),
                    trait_type: TraitType::Class,
                    description: "Choose a divine domain for bonus spells and features".to_string(),
                    mechanical_effect: Some("Domain spells and Channel Divinity".to_string()),
                });
            }
            "paladin" => {
                traits.push(CharacterTrait {
                    name: "Divine Sense".to_string(),
                    trait_type: TraitType::Class,
                    description: "Detect celestials, fiends, and undead".to_string(),
                    mechanical_effect: Some("Detect supernatural beings 60 ft.".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Lay on Hands".to_string(),
                    trait_type: TraitType::Class,
                    description: "Heal creatures with your touch".to_string(),
                    mechanical_effect: Some(format!("Heal pool: {} HP", level * 5)),
                });
                if level >= 2 {
                    traits.push(CharacterTrait {
                        name: "Divine Smite".to_string(),
                        trait_type: TraitType::Class,
                        description: "Expend spell slots to deal extra radiant damage".to_string(),
                        mechanical_effect: Some("2d8 + 1d8 per slot level above 1st".to_string()),
                    });
                }
            }
            "barbarian" => {
                traits.push(CharacterTrait {
                    name: "Rage".to_string(),
                    trait_type: TraitType::Class,
                    description: "Enter a battle fury for bonus damage and resistances".to_string(),
                    mechanical_effect: Some("Bonus damage, resistance to physical damage".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Unarmored Defense".to_string(),
                    trait_type: TraitType::Class,
                    description: "Add Constitution modifier to AC when not wearing armor".to_string(),
                    mechanical_effect: Some("AC = 10 + Dex + Con".to_string()),
                });
            }
            "bard" => {
                traits.push(CharacterTrait {
                    name: "Spellcasting".to_string(),
                    trait_type: TraitType::Class,
                    description: "Cast bard spells using Charisma".to_string(),
                    mechanical_effect: Some("Charisma-based spellcasting".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Bardic Inspiration".to_string(),
                    trait_type: TraitType::Class,
                    description: "Inspire allies with your performance".to_string(),
                    mechanical_effect: Some(format!("d{} inspiration die, {} uses", if level >= 5 { 8 } else { 6 }, level.max(1))),
                });
            }
            "druid" => {
                traits.push(CharacterTrait {
                    name: "Spellcasting".to_string(),
                    trait_type: TraitType::Class,
                    description: "Cast druid spells using Wisdom".to_string(),
                    mechanical_effect: Some("Wisdom-based spellcasting".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Druidic".to_string(),
                    trait_type: TraitType::Class,
                    description: "Secret language of druids".to_string(),
                    mechanical_effect: Some("Speak Druidic".to_string()),
                });
                if level >= 2 {
                    traits.push(CharacterTrait {
                        name: "Wild Shape".to_string(),
                        trait_type: TraitType::Class,
                        description: "Transform into beasts you have seen".to_string(),
                        mechanical_effect: Some("Transform 2x per short rest".to_string()),
                    });
                }
            }
            "monk" => {
                traits.push(CharacterTrait {
                    name: "Unarmored Defense".to_string(),
                    trait_type: TraitType::Class,
                    description: "Add Wisdom modifier to AC when not wearing armor".to_string(),
                    mechanical_effect: Some("AC = 10 + Dex + Wis".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Martial Arts".to_string(),
                    trait_type: TraitType::Class,
                    description: "Use Dexterity for unarmed strikes and monk weapons".to_string(),
                    mechanical_effect: Some(format!("d{} unarmed damage", if level >= 5 { 6 } else { 4 })),
                });
                if level >= 2 {
                    traits.push(CharacterTrait {
                        name: "Ki".to_string(),
                        trait_type: TraitType::Class,
                        description: "Harness mystical energy for special abilities".to_string(),
                        mechanical_effect: Some(format!("{} ki points", level)),
                    });
                }
            }
            "ranger" => {
                traits.push(CharacterTrait {
                    name: "Favored Enemy".to_string(),
                    trait_type: TraitType::Class,
                    description: "Advantage on tracking and recalling information about chosen enemy type".to_string(),
                    mechanical_effect: Some("Advantage on Survival and Intelligence checks".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Natural Explorer".to_string(),
                    trait_type: TraitType::Class,
                    description: "Expertise in navigating and surviving in chosen terrain".to_string(),
                    mechanical_effect: Some("Benefits in favored terrain".to_string()),
                });
            }
            "sorcerer" => {
                traits.push(CharacterTrait {
                    name: "Spellcasting".to_string(),
                    trait_type: TraitType::Class,
                    description: "Cast sorcerer spells using Charisma".to_string(),
                    mechanical_effect: Some("Charisma-based spellcasting".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Sorcerous Origin".to_string(),
                    trait_type: TraitType::Class,
                    description: "The source of your magical power".to_string(),
                    mechanical_effect: Some("Origin features".to_string()),
                });
                if level >= 2 {
                    traits.push(CharacterTrait {
                        name: "Font of Magic".to_string(),
                        trait_type: TraitType::Class,
                        description: "Sorcery points for metamagic and spell slot creation".to_string(),
                        mechanical_effect: Some(format!("{} sorcery points", level)),
                    });
                }
            }
            "warlock" => {
                traits.push(CharacterTrait {
                    name: "Pact Magic".to_string(),
                    trait_type: TraitType::Class,
                    description: "Cast warlock spells using Charisma, slots refresh on short rest".to_string(),
                    mechanical_effect: Some("Short rest spell slot recovery".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Otherworldly Patron".to_string(),
                    trait_type: TraitType::Class,
                    description: "The entity that grants you power".to_string(),
                    mechanical_effect: Some("Patron features and expanded spells".to_string()),
                });
            }
            _ => {
                traits.push(CharacterTrait {
                    name: format!("{} Training", class),
                    trait_type: TraitType::Class,
                    description: format!("Trained in the ways of the {}.", class),
                    mechanical_effect: None,
                });
            }
        }

        traits
    }
}

impl Default for DnD5eGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemGenerator for DnD5eGenerator {
    fn system(&self) -> GameSystem {
        GameSystem::DnD5e
    }

    fn generate(&self, options: &GenerationOptions) -> Result<Character> {
        let mut rng = rand::thread_rng();

        let name = options.name.clone()
            .unwrap_or_else(|| random_fantasy_name(&mut rng));

        let level = options.level.unwrap_or(1);

        let attributes = if options.random_stats {
            Self::roll_stats(&mut rng)
        } else {
            Self::standard_array()
        };

        let skills = Self::get_skills();
        let traits = Self::generate_traits(options);

        let equipment = if options.include_equipment {
            self.starting_equipment(options.class.as_deref())
        } else {
            vec![]
        };

        let race = options.race.clone().unwrap_or_else(|| "Human".to_string());
        let class = options.class.clone().unwrap_or_else(|| "Fighter".to_string());

        let background = CharacterBackground {
            origin: options.background.clone().unwrap_or_else(|| "Folk Hero".to_string()),
            occupation: Some(class.clone()),
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
                format!("{} {}", race, class)
            }),
            race: Some(race),
            class: Some(class),
            level,
            attributes,
            skills,
            traits,
            equipment,
            background,
            backstory: None,
            notes: String::new(),
            portrait_prompt: None,
        })
    }

    fn available_races(&self) -> Vec<String> {
        vec![
            "Human".to_string(),
            "Elf".to_string(),
            "High Elf".to_string(),
            "Wood Elf".to_string(),
            "Drow".to_string(),
            "Dwarf".to_string(),
            "Hill Dwarf".to_string(),
            "Mountain Dwarf".to_string(),
            "Halfling".to_string(),
            "Lightfoot Halfling".to_string(),
            "Stout Halfling".to_string(),
            "Dragonborn".to_string(),
            "Gnome".to_string(),
            "Half-Elf".to_string(),
            "Half-Orc".to_string(),
            "Tiefling".to_string(),
        ]
    }

    fn available_classes(&self) -> Vec<String> {
        vec![
            "Barbarian".to_string(),
            "Bard".to_string(),
            "Cleric".to_string(),
            "Druid".to_string(),
            "Fighter".to_string(),
            "Monk".to_string(),
            "Paladin".to_string(),
            "Ranger".to_string(),
            "Rogue".to_string(),
            "Sorcerer".to_string(),
            "Warlock".to_string(),
            "Wizard".to_string(),
        ]
    }

    fn available_backgrounds(&self) -> Vec<String> {
        vec![
            "Acolyte".to_string(),
            "Charlatan".to_string(),
            "Criminal".to_string(),
            "Entertainer".to_string(),
            "Folk Hero".to_string(),
            "Guild Artisan".to_string(),
            "Hermit".to_string(),
            "Noble".to_string(),
            "Outlander".to_string(),
            "Sage".to_string(),
            "Sailor".to_string(),
            "Soldier".to_string(),
            "Urchin".to_string(),
        ]
    }

    fn attribute_names(&self) -> Vec<String> {
        vec![
            "Strength".to_string(),
            "Dexterity".to_string(),
            "Constitution".to_string(),
            "Intelligence".to_string(),
            "Wisdom".to_string(),
            "Charisma".to_string(),
        ]
    }

    fn starting_equipment(&self, class: Option<&str>) -> Vec<Equipment> {
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

        equipment.push(Equipment {
            name: "Rations (10 days)".to_string(),
            category: EquipmentCategory::Consumable,
            description: "Trail rations".to_string(),
            stats: HashMap::new(),
        });

        equipment.push(Equipment {
            name: "Waterskin".to_string(),
            category: EquipmentCategory::Tool,
            description: "Leather water container".to_string(),
            stats: HashMap::new(),
        });

        // Class-specific equipment
        match class.map(|s| s.to_lowercase()).as_deref() {
            Some("fighter") | Some("paladin") => {
                equipment.push(Equipment {
                    name: "Longsword".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A versatile martial weapon".to_string(),
                    stats: [("Damage".to_string(), "1d8 slashing (1d10 versatile)".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Shield".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "A sturdy wooden shield".to_string(),
                    stats: [("AC".to_string(), "+2".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Chain Mail".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "Heavy armor made of interlocking rings".to_string(),
                    stats: [("AC".to_string(), "16".to_string())].into(),
                });
            }
            Some("rogue") => {
                equipment.push(Equipment {
                    name: "Shortsword".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A finesse weapon perfect for quick strikes".to_string(),
                    stats: [("Damage".to_string(), "1d6 piercing".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Shortbow".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A simple ranged weapon".to_string(),
                    stats: [("Damage".to_string(), "1d6 piercing".to_string()), ("Range".to_string(), "80/320 ft.".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Leather Armor".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "Light armor made of leather".to_string(),
                    stats: [("AC".to_string(), "11 + Dex".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Thieves' Tools".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "Lockpicks and other tools of the trade".to_string(),
                    stats: HashMap::new(),
                });
            }
            Some("wizard") => {
                equipment.push(Equipment {
                    name: "Quarterstaff".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A simple wooden staff".to_string(),
                    stats: [("Damage".to_string(), "1d6 bludgeoning (1d8 versatile)".to_string())].into(),
                });
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
                equipment.push(Equipment {
                    name: "Component Pouch".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "Contains common spell components".to_string(),
                    stats: HashMap::new(),
                });
            }
            Some("cleric") => {
                equipment.push(Equipment {
                    name: "Mace".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A simple bludgeoning weapon".to_string(),
                    stats: [("Damage".to_string(), "1d6 bludgeoning".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Scale Mail".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "Medium armor of overlapping metal scales".to_string(),
                    stats: [("AC".to_string(), "14 + Dex (max 2)".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Shield".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "A sturdy wooden shield".to_string(),
                    stats: [("AC".to_string(), "+2".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Holy Symbol".to_string(),
                    category: EquipmentCategory::Magic,
                    description: "A symbol of your deity".to_string(),
                    stats: HashMap::new(),
                });
            }
            Some("barbarian") => {
                equipment.push(Equipment {
                    name: "Greataxe".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A massive two-handed axe".to_string(),
                    stats: [("Damage".to_string(), "1d12 slashing".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Handaxe".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A light throwing axe".to_string(),
                    stats: [("Damage".to_string(), "1d6 slashing".to_string()), ("Range".to_string(), "20/60 ft.".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Handaxe".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A light throwing axe".to_string(),
                    stats: [("Damage".to_string(), "1d6 slashing".to_string()), ("Range".to_string(), "20/60 ft.".to_string())].into(),
                });
            }
            Some("ranger") => {
                equipment.push(Equipment {
                    name: "Longbow".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A powerful ranged weapon".to_string(),
                    stats: [("Damage".to_string(), "1d8 piercing".to_string()), ("Range".to_string(), "150/600 ft.".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Quiver".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "Contains 20 arrows".to_string(),
                    stats: HashMap::new(),
                });
                equipment.push(Equipment {
                    name: "Shortsword".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A finesse weapon".to_string(),
                    stats: [("Damage".to_string(), "1d6 piercing".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Leather Armor".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "Light armor made of leather".to_string(),
                    stats: [("AC".to_string(), "11 + Dex".to_string())].into(),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dnd5e_generation() {
        let generator = DnD5eGenerator::new();
        let options = GenerationOptions {
            system: Some("dnd5e".to_string()),
            name: Some("Test Fighter".to_string()),
            class: Some("Fighter".to_string()),
            race: Some("Human".to_string()),
            random_stats: true,
            include_equipment: true,
            ..Default::default()
        };

        let character = generator.generate(&options).unwrap();
        assert_eq!(character.name, "Test Fighter");
        assert_eq!(character.system, GameSystem::DnD5e);
        assert!(!character.equipment.is_empty());
        assert!(character.attributes.contains_key("Strength"));
        assert!(character.attributes.contains_key("Dexterity"));
    }

    #[test]
    fn test_available_options() {
        let generator = DnD5eGenerator::new();
        assert!(!generator.available_races().is_empty());
        assert!(!generator.available_classes().is_empty());
        assert!(!generator.available_backgrounds().is_empty());
        assert_eq!(generator.attribute_names().len(), 6);
    }
}
