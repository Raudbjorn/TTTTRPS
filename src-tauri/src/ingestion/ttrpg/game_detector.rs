//! Game System Detection Module
//!
//! Auto-detects the TTRPG game system from document content using
//! pattern matching on system-specific terminology.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

/// Supported TTRPG game systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameSystem {
    /// Dungeons & Dragons 5th Edition
    DnD5e,
    /// Dungeons & Dragons (other editions - OSR compatible)
    DnDClassic,
    /// Pathfinder 2nd Edition
    Pathfinder2e,
    /// Pathfinder 1st Edition
    Pathfinder1e,
    /// Call of Cthulhu
    CallOfCthulhu,
    /// Delta Green (modern Cthulhu horror)
    DeltaGreen,
    /// World of Darkness / Chronicles of Darkness
    WorldOfDarkness,
    /// Shadowrun
    Shadowrun,
    /// FATE / FATE Core / FATE Accelerated
    Fate,
    /// Powered by the Apocalypse games
    PbtA,
    /// Blades in the Dark
    BladesInTheDark,
    /// Forged in the Dark (BitD derivatives)
    ForgedInTheDark,
    /// Savage Worlds
    SavageWorlds,
    /// OSR (Old School Renaissance / Revival)
    OSR,
    /// Mothership (sci-fi horror)
    Mothership,
    /// Traveller
    Traveller,
    /// Cypher System (Numenera, The Strange)
    CypherSystem,
    /// Year Zero Engine (Alien, Vaesen, etc.)
    YearZeroEngine,
    /// GURPS
    GURPS,
    /// Unknown or unsupported system
    Other,
}

impl GameSystem {
    /// Get a machine-readable identifier for this game system.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DnD5e => "dnd5e",
            Self::DnDClassic => "dnd_classic",
            Self::Pathfinder2e => "pf2e",
            Self::Pathfinder1e => "pf1e",
            Self::CallOfCthulhu => "coc",
            Self::DeltaGreen => "delta_green",
            Self::WorldOfDarkness => "wod",
            Self::Shadowrun => "shadowrun",
            Self::Fate => "fate",
            Self::PbtA => "pbta",
            Self::BladesInTheDark => "bitd",
            Self::ForgedInTheDark => "fitd",
            Self::SavageWorlds => "savage_worlds",
            Self::OSR => "osr",
            Self::Mothership => "mothership",
            Self::Traveller => "traveller",
            Self::CypherSystem => "cypher",
            Self::YearZeroEngine => "year_zero",
            Self::GURPS => "gurps",
            Self::Other => "other",
        }
    }

    /// Get a human-readable display name for this game system.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::DnD5e => "D&D 5th Edition",
            Self::DnDClassic => "D&D Classic/OSR",
            Self::Pathfinder2e => "Pathfinder 2e",
            Self::Pathfinder1e => "Pathfinder 1e",
            Self::CallOfCthulhu => "Call of Cthulhu",
            Self::DeltaGreen => "Delta Green",
            Self::WorldOfDarkness => "World of Darkness",
            Self::Shadowrun => "Shadowrun",
            Self::Fate => "FATE",
            Self::PbtA => "Powered by the Apocalypse",
            Self::BladesInTheDark => "Blades in the Dark",
            Self::ForgedInTheDark => "Forged in the Dark",
            Self::SavageWorlds => "Savage Worlds",
            Self::OSR => "OSR",
            Self::Mothership => "Mothership",
            Self::Traveller => "Traveller",
            Self::CypherSystem => "Cypher System",
            Self::YearZeroEngine => "Year Zero Engine",
            Self::GURPS => "GURPS",
            Self::Other => "Unknown System",
        }
    }

    /// Get the typical genre/theme for this game system.
    pub fn genre(&self) -> &'static str {
        match self {
            Self::DnD5e | Self::DnDClassic | Self::Pathfinder2e | Self::Pathfinder1e => "fantasy",
            Self::CallOfCthulhu => "cosmic horror",
            Self::DeltaGreen => "modern horror/conspiracy",
            Self::WorldOfDarkness => "urban fantasy/horror",
            Self::Shadowrun => "cyberpunk fantasy",
            Self::Fate => "generic/narrative",
            Self::PbtA => "narrative/genre-specific",
            Self::BladesInTheDark | Self::ForgedInTheDark => "heist/dark fantasy",
            Self::SavageWorlds => "pulp/action",
            Self::OSR => "classic fantasy",
            Self::Mothership => "sci-fi horror",
            Self::Traveller => "hard sci-fi",
            Self::CypherSystem => "science fantasy",
            Self::YearZeroEngine => "survival/horror",
            Self::GURPS => "generic/simulationist",
            Self::Other => "unknown",
        }
    }

    /// Parse a game system from its string identifier.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dnd5e" | "dnd_5e" | "d&d5e" => Some(Self::DnD5e),
            "dnd_classic" | "dndclassic" | "adnd" => Some(Self::DnDClassic),
            "pf2e" | "pathfinder2e" | "pathfinder_2e" => Some(Self::Pathfinder2e),
            "pf1e" | "pathfinder1e" | "pathfinder_1e" => Some(Self::Pathfinder1e),
            "coc" | "call_of_cthulhu" | "callofcthulhu" => Some(Self::CallOfCthulhu),
            "delta_green" | "deltagreen" | "dg" => Some(Self::DeltaGreen),
            "wod" | "world_of_darkness" | "worldofdarkness" => Some(Self::WorldOfDarkness),
            "shadowrun" | "sr" => Some(Self::Shadowrun),
            "fate" | "fate_core" => Some(Self::Fate),
            "pbta" | "powered_by_the_apocalypse" => Some(Self::PbtA),
            "bitd" | "blades_in_the_dark" | "bladesinthedark" => Some(Self::BladesInTheDark),
            "fitd" | "forged_in_the_dark" | "forgedinthedark" => Some(Self::ForgedInTheDark),
            "savage_worlds" | "savageworlds" | "sw" => Some(Self::SavageWorlds),
            "osr" | "old_school" => Some(Self::OSR),
            "mothership" | "mother_ship" => Some(Self::Mothership),
            "traveller" | "traveler" => Some(Self::Traveller),
            "cypher" | "cypher_system" | "numenera" => Some(Self::CypherSystem),
            "year_zero" | "yearzero" | "y0" | "year_zero_engine" => Some(Self::YearZeroEngine),
            "gurps" => Some(Self::GURPS),
            "other" | "unknown" => Some(Self::Other),
            _ => None,
        }
    }
}

// ============================================================================
// Detection
// ============================================================================

/// Detection result with confidence.
#[derive(Debug, Clone)]
pub struct DetectionResult {
    /// The detected game system
    pub system: GameSystem,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Indicators that matched
    pub matched_indicators: Vec<String>,
}

/// Indicator patterns for each game system.
struct SystemIndicators {
    /// Patterns that strongly indicate this system
    strong: Vec<&'static str>,
    /// Patterns that weakly indicate this system
    weak: Vec<&'static str>,
}

/// Detect the game system from document content.
///
/// # Arguments
/// * `text` - The document text to analyze
///
/// # Returns
/// * `Option<GameSystem>` - The detected system, or None if confidence is too low
pub fn detect_game_system(text: &str) -> Option<GameSystem> {
    let result = detect_game_system_with_confidence(text);
    if result.confidence >= 0.5 {
        Some(result.system)
    } else {
        None
    }
}

/// Detect the game system with detailed confidence information.
pub fn detect_game_system_with_confidence(text: &str) -> DetectionResult {
    let text_lower = text.to_lowercase();
    let indicators = get_system_indicators();

    let mut scores: HashMap<GameSystem, (f32, Vec<String>)> = HashMap::new();

    for (system, patterns) in &indicators {
        let mut score = 0.0_f32;
        let mut matched = Vec::new();

        // Strong indicators (1.0 each)
        for pattern in &patterns.strong {
            if text_lower.contains(pattern) {
                score += 1.0;
                matched.push(pattern.to_string());
            }
        }

        // Weak indicators (0.3 each)
        for pattern in &patterns.weak {
            if text_lower.contains(pattern) {
                score += 0.3;
                matched.push(pattern.to_string());
            }
        }

        scores.insert(*system, (score, matched));
    }

    // Find the system with the highest score
    let (best_system, (best_score, matched)) = scores
        .into_iter()
        .max_by(|(_, (a, _)), (_, (b, _))| a.partial_cmp(b).unwrap())
        .unwrap_or((GameSystem::Other, (0.0, vec![])));

    // Calculate confidence (normalized against threshold)
    let threshold = 3.0_f32; // Need at least 3 strong indicators for full confidence
    let confidence = (best_score / threshold).min(1.0);

    DetectionResult {
        system: if confidence >= 0.3 { best_system } else { GameSystem::Other },
        confidence,
        matched_indicators: matched,
    }
}

/// Get indicator patterns for all supported game systems.
fn get_system_indicators() -> HashMap<GameSystem, SystemIndicators> {
    let mut indicators = HashMap::new();

    // D&D 5e indicators
    indicators.insert(GameSystem::DnD5e, SystemIndicators {
        strong: vec![
            "armor class",
            "hit dice",
            "spell slots",
            "proficiency bonus",
            "5th edition",
            "5e",
            "dungeon master",
            "saving throw",
            "advantage",
            "disadvantage",
            "death saving throw",
            "cantrip",
            "wizard's handbook",
            "monster manual",
        ],
        weak: vec![
            "d20",
            "hit points",
            "ability score",
            "attack roll",
            "damage roll",
            "short rest",
            "long rest",
            "spellcasting",
        ],
    });

    // Pathfinder 2e indicators
    indicators.insert(GameSystem::Pathfinder2e, SystemIndicators {
        strong: vec![
            "three actions",
            "ancestry",
            "heritage",
            "proficiency rank",
            "pathfinder",
            "2nd edition",
            "paizo",
            "golarion",
            "archetypes",
            "trained",
            "expert",
            "master",
            "legendary",
        ],
        weak: vec![
            "feat",
            "skill check",
            "ability modifier",
            "reaction",
            "free action",
        ],
    });

    // Call of Cthulhu indicators
    indicators.insert(GameSystem::CallOfCthulhu, SystemIndicators {
        strong: vec![
            "sanity",
            "sanity check",
            "sanity points",
            "mythos",
            "investigator",
            "keeper",
            "cthulhu",
            "call of cthulhu",
            "chaosium",
            "luck points",
            "credit rating",
        ],
        weak: vec![
            "horror",
            "madness",
            "cosmic",
            "eldritch",
            "1920s",
        ],
    });

    // World of Darkness indicators
    indicators.insert(GameSystem::WorldOfDarkness, SystemIndicators {
        strong: vec![
            "storyteller",
            "dice pool",
            "vampire",
            "werewolf",
            "mage",
            "changeling",
            "world of darkness",
            "chronicles of darkness",
            "white wolf",
            "blood potency",
            "humanity",
        ],
        weak: vec![
            "willpower",
            "disciplines",
            "clan",
            "covenant",
        ],
    });

    // Shadowrun indicators
    indicators.insert(GameSystem::Shadowrun, SystemIndicators {
        strong: vec![
            "shadowrun",
            "nuyen",
            "decker",
            "rigger",
            "street samurai",
            "awakened",
            "technomancer",
            "matrix",
            "astral",
            "sixth world",
            "karma",
        ],
        weak: vec![
            "cyberware",
            "megacorp",
            "corp",
            "seattle",
            "essence",
        ],
    });

    // FATE indicators
    indicators.insert(GameSystem::Fate, SystemIndicators {
        strong: vec![
            "fate",
            "fate core",
            "fate points",
            "aspects",
            "invoke",
            "compel",
            "stunts",
            "fate dice",
            "fudge dice",
        ],
        weak: vec![
            "approach",
            "high concept",
            "trouble",
        ],
    });

    // PbtA indicators
    indicators.insert(GameSystem::PbtA, SystemIndicators {
        strong: vec![
            "powered by the apocalypse",
            "pbta",
            "moves",
            "playbook",
            "hard move",
            "soft move",
            "2d6",
            "7-9",
            "10+",
            "miss",
            "partial success",
        ],
        weak: vec![
            "mc",
            "basic moves",
            "advance",
        ],
    });

    // Delta Green indicators
    indicators.insert(GameSystem::DeltaGreen, SystemIndicators {
        strong: vec![
            "delta green",
            "handler",
            "agent",
            "the program",
            "the conspiracy",
            "the cowboy",
            "bonds",
            "san loss",
            "willpower points",
            "unnatural",
            "hypergeometry",
            "night at the opera",
            "arc dream",
        ],
        weak: vec![
            "need to know",
            "cell",
            "operation",
            "briefing",
            "majestic",
            "friendly",
        ],
    });

    // Blades in the Dark indicators
    indicators.insert(GameSystem::BladesInTheDark, SystemIndicators {
        strong: vec![
            "blades in the dark",
            "doskvol",
            "duskwall",
            "crew sheet",
            "position and effect",
            "desperate",
            "risky",
            "controlled",
            "stress",
            "trauma",
            "resistance roll",
            "flashback",
            "devil's bargain",
            "coin",
            "rep",
            "tier",
            "turf",
            "heat",
            "entanglements",
        ],
        weak: vec![
            "score",
            "downtime",
            "vice",
            "load",
        ],
    });

    // Forged in the Dark (generic)
    indicators.insert(GameSystem::ForgedInTheDark, SystemIndicators {
        strong: vec![
            "forged in the dark",
            "fitd",
            "action roll",
            "position and effect",
            "desperate position",
            "risky position",
            "controlled position",
            "limited effect",
            "standard effect",
            "great effect",
            "resistance roll",
            "devil's bargain",
        ],
        weak: vec![
            "stress",
            "trauma",
            "flashback",
            "load",
        ],
    });

    // Savage Worlds indicators
    indicators.insert(GameSystem::SavageWorlds, SystemIndicators {
        strong: vec![
            "savage worlds",
            "bennies",
            "wild card",
            "extras",
            "shaken",
            "raise",
            "exploding dice",
            "wild die",
            "pace",
            "parry",
            "toughness",
            "pinnacle",
        ],
        weak: vec![
            "edges",
            "hindrances",
            "setting rules",
            "d4",
            "d6",
            "d8",
            "d10",
            "d12",
        ],
    });

    // OSR indicators
    indicators.insert(GameSystem::OSR, SystemIndicators {
        strong: vec![
            "old school",
            "osr",
            "thac0",
            "descending ac",
            "ascending ac",
            "save vs",
            "saving throw vs",
            "reaction roll",
            "morale check",
            "b/x",
            "ose",
            "old-school essentials",
            "labyrinth lord",
            "basic/expert",
        ],
        weak: vec![
            "dungeon crawl",
            "hex crawl",
            "retro-clone",
            "rulings not rules",
        ],
    });

    // Mothership indicators
    indicators.insert(GameSystem::Mothership, SystemIndicators {
        strong: vec![
            "mothership",
            "stress",
            "panic check",
            "android",
            "scientist",
            "teamster",
            "marine",
            "warden",
            "hull breach",
            "dead planet",
            "tuesday knight",
        ],
        weak: vec![
            "space horror",
            "0hr",
            "ship",
        ],
    });

    // Traveller indicators
    indicators.insert(GameSystem::Traveller, SystemIndicators {
        strong: vec![
            "traveller",
            "mongoose",
            "imperium",
            "jump drive",
            "starship",
            "subsector",
            "uwp",
            "trade goods",
            "mustering out",
            "career",
            "terms",
            "patron",
        ],
        weak: vec![
            "2d6",
            "characteristic",
            "skill check",
        ],
    });

    // Cypher System indicators
    indicators.insert(GameSystem::CypherSystem, SystemIndicators {
        strong: vec![
            "cypher system",
            "numenera",
            "the strange",
            "cipher",
            "cypher",
            "effort",
            "edge",
            "pools",
            "might pool",
            "speed pool",
            "intellect pool",
            "gm intrusion",
            "monte cook",
        ],
        weak: vec![
            "descriptor",
            "focus",
            "type",
            "tier",
        ],
    });

    // Year Zero Engine indicators
    indicators.insert(GameSystem::YearZeroEngine, SystemIndicators {
        strong: vec![
            "year zero",
            "y0",
            "fria ligan",
            "free league",
            "mutant year zero",
            "forbidden lands",
            "vaesen",
            "alien rpg",
            "tales from the loop",
            "pushing",
            "base dice",
            "gear dice",
            "stress dice",
        ],
        weak: vec![
            "pride",
            "dark secret",
            "relationships",
        ],
    });

    // GURPS indicators
    indicators.insert(GameSystem::GURPS, SystemIndicators {
        strong: vec![
            "gurps",
            "generic universal",
            "steve jackson games",
            "character points",
            "advantages",
            "disadvantages",
            "3d6",
            "success roll",
            "basic set",
            "campaigns",
            "characters",
        ],
        weak: vec![
            "skill level",
            "attribute",
            "st",
            "dx",
            "iq",
            "ht",
        ],
    });

    // Pathfinder 1e indicators
    indicators.insert(GameSystem::Pathfinder1e, SystemIndicators {
        strong: vec![
            "pathfinder first edition",
            "pathfinder 1e",
            "pf1",
            "base attack bonus",
            "cmb",
            "cmd",
            "combat maneuver",
            "paizo",
            "golarion",
        ],
        weak: vec![
            "feat",
            "class feature",
            "caster level",
        ],
    });

    // D&D Classic/OSR indicators
    indicators.insert(GameSystem::DnDClassic, SystemIndicators {
        strong: vec![
            "ad&d",
            "adnd",
            "1st edition",
            "2nd edition",
            "basic d&d",
            "becmi",
            "rules cyclopedia",
            "thac0",
            "gygax",
            "arneson",
        ],
        weak: vec![
            "dungeon master",
            "hit dice",
            "saving throw",
        ],
    });

    indicators
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_dnd5e() {
        let text = r#"
            The goblin has Armor Class 15 and Hit Points 7 (2d6).
            It has proficiency bonus +2 and can make a saving throw.
            The DM may grant advantage on the roll.
        "#;

        let result = detect_game_system_with_confidence(text);
        assert_eq!(result.system, GameSystem::DnD5e);
        assert!(result.confidence >= 0.5);
    }

    #[test]
    fn test_detect_pathfinder2e() {
        let text = r#"
            The elf has a heritage of woodland ancestry.
            Using three actions, they can achieve a legendary proficiency rank.
            This is a Pathfinder 2nd Edition character.
        "#;

        let result = detect_game_system_with_confidence(text);
        assert_eq!(result.system, GameSystem::Pathfinder2e);
        assert!(result.confidence >= 0.5);
    }

    #[test]
    fn test_detect_coc() {
        let text = r#"
            The investigator must make a sanity check after witnessing
            the eldritch horror. The Keeper describes the mythos creature.
            Roll against your sanity points.
        "#;

        let result = detect_game_system_with_confidence(text);
        assert_eq!(result.system, GameSystem::CallOfCthulhu);
        assert!(result.confidence >= 0.5);
    }

    #[test]
    fn test_detect_unknown() {
        let text = "This is just regular text without any game system indicators.";

        let result = detect_game_system_with_confidence(text);
        assert!(result.confidence < 0.5);
    }

    #[test]
    fn test_game_system_as_str() {
        assert_eq!(GameSystem::DnD5e.as_str(), "dnd5e");
        assert_eq!(GameSystem::Pathfinder2e.as_str(), "pf2e");
        assert_eq!(GameSystem::CallOfCthulhu.as_str(), "coc");
    }
}
