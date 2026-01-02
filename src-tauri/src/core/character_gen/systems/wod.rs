//! World of Darkness Character Generator
//!
//! Generates characters for Chronicles of Darkness/World of Darkness games.

use crate::core::character_gen::{
    SystemGenerator, Character, GameSystem, GenerationOptions,
    AttributeValue, CharacterTrait, TraitType, Equipment, EquipmentCategory,
    CharacterBackground, Result, random_modern_name,
};
use rand::Rng;
use uuid::Uuid;
use std::collections::HashMap;

pub struct WorldOfDarknessGenerator;

impl WorldOfDarknessGenerator {
    pub fn new() -> Self {
        Self
    }

    fn distribute_attributes(rng: &mut impl Rng) -> HashMap<String, AttributeValue> {
        // WoD uses 5/4/3 distribution across Mental/Physical/Social
        let primary = [4, 3, 2];
        let secondary = [3, 2, 2];
        let tertiary = [2, 2, 1];

        let mental = ["Intelligence", "Wits", "Resolve"];
        let physical = ["Strength", "Dexterity", "Stamina"];
        let social = ["Presence", "Manipulation", "Composure"];

        let distributions = [&primary[..], &secondary[..], &tertiary[..]];
        let categories = [&mental[..], &physical[..], &social[..]];

        // Randomize category priority
        let mut cat_order: Vec<usize> = (0..3).collect();
        for i in (1..3).rev() {
            let j = rng.gen_range(0..=i);
            cat_order.swap(i, j);
        }

        let mut attrs = HashMap::new();
        for (i, &cat_idx) in cat_order.iter().enumerate() {
            let dist = distributions[i];
            let mut vals = dist.to_vec();

            // Shuffle within category
            for j in (1..3).rev() {
                let k = rng.gen_range(0..=j);
                vals.swap(j, k);
            }

            for (j, attr) in categories[cat_idx].iter().enumerate() {
                attrs.insert(attr.to_string(), AttributeValue::new_raw(vals[j]));
            }
        }

        attrs
    }

    fn get_skills() -> HashMap<String, i32> {
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

    fn random_virtue(rng: &mut impl Rng) -> String {
        let virtues = ["Charity", "Faith", "Fortitude", "Hope", "Justice", "Prudence", "Temperance"];
        virtues[rng.gen_range(0..virtues.len())].to_string()
    }

    fn random_vice(rng: &mut impl Rng) -> String {
        let vices = ["Envy", "Gluttony", "Greed", "Lust", "Pride", "Sloth", "Wrath"];
        vices[rng.gen_range(0..vices.len())].to_string()
    }

    fn get_splat_traits(splat: &str) -> Vec<CharacterTrait> {
        match splat.to_lowercase().as_str() {
            "vampire" | "kindred" => vec![
                CharacterTrait {
                    name: "Disciplines".to_string(),
                    trait_type: TraitType::Class,
                    description: "Vampiric powers".to_string(),
                    mechanical_effect: Some("Supernatural abilities based on clan".to_string()),
                },
                CharacterTrait {
                    name: "Blood Potency".to_string(),
                    trait_type: TraitType::Class,
                    description: "The strength of vampiric blood".to_string(),
                    mechanical_effect: Some("Determines power ceiling".to_string()),
                },
                CharacterTrait {
                    name: "The Beast".to_string(),
                    trait_type: TraitType::Flaw,
                    description: "The predatory urge within".to_string(),
                    mechanical_effect: Some("Risk of frenzy".to_string()),
                },
            ],
            "werewolf" | "uratha" | "forsaken" => vec![
                CharacterTrait {
                    name: "Gifts".to_string(),
                    trait_type: TraitType::Class,
                    description: "Spirit-granted powers".to_string(),
                    mechanical_effect: Some("Supernatural abilities from spirits".to_string()),
                },
                CharacterTrait {
                    name: "Primal Urge".to_string(),
                    trait_type: TraitType::Class,
                    description: "Connection to the hunt".to_string(),
                    mechanical_effect: Some("Determines power ceiling".to_string()),
                },
                CharacterTrait {
                    name: "Death Rage".to_string(),
                    trait_type: TraitType::Flaw,
                    description: "Kuruth - the mindless killing fury".to_string(),
                    mechanical_effect: Some("Risk of uncontrolled transformation".to_string()),
                },
            ],
            "mage" | "awakened" => vec![
                CharacterTrait {
                    name: "Arcana".to_string(),
                    trait_type: TraitType::Class,
                    description: "Magical spheres of influence".to_string(),
                    mechanical_effect: Some("Supernatural abilities based on path".to_string()),
                },
                CharacterTrait {
                    name: "Gnosis".to_string(),
                    trait_type: TraitType::Class,
                    description: "Magical enlightenment".to_string(),
                    mechanical_effect: Some("Determines power ceiling".to_string()),
                },
                CharacterTrait {
                    name: "Paradox".to_string(),
                    trait_type: TraitType::Flaw,
                    description: "Reality fights back against magic".to_string(),
                    mechanical_effect: Some("Risk of paradox backlash".to_string()),
                },
            ],
            "changeling" | "lost" => vec![
                CharacterTrait {
                    name: "Contracts".to_string(),
                    trait_type: TraitType::Class,
                    description: "Fae bargains with reality".to_string(),
                    mechanical_effect: Some("Supernatural abilities".to_string()),
                },
                CharacterTrait {
                    name: "Wyrd".to_string(),
                    trait_type: TraitType::Class,
                    description: "Fae power".to_string(),
                    mechanical_effect: Some("Determines power ceiling".to_string()),
                },
                CharacterTrait {
                    name: "Clarity".to_string(),
                    trait_type: TraitType::Flaw,
                    description: "Sanity and connection to reality".to_string(),
                    mechanical_effect: Some("Can be lost to madness".to_string()),
                },
            ],
            _ => vec![
                CharacterTrait {
                    name: "Mortal".to_string(),
                    trait_type: TraitType::Background,
                    description: "An ordinary human in an extraordinary world".to_string(),
                    mechanical_effect: None,
                },
            ],
        }
    }
}

impl Default for WorldOfDarknessGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemGenerator for WorldOfDarknessGenerator {
    fn system(&self) -> GameSystem {
        GameSystem::WorldOfDarkness
    }

    fn generate(&self, options: &GenerationOptions) -> Result<Character> {
        let mut rng = rand::thread_rng();

        let name = options.name.clone()
            .unwrap_or_else(|| random_modern_name(&mut rng));

        let attributes = Self::distribute_attributes(&mut rng);
        let skills = Self::get_skills();

        let splat = options.class.clone().unwrap_or_else(|| "Mortal".to_string());

        let mut traits = Self::get_splat_traits(&splat);

        // Add virtue and vice
        let virtue = Self::random_virtue(&mut rng);
        let vice = Self::random_vice(&mut rng);

        traits.push(CharacterTrait {
            name: "Virtue".to_string(),
            trait_type: TraitType::Personality,
            description: virtue.clone(),
            mechanical_effect: Some("Regain all Willpower when acted upon at cost".to_string()),
        });

        traits.push(CharacterTrait {
            name: "Vice".to_string(),
            trait_type: TraitType::Flaw,
            description: vice.clone(),
            mechanical_effect: Some("Regain 1 Willpower when indulged".to_string()),
        });

        let equipment = if options.include_equipment {
            self.starting_equipment(Some(&splat))
        } else {
            vec![]
        };

        let background = CharacterBackground {
            origin: options.background.clone().unwrap_or_else(|| "Urban".to_string()),
            occupation: options.class.clone(),
            motivation: format!("Guided by {} and tempted by {}", virtue, vice),
            connections: vec!["Family".to_string(), "Work contact".to_string()],
            secrets: vec!["Something you've witnessed".to_string()],
            history: String::new(),
        };

        // Calculate derived stats
        let size = 5;
        let health = attributes.get("Stamina").map(|a| a.base).unwrap_or(2) + size;
        let willpower = attributes.get("Resolve").map(|a| a.base).unwrap_or(2)
            + attributes.get("Composure").map(|a| a.base).unwrap_or(2);

        Ok(Character {
            id: Uuid::new_v4().to_string(),
            name,
            system: GameSystem::WorldOfDarkness,
            concept: options.concept.clone().unwrap_or_else(|| format!("{} {}", splat, "Character")),
            race: None,
            class: Some(splat),
            level: 1,
            attributes,
            skills,
            traits,
            equipment,
            background,
            backstory: None,
            notes: format!("Health: {}\nWillpower: {}\nVirtue: {}\nVice: {}", health, willpower, virtue, vice),
            portrait_prompt: None,
        })
    }

    fn available_races(&self) -> Vec<String> {
        vec!["Human".to_string()]
    }

    fn available_classes(&self) -> Vec<String> {
        vec![
            "Mortal".to_string(),
            "Vampire".to_string(),
            "Werewolf".to_string(),
            "Mage".to_string(),
            "Changeling".to_string(),
            "Hunter".to_string(),
            "Geist".to_string(),
            "Promethean".to_string(),
            "Demon".to_string(),
            "Beast".to_string(),
        ]
    }

    fn available_backgrounds(&self) -> Vec<String> {
        vec![
            "Allies".to_string(),
            "Contacts".to_string(),
            "Fame".to_string(),
            "Mentor".to_string(),
            "Resources".to_string(),
            "Status".to_string(),
            "Retainer".to_string(),
        ]
    }

    fn attribute_names(&self) -> Vec<String> {
        vec![
            "Intelligence".to_string(), "Wits".to_string(), "Resolve".to_string(),
            "Strength".to_string(), "Dexterity".to_string(), "Stamina".to_string(),
            "Presence".to_string(), "Manipulation".to_string(), "Composure".to_string(),
        ]
    }

    fn starting_equipment(&self, splat: Option<&str>) -> Vec<Equipment> {
        let mut equipment = vec![
            Equipment {
                name: "Smartphone".to_string(),
                category: EquipmentCategory::Tool,
                description: "Essential modern device".to_string(),
                stats: HashMap::new(),
            },
            Equipment {
                name: "Wallet/Purse".to_string(),
                category: EquipmentCategory::Tool,
                description: "Contains ID and some cash".to_string(),
                stats: HashMap::new(),
            },
        ];

        match splat.map(|s| s.to_lowercase()).as_deref() {
            Some("hunter") => {
                equipment.push(Equipment {
                    name: "Compact Pistol".to_string(),
                    category: EquipmentCategory::Weapon,
                    description: "Concealed firearm".to_string(),
                    stats: [("Damage".to_string(), "2".to_string())].into(),
                });
                equipment.push(Equipment {
                    name: "Hunter Kit".to_string(),
                    category: EquipmentCategory::Tool,
                    description: "Stakes, holy water, silver knife".to_string(),
                    stats: HashMap::new(),
                });
            }
            Some("vampire") | Some("kindred") => {
                equipment.push(Equipment {
                    name: "Designer Clothes".to_string(),
                    category: EquipmentCategory::Other,
                    description: "Projecting the right image".to_string(),
                    stats: HashMap::new(),
                });
            }
            _ => {}
        }

        equipment
    }
}
