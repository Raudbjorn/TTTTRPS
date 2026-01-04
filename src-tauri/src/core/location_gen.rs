//! Location Generation Module
//!
//! AI-powered generation of locations including taverns, dungeons, cities,
//! wilderness areas, and other points of interest for TTRPG campaigns.
//!
//! Supports both procedural (template-based) and AI-enhanced generation.
//! Each location includes:
//! - Rich descriptions and atmosphere
//! - Notable features (interactive and decorative)
//! - Inhabitants/NPCs with personalities
//! - Secrets and hidden elements
//! - Potential encounters
//! - Connected locations
//! - Map reference placeholders

use crate::core::llm::{LLMClient, LLMConfig, ChatMessage, ChatRequest, MessageRole};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use thiserror::Error;
use chrono::{DateTime, Utc};

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
// Location Types
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
            "temple" | "church" | "shrine" => Self::Temple,
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
            Self::Tavern, Self::Inn, Self::Shop, Self::Guild, Self::Temple,
            Self::Castle, Self::Manor, Self::Prison, Self::Slum, Self::Market,
            Self::City, Self::Town, Self::Village,
            Self::Forest, Self::Mountain, Self::Swamp, Self::Desert, Self::Plains,
            Self::Coast, Self::Island, Self::River, Self::Lake, Self::Cave,
            Self::Dungeon, Self::Ruins, Self::Tower, Self::Tomb, Self::Mine,
            Self::Stronghold, Self::Lair, Self::Camp, Self::Shrine, Self::Portal,
            Self::Planar, Self::Underwater, Self::Aerial,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atmosphere {
    pub lighting: String,
    pub sounds: Vec<String>,
    pub smells: Vec<String>,
    pub mood: String,
    pub weather: Option<String>,
    pub time_of_day_effects: Option<String>,
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

// ============================================================================
// Location Generator
// ============================================================================

pub struct LocationGenerator {
    llm_client: Option<LLMClient>,
}

impl Default for LocationGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl LocationGenerator {
    pub fn new() -> Self {
        Self { llm_client: None }
    }

    pub fn with_llm(llm_config: LLMConfig) -> Self {
        Self {
            llm_client: Some(LLMClient::new(llm_config)),
        }
    }

    /// Generate a location without LLM (uses templates)
    pub fn generate_quick(&self, options: &LocationGenerationOptions) -> Location {
        let mut rng = rand::thread_rng();


        let location_type = options.location_type.as_deref()
            .map(LocationType::from_str)
            .unwrap_or(LocationType::Tavern);

        let name = options.name.clone()
            .unwrap_or_else(|| self.generate_name(&location_type, &mut rng));

        let description = self.generate_description(&location_type, &options.theme, &mut rng);
        let atmosphere = self.generate_atmosphere(&location_type, &mut rng);

        let notable_features = self.generate_features(&location_type, &mut rng);

        let inhabitants = if options.include_inhabitants {
            self.generate_inhabitants(&location_type, &mut rng)
        } else {
            vec![]
        };

        let secrets = if options.include_secrets {
            self.generate_secrets(&location_type, &mut rng)
        } else {
            vec![]
        };

        let encounters = if options.include_encounters {
            self.generate_encounters(&location_type, options.danger_level.clone(), &mut rng)
        } else {
            vec![]
        };

        let loot_potential = if options.include_loot {
            Some(self.generate_loot(&location_type, &mut rng))
        } else {
            None
        };

        let tags = self.generate_tags(&location_type);
        let now = Utc::now();
        Location {
            id: Uuid::new_v4().to_string(),
            campaign_id: options.campaign_id.clone(),
            name,
            location_type,
            description,
            atmosphere,
            notable_features,
            inhabitants,
            secrets,
            encounters,
            connected_locations: vec![],
            loot_potential,
            map_reference: None,
            tags,
            notes: String::new(),
            created_at: now,
            updated_at: now,
        }
    }

    fn generate_tags(&self, loc_type: &LocationType) -> Vec<String> {
        let mut tags = vec![loc_type.display_name().to_lowercase()];
        match loc_type {
            LocationType::Tavern | LocationType::Inn | LocationType::Shop |
            LocationType::Guild | LocationType::Temple | LocationType::Market => {
                tags.push("urban".to_string());
                tags.push("social".to_string());
            }
            LocationType::Castle | LocationType::Manor | LocationType::Stronghold => {
                tags.push("fortification".to_string());
                tags.push("noble".to_string());
            }
            LocationType::City | LocationType::Town | LocationType::Village => {
                tags.push("settlement".to_string());
            }
            LocationType::Forest | LocationType::Mountain | LocationType::Swamp |
            LocationType::Desert | LocationType::Plains | LocationType::Coast |
            LocationType::Island | LocationType::River | LocationType::Lake => {
                tags.push("wilderness".to_string());
                tags.push("outdoor".to_string());
            }
            LocationType::Dungeon | LocationType::Cave | LocationType::Ruins |
            LocationType::Tower | LocationType::Tomb | LocationType::Mine |
            LocationType::Lair => {
                tags.push("adventure".to_string());
                tags.push("dangerous".to_string());
            }
            LocationType::Shrine | LocationType::Portal | LocationType::Planar => {
                tags.push("magical".to_string());
            }
            _ => {}
        }
        tags
    }

    /// Generate a location using LLM for rich descriptions
    pub async fn generate_detailed(&self, options: &LocationGenerationOptions) -> Result<Location> {
        let llm = self.llm_client.as_ref()
            .ok_or_else(|| LocationGenError::GenerationFailed("No LLM configured".to_string()))?;

        let prompt = self.build_prompt(options);
        let system = self.build_system_prompt(options);

        let request = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: prompt,
                    images: None,
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            system_prompt: Some(system),
            temperature: Some(0.8),
            max_tokens: Some(2000),
            provider: None,
            tools: None,
            tool_choice: None,
        };

        let response = llm.chat(request).await
            .map_err(|e| LocationGenError::LLMError(e.to_string()))?;

        self.parse_response(&response.content, options)
    }

    fn generate_name(&self, loc_type: &LocationType, rng: &mut impl rand::Rng) -> String {
        let adjectives = ["Old", "Golden", "Silver", "Red", "Black", "White", "Green", "Blue", "Rusty", "Ancient"];
        let tavern_nouns = ["Dragon", "Griffin", "Stag", "Boar", "Fox", "Raven", "Wolf", "Bear", "Eagle", "Lion"];
        let adjective = adjectives[rng.gen_range(0..adjectives.len())];

        match loc_type {
            LocationType::Tavern | LocationType::Inn => {
                let noun = tavern_nouns[rng.gen_range(0..tavern_nouns.len())];
                format!("The {} {}", adjective, noun)
            }
            LocationType::Shop => {
                let types = ["Emporium", "Goods", "Supplies", "Trading Post"];
                format!("{}'s {}", self.random_name(rng), types[rng.gen_range(0..types.len())])
            }
            LocationType::Dungeon | LocationType::Ruins => {
                let types = ["Depths", "Ruins", "Crypts", "Halls", "Warrens"];
                format!("{} {}", adjective, types[rng.gen_range(0..types.len())])
            }
            LocationType::Forest => {
                let types = ["Woods", "Forest", "Grove", "Thicket"];
                format!("{} {}", adjective, types[rng.gen_range(0..types.len())])
            }
            LocationType::Mountain => {
                let types = ["Peak", "Summit", "Mountain", "Heights"];
                format!("{} {}", adjective, types[rng.gen_range(0..types.len())])
            }
            _ => format!("{} {}", adjective, loc_type.display_name()),
        }
    }

    fn random_name(&self, rng: &mut impl rand::Rng) -> String {
        let names = ["Barnaby", "Eliza", "Marcus", "Helena", "Theron", "Lyra", "Gareth", "Mira"];
        names[rng.gen_range(0..names.len())].to_string()
    }

    fn generate_description(&self, loc_type: &LocationType, theme: &Option<String>, rng: &mut impl rand::Rng) -> String {
        let base = match loc_type {
            LocationType::Tavern => "A well-worn establishment where travelers and locals gather to share tales over drinks.",
            LocationType::Inn => "A welcoming rest stop offering warm beds and hot meals to weary travelers.",
            LocationType::Dungeon => "A dark and foreboding underground complex, filled with dangers and forgotten treasures.",
            LocationType::Forest => "A dense woodland where ancient trees tower overhead and mysterious creatures lurk in the shadows.",
            LocationType::Cave => "A natural cavern system carved by ages of water and time.",
            LocationType::Ruins => "Crumbling remnants of a once-great civilization, now home to squatters and scavengers.",
            LocationType::Tower => "A tall spire reaching toward the sky, its original purpose now shrouded in mystery.",
            LocationType::Village => "A small settlement where simple folk go about their daily lives.",
            LocationType::City => "A bustling metropolis teeming with life, commerce, and intrigue.",
            LocationType::Temple => "A sacred place of worship, maintained by devoted clergy.",
            _ => "A location of interest in the world.",
        };

        if let Some(t) = theme {
            format!("{} The atmosphere carries a distinctly {} feeling.", base, t)
        } else {
            base.to_string()
        }
    }

    fn generate_atmosphere(&self, loc_type: &LocationType, rng: &mut impl rand::Rng) -> Atmosphere {
        match loc_type {
            LocationType::Tavern | LocationType::Inn => Atmosphere {
                lighting: "Warm candlelight and flickering fireplace".to_string(),
                sounds: vec!["Murmured conversations".to_string(), "Clinking glasses".to_string(), "Crackling fire".to_string()],
                smells: vec!["Roasting meat".to_string(), "Spilled ale".to_string(), "Wood smoke".to_string()],
                mood: "Welcoming but watchful".to_string(),
                weather: None,
                time_of_day_effects: Some("Busier in the evening, quieter at midday".to_string()),
            },
            LocationType::Dungeon | LocationType::Cave => Atmosphere {
                lighting: "Pitch darkness, torches required".to_string(),
                sounds: vec!["Dripping water".to_string(), "Distant echoes".to_string(), "Scuttling creatures".to_string()],
                smells: vec!["Damp stone".to_string(), "Decay".to_string(), "Stale air".to_string()],
                mood: "Oppressive and dangerous".to_string(),
                weather: None,
                time_of_day_effects: None,
            },
            LocationType::Forest => Atmosphere {
                lighting: "Dappled sunlight through the canopy".to_string(),
                sounds: vec!["Birdsong".to_string(), "Rustling leaves".to_string(), "Distant wildlife".to_string()],
                smells: vec!["Pine".to_string(), "Damp earth".to_string(), "Wild flowers".to_string()],
                mood: "Serene but watchful".to_string(),
                weather: Some("Partly cloudy".to_string()),
                time_of_day_effects: Some("More dangerous at night".to_string()),
            },
            _ => Atmosphere {
                lighting: "Variable".to_string(),
                sounds: vec!["Ambient sounds".to_string()],
                smells: vec!["Local scents".to_string()],
                mood: "Neutral".to_string(),
                weather: None,
                time_of_day_effects: None,
            },
        }
    }

    fn generate_features(&self, loc_type: &LocationType, rng: &mut impl rand::Rng) -> Vec<NotableFeature> {
        use rand::seq::SliceRandom;

        let tavern_features = vec![
            ("Notice Board", "A cork board covered in job postings, wanted posters, and local advertisements", true, false, Some("Find quest hooks")),
            ("Trophy Wall", "Mounted heads and weapons from past adventurers who passed through", false, false, None),
            ("Grand Fireplace", "A massive stone hearth that dominates one wall, always crackling with warm flames", false, false, None),
            ("Bar Counter", "A long, polished wooden bar with brass fixtures and comfortable stools", true, false, Some("Order drinks, gather rumors")),
            ("Private Booth", "A curtained alcove in the back, favored by those with secrets to discuss", true, false, Some("Advantage on Stealth for private conversations")),
            ("Stage Corner", "A small raised platform where bards and entertainers perform", true, false, Some("Performance opportunity")),
            ("Cellar Trapdoor", "A reinforced trapdoor behind the bar leading to the ale cellar", true, true, Some("Investigation DC 12 to notice")),
        ];

        let dungeon_features = vec![
            ("Ancient Runes", "Weathered inscriptions carved into the walls, glowing faintly with residual magic", true, false, Some("Arcana DC 15 to decipher")),
            ("Hidden Passage", "A concealed door behind a loose stone block", true, true, Some("Investigation DC 15 to find")),
            ("Crumbling Pillars", "Massive stone columns showing signs of age and structural weakness", true, false, Some("Attack or loud noise may cause collapse")),
            ("Ritual Circle", "A faded magical circle etched into the floor, its purpose unclear", true, false, Some("Arcana DC 13 to identify")),
            ("Drainage Grate", "A rusted iron grate covering a deep shaft that descends into darkness", true, false, Some("Leads to lower level")),
            ("Skeleton Alcoves", "Wall niches containing the bones of those who died here long ago", false, false, None),
            ("Trapped Chest", "An ornate chest sitting conspicuously in the center of the room", true, true, Some("Perception DC 14 to spot trap")),
        ];

        let forest_features = vec![
            ("Ancient Oak", "A massive oak tree with gnarled roots that form natural shelters", true, false, Some("Safe rest location")),
            ("Fairy Ring", "A circle of mushrooms with an otherworldly glow at night", true, false, Some("Fey presence possible")),
            ("Hidden Spring", "A crystal-clear spring hidden among the undergrowth", true, true, Some("Survival DC 12 to find")),
            ("Hunting Blind", "A concealed platform in the trees used by local hunters", true, true, Some("Perception DC 14 to spot")),
            ("Carved Warning", "Strange symbols carved into a tree trunk", true, false, Some("History or Nature DC 13 to interpret")),
            ("Animal Trail", "A well-worn path through the brush used by forest creatures", true, false, Some("Advantage on Survival to track")),
        ];

        let shop_features = vec![
            ("Display Cases", "Glass cases showing the shop's finest wares", false, false, None),
            ("Locked Cabinet", "A reinforced cabinet behind the counter for valuable items", true, true, Some("Contains rare merchandise")),
            ("Bargain Bin", "A basket of discounted or damaged goods near the entrance", true, false, Some("Roll d20 for hidden treasure")),
            ("Workshop Access", "A doorway leading to the back workshop where items are crafted", true, false, Some("Custom orders possible")),
            ("Wanted Board", "A small board showing items the proprietor wishes to purchase", true, false, Some("Selling opportunities")),
        ];

        let castle_features = vec![
            ("Grand Staircase", "An impressive marble staircase sweeping up to the upper floors", false, false, None),
            ("Throne Room", "The seat of power, adorned with banners and guarded by elite soldiers", true, false, Some("Audience with nobility possible")),
            ("Secret Passage", "A hidden corridor behind a tapestry for servants and spies", true, true, Some("Investigation DC 16 to find")),
            ("Arrow Slits", "Narrow windows designed for archers to defend the castle", true, false, Some("Cover during combat")),
            ("Portcullis", "A heavy iron gate that can be dropped to seal the entrance", true, false, Some("Strength DC 20 to lift when lowered")),
            ("Murder Holes", "Openings in the ceiling of the gatehouse for pouring oil or shooting arrows", true, true, Some("Perception DC 14 to notice")),
        ];

        let temple_features = vec![
            ("Sacred Altar", "An ornate altar dedicated to the temple's deity", true, false, Some("Prayer grants Minor Blessing")),
            ("Confessional", "A private booth for confessions and spiritual guidance", true, false, Some("Information about local sins")),
            ("Healing Font", "A basin of holy water with restorative properties", true, false, Some("Heals 1d4 HP once per day")),
            ("Reliquary", "A protected case containing sacred relics", true, false, Some("Powerful divine magic")),
            ("Bell Tower Access", "A ladder leading up to the temple bells", true, false, Some("Signal the town, vantage point")),
            ("Crypt Entrance", "A sealed doorway leading to the crypts below", true, true, Some("Locked, key held by clergy")),
        ];

        let features_pool = match loc_type {
            LocationType::Tavern | LocationType::Inn => &tavern_features,
            LocationType::Dungeon | LocationType::Cave | LocationType::Ruins | LocationType::Tomb => &dungeon_features,
            LocationType::Forest | LocationType::Mountain | LocationType::Swamp => &forest_features,
            LocationType::Shop | LocationType::Market => &shop_features,
            LocationType::Castle | LocationType::Stronghold | LocationType::Manor => &castle_features,
            LocationType::Temple | LocationType::Shrine => &temple_features,
            _ => &tavern_features, // Default fallback
        };

        // Select 3-5 random features
        let count = rng.gen_range(3..=5).min(features_pool.len());
        let selected: Vec<_> = features_pool.choose_multiple(rng, count).collect();

        selected.iter().map(|(name, desc, interactive, hidden, effect)| {
            NotableFeature {
                name: name.to_string(),
                description: desc.to_string(),
                interactive: *interactive,
                hidden: *hidden,
                mechanical_effect: effect.map(|s| s.to_string()),
            }
        }).collect()
    }

    fn generate_inhabitants(&self, loc_type: &LocationType, rng: &mut impl rand::Rng) -> Vec<Inhabitant> {
        use rand::seq::SliceRandom;

        let barkeep_descriptions = [
            "A stout, no-nonsense proprietor with a keen eye for trouble",
            "A jovial former adventurer who retired after one too many close calls",
            "A stern matriarch who runs the establishment with an iron fist",
            "A charming host with a talent for remembering every patron's favorite drink",
            "A grizzled veteran who keeps a loaded crossbow under the bar",
        ];

        let barmaid_descriptions = [
            "A quick-witted server who hears everything and forgets nothing",
            "A cheerful young woman working to save money for her education",
            "A mysterious figure who seems to know more than they should",
            "A former noble fallen on hard times, maintaining dignity in service",
        ];

        let shopkeeper_descriptions = [
            "A shrewd merchant with an eye for profit and a nose for forgeries",
            "An elderly craftsman who takes pride in quality over quantity",
            "A nervous proprietor always watching for thieves",
            "A friendly halfling who seems to have connections everywhere",
        ];

        let guard_descriptions = [
            "A bored soldier counting days until retirement",
            "A zealous young recruit eager to prove themselves",
            "A scarred veteran who has seen too much to be surprised",
            "A corrupt official open to the right kind of persuasion",
        ];

        let priest_descriptions = [
            "A kindly elder devoted to helping the poor and sick",
            "A fanatical zealot who sees heresy in every shadow",
            "A pragmatic clergy member who understands political realities",
            "A young acolyte struggling with doubts about their faith",
        ];

        match loc_type {
            LocationType::Tavern | LocationType::Inn => {
                let mut inhabitants = vec![
                    Inhabitant {
                        name: self.random_name(rng),
                        role: "Barkeep".to_string(),
                        description: barkeep_descriptions.choose(rng).unwrap().to_string(),
                        disposition: Disposition::Neutral,
                        secrets: vec!["Knows local rumors and gossip".to_string(), "Has connections to the underground".to_string()],
                        services: vec!["Drinks".to_string(), "Food".to_string(), "Rooms".to_string(), "Local information".to_string()],
                    },
                ];
                if rng.gen_bool(0.7) {
                    inhabitants.push(Inhabitant {
                        name: self.random_name(rng),
                        role: "Server".to_string(),
                        description: barmaid_descriptions.choose(rng).unwrap().to_string(),
                        disposition: Disposition::Friendly,
                        secrets: vec!["Overhears private conversations".to_string()],
                        services: vec!["Table service".to_string(), "Room cleaning".to_string()],
                    });
                }
                if rng.gen_bool(0.5) {
                    inhabitants.push(Inhabitant {
                        name: self.random_name(rng),
                        role: "Regular Patron".to_string(),
                        description: "A local who practically lives at the bar".to_string(),
                        disposition: Disposition::Varies,
                        secrets: vec!["Has gambling debts".to_string()],
                        services: vec!["Local history".to_string(), "Introductions".to_string()],
                    });
                }
                inhabitants
            },
            LocationType::Shop | LocationType::Market => vec![
                Inhabitant {
                    name: self.random_name(rng),
                    role: "Shopkeeper".to_string(),
                    description: shopkeeper_descriptions.choose(rng).unwrap().to_string(),
                    disposition: Disposition::Friendly,
                    secrets: vec!["Has rare items for trusted customers".to_string(), "Fences stolen goods on the side".to_string()],
                    services: vec!["Buy/Sell goods".to_string(), "Identify items".to_string(), "Special orders".to_string()],
                },
            ],
            LocationType::Temple | LocationType::Shrine => {
                let mut inhabitants = vec![
                    Inhabitant {
                        name: self.random_name(rng),
                        role: "Head Priest".to_string(),
                        description: priest_descriptions.choose(rng).unwrap().to_string(),
                        disposition: Disposition::Friendly,
                        secrets: vec!["Knows dark secrets from confessions".to_string()],
                        services: vec!["Healing".to_string(), "Blessings".to_string(), "Spiritual guidance".to_string()],
                    },
                ];
                if rng.gen_bool(0.6) {
                    inhabitants.push(Inhabitant {
                        name: self.random_name(rng),
                        role: "Acolyte".to_string(),
                        description: "A young initiate learning the ways of the faith".to_string(),
                        disposition: Disposition::Friendly,
                        secrets: vec!["Witnessed something they shouldn't have".to_string()],
                        services: vec!["Minor healing".to_string(), "Temple tours".to_string()],
                    });
                }
                inhabitants
            },
            LocationType::Castle | LocationType::Stronghold | LocationType::Manor => vec![
                Inhabitant {
                    name: self.random_name(rng),
                    role: "Guard Captain".to_string(),
                    description: guard_descriptions.choose(rng).unwrap().to_string(),
                    disposition: Disposition::Wary,
                    secrets: vec!["Knows the patrol schedules".to_string()],
                    services: vec!["Security".to_string(), "Escort".to_string()],
                },
                Inhabitant {
                    name: self.random_name(rng),
                    role: "Steward".to_string(),
                    description: "A meticulous administrator who manages the household affairs".to_string(),
                    disposition: Disposition::Neutral,
                    secrets: vec!["Knows all the castle's secret passages".to_string()],
                    services: vec!["Appointments".to_string(), "Lodging arrangements".to_string()],
                },
            ],
            LocationType::Guild => vec![
                Inhabitant {
                    name: self.random_name(rng),
                    role: "Guildmaster".to_string(),
                    description: "A wealthy professional who rose through the ranks".to_string(),
                    disposition: Disposition::Neutral,
                    secrets: vec!["Controls more of the city than people realize".to_string()],
                    services: vec!["Guild membership".to_string(), "Contracts".to_string(), "Training".to_string()],
                },
            ],
            LocationType::Dungeon | LocationType::Cave | LocationType::Ruins => {
                if rng.gen_bool(0.3) {
                    vec![Inhabitant {
                        name: self.random_name(rng),
                        role: "Hermit".to_string(),
                        description: "A reclusive figure who has made this dark place their home".to_string(),
                        disposition: Disposition::Wary,
                        secrets: vec!["Knows safe paths through the area".to_string()],
                        services: vec!["Guidance".to_string(), "Shelter".to_string()],
                    }]
                } else {
                    vec![]
                }
            },
            _ => vec![],
        }
    }

    fn generate_secrets(&self, loc_type: &LocationType, rng: &mut impl rand::Rng) -> Vec<Secret> {
        use rand::seq::SliceRandom;

        let urban_secrets = vec![
            ("The owner is involved in smuggling operations", Difficulty::Medium, "Could make a powerful enemy or ally", vec!["Late night deliveries", "Hidden basement"]),
            ("A secret society meets here regularly", Difficulty::Hard, "Uncovering powerful conspirators", vec!["Mysterious hooded figures", "Strange symbols scratched into furniture"]),
            ("The building was once used for dark rituals", Difficulty::VeryHard, "Awakening dormant evil", vec!["Strange cold spots", "Nightmares when sleeping here"]),
            ("A murdered person is buried beneath the floor", Difficulty::Hard, "The ghost may seek justice or revenge", vec!["Occasional strange noises", "Bloodstains that reappear"]),
            ("Valuable treasure is hidden in a secret compartment", Difficulty::Medium, "Wealth and potential conflict with owner", vec!["Hollow-sounding wall section", "Owner's nervous glances at one spot"]),
        ];

        let dungeon_secrets = vec![
            ("A powerful artifact is sealed in the deepest chamber", Difficulty::VeryHard, "Great power or terrible curse", vec!["Ancient warnings", "Magical resonance"]),
            ("The dungeon was built to contain something, not protect treasure", Difficulty::Hard, "The contained entity may be awakened", vec!["Inverted defensive structures", "Sealing runes"]),
            ("A hidden exit leads to an unexpected location", Difficulty::Medium, "Strategic advantage or danger", vec!["Fresh air from nowhere", "Animal tracks leading in"]),
            ("The original builders left a message for those who would follow", Difficulty::Medium, "Historical revelation or hidden treasure", vec!["Unusual stonework patterns", "Recurring symbols"]),
            ("Someone is already living here secretly", Difficulty::Easy, "Potential ally or dangerous enemy", vec!["Fresh food scraps", "Recently used fire pit"]),
        ];

        let wilderness_secrets = vec![
            ("An ancient burial ground lies hidden nearby", Difficulty::Medium, "Disturbed spirits or ancient treasures", vec!["Unnaturally quiet area", "Stone markers hidden by overgrowth"]),
            ("A rare magical plant grows only in one spot", Difficulty::Hard, "Valuable alchemical ingredient", vec!["Unusual wildlife behavior", "Faint magical glow at night"]),
            ("A hermit with forbidden knowledge lives in hiding", Difficulty::Medium, "Dangerous secrets or powerful magic", vec!["Strange tracks", "Traps around an area"]),
            ("A portal to another plane exists here", Difficulty::VeryHard, "Access to other realms or invasion", vec!["Reality distortions", "Strange creatures appearing"]),
        ];

        let temple_secrets = vec![
            ("The clergy worship a different deity in secret", Difficulty::VeryHard, "Religious upheaval and dangerous enemies", vec!["Inconsistent iconography", "Nervous reactions to certain topics"]),
            ("Sacred relics have been replaced with forgeries", Difficulty::Hard, "Scandal and valuable originals somewhere", vec!["Faded enchantments", "Clergy's over-protectiveness"]),
            ("The crypt contains an undead creature the priests cannot destroy", Difficulty::Hard, "Powerful enemy or ally", vec!["Locked crypt doors", "Offerings left at the entrance"]),
            ("A secret tunnel connects to the local criminal underground", Difficulty::Medium, "Corruption or useful connections", vec!["Coming and going at odd hours", "Unexplained wealth"]),
        ];

        let secrets_pool = match loc_type {
            LocationType::Tavern | LocationType::Inn | LocationType::Shop | LocationType::Market | LocationType::Guild => &urban_secrets,
            LocationType::Dungeon | LocationType::Cave | LocationType::Ruins | LocationType::Tomb | LocationType::Mine => &dungeon_secrets,
            LocationType::Forest | LocationType::Mountain | LocationType::Swamp | LocationType::Desert | LocationType::Plains => &wilderness_secrets,
            LocationType::Temple | LocationType::Shrine => &temple_secrets,
            _ => &urban_secrets,
        };

        // Select 1-3 secrets
        let count = rng.gen_range(1..=3).min(secrets_pool.len());
        let selected: Vec<_> = secrets_pool.choose_multiple(rng, count).collect();

        selected.iter().map(|(desc, diff, consequences, clues)| {
            Secret {
                description: desc.to_string(),
                difficulty_to_discover: diff.clone(),
                consequences_if_revealed: consequences.to_string(),
                clues: clues.iter().map(|s| s.to_string()).collect(),
            }
        }).collect()
    }

    fn generate_encounters(&self, loc_type: &LocationType, danger: Option<Difficulty>, rng: &mut impl rand::Rng) -> Vec<Encounter> {
        use rand::seq::SliceRandom;

        let difficulty = danger.unwrap_or(Difficulty::Medium);

        let dungeon_encounters = vec![
            ("Guardian Beast", "A territorial creature protecting its lair", "Entering the main chamber", vec!["Beast parts", "Treasure it guards"], false),
            ("Trap Corridor", "A hallway filled with deadly mechanisms", "Proceeding without caution", vec!["Safe passage", "Salvageable trap parts"], false),
            ("Patrol Creatures", "A group of monsters making their rounds", "Random encounter chance each hour", vec!["Equipment", "Information about deeper levels"], true),
            ("Puzzle Chamber", "A room with a complex mechanism blocking progress", "Attempting to pass", vec!["Access to next area", "Hidden treasure compartment"], false),
            ("Boss Lair", "The den of a powerful creature", "Reaching the deepest chamber", vec!["Major treasure hoard", "Rare materials"], false),
            ("Ambush Point", "Clever predators waiting for prey", "Passing through narrow areas", vec!["Survival", "Predator resources"], true),
        ];

        let wilderness_encounters = vec![
            ("Predator Pack", "Hungry beasts on the hunt", "Camping or traveling at night", vec!["Safe passage", "Pelts and meat"], true),
            ("Territorial Creature", "A beast defending its home", "Entering marked territory", vec!["Access to area resources"], true),
            ("Natural Hazard", "Environmental danger like rockslide or flash flood", "Weather changes or wrong path", vec!["Safe passage"], true),
            ("Hostile Travelers", "Bandits or enemy patrol", "Random encounter on roads", vec!["Equipment", "Information"], true),
            ("Mystical Guardian", "A spirit or creature protecting sacred ground", "Approaching forbidden area", vec!["Blessing or curse", "Ancient knowledge"], false),
        ];

        let urban_encounters = vec![
            ("Bar Fight", "Tensions boil over between patrons", "Saying the wrong thing or random chance", vec!["Reputation change", "New contacts"], true),
            ("Thieves", "Pickpockets or burglars at work", "Displaying wealth or being careless", vec!["Catching the thief", "Leads to thieves guild"], true),
            ("Guard Patrol", "City watch investigating disturbances", "Suspicious behavior or bad timing", vec!["Avoiding trouble", "Official help"], true),
            ("Street Performance", "Entertainer attracting a crowd", "Passing through the area", vec!["Information from gathered crowd"], true),
            ("Mysterious Stranger", "Someone with urgent business approaches", "Random chance or reputation", vec!["Quest hook", "Valuable information"], true),
        ];

        let encounters_pool = match loc_type {
            LocationType::Dungeon | LocationType::Cave | LocationType::Ruins | LocationType::Tomb | LocationType::Mine | LocationType::Lair => &dungeon_encounters,
            LocationType::Forest | LocationType::Mountain | LocationType::Swamp | LocationType::Desert | LocationType::Plains | LocationType::Coast => &wilderness_encounters,
            LocationType::Tavern | LocationType::Inn | LocationType::City | LocationType::Town | LocationType::Village | LocationType::Market => &urban_encounters,
            _ => &wilderness_encounters,
        };

        // Select 2-4 encounters
        let count = rng.gen_range(2..=4).min(encounters_pool.len());
        let selected: Vec<_> = encounters_pool.choose_multiple(rng, count).collect();

        selected.iter().map(|(name, desc, trigger, rewards, optional)| {
            Encounter {
                name: name.to_string(),
                description: desc.to_string(),
                trigger: trigger.to_string(),
                difficulty: difficulty.clone(),
                rewards: rewards.iter().map(|s| s.to_string()).collect(),
                optional: *optional,
            }
        }).collect()
    }

    fn generate_loot(&self, loc_type: &LocationType, rng: &mut impl rand::Rng) -> LootPotential {
        use rand::seq::SliceRandom;

        let dungeon_items = ["Ancient artifact", "Enchanted weapon", "Spellbook", "Gemstones", "Gold coins", "Magical ring", "Rare potion", "Cursed item"];
        let wilderness_items = ["Rare herbs", "Monster parts", "Natural crystals", "Lost traveler's gear", "Hunter's cache"];
        let urban_items = ["Stolen goods", "Hidden savings", "Blackmail material", "Trade goods", "Contraband"];

        match loc_type {
            LocationType::Dungeon | LocationType::Tomb | LocationType::Ruins | LocationType::Lair => {
                let count = rng.gen_range(2..=4);
                let items: Vec<String> = dungeon_items.choose_multiple(rng, count)
                    .map(|s| s.to_string()).collect();
                LootPotential {
                    treasure_level: TreasureLevel::Rich,
                    notable_items: items,
                    hidden_caches: rng.gen_range(1..=3),
                }
            },
            LocationType::Cave | LocationType::Mine => {
                let count = rng.gen_range(1..=2);
                let items: Vec<String> = dungeon_items.choose_multiple(rng, count)
                    .map(|s| s.to_string()).collect();
                LootPotential {
                    treasure_level: TreasureLevel::Average,
                    notable_items: items,
                    hidden_caches: rng.gen_range(0..=2),
                }
            },
            LocationType::Forest | LocationType::Mountain | LocationType::Swamp => {
                let count = rng.gen_range(1..=2);
                let items: Vec<String> = wilderness_items.choose_multiple(rng, count)
                    .map(|s| s.to_string()).collect();
                LootPotential {
                    treasure_level: TreasureLevel::Modest,
                    notable_items: items,
                    hidden_caches: rng.gen_range(0..=1),
                }
            },
            LocationType::Castle | LocationType::Manor | LocationType::Stronghold => {
                LootPotential {
                    treasure_level: TreasureLevel::Rich,
                    notable_items: vec!["Noble treasures".to_string(), "Artwork".to_string(), "Jeweled items".to_string()],
                    hidden_caches: rng.gen_range(1..=2),
                }
            },
            LocationType::Tavern | LocationType::Inn => LootPotential {
                treasure_level: TreasureLevel::Poor,
                notable_items: vec![],
                hidden_caches: 0,
            },
            LocationType::Shop | LocationType::Market => {
                let count = rng.gen_range(0..=1);
                let items: Vec<String> = urban_items.choose_multiple(rng, count)
                    .map(|s| s.to_string()).collect();
                LootPotential {
                    treasure_level: TreasureLevel::Modest,
                    notable_items: items,
                    hidden_caches: rng.gen_range(0..=1),
                }
            },
            LocationType::Temple | LocationType::Shrine => {
                LootPotential {
                    treasure_level: TreasureLevel::Average,
                    notable_items: vec!["Religious artifacts".to_string(), "Offerings".to_string()],
                    hidden_caches: 1,
                }
            },
            _ => LootPotential {
                treasure_level: TreasureLevel::Modest,
                notable_items: vec![],
                hidden_caches: rng.gen_range(0..=1),
            },
        }
    }

    fn build_prompt(&self, options: &LocationGenerationOptions) -> String {
        let mut prompt = String::from("Generate a detailed location for a TTRPG campaign.\n\n");

        if let Some(loc_type) = &options.location_type {
            prompt.push_str(&format!("Location Type: {}\n", loc_type));
        }

        if let Some(name) = &options.name {
            prompt.push_str(&format!("Name: {}\n", name));
        }

        if let Some(size) = &options.size {
            prompt.push_str(&format!("Size: {:?}\n", size));
        }

        if let Some(theme) = &options.theme {
            prompt.push_str(&format!("Theme: {}\n", theme));
        }

        if let Some(setting) = &options.setting {
            prompt.push_str(&format!("Setting: {}\n", setting));
        }

        if let Some(danger) = &options.danger_level {
            prompt.push_str(&format!("Danger Level: {:?}\n", danger));
        }

        prompt.push_str("\nInclude:\n");
        if options.include_inhabitants { prompt.push_str("- NPCs/Inhabitants\n"); }
        if options.include_secrets { prompt.push_str("- Secrets and hidden elements\n"); }
        if options.include_encounters { prompt.push_str("- Possible encounters\n"); }
        if options.include_loot { prompt.push_str("- Treasure and rewards\n"); }

        prompt.push_str("\nProvide a rich, detailed description suitable for a game master to use.");
        prompt
    }

    fn build_system_prompt(&self, _options: &LocationGenerationOptions) -> String {
        "You are a creative TTRPG location designer. Generate detailed, \
         atmospheric locations with interesting features, NPCs, and secrets. \
         Make locations feel alive and full of adventure potential. \
         Return your response as a JSON object with the following structure:\n\
         {\"name\", \"description\", \"atmosphere\", \"notable_features\", \
         \"inhabitants\", \"secrets\", \"encounters\", \"loot_potential\"}".to_string()
    }

    fn parse_response(&self, content: &str, options: &LocationGenerationOptions) -> Result<Location> {
        // Try to parse JSON response, fall back to creating from text
        let json_str = if let Some(start) = content.find('{') {
            if let Some(end) = content.rfind('}') {
                &content[start..=end]
            } else {
                content
            }
        } else {
            content
        };

        // Try to parse the JSON response
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
            return self.build_location_from_json(&parsed, options);
        }

        // Fall back to creating a basic location from the content
        let location_type = options.location_type.as_deref()
            .map(LocationType::from_str)
            .unwrap_or(LocationType::Tavern);

        let tags = self.generate_tags(&location_type);
        let now = Utc::now();

        Ok(Location {
            id: Uuid::new_v4().to_string(),
            campaign_id: options.campaign_id.clone(),
            name: options.name.clone().unwrap_or_else(|| "Generated Location".to_string()),
            location_type,
            description: content.to_string(),
            atmosphere: Atmosphere {
                lighting: "Variable".to_string(),
                sounds: vec![],
                smells: vec![],
                mood: "Mysterious".to_string(),
                weather: None,
                time_of_day_effects: None,
            },
            notable_features: vec![],
            inhabitants: vec![],
            secrets: vec![],
            encounters: vec![],
            connected_locations: vec![],
            loot_potential: None,
            map_reference: None,
            tags,
            notes: String::new(),
            created_at: now,
            updated_at: now,
        })
    }

    fn build_location_from_json(
        &self,
        json: &serde_json::Value,
        options: &LocationGenerationOptions,
    ) -> Result<Location> {
        let location_type = options.location_type.as_deref()
            .map(LocationType::from_str)
            .unwrap_or(LocationType::Tavern);

        let name = json.get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| options.name.clone())
            .unwrap_or_else(|| "Generated Location".to_string());

        let description = json.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let atmosphere = if let Some(atm) = json.get("atmosphere") {
            Atmosphere {
                lighting: atm.get("lighting").and_then(|v| v.as_str()).unwrap_or("Variable").to_string(),
                sounds: atm.get("sounds")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default(),
                smells: atm.get("smells")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default(),
                mood: atm.get("mood").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                weather: atm.get("weather").and_then(|v| v.as_str()).map(String::from),
                time_of_day_effects: atm.get("time_of_day_effects").and_then(|v| v.as_str()).map(String::from),
            }
        } else {
            Atmosphere {
                lighting: "Variable".to_string(),
                sounds: vec![],
                smells: vec![],
                mood: "Mysterious".to_string(),
                weather: None,
                time_of_day_effects: None,
            }
        };

        let notable_features = json.get("notable_features")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|f| {
                    Some(NotableFeature {
                        name: f.get("name")?.as_str()?.to_string(),
                        description: f.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        interactive: f.get("interactive").and_then(|v| v.as_bool()).unwrap_or(false),
                        hidden: f.get("hidden").and_then(|v| v.as_bool()).unwrap_or(false),
                        mechanical_effect: f.get("mechanical_effect").and_then(|v| v.as_str()).map(String::from),
                    })
                }).collect()
            })
            .unwrap_or_default();

        let inhabitants = json.get("inhabitants")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|i| {
                    Some(Inhabitant {
                        name: i.get("name")?.as_str()?.to_string(),
                        role: i.get("role").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        description: i.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        disposition: match i.get("disposition").and_then(|v| v.as_str()).unwrap_or("neutral") {
                            "friendly" => Disposition::Friendly,
                            "hostile" => Disposition::Hostile,
                            "wary" => Disposition::Wary,
                            "varies" => Disposition::Varies,
                            _ => Disposition::Neutral,
                        },
                        secrets: i.get("secrets")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_default(),
                        services: i.get("services")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_default(),
                    })
                }).collect()
            })
            .unwrap_or_default();

        let secrets = json.get("secrets")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|s| {
                    Some(Secret {
                        description: s.get("description")?.as_str()?.to_string(),
                        difficulty_to_discover: match s.get("difficulty").and_then(|v| v.as_str()).unwrap_or("medium") {
                            "easy" => Difficulty::Easy,
                            "hard" => Difficulty::Hard,
                            "very_hard" => Difficulty::VeryHard,
                            "nearly_impossible" => Difficulty::NearlyImpossible,
                            _ => Difficulty::Medium,
                        },
                        consequences_if_revealed: s.get("consequences").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        clues: s.get("clues")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_default(),
                    })
                }).collect()
            })
            .unwrap_or_default();

        let encounters = json.get("encounters")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|e| {
                    Some(Encounter {
                        name: e.get("name")?.as_str()?.to_string(),
                        description: e.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        trigger: e.get("trigger").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        difficulty: match e.get("difficulty").and_then(|v| v.as_str()).unwrap_or("medium") {
                            "easy" => Difficulty::Easy,
                            "hard" => Difficulty::Hard,
                            "very_hard" => Difficulty::VeryHard,
                            "nearly_impossible" => Difficulty::NearlyImpossible,
                            _ => Difficulty::Medium,
                        },
                        rewards: e.get("rewards")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_default(),
                        optional: e.get("optional").and_then(|v| v.as_bool()).unwrap_or(true),
                    })
                }).collect()
            })
            .unwrap_or_default();

        let tags = self.generate_tags(&location_type);
        let now = Utc::now();

        Ok(Location {
            id: Uuid::new_v4().to_string(),
            campaign_id: options.campaign_id.clone(),
            name,
            location_type,
            description,
            atmosphere,
            notable_features,
            inhabitants,
            secrets,
            encounters,
            connected_locations: vec![],
            loot_potential: None,
            map_reference: options.map_reference.clone(),
            tags,
            notes: String::new(),
            created_at: now,
            updated_at: now,
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_type_parsing() {
        assert_eq!(LocationType::from_str("tavern"), LocationType::Tavern);
        assert_eq!(LocationType::from_str("dungeon"), LocationType::Dungeon);
        assert_eq!(LocationType::from_str("prison"), LocationType::Prison);
        assert_eq!(LocationType::from_str("forest"), LocationType::Forest);
    }

    #[test]
    fn test_quick_generation() {
        let generator = LocationGenerator::new();
        let options = LocationGenerationOptions {
            location_type: Some("tavern".to_string()),
            include_inhabitants: true,
            include_secrets: true,
            ..Default::default()
        };

        let location = generator.generate_quick(&options);
        assert!(!location.name.is_empty());
        assert_eq!(location.location_type, LocationType::Tavern);
    }
}
