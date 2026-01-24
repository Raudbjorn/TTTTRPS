use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rand::seq::SliceRandom;
use rand::Rng;

/// TTRPG genre/setting categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TTRPGGenre {
    // Core fantasy variants
    Fantasy,
    HighFantasy,
    LowFantasy,
    DarkFantasy,
    UrbanFantasy,
    Grimdark,

    // Science fiction variants
    SciFi,
    SpaceOpera,
    HardSciFi,
    Cyberpunk,
    PostCyberpunk,
    Biopunk,
    Solarpunk,

    // Horror variants
    CosmicHorror,
    GothicHorror,
    SurvivalHorror,
    FolkHorror,

    // Historical/Period
    Historical,
    Medieval,
    Viking,
    Roman,
    Egyptian,
    Samurai,
    WuxiaWulin,

    // Other genres
    PostApocalyptic,
    Superhero,
    Western,
    Steampunk,
    Dieselpunk,
    Noir,
    Pulp,
    Swashbuckling,
    Mythology,
    FairyTale,

    // Mixed/Modern
    ModernDay,
    MilitaryTactical,
    Espionage,

    // User-defined
    Custom(String),
}

/// Character class/archetype
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CharacterClass {
    // Fantasy classes
    Fighter, Wizard, Cleric, Rogue, Ranger, Paladin, Barbarian,
    Sorcerer, Warlock, Druid, Monk, Bard, Artificer, Necromancer,
    Knight, Spellblade, Shaman, Witch, Alchemist, Summoner,

    // Sci-Fi classes
    Soldier, Pilot, Engineer, Scientist, Hacker, Medic, Psionic,
    BountyHunter, Smuggler, MechPilot, StarshipCaptain,

    // Cyberpunk classes
    Netrunner, Solo, Techie, Fixer, Rockerboy, Nomad, Corporate,
    MediaOp, Lawman, MedTech,

    // Horror classes
    Investigator, Occultist, Hunter, Survivor, Medium, Exorcist,

    // Western classes
    Gunslinger, Sheriff, Outlaw, Prospector, Gambler, Preacher,
    Bounty, Marshal, Rancher,

    // Steampunk classes
    Inventor, Aeronaut, Clockworker, Gadgeteer, SteamKnight,

    // Noir classes
    Detective, Gumshoe, Femme, Enforcer, Journalist, Snitch,

    // Superhero classes
    Paragon, Vigilante, Mastermind, Mystic, Mutant, Gadgeteer2,

    // Historical/Period
    Gladiator, Centurion, Samurai, Ronin, Viking, Berserker,
    Crusader, Inquisitor, Courtier, Merchant, Scholar,

    // Wuxia/Martial arts
    MartialArtist, SwordSaint, QiMaster, AssassinBlade, MonkWarrior,

    // Swashbuckling
    Duelist, Pirate, Musketeer, Privateer, Corsair,

    // Modern
    Operative, Specialist, Tactician, Diplomat, Analyst, Explorer,

    // Custom
    Custom(String),
}

/// Character race/species/origin
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CharacterRace {
    // Fantasy races
    Human, Elf, Dwarf, Halfling, Orc, Tiefling, Dragonborn, Gnome,
    HalfElf, HalfOrc, Aasimar, Genasi, Goliath, Firbolg, Kenku,
    Tabaxi, Triton, Tortle, Aarakocra, Goblin, Hobgoblin, Bugbear,
    Kobold, Lizardfolk, Yuan, Changeling, Shifter, Warforged,

    // Dark fantasy
    Revenant, Dhampir, Hexblood, Reborn,

    // Sci-Fi species
    Android, Cyborg, Clone, Mutant, Alien, Uplifted, Synthetic,
    HybridSpecies, GeneModded,

    // Horror
    Cursed, Touched, Haunted, Infected,

    // Mythology
    Demigod, Nephilim, Fae, Elemental, Spirit, Yokai,

    // Custom
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
            // Fantasy variants
            TTRPGGenre::Fantasy | TTRPGGenre::HighFantasy => vec![
                CharacterRace::Human, CharacterRace::Elf, CharacterRace::Dwarf,
                CharacterRace::Halfling, CharacterRace::Gnome, CharacterRace::HalfElf,
                CharacterRace::Tiefling, CharacterRace::Dragonborn, CharacterRace::Aasimar,
            ],
            TTRPGGenre::LowFantasy | TTRPGGenre::Medieval => vec![
                CharacterRace::Human, CharacterRace::HalfElf, CharacterRace::HalfOrc,
            ],
            TTRPGGenre::DarkFantasy | TTRPGGenre::Grimdark => vec![
                CharacterRace::Human, CharacterRace::Tiefling, CharacterRace::Dhampir,
                CharacterRace::Hexblood, CharacterRace::Revenant, CharacterRace::Reborn,
            ],
            TTRPGGenre::UrbanFantasy => vec![
                CharacterRace::Human, CharacterRace::Changeling, CharacterRace::Fae,
                CharacterRace::Dhampir, CharacterRace::Shifter,
            ],

            // Sci-Fi variants
            TTRPGGenre::SciFi | TTRPGGenre::SpaceOpera | TTRPGGenre::HardSciFi => vec![
                CharacterRace::Human, CharacterRace::Android, CharacterRace::Cyborg,
                CharacterRace::Alien, CharacterRace::Clone, CharacterRace::GeneModded,
            ],
            TTRPGGenre::Cyberpunk | TTRPGGenre::PostCyberpunk => vec![
                CharacterRace::Human, CharacterRace::Cyborg, CharacterRace::Clone,
                CharacterRace::GeneModded, CharacterRace::Synthetic,
            ],
            TTRPGGenre::Biopunk => vec![
                CharacterRace::Human, CharacterRace::Mutant, CharacterRace::GeneModded,
                CharacterRace::HybridSpecies, CharacterRace::Uplifted,
            ],
            TTRPGGenre::Solarpunk => vec![
                CharacterRace::Human, CharacterRace::GeneModded, CharacterRace::Uplifted,
            ],

            // Horror variants
            TTRPGGenre::CosmicHorror => vec![
                CharacterRace::Human, CharacterRace::Touched, CharacterRace::Cursed,
            ],
            TTRPGGenre::GothicHorror => vec![
                CharacterRace::Human, CharacterRace::Dhampir, CharacterRace::Haunted,
                CharacterRace::Cursed,
            ],
            TTRPGGenre::SurvivalHorror | TTRPGGenre::FolkHorror => vec![
                CharacterRace::Human, CharacterRace::Infected, CharacterRace::Cursed,
            ],

            // Historical/Period
            TTRPGGenre::Historical | TTRPGGenre::Roman | TTRPGGenre::Egyptian => vec![
                CharacterRace::Human,
            ],
            TTRPGGenre::Viking => vec![
                CharacterRace::Human, CharacterRace::Goliath,
            ],
            TTRPGGenre::Samurai | TTRPGGenre::WuxiaWulin => vec![
                CharacterRace::Human, CharacterRace::Spirit, CharacterRace::Yokai,
            ],

            // Mythology
            TTRPGGenre::Mythology => vec![
                CharacterRace::Human, CharacterRace::Demigod, CharacterRace::Nephilim,
                CharacterRace::Fae, CharacterRace::Elemental, CharacterRace::Spirit,
            ],
            TTRPGGenre::FairyTale => vec![
                CharacterRace::Human, CharacterRace::Fae, CharacterRace::Changeling,
                CharacterRace::Goblin, CharacterRace::Gnome,
            ],

            // Other genres (primarily human)
            TTRPGGenre::PostApocalyptic => vec![
                CharacterRace::Human, CharacterRace::Mutant, CharacterRace::Infected,
            ],
            TTRPGGenre::Superhero => vec![
                CharacterRace::Human, CharacterRace::Mutant, CharacterRace::Alien,
                CharacterRace::Android,
            ],
            TTRPGGenre::Steampunk | TTRPGGenre::Dieselpunk => vec![
                CharacterRace::Human, CharacterRace::Warforged, CharacterRace::Cyborg,
            ],
            _ => vec![CharacterRace::Human],
        };
        races.choose(rng).cloned().unwrap_or(CharacterRace::Human)
    }

    fn select_random_class(genre: &TTRPGGenre, rng: &mut impl Rng) -> CharacterClass {
        let classes = match genre {
            // Fantasy variants
            TTRPGGenre::Fantasy | TTRPGGenre::HighFantasy => vec![
                CharacterClass::Fighter, CharacterClass::Wizard, CharacterClass::Rogue,
                CharacterClass::Cleric, CharacterClass::Ranger, CharacterClass::Paladin,
                CharacterClass::Barbarian, CharacterClass::Bard, CharacterClass::Druid,
                CharacterClass::Monk, CharacterClass::Sorcerer, CharacterClass::Warlock,
            ],
            TTRPGGenre::LowFantasy | TTRPGGenre::Medieval => vec![
                CharacterClass::Fighter, CharacterClass::Rogue, CharacterClass::Ranger,
                CharacterClass::Knight, CharacterClass::Scholar, CharacterClass::Merchant,
            ],
            TTRPGGenre::DarkFantasy | TTRPGGenre::Grimdark => vec![
                CharacterClass::Fighter, CharacterClass::Warlock, CharacterClass::Necromancer,
                CharacterClass::Witch, CharacterClass::Hunter, CharacterClass::Inquisitor,
            ],
            TTRPGGenre::UrbanFantasy => vec![
                CharacterClass::Detective, CharacterClass::Occultist, CharacterClass::Hunter,
                CharacterClass::Wizard, CharacterClass::Rogue,
            ],

            // Sci-Fi variants
            TTRPGGenre::SciFi | TTRPGGenre::HardSciFi => vec![
                CharacterClass::Soldier, CharacterClass::Pilot, CharacterClass::Engineer,
                CharacterClass::Scientist, CharacterClass::Medic,
            ],
            TTRPGGenre::SpaceOpera => vec![
                CharacterClass::Soldier, CharacterClass::Pilot, CharacterClass::BountyHunter,
                CharacterClass::Smuggler, CharacterClass::StarshipCaptain, CharacterClass::Psionic,
            ],
            TTRPGGenre::Cyberpunk | TTRPGGenre::PostCyberpunk => vec![
                CharacterClass::Netrunner, CharacterClass::Solo, CharacterClass::Techie,
                CharacterClass::Fixer, CharacterClass::Rockerboy, CharacterClass::Nomad,
                CharacterClass::Corporate, CharacterClass::MediaOp, CharacterClass::MedTech,
            ],
            TTRPGGenre::Biopunk | TTRPGGenre::Solarpunk => vec![
                CharacterClass::Scientist, CharacterClass::Engineer, CharacterClass::Medic,
                CharacterClass::Operative, CharacterClass::Specialist,
            ],

            // Horror variants
            TTRPGGenre::CosmicHorror | TTRPGGenre::GothicHorror => vec![
                CharacterClass::Investigator, CharacterClass::Occultist, CharacterClass::Hunter,
                CharacterClass::Medium, CharacterClass::Scholar,
            ],
            TTRPGGenre::SurvivalHorror => vec![
                CharacterClass::Survivor, CharacterClass::Medic, CharacterClass::Hunter,
                CharacterClass::Soldier,
            ],
            TTRPGGenre::FolkHorror => vec![
                CharacterClass::Investigator, CharacterClass::Survivor, CharacterClass::Exorcist,
            ],

            // Historical/Period
            TTRPGGenre::Historical => vec![
                CharacterClass::Fighter, CharacterClass::Scholar, CharacterClass::Merchant,
                CharacterClass::Courtier, CharacterClass::Knight,
            ],
            TTRPGGenre::Viking => vec![
                CharacterClass::Viking, CharacterClass::Berserker, CharacterClass::Shaman,
                CharacterClass::Fighter, CharacterClass::Ranger,
            ],
            TTRPGGenre::Roman => vec![
                CharacterClass::Gladiator, CharacterClass::Centurion, CharacterClass::Scholar,
                CharacterClass::Merchant,
            ],
            TTRPGGenre::Samurai => vec![
                CharacterClass::Samurai, CharacterClass::Ronin, CharacterClass::Monk,
                CharacterClass::Scholar, CharacterClass::Merchant,
            ],
            TTRPGGenre::WuxiaWulin => vec![
                CharacterClass::MartialArtist, CharacterClass::SwordSaint, CharacterClass::QiMaster,
                CharacterClass::AssassinBlade, CharacterClass::MonkWarrior,
            ],

            // Other genres
            TTRPGGenre::Western => vec![
                CharacterClass::Gunslinger, CharacterClass::Sheriff, CharacterClass::Outlaw,
                CharacterClass::Prospector, CharacterClass::Gambler, CharacterClass::Preacher,
                CharacterClass::Bounty, CharacterClass::Marshal,
            ],
            TTRPGGenre::Steampunk => vec![
                CharacterClass::Inventor, CharacterClass::Aeronaut, CharacterClass::Clockworker,
                CharacterClass::Gadgeteer, CharacterClass::SteamKnight,
            ],
            TTRPGGenre::Dieselpunk => vec![
                CharacterClass::Soldier, CharacterClass::Pilot, CharacterClass::Engineer,
                CharacterClass::Operative, CharacterClass::Scientist,
            ],
            TTRPGGenre::Noir => vec![
                CharacterClass::Detective, CharacterClass::Gumshoe, CharacterClass::Femme,
                CharacterClass::Enforcer, CharacterClass::Journalist,
            ],
            TTRPGGenre::Pulp => vec![
                CharacterClass::Detective, CharacterClass::Scientist, CharacterClass::Pilot,
                CharacterClass::Soldier, CharacterClass::Explorer,
            ],
            TTRPGGenre::Swashbuckling => vec![
                CharacterClass::Duelist, CharacterClass::Pirate, CharacterClass::Musketeer,
                CharacterClass::Privateer, CharacterClass::Corsair,
            ],
            TTRPGGenre::Superhero => vec![
                CharacterClass::Paragon, CharacterClass::Vigilante, CharacterClass::Mastermind,
                CharacterClass::Mystic, CharacterClass::Mutant,
            ],
            TTRPGGenre::PostApocalyptic => vec![
                CharacterClass::Survivor, CharacterClass::Hunter, CharacterClass::Soldier,
                CharacterClass::Medic, CharacterClass::Engineer,
            ],
            TTRPGGenre::Mythology => vec![
                CharacterClass::Fighter, CharacterClass::Cleric, CharacterClass::Shaman,
                CharacterClass::Sorcerer, CharacterClass::Ranger,
            ],
            TTRPGGenre::FairyTale => vec![
                CharacterClass::Knight, CharacterClass::Witch, CharacterClass::Bard,
                CharacterClass::Ranger, CharacterClass::Rogue,
            ],
            TTRPGGenre::ModernDay | TTRPGGenre::MilitaryTactical => vec![
                CharacterClass::Soldier, CharacterClass::Operative, CharacterClass::Specialist,
                CharacterClass::Medic, CharacterClass::Engineer,
            ],
            TTRPGGenre::Espionage => vec![
                CharacterClass::Operative, CharacterClass::Analyst, CharacterClass::Diplomat,
                CharacterClass::Specialist, CharacterClass::Hacker,
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
        let last_names = ["Smith", "Doe", "Lightfoot", "Stone"];

        format!("{} {}",
            first_names.choose(rng).unwrap(),
            last_names.choose(rng).unwrap()
        )
    }
}
