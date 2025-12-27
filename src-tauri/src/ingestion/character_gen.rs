use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rand::seq::SliceRandom;
use rand::Rng;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TTRPGGenre {
    Fantasy,
    SciFi,
    Cyberpunk,
    CosmicHorror,
    PostApocalyptic,
    Superhero,
    Western,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CharacterClass {
    Fighter, Wizard, Cleric, Rogue, Ranger, Paladin, Barbarian,
    Sorcerer, Warlock, Druid, Monk, Bard, Artificer,
    // Add genre specific...
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CharacterRace {
    Human, Elf, Dwarf, Halfling, Orc, Tiefling, Dragonborn, Gnome, HalfElf, HalfOrc,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterStats {
    pub strength: i32,
    pub dexterity: i32,
    pub constitution: i32,
    pub intelligence: i32,
    pub wisdom: i32,
    pub charisma: i32,
    pub hit_points: i32,
    pub max_hit_points: i32,
    pub armor_class: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    pub system: String,
    pub genre: TTRPGGenre,
    pub race: CharacterRace,
    pub class: CharacterClass,
    pub level: i32,
    pub stats: CharacterStats,
    pub equipment: Vec<String>,
    pub skills: HashMap<String, i32>,
    pub features: Vec<String>,
    pub alignment: String,
    pub backstory: String,
}

pub struct CharacterGenerator;

impl CharacterGenerator {
    pub fn generate(system: &str, level: i32, genre: Option<TTRPGGenre>) -> Character {
        let mut rng = rand::thread_rng();
        let genre = genre.unwrap_or(TTRPGGenre::Fantasy);

        let race = Self::select_random_race(&genre, &mut rng);
        let class = Self::select_random_class(&genre, &mut rng);
        let stats = Self::generate_stats(&class, &race, level);
        let name = Self::generate_name(&race, &genre, &mut rng);

        Character {
            name,
            system: system.to_string(),
            genre,
            race,
            class,
            level,
            stats,
            equipment: vec!["Backpack".to_string(), "Rations".to_string()], // Placeholder
            skills: HashMap::new(),
            features: vec![],
            alignment: "Neutral".to_string(),
            backstory: "Generated backstory...".to_string(),
        }
    }

    fn select_random_race(genre: &TTRPGGenre, rng: &mut impl Rng) -> CharacterRace {
        let races = match genre {
            TTRPGGenre::Fantasy => vec![
                CharacterRace::Human, CharacterRace::Elf, CharacterRace::Dwarf,
                CharacterRace::Halfling, CharacterRace::Orc
            ],
            _ => vec![CharacterRace::Human], // Simplify for now
        };
        races.choose(rng).cloned().unwrap_or(CharacterRace::Human)
    }

    fn select_random_class(genre: &TTRPGGenre, rng: &mut impl Rng) -> CharacterClass {
        let classes = match genre {
            TTRPGGenre::Fantasy => vec![
                CharacterClass::Fighter, CharacterClass::Wizard, CharacterClass::Rogue, CharacterClass::Cleric
            ],
            _ => vec![CharacterClass::Fighter],
        };
        classes.choose(rng).cloned().unwrap_or(CharacterClass::Fighter)
    }

    fn generate_stats(_class: &CharacterClass, _race: &CharacterRace, level: i32) -> CharacterStats {
        // Standard Array shuffle logic would go here
        CharacterStats {
            strength: 15, dexterity: 14, constitution: 13,
            intelligence: 12, wisdom: 10, charisma: 8,
            hit_points: 10 + (level * 2), // Simplified
            max_hit_points: 10 + (level * 2),
            armor_class: 10,
        }
    }

    fn generate_name(race: &CharacterRace, _genre: &TTRPGGenre, rng: &mut impl Rng) -> String {
        let first_names = match race {
            CharacterRace::Elf => vec!["Adran", "Aelar", "Aramil"],
            CharacterRace::Dwarf => vec!["Thorin", "Balin", "Gimli"],
            _ => vec!["John", "Jane", "Alex"],
        };
        let last_names = vec!["Smith", "Doe", "Lightfoot", "Stone"];

        format!("{} {}",
            first_names.choose(rng).unwrap(),
            last_names.choose(rng).unwrap()
        )
    }
}
