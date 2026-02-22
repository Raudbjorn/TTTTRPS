//! Query Expansion Module
//!
//! Expands search queries with synonyms, abbreviations, and related terms
//! to improve TTRPG search recall.

use std::collections::{HashMap, HashSet};

// ============================================================================
// Constants - Abbreviation Expansions
// ============================================================================

/// Common TTRPG abbreviations and their expansions
const ABBREVIATIONS: &[(&str, &[&str])] = &[
    // Core mechanics
    ("ac", &["armor class", "ac"]),
    ("hp", &["hit points", "hp", "health", "hit point"]),
    ("thp", &["temporary hit points", "temp hp", "thp"]),
    ("dm", &["dungeon master", "dm", "game master", "gm", "referee"]),
    ("gm", &["game master", "gm", "dungeon master", "dm"]),
    ("pc", &["player character", "pc", "character"]),
    ("npc", &["non-player character", "npc", "non player character"]),
    ("xp", &["experience points", "xp", "exp", "experience"]),
    ("cr", &["challenge rating", "cr"]),
    ("dc", &["difficulty class", "dc"]),
    ("aoe", &["area of effect", "aoe"]),
    ("dpr", &["damage per round", "dpr"]),
    ("tpk", &["total party kill", "tpk"]),

    // Ability scores
    ("str", &["strength", "str"]),
    ("dex", &["dexterity", "dex"]),
    ("con", &["constitution", "con"]),
    ("int", &["intelligence", "int"]),
    ("wis", &["wisdom", "wis"]),
    ("cha", &["charisma", "cha"]),

    // Saving throws
    ("save", &["saving throw", "save", "saves"]),
    ("fort", &["fortitude", "fort", "fortitude save"]),
    ("ref", &["reflex", "ref", "reflex save"]),
    ("will", &["will", "will save", "willpower"]),

    // Actions
    ("ba", &["bonus action", "ba"]),
    ("oa", &["opportunity attack", "oa", "attack of opportunity", "aoo"]),
    ("aoo", &["attack of opportunity", "aoo", "opportunity attack", "oa"]),

    // Classes (common abbreviations)
    ("barb", &["barbarian", "barb"]),
    ("sorc", &["sorcerer", "sorc"]),
    ("wiz", &["wizard", "wiz"]),
    ("rog", &["rogue", "rog"]),
    ("pally", &["paladin", "pally", "pala"]),
    ("lock", &["warlock", "lock"]),

    // Source books (D&D 5e)
    ("phb", &["player's handbook", "phb", "players handbook"]),
    ("dmg", &["dungeon master's guide", "dmg", "dungeon masters guide"]),
    ("mm", &["monster manual", "mm"]),
    ("xge", &["xanathar's guide to everything", "xge", "xanathars"]),
    ("tce", &["tasha's cauldron of everything", "tce", "tashas"]),
    ("vgm", &["volo's guide to monsters", "vgm", "volos"]),
    ("mtof", &["mordenkainen's tome of foes", "mtof", "mordenkainens"]),
    ("scag", &["sword coast adventurer's guide", "scag"]),
    ("ftd", &["fizban's treasury of dragons", "ftd", "fizbans"]),

    // Source books (Pathfinder 2e)
    ("crb", &["core rulebook", "crb"]),
    ("apg", &["advanced player's guide", "apg"]),
    ("gmg", &["gamemastery guide", "gmg"]),
    ("b1", &["bestiary", "b1", "bestiary 1"]),
    ("b2", &["bestiary 2", "b2"]),
    ("b3", &["bestiary 3", "b3"]),
    ("sog", &["secrets of magic", "sog", "som"]),
    ("g&g", &["guns and gears", "g&g"]),

    // Dice
    ("d20", &["d20", "twenty-sided", "icosahedron"]),
    ("d12", &["d12", "twelve-sided"]),
    ("d10", &["d10", "ten-sided"]),
    ("d8", &["d8", "eight-sided"]),
    ("d6", &["d6", "six-sided"]),
    ("d4", &["d4", "four-sided"]),
    ("d100", &["d100", "percentile", "d%"]),
];

// ============================================================================
// Constants - Synonym Groups
// ============================================================================

/// Synonym groups for common TTRPG terms
const SYNONYM_GROUPS: &[&[&str]] = &[
    // Content types
    &["spell", "magic", "incantation", "cantrip", "enchantment"],
    &["monster", "creature", "enemy", "foe", "beast", "mob"],
    &["item", "equipment", "gear", "treasure", "loot"],
    &["weapon", "armament", "arms"],
    &["armor", "armour", "protection", "mail"],

    // Actions
    &["damage", "harm", "hurt", "wound", "injure"],
    &["heal", "cure", "restore", "recover", "mend"],
    &["attack", "strike", "hit", "assault", "swing"],
    &["defend", "protect", "guard", "shield", "block"],
    &["move", "movement", "travel", "walk", "run"],
    &["cast", "invoke", "channel", "conjure"],

    // Classes
    &["wizard", "mage", "sorcerer", "spellcaster", "arcanist"],
    &["warrior", "fighter", "soldier", "combatant", "martial"],
    &["rogue", "thief", "assassin", "scoundrel", "sneak"],
    &["cleric", "priest", "healer", "divine caster"],
    &["ranger", "hunter", "scout", "tracker"],
    &["druid", "nature caster", "shapeshifter"],
    &["monk", "martial artist", "ki user"],
    &["paladin", "holy warrior", "champion"],
    &["bard", "minstrel", "performer", "skald"],
    &["warlock", "pact magic", "patron"],

    // Creature categories
    &["undead", "zombie", "skeleton", "ghost", "vampire", "lich"],
    &["demon", "devil", "fiend", "daemon"],
    &["angel", "celestial", "archon"],
    &["dragon", "wyrm", "drake"],
    &["goblin", "goblinoid", "hobgoblin", "bugbear"],
    &["orc", "orcish", "half-orc"],
    &["elf", "elven", "elvish", "half-elf"],
    &["dwarf", "dwarven", "dwarfish"],

    // Mechanics
    &["bonus", "modifier", "mod", "adjustment"],
    &["level", "tier", "rank"],
    &["class", "archetype", "subclass"],
    &["race", "ancestry", "heritage", "lineage"],
    &["feat", "feature", "ability", "talent"],
    &["skill", "proficiency", "training"],
    &["check", "roll", "test"],

    // Combat
    &["initiative", "turn order", "combat order"],
    &["round", "turn", "combat round"],
    &["melee", "close combat", "hand-to-hand"],
    &["ranged", "distance", "projectile"],
    &["critical", "crit", "critical hit"],

    // Status
    &["death", "dying", "dead", "kill", "slay"],
    &["rest", "recover", "recuperate"],
    &["short rest", "breather", "quick rest"],
    &["long rest", "sleep", "overnight rest", "full rest"],
];

// ============================================================================
// Constants - TTRPG Vocabulary (for fuzzy matching)
// ============================================================================

/// Common TTRPG terms for spell correction and fuzzy matching
const TTRPG_VOCABULARY: &[&str] = &[
    // Spells (D&D 5e common)
    "fireball", "magic missile", "cure wounds", "healing word",
    "shield", "mage armor", "detect magic", "identify",
    "counterspell", "dispel magic", "fly", "haste",
    "invisibility", "polymorph", "wish", "meteor swarm",
    "eldritch blast", "guiding bolt", "spiritual weapon",
    "thunderwave", "burning hands", "chromatic orb",
    "lightning bolt", "ice storm", "wall of fire",
    "animate dead", "raise dead", "resurrection",
    "teleport", "dimension door", "misty step",
    "bless", "bane", "hex", "hunter's mark",

    // Mechanics
    "armor class", "hit points", "saving throw", "ability check",
    "advantage", "disadvantage", "critical hit", "initiative",
    "spell slot", "cantrip", "ritual", "concentration",
    "proficiency", "expertise", "inspiration",
    "attunement", "attuned", "magical",
    "components", "somatic", "verbal", "material",
    "spell save", "spell attack", "attack roll",
    "damage roll", "ability modifier", "proficiency bonus",

    // Actions
    "action", "bonus action", "reaction", "movement",
    "attack", "cast", "dash", "disengage", "dodge",
    "help", "hide", "ready", "search", "use object",
    "grapple", "shove", "opportunity attack",

    // Conditions
    "blinded", "charmed", "deafened", "frightened",
    "grappled", "incapacitated", "invisible", "paralyzed",
    "petrified", "poisoned", "prone", "restrained",
    "stunned", "unconscious", "exhaustion",

    // Damage types
    "acid", "bludgeoning", "cold", "fire", "force",
    "lightning", "necrotic", "piercing", "poison",
    "psychic", "radiant", "slashing", "thunder",

    // Creature types
    "aberration", "beast", "celestial", "construct",
    "dragon", "elemental", "fey", "fiend", "giant",
    "humanoid", "monstrosity", "ooze", "plant", "undead",

    // Common monsters
    "goblin", "orc", "kobold", "skeleton", "zombie",
    "wolf", "bear", "spider", "giant rat",
    "ogre", "troll", "giant", "dragon",
    "beholder", "mind flayer", "lich", "vampire",
    "demon", "devil", "elemental",

    // Equipment
    "longsword", "shortsword", "greatsword", "rapier",
    "dagger", "handaxe", "battleaxe", "greataxe",
    "longbow", "shortbow", "crossbow", "hand crossbow",
    "staff", "wand", "rod", "orb",
    "shield", "armor", "plate", "chain mail", "leather",
    "potion", "scroll", "ring", "amulet", "cloak",

    // Classes
    "barbarian", "bard", "cleric", "druid", "fighter",
    "monk", "paladin", "ranger", "rogue", "sorcerer",
    "warlock", "wizard", "artificer",

    // Spell schools
    "abjuration", "conjuration", "divination", "enchantment",
    "evocation", "illusion", "necromancy", "transmutation",
];

// ============================================================================
// Constants - Stop Words
// ============================================================================

/// Stop words to filter from keyword search
const STOP_WORDS: &[&str] = &[
    // Articles
    "a", "an", "the",
    // Pronouns
    "i", "you", "he", "she", "it", "we", "they",
    "me", "him", "her", "us", "them",
    "my", "your", "his", "its", "our", "their",
    "this", "that", "these", "those",
    "who", "whom", "whose", "which", "what",
    // Prepositions
    "in", "on", "at", "to", "for", "of", "with",
    "by", "from", "as", "into", "through", "during",
    "before", "after", "above", "below", "between",
    "under", "over", "out", "up", "down", "off",
    "about", "against", "among", "around",
    // Conjunctions
    "and", "or", "but", "nor", "so", "yet",
    "because", "although", "while", "if", "unless",
    // Verbs (common)
    "is", "are", "was", "were", "be", "been", "being",
    "have", "has", "had", "having",
    "do", "does", "did", "doing",
    "will", "would", "could", "should", "may", "might", "must",
    "can", "shall",
    // Adverbs
    "not", "no", "very", "just", "only", "also",
    "too", "more", "most", "less", "least",
    "now", "then", "here", "there", "when", "where",
    "why", "how", "all", "each", "every", "both",
    "few", "many", "some", "any", "other", "such",
    "own", "same",
];

// ============================================================================
// Query Expander
// ============================================================================

/// Expands search queries with synonyms, abbreviations, and related terms.
#[derive(Debug, Clone)]
pub struct QueryExpander {
    /// Abbreviation to expansions map
    abbreviations: HashMap<String, Vec<String>>,
    /// Term to synonym group map
    synonyms: HashMap<String, Vec<String>>,
    /// TTRPG vocabulary set for fuzzy matching
    vocabulary: HashSet<String>,
    /// Stop words set
    stop_words: HashSet<String>,
}

impl Default for QueryExpander {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryExpander {
    /// Create a new query expander with default vocabulary.
    pub fn new() -> Self {
        let mut expander = Self {
            abbreviations: HashMap::new(),
            synonyms: HashMap::new(),
            vocabulary: HashSet::new(),
            stop_words: HashSet::new(),
        };

        // Load abbreviations
        for (abbr, expansions) in ABBREVIATIONS {
            let exp_vec: Vec<String> = expansions.iter().map(|s| s.to_string()).collect();
            expander.abbreviations.insert(abbr.to_string(), exp_vec);
        }

        // Load synonym groups
        for group in SYNONYM_GROUPS {
            let group_vec: Vec<String> = group.iter().map(|s| s.to_string()).collect();
            for term in group.iter() {
                expander.synonyms.insert(term.to_string(), group_vec.clone());
            }
        }

        // Load vocabulary
        for term in TTRPG_VOCABULARY {
            expander.vocabulary.insert(term.to_string());
        }

        // Load stop words
        for word in STOP_WORDS {
            expander.stop_words.insert(word.to_string());
        }

        expander
    }

    /// Expand a query with synonyms and abbreviations.
    ///
    /// Returns the expanded query terms (original + expansions).
    pub fn expand_query(&self, query: &str) -> Vec<String> {
        let mut expanded = Vec::new();
        let mut seen = HashSet::new();

        for word in query.split_whitespace() {
            let word_lower = word.to_lowercase();

            // Skip stop words in expansion
            if self.stop_words.contains(&word_lower) {
                continue;
            }

            // Add original term
            if seen.insert(word_lower.clone()) {
                expanded.push(word_lower.clone());
            }

            // Check for abbreviation expansions
            if let Some(expansions) = self.abbreviations.get(&word_lower) {
                for exp in expansions {
                    if seen.insert(exp.clone()) {
                        expanded.push(exp.clone());
                    }
                }
            }

            // Check for synonyms
            if let Some(synonyms) = self.synonyms.get(&word_lower) {
                for syn in synonyms {
                    if seen.insert(syn.clone()) {
                        expanded.push(syn.clone());
                    }
                }
            }
        }

        expanded
    }

    /// Expand abbreviations only (without synonyms).
    pub fn expand_abbreviations(&self, query: &str) -> String {
        let mut result = Vec::new();

        for word in query.split_whitespace() {
            let word_lower = word.to_lowercase();

            if let Some(expansions) = self.abbreviations.get(&word_lower) {
                // Use the first expansion (usually the full form)
                if let Some(first) = expansions.first() {
                    result.push(first.clone());
                } else {
                    result.push(word_lower);
                }
            } else {
                result.push(word_lower);
            }
        }

        result.join(" ")
    }

    /// Get synonyms for a term.
    pub fn get_synonyms(&self, term: &str) -> Option<&Vec<String>> {
        self.synonyms.get(&term.to_lowercase())
    }

    /// Get abbreviation expansions.
    pub fn get_expansions(&self, abbr: &str) -> Option<&Vec<String>> {
        self.abbreviations.get(&abbr.to_lowercase())
    }

    /// Check if a term is in the TTRPG vocabulary.
    pub fn is_known_term(&self, term: &str) -> bool {
        self.vocabulary.contains(&term.to_lowercase())
    }

    /// Check if a word is a stop word.
    pub fn is_stop_word(&self, word: &str) -> bool {
        self.stop_words.contains(&word.to_lowercase())
    }

    /// Filter stop words from a list of tokens.
    pub fn filter_stop_words(&self, tokens: &[String]) -> Vec<String> {
        tokens
            .iter()
            .filter(|t| !self.is_stop_word(t) && t.len() > 2)
            .cloned()
            .collect()
    }

    /// Find the closest matching term in vocabulary using edit distance.
    ///
    /// Returns `Some((term, distance))` if a match is found within threshold.
    pub fn fuzzy_match(&self, query: &str, max_distance: usize) -> Option<(String, usize)> {
        let query_lower = query.to_lowercase();
        let mut best_match: Option<(String, usize)> = None;

        for term in &self.vocabulary {
            let distance = Self::levenshtein(&query_lower, term);
            if distance <= max_distance {
                match &best_match {
                    None => best_match = Some((term.clone(), distance)),
                    Some((_, best_dist)) if distance < *best_dist => {
                        best_match = Some((term.clone(), distance));
                    }
                    _ => {}
                }
            }
        }

        best_match
    }

    /// Calculate Levenshtein edit distance between two strings.
    fn levenshtein(a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let a_len = a_chars.len();
        let b_len = b_chars.len();

        if a_len == 0 {
            return b_len;
        }
        if b_len == 0 {
            return a_len;
        }

        let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

        for i in 0..=a_len {
            matrix[i][0] = i;
        }
        for j in 0..=b_len {
            matrix[0][j] = j;
        }

        for i in 1..=a_len {
            for j in 1..=b_len {
                let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[a_len][b_len]
    }

    /// Suggest corrections for a potentially misspelled term.
    pub fn suggest_correction(&self, term: &str) -> Option<String> {
        // Only try to correct if term is not already known
        if self.is_known_term(term) {
            return None;
        }

        // Allow more distance for longer terms
        let max_distance = match term.len() {
            0..=3 => 1,
            4..=6 => 2,
            _ => 3,
        };

        self.fuzzy_match(term, max_distance).map(|(matched, _)| matched)
    }

    /// Get vocabulary size.
    pub fn vocabulary_size(&self) -> usize {
        self.vocabulary.len()
    }

    /// Get number of abbreviations.
    pub fn abbreviation_count(&self) -> usize {
        self.abbreviations.len()
    }

    /// Get number of synonym groups.
    pub fn synonym_group_count(&self) -> usize {
        SYNONYM_GROUPS.len()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_abbreviations() {
        let expander = QueryExpander::new();

        assert_eq!(
            expander.expand_abbreviations("ac hp"),
            "armor class hit points"
        );
        assert_eq!(
            expander.expand_abbreviations("str dex con"),
            "strength dexterity constitution"
        );
    }

    #[test]
    fn test_expand_query() {
        let expander = QueryExpander::new();

        let expanded = expander.expand_query("fireball damage");
        assert!(expanded.contains(&"fireball".to_string()));
        assert!(expanded.contains(&"damage".to_string()));
        // Should include synonyms for damage
        assert!(expanded.contains(&"harm".to_string()) || expanded.contains(&"hurt".to_string()));
    }

    #[test]
    fn test_get_synonyms() {
        let expander = QueryExpander::new();

        let synonyms = expander.get_synonyms("monster").unwrap();
        assert!(synonyms.contains(&"creature".to_string()));
        assert!(synonyms.contains(&"enemy".to_string()));
    }

    #[test]
    fn test_get_expansions() {
        let expander = QueryExpander::new();

        let expansions = expander.get_expansions("phb").unwrap();
        assert!(expansions.iter().any(|e| e.contains("handbook")));
    }

    #[test]
    fn test_is_known_term() {
        let expander = QueryExpander::new();

        assert!(expander.is_known_term("fireball"));
        assert!(expander.is_known_term("FIREBALL")); // Case-insensitive
        assert!(expander.is_known_term("armor class"));
        assert!(!expander.is_known_term("xyznotaword"));
    }

    #[test]
    fn test_is_stop_word() {
        let expander = QueryExpander::new();

        assert!(expander.is_stop_word("the"));
        assert!(expander.is_stop_word("and"));
        assert!(expander.is_stop_word("is"));
        assert!(!expander.is_stop_word("fireball"));
    }

    #[test]
    fn test_filter_stop_words() {
        let expander = QueryExpander::new();

        let tokens: Vec<String> = vec![
            "the".to_string(),
            "fireball".to_string(),
            "is".to_string(),
            "a".to_string(),
            "spell".to_string(),
        ];

        let filtered = expander.filter_stop_words(&tokens);
        assert_eq!(filtered, vec!["fireball", "spell"]);
    }

    #[test]
    fn test_fuzzy_match() {
        let expander = QueryExpander::new();

        // Misspelled "fireball"
        let result = expander.fuzzy_match("firebll", 2);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "fireball");

        // Too far from any term
        let result = expander.fuzzy_match("xyzabc", 2);
        assert!(result.is_none());
    }

    #[test]
    fn test_suggest_correction() {
        let expander = QueryExpander::new();

        // Misspelled terms
        assert_eq!(
            expander.suggest_correction("firebll"),
            Some("fireball".to_string())
        );
        assert_eq!(
            expander.suggest_correction("lightening"),
            Some("lightning".to_string())
        );

        // Already correct - no suggestion
        assert_eq!(expander.suggest_correction("fireball"), None);
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(QueryExpander::levenshtein("", ""), 0);
        assert_eq!(QueryExpander::levenshtein("abc", ""), 3);
        assert_eq!(QueryExpander::levenshtein("", "abc"), 3);
        assert_eq!(QueryExpander::levenshtein("abc", "abc"), 0);
        assert_eq!(QueryExpander::levenshtein("abc", "abd"), 1);
        assert_eq!(QueryExpander::levenshtein("kitten", "sitting"), 3);
    }

    #[test]
    fn test_case_insensitive() {
        let expander = QueryExpander::new();

        // Abbreviations should be case-insensitive
        assert!(expander.get_expansions("AC").is_some());
        assert!(expander.get_expansions("ac").is_some());
        assert!(expander.get_expansions("Ac").is_some());

        // Synonyms should be case-insensitive
        assert!(expander.get_synonyms("MONSTER").is_some());
    }

    #[test]
    fn test_source_book_abbreviations() {
        let expander = QueryExpander::new();

        let phb = expander.get_expansions("phb").unwrap();
        assert!(phb.iter().any(|e| e.contains("handbook")));

        let mm = expander.get_expansions("mm").unwrap();
        assert!(mm.iter().any(|e| e.contains("monster manual")));
    }

    #[test]
    fn test_dice_abbreviations() {
        let expander = QueryExpander::new();

        let d20 = expander.get_expansions("d20").unwrap();
        assert!(d20.contains(&"d20".to_string()));

        let d100 = expander.get_expansions("d100").unwrap();
        assert!(d100.iter().any(|e| e.contains("percentile")));
    }

    #[test]
    fn test_stats() {
        let expander = QueryExpander::new();

        assert!(expander.vocabulary_size() > 100);
        assert!(expander.abbreviation_count() > 30);
        assert!(expander.synonym_group_count() > 20);
    }
}
