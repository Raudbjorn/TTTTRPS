//! TTRPG Synonym Dictionary
//!
//! Comprehensive dictionary of TTRPG-specific abbreviations, synonyms,
//! and related terms for query expansion. Supports multiple game systems
//! including D&D 5e, Pathfinder 2e, and generic TTRPG terms.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ============================================================================
// Query Expansion Result
// ============================================================================

/// Result of query expansion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExpansionResult {
    /// Original query
    pub original: String,
    /// Expanded query
    pub expanded_query: String,
    /// Whether expansion was applied
    pub was_expanded: bool,
    /// Applied expansions
    pub expansions: Vec<ExpansionInfo>,
    /// Hints for the user
    pub hints: Vec<String>,
}

/// Information about an expansion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpansionInfo {
    /// Original term
    pub original: String,
    /// Expanded terms
    pub expanded_to: Vec<String>,
    /// Category of expansion
    pub category: String,
}

// ============================================================================
// Dice Notation
// ============================================================================

/// Parse and expand dice notation (d20, 2d6, etc.)
#[derive(Debug, Clone)]
pub struct DiceNotation {
    patterns: HashMap<String, String>,
}

impl DiceNotation {
    pub fn new() -> Self {
        let mut patterns = HashMap::new();
        // Common dice patterns
        patterns.insert("d4".to_string(), "four-sided die".to_string());
        patterns.insert("d6".to_string(), "six-sided die".to_string());
        patterns.insert("d8".to_string(), "eight-sided die".to_string());
        patterns.insert("d10".to_string(), "ten-sided die".to_string());
        patterns.insert("d12".to_string(), "twelve-sided die".to_string());
        patterns.insert("d20".to_string(), "twenty-sided die".to_string());
        patterns.insert("d100".to_string(), "percentile dice".to_string());
        patterns.insert("2d6".to_string(), "two six-sided dice".to_string());
        patterns.insert("3d6".to_string(), "three six-sided dice".to_string());
        patterns.insert("4d6".to_string(), "four six-sided dice".to_string());
        patterns.insert("1d20".to_string(), "one twenty-sided die".to_string());
        patterns.insert("2d10".to_string(), "two ten-sided dice".to_string());
        Self { patterns }
    }

    pub fn expand(&self, text: &str) -> Option<String> {
        self.patterns.get(&text.to_lowercase()).cloned()
    }

    pub fn is_dice_notation(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();
        self.patterns.contains_key(&text_lower) ||
            text_lower.chars().all(|c| c.is_ascii_digit() || c == 'd')
    }
}

impl Default for DiceNotation {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TTRPG Synonyms
// ============================================================================

/// TTRPG-specific synonym dictionary
pub struct TTRPGSynonyms {
    /// Abbreviations -> full terms
    abbreviations: HashMap<String, String>,
    /// Reverse lookup: full term -> abbreviations
    reverse_abbreviations: HashMap<String, Vec<String>>,
    /// Terms -> synonyms
    synonyms: HashMap<String, Vec<String>>,
    /// Related terms (broader relationships)
    related: HashMap<String, Vec<String>>,
    /// Game system specific terms
    system_terms: HashMap<String, HashMap<String, Vec<String>>>,
    /// Dice notation expander
    dice: DiceNotation,
    /// Common TTRPG typos and corrections
    common_typos: HashMap<String, String>,
}

impl TTRPGSynonyms {
    /// Create a new synonym dictionary with comprehensive TTRPG vocabulary
    pub fn new() -> Self {
        let mut dict = Self {
            abbreviations: HashMap::new(),
            reverse_abbreviations: HashMap::new(),
            synonyms: HashMap::new(),
            related: HashMap::new(),
            system_terms: HashMap::new(),
            dice: DiceNotation::new(),
            common_typos: HashMap::new(),
        };
        dict.load_core_vocabulary();
        dict.load_dnd_vocabulary();
        dict.load_pathfinder_vocabulary();
        dict.load_general_ttrpg_vocabulary();
        dict.load_monster_vocabulary();
        dict.load_spell_vocabulary();
        dict.load_equipment_vocabulary();
        dict.load_common_typos();
        dict.build_reverse_abbreviations();
        dict
    }

    /// Build reverse lookup for abbreviations
    fn build_reverse_abbreviations(&mut self) {
        for (abbr, full) in &self.abbreviations {
            self.reverse_abbreviations
                .entry(full.to_lowercase())
                .or_default()
                .push(abbr.clone());
        }
    }

    /// Load core TTRPG vocabulary (system-agnostic)
    fn load_core_vocabulary(&mut self) {
        // Comprehensive abbreviations
        let abbreviations = [
            // Stats and Attributes
            ("hp", "hit points"),
            ("ac", "armor class"),
            ("dc", "difficulty class"),
            ("str", "strength"),
            ("dex", "dexterity"),
            ("con", "constitution"),
            ("int", "intelligence"),
            ("wis", "wisdom"),
            ("cha", "charisma"),
            ("lvl", "level"),
            ("xp", "experience points"),
            ("exp", "experience"),

            // Currency
            ("gp", "gold pieces"),
            ("sp", "silver pieces"),
            ("cp", "copper pieces"),
            ("pp", "platinum pieces"),
            ("ep", "electrum pieces"),

            // Combat and Mechanics
            ("cr", "challenge rating"),
            ("atk", "attack"),
            ("dmg", "damage"),
            ("aoe", "area of effect"),
            ("bab", "base attack bonus"),
            ("thac0", "to hit armor class zero"),
            ("init", "initiative"),
            ("prof", "proficiency"),
            ("adv", "advantage"),
            ("disadv", "disadvantage"),
            ("hd", "hit dice"),
            ("sr", "spell resistance"),
            ("dr", "damage reduction"),
            ("mr", "magic resistance"),
            ("cmd", "combat maneuver defense"),
            ("cmb", "combat maneuver bonus"),
            ("tpk", "total party kill"),
            ("aoo", "attack of opportunity"),
            ("oa", "opportunity attack"),
            ("dpr", "damage per round"),
            ("rac", "race armor class"),
            ("nat", "natural"),
            ("nat20", "natural twenty"),
            ("nat1", "natural one"),
            ("crit", "critical hit"),

            // Roles and Characters
            ("npc", "non-player character"),
            ("pc", "player character"),
            ("dm", "dungeon master"),
            ("gm", "game master"),
            ("bbeg", "big bad evil guy"),

            // Distances and Time
            ("ft", "feet"),
            ("sq", "square"),
            ("rnd", "round"),
            ("min", "minute"),
            ("hr", "hour"),

            // Spell and Magic
            ("conc", "concentration"),
            ("rit", "ritual"),
            ("somatic", "somatic component"),
            ("verbal", "verbal component"),
            ("material", "material component"),
            ("vsm", "verbal somatic material"),
            ("vs", "verbal somatic"),
            ("aoe", "area of effect"),
            ("los", "line of sight"),
            ("loe", "line of effect"),

            // Saves
            ("ref", "reflex"),
            ("fort", "fortitude"),
            ("will", "will"),
            ("st", "saving throw"),

            // Misc
            ("raw", "rules as written"),
            ("rai", "rules as intended"),
            ("phb", "player's handbook"),
            ("dmg", "dungeon master's guide"),
            ("mm", "monster manual"),
            ("xgte", "xanathar's guide to everything"),
            ("tce", "tasha's cauldron of everything"),
            ("vgtm", "volo's guide to monsters"),
            ("mtof", "mordenkainen's tome of foes"),
            ("ua", "unearthed arcana"),
            ("al", "adventurer's league"),
            ("srd", "system reference document"),
            ("ogl", "open game license"),
        ];

        for (abbr, full) in abbreviations {
            self.abbreviations.insert(abbr.to_string(), full.to_string());
        }

        // Core synonyms
        let synonyms = [
            ("hit points", vec!["hp", "health", "life", "vitality", "life points", "health points"]),
            ("armor class", vec!["ac", "defense", "protection", "ac score", "armour class"]),
            ("difficulty class", vec!["dc", "target number", "check dc", "target dc"]),
            ("attack", vec!["strike", "hit", "assault", "atk", "swing", "slash"]),
            ("damage", vec!["harm", "injury", "dmg", "hurt", "wounds"]),
            ("spell", vec!["magic", "incantation", "cantrip", "enchantment", "casting"]),
            ("monster", vec!["creature", "beast", "enemy", "foe", "mob", "adversary"]),
            ("weapon", vec!["arm", "armament", "blade", "arms", "implement"]),
            ("saving throw", vec!["save", "saving", "st"]),
            ("ability score", vec!["stat", "attribute", "ability", "score"]),
            ("skill check", vec!["check", "roll", "test", "skill test"]),
            ("feat", vec!["talent", "ability", "feature", "perk", "power"]),
            ("race", vec!["species", "ancestry", "lineage", "heritage", "kin"]),
            ("class", vec!["profession", "archetype", "role", "calling", "vocation"]),
            ("level", vec!["tier", "rank", "lvl", "character level"]),
            ("gold", vec!["gp", "coins", "money", "currency", "treasure", "wealth"]),
            ("combat", vec!["battle", "fight", "encounter", "conflict", "skirmish"]),
            ("round", vec!["turn", "rnd", "combat round"]),
            ("bonus", vec!["modifier", "mod", "plus", "buff"]),
            ("penalty", vec!["minus", "malus", "negative", "debuff"]),
            ("rest", vec!["recuperate", "recover", "sleep", "downtime"]),
            ("death", vec!["dying", "dead", "kill", "slay", "fall"]),
            ("heal", vec!["cure", "restore", "recover", "mend", "regenerate"]),
            ("buff", vec!["enhance", "boost", "strengthen", "empower", "augment"]),
            ("debuff", vec!["weaken", "impair", "hinder", "curse", "afflict"]),
            ("initiative", vec!["init", "turn order", "combat order"]),
            ("proficiency", vec!["prof", "trained", "proficient"]),
            ("resistance", vec!["resist", "reduction", "halved"]),
            ("immunity", vec!["immune", "unaffected", "protected"]),
            ("vulnerability", vec!["vulnerable", "weakness", "double damage"]),
            ("concentration", vec!["conc", "focus", "maintain"]),
            ("ritual", vec!["rit", "ritual casting", "ceremony"]),
            ("movement", vec!["speed", "move", "travel", "locomotion"]),
            ("action", vec!["act", "standard action", "main action"]),
            ("bonus action", vec!["swift action", "minor action", "quick action"]),
            ("reaction", vec!["react", "immediate", "triggered action"]),
            ("opportunity attack", vec!["oa", "aoo", "attack of opportunity"]),
            ("critical hit", vec!["crit", "critical", "nat20 damage", "crit damage"]),
            ("advantage", vec!["adv", "upper hand", "favorable"]),
            ("disadvantage", vec!["disadv", "hindrance", "unfavorable"]),
            ("experience", vec!["xp", "exp", "experience points"]),
            ("dungeon master", vec!["dm", "game master", "gm", "referee", "storyteller"]),
            ("player character", vec!["pc", "character", "hero", "protagonist"]),
            ("non-player character", vec!["npc", "npc character", "supporting character"]),
        ];

        for (term, syns) in synonyms {
            self.synonyms.insert(
                term.to_string(),
                syns.into_iter().map(String::from).collect(),
            );
        }

        // Related terms
        let related = [
            ("combat", vec!["attack", "damage", "initiative", "action", "weapon", "armor", "hit", "miss", "critical"]),
            ("magic", vec!["spell", "cantrip", "ritual", "arcane", "divine", "component", "slot", "casting"]),
            ("character", vec!["class", "race", "level", "background", "ability", "skill", "feat", "alignment"]),
            ("adventure", vec!["quest", "mission", "dungeon", "exploration", "encounter", "treasure", "reward"]),
            ("equipment", vec!["weapon", "armor", "item", "gear", "tool", "supplies", "potion", "scroll"]),
            ("healing", vec!["restore", "cure", "potion", "spell", "rest", "hit points", "recovery"]),
            ("stealth", vec!["sneak", "hide", "invisible", "perception", "detection", "surprise"]),
            ("social", vec!["persuasion", "deception", "intimidation", "insight", "charisma", "diplomacy"]),
            ("exploration", vec!["perception", "investigation", "survival", "navigation", "search", "find"]),
        ];

        for (term, rel) in related {
            self.related.insert(
                term.to_string(),
                rel.into_iter().map(String::from).collect(),
            );
        }
    }

    /// Load D&D-specific vocabulary
    fn load_dnd_vocabulary(&mut self) {
        let mut dnd = HashMap::new();

        // D&D Classes
        dnd.insert(
            "classes".to_string(),
            vec![
                "barbarian", "bard", "cleric", "druid", "fighter", "monk",
                "paladin", "ranger", "rogue", "sorcerer", "warlock", "wizard",
                "artificer", "blood hunter",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        );

        // D&D Subclasses (popular ones)
        dnd.insert(
            "subclasses".to_string(),
            vec![
                // Barbarian
                "berserker", "totem warrior", "ancestral guardian", "storm herald", "zealot",
                // Bard
                "lore", "valor", "glamour", "swords", "whispers", "eloquence",
                // Cleric
                "life", "light", "war", "knowledge", "tempest", "trickery", "forge", "grave", "order", "peace", "twilight",
                // Druid
                "land", "moon", "shepherd", "dreams", "spores", "stars", "wildfire",
                // Fighter
                "champion", "battle master", "eldritch knight", "samurai", "cavalier", "echo knight", "psi warrior", "rune knight",
                // Monk
                "open hand", "shadow", "four elements", "drunken master", "kensei", "sun soul", "long death", "astral self", "mercy",
                // Paladin
                "devotion", "ancients", "vengeance", "oathbreaker", "conquest", "redemption", "glory", "watchers",
                // Ranger
                "hunter", "beast master", "gloom stalker", "horizon walker", "monster slayer", "fey wanderer", "swarmkeeper", "drakewarden",
                // Rogue
                "thief", "assassin", "arcane trickster", "mastermind", "swashbuckler", "inquisitive", "scout", "phantom", "soulknife",
                // Sorcerer
                "draconic", "wild magic", "divine soul", "shadow", "storm", "aberrant mind", "clockwork soul",
                // Warlock
                "archfey", "fiend", "great old one", "celestial", "hexblade", "fathomless", "genie", "undead",
                // Wizard
                "abjuration", "conjuration", "divination", "enchantment", "evocation", "illusion", "necromancy", "transmutation", "war magic", "bladesinging", "chronurgy", "graviturgy", "scribes",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        );

        // D&D Races
        dnd.insert(
            "races".to_string(),
            vec![
                "human", "elf", "dwarf", "halfling", "gnome", "half-elf",
                "half-orc", "tiefling", "dragonborn", "aasimar", "goliath",
                "tabaxi", "kenku", "firbolg", "genasi", "warforged", "changeling",
                "shifter", "kalashtar", "aarakocra", "triton", "tortle", "yuan-ti",
                "gith", "githyanki", "githzerai", "bugbear", "goblin", "hobgoblin",
                "kobold", "orc", "leonin", "satyr", "fairy", "harengon", "owlin",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        );

        // D&D Conditions
        dnd.insert(
            "conditions".to_string(),
            vec![
                "blinded", "charmed", "deafened", "exhausted", "frightened",
                "grappled", "incapacitated", "invisible", "paralyzed",
                "petrified", "poisoned", "prone", "restrained", "stunned",
                "unconscious", "concentration",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        );

        // D&D Damage Types
        dnd.insert(
            "damage_types".to_string(),
            vec![
                "bludgeoning", "piercing", "slashing", "acid", "cold", "fire",
                "force", "lightning", "necrotic", "poison", "psychic", "radiant",
                "thunder",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        );

        // D&D Schools of Magic
        dnd.insert(
            "magic_schools".to_string(),
            vec![
                "abjuration", "conjuration", "divination", "enchantment",
                "evocation", "illusion", "necromancy", "transmutation",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        );

        // D&D Skills
        dnd.insert(
            "skills".to_string(),
            vec![
                "acrobatics", "animal handling", "arcana", "athletics",
                "deception", "history", "insight", "intimidation",
                "investigation", "medicine", "nature", "perception",
                "performance", "persuasion", "religion", "sleight of hand",
                "stealth", "survival",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        );

        // D&D Monster Types
        dnd.insert(
            "monster_types".to_string(),
            vec![
                "aberration", "beast", "celestial", "construct", "dragon",
                "elemental", "fey", "fiend", "giant", "humanoid", "monstrosity",
                "ooze", "plant", "undead",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        );

        // D&D Alignments
        dnd.insert(
            "alignments".to_string(),
            vec![
                "lawful good", "neutral good", "chaotic good",
                "lawful neutral", "true neutral", "chaotic neutral",
                "lawful evil", "neutral evil", "chaotic evil",
                "unaligned", "lg", "ng", "cg", "ln", "n", "cn", "le", "ne", "ce",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        );

        self.system_terms.insert("dnd".to_string(), dnd.clone());
        self.system_terms.insert("d&d".to_string(), dnd.clone());
        self.system_terms.insert("5e".to_string(), dnd.clone());
        self.system_terms.insert("dnd5e".to_string(), dnd);
    }

    /// Load Pathfinder-specific vocabulary
    fn load_pathfinder_vocabulary(&mut self) {
        let mut pf = HashMap::new();

        // Pathfinder-specific actions
        pf.insert(
            "actions".to_string(),
            vec![
                "stride", "strike", "interact", "seek", "recall knowledge",
                "demoralize", "feint", "tumble through", "raise shield",
                "step", "crawl", "stand", "drop prone", "escape", "force open",
                "grab an edge", "high jump", "long jump", "release", "sustain a spell",
                "take cover", "point out", "ready", "delay", "dismiss",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        );

        // Pathfinder ancestries
        pf.insert(
            "ancestries".to_string(),
            vec![
                "human", "elf", "dwarf", "gnome", "goblin", "halfling",
                "leshy", "orc", "catfolk", "tengu", "kitsune", "fetchling",
                "kobold", "ratfolk", "grippli", "nagaji", "shisk", "vanara",
                "vishkanya", "ghoran", "kashrishi", "automaton", "android",
                "fleshwarp", "sprite", "azarketi", "strix", "anadi", "conrasu",
                "poppet", "skeleton",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        );

        // Pathfinder classes
        pf.insert(
            "classes".to_string(),
            vec![
                "alchemist", "barbarian", "bard", "champion", "cleric",
                "druid", "fighter", "gunslinger", "inventor", "investigator",
                "magus", "monk", "oracle", "psychic", "ranger", "rogue",
                "sorcerer", "summoner", "swashbuckler", "thaumaturge", "witch",
                "wizard", "kineticist",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        );

        // Pathfinder action economy
        pf.insert(
            "action_types".to_string(),
            vec![
                "single action", "two-action", "three-action", "free action",
                "reaction", "activity", "exploration", "downtime",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        );

        // Pathfinder conditions
        pf.insert(
            "conditions".to_string(),
            vec![
                "blinded", "broken", "clumsy", "concealed", "confused",
                "controlled", "dazzled", "deafened", "doomed", "drained",
                "dying", "encumbered", "enfeebled", "fascinated", "fatigued",
                "flat-footed", "fleeing", "frightened", "grabbed", "helpful",
                "hidden", "hostile", "immobilized", "indifferent", "invisible",
                "observed", "paralyzed", "persistent damage", "petrified",
                "quickened", "restrained", "sickened", "slowed", "stunned",
                "stupefied", "unconscious", "undetected", "unfriendly", "unnoticed",
                "wounded",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        );

        self.system_terms.insert("pathfinder".to_string(), pf.clone());
        self.system_terms.insert("pf2e".to_string(), pf.clone());
        self.system_terms.insert("pf2".to_string(), pf);
    }

    /// Load general TTRPG vocabulary
    fn load_general_ttrpg_vocabulary(&mut self) {
        // Common TTRPG terms for any system
        self.synonyms.insert(
            "advantage".to_string(),
            vec!["favorable", "edge", "upper hand", "boon"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "disadvantage".to_string(),
            vec!["unfavorable", "hindrance", "penalty", "bane"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "campaign".to_string(),
            vec!["adventure", "game", "chronicle", "saga", "story"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "session".to_string(),
            vec!["game night", "play session", "meeting", "gathering"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "backstory".to_string(),
            vec!["background", "history", "origin", "past", "lore"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "roleplay".to_string(),
            vec!["rp", "acting", "character play", "in-character"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "metagaming".to_string(),
            vec!["meta", "out of character", "ooc", "player knowledge"]
                .into_iter()
                .map(String::from)
                .collect(),
        );
    }

    /// Load monster vocabulary
    fn load_monster_vocabulary(&mut self) {
        // Common monster categories
        self.synonyms.insert(
            "undead".to_string(),
            vec!["zombie", "skeleton", "vampire", "ghost", "lich", "wraith", "wight", "specter", "ghoul", "mummy", "revenant"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "dragon".to_string(),
            vec!["wyrm", "drake", "wyvern", "chromatic", "metallic", "dragon turtle", "pseudodragon", "faerie dragon"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "demon".to_string(),
            vec!["fiend", "abyssal", "balor", "glabrezu", "hezrou", "marilith", "nalfeshnee", "vrock", "dretch", "quasit"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "devil".to_string(),
            vec!["fiend", "infernal", "pit fiend", "erinyes", "ice devil", "bone devil", "chain devil", "bearded devil", "imp", "lemure"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "giant".to_string(),
            vec!["hill giant", "stone giant", "frost giant", "fire giant", "cloud giant", "storm giant", "ogre", "troll", "ettin"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "elemental".to_string(),
            vec!["fire elemental", "water elemental", "earth elemental", "air elemental", "mephit", "salamander", "xorn", "galeb duhr"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "aberration".to_string(),
            vec!["beholder", "mind flayer", "illithid", "aboleth", "gibbering mouther", "otyugh", "intellect devourer", "slaad", "chuul"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "goblinoid".to_string(),
            vec!["goblin", "hobgoblin", "bugbear", "goblin boss", "nilbog"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        // Add related monster terms
        self.related.insert(
            "beholder".to_string(),
            vec!["eye ray", "antimagic cone", "aberration", "eye tyrant", "spectator", "death tyrant"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.related.insert(
            "mind flayer".to_string(),
            vec!["illithid", "elder brain", "ceremorphosis", "psionics", "tentacles", "intellect devourer"]
                .into_iter()
                .map(String::from)
                .collect(),
        );
    }

    /// Load spell vocabulary
    fn load_spell_vocabulary(&mut self) {
        // Common spell categories
        self.synonyms.insert(
            "fireball".to_string(),
            vec!["fire spell", "evocation", "aoe damage", "explosion"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "healing word".to_string(),
            vec!["heal", "restoration", "bonus action heal", "healing"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "cure wounds".to_string(),
            vec!["heal", "touch healing", "restoration", "healing spell"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        // Spell types
        self.synonyms.insert(
            "buff spell".to_string(),
            vec!["enhancement", "blessing", "empowerment", "boon"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "debuff spell".to_string(),
            vec!["curse", "hex", "bane", "weakening", "hindering"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "aoe spell".to_string(),
            vec!["area spell", "mass spell", "cone", "sphere", "cube", "cylinder", "line", "blast"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        // Related spell terms
        self.related.insert(
            "counterspell".to_string(),
            vec!["dispel", "negate", "reaction", "anti-magic", "spell duel"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.related.insert(
            "teleport".to_string(),
            vec!["dimension door", "misty step", "plane shift", "transport", "travel", "blink"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.related.insert(
            "summon".to_string(),
            vec!["conjure", "call", "summon creature", "animate", "create"]
                .into_iter()
                .map(String::from)
                .collect(),
        );
    }

    /// Load equipment vocabulary
    fn load_equipment_vocabulary(&mut self) {
        // Weapon categories
        self.synonyms.insert(
            "sword".to_string(),
            vec!["blade", "longsword", "shortsword", "greatsword", "rapier", "scimitar", "cutlass", "sabre", "katana", "broadsword"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "axe".to_string(),
            vec!["battleaxe", "greataxe", "handaxe", "hatchet", "tomahawk", "war axe"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "polearm".to_string(),
            vec!["glaive", "halberd", "pike", "lance", "spear", "javelin", "trident", "quarterstaff"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "bow".to_string(),
            vec!["longbow", "shortbow", "crossbow", "light crossbow", "heavy crossbow", "hand crossbow", "ranged weapon"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        // Armor categories
        self.synonyms.insert(
            "armor".to_string(),
            vec!["armour", "protection", "mail", "plate", "leather armor", "chain mail", "scale mail", "breastplate", "half plate", "full plate"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "shield".to_string(),
            vec!["buckler", "tower shield", "protection", "defense", "block"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        // Magic items
        self.synonyms.insert(
            "potion".to_string(),
            vec!["elixir", "draught", "philter", "brew", "tonic", "vial", "bottle"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "scroll".to_string(),
            vec!["spell scroll", "magic scroll", "parchment", "written spell"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "wand".to_string(),
            vec!["magic wand", "arcane focus", "implement", "rod"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.synonyms.insert(
            "staff".to_string(),
            vec!["magic staff", "arcane focus", "quarterstaff", "rod"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        // Related equipment terms
        self.related.insert(
            "magic weapon".to_string(),
            vec!["enchanted", "magical", "plus one", "+1", "+2", "+3", "legendary", "artifact"]
                .into_iter()
                .map(String::from)
                .collect(),
        );

        self.related.insert(
            "magic armor".to_string(),
            vec!["enchanted", "magical", "plus one", "+1", "+2", "+3", "legendary", "artifact"]
                .into_iter()
                .map(String::from)
                .collect(),
        );
    }

    /// Load common TTRPG-specific typos
    fn load_common_typos(&mut self) {
        let typos = [
            // Common misspellings
            ("strenght", "strength"),
            ("dexerity", "dexterity"),
            ("constituion", "constitution"),
            ("intellegence", "intelligence"),
            ("wisdon", "wisdom"),
            ("charisam", "charisma"),
            ("armour", "armor"),
            ("defence", "defense"),
            ("colour", "color"),
            ("favour", "favor"),
            ("behaviour", "behavior"),
            ("skillz", "skills"),
            ("atack", "attack"),
            ("damge", "damage"),
            ("helth", "health"),
            ("speel", "spell"),
            ("levle", "level"),
            ("monstr", "monster"),
            ("dunegon", "dungeon"),
            ("inititive", "initiative"),
            ("proficency", "proficiency"),
            ("resistence", "resistance"),
            ("vulnerabilty", "vulnerability"),
            ("imunity", "immunity"),
            ("concetration", "concentration"),
            ("ritaul", "ritual"),
            ("catrip", "cantrip"),
            ("paliden", "paladin"),
            ("barberian", "barbarian"),
            ("sourcerer", "sorcerer"),
            ("wizzard", "wizard"),
            ("theif", "thief"),
            ("rouge", "rogue"),
            ("druid", "druid"),
            ("cleirc", "cleric"),
            ("artifcer", "artificer"),
            ("warrok", "warlock"),
            ("dwraf", "dwarf"),
            ("gnoe", "gnome"),
            ("halflng", "halfling"),
            ("tiefeling", "tiefling"),
            ("dragonbon", "dragonborn"),
            ("aasimr", "aasimar"),
            ("gollath", "goliath"),
            ("beholder", "beholder"),
            ("mindflayer", "mind flayer"),
            ("abolth", "aboleth"),
            ("firebll", "fireball"),
            ("ligthning", "lightning"),
            ("thundr", "thunder"),
            ("necrtoic", "necrotic"),
            ("psycic", "psychic"),
            ("readiant", "radiant"),
        ];

        for (typo, correction) in typos {
            self.common_typos.insert(typo.to_string(), correction.to_string());
        }
    }

    /// Expand a query with TTRPG knowledge
    pub fn expand_query(&self, query: &str) -> QueryExpansionResult {
        let query_lower = query.to_lowercase();
        let words: Vec<&str> = query_lower.split_whitespace().collect();
        let mut expanded_terms: HashSet<String> = HashSet::new();
        let mut expansions: Vec<ExpansionInfo> = Vec::new();
        let mut hints: Vec<String> = Vec::new();

        // Add original terms
        for word in &words {
            expanded_terms.insert(word.to_string());
        }

        // Expand abbreviations
        for word in &words {
            if let Some(expansion) = self.abbreviations.get(*word) {
                expanded_terms.insert(expansion.clone());
                expansions.push(ExpansionInfo {
                    original: word.to_string(),
                    expanded_to: vec![expansion.clone()],
                    category: "abbreviation".to_string(),
                });
                hints.push(format!("{} = {}", word.to_uppercase(), expansion));
            }
        }

        // Expand dice notation
        for word in &words {
            if let Some(expansion) = self.dice.expand(word) {
                expanded_terms.insert(expansion.clone());
                expansions.push(ExpansionInfo {
                    original: word.to_string(),
                    expanded_to: vec![expansion.clone()],
                    category: "dice".to_string(),
                });
            }
        }

        // Expand synonyms
        let query_joined = words.join(" ");
        for (term, syns) in &self.synonyms {
            if query_joined.contains(term) || words.iter().any(|w| *w == term) {
                for syn in syns {
                    expanded_terms.insert(syn.clone());
                }
                expansions.push(ExpansionInfo {
                    original: term.clone(),
                    expanded_to: syns.clone(),
                    category: "synonym".to_string(),
                });
            }
        }

        // Check if synonyms are in query (reverse lookup)
        for (term, syns) in &self.synonyms {
            for syn in syns {
                if words.iter().any(|w| w == syn) {
                    expanded_terms.insert(term.clone());
                }
            }
        }

        // Check reverse abbreviations
        for (full, abbrs) in &self.reverse_abbreviations {
            if query_joined.contains(full) {
                for abbr in abbrs {
                    expanded_terms.insert(abbr.clone());
                }
            }
        }

        // Add related terms for broader search
        for word in &words {
            if let Some(related) = self.related.get(*word) {
                for rel in related.iter().take(3) { // Limit to top 3 related terms
                    expanded_terms.insert(rel.clone());
                }
            }
        }

        // Build expanded query
        let was_expanded = expanded_terms.len() > words.len();
        let expanded_query = if was_expanded {
            let extra: Vec<_> = expanded_terms
                .iter()
                .filter(|t| !words.contains(&t.as_str()))
                .cloned()
                .collect();
            if extra.is_empty() {
                query.to_string()
            } else {
                format!("{} OR {}", query, extra.join(" OR "))
            }
        } else {
            query.to_string()
        };

        QueryExpansionResult {
            original: query.to_string(),
            expanded_query,
            was_expanded,
            expansions,
            hints,
        }
    }

    /// Get suggestions for partial query (autocomplete)
    pub fn suggest(&self, partial: &str) -> Vec<String> {
        let partial_lower = partial.to_lowercase();
        let mut suggestions = Vec::new();

        // Suggest from abbreviations
        for (abbr, expansion) in &self.abbreviations {
            if abbr.starts_with(&partial_lower) {
                suggestions.push(format!("{} ({})", abbr.to_uppercase(), expansion));
            }
            if expansion.starts_with(&partial_lower) {
                suggestions.push(expansion.clone());
            }
        }

        // Suggest from synonym keys
        for term in self.synonyms.keys() {
            if term.starts_with(&partial_lower) {
                suggestions.push(term.clone());
            }
        }

        // Suggest from related term keys
        for term in self.related.keys() {
            if term.starts_with(&partial_lower) && !suggestions.contains(term) {
                suggestions.push(term.clone());
            }
        }

        // Sort and deduplicate
        suggestions.sort();
        suggestions.dedup();
        suggestions.truncate(10);
        suggestions
    }

    /// Get query completions based on context
    pub fn get_completions(&self, partial: &str, context: Option<&str>) -> Vec<String> {
        let partial_lower = partial.to_lowercase();
        let mut completions = Vec::new();

        // Context-aware completions
        if let Some(ctx) = context {
            let ctx_lower = ctx.to_lowercase();

            // If context mentions combat, suggest combat-related terms
            if ctx_lower.contains("combat") || ctx_lower.contains("attack") || ctx_lower.contains("damage") {
                for term in ["attack roll", "damage roll", "hit", "miss", "critical hit", "armor class", "saving throw"] {
                    if term.starts_with(&partial_lower) {
                        completions.push(term.to_string());
                    }
                }
            }

            // If context mentions spells, suggest spell-related terms
            if ctx_lower.contains("spell") || ctx_lower.contains("magic") || ctx_lower.contains("cast") {
                for term in ["spell slot", "concentration", "ritual", "cantrip", "spell level", "spell save dc", "spell attack"] {
                    if term.starts_with(&partial_lower) {
                        completions.push(term.to_string());
                    }
                }
            }

            // If context mentions character, suggest character-related terms
            if ctx_lower.contains("character") || ctx_lower.contains("class") || ctx_lower.contains("level") {
                for term in ["ability score", "proficiency bonus", "hit dice", "class feature", "background", "alignment"] {
                    if term.starts_with(&partial_lower) {
                        completions.push(term.to_string());
                    }
                }
            }
        }

        // Add general suggestions
        completions.extend(self.suggest(&partial_lower));

        // Deduplicate and limit
        completions.sort();
        completions.dedup();
        completions.truncate(10);
        completions
    }

    /// Check if a term is a known TTRPG term
    pub fn is_ttrpg_term(&self, term: &str) -> bool {
        let term_lower = term.to_lowercase();
        self.abbreviations.contains_key(&term_lower)
            || self.synonyms.contains_key(&term_lower)
            || self.synonyms.values().any(|syns| syns.contains(&term_lower))
            || self.related.contains_key(&term_lower)
            || self.dice.is_dice_notation(&term_lower)
    }

    /// Get the expansion for an abbreviation
    pub fn expand_abbreviation(&self, abbr: &str) -> Option<&String> {
        self.abbreviations.get(&abbr.to_lowercase())
    }

    /// Get synonyms for a term
    pub fn get_synonyms(&self, term: &str) -> Option<&Vec<String>> {
        self.synonyms.get(&term.to_lowercase())
    }

    /// Get related terms
    pub fn get_related(&self, term: &str) -> Option<&Vec<String>> {
        self.related.get(&term.to_lowercase())
    }

    /// Get system-specific terms
    pub fn get_system_terms(&self, system: &str, category: &str) -> Option<&Vec<String>> {
        self.system_terms
            .get(&system.to_lowercase())
            .and_then(|sys| sys.get(&category.to_lowercase()))
    }

    /// Get common typo corrections
    pub fn get_typo_correction(&self, word: &str) -> Option<&String> {
        self.common_typos.get(&word.to_lowercase())
    }

    /// Get all known abbreviations
    pub fn all_abbreviations(&self) -> &HashMap<String, String> {
        &self.abbreviations
    }

    /// Get clarification prompts for ambiguous queries
    pub fn get_clarification(&self, query: &str) -> Option<ClarificationPrompt> {
        let query_lower = query.to_lowercase();
        let words: Vec<&str> = query_lower.split_whitespace().collect();

        // Check for ambiguous abbreviations
        for word in &words {
            // "int" could be intelligence or integer
            if *word == "int" {
                return Some(ClarificationPrompt {
                    original: word.to_string(),
                    question: "Did you mean 'intelligence' (the ability score)?".to_string(),
                    options: vec![
                        "intelligence (ability score)".to_string(),
                        "integer (number)".to_string(),
                    ],
                });
            }

            // "con" could be constitution or con artist
            if *word == "con" {
                return Some(ClarificationPrompt {
                    original: word.to_string(),
                    question: "Did you mean 'constitution' (the ability score)?".to_string(),
                    options: vec![
                        "constitution (ability score)".to_string(),
                        "con artist".to_string(),
                        "convention".to_string(),
                    ],
                });
            }

            // "dr" could be damage reduction or doctor
            if *word == "dr" {
                return Some(ClarificationPrompt {
                    original: word.to_string(),
                    question: "Did you mean 'damage reduction'?".to_string(),
                    options: vec![
                        "damage reduction".to_string(),
                        "doctor".to_string(),
                    ],
                });
            }

            // "sr" could be spell resistance or senior
            if *word == "sr" {
                return Some(ClarificationPrompt {
                    original: word.to_string(),
                    question: "Did you mean 'spell resistance'?".to_string(),
                    options: vec![
                        "spell resistance".to_string(),
                        "senior".to_string(),
                    ],
                });
            }
        }

        // Check for ambiguous class names
        if words.contains(&"ranger") && words.contains(&"park") {
            return Some(ClarificationPrompt {
                original: "ranger".to_string(),
                question: "Are you looking for the D&D class or information about park rangers?".to_string(),
                options: vec![
                    "Ranger class (D&D/Pathfinder)".to_string(),
                    "Park ranger (occupation)".to_string(),
                ],
            });
        }

        None
    }
}

impl Default for TTRPGSynonyms {
    fn default() -> Self {
        Self::new()
    }
}

/// Clarification prompt for ambiguous queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationPrompt {
    /// Original ambiguous term
    pub original: String,
    /// Question to ask the user
    pub question: String,
    /// Possible options
    pub options: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abbreviation_expansion() {
        let synonyms = TTRPGSynonyms::new();
        assert_eq!(
            synonyms.expand_abbreviation("hp"),
            Some(&"hit points".to_string())
        );
        assert_eq!(
            synonyms.expand_abbreviation("AC"),
            Some(&"armor class".to_string())
        );
        assert_eq!(
            synonyms.expand_abbreviation("str"),
            Some(&"strength".to_string())
        );
        assert_eq!(
            synonyms.expand_abbreviation("dex"),
            Some(&"dexterity".to_string())
        );
    }

    #[test]
    fn test_query_expansion() {
        let synonyms = TTRPGSynonyms::new();
        let result = synonyms.expand_query("how much hp does a goblin have");

        assert!(result.was_expanded);
        assert!(result.expanded_query.contains("hit points"));
        assert!(!result.hints.is_empty());
    }

    #[test]
    fn test_synonym_lookup() {
        let synonyms = TTRPGSynonyms::new();
        let syns = synonyms.get_synonyms("hit points");

        assert!(syns.is_some());
        let syns = syns.unwrap();
        assert!(syns.contains(&"hp".to_string()));
        assert!(syns.contains(&"health".to_string()));
    }

    #[test]
    fn test_system_terms() {
        let synonyms = TTRPGSynonyms::new();
        let classes = synonyms.get_system_terms("dnd", "classes");

        assert!(classes.is_some());
        let classes = classes.unwrap();
        assert!(classes.contains(&"fighter".to_string()));
        assert!(classes.contains(&"wizard".to_string()));
        assert!(classes.contains(&"barbarian".to_string()));
    }

    #[test]
    fn test_suggestions() {
        let synonyms = TTRPGSynonyms::new();
        let suggestions = synonyms.suggest("hp");

        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains("hit points")));
    }

    #[test]
    fn test_is_ttrpg_term() {
        let synonyms = TTRPGSynonyms::new();

        assert!(synonyms.is_ttrpg_term("hp"));
        assert!(synonyms.is_ttrpg_term("hit points"));
        assert!(synonyms.is_ttrpg_term("armor class"));
        assert!(synonyms.is_ttrpg_term("d20"));
        assert!(!synonyms.is_ttrpg_term("randomword123"));
    }

    #[test]
    fn test_dice_notation() {
        let dice = DiceNotation::new();

        assert!(dice.is_dice_notation("d20"));
        assert!(dice.is_dice_notation("2d6"));
        assert_eq!(dice.expand("d20"), Some("twenty-sided die".to_string()));
    }

    #[test]
    fn test_typo_correction() {
        let synonyms = TTRPGSynonyms::new();

        assert_eq!(
            synonyms.get_typo_correction("strenght"),
            Some(&"strength".to_string())
        );
        assert_eq!(
            synonyms.get_typo_correction("rouge"),
            Some(&"rogue".to_string())
        );
    }

    #[test]
    fn test_clarification() {
        let synonyms = TTRPGSynonyms::new();

        let clarification = synonyms.get_clarification("what is int");
        assert!(clarification.is_some());
        let clarification = clarification.unwrap();
        assert!(clarification.options.len() >= 2);
    }

    #[test]
    fn test_completions() {
        let synonyms = TTRPGSynonyms::new();

        // Context-aware completions
        let completions = synonyms.get_completions("att", Some("combat encounter"));
        assert!(completions.iter().any(|c| c.contains("attack")));
    }

    #[test]
    fn test_monster_vocabulary() {
        let synonyms = TTRPGSynonyms::new();

        let undead = synonyms.get_synonyms("undead");
        assert!(undead.is_some());
        let undead = undead.unwrap();
        assert!(undead.contains(&"zombie".to_string()));
        assert!(undead.contains(&"skeleton".to_string()));
        assert!(undead.contains(&"lich".to_string()));
    }

    #[test]
    fn test_equipment_vocabulary() {
        let synonyms = TTRPGSynonyms::new();

        let sword = synonyms.get_synonyms("sword");
        assert!(sword.is_some());
        let sword = sword.unwrap();
        assert!(sword.contains(&"longsword".to_string()));
        assert!(sword.contains(&"rapier".to_string()));
    }

    #[test]
    fn test_pathfinder_terms() {
        let synonyms = TTRPGSynonyms::new();

        let ancestries = synonyms.get_system_terms("pf2e", "ancestries");
        assert!(ancestries.is_some());
        let ancestries = ancestries.unwrap();
        assert!(ancestries.contains(&"leshy".to_string()));
        assert!(ancestries.contains(&"catfolk".to_string()));
    }
}
