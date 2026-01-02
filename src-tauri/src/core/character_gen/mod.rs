//! Character Generation Module
//!
//! Provides a trait-based multi-system character generation framework.
//! Supports character creation for multiple TTRPG systems including:
//! - D&D 5e / Pathfinder 2e (Fantasy)
//! - Call of Cthulhu (Horror)
//! - Cyberpunk / Shadowrun (Sci-Fi)
//! - Fate Core (Generic)
//! - World of Darkness (Urban Fantasy)
//! - Dungeon World (PbtA)
//! - GURPS (Universal)
//! - Warhammer Fantasy (Grimdark)

pub mod systems;
pub mod backstory;
pub mod prompts;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rand::Rng;
use uuid::Uuid;
use thiserror::Error;

// Re-export system generators
pub use systems::*;

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

    #[error("Backstory generation failed: {0}")]
    BackstoryError(String),
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
    pub race: Option<String>,
    pub class: Option<String>,
    pub level: u32,
    pub attributes: HashMap<String, AttributeValue>,
    pub skills: HashMap<String, i32>,
    pub traits: Vec<CharacterTrait>,
    pub equipment: Vec<Equipment>,
    pub background: CharacterBackground,
    pub backstory: Option<String>,
    pub notes: String,
    pub portrait_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum GameSystem {
    DnD5e,
    Pathfinder2e,
    CallOfCthulhu,
    Cyberpunk,
    Shadowrun,
    FateCore,
    WorldOfDarkness,
    DungeonWorld,
    GURPS,
    Warhammer,
    Custom(String),
}

impl GameSystem {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "dnd5e" | "d&d 5e" | "d&d" | "5e" | "dnd" => Self::DnD5e,
            "pathfinder" | "pf2e" | "pathfinder 2e" | "pathfinder2e" => Self::Pathfinder2e,
            "coc" | "call of cthulhu" | "cthulhu" => Self::CallOfCthulhu,
            "cyberpunk" | "cp2077" | "cyberpunk red" | "cpred" => Self::Cyberpunk,
            "shadowrun" | "sr" | "sr6" => Self::Shadowrun,
            "fate" | "fate core" | "fae" | "fate accelerated" => Self::FateCore,
            "wod" | "world of darkness" | "vtm" | "vampire" | "chronicles of darkness" => Self::WorldOfDarkness,
            "dw" | "dungeon world" | "dungeonworld" | "pbta" => Self::DungeonWorld,
            "gurps" => Self::GURPS,
            "warhammer" | "wfrp" | "warhammer fantasy" => Self::Warhammer,
            other => Self::Custom(other.to_string()),
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::DnD5e => "D&D 5th Edition",
            Self::Pathfinder2e => "Pathfinder 2nd Edition",
            Self::CallOfCthulhu => "Call of Cthulhu",
            Self::Cyberpunk => "Cyberpunk Red",
            Self::Shadowrun => "Shadowrun 6e",
            Self::FateCore => "Fate Core",
            Self::WorldOfDarkness => "World of Darkness",
            Self::DungeonWorld => "Dungeon World",
            Self::GURPS => "GURPS",
            Self::Warhammer => "Warhammer Fantasy",
            Self::Custom(name) => name,
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Self::DnD5e => "dnd5e",
            Self::Pathfinder2e => "pf2e",
            Self::CallOfCthulhu => "coc",
            Self::Cyberpunk => "cyberpunk",
            Self::Shadowrun => "shadowrun",
            Self::FateCore => "fate",
            Self::WorldOfDarkness => "wod",
            Self::DungeonWorld => "dungeon_world",
            Self::GURPS => "gurps",
            Self::Warhammer => "warhammer",
            Self::Custom(name) => name,
        }
    }

    /// List all supported systems
    pub fn all_systems() -> Vec<GameSystem> {
        vec![
            Self::DnD5e,
            Self::Pathfinder2e,
            Self::CallOfCthulhu,
            Self::Cyberpunk,
            Self::Shadowrun,
            Self::FateCore,
            Self::WorldOfDarkness,
            Self::DungeonWorld,
            Self::GURPS,
            Self::Warhammer,
        ]
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

    pub fn new_raw(base: i32) -> Self {
        Self {
            base,
            modifier: 0,
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
    Aspect,     // Fate
    Stunt,      // Fate
    Merit,      // WoD
    Edge,       // Shadowrun
    Cyberware,  // Cyberpunk
    Talent,     // Warhammer
    Move,       // Dungeon World
    Advantage,  // GURPS
    Disadvantage, // GURPS
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    pub backstory_length: Option<BackstoryLength>,
    pub theme: Option<String>,
    pub campaign_setting: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum BackstoryLength {
    Brief,
    #[default]
    Medium,
    Detailed,
}

impl BackstoryLength {
    pub fn word_count(&self) -> (usize, usize) {
        match self {
            Self::Brief => (50, 100),
            Self::Medium => (150, 300),
            Self::Detailed => (400, 600),
        }
    }
}

// ============================================================================
// System Generator Trait
// ============================================================================

/// Trait for system-specific character generators
pub trait SystemGenerator: Send + Sync {
    /// Returns the game system this generator supports
    fn system(&self) -> GameSystem;

    /// Generate a character with the given options
    fn generate(&self, options: &GenerationOptions) -> Result<Character>;

    /// Get available races/ancestries for this system
    fn available_races(&self) -> Vec<String>;

    /// Get available classes/playbooks for this system
    fn available_classes(&self) -> Vec<String>;

    /// Get available backgrounds for this system
    fn available_backgrounds(&self) -> Vec<String>;

    /// Get the attribute names for this system
    fn attribute_names(&self) -> Vec<String>;

    /// Get default equipment for a class
    fn starting_equipment(&self, class: Option<&str>) -> Vec<Equipment>;

    /// Validate that options are appropriate for this system
    fn validate_options(&self, options: &GenerationOptions) -> Result<()> {
        // Default implementation accepts all options
        Ok(())
    }
}

// ============================================================================
// System Info (for frontend)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub races: Vec<String>,
    pub classes: Vec<String>,
    pub backgrounds: Vec<String>,
    pub attributes: Vec<String>,
    pub has_levels: bool,
    pub max_level: Option<u32>,
}

impl SystemInfo {
    pub fn from_generator(generator: &dyn SystemGenerator) -> Self {
        let system = generator.system();
        Self {
            id: system.id().to_string(),
            name: system.display_name().to_string(),
            description: Self::system_description(&system),
            races: generator.available_races(),
            classes: generator.available_classes(),
            backgrounds: generator.available_backgrounds(),
            attributes: generator.attribute_names(),
            has_levels: Self::system_has_levels(&system),
            max_level: Self::system_max_level(&system),
        }
    }

    fn system_description(system: &GameSystem) -> String {
        match system {
            GameSystem::DnD5e => "The world's most popular fantasy RPG".to_string(),
            GameSystem::Pathfinder2e => "A tactical fantasy RPG with deep customization".to_string(),
            GameSystem::CallOfCthulhu => "Lovecraftian horror investigation RPG".to_string(),
            GameSystem::Cyberpunk => "Dystopian future street-level action".to_string(),
            GameSystem::Shadowrun => "Cyberpunk meets fantasy in a dark future".to_string(),
            GameSystem::FateCore => "Narrative-focused universal RPG system".to_string(),
            GameSystem::WorldOfDarkness => "Modern horror with supernatural themes".to_string(),
            GameSystem::DungeonWorld => "Fiction-first fantasy adventure".to_string(),
            GameSystem::GURPS => "Generic Universal RolePlaying System".to_string(),
            GameSystem::Warhammer => "Grimdark fantasy in the Old World".to_string(),
            GameSystem::Custom(name) => format!("Custom system: {}", name),
        }
    }

    fn system_has_levels(system: &GameSystem) -> bool {
        matches!(system,
            GameSystem::DnD5e |
            GameSystem::Pathfinder2e |
            GameSystem::DungeonWorld |
            GameSystem::Warhammer
        )
    }

    fn system_max_level(system: &GameSystem) -> Option<u32> {
        match system {
            GameSystem::DnD5e => Some(20),
            GameSystem::Pathfinder2e => Some(20),
            GameSystem::DungeonWorld => Some(10),
            GameSystem::Warhammer => Some(4), // Career ranks
            _ => None,
        }
    }
}

// ============================================================================
// Character Generator Registry
// ============================================================================

/// Registry for all system generators
pub struct GeneratorRegistry {
    generators: HashMap<GameSystem, Box<dyn SystemGenerator>>,
}

impl GeneratorRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            generators: HashMap::new(),
        };

        // Register all built-in generators
        registry.register(Box::new(systems::dnd5e::DnD5eGenerator::new()));
        registry.register(Box::new(systems::pf2e::Pathfinder2eGenerator::new()));
        registry.register(Box::new(systems::coc::CallOfCthulhuGenerator::new()));
        registry.register(Box::new(systems::cyberpunk::CyberpunkGenerator::new()));
        registry.register(Box::new(systems::shadowrun::ShadowrunGenerator::new()));
        registry.register(Box::new(systems::fate::FateCoreGenerator::new()));
        registry.register(Box::new(systems::wod::WorldOfDarknessGenerator::new()));
        registry.register(Box::new(systems::dungeon_world::DungeonWorldGenerator::new()));
        registry.register(Box::new(systems::gurps::GURPSGenerator::new()));
        registry.register(Box::new(systems::warhammer::WarhammerGenerator::new()));

        registry
    }

    pub fn register(&mut self, generator: Box<dyn SystemGenerator>) {
        self.generators.insert(generator.system(), generator);
    }

    pub fn get(&self, system: &GameSystem) -> Option<&dyn SystemGenerator> {
        self.generators.get(system).map(|g| g.as_ref())
    }

    pub fn generate(&self, options: &GenerationOptions) -> Result<Character> {
        let system = options.system.as_deref()
            .map(GameSystem::from_str)
            .unwrap_or(GameSystem::DnD5e);

        let generator = self.get(&system)
            .ok_or_else(|| CharacterGenError::UnsupportedSystem(system.display_name().to_string()))?;

        generator.validate_options(options)?;
        generator.generate(options)
    }

    pub fn list_systems(&self) -> Vec<SystemInfo> {
        self.generators.values()
            .map(|g| SystemInfo::from_generator(g.as_ref()))
            .collect()
    }

    pub fn get_system_info(&self, system: &GameSystem) -> Option<SystemInfo> {
        self.get(system).map(|g| SystemInfo::from_generator(g))
    }
}

impl Default for GeneratorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Character Generator Facade (for backward compatibility with commands.rs)
// ============================================================================

/// Static character generator that uses the internal registry
pub struct CharacterGenerator;

impl CharacterGenerator {
    /// Generate a character using the registry
    pub fn generate(options: &GenerationOptions) -> Result<Character> {
        let registry = GeneratorRegistry::new();
        registry.generate(options)
    }

    /// Get list of supported system names
    pub fn supported_systems() -> Vec<String> {
        GameSystem::all_systems()
            .into_iter()
            .map(|s| s.display_name().to_string())
            .collect()
    }

    /// Get detailed info for all systems
    pub fn list_system_info() -> Vec<SystemInfo> {
        let registry = GeneratorRegistry::new();
        registry.list_systems()
    }

    /// Get info for a specific system
    pub fn get_system_info(system: &str) -> Option<SystemInfo> {
        let registry = GeneratorRegistry::new();
        let game_system = GameSystem::from_str(system);
        registry.get_system_info(&game_system)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

pub fn random_fantasy_name(rng: &mut impl Rng) -> String {
    let first = ["Aldric", "Branwen", "Caden", "Dara", "Elara",
                 "Finn", "Gwyn", "Hadrian", "Isolde", "Kael",
                 "Lyra", "Magnus", "Nadia", "Orin", "Petra"];
    let last = ["Blackwood", "Ironforge", "Silverleaf", "Stormwind",
                "Thornwood", "Winterborne", "Ravencrest", "Shadowmere",
                "Fireheart", "Nightbane", "Dawnbringer", "Mistwalker"];

    format!("{} {}",
        first[rng.gen_range(0..first.len())],
        last[rng.gen_range(0..last.len())]
    )
}

pub fn random_modern_name(rng: &mut impl Rng) -> String {
    let first = ["Alex", "Casey", "Jordan", "Morgan", "Quinn",
                 "Riley", "Sam", "Taylor", "Blake", "Cameron",
                 "Drew", "Emery", "Finley", "Harper", "Jude"];
    let last = ["Anderson", "Brooks", "Chen", "Davis", "Evans",
                "Foster", "Garcia", "Hayes", "Kim", "Lewis",
                "Martinez", "Nelson", "O'Brien", "Patel", "Rodriguez"];

    format!("{} {}",
        first[rng.gen_range(0..first.len())],
        last[rng.gen_range(0..last.len())]
    )
}

pub fn random_1920s_name(rng: &mut impl Rng) -> String {
    let first = ["Arthur", "Dorothy", "Edward", "Florence", "George",
                 "Helen", "James", "Margaret", "Robert", "Virginia",
                 "William", "Eleanor", "Charles", "Beatrice", "Harold"];
    let last = ["Blackwell", "Crawford", "Fitzgerald", "Harrison",
                "Montgomery", "Patterson", "Sinclair", "Thornton",
                "Whitmore", "Ashworth", "Pemberton", "Sterling"];

    format!("{} {}",
        first[rng.gen_range(0..first.len())],
        last[rng.gen_range(0..last.len())]
    )
}

pub fn random_cyberpunk_handle(rng: &mut impl Rng) -> String {
    let handles = ["Razor", "Chrome", "Neon", "Ghost", "Virus",
                   "Zero", "Glitch", "Spike", "Nova", "Crash",
                   "Static", "Cipher", "Flux", "Rogue", "Synth"];

    handles[rng.gen_range(0..handles.len())].to_string()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_system_parsing() {
        assert_eq!(GameSystem::from_str("dnd5e"), GameSystem::DnD5e);
        assert_eq!(GameSystem::from_str("D&D"), GameSystem::DnD5e);
        assert_eq!(GameSystem::from_str("pathfinder"), GameSystem::Pathfinder2e);
        assert_eq!(GameSystem::from_str("call of cthulhu"), GameSystem::CallOfCthulhu);
        assert_eq!(GameSystem::from_str("cyberpunk"), GameSystem::Cyberpunk);
        assert_eq!(GameSystem::from_str("fate"), GameSystem::FateCore);
        assert_eq!(GameSystem::from_str("gurps"), GameSystem::GURPS);
        assert_eq!(GameSystem::from_str("warhammer"), GameSystem::Warhammer);
    }

    #[test]
    fn test_registry_creation() {
        let registry = GeneratorRegistry::new();
        let systems = registry.list_systems();
        assert!(systems.len() >= 10); // At least 10 systems registered
    }

    #[test]
    fn test_attribute_value() {
        let attr = AttributeValue::new(14);
        assert_eq!(attr.base, 14);
        assert_eq!(attr.modifier, 2); // (14-10)/2 = 2

        let attr2 = AttributeValue::new(8);
        assert_eq!(attr2.modifier, -1); // (8-10)/2 = -1
    }
}
