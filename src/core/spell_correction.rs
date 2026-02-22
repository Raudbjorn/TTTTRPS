//! Spell Correction Module
//!
//! Provides TTRPG-aware spelling suggestions for search queries.
//! Includes comprehensive TTRPG vocabulary with high-frequency terms
//! prioritized for better correction accuracy.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ============================================================================
// Types
// ============================================================================

/// Spelling suggestion for a single word
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellingSuggestion {
    /// Original word
    pub original: String,
    /// Suggested correction
    pub suggestion: String,
    /// Edit distance (Levenshtein)
    pub distance: usize,
    /// Confidence (0.0 - 1.0)
    pub confidence: f64,
}

/// Correction result for a complete query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionResult {
    /// Original query
    pub original_query: String,
    /// Corrected query
    pub corrected_query: String,
    /// Individual word corrections
    pub corrections: Vec<SpellingSuggestion>,
    /// Whether any corrections were made
    pub has_corrections: bool,
}

/// Frequency tier for prioritizing corrections
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FrequencyTier {
    /// Core TTRPG terms (highest priority)
    Core = 1000,
    /// Common TTRPG terms
    Common = 500,
    /// Standard TTRPG vocabulary
    Standard = 200,
    /// Less common terms
    Uncommon = 100,
    /// General English words
    General = 50,
}

// ============================================================================
// Spell Corrector
// ============================================================================

/// TTRPG-aware spell corrector with comprehensive vocabulary
pub struct SpellCorrector {
    /// Dictionary of known words
    dictionary: HashSet<String>,
    /// Word frequency map (for ranking suggestions)
    word_frequencies: HashMap<String, u32>,
    /// Common TTRPG typos with direct corrections
    known_typos: HashMap<String, String>,
    /// Phonetic groups for soundex-like matching
    phonetic_groups: HashMap<String, Vec<String>>,
}

impl SpellCorrector {
    pub fn new() -> Self {
        let mut corrector = Self {
            dictionary: HashSet::new(),
            word_frequencies: HashMap::new(),
            known_typos: HashMap::new(),
            phonetic_groups: HashMap::new(),
        };
        corrector.load_core_vocabulary();
        corrector.load_ttrpg_vocabulary();
        corrector.load_class_and_race_vocabulary();
        corrector.load_monster_vocabulary();
        corrector.load_spell_vocabulary();
        corrector.load_equipment_vocabulary();
        corrector.load_common_english();
        corrector.load_known_typos();
        corrector.build_phonetic_groups();
        corrector
    }

    /// Load core TTRPG terms (highest frequency)
    fn load_core_vocabulary(&mut self) {
        let core_terms = [
            // Core mechanics
            "ability", "action", "armor", "attack", "bonus", "cantrip", "character",
            "class", "combat", "concentration", "condition", "constitution", "critical",
            "damage", "dexterity", "difficulty", "dungeon", "encounter", "equipment",
            "experience", "feat", "feature", "grapple", "health", "hit", "initiative",
            "intelligence", "level", "modifier", "monster", "multiclass", "perception",
            "points", "proficiency", "race", "range", "reaction", "resistance", "rest",
            "ritual", "roll", "save", "saving", "skill", "speed", "spell", "spellcasting",
            "stealth", "strength", "target", "terrain", "throw", "trait", "turn",
            "vulnerability", "weapon", "wisdom", "charisma",
            // Common TTRPG abbreviations as words
            "hp", "ac", "dc", "str", "dex", "con", "int", "wis", "cha",
            "xp", "gp", "sp", "cp", "pp", "cr", "npc", "pc", "dm", "gm",
        ];

        for term in core_terms {
            self.add_word(term, FrequencyTier::Core as u32);
        }
    }

    /// Load TTRPG-specific vocabulary
    fn load_ttrpg_vocabulary(&mut self) {
        let ttrpg_terms = [
            // Combat terms
            "melee", "ranged", "finesse", "versatile", "two-handed", "light", "heavy",
            "thrown", "ammunition", "loading", "reach", "special", "silvered", "adamantine",
            "magical", "nonmagical", "bludgeoning", "piercing", "slashing", "acid", "cold",
            "fire", "force", "lightning", "necrotic", "poison", "psychic", "radiant", "thunder",
            // Spell terms
            "abjuration", "conjuration", "divination", "enchantment", "evocation",
            "illusion", "necromancy", "transmutation", "component", "somatic", "verbal",
            "material", "duration", "instantaneous", "upcast", "upcasting",
            // Conditions
            "blinded", "charmed", "deafened", "exhausted", "frightened", "grappled",
            "incapacitated", "invisible", "paralyzed", "petrified", "poisoned", "prone",
            "restrained", "stunned", "unconscious",
            // Game terms
            "campaign", "adventure", "quest", "dungeon", "master", "player", "session",
            "backstory", "alignment", "lawful", "chaotic", "neutral", "good", "evil",
            "background", "inspiration", "advantage", "disadvantage", "legendary",
            "proficient", "expertise", "multiclassing", "subclass", "archetype",
        ];

        for term in ttrpg_terms {
            self.add_word(term, FrequencyTier::Common as u32);
        }
    }

    /// Load class and race vocabulary
    fn load_class_and_race_vocabulary(&mut self) {
        // Classes
        let classes = [
            "artificer", "barbarian", "bard", "cleric", "druid", "fighter", "monk",
            "paladin", "ranger", "rogue", "sorcerer", "warlock", "wizard",
        ];

        // Races
        let races = [
            "aasimar", "dragonborn", "dwarf", "elf", "gnome", "goliath", "halfling",
            "half-elf", "half-orc", "human", "tiefling", "orc", "goblin", "kobold",
            "tabaxi", "kenku", "firbolg", "genasi", "warforged", "changeling",
            "shifter", "kalashtar", "aarakocra", "triton", "tortle",
        ];

        for class in classes {
            self.add_word(class, FrequencyTier::Core as u32);
        }

        for race in races {
            self.add_word(race, FrequencyTier::Common as u32);
        }
    }

    /// Load monster vocabulary
    fn load_monster_vocabulary(&mut self) {
        let monsters = [
            // Common monsters
            "aboleth", "beholder", "bugbear", "demon", "devil", "dragon", "elemental",
            "fiend", "giant", "hobgoblin", "hydra", "lich", "mimic", "mind flayer",
            "ogre", "owlbear", "skeleton", "troll", "undead", "vampire", "werewolf",
            "wyvern", "zombie", "ghoul", "ghost", "specter", "wraith", "wight",
            "golem", "chimera", "basilisk", "cockatrice", "griffon", "hippogriff",
            "manticore", "medusa", "sphinx", "treant", "dryad", "satyr", "centaur",
            // Monster types
            "aberration", "beast", "celestial", "construct", "fey", "fiend",
            "giant", "humanoid", "monstrosity", "ooze", "plant",
        ];

        for monster in monsters {
            self.add_word(monster, FrequencyTier::Standard as u32);
        }
    }

    /// Load spell vocabulary
    fn load_spell_vocabulary(&mut self) {
        let spells = [
            // Popular spells
            "fireball", "lightning", "bolt", "magic", "missile", "shield", "healing",
            "word", "cure", "wounds", "dispel", "detect", "identify", "invisibility",
            "polymorph", "teleport", "resurrection", "dimension", "door", "counterspell",
            "eldritch", "blast", "hex", "hunter's", "mark", "smite", "guidance",
            "prestidigitation", "thaumaturgy", "druidcraft", "mage", "hand", "minor",
            "illusion", "sacred", "flame", "toll", "dead", "chill", "touch",
            "vicious", "mockery", "thunderwave", "burning", "hands", "chromatic",
            "orb", "ray", "frost", "poison", "spray", "sleep", "charm", "person",
            "hold", "suggestion", "dominate", "fear", "slow", "haste", "fly",
            "misty", "step", "thunder", "step", "blink", "banishment", "wall",
        ];

        for spell in spells {
            self.add_word(spell, FrequencyTier::Standard as u32);
        }
    }

    /// Load equipment vocabulary
    fn load_equipment_vocabulary(&mut self) {
        let equipment = [
            // Weapons
            "sword", "axe", "bow", "crossbow", "dagger", "mace", "staff", "wand",
            "longsword", "shortsword", "greatsword", "battleaxe", "greataxe", "handaxe",
            "longbow", "shortbow", "rapier", "scimitar", "flail", "morningstar",
            "warhammer", "maul", "glaive", "halberd", "pike", "lance", "spear",
            "javelin", "trident", "quarterstaff", "club", "greatclub", "sickle",
            "whip", "blowgun", "sling", "dart",
            // Armor
            "plate", "chain", "leather", "scale", "ring", "mail", "splint", "half",
            "padded", "hide", "studded", "breastplate", "shield", "buckler",
            // Items
            "potion", "scroll", "ring", "amulet", "cloak", "boots", "gauntlets",
            "helm", "helmet", "belt", "bracers", "gloves", "robes", "vestments",
            "bag", "holding", "portable", "hole", "rope", "torch", "lantern",
            "rations", "bedroll", "tent", "component", "pouch", "spellbook",
        ];

        for item in equipment {
            self.add_word(item, FrequencyTier::Standard as u32);
        }
    }

    /// Load common English words (lower priority)
    fn load_common_english(&mut self) {
        let common_words = [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
            "of", "with", "by", "from", "up", "about", "into", "through", "during",
            "before", "after", "above", "below", "between", "under", "again", "further",
            "then", "once", "here", "there", "when", "where", "why", "how", "all",
            "each", "few", "more", "most", "other", "some", "such", "no", "nor",
            "not", "only", "own", "same", "so", "than", "too", "very", "can", "will",
            "just", "should", "now", "does", "what", "which", "who", "this", "that",
            "these", "those", "am", "is", "are", "was", "were", "be", "been", "being",
            "have", "has", "had", "having", "do", "did", "doing", "would", "could",
            "might", "must", "shall", "make", "made", "get", "got", "take", "took",
            "give", "gave", "find", "found", "use", "used", "new", "first", "last",
            "long", "great", "little", "own", "old", "right", "big", "high", "different",
            "small", "large", "next", "early", "young", "important", "public", "bad",
            "same", "able", "know", "see", "think", "want", "tell", "call", "need",
            "feel", "become", "leave", "put", "mean", "keep", "let", "begin", "seem",
            "help", "show", "hear", "play", "run", "move", "live", "believe", "hold",
            "bring", "happen", "write", "provide", "sit", "stand", "lose", "pay",
            "meet", "include", "continue", "set", "learn", "change", "lead", "understand",
            "watch", "follow", "stop", "create", "speak", "read", "allow", "add",
            "spend", "grow", "open", "walk", "win", "offer", "remember", "love",
            "consider", "appear", "buy", "wait", "serve", "die", "send", "expect",
            "build", "stay", "fall", "cut", "reach", "kill", "remain",
        ];

        for word in common_words {
            self.add_word(word, FrequencyTier::General as u32);
        }
    }

    /// Load known TTRPG-specific typos
    fn load_known_typos(&mut self) {
        let typos = [
            // Ability scores
            ("strenght", "strength"),
            ("stength", "strength"),
            ("stregnth", "strength"),
            ("dexerity", "dexterity"),
            ("dexterit", "dexterity"),
            ("dexterty", "dexterity"),
            ("constituion", "constitution"),
            ("consitution", "constitution"),
            ("constituton", "constitution"),
            ("intellegence", "intelligence"),
            ("inteligence", "intelligence"),
            ("intelligance", "intelligence"),
            ("wisdon", "wisdom"),
            ("wisdome", "wisdom"),
            ("charisam", "charisma"),
            ("charima", "charisma"),
            ("charsima", "charisma"),

            // British/American spelling
            ("armour", "armor"),
            ("defence", "defense"),
            ("colour", "color"),
            ("favour", "favor"),
            ("behaviour", "behavior"),

            // Common misspellings
            ("skillz", "skills"),
            ("atack", "attack"),
            ("attck", "attack"),
            ("damge", "damage"),
            ("dmage", "damage"),
            ("helth", "health"),
            ("heatlh", "health"),
            ("speel", "spell"),
            ("seplll", "spell"),
            ("levle", "level"),
            ("lveel", "level"),
            ("monstr", "monster"),
            ("monstre", "monster"),
            ("dunegon", "dungeon"),
            ("dungoen", "dungeon"),
            ("inititive", "initiative"),
            ("iniative", "initiative"),
            ("initiativ", "initiative"),
            ("proficency", "proficiency"),
            ("proficiancy", "proficiency"),
            ("resistence", "resistance"),
            ("resistnce", "resistance"),
            ("vulnerabilty", "vulnerability"),
            ("vulnerablity", "vulnerability"),
            ("imunity", "immunity"),
            ("immuntiy", "immunity"),
            ("concetration", "concentration"),
            ("concentraton", "concentration"),
            ("ritaul", "ritual"),
            ("ritula", "ritual"),
            ("catrip", "cantrip"),
            ("cantirp", "cantrip"),

            // Classes
            ("paliden", "paladin"),
            ("palidin", "paladin"),
            ("barberian", "barbarian"),
            ("barbarin", "barbarian"),
            ("sourcerer", "sorcerer"),
            ("sorceror", "sorcerer"),
            ("wizzard", "wizard"),
            ("wizrd", "wizard"),
            ("theif", "thief"),
            ("rouge", "rogue"),
            ("rogu", "rogue"),
            ("cleirc", "cleric"),
            ("clerik", "cleric"),
            ("artifcer", "artificer"),
            ("artificier", "artificer"),
            ("warrok", "warlock"),
            ("warlok", "warlock"),
            ("fihgter", "fighter"),
            ("figher", "fighter"),
            ("monke", "monk"),
            ("barde", "bard"),

            // Races
            ("dwraf", "dwarf"),
            ("dwarve", "dwarf"),
            ("gnoe", "gnome"),
            ("gnomme", "gnome"),
            ("halflng", "halfling"),
            ("halflig", "halfling"),
            ("tiefeling", "tiefling"),
            ("teifling", "tiefling"),
            ("dragonbon", "dragonborn"),
            ("dragonbron", "dragonborn"),
            ("aasimr", "aasimar"),
            ("aasimor", "aasimar"),
            ("gollath", "goliath"),
            ("golaith", "goliath"),

            // Monsters
            ("mindflayer", "mind flayer"),
            ("abolth", "aboleth"),
            ("aboleht", "aboleth"),
            ("behoder", "beholder"),
            ("beholdr", "beholder"),
            ("illithd", "illithid"),
            ("illathid", "illithid"),

            // Spells and magic
            ("firebll", "fireball"),
            ("firebal", "fireball"),
            ("ligthning", "lightning"),
            ("ligtning", "lightning"),
            ("thundr", "thunder"),
            ("thurder", "thunder"),
            ("necrtoic", "necrotic"),
            ("necrtoic", "necrotic"),
            ("psycic", "psychic"),
            ("pschic", "psychic"),
            ("readiant", "radiant"),
            ("radaint", "radiant"),
            ("abjuraton", "abjuration"),
            ("conjuartion", "conjuration"),
            ("divinaton", "divination"),
            ("enchatment", "enchantment"),
            ("evocaton", "evocation"),
            ("ilusion", "illusion"),
            ("necromacy", "necromancy"),
            ("transmutaton", "transmutation"),

            // Equipment
            ("longsowrd", "longsword"),
            ("shortsowrd", "shortsword"),
            ("greatsowrd", "greatsword"),
            ("battleax", "battleaxe"),
            ("crossbo", "crossbow"),
            ("longbow", "longbow"),
            ("shortbo", "shortbow"),
        ];

        for (typo, correction) in typos {
            self.known_typos.insert(typo.to_string(), correction.to_string());
        }
    }

    /// Build phonetic groups for soundex-like matching
    fn build_phonetic_groups(&mut self) {
        // Group similar sounding words
        let groups = [
            ("strength", vec!["strenght", "stength", "stregnth"]),
            ("dexterity", vec!["dexerity", "dexterit", "dexterty"]),
            ("constitution", vec!["constituion", "consitution", "constituton"]),
            ("intelligence", vec!["intellegence", "inteligence", "intelligance"]),
            ("rogue", vec!["rouge", "rogu"]),
            ("paladin", vec!["paliden", "palidin"]),
            ("wizard", vec!["wizzard", "wizrd"]),
            ("sorcerer", vec!["sourcerer", "sorceror"]),
        ];

        for (correct, variants) in groups {
            self.phonetic_groups.insert(
                correct.to_string(),
                variants.into_iter().map(String::from).collect(),
            );
        }
    }

    /// Add a word to the dictionary
    fn add_word(&mut self, word: &str, frequency: u32) {
        let word_lower = word.to_lowercase();
        self.dictionary.insert(word_lower.clone());
        // Keep highest frequency if already exists
        let current = self.word_frequencies.get(&word_lower).copied().unwrap_or(0);
        if frequency > current {
            self.word_frequencies.insert(word_lower, frequency);
        }
    }

    /// Add a custom word to the dictionary
    pub fn add_custom_word(&mut self, word: &str, frequency: u32) {
        self.add_word(word, frequency);
    }

    /// Check if a word is in the dictionary
    pub fn is_known(&self, word: &str) -> bool {
        self.dictionary.contains(&word.to_lowercase())
    }

    /// Get frequency for a word
    pub fn get_frequency(&self, word: &str) -> u32 {
        *self.word_frequencies.get(&word.to_lowercase()).unwrap_or(&0)
    }

    /// Correct a complete query
    pub fn correct(&self, query: &str) -> CorrectionResult {
        let words: Vec<&str> = query.split_whitespace().collect();
        let mut corrections = Vec::new();
        let mut corrected_words = Vec::new();

        for word in &words {
            let word_lower = word.to_lowercase();

            // Skip if word is in dictionary or too short
            if self.dictionary.contains(&word_lower) || word.len() < 3 {
                corrected_words.push(word.to_string());
                continue;
            }

            // Check for known typo first
            if let Some(known_correction) = self.known_typos.get(&word_lower) {
                corrections.push(SpellingSuggestion {
                    original: word.to_string(),
                    suggestion: known_correction.clone(),
                    distance: 1, // Approximate
                    confidence: 0.95, // High confidence for known typos
                });
                corrected_words.push(known_correction.clone());
                continue;
            }

            // Find best suggestion using Levenshtein distance
            if let Some(suggestion) = self.find_best_suggestion(&word_lower) {
                corrections.push(SpellingSuggestion {
                    original: word.to_string(),
                    suggestion: suggestion.clone(),
                    distance: self.levenshtein(&word_lower, &suggestion),
                    confidence: self.calculate_confidence(&word_lower, &suggestion),
                });
                corrected_words.push(suggestion);
            } else {
                corrected_words.push(word.to_string());
            }
        }

        let corrected_query = corrected_words.join(" ");
        let has_corrections = !corrections.is_empty();

        CorrectionResult {
            original_query: query.to_string(),
            corrected_query,
            corrections,
            has_corrections,
        }
    }

    /// Find the best suggestion for a misspelled word
    fn find_best_suggestion(&self, word: &str) -> Option<String> {
        // Dynamic max distance based on word length
        let max_distance = match word.len() {
            0..=3 => 1,
            4..=5 => 2,
            _ => 3,
        };

        let mut best: Option<(String, usize, u32)> = None;

        for dict_word in &self.dictionary {
            let distance = self.levenshtein(word, dict_word);

            if distance <= max_distance && distance > 0 {
                let freq = *self.word_frequencies.get(dict_word).unwrap_or(&1);

                match &best {
                    None => best = Some((dict_word.clone(), distance, freq)),
                    Some((_, best_dist, best_freq)) => {
                        // Prefer lower distance, then higher frequency
                        if distance < *best_dist
                            || (distance == *best_dist && freq > *best_freq)
                        {
                            best = Some((dict_word.clone(), distance, freq));
                        }
                    }
                }
            }
        }

        best.map(|(word, _, _)| word)
    }

    /// Calculate Levenshtein distance between two strings
    fn levenshtein(&self, s1: &str, s2: &str) -> usize {
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();

        let len1 = s1_chars.len();
        let len2 = s2_chars.len();

        if len1 == 0 {
            return len2;
        }
        if len2 == 0 {
            return len1;
        }

        // Early exit for very different lengths
        if (len1 as isize - len2 as isize).unsigned_abs() > 3 {
            return len1.max(len2);
        }

        let mut matrix = vec![vec![0usize; len2 + 1]; len1 + 1];

        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                    0
                } else {
                    1
                };

                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);

                // Transposition (Damerau-Levenshtein)
                if i > 1
                    && j > 1
                    && s1_chars[i - 1] == s2_chars[j - 2]
                    && s1_chars[i - 2] == s2_chars[j - 1]
                {
                    matrix[i][j] = matrix[i][j].min(matrix[i - 2][j - 2] + 1);
                }
            }
        }

        matrix[len1][len2]
    }

    /// Calculate confidence for a suggestion
    fn calculate_confidence(&self, original: &str, suggestion: &str) -> f64 {
        let distance = self.levenshtein(original, suggestion);
        let max_len = original.len().max(suggestion.len()) as f64;

        // Base confidence from relative distance
        let relative_distance = distance as f64 / max_len;
        let base_confidence = 1.0 - relative_distance;

        // Boost for high-frequency words
        let freq_boost = if let Some(&freq) = self.word_frequencies.get(suggestion) {
            (freq as f64 / 1000.0).min(0.15)
        } else {
            0.0
        };

        // Boost for same starting letter
        let start_boost = if original.chars().next() == suggestion.chars().next() {
            0.05
        } else {
            0.0
        };

        // Boost for same length
        let len_boost = if original.len() == suggestion.len() {
            0.03
        } else {
            0.0
        };

        (base_confidence + freq_boost + start_boost + len_boost).min(1.0)
    }

    /// Get "did you mean" suggestions for a word
    pub fn did_you_mean(&self, word: &str) -> Vec<String> {
        let word_lower = word.to_lowercase();
        let mut suggestions: Vec<(String, usize, u32)> = Vec::new();

        // Check known typos first
        if let Some(known) = self.known_typos.get(&word_lower) {
            return vec![known.clone()];
        }

        for dict_word in &self.dictionary {
            let distance = self.levenshtein(&word_lower, dict_word);

            if distance <= 3 && distance > 0 {
                let freq = *self.word_frequencies.get(dict_word).unwrap_or(&1);
                suggestions.push((dict_word.clone(), distance, freq));
            }
        }

        // Sort by distance, then frequency
        suggestions.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| b.2.cmp(&a.2)));

        suggestions.into_iter().take(5).map(|(w, _, _)| w).collect()
    }

    /// Get suggestions with confidence scores
    pub fn suggest_with_confidence(&self, word: &str) -> Vec<SpellingSuggestion> {
        let word_lower = word.to_lowercase();

        if self.dictionary.contains(&word_lower) {
            return Vec::new();
        }

        let mut suggestions = Vec::new();

        // Check known typos first
        if let Some(known) = self.known_typos.get(&word_lower) {
            suggestions.push(SpellingSuggestion {
                original: word.to_string(),
                suggestion: known.clone(),
                distance: 1,
                confidence: 0.95,
            });
            return suggestions;
        }

        for dict_word in &self.dictionary {
            let distance = self.levenshtein(&word_lower, dict_word);

            if distance <= 3 && distance > 0 {
                let confidence = self.calculate_confidence(&word_lower, dict_word);
                if confidence >= 0.5 {
                    suggestions.push(SpellingSuggestion {
                        original: word.to_string(),
                        suggestion: dict_word.clone(),
                        distance,
                        confidence,
                    });
                }
            }
        }

        suggestions.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        suggestions.truncate(5);
        suggestions
    }

    /// Check if a word is a known TTRPG typo
    pub fn is_known_typo(&self, word: &str) -> bool {
        self.known_typos.contains_key(&word.to_lowercase())
    }

    /// Get the correction for a known typo
    pub fn get_typo_correction(&self, word: &str) -> Option<&String> {
        self.known_typos.get(&word.to_lowercase())
    }
}

impl Default for SpellCorrector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein() {
        let corrector = SpellCorrector::new();

        assert_eq!(corrector.levenshtein("kitten", "sitting"), 3);
        assert_eq!(corrector.levenshtein("spell", "spell"), 0);
        assert_eq!(corrector.levenshtein("", "test"), 4);
        assert_eq!(corrector.levenshtein("test", ""), 4);
    }

    #[test]
    fn test_correction() {
        let corrector = SpellCorrector::new();

        let result = corrector.correct("firebll damage");
        assert!(result.has_corrections);
        assert!(result.corrected_query.contains("fireball"));
    }

    #[test]
    fn test_known_typo() {
        let corrector = SpellCorrector::new();

        let result = corrector.correct("rouge attack");
        assert!(result.has_corrections);
        assert!(result.corrected_query.contains("rogue"));
    }

    #[test]
    fn test_ability_score_typo() {
        let corrector = SpellCorrector::new();

        let result = corrector.correct("strenght check");
        assert!(result.has_corrections);
        assert!(result.corrected_query.contains("strength"));
    }

    #[test]
    fn test_did_you_mean() {
        let corrector = SpellCorrector::new();

        let suggestions = corrector.did_you_mean("fiorball");
        assert!(!suggestions.is_empty());
        assert!(suggestions.contains(&"fireball".to_string()));
    }

    #[test]
    fn test_known_words() {
        let corrector = SpellCorrector::new();

        assert!(corrector.is_known("fireball"));
        assert!(corrector.is_known("dragon"));
        assert!(corrector.is_known("barbarian"));
        assert!(corrector.is_known("paladin"));
        assert!(!corrector.is_known("xyzabc123"));
    }

    #[test]
    fn test_frequency_tiers() {
        let corrector = SpellCorrector::new();

        // Core terms should have highest frequency
        assert!(corrector.get_frequency("strength") >= FrequencyTier::Core as u32);
        assert!(corrector.get_frequency("barbarian") >= FrequencyTier::Core as u32);

        // Common terms should have high frequency
        assert!(corrector.get_frequency("concentration") >= FrequencyTier::Common as u32);

        // General English should have lower frequency
        assert!(corrector.get_frequency("the") < FrequencyTier::Common as u32);
    }

    #[test]
    fn test_is_known_typo() {
        let corrector = SpellCorrector::new();

        assert!(corrector.is_known_typo("rouge"));
        assert!(corrector.is_known_typo("strenght"));
        assert!(corrector.is_known_typo("wizzard"));
        assert!(!corrector.is_known_typo("wizard"));
    }

    #[test]
    fn test_suggest_with_confidence() {
        let corrector = SpellCorrector::new();

        let suggestions = corrector.suggest_with_confidence("fiorball");
        assert!(!suggestions.is_empty());
        assert!(suggestions[0].suggestion == "fireball");
        assert!(suggestions[0].confidence > 0.5);
    }

    #[test]
    fn test_class_corrections() {
        let corrector = SpellCorrector::new();

        assert!(corrector.correct("barberian").corrected_query.contains("barbarian"));
        assert!(corrector.correct("wizzard").corrected_query.contains("wizard"));
        assert!(corrector.correct("paliden").corrected_query.contains("paladin"));
    }

    #[test]
    fn test_race_corrections() {
        let corrector = SpellCorrector::new();

        assert!(corrector.correct("tiefeling").corrected_query.contains("tiefling"));
        assert!(corrector.correct("dragonbon").corrected_query.contains("dragonborn"));
    }

    #[test]
    fn test_custom_word() {
        let mut corrector = SpellCorrector::new();

        assert!(!corrector.is_known("customword"));
        corrector.add_custom_word("customword", 500);
        assert!(corrector.is_known("customword"));
    }
}
