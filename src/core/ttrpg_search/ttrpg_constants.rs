//! TTRPG Constants Module
//!
//! Comprehensive static data for TTRPG content including genres, classes,
//! races, traits, backgrounds, motivations, equipment, and name pools.
//! Ported from the legacy Python MDMAI project.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Genre Enum
// ============================================================================

/// TTRPG genre categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TTRPGGenre {
    Fantasy,
    SciFi,
    Cyberpunk,
    CosmicHorror,
    PostApocalyptic,
    Superhero,
    Steampunk,
    Western,
    Modern,
    SpaceOpera,
    UrbanFantasy,
    Historical,
    Noir,
    Pulp,
    Military,
    Horror,
    Mystery,
    Mythological,
    Anime,
    Generic,
    Custom,
    Unknown,
}

impl TTRPGGenre {
    /// Get all genre variants.
    pub fn all() -> &'static [TTRPGGenre] {
        &[
            Self::Fantasy, Self::SciFi, Self::Cyberpunk, Self::CosmicHorror,
            Self::PostApocalyptic, Self::Superhero, Self::Steampunk, Self::Western,
            Self::Modern, Self::SpaceOpera, Self::UrbanFantasy, Self::Historical,
            Self::Noir, Self::Pulp, Self::Military, Self::Horror, Self::Mystery,
            Self::Mythological, Self::Anime, Self::Generic, Self::Custom, Self::Unknown,
        ]
    }

    /// Get the display name for this genre.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Fantasy => "Fantasy",
            Self::SciFi => "Sci-Fi",
            Self::Cyberpunk => "Cyberpunk",
            Self::CosmicHorror => "Cosmic Horror",
            Self::PostApocalyptic => "Post-Apocalyptic",
            Self::Superhero => "Superhero",
            Self::Steampunk => "Steampunk",
            Self::Western => "Western",
            Self::Modern => "Modern",
            Self::SpaceOpera => "Space Opera",
            Self::UrbanFantasy => "Urban Fantasy",
            Self::Historical => "Historical",
            Self::Noir => "Noir",
            Self::Pulp => "Pulp",
            Self::Military => "Military",
            Self::Horror => "Horror",
            Self::Mystery => "Mystery",
            Self::Mythological => "Mythological",
            Self::Anime => "Anime",
            Self::Generic => "Generic",
            Self::Custom => "Custom",
            Self::Unknown => "Unknown",
        }
    }
}

// ============================================================================
// Character Class Enum
// ============================================================================

/// Character classes across multiple TTRPG genres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterClass {
    // Fantasy Classes
    Fighter,
    Wizard,
    Cleric,
    Rogue,
    Ranger,
    Paladin,
    Barbarian,
    Sorcerer,
    Warlock,
    Druid,
    Monk,
    Bard,
    Artificer,

    // Sci-Fi Classes
    Engineer,
    Scientist,
    Pilot,
    Marine,
    Diplomat,
    Xenobiologist,
    TechSpecialist,
    Psion,
    BountyHunter,

    // Cyberpunk Classes
    Netrunner,
    Solo,
    Fixer,
    Corporate,
    Rockerboy,
    Techie,
    Media,
    Cop,
    Nomad,

    // Cosmic Horror Classes
    Investigator,
    Scholar,
    Antiquarian,
    Occultist,
    Alienist,
    Archaeologist,
    Journalist,
    Detective,
    Professor,

    // Post-Apocalyptic Classes
    Survivor,
    Scavenger,
    Raider,
    Medic,
    Mechanic,
    Trader,
    Warlord,
    MutantHunter,
    VaultDweller,

    // Superhero Classes
    Vigilante,
    Powered,
    Genius,
    MartialArtist,
    Mystic,
    AlienHero,
    TechHero,
    Sidekick,

    // Western Classes
    Gunslinger,
    Lawman,
    Outlaw,
    Gambler,
    Preacher,
    Prospector,
    NativeScout,

    Custom,
}

impl CharacterClass {
    /// Get classes for a specific genre.
    pub fn for_genre(genre: TTRPGGenre) -> &'static [CharacterClass] {
        match genre {
            TTRPGGenre::Fantasy | TTRPGGenre::UrbanFantasy => &[
                Self::Fighter, Self::Wizard, Self::Cleric, Self::Rogue,
                Self::Ranger, Self::Paladin, Self::Barbarian, Self::Sorcerer,
                Self::Warlock, Self::Druid, Self::Monk, Self::Bard, Self::Artificer,
            ],
            TTRPGGenre::SciFi | TTRPGGenre::SpaceOpera => &[
                Self::Engineer, Self::Scientist, Self::Pilot, Self::Marine,
                Self::Diplomat, Self::Xenobiologist, Self::TechSpecialist,
                Self::Psion, Self::BountyHunter,
            ],
            TTRPGGenre::Cyberpunk => &[
                Self::Netrunner, Self::Solo, Self::Fixer, Self::Corporate,
                Self::Rockerboy, Self::Techie, Self::Media, Self::Cop, Self::Nomad,
            ],
            TTRPGGenre::CosmicHorror | TTRPGGenre::Horror | TTRPGGenre::Mystery => &[
                Self::Investigator, Self::Scholar, Self::Antiquarian, Self::Occultist,
                Self::Alienist, Self::Archaeologist, Self::Journalist,
                Self::Detective, Self::Professor,
            ],
            TTRPGGenre::PostApocalyptic => &[
                Self::Survivor, Self::Scavenger, Self::Raider, Self::Medic,
                Self::Mechanic, Self::Trader, Self::Warlord, Self::MutantHunter,
                Self::VaultDweller,
            ],
            TTRPGGenre::Superhero => &[
                Self::Vigilante, Self::Powered, Self::Genius, Self::MartialArtist,
                Self::Mystic, Self::AlienHero, Self::TechHero, Self::Sidekick,
            ],
            TTRPGGenre::Western => &[
                Self::Gunslinger, Self::Lawman, Self::Outlaw, Self::Gambler,
                Self::Preacher, Self::Prospector, Self::NativeScout,
            ],
            _ => &[Self::Fighter, Self::Rogue, Self::Custom],
        }
    }

    /// Get the display name for this class.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Fighter => "Fighter",
            Self::Wizard => "Wizard",
            Self::Cleric => "Cleric",
            Self::Rogue => "Rogue",
            Self::Ranger => "Ranger",
            Self::Paladin => "Paladin",
            Self::Barbarian => "Barbarian",
            Self::Sorcerer => "Sorcerer",
            Self::Warlock => "Warlock",
            Self::Druid => "Druid",
            Self::Monk => "Monk",
            Self::Bard => "Bard",
            Self::Artificer => "Artificer",
            Self::Engineer => "Engineer",
            Self::Scientist => "Scientist",
            Self::Pilot => "Pilot",
            Self::Marine => "Marine",
            Self::Diplomat => "Diplomat",
            Self::Xenobiologist => "Xenobiologist",
            Self::TechSpecialist => "Tech Specialist",
            Self::Psion => "Psion",
            Self::BountyHunter => "Bounty Hunter",
            Self::Netrunner => "Netrunner",
            Self::Solo => "Solo",
            Self::Fixer => "Fixer",
            Self::Corporate => "Corporate",
            Self::Rockerboy => "Rockerboy",
            Self::Techie => "Techie",
            Self::Media => "Media",
            Self::Cop => "Cop",
            Self::Nomad => "Nomad",
            Self::Investigator => "Investigator",
            Self::Scholar => "Scholar",
            Self::Antiquarian => "Antiquarian",
            Self::Occultist => "Occultist",
            Self::Alienist => "Alienist",
            Self::Archaeologist => "Archaeologist",
            Self::Journalist => "Journalist",
            Self::Detective => "Detective",
            Self::Professor => "Professor",
            Self::Survivor => "Survivor",
            Self::Scavenger => "Scavenger",
            Self::Raider => "Raider",
            Self::Medic => "Medic",
            Self::Mechanic => "Mechanic",
            Self::Trader => "Trader",
            Self::Warlord => "Warlord",
            Self::MutantHunter => "Mutant Hunter",
            Self::VaultDweller => "Vault Dweller",
            Self::Vigilante => "Vigilante",
            Self::Powered => "Powered",
            Self::Genius => "Genius",
            Self::MartialArtist => "Martial Artist",
            Self::Mystic => "Mystic",
            Self::AlienHero => "Alien Hero",
            Self::TechHero => "Tech Hero",
            Self::Sidekick => "Sidekick",
            Self::Gunslinger => "Gunslinger",
            Self::Lawman => "Lawman",
            Self::Outlaw => "Outlaw",
            Self::Gambler => "Gambler",
            Self::Preacher => "Preacher",
            Self::Prospector => "Prospector",
            Self::NativeScout => "Native Scout",
            Self::Custom => "Custom",
        }
    }
}

// ============================================================================
// Character Race Enum
// ============================================================================

/// Character races across multiple TTRPG genres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterRace {
    // Fantasy Races
    Human,
    Elf,
    Dwarf,
    Halfling,
    Orc,
    Tiefling,
    Dragonborn,
    Gnome,
    HalfElf,
    HalfOrc,

    // Sci-Fi Races
    Terran,
    Martian,
    Belter,
    Cyborg,
    Android,
    AiConstruct,
    GreyAlien,
    Reptilian,
    Insectoid,
    EnergyBeing,
    SiliconBased,
    UpliftedAnimal,

    // Cyberpunk Races
    AugmentedHuman,
    FullConversionCyborg,
    Bioengineered,
    Clone,
    DigitalConsciousness,

    // Cosmic Horror Races
    DeepOneHybrid,
    Ghoul,
    DreamlandsNative,
    Touched,

    // Post-Apocalyptic Races
    PureStrainHuman,
    Mutant,
    GhoulWastelander,
    Synthetic,
    Hybrid,
    Radiant,

    // Superhero Races
    Metahuman,
    Inhuman,
    Atlantean,
    Amazonian,
    Kryptonian,
    Asgardian,

    Custom,
}

impl CharacterRace {
    /// Get races for a specific genre.
    pub fn for_genre(genre: TTRPGGenre) -> &'static [CharacterRace] {
        match genre {
            TTRPGGenre::Fantasy | TTRPGGenre::UrbanFantasy => &[
                Self::Human, Self::Elf, Self::Dwarf, Self::Halfling, Self::Orc,
                Self::Tiefling, Self::Dragonborn, Self::Gnome, Self::HalfElf, Self::HalfOrc,
            ],
            TTRPGGenre::SciFi | TTRPGGenre::SpaceOpera => &[
                Self::Terran, Self::Martian, Self::Belter, Self::Cyborg, Self::Android,
                Self::AiConstruct, Self::GreyAlien, Self::Reptilian, Self::Insectoid,
                Self::EnergyBeing, Self::SiliconBased, Self::UpliftedAnimal,
            ],
            TTRPGGenre::Cyberpunk => &[
                Self::Human, Self::AugmentedHuman, Self::FullConversionCyborg,
                Self::Bioengineered, Self::Clone, Self::DigitalConsciousness,
            ],
            TTRPGGenre::CosmicHorror | TTRPGGenre::Horror => &[
                Self::Human, Self::DeepOneHybrid, Self::Ghoul,
                Self::DreamlandsNative, Self::Touched,
            ],
            TTRPGGenre::PostApocalyptic => &[
                Self::PureStrainHuman, Self::Mutant, Self::GhoulWastelander,
                Self::Synthetic, Self::Hybrid, Self::Radiant,
            ],
            TTRPGGenre::Superhero => &[
                Self::Human, Self::Metahuman, Self::Inhuman, Self::Atlantean,
                Self::Amazonian, Self::Kryptonian, Self::Asgardian,
            ],
            _ => &[Self::Human, Self::Custom],
        }
    }

    /// Get the display name for this race.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Human => "Human",
            Self::Elf => "Elf",
            Self::Dwarf => "Dwarf",
            Self::Halfling => "Halfling",
            Self::Orc => "Orc",
            Self::Tiefling => "Tiefling",
            Self::Dragonborn => "Dragonborn",
            Self::Gnome => "Gnome",
            Self::HalfElf => "Half-Elf",
            Self::HalfOrc => "Half-Orc",
            Self::Terran => "Terran",
            Self::Martian => "Martian",
            Self::Belter => "Belter",
            Self::Cyborg => "Cyborg",
            Self::Android => "Android",
            Self::AiConstruct => "AI Construct",
            Self::GreyAlien => "Grey Alien",
            Self::Reptilian => "Reptilian",
            Self::Insectoid => "Insectoid",
            Self::EnergyBeing => "Energy Being",
            Self::SiliconBased => "Silicon-Based",
            Self::UpliftedAnimal => "Uplifted Animal",
            Self::AugmentedHuman => "Augmented Human",
            Self::FullConversionCyborg => "Full Conversion Cyborg",
            Self::Bioengineered => "Bioengineered",
            Self::Clone => "Clone",
            Self::DigitalConsciousness => "Digital Consciousness",
            Self::DeepOneHybrid => "Deep One Hybrid",
            Self::Ghoul => "Ghoul",
            Self::DreamlandsNative => "Dreamlands Native",
            Self::Touched => "Touched",
            Self::PureStrainHuman => "Pure Strain Human",
            Self::Mutant => "Mutant",
            Self::GhoulWastelander => "Ghoul (Wastelander)",
            Self::Synthetic => "Synthetic",
            Self::Hybrid => "Hybrid",
            Self::Radiant => "Radiant",
            Self::Metahuman => "Metahuman",
            Self::Inhuman => "Inhuman",
            Self::Atlantean => "Atlantean",
            Self::Amazonian => "Amazonian",
            Self::Kryptonian => "Kryptonian",
            Self::Asgardian => "Asgardian",
            Self::Custom => "Custom",
        }
    }
}

// ============================================================================
// Character Trait Enum
// ============================================================================

/// Character trait categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraitCategory {
    Physical,
    Mental,
    Emotional,
    Social,
}

/// Comprehensive character traits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterTrait {
    // Physical Traits
    Agile, Athletic, Brawny, Burly, Delicate, Dexterous, Enduring, Energetic,
    Graceful, Hardy, Lithe, Muscular, Nimble, Powerful, Quick, Resilient,
    Robust, Rugged, Scarred, Slender, Stocky, Strong, Sturdy, Swift,
    Tall, Tough, Towering, Weathered, Wiry,

    // Mental Traits
    Analytical, Astute, Brilliant, Calculating, Clever, Creative, Cunning,
    Curious, Focused, Imaginative, Insightful, Intellectual, Intelligent,
    Intuitive, Knowledgeable, Learned, Logical, Methodical, Observant,
    Perceptive, Philosophical, QuickWitted, Rational, Resourceful,
    Scholarly, Sharp, Shrewd, Strategic, Studious, Tactical, Thoughtful, Wise,

    // Emotional Traits
    Ambitious, Anxious, Bold, Brave, Calm, Cautious, Cheerful, Compassionate,
    Confident, Courageous, Determined, Devoted, Disciplined, Empathetic,
    Enthusiastic, Fearless, Fierce, Gentle, Grim, Hopeful, Humble, Impulsive,
    Independent, Joyful, Kind, Loyal, Melancholic, Merciful, Passionate,
    Patient, Proud, Rebellious, Reckless, Resolute, Ruthless, Selfless,
    Serene, Sincere, Skeptical, Steadfast, Stoic, Stubborn, Sympathetic,
    Tenacious, Vengeful, Vigilant, Volatile, Zealous,

    // Social Traits
    Charismatic, Charming, Diplomatic, Eloquent, Gregarious, Intimidating,
    Mysterious, Persuasive, Reserved, Shy, Sociable, Witty,
}

impl CharacterTrait {
    /// Get the category for this trait.
    pub fn category(&self) -> TraitCategory {
        match self {
            Self::Agile | Self::Athletic | Self::Brawny | Self::Burly |
            Self::Delicate | Self::Dexterous | Self::Enduring | Self::Energetic |
            Self::Graceful | Self::Hardy | Self::Lithe | Self::Muscular |
            Self::Nimble | Self::Powerful | Self::Quick | Self::Resilient |
            Self::Robust | Self::Rugged | Self::Scarred | Self::Slender |
            Self::Stocky | Self::Strong | Self::Sturdy | Self::Swift |
            Self::Tall | Self::Tough | Self::Towering | Self::Weathered |
            Self::Wiry => TraitCategory::Physical,

            Self::Analytical | Self::Astute | Self::Brilliant | Self::Calculating |
            Self::Clever | Self::Creative | Self::Cunning | Self::Curious |
            Self::Focused | Self::Imaginative | Self::Insightful | Self::Intellectual |
            Self::Intelligent | Self::Intuitive | Self::Knowledgeable | Self::Learned |
            Self::Logical | Self::Methodical | Self::Observant | Self::Perceptive |
            Self::Philosophical | Self::QuickWitted | Self::Rational | Self::Resourceful |
            Self::Scholarly | Self::Sharp | Self::Shrewd | Self::Strategic |
            Self::Studious | Self::Tactical | Self::Thoughtful | Self::Wise => TraitCategory::Mental,

            Self::Ambitious | Self::Anxious | Self::Bold | Self::Brave |
            Self::Calm | Self::Cautious | Self::Cheerful | Self::Compassionate |
            Self::Confident | Self::Courageous | Self::Determined | Self::Devoted |
            Self::Disciplined | Self::Empathetic | Self::Enthusiastic | Self::Fearless |
            Self::Fierce | Self::Gentle | Self::Grim | Self::Hopeful |
            Self::Humble | Self::Impulsive | Self::Independent | Self::Joyful |
            Self::Kind | Self::Loyal | Self::Melancholic | Self::Merciful |
            Self::Passionate | Self::Patient | Self::Proud | Self::Rebellious |
            Self::Reckless | Self::Resolute | Self::Ruthless | Self::Selfless |
            Self::Serene | Self::Sincere | Self::Skeptical | Self::Steadfast |
            Self::Stoic | Self::Stubborn | Self::Sympathetic | Self::Tenacious |
            Self::Vengeful | Self::Vigilant | Self::Volatile | Self::Zealous => TraitCategory::Emotional,

            Self::Charismatic | Self::Charming | Self::Diplomatic | Self::Eloquent |
            Self::Gregarious | Self::Intimidating | Self::Mysterious | Self::Persuasive |
            Self::Reserved | Self::Shy | Self::Sociable | Self::Witty => TraitCategory::Social,
        }
    }

    /// Get traits for a specific category.
    pub fn for_category(category: TraitCategory) -> Vec<CharacterTrait> {
        Self::all().into_iter().filter(|t| t.category() == category).collect()
    }

    /// Get all trait variants.
    pub fn all() -> Vec<CharacterTrait> {
        vec![
            // Physical
            Self::Agile, Self::Athletic, Self::Brawny, Self::Burly, Self::Delicate,
            Self::Dexterous, Self::Enduring, Self::Energetic, Self::Graceful, Self::Hardy,
            Self::Lithe, Self::Muscular, Self::Nimble, Self::Powerful, Self::Quick,
            Self::Resilient, Self::Robust, Self::Rugged, Self::Scarred, Self::Slender,
            Self::Stocky, Self::Strong, Self::Sturdy, Self::Swift, Self::Tall,
            Self::Tough, Self::Towering, Self::Weathered, Self::Wiry,
            // Mental
            Self::Analytical, Self::Astute, Self::Brilliant, Self::Calculating,
            Self::Clever, Self::Creative, Self::Cunning, Self::Curious, Self::Focused,
            Self::Imaginative, Self::Insightful, Self::Intellectual, Self::Intelligent,
            Self::Intuitive, Self::Knowledgeable, Self::Learned, Self::Logical,
            Self::Methodical, Self::Observant, Self::Perceptive, Self::Philosophical,
            Self::QuickWitted, Self::Rational, Self::Resourceful, Self::Scholarly,
            Self::Sharp, Self::Shrewd, Self::Strategic, Self::Studious, Self::Tactical,
            Self::Thoughtful, Self::Wise,
            // Emotional
            Self::Ambitious, Self::Anxious, Self::Bold, Self::Brave, Self::Calm,
            Self::Cautious, Self::Cheerful, Self::Compassionate, Self::Confident,
            Self::Courageous, Self::Determined, Self::Devoted, Self::Disciplined,
            Self::Empathetic, Self::Enthusiastic, Self::Fearless, Self::Fierce,
            Self::Gentle, Self::Grim, Self::Hopeful, Self::Humble, Self::Impulsive,
            Self::Independent, Self::Joyful, Self::Kind, Self::Loyal, Self::Melancholic,
            Self::Merciful, Self::Passionate, Self::Patient, Self::Proud, Self::Rebellious,
            Self::Reckless, Self::Resolute, Self::Ruthless, Self::Selfless, Self::Serene,
            Self::Sincere, Self::Skeptical, Self::Steadfast, Self::Stoic, Self::Stubborn,
            Self::Sympathetic, Self::Tenacious, Self::Vengeful, Self::Vigilant,
            Self::Volatile, Self::Zealous,
            // Social
            Self::Charismatic, Self::Charming, Self::Diplomatic, Self::Eloquent,
            Self::Gregarious, Self::Intimidating, Self::Mysterious, Self::Persuasive,
            Self::Reserved, Self::Shy, Self::Sociable, Self::Witty,
        ]
    }
}

// ============================================================================
// Character Background Enum
// ============================================================================

/// Character backgrounds across genres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterBackground {
    // Traditional Backgrounds
    Acolyte, Criminal, FolkHero, Noble, Sage, Soldier, Hermit, Entertainer,
    GuildArtisan, Outlander, Sailor, Urchin,

    // Expanded Traditional
    Alchemist, Ambassador, Aristocrat, Assassin, Bandit, Blacksmith,
    CaravanGuard, Charlatan, ClanCrafter, CloisteredScholar, Courtier,
    CultInitiate, Exile, Explorer, Farmer, Fisher, Gladiator, Guard,
    Healer, Hunter, Innkeeper, Knight, Librarian, Merchant, Mercenary,
    Miner, MonkInitiate, Pirate, Priest, RangerScout, Refugee,
    ScholarMage, Scribe, ShipCaptain, Smuggler, Spy, StreetThief,
    TavernKeeper, TempleGuardian, Thief, Wanderer, WarRefugee,
    TribalWarrior, Veteran,

    // Sci-Fi Backgrounds
    AsteroidMiner, ColonyAdministrator, CorporateAgent, CyborgEngineer,
    DataAnalyst, DiplomatEnvoy, GeneticResearcher, Hacker, JumpPilot,
    OrbitalMechanic, SpaceMarine, StarshipEngineer, Terraformer, VoidTrader,

    // Cyberpunk Backgrounds
    CorporateExec, GangMember, MediaJournalist, Ripperdoc, StreetSamurai,

    // Post-Apocalyptic Backgrounds
    BunkerSurvivor, CaravanTrader, MutantOutcast, ScavengerBg,
    SettlementLeader, TribalShaman, WastelandDoctor, WastelandScout,

    // Cosmic Horror Backgrounds
    AsylumPatient, CultSurvivor, CursedBloodline, DreamTouched,
    OccultInvestigator, PsychicSensitive,

    // Western Backgrounds
    BountyKiller, CattleRustler, FrontierDoctor, Homesteader, RanchHand,
    SaloonOwner, StageDriver,

    // Superhero Backgrounds
    AlienRefugee, GovernmentAgent, LabAccidentSurvivor, MaskedVigilante,
    MilitaryExperiment, MutantActivist, Reporter, TechGenius,

    Custom,
}

// ============================================================================
// Character Motivation Enum
// ============================================================================

/// Character motivations including desires and fears.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterMotivation {
    // Core Desires
    Acceptance, Achievement, Adventure, Approval, Balance, Belonging,
    Challenge, Change, Comfort, Connection, Control, Discovery, Duty,
    Excellence, Excitement, Exploration, Fame, Freedom, Glory, Growth,
    Happiness, Harmony, Honor, Hope, Independence, Influence, Justice,
    Knowledge, Legacy, Love, Mastery, Meaning, Order, Peace, Perfection,
    Pleasure, Power, Prestige, Progress, Prosperity, Protection, Purpose,
    Recognition, Redemption, Respect, Restoration, Revenge, Safety,
    Salvation, Security, Service, Stability, Status, Strength, Success,
    Survival, Tradition, Transcendence, Transformation, Truth,
    Understanding, Unity, Validation, Vengeance, Victory, Wealth, Wisdom,

    // Fears
    Abandonment, Betrayal, Chaos, Confinement, Corruption, Darkness,
    Death, Defeat, Disgrace, Disease, Exposure, Failure, Forgetting,
    Helplessness, Humiliation, Ignorance, Insignificance, Isolation,
    Loss, Madness, Meaninglessness, Obscurity, Pain, Poverty,
    Powerlessness, Rejection, Responsibility, Stagnation, Suffering,
    TheUnknown, Vulnerability, Weakness,

    // Complex Motivations
    Atonement, BreakingChains, BuildingEmpire, ChasingLegend,
    ClaimingBirthright, ConqueringFear, DefendingHomeland, DestroyingEvil,
    DiscoveringHeritage, EndingTyranny, EscapingPast, FindingHome,
    FindingIdentity, FulfillingDestiny, FulfillingOath, HonoringAncestors,
    LiberatingOppressed, MaintainingBalance, MakingAmends, PreservingMemory,
    ProtectingInnocent, ProvingWorth, ReclaimingThrone, RecoveringArtifact,
    RedeemingFamily, RestoringHonor, ReunitingFamily, RevealingTruth,
    SavingLovedOne, SeekingEnlightenment, SolvingMystery, StoppingProphecy,
    UncoveringConspiracy, UnitingPeople,
}

impl CharacterMotivation {
    /// Check if this is a fear-type motivation.
    pub fn is_fear(&self) -> bool {
        matches!(self,
            Self::Abandonment | Self::Betrayal | Self::Chaos | Self::Confinement |
            Self::Corruption | Self::Darkness | Self::Death | Self::Defeat |
            Self::Disgrace | Self::Disease | Self::Exposure | Self::Failure |
            Self::Forgetting | Self::Helplessness | Self::Humiliation |
            Self::Ignorance | Self::Insignificance | Self::Isolation |
            Self::Loss | Self::Madness | Self::Meaninglessness | Self::Obscurity |
            Self::Pain | Self::Poverty | Self::Powerlessness | Self::Rejection |
            Self::Responsibility | Self::Stagnation | Self::Suffering |
            Self::TheUnknown | Self::Vulnerability | Self::Weakness
        )
    }

    /// Check if this is a complex/narrative motivation.
    pub fn is_complex(&self) -> bool {
        matches!(self,
            Self::Atonement | Self::BreakingChains | Self::BuildingEmpire |
            Self::ChasingLegend | Self::ClaimingBirthright | Self::ConqueringFear |
            Self::DefendingHomeland | Self::DestroyingEvil | Self::DiscoveringHeritage |
            Self::EndingTyranny | Self::EscapingPast | Self::FindingHome |
            Self::FindingIdentity | Self::FulfillingDestiny | Self::FulfillingOath |
            Self::HonoringAncestors | Self::LiberatingOppressed | Self::MaintainingBalance |
            Self::MakingAmends | Self::PreservingMemory | Self::ProtectingInnocent |
            Self::ProvingWorth | Self::ReclaimingThrone | Self::RecoveringArtifact |
            Self::RedeemingFamily | Self::RestoringHonor | Self::ReunitingFamily |
            Self::RevealingTruth | Self::SavingLovedOne | Self::SeekingEnlightenment |
            Self::SolvingMystery | Self::StoppingProphecy | Self::UncoveringConspiracy |
            Self::UnitingPeople
        )
    }
}

// ============================================================================
// NPC Role Enum
// ============================================================================

/// NPC roles across multiple TTRPG genres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NPCRole {
    // Fantasy Roles
    MerchantNpc, GuardNpc, NobleNpc, ScholarNpc, CriminalNpc, InnkeeperNpc,
    PriestNpc, AdventurerNpc, ArtisanNpc, CommonerNpc, SoldierNpc,
    MageNpc, AssassinNpc, HealerNpc,

    // Sci-Fi Roles
    StationCommander, ShipCaptain, ColonyAdmin, XenobiologistNpc,
    SpaceTrader, AsteroidMinerNpc, JumpGateOperator, AlienAmbassador,
    SmugglerNpc,

    // Cyberpunk Roles
    StreetSamuraiNpc, CorporateExecNpc, BlackMarketDealer, RipperdocNpc,
    GangLeader, InfoBroker, ClubOwner, CorruptCop,

    // Cosmic Horror Roles
    Cultist, CultLeader, MadScientist, LibrarianNpc, AsylumDoctor,
    PrivateInvestigator, MuseumCurator, DoomsdayProphet,

    // Post-Apocalyptic Roles
    SettlementLeaderNpc, WastelandDoctorNpc, CaravanMaster, RaiderChief,
    VaultOverseer, ScrapDealer, WaterMerchant, TribalElder,

    // Superhero Roles
    PoliceCommissioner, NewsReporter, ScientistAlly, GovernmentAgentNpc,
    Villain, Henchman, Civilian,

    // Western Roles
    Sheriff, SaloonKeeperNpc, RanchOwner, BankTeller, StageCoachDriver,
    BlacksmithNpc, MedicineMan,

    Custom,
}

// ============================================================================
// Weapon Type Enum
// ============================================================================

/// Weapon types across genres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponType {
    // Melee - Swords
    Sword, Longsword, Shortsword, Greatsword, Rapier, Scimitar,
    Katana, Cutlass, Broadsword, Claymore,
    // Melee - Axes
    Axe, Battleaxe, Handaxe, Greataxe, Waraxe, ThrowingAxe,
    // Melee - Blunt
    Mace, Club, Warhammer, Maul, Morningstar, Flail,
    // Melee - Polearms
    Spear, Pike, Halberd, Glaive, Trident, Lance,
    // Melee - Daggers
    Dagger, Knife, Stiletto, Dirk, Kris,
    // Melee - Staves
    Staff, Quarterstaff, WalkingStick, BoStaff,
    // Ranged - Bows
    Bow, Longbow, Shortbow, CompositeBow,
    Crossbow, HeavyCrossbow, HandCrossbow,
    // Firearms
    Pistol, Revolver, Rifle, Shotgun, Musket, Blunderbuss,
    // Energy Weapons
    LaserPistol, LaserRifle, PlasmaGun, PulseRifle, Disruptor, Stunner,
    // Exotic
    Whip, Net, Bolas, Chakram, Shuriken, Blowgun,
    // Natural/Improvised
    Claws, Bite, Tentacle, Improvised,
}

// ============================================================================
// Item Type Enum
// ============================================================================

/// Item types for equipment and loot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    // Adventuring Gear
    Rope, GrapplingHook, Torch, Lantern, OilFlask, Tinderbox,
    Bedroll, Tent, Rations, Waterskin, Backpack, Pouch, Sack, Chest, Barrel,
    // Tools
    ThievesTools, Lockpicks, Crowbar, Hammer, Piton, Shovel, Pickaxe,
    ClimbingKit, HealersKit, HerbalismKit, AlchemistSupplies, BrewersSupplies,
    CalligraphersSupplies, CarpentersTools, CartographersTools, CobblersTools,
    CooksUtensils, GlassblowersTools, JewelersTools, LeatherworkersTools,
    MasonsTools, PaintersSupplies, PottersTools, SmithsTools, TinkersTools,
    WeaversTools, WoodcarversTools, DisguiseKit, ForgeryKit, GamingSet,
    MusicalInstrument, NavigatorsTools, PoisonersKit,
    // Magic Items
    Potion, Scroll, Wand, Rod, StaffMagical, Ring, Amulet,
    Cloak, Boots, Gloves, Belt, Bracers, Circlet,
    // Books & Documents
    Spellbook, Tome, Map, Letter, Journal, Contract, Deed,
    // Technology
    Communicator, Scanner, Datapad, HoloProjector, Medkit, RepairKit,
    EnergyCell, Cyberdeck, NeuralImplant,
    // Miscellaneous
    HolySymbol, ComponentPouch, ArcaneFocus, DruidicFocus, Mirror,
    MagnifyingGlass, Spyglass, Compass, Hourglass, Scales, Vial,
    Flask, Bottle, Soap, Bell, Whistle, SignalHorn, Manacles, Chain,
    BallBearings, Caltrops, Chalk, Ladder, Pole, Ram, SignetRing, SealingWax,
}

// ============================================================================
// Equipment Quality Enum
// ============================================================================

/// Quality levels for equipment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentQuality {
    Poor,
    Common,
    Fine,
    Masterwork,
    Magical,
    Legendary,
    Artifact,
}

impl EquipmentQuality {
    /// Get the value multiplier for this quality.
    pub fn value_multiplier(&self) -> f32 {
        match self {
            Self::Poor => 0.5,
            Self::Common => 1.0,
            Self::Fine => 2.0,
            Self::Masterwork => 5.0,
            Self::Magical => 10.0,
            Self::Legendary => 50.0,
            Self::Artifact => 100.0,
        }
    }

    /// Get the effectiveness multiplier for this quality.
    pub fn effectiveness_multiplier(&self) -> f32 {
        match self {
            Self::Poor => 0.8,
            Self::Common => 1.0,
            Self::Fine => 1.1,
            Self::Masterwork => 1.2,
            Self::Magical => 1.5,
            Self::Legendary => 2.0,
            Self::Artifact => 3.0,
        }
    }
}

// ============================================================================
// Name Pools
// ============================================================================

/// Name pools for character generation.
pub struct NamePools;

impl NamePools {
    /// Fantasy first names (male).
    pub const FANTASY_MALE: &'static [&'static str] = &[
        "Aldric", "Theron", "Marcus", "Gareth", "Darius", "Lysander",
        "Caspian", "Rowan", "Felix", "Lucian", "Dorian", "Silas",
        "Edmund", "Roland", "Percival", "Cedric", "Alaric", "Roderick",
    ];

    /// Fantasy first names (female).
    pub const FANTASY_FEMALE: &'static [&'static str] = &[
        "Elena", "Lyra", "Mira", "Seraphina", "Aurora", "Celeste",
        "Iris", "Luna", "Nova", "Ophelia", "Thalia", "Violet",
        "Isolde", "Rowena", "Elara", "Vivienne", "Morgana", "Gwendolyn",
    ];

    /// Fantasy last names.
    pub const FANTASY_LAST: &'static [&'static str] = &[
        "Blackwood", "Stormwind", "Ironforge", "Goldshire", "Silverleaf",
        "Brightblade", "Shadowbane", "Moonwhisper", "Starweaver", "Flameheart",
        "Thornwood", "Ravencrest", "Dragonmoor", "Ashford", "Winterhold",
    ];

    /// Fantasy titles.
    pub const FANTASY_TITLES: &'static [&'static str] = &[
        "Sir", "Lady", "Lord", "Dame", "Master", "Mistress", "Baron", "Baroness",
        "Count", "Countess", "Duke", "Duchess", "Prince", "Princess",
    ];

    /// Sci-Fi first names (male).
    pub const SCIFI_MALE: &'static [&'static str] = &[
        "Nova", "Orion", "Atlas", "Zephyr", "Kai", "Leo", "Phoenix",
        "Axel", "Cyrus", "Rex", "Jax", "Zane", "Cade", "Dex", "Talon",
    ];

    /// Sci-Fi first names (female).
    pub const SCIFI_FEMALE: &'static [&'static str] = &[
        "Luna", "Vega", "Stella", "Astra", "Lyra", "Cora", "Zara",
        "Nyx", "Echo", "Aria", "Sage", "Iris", "Nova", "Celeste", "Andromeda",
    ];

    /// Sci-Fi last names.
    pub const SCIFI_LAST: &'static [&'static str] = &[
        "Stardust", "Cosmos", "Nebula", "Quasar", "Pulsar", "Void",
        "Stellar", "Photon", "Quantum", "Nexus", "Vector", "Prime",
        "Mars-7", "Terra-Prime", "Alpha-3", "Sigma-9", "Omega-X",
    ];

    /// Cyberpunk first names.
    pub const CYBERPUNK_FIRST: &'static [&'static str] = &[
        "Neon", "Chrome", "Razor", "Ghost", "Binary", "Cipher",
        "Dex", "Volt", "Glitch", "Static", "Vector", "Pixel",
        "Shadow", "Hex", "Nova", "Synth", "Echo", "Viper",
        "Zero", "Byte", "Code", "Virus", "Daemon", "Script",
    ];

    /// Cyberpunk handles/nicknames.
    pub const CYBERPUNK_HANDLES: &'static [&'static str] = &[
        "CrashOverride", "AcidBurn", "ZeroCool", "ThePhantom", "NightCrawler",
        "GhostInShell", "NeuralBurn", "DataThief", "NetDemon", "ByteBandit",
        "ChromeHeart", "NeonShadow", "IceBreaker", "DarkNet", "SilverHand",
    ];

    /// Cosmic Horror first names (male).
    pub const COSMIC_HORROR_MALE: &'static [&'static str] = &[
        "Randolph", "Herbert", "Wilbur", "Silas", "Ephraim", "Jeremiah",
        "Barnabas", "Obadiah", "Ezekiel", "Thaddeus", "Ambrose", "Cornelius",
    ];

    /// Cosmic Horror first names (female).
    pub const COSMIC_HORROR_FEMALE: &'static [&'static str] = &[
        "Lavinia", "Prudence", "Constance", "Temperance", "Mercy", "Patience",
        "Charity", "Verity", "Felicity", "Agatha", "Minerva", "Cordelia",
    ];

    /// Cosmic Horror last names (New England style).
    pub const COSMIC_HORROR_LAST: &'static [&'static str] = &[
        "Whateley", "Armitage", "Marsh", "Gilman", "Ward", "Carter",
        "Pickman", "Wilmarth", "Akeley", "Derby", "Peaslee", "Olmstead",
    ];

    /// Post-Apocalyptic first names.
    pub const POST_APOC_FIRST: &'static [&'static str] = &[
        "Ash", "Rust", "Storm", "Dust", "Hawk", "Wolf", "Stone",
        "Blade", "Rex", "Max", "Tank", "Diesel", "Raven", "Scar",
        "Ghost", "Reaper", "Scrap", "Zero", "Rad", "Grit", "Spike",
    ];

    /// Post-Apocalyptic nicknames.
    pub const POST_APOC_NICKNAMES: &'static [&'static str] = &[
        "Two-Shot", "Dogmeat", "Grognak", "Psycho", "Jet", "Radroach",
        "Deathclaw", "Smoothskin", "Wastelander", "Road Warrior", "Mad",
    ];

    /// Western first names (male).
    pub const WESTERN_MALE: &'static [&'static str] = &[
        "Jesse", "Wyatt", "Doc", "Billy", "Frank", "Butch", "Cole",
        "Jake", "Luke", "Wade", "Clay", "Hank", "Clint", "Eli", "Amos",
    ];

    /// Western first names (female).
    pub const WESTERN_FEMALE: &'static [&'static str] = &[
        "Belle", "Calamity", "Annie", "Rose", "Pearl", "Daisy",
        "Grace", "Sally", "Kate", "Lilly", "Ruby", "May", "Clara", "Sadie",
    ];

    /// Western last names.
    pub const WESTERN_LAST: &'static [&'static str] = &[
        "Morgan", "Earp", "Holliday", "James", "Cassidy", "Starr",
        "Black", "Stone", "Walker", "Rider", "Turner", "Miller",
    ];

    /// Elf name prefixes.
    pub const ELF_PREFIXES: &'static [&'static str] = &[
        "Sil", "Gal", "El", "Ar", "Leg", "Thaur", "Cel", "Fin", "Gil", "Luth",
    ];

    /// Elf name suffixes.
    pub const ELF_SUFFIXES: &'static [&'static str] = &[
        "wen", "riel", "iel", "dor", "las", "ion", "dir", "oth", "orn", "hil",
    ];

    /// Dwarf name prefixes.
    pub const DWARF_PREFIXES: &'static [&'static str] = &[
        "Thor", "Gim", "Bal", "Dur", "Oin", "Dwal", "Bof", "Glor", "Nor", "Brom",
    ];

    /// Dwarf name suffixes.
    pub const DWARF_SUFFIXES: &'static [&'static str] = &[
        "in", "li", "ori", "oin", "rim", "din", "bur", "gar", "mund", "rik",
    ];

    /// Orc name prefixes.
    pub const ORC_PREFIXES: &'static [&'static str] = &[
        "Gro", "Ug", "Mog", "Grim", "Gor", "Kro", "Thok", "Zug", "Dro", "Bur",
    ];

    /// Orc name suffixes.
    pub const ORC_SUFFIXES: &'static [&'static str] = &[
        "bash", "tooth", "jaw", "skull", "bone", "blood", "fist", "mash", "gore", "ruk",
    ];
}

// ============================================================================
// Equipment Pools
// ============================================================================

/// Equipment pools for generation.
pub struct EquipmentPools;

impl EquipmentPools {
    // Fantasy Weapons
    pub const FANTASY_MELEE: &'static [&'static str] = &[
        "Longsword", "Shortsword", "Greatsword", "Battleaxe", "Warhammer",
        "Mace", "Flail", "Morningstar", "Spear", "Halberd", "Glaive",
    ];

    pub const FANTASY_RANGED: &'static [&'static str] = &[
        "Longbow", "Shortbow", "Crossbow", "Heavy Crossbow", "Sling",
    ];

    pub const FANTASY_LIGHT: &'static [&'static str] = &[
        "Dagger", "Shortsword", "Rapier", "Scimitar", "Handaxe",
    ];

    // Fantasy Armor
    pub const FANTASY_LIGHT_ARMOR: &'static [&'static str] = &[
        "Padded Armor", "Leather Armor", "Studded Leather",
    ];

    pub const FANTASY_MEDIUM_ARMOR: &'static [&'static str] = &[
        "Hide Armor", "Chain Shirt", "Scale Mail", "Breastplate", "Half Plate",
    ];

    pub const FANTASY_HEAVY_ARMOR: &'static [&'static str] = &[
        "Ring Mail", "Chain Mail", "Splint Mail", "Plate Mail", "Full Plate",
    ];

    // Fantasy Items
    pub const FANTASY_ADVENTURING: &'static [&'static str] = &[
        "Rope (50 ft)", "Grappling Hook", "Torches", "Lantern",
        "Oil Flask", "Tinderbox", "Bedroll", "Rations", "Waterskin",
    ];

    pub const FANTASY_MAGICAL: &'static [&'static str] = &[
        "Potion of Healing", "Scroll of Magic Missile", "Wand of Magic Detection",
        "Ring of Protection", "Cloak of Elvenkind", "Boots of Speed",
        "Bag of Holding", "Immovable Rod", "Decanter of Endless Water",
    ];

    // Sci-Fi Weapons
    pub const SCIFI_ENERGY: &'static [&'static str] = &[
        "Laser Rifle", "Plasma Pistol", "Ion Cannon", "Pulse Rifle",
        "Photon Blade", "Disruptor", "Particle Beam", "Fusion Lance",
    ];

    pub const SCIFI_KINETIC: &'static [&'static str] = &[
        "Gauss Rifle", "Rail Gun", "Mass Driver", "Needle Pistol",
        "Flechette Gun", "Mag-Rifle", "Coil Gun",
    ];

    // Sci-Fi Armor
    pub const SCIFI_ARMOR: &'static [&'static str] = &[
        "Flex Suit", "Nano-weave Vest", "Energy Shield Belt", "Ablative Coating",
        "Combat Armor", "Powered Exoskeleton", "Power Armor", "Battle Suit",
    ];

    // Cyberpunk Weapons
    pub const CYBERPUNK_FIREARMS: &'static [&'static str] = &[
        "Heavy Pistol", "SMG", "Assault Rifle", "Shotgun", "Sniper Rifle",
        "Smart Pistol", "Smart Rifle", "Tech Shotgun",
    ];

    pub const CYBERPUNK_MELEE: &'static [&'static str] = &[
        "Monoblade", "Mantis Blades", "Gorilla Arms", "Nanowire",
        "Combat Knife", "Thermal Katana", "Monowhip",
    ];

    // Cyberpunk Cyberware
    pub const CYBERPUNK_CYBERWARE: &'static [&'static str] = &[
        "Cybereye", "Neural Interface", "Cyberdeck", "Chipware Socket",
        "Reflex Booster", "Muscle Enhancement", "Neural Processor",
        "Subdermal Armor", "Skinweave", "Sandevistan", "Kerenzikov",
    ];

    // Post-Apocalyptic Weapons
    pub const POST_APOC_FIREARMS: &'static [&'static str] = &[
        "Pipe Rifle", "Sawed-off Shotgun", "Hunting Rifle", "Revolver",
        "Makeshift SMG", "Scrap Pistol", "Jury-rigged Assault Rifle",
    ];

    pub const POST_APOC_MELEE: &'static [&'static str] = &[
        "Baseball Bat", "Tire Iron", "Machete", "Fire Axe", "Sledgehammer",
        "Sharpened Rebar", "Chain", "Nail Board", "Power Fist",
    ];

    // Post-Apocalyptic Survival
    pub const POST_APOC_SURVIVAL: &'static [&'static str] = &[
        "Gas Mask", "Rad-Away", "Rad-X", "Stimpak", "Water Purifier",
        "Geiger Counter", "Hazmat Suit", "Duct Tape", "Scrap Metal",
    ];

    pub const POST_APOC_CHEMS: &'static [&'static str] = &[
        "Med-X", "Psycho", "Jet", "Buffout", "Mentats", "Fixer",
    ];

    // Western Weapons
    pub const WESTERN_PISTOLS: &'static [&'static str] = &[
        "Colt Peacemaker", "Smith & Wesson", "Derringer", "Navy Revolver",
    ];

    pub const WESTERN_RIFLES: &'static [&'static str] = &[
        "Winchester Rifle", "Henry Rifle", "Sharps Rifle", "Spencer Carbine",
    ];

    // Western Items
    pub const WESTERN_GEAR: &'static [&'static str] = &[
        "Saddle", "Saddlebags", "Bedroll", "Canteen", "Compass", "Map",
        "Lasso", "Duster Coat", "Bandolier", "Holster", "Spurs",
    ];
}

// ============================================================================
// Search Integration Helpers
// ============================================================================

/// Get all searchable terms from the constants.
pub fn get_all_searchable_terms() -> Vec<&'static str> {
    let mut terms = Vec::with_capacity(1000);

    // Add genre names
    for genre in TTRPGGenre::all() {
        terms.push(genre.display_name());
    }

    // Add name pool entries
    terms.extend_from_slice(NamePools::FANTASY_MALE);
    terms.extend_from_slice(NamePools::FANTASY_FEMALE);
    terms.extend_from_slice(NamePools::FANTASY_LAST);
    terms.extend_from_slice(NamePools::SCIFI_MALE);
    terms.extend_from_slice(NamePools::SCIFI_FEMALE);
    terms.extend_from_slice(NamePools::SCIFI_LAST);
    terms.extend_from_slice(NamePools::CYBERPUNK_FIRST);
    terms.extend_from_slice(NamePools::CYBERPUNK_HANDLES);
    terms.extend_from_slice(NamePools::COSMIC_HORROR_MALE);
    terms.extend_from_slice(NamePools::COSMIC_HORROR_FEMALE);
    terms.extend_from_slice(NamePools::COSMIC_HORROR_LAST);
    terms.extend_from_slice(NamePools::POST_APOC_FIRST);
    terms.extend_from_slice(NamePools::WESTERN_MALE);
    terms.extend_from_slice(NamePools::WESTERN_FEMALE);
    terms.extend_from_slice(NamePools::WESTERN_LAST);

    // Add equipment
    terms.extend_from_slice(EquipmentPools::FANTASY_MELEE);
    terms.extend_from_slice(EquipmentPools::FANTASY_RANGED);
    terms.extend_from_slice(EquipmentPools::FANTASY_MAGICAL);
    terms.extend_from_slice(EquipmentPools::SCIFI_ENERGY);
    terms.extend_from_slice(EquipmentPools::SCIFI_KINETIC);
    terms.extend_from_slice(EquipmentPools::CYBERPUNK_FIREARMS);
    terms.extend_from_slice(EquipmentPools::CYBERPUNK_MELEE);
    terms.extend_from_slice(EquipmentPools::CYBERPUNK_CYBERWARE);
    terms.extend_from_slice(EquipmentPools::POST_APOC_FIREARMS);
    terms.extend_from_slice(EquipmentPools::POST_APOC_MELEE);
    terms.extend_from_slice(EquipmentPools::POST_APOC_SURVIVAL);
    terms.extend_from_slice(EquipmentPools::WESTERN_PISTOLS);
    terms.extend_from_slice(EquipmentPools::WESTERN_RIFLES);

    terms
}

/// Build a lookup map for genre detection from text.
pub fn build_genre_keywords() -> HashMap<&'static str, TTRPGGenre> {
    let mut map = HashMap::new();

    // Fantasy keywords
    for kw in &["magic", "spell", "wizard", "dragon", "elf", "dwarf", "orc", "dungeon", "castle", "knight"] {
        map.insert(*kw, TTRPGGenre::Fantasy);
    }

    // Sci-Fi keywords
    for kw in &["starship", "laser", "plasma", "alien", "space", "galaxy", "planet", "asteroid", "warp"] {
        map.insert(*kw, TTRPGGenre::SciFi);
    }

    // Cyberpunk keywords
    for kw in &["cyberware", "netrunner", "chrome", "neon", "corpo", "street", "implant", "hack", "ice"] {
        map.insert(*kw, TTRPGGenre::Cyberpunk);
    }

    // Cosmic Horror keywords
    for kw in &["sanity", "madness", "eldritch", "cosmic", "tentacle", "cultist", "ancient", "forbidden"] {
        map.insert(*kw, TTRPGGenre::CosmicHorror);
    }

    // Post-Apocalyptic keywords
    for kw in &["wasteland", "radiation", "mutant", "vault", "scavenger", "raider", "bunker", "fallout"] {
        map.insert(*kw, TTRPGGenre::PostApocalyptic);
    }

    // Western keywords
    for kw in &["gunslinger", "sheriff", "outlaw", "saloon", "frontier", "cowboy", "ranch", "deputy"] {
        map.insert(*kw, TTRPGGenre::Western);
    }

    // Superhero keywords
    for kw in &["superhero", "superpower", "vigilante", "sidekick", "villain", "hero", "cape", "mask"] {
        map.insert(*kw, TTRPGGenre::Superhero);
    }

    map
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genre_all() {
        let genres = TTRPGGenre::all();
        assert!(genres.len() >= 20);
        assert!(genres.contains(&TTRPGGenre::Fantasy));
        assert!(genres.contains(&TTRPGGenre::Cyberpunk));
    }

    #[test]
    fn test_class_for_genre() {
        let fantasy_classes = CharacterClass::for_genre(TTRPGGenre::Fantasy);
        assert!(fantasy_classes.contains(&CharacterClass::Fighter));
        assert!(fantasy_classes.contains(&CharacterClass::Wizard));

        let cyberpunk_classes = CharacterClass::for_genre(TTRPGGenre::Cyberpunk);
        assert!(cyberpunk_classes.contains(&CharacterClass::Netrunner));
        assert!(cyberpunk_classes.contains(&CharacterClass::Solo));
    }

    #[test]
    fn test_race_for_genre() {
        let fantasy_races = CharacterRace::for_genre(TTRPGGenre::Fantasy);
        assert!(fantasy_races.contains(&CharacterRace::Elf));
        assert!(fantasy_races.contains(&CharacterRace::Dwarf));

        let scifi_races = CharacterRace::for_genre(TTRPGGenre::SciFi);
        assert!(scifi_races.contains(&CharacterRace::Android));
        assert!(scifi_races.contains(&CharacterRace::Cyborg));
    }

    #[test]
    fn test_trait_categories() {
        assert_eq!(CharacterTrait::Agile.category(), TraitCategory::Physical);
        assert_eq!(CharacterTrait::Clever.category(), TraitCategory::Mental);
        assert_eq!(CharacterTrait::Brave.category(), TraitCategory::Emotional);
        assert_eq!(CharacterTrait::Charismatic.category(), TraitCategory::Social);
    }

    #[test]
    fn test_motivation_types() {
        assert!(!CharacterMotivation::Adventure.is_fear());
        assert!(CharacterMotivation::Death.is_fear());
        assert!(CharacterMotivation::Atonement.is_complex());
    }

    #[test]
    fn test_equipment_quality() {
        assert!(EquipmentQuality::Poor.value_multiplier() < 1.0);
        assert!(EquipmentQuality::Legendary.value_multiplier() > 10.0);
        assert!(EquipmentQuality::Artifact.effectiveness_multiplier() > 2.0);
    }

    #[test]
    fn test_name_pools_not_empty() {
        assert!(!NamePools::FANTASY_MALE.is_empty());
        assert!(!NamePools::FANTASY_FEMALE.is_empty());
        assert!(!NamePools::CYBERPUNK_HANDLES.is_empty());
        assert!(!NamePools::COSMIC_HORROR_LAST.is_empty());
    }

    #[test]
    fn test_equipment_pools_not_empty() {
        assert!(!EquipmentPools::FANTASY_MELEE.is_empty());
        assert!(!EquipmentPools::SCIFI_ENERGY.is_empty());
        assert!(!EquipmentPools::CYBERPUNK_CYBERWARE.is_empty());
        assert!(!EquipmentPools::POST_APOC_CHEMS.is_empty());
    }

    #[test]
    fn test_searchable_terms() {
        let terms = get_all_searchable_terms();
        assert!(terms.len() > 200);
        assert!(terms.contains(&"Longsword"));
        assert!(terms.contains(&"Laser Rifle"));
    }

    #[test]
    fn test_genre_keywords() {
        let keywords = build_genre_keywords();
        assert_eq!(keywords.get("dragon"), Some(&TTRPGGenre::Fantasy));
        assert_eq!(keywords.get("netrunner"), Some(&TTRPGGenre::Cyberpunk));
        assert_eq!(keywords.get("wasteland"), Some(&TTRPGGenre::PostApocalyptic));
    }
}
