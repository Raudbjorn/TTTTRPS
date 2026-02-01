//! Static data for location generation.
//!
//! This module contains all the hardcoded data arrays used for procedural
//! location generation, organized by location type and data category.

use super::types::Difficulty;

// ============================================================================
// Name Generation Data
// ============================================================================

pub const NAME_ADJECTIVES: &[&str] = &[
    "Old", "Golden", "Silver", "Red", "Black", "White", "Green", "Blue", "Rusty", "Ancient",
];

pub const TAVERN_NOUNS: &[&str] = &[
    "Dragon", "Griffin", "Stag", "Boar", "Fox", "Raven", "Wolf", "Bear", "Eagle", "Lion",
];

pub const SHOP_TYPES: &[&str] = &["Emporium", "Goods", "Supplies", "Trading Post"];

pub const DUNGEON_NAME_TYPES: &[&str] = &["Depths", "Ruins", "Crypts", "Halls", "Warrens"];

pub const FOREST_NAME_TYPES: &[&str] = &["Woods", "Forest", "Grove", "Thicket"];

pub const MOUNTAIN_NAME_TYPES: &[&str] = &["Peak", "Summit", "Mountain", "Heights"];

pub const NPC_NAMES: &[&str] = &[
    "Barnaby", "Eliza", "Marcus", "Helena", "Theron", "Lyra", "Gareth", "Mira",
];

// ============================================================================
// Description Templates
// ============================================================================

pub const DESCRIPTIONS: &[(&str, &str)] = &[
    ("tavern", "A well-worn establishment where travelers and locals gather to share tales over drinks."),
    ("inn", "A welcoming rest stop offering warm beds and hot meals to weary travelers."),
    ("dungeon", "A dark and foreboding underground complex, filled with dangers and forgotten treasures."),
    ("forest", "A dense woodland where ancient trees tower overhead and mysterious creatures lurk in the shadows."),
    ("cave", "A natural cavern system carved by ages of water and time."),
    ("ruins", "Crumbling remnants of a once-great civilization, now home to squatters and scavengers."),
    ("tower", "A tall spire reaching toward the sky, its original purpose now shrouded in mystery."),
    ("village", "A small settlement where simple folk go about their daily lives."),
    ("city", "A bustling metropolis teeming with life, commerce, and intrigue."),
    ("temple", "A sacred place of worship, maintained by devoted clergy."),
];

// ============================================================================
// Feature Data by Location Type
// ============================================================================

/// Feature tuple: (name, description, interactive, hidden, mechanical_effect)
pub type FeatureData = (&'static str, &'static str, bool, bool, Option<&'static str>);

pub const TAVERN_FEATURES: &[FeatureData] = &[
    ("Notice Board", "A cork board covered in job postings, wanted posters, and local advertisements", true, false, Some("Find quest hooks")),
    ("Trophy Wall", "Mounted heads and weapons from past adventurers who passed through", false, false, None),
    ("Grand Fireplace", "A massive stone hearth that dominates one wall, always crackling with warm flames", false, false, None),
    ("Bar Counter", "A long, polished wooden bar with brass fixtures and comfortable stools", true, false, Some("Order drinks, gather rumors")),
    ("Private Booth", "A curtained alcove in the back, favored by those with secrets to discuss", true, false, Some("Advantage on Stealth for private conversations")),
    ("Stage Corner", "A small raised platform where bards and entertainers perform", true, false, Some("Performance opportunity")),
    ("Cellar Trapdoor", "A reinforced trapdoor behind the bar leading to the ale cellar", true, true, Some("Investigation DC 12 to notice")),
];

pub const DUNGEON_FEATURES: &[FeatureData] = &[
    ("Ancient Runes", "Weathered inscriptions carved into the walls, glowing faintly with residual magic", true, false, Some("Arcana DC 15 to decipher")),
    ("Hidden Passage", "A concealed door behind a loose stone block", true, true, Some("Investigation DC 15 to find")),
    ("Crumbling Pillars", "Massive stone columns showing signs of age and structural weakness", true, false, Some("Attack or loud noise may cause collapse")),
    ("Ritual Circle", "A faded magical circle etched into the floor, its purpose unclear", true, false, Some("Arcana DC 13 to identify")),
    ("Drainage Grate", "A rusted iron grate covering a deep shaft that descends into darkness", true, false, Some("Leads to lower level")),
    ("Skeleton Alcoves", "Wall niches containing the bones of those who died here long ago", false, false, None),
    ("Trapped Chest", "An ornate chest sitting conspicuously in the center of the room", true, true, Some("Perception DC 14 to spot trap")),
];

pub const FOREST_FEATURES: &[FeatureData] = &[
    ("Ancient Oak", "A massive oak tree with gnarled roots that form natural shelters", true, false, Some("Safe rest location")),
    ("Fairy Ring", "A circle of mushrooms with an otherworldly glow at night", true, false, Some("Fey presence possible")),
    ("Hidden Spring", "A crystal-clear spring hidden among the undergrowth", true, true, Some("Survival DC 12 to find")),
    ("Hunting Blind", "A concealed platform in the trees used by local hunters", true, true, Some("Perception DC 14 to spot")),
    ("Carved Warning", "Strange symbols carved into a tree trunk", true, false, Some("History or Nature DC 13 to interpret")),
    ("Animal Trail", "A well-worn path through the brush used by forest creatures", true, false, Some("Advantage on Survival to track")),
];

pub const SHOP_FEATURES: &[FeatureData] = &[
    ("Display Cases", "Glass cases showing the shop's finest wares", false, false, None),
    ("Locked Cabinet", "A reinforced cabinet behind the counter for valuable items", true, true, Some("Contains rare merchandise")),
    ("Bargain Bin", "A basket of discounted or damaged goods near the entrance", true, false, Some("Roll d20 for hidden treasure")),
    ("Workshop Access", "A doorway leading to the back workshop where items are crafted", true, false, Some("Custom orders possible")),
    ("Wanted Board", "A small board showing items the proprietor wishes to purchase", true, false, Some("Selling opportunities")),
];

pub const CASTLE_FEATURES: &[FeatureData] = &[
    ("Grand Staircase", "An impressive marble staircase sweeping up to the upper floors", false, false, None),
    ("Throne Room", "The seat of power, adorned with banners and guarded by elite soldiers", true, false, Some("Audience with nobility possible")),
    ("Secret Passage", "A hidden corridor behind a tapestry for servants and spies", true, true, Some("Investigation DC 16 to find")),
    ("Arrow Slits", "Narrow windows designed for archers to defend the castle", true, false, Some("Cover during combat")),
    ("Portcullis", "A heavy iron gate that can be dropped to seal the entrance", true, false, Some("Strength DC 20 to lift when lowered")),
    ("Murder Holes", "Openings in the ceiling of the gatehouse for pouring oil or shooting arrows", true, true, Some("Perception DC 14 to notice")),
];

pub const TEMPLE_FEATURES: &[FeatureData] = &[
    ("Sacred Altar", "An ornate altar dedicated to the temple's deity", true, false, Some("Prayer grants Minor Blessing")),
    ("Confessional", "A private booth for confessions and spiritual guidance", true, false, Some("Information about local sins")),
    ("Healing Font", "A basin of holy water with restorative properties", true, false, Some("Heals 1d4 HP once per day")),
    ("Reliquary", "A protected case containing sacred relics", true, false, Some("Powerful divine magic")),
    ("Bell Tower Access", "A ladder leading up to the temple bells", true, false, Some("Signal the town, vantage point")),
    ("Crypt Entrance", "A sealed doorway leading to the crypts below", true, true, Some("Locked, key held by clergy")),
];

// ============================================================================
// Inhabitant Description Data
// ============================================================================

pub const BARKEEP_DESCRIPTIONS: &[&str] = &[
    "A stout, no-nonsense proprietor with a keen eye for trouble",
    "A jovial former adventurer who retired after one too many close calls",
    "A stern matriarch who runs the establishment with an iron fist",
    "A charming host with a talent for remembering every patron's favorite drink",
    "A grizzled veteran who keeps a loaded crossbow under the bar",
];

pub const BARMAID_DESCRIPTIONS: &[&str] = &[
    "A quick-witted server who hears everything and forgets nothing",
    "A cheerful young woman working to save money for her education",
    "A mysterious figure who seems to know more than they should",
    "A former noble fallen on hard times, maintaining dignity in service",
];

pub const SHOPKEEPER_DESCRIPTIONS: &[&str] = &[
    "A shrewd merchant with an eye for profit and a nose for forgeries",
    "An elderly craftsman who takes pride in quality over quantity",
    "A nervous proprietor always watching for thieves",
    "A friendly halfling who seems to have connections everywhere",
];

pub const GUARD_DESCRIPTIONS: &[&str] = &[
    "A bored soldier counting days until retirement",
    "A zealous young recruit eager to prove themselves",
    "A scarred veteran who has seen too much to be surprised",
    "A corrupt official open to the right kind of persuasion",
];

pub const PRIEST_DESCRIPTIONS: &[&str] = &[
    "A kindly elder devoted to helping the poor and sick",
    "A fanatical zealot who sees heresy in every shadow",
    "A pragmatic clergy member who understands political realities",
    "A young acolyte struggling with doubts about their faith",
];

// ============================================================================
// Secret Data by Category
// ============================================================================

/// Secret tuple: (description, difficulty, consequences, clues)
pub type SecretData = (
    &'static str,
    Difficulty,
    &'static str,
    &'static [&'static str],
);

pub const URBAN_SECRETS: &[SecretData] = &[
    (
        "The owner is involved in smuggling operations",
        Difficulty::Medium,
        "Could make a powerful enemy or ally",
        &["Late night deliveries", "Hidden basement"],
    ),
    (
        "A secret society meets here regularly",
        Difficulty::Hard,
        "Uncovering powerful conspirators",
        &["Mysterious hooded figures", "Strange symbols scratched into furniture"],
    ),
    (
        "The building was once used for dark rituals",
        Difficulty::VeryHard,
        "Awakening dormant evil",
        &["Strange cold spots", "Nightmares when sleeping here"],
    ),
    (
        "A murdered person is buried beneath the floor",
        Difficulty::Hard,
        "The ghost may seek justice or revenge",
        &["Occasional strange noises", "Bloodstains that reappear"],
    ),
    (
        "Valuable treasure is hidden in a secret compartment",
        Difficulty::Medium,
        "Wealth and potential conflict with owner",
        &["Hollow-sounding wall section", "Owner's nervous glances at one spot"],
    ),
];

pub const DUNGEON_SECRETS: &[SecretData] = &[
    (
        "A powerful artifact is sealed in the deepest chamber",
        Difficulty::VeryHard,
        "Great power or terrible curse",
        &["Ancient warnings", "Magical resonance"],
    ),
    (
        "The dungeon was built to contain something, not protect treasure",
        Difficulty::Hard,
        "The contained entity may be awakened",
        &["Inverted defensive structures", "Sealing runes"],
    ),
    (
        "A hidden exit leads to an unexpected location",
        Difficulty::Medium,
        "Strategic advantage or danger",
        &["Fresh air from nowhere", "Animal tracks leading in"],
    ),
    (
        "The original builders left a message for those who would follow",
        Difficulty::Medium,
        "Historical revelation or hidden treasure",
        &["Unusual stonework patterns", "Recurring symbols"],
    ),
    (
        "Someone is already living here secretly",
        Difficulty::Easy,
        "Potential ally or dangerous enemy",
        &["Fresh food scraps", "Recently used fire pit"],
    ),
];

pub const WILDERNESS_SECRETS: &[SecretData] = &[
    (
        "An ancient burial ground lies hidden nearby",
        Difficulty::Medium,
        "Disturbed spirits or ancient treasures",
        &["Unnaturally quiet area", "Stone markers hidden by overgrowth"],
    ),
    (
        "A rare magical plant grows only in one spot",
        Difficulty::Hard,
        "Valuable alchemical ingredient",
        &["Unusual wildlife behavior", "Faint magical glow at night"],
    ),
    (
        "A hermit with forbidden knowledge lives in hiding",
        Difficulty::Medium,
        "Dangerous secrets or powerful magic",
        &["Strange tracks", "Traps around an area"],
    ),
    (
        "A portal to another plane exists here",
        Difficulty::VeryHard,
        "Access to other realms or invasion",
        &["Reality distortions", "Strange creatures appearing"],
    ),
];

pub const TEMPLE_SECRETS: &[SecretData] = &[
    (
        "The clergy worship a different deity in secret",
        Difficulty::VeryHard,
        "Religious upheaval and dangerous enemies",
        &["Inconsistent iconography", "Nervous reactions to certain topics"],
    ),
    (
        "Sacred relics have been replaced with forgeries",
        Difficulty::Hard,
        "Scandal and valuable originals somewhere",
        &["Faded enchantments", "Clergy's over-protectiveness"],
    ),
    (
        "The crypt contains an undead creature the priests cannot destroy",
        Difficulty::Hard,
        "Powerful enemy or ally",
        &["Locked crypt doors", "Offerings left at the entrance"],
    ),
    (
        "A secret tunnel connects to the local criminal underground",
        Difficulty::Medium,
        "Corruption or useful connections",
        &["Coming and going at odd hours", "Unexplained wealth"],
    ),
];

// ============================================================================
// Encounter Data by Category
// ============================================================================

/// Encounter tuple: (name, description, trigger, rewards, optional)
pub type EncounterData = (
    &'static str,
    &'static str,
    &'static str,
    &'static [&'static str],
    bool,
);

pub const DUNGEON_ENCOUNTERS: &[EncounterData] = &[
    (
        "Guardian Beast",
        "A territorial creature protecting its lair",
        "Entering the main chamber",
        &["Beast parts", "Treasure it guards"],
        false,
    ),
    (
        "Trap Corridor",
        "A hallway filled with deadly mechanisms",
        "Proceeding without caution",
        &["Safe passage", "Salvageable trap parts"],
        false,
    ),
    (
        "Patrol Creatures",
        "A group of monsters making their rounds",
        "Random encounter chance each hour",
        &["Equipment", "Information about deeper levels"],
        true,
    ),
    (
        "Puzzle Chamber",
        "A room with a complex mechanism blocking progress",
        "Attempting to pass",
        &["Access to next area", "Hidden treasure compartment"],
        false,
    ),
    (
        "Boss Lair",
        "The den of a powerful creature",
        "Reaching the deepest chamber",
        &["Major treasure hoard", "Rare materials"],
        false,
    ),
    (
        "Ambush Point",
        "Clever predators waiting for prey",
        "Passing through narrow areas",
        &["Survival", "Predator resources"],
        true,
    ),
];

pub const WILDERNESS_ENCOUNTERS: &[EncounterData] = &[
    (
        "Predator Pack",
        "Hungry beasts on the hunt",
        "Camping or traveling at night",
        &["Safe passage", "Pelts and meat"],
        true,
    ),
    (
        "Territorial Creature",
        "A beast defending its home",
        "Entering marked territory",
        &["Access to area resources"],
        true,
    ),
    (
        "Natural Hazard",
        "Environmental danger like rockslide or flash flood",
        "Weather changes or wrong path",
        &["Safe passage"],
        true,
    ),
    (
        "Hostile Travelers",
        "Bandits or enemy patrol",
        "Random encounter on roads",
        &["Equipment", "Information"],
        true,
    ),
    (
        "Mystical Guardian",
        "A spirit or creature protecting sacred ground",
        "Approaching forbidden area",
        &["Blessing or curse", "Ancient knowledge"],
        false,
    ),
];

pub const URBAN_ENCOUNTERS: &[EncounterData] = &[
    (
        "Bar Fight",
        "Tensions boil over between patrons",
        "Saying the wrong thing or random chance",
        &["Reputation change", "New contacts"],
        true,
    ),
    (
        "Thieves",
        "Pickpockets or burglars at work",
        "Displaying wealth or being careless",
        &["Catching the thief", "Leads to thieves guild"],
        true,
    ),
    (
        "Guard Patrol",
        "City watch investigating disturbances",
        "Suspicious behavior or bad timing",
        &["Avoiding trouble", "Official help"],
        true,
    ),
    (
        "Street Performance",
        "Entertainer attracting a crowd",
        "Passing through the area",
        &["Information from gathered crowd"],
        true,
    ),
    (
        "Mysterious Stranger",
        "Someone with urgent business approaches",
        "Random chance or reputation",
        &["Quest hook", "Valuable information"],
        true,
    ),
];

// ============================================================================
// Loot Data
// ============================================================================

pub const DUNGEON_LOOT_ITEMS: &[&str] = &[
    "Ancient artifact",
    "Enchanted weapon",
    "Spellbook",
    "Gemstones",
    "Gold coins",
    "Magical ring",
    "Rare potion",
    "Cursed item",
];

pub const WILDERNESS_LOOT_ITEMS: &[&str] = &[
    "Rare herbs",
    "Monster parts",
    "Natural crystals",
    "Lost traveler's gear",
    "Hunter's cache",
];

pub const URBAN_LOOT_ITEMS: &[&str] = &[
    "Stolen goods",
    "Hidden savings",
    "Blackmail material",
    "Trade goods",
    "Contraband",
];
