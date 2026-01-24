//! Pathfinder 2nd Edition Character Generator
//!
//! Generates characters for Pathfinder 2nd Edition with its three-action economy,
//! ancestry/heritage system, and proficiency-based mechanics.

use crate::core::character_gen::{
    SystemGenerator, Character, GameSystem, GenerationOptions,
    AttributeValue, CharacterTrait, TraitType, Equipment, EquipmentCategory,
    CharacterBackground, Result, random_fantasy_name,
};
use rand::Rng;
use uuid::Uuid;
use std::collections::HashMap;

pub struct Pathfinder2eGenerator;

impl Pathfinder2eGenerator {
    pub fn new() -> Self {
        Self
    }

    fn generate_attributes(rng: &mut impl Rng, random: bool) -> HashMap<String, AttributeValue> {
        let attrs = ["Strength", "Dexterity", "Constitution", "Intelligence", "Wisdom", "Charisma"];

        if random {
            // Roll 4d6 drop lowest
            attrs.iter().map(|&attr| {
                let mut rolls: Vec<i32> = (0..4).map(|_| rng.gen_range(1..=6)).collect();
                rolls.sort();
                let total: i32 = rolls[1..].iter().sum();
                (attr.to_string(), AttributeValue::new(total))
            }).collect()
        } else {
            // Standard array equivalent
            let standard = [16, 14, 14, 12, 10, 10];
            attrs.iter().zip(standard.iter()).map(|(&attr, &val)| {
                (attr.to_string(), AttributeValue::new(val))
            }).collect()
        }
    }

    fn get_skills() -> HashMap<String, i32> {
        let skills = [
            "Acrobatics", "Arcana", "Athletics", "Crafting",
            "Deception", "Diplomacy", "Intimidation", "Lore",
            "Medicine", "Nature", "Occultism", "Performance",
            "Religion", "Society", "Stealth", "Survival", "Thievery",
        ];

        skills.iter().map(|s| (s.to_string(), 0)).collect()
    }

    fn generate_traits(options: &GenerationOptions, level: u32) -> Vec<CharacterTrait> {
        let mut traits = vec![];

        // Ancestry traits
        if let Some(ancestry) = &options.race {
            traits.extend(Self::get_ancestry_traits(ancestry));
        }

        // Class features
        if let Some(class) = &options.class {
            traits.extend(Self::get_class_features(class, level));
        }

        traits
    }

    fn get_ancestry_traits(ancestry: &str) -> Vec<CharacterTrait> {
        match ancestry.to_lowercase().as_str() {
            "human" => vec![
                CharacterTrait {
                    name: "Human".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Ambitious, versatile, and adaptable".to_string(),
                    mechanical_effect: Some("Two free ability boosts, one ancestry feat".to_string()),
                },
            ],
            "elf" => vec![
                CharacterTrait {
                    name: "Elf".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Long-lived and magically attuned".to_string(),
                    mechanical_effect: Some("Dex +2, Int +2, Con -2, Low-Light Vision".to_string()),
                },
                CharacterTrait {
                    name: "Low-Light Vision".to_string(),
                    trait_type: TraitType::Racial,
                    description: "See in dim light as if it were bright light".to_string(),
                    mechanical_effect: Some("Low-Light Vision".to_string()),
                },
            ],
            "dwarf" => vec![
                CharacterTrait {
                    name: "Dwarf".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Sturdy and traditional".to_string(),
                    mechanical_effect: Some("Con +2, Wis +2, Cha -2, Darkvision".to_string()),
                },
                CharacterTrait {
                    name: "Darkvision".to_string(),
                    trait_type: TraitType::Racial,
                    description: "See in darkness as if it were dim light".to_string(),
                    mechanical_effect: Some("Darkvision".to_string()),
                },
                CharacterTrait {
                    name: "Clan Dagger".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Traditional dwarven weapon".to_string(),
                    mechanical_effect: Some("Free clan dagger".to_string()),
                },
            ],
            "gnome" => vec![
                CharacterTrait {
                    name: "Gnome".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Curious and fey-touched".to_string(),
                    mechanical_effect: Some("Con +2, Cha +2, Str -2, Low-Light Vision".to_string()),
                },
                CharacterTrait {
                    name: "Low-Light Vision".to_string(),
                    trait_type: TraitType::Racial,
                    description: "See in dim light as if it were bright light".to_string(),
                    mechanical_effect: Some("Low-Light Vision".to_string()),
                },
            ],
            "goblin" => vec![
                CharacterTrait {
                    name: "Goblin".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Scrappy and surprisingly resilient".to_string(),
                    mechanical_effect: Some("Dex +2, Cha +2, Wis -2, Darkvision".to_string()),
                },
                CharacterTrait {
                    name: "Darkvision".to_string(),
                    trait_type: TraitType::Racial,
                    description: "See in darkness as if it were dim light".to_string(),
                    mechanical_effect: Some("Darkvision".to_string()),
                },
            ],
            "halfling" => vec![
                CharacterTrait {
                    name: "Halfling".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Optimistic and lucky".to_string(),
                    mechanical_effect: Some("Dex +2, Wis +2, Str -2, Keen Eyes".to_string()),
                },
                CharacterTrait {
                    name: "Keen Eyes".to_string(),
                    trait_type: TraitType::Racial,
                    description: "+2 to Seek for hidden or undetected creatures".to_string(),
                    mechanical_effect: Some("+2 circumstance bonus to Seek".to_string()),
                },
            ],
            "leshy" => vec![
                CharacterTrait {
                    name: "Leshy".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Plant spirits given physical form".to_string(),
                    mechanical_effect: Some("Con +2, Wis +2, Int -2, Low-Light Vision, Plant Nourishment".to_string()),
                },
            ],
            "orc" => vec![
                CharacterTrait {
                    name: "Orc".to_string(),
                    trait_type: TraitType::Racial,
                    description: "Strong and passionate".to_string(),
                    mechanical_effect: Some("Str +2, free boost, Darkvision".to_string()),
                },
                CharacterTrait {
                    name: "Darkvision".to_string(),
                    trait_type: TraitType::Racial,
                    description: "See in darkness as if it were dim light".to_string(),
                    mechanical_effect: Some("Darkvision".to_string()),
                },
            ],
            _ => vec![
                CharacterTrait {
                    name: format!("{} Ancestry", ancestry),
                    trait_type: TraitType::Racial,
                    description: format!("Traits of the {} ancestry.", ancestry),
                    mechanical_effect: None,
                },
            ],
        }
    }

    fn get_class_features(class: &str, level: u32) -> Vec<CharacterTrait> {
        let mut traits = vec![];

        match class.to_lowercase().as_str() {
            "fighter" => {
                traits.push(CharacterTrait {
                    name: "Attack of Opportunity".to_string(),
                    trait_type: TraitType::Class,
                    description: "React to enemies leaving your reach".to_string(),
                    mechanical_effect: Some("Reaction to Strike when enemy leaves reach".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Shield Block".to_string(),
                    trait_type: TraitType::Class,
                    description: "Use your shield to reduce damage".to_string(),
                    mechanical_effect: Some("Reaction to reduce damage by shield hardness".to_string()),
                });
                if level >= 3 {
                    traits.push(CharacterTrait {
                        name: "Bravery".to_string(),
                        trait_type: TraitType::Class,
                        description: "Resist fear effects".to_string(),
                        mechanical_effect: Some("Success vs. fear becomes critical success".to_string()),
                    });
                }
            }
            "wizard" => {
                traits.push(CharacterTrait {
                    name: "Arcane Spellcasting".to_string(),
                    trait_type: TraitType::Class,
                    description: "Cast arcane spells from your spellbook".to_string(),
                    mechanical_effect: Some("Prepared arcane spellcasting".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Arcane School".to_string(),
                    trait_type: TraitType::Class,
                    description: "Specialize in a school of magic".to_string(),
                    mechanical_effect: Some("School spell and focus pool".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Arcane Bond".to_string(),
                    trait_type: TraitType::Class,
                    description: "Bond with an item or familiar".to_string(),
                    mechanical_effect: Some("Free spell or familiar".to_string()),
                });
            }
            "rogue" => {
                traits.push(CharacterTrait {
                    name: "Sneak Attack".to_string(),
                    trait_type: TraitType::Class,
                    description: "Deal extra precision damage to flat-footed enemies".to_string(),
                    mechanical_effect: Some(format!("{}d6 precision damage", level.div_ceil(6))),
                });
                traits.push(CharacterTrait {
                    name: "Surprise Attack".to_string(),
                    trait_type: TraitType::Class,
                    description: "Enemies are flat-footed to you in the first round".to_string(),
                    mechanical_effect: Some("Enemies flat-footed if they haven't acted".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Racket".to_string(),
                    trait_type: TraitType::Class,
                    description: "Your criminal specialty".to_string(),
                    mechanical_effect: Some("Determines key ability and bonus features".to_string()),
                });
            }
            "cleric" => {
                traits.push(CharacterTrait {
                    name: "Divine Spellcasting".to_string(),
                    trait_type: TraitType::Class,
                    description: "Cast divine spells granted by your deity".to_string(),
                    mechanical_effect: Some("Prepared divine spellcasting".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Divine Font".to_string(),
                    trait_type: TraitType::Class,
                    description: "Channel your deity's power to heal or harm".to_string(),
                    mechanical_effect: Some("Extra heal or harm spells".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Doctrine".to_string(),
                    trait_type: TraitType::Class,
                    description: "Your approach to serving your deity".to_string(),
                    mechanical_effect: Some("Cloistered or Warpriest".to_string()),
                });
            }
            "champion" => {
                traits.push(CharacterTrait {
                    name: "Champion's Code".to_string(),
                    trait_type: TraitType::Class,
                    description: "Follow the tenets of your cause".to_string(),
                    mechanical_effect: Some("Lawful good, neutral good, or chaotic good".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Champion's Reaction".to_string(),
                    trait_type: TraitType::Class,
                    description: "React to protect allies or punish enemies".to_string(),
                    mechanical_effect: Some("Retributive Strike, Glimpse of Redemption, or Liberating Step".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Deity's Domain".to_string(),
                    trait_type: TraitType::Class,
                    description: "Access to a domain focus spell".to_string(),
                    mechanical_effect: Some("Focus spell based on deity".to_string()),
                });
            }
            "barbarian" => {
                traits.push(CharacterTrait {
                    name: "Rage".to_string(),
                    trait_type: TraitType::Class,
                    description: "Enter a battle fury".to_string(),
                    mechanical_effect: Some("Temporary HP, bonus damage, AC penalty".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Instinct".to_string(),
                    trait_type: TraitType::Class,
                    description: "The source of your rage".to_string(),
                    mechanical_effect: Some("Determines rage damage type and abilities".to_string()),
                });
                if level >= 3 {
                    traits.push(CharacterTrait {
                        name: "Deny Advantage".to_string(),
                        trait_type: TraitType::Class,
                        description: "Enemies can't make you flat-footed easily".to_string(),
                        mechanical_effect: Some("Not flat-footed to hidden, undetected, or flanking enemies".to_string()),
                    });
                }
            }
            "ranger" => {
                traits.push(CharacterTrait {
                    name: "Hunt Prey".to_string(),
                    trait_type: TraitType::Class,
                    description: "Designate a creature as your prey".to_string(),
                    mechanical_effect: Some("Bonus to Track, ignore terrain penalty".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Hunter's Edge".to_string(),
                    trait_type: TraitType::Class,
                    description: "Your hunting style".to_string(),
                    mechanical_effect: Some("Flurry, Precision, or Outwit".to_string()),
                });
            }
            "bard" => {
                traits.push(CharacterTrait {
                    name: "Occult Spellcasting".to_string(),
                    trait_type: TraitType::Class,
                    description: "Cast occult spells".to_string(),
                    mechanical_effect: Some("Spontaneous occult spellcasting".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Composition Spells".to_string(),
                    trait_type: TraitType::Class,
                    description: "Cast composition cantrips".to_string(),
                    mechanical_effect: Some("Inspire Courage, etc.".to_string()),
                });
                traits.push(CharacterTrait {
                    name: "Muse".to_string(),
                    trait_type: TraitType::Class,
                    description: "The source of your inspiration".to_string(),
                    mechanical_effect: Some("Enigma, Maestro, Polymath, or Warrior".to_string()),
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

impl Default for Pathfinder2eGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemGenerator for Pathfinder2eGenerator {
    fn system(&self) -> GameSystem {
        GameSystem::Pathfinder2e
    }

    fn generate(&self, options: &GenerationOptions) -> Result<Character> {
        let mut rng = rand::thread_rng();

        let name = options.name.clone()
            .unwrap_or_else(|| random_fantasy_name(&mut rng));

        let level = options.level.unwrap_or(1);

        let attributes = Self::generate_attributes(&mut rng, options.random_stats);
        let skills = Self::get_skills();
        let traits = Self::generate_traits(options, level);

        let equipment = if options.include_equipment {
            self.starting_equipment(options.class.as_deref())
        } else {
            vec![]
        };

        let ancestry = options.race.clone().unwrap_or_else(|| "Human".to_string());
        let class = options.class.clone().unwrap_or_else(|| "Fighter".to_string());

        let background = CharacterBackground {
            origin: options.background.clone().unwrap_or_else(|| "Farmhand".to_string()),
            occupation: Some(class.clone()),
            motivation: "Seek adventure and glory".to_string(),
            connections: vec![],
            secrets: vec![],
            history: String::new(),
        };

        Ok(Character {
            id: Uuid::new_v4().to_string(),
            name,
            system: GameSystem::Pathfinder2e,
            concept: options.concept.clone().unwrap_or_else(|| {
                format!("{} {}", ancestry, class)
            }),
            race: Some(ancestry),
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
            "Dwarf".to_string(),
            "Gnome".to_string(),
            "Goblin".to_string(),
            "Halfling".to_string(),
            "Leshy".to_string(),
            "Orc".to_string(),
            "Catfolk".to_string(),
            "Kobold".to_string(),
            "Tengu".to_string(),
            "Android".to_string(),
            "Automaton".to_string(),
            "Fetchling".to_string(),
            "Fleshwarp".to_string(),
            "Kitsune".to_string(),
            "Sprite".to_string(),
            "Strix".to_string(),
        ]
    }

    fn available_classes(&self) -> Vec<String> {
        vec![
            "Alchemist".to_string(),
            "Barbarian".to_string(),
            "Bard".to_string(),
            "Champion".to_string(),
            "Cleric".to_string(),
            "Druid".to_string(),
            "Fighter".to_string(),
            "Gunslinger".to_string(),
            "Inventor".to_string(),
            "Investigator".to_string(),
            "Kineticist".to_string(),
            "Magus".to_string(),
            "Monk".to_string(),
            "Oracle".to_string(),
            "Psychic".to_string(),
            "Ranger".to_string(),
            "Rogue".to_string(),
            "Sorcerer".to_string(),
            "Summoner".to_string(),
            "Swashbuckler".to_string(),
            "Thaumaturge".to_string(),
            "Witch".to_string(),
            "Wizard".to_string(),
        ]
    }

    fn available_backgrounds(&self) -> Vec<String> {
        vec![
            "Acolyte".to_string(),
            "Acrobat".to_string(),
            "Animal Whisperer".to_string(),
            "Artisan".to_string(),
            "Artist".to_string(),
            "Barkeep".to_string(),
            "Barrister".to_string(),
            "Bounty Hunter".to_string(),
            "Charlatan".to_string(),
            "Criminal".to_string(),
            "Detective".to_string(),
            "Emissary".to_string(),
            "Entertainer".to_string(),
            "Farmhand".to_string(),
            "Field Medic".to_string(),
            "Fortune Teller".to_string(),
            "Gambler".to_string(),
            "Gladiator".to_string(),
            "Guard".to_string(),
            "Herbalist".to_string(),
            "Hermit".to_string(),
            "Hunter".to_string(),
            "Laborer".to_string(),
            "Martial Disciple".to_string(),
            "Merchant".to_string(),
            "Miner".to_string(),
            "Noble".to_string(),
            "Nomad".to_string(),
            "Prisoner".to_string(),
            "Sailor".to_string(),
            "Scholar".to_string(),
            "Scout".to_string(),
            "Street Urchin".to_string(),
            "Tinker".to_string(),
            "Warrior".to_string(),
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

        // Common gear
        equipment.push(Equipment {
            name: "Backpack".to_string(),
            category: EquipmentCategory::Tool,
            description: "Standard adventurer's backpack".to_string(),
            stats: HashMap::new(),
        });

        equipment.push(Equipment {
            name: "Bedroll".to_string(),
            category: EquipmentCategory::Tool,
            description: "For camping".to_string(),
            stats: HashMap::new(),
        });

        equipment.push(Equipment {
            name: "Rations (2 weeks)".to_string(),
            category: EquipmentCategory::Consumable,
            description: "Trail rations".to_string(),
            stats: HashMap::new(),
        });

        // Class-specific gear
        match class.map(|s| s.to_lowercase()).as_deref() {
            Some("fighter") | Some("champion") => {
                equipment.push(Equipment {
                    name: "Longsword".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A versatile martial weapon".to_string(),
                    stats: [("Damage".to_string(), "1d8 S".to_string()), ("Traits".to_string(), "Versatile P".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Steel Shield".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "A sturdy steel shield".to_string(),
                    stats: [("AC".to_string(), "+2".to_string()), ("Hardness".to_string(), "5".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Chain Mail".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "Medium armor".to_string(),
                    stats: [("AC".to_string(), "+4".to_string()), ("Dex Cap".to_string(), "+1".to_string())].into(),
                });
            }
            Some("rogue") => {
                equipment.push(Equipment {
                    name: "Rapier".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A finesse weapon".to_string(),
                    stats: [("Damage".to_string(), "1d6 P".to_string()), ("Traits".to_string(), "Deadly d8, Disarm, Finesse".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Shortbow".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A simple ranged weapon".to_string(),
                    stats: [("Damage".to_string(), "1d6 P".to_string()), ("Range".to_string(), "60 ft.".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Leather Armor".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "Light armor".to_string(),
                    stats: [("AC".to_string(), "+1".to_string()), ("Dex Cap".to_string(), "+4".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Thieves' Tools".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "Lockpicks and similar tools".to_string(),
                    stats: HashMap::new(),
                });
            }
            Some("wizard") => {
                equipment.push(Equipment {
                    name: "Staff".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A simple wooden staff".to_string(),
                    stats: [("Damage".to_string(), "1d4 B".to_string()), ("Traits".to_string(), "Two-hand d8".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Spellbook".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "Contains your prepared spells".to_string(),
                    stats: HashMap::new(),
                });
                equipment.push(Equipment {
                    name: "Material Component Pouch".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "Contains common spell components".to_string(),
                    stats: HashMap::new(),
                });
            }
            Some("cleric") => {
                equipment.push(Equipment {
                    name: "Religious Symbol".to_string(),
                    category: EquipmentCategory::Magic,
                    description: "Symbol of your deity".to_string(),
                    stats: HashMap::new(),
                });
                equipment.push(Equipment {
                    name: "Mace".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A simple bludgeoning weapon".to_string(),
                    stats: [("Damage".to_string(), "1d6 B".to_string()), ("Traits".to_string(), "Shove".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Scale Mail".to_string(),
                    category: EquipmentCategory::Armor,
                    description: "Medium armor".to_string(),
                    stats: [("AC".to_string(), "+3".to_string()), ("Dex Cap".to_string(), "+2".to_string())].into(),
                });
            }
            _ => {
                equipment.push(Equipment {
                    name: "Dagger".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "A simple weapon".to_string(),
                    stats: [("Damage".to_string(), "1d4 P".to_string()), ("Traits".to_string(), "Agile, Finesse, Thrown 10 ft.".to_string())].into(),
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
    fn test_pf2e_generation() {
        let generator = Pathfinder2eGenerator::new();
        let options = GenerationOptions {
            system: Some("pf2e".to_string()),
            name: Some("Valeros".to_string()),
            class: Some("Fighter".to_string()),
            race: Some("Human".to_string()),
            random_stats: false,
            include_equipment: true,
            ..Default::default()
        };

        let character = generator.generate(&options).unwrap();
        assert_eq!(character.name, "Valeros");
        assert_eq!(character.system, GameSystem::Pathfinder2e);
        assert!(!character.equipment.is_empty());
    }

    #[test]
    fn test_available_options() {
        let generator = Pathfinder2eGenerator::new();
        assert!(generator.available_races().len() >= 8);
        assert!(generator.available_classes().len() >= 12);
        assert!(!generator.available_backgrounds().is_empty());
    }
}
