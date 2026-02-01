//! Location types and data structures.
//!
//! This module contains all the type definitions for location generation,
//! separated from the generator logic for maintainability.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum LocationGenError {
    #[error("Generation failed: {0}")]
    GenerationFailed(String),

    #[error("LLM error: {0}")]
    LLMError(String),

    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
}

pub type Result<T> = std::result::Result<T, LocationGenError>;

// ============================================================================
// Core Location Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub id: String,
    pub campaign_id: Option<String>,
    pub name: String,
    pub location_type: LocationType,
    pub description: String,
    pub atmosphere: Atmosphere,
    pub notable_features: Vec<NotableFeature>,
    pub inhabitants: Vec<Inhabitant>,
    pub secrets: Vec<Secret>,
    pub encounters: Vec<Encounter>,
    pub connected_locations: Vec<LocationConnection>,
    pub loot_potential: Option<LootPotential>,
    pub map_reference: Option<MapReference>,
    pub tags: Vec<String>,
    pub notes: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LocationType {
    // Urban
    Tavern,
    Inn,
    Shop,
    Guild,
    Temple,
    Castle,
    Manor,
    Prison,
    Slum,
    Market,
    City,
    Town,
    Village,

    // Wilderness
    Forest,
    Mountain,
    Swamp,
    Desert,
    Plains,
    Coast,
    Island,
    River,
    Lake,
    Cave,

    // Adventure Sites
    Dungeon,
    Ruins,
    Tower,
    Tomb,
    Mine,
    Stronghold,
    Lair,
    Camp,
    Shrine,
    Portal,

    // Special
    Planar,
    Underwater,
    Aerial,
    Custom(String),
}

impl LocationType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "tavern" | "bar" | "pub" => Self::Tavern,
            "inn" | "hotel" => Self::Inn,
            "shop" | "store" | "merchant" => Self::Shop,
            "guild" | "guildhall" => Self::Guild,
            "temple" | "church" => Self::Temple,
            "shrine" => Self::Shrine,
            "castle" | "fortress" => Self::Castle,
            "manor" | "mansion" | "estate" => Self::Manor,
            "prison" | "jail" => Self::Prison,
            "slum" | "slums" => Self::Slum,
            "market" | "bazaar" => Self::Market,
            "city" => Self::City,
            "town" => Self::Town,
            "village" | "hamlet" => Self::Village,
            "forest" | "woods" | "woodland" => Self::Forest,
            "mountain" | "mountains" | "peak" => Self::Mountain,
            "swamp" | "marsh" | "bog" => Self::Swamp,
            "desert" | "wasteland" => Self::Desert,
            "plains" | "grassland" | "prairie" => Self::Plains,
            "coast" | "beach" | "shore" => Self::Coast,
            "island" => Self::Island,
            "river" => Self::River,
            "lake" => Self::Lake,
            "cave" | "cavern" | "grotto" => Self::Cave,
            "dungeon" => Self::Dungeon,
            "ruins" | "ruin" => Self::Ruins,
            "tower" => Self::Tower,
            "tomb" | "crypt" | "mausoleum" => Self::Tomb,
            "mine" => Self::Mine,
            "stronghold" | "keep" | "fort" => Self::Stronghold,
            "lair" | "den" => Self::Lair,
            "camp" | "encampment" => Self::Camp,
            "portal" | "gate" => Self::Portal,
            "planar" | "plane" | "demiplane" => Self::Planar,
            "underwater" | "undersea" => Self::Underwater,
            "aerial" | "floating" | "sky" => Self::Aerial,
            other => Self::Custom(other.to_string()),
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::Tavern => "Tavern",
            Self::Inn => "Inn",
            Self::Shop => "Shop",
            Self::Guild => "Guild Hall",
            Self::Temple => "Temple",
            Self::Castle => "Castle",
            Self::Manor => "Manor",
            Self::Prison => "Prison",
            Self::Slum => "Slum",
            Self::Market => "Market",
            Self::City => "City",
            Self::Town => "Town",
            Self::Village => "Village",
            Self::Forest => "Forest",
            Self::Mountain => "Mountain",
            Self::Swamp => "Swamp",
            Self::Desert => "Desert",
            Self::Plains => "Plains",
            Self::Coast => "Coast",
            Self::Island => "Island",
            Self::River => "River",
            Self::Lake => "Lake",
            Self::Cave => "Cave",
            Self::Dungeon => "Dungeon",
            Self::Ruins => "Ruins",
            Self::Tower => "Tower",
            Self::Tomb => "Tomb",
            Self::Mine => "Mine",
            Self::Stronghold => "Stronghold",
            Self::Lair => "Lair",
            Self::Camp => "Camp",
            Self::Shrine => "Shrine",
            Self::Portal => "Portal",
            Self::Planar => "Planar Location",
            Self::Underwater => "Underwater",
            Self::Aerial => "Aerial",
            Self::Custom(name) => name,
        }
    }

    pub fn all_types() -> Vec<LocationType> {
        vec![
            Self::Tavern,
            Self::Inn,
            Self::Shop,
            Self::Guild,
            Self::Temple,
            Self::Castle,
            Self::Manor,
            Self::Prison,
            Self::Slum,
            Self::Market,
            Self::City,
            Self::Town,
            Self::Village,
            Self::Forest,
            Self::Mountain,
            Self::Swamp,
            Self::Desert,
            Self::Plains,
            Self::Coast,
            Self::Island,
            Self::River,
            Self::Lake,
            Self::Cave,
            Self::Dungeon,
            Self::Ruins,
            Self::Tower,
            Self::Tomb,
            Self::Mine,
            Self::Stronghold,
            Self::Lair,
            Self::Camp,
            Self::Shrine,
            Self::Portal,
            Self::Planar,
            Self::Underwater,
            Self::Aerial,
        ]
    }
}

// ============================================================================
// Location Components
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atmosphere {
    pub lighting: String,
    pub sounds: Vec<String>,
    pub smells: Vec<String>,
    pub mood: String,
    pub weather: Option<String>,
    pub time_of_day_effects: Option<String>,
}

impl Default for Atmosphere {
    fn default() -> Self {
        Self {
            lighting: "Variable".to_string(),
            sounds: vec![],
            smells: vec![],
            mood: "Neutral".to_string(),
            weather: None,
            time_of_day_effects: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotableFeature {
    pub name: String,
    pub description: String,
    pub interactive: bool,
    pub hidden: bool,
    pub mechanical_effect: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inhabitant {
    pub name: String,
    pub role: String,
    pub description: String,
    pub disposition: Disposition,
    pub secrets: Vec<String>,
    pub services: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Disposition {
    Friendly,
    Neutral,
    Wary,
    Hostile,
    Varies,
}

impl Disposition {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "friendly" => Self::Friendly,
            "hostile" => Self::Hostile,
            "wary" => Self::Wary,
            "varies" => Self::Varies,
            _ => Self::Neutral,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    pub description: String,
    pub difficulty_to_discover: Difficulty,
    pub consequences_if_revealed: String,
    pub clues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    VeryHard,
    NearlyImpossible,
}

impl Difficulty {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "easy" => Self::Easy,
            "hard" => Self::Hard,
            "very_hard" | "veryhard" => Self::VeryHard,
            "nearly_impossible" | "nearlyimpossible" => Self::NearlyImpossible,
            _ => Self::Medium,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encounter {
    pub name: String,
    pub description: String,
    pub trigger: String,
    pub difficulty: Difficulty,
    pub rewards: Vec<String>,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationConnection {
    pub target_id: Option<String>,
    pub target_name: String,
    pub connection_type: ConnectionType,
    pub description: Option<String>,
    pub travel_time: Option<String>,
    pub hazards: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    Door,
    Path,
    Road,
    Stairs,
    Ladder,
    Portal,
    Secret,
    Water,
    Climb,
    Flight,
}

impl std::str::FromStr for ConnectionType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "door" => Ok(Self::Door),
            "path" => Ok(Self::Path),
            "road" => Ok(Self::Road),
            "stairs" => Ok(Self::Stairs),
            "ladder" => Ok(Self::Ladder),
            "portal" => Ok(Self::Portal),
            "secret" => Ok(Self::Secret),
            "water" => Ok(Self::Water),
            "climb" => Ok(Self::Climb),
            "flight" => Ok(Self::Flight),
            _ => Err(format!("Unknown connection type: {}", s)),
        }
    }
}

impl std::fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Door => write!(f, "door"),
            Self::Path => write!(f, "path"),
            Self::Road => write!(f, "road"),
            Self::Stairs => write!(f, "stairs"),
            Self::Ladder => write!(f, "ladder"),
            Self::Portal => write!(f, "portal"),
            Self::Secret => write!(f, "secret"),
            Self::Water => write!(f, "water"),
            Self::Climb => write!(f, "climb"),
            Self::Flight => write!(f, "flight"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootPotential {
    pub treasure_level: TreasureLevel,
    pub notable_items: Vec<String>,
    pub hidden_caches: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TreasureLevel {
    None,
    Poor,
    Modest,
    Average,
    Rich,
    Hoard,
    Legendary,
}

impl TreasureLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "poor" => Self::Poor,
            "modest" => Self::Modest,
            "average" => Self::Average,
            "rich" => Self::Rich,
            "hoard" => Self::Hoard,
            "legendary" => Self::Legendary,
            _ => Self::None,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::None => "none",
            Self::Poor => "poor",
            Self::Modest => "modest",
            Self::Average => "average",
            Self::Rich => "rich",
            Self::Hoard => "hoard",
            Self::Legendary => "legendary",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapReference {
    pub grid_position: Option<(i32, i32)>,
    pub floor: Option<i32>,
    pub notes: String,
}

// ============================================================================
// Generation Options
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocationGenerationOptions {
    pub location_type: Option<String>,
    pub name: Option<String>,
    pub size: Option<LocationSize>,
    pub theme: Option<String>,
    pub setting: Option<String>,
    pub danger_level: Option<Difficulty>,
    pub include_inhabitants: bool,
    pub include_secrets: bool,
    pub include_encounters: bool,
    pub include_loot: bool,
    pub connected_to: Option<String>,
    pub campaign_id: Option<String>,
    pub map_reference: Option<MapReference>,
    pub parent_location_id: Option<String>,
    pub use_ai: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum LocationSize {
    Tiny,
    Small,
    #[default]
    Medium,
    Large,
    Massive,
}
