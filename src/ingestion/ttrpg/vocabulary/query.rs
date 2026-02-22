//! Query Processing
//!
//! Functions and data for query expansion, spell correction, fuzzy matching,
//! and stop word filtering. Ported from MDMAI query_processor.py.

use once_cell::sync::Lazy;
use std::collections::HashSet;

use super::systems::DnD5eVocabulary;
use super::GameVocabulary;

// ============================================================================
// CORE TTRPG VOCABULARY
// ============================================================================

/// Core TTRPG vocabulary for spell correction and fuzzy matching
/// Based on MDMAI query_processor.py ttrpg_vocabulary
pub static TTRPG_CORE_VOCABULARY: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut vocab = HashSet::new();

    // D&D/Generic TTRPG terms
    vocab.extend([
        "fireball",
        "magic missile",
        "cure wounds",
        "healing word",
        "armor class",
        "hit points",
        "saving throw",
        "ability check",
        "advantage",
        "disadvantage",
        "critical hit",
        "initiative",
        "dungeon master",
        "player character",
        "non-player character",
        "experience points",
        "challenge rating",
        "spell slot",
        "cantrip",
        "ritual",
        "concentration",
        "components",
        "somatic",
        "verbal",
        "material",
        "spell save",
        "spell attack",
    ]);

    // Stats
    vocab.extend([
        "strength",
        "dexterity",
        "constitution",
        "intelligence",
        "wisdom",
        "charisma",
        "proficiency",
        "expertise",
    ]);

    // Conditions
    vocab.extend([
        "blinded",
        "charmed",
        "deafened",
        "frightened",
        "grappled",
        "incapacitated",
        "invisible",
        "paralyzed",
        "petrified",
        "poisoned",
        "prone",
        "restrained",
        "stunned",
        "unconscious",
    ]);

    // Damage types
    vocab.extend([
        "acid",
        "bludgeoning",
        "cold",
        "fire",
        "force",
        "lightning",
        "necrotic",
        "piercing",
        "poison",
        "psychic",
        "radiant",
        "slashing",
        "thunder",
    ]);

    // Creature types
    vocab.extend([
        "aberration",
        "beast",
        "celestial",
        "construct",
        "dragon",
        "elemental",
        "fey",
        "fiend",
        "giant",
        "humanoid",
        "monstrosity",
        "ooze",
        "plant",
        "undead",
    ]);

    // Actions
    vocab.extend([
        "action",
        "bonus action",
        "reaction",
        "movement",
        "attack",
        "cast",
        "dash",
        "disengage",
        "dodge",
        "help",
        "hide",
        "ready",
        "search",
        "use",
    ]);

    // Horror/Delta Green terms
    vocab.extend([
        "sanity",
        "willpower",
        "bond",
        "breaking point",
        "unnatural",
        "agent",
        "handler",
        "operation",
        "green box",
    ]);

    vocab
});

// ============================================================================
// QUERY EXPANSIONS
// ============================================================================

/// Query expansion mappings - abbreviations to full terms
/// Based on MDMAI query_processor.py expansions
pub static QUERY_EXPANSIONS: Lazy<Vec<(&'static str, &'static [&'static str])>> = Lazy::new(|| {
    vec![
        ("ac", &["armor class", "ac"] as &[&str]),
        ("hp", &["hit points", "hp", "health"]),
        ("dm", &["dungeon master", "dm", "game master", "gm"]),
        ("gm", &["game master", "gm", "dungeon master", "dm"]),
        ("pc", &["player character", "pc", "character"]),
        ("npc", &["non-player character", "npc"]),
        ("xp", &["experience points", "xp", "exp"]),
        ("cr", &["challenge rating", "cr"]),
        ("str", &["strength", "str"]),
        ("dex", &["dexterity", "dex"]),
        ("con", &["constitution", "con"]),
        ("int", &["intelligence", "int"]),
        ("wis", &["wisdom", "wis"]),
        ("cha", &["charisma", "cha"]),
        ("save", &["saving throw", "save"]),
        ("dc", &["difficulty class", "dc"]),
        ("san", &["sanity", "san"]),
        ("wp", &["willpower", "wp"]),
    ]
});

/// Synonym groups for query expansion
/// Based on MDMAI query_processor.py synonyms
pub static QUERY_SYNONYMS: Lazy<Vec<&'static [&'static str]>> = Lazy::new(|| {
    vec![
        &["spell", "magic", "incantation", "cantrip"] as &[&str],
        &["monster", "creature", "enemy", "foe", "beast"],
        &["damage", "harm", "hurt", "wound"],
        &["heal", "cure", "restore", "recover"],
        &["attack", "strike", "hit", "assault"],
        &["defend", "protect", "guard", "shield"],
        &["wizard", "mage", "sorcerer", "spellcaster"],
        &["warrior", "fighter", "soldier", "combatant"],
        &["rogue", "thief", "assassin", "scoundrel"],
        &["cleric", "priest", "healer", "divine"],
        &["sanity", "mental health", "psychological"],
        &["horror", "terror", "fear", "dread"],
    ]
});

/// Mechanic type classification keywords
/// Used to detect mechanic_type for SearchDocument
pub static MECHANIC_TYPE_KEYWORDS: Lazy<Vec<(&'static str, &'static [&'static str])>> =
    Lazy::new(|| {
        vec![
            (
                "skill_check",
                &["skill check", "ability check", "roll", "test", "contested"] as &[&str],
            ),
            (
                "combat",
                &[
                    "attack",
                    "damage",
                    "hit",
                    "miss",
                    "armor class",
                    "initiative",
                    "weapon",
                ],
            ),
            (
                "damage",
                &["damage", "hit points", "hp", "wound", "injury", "lethal"],
            ),
            (
                "healing",
                &["heal", "cure", "restore", "recovery", "rest", "medicine"],
            ),
            (
                "sanity",
                &[
                    "sanity",
                    "san",
                    "mental",
                    "madness",
                    "insanity",
                    "psychological",
                ],
            ),
            (
                "equipment",
                &["equipment", "gear", "weapon", "armor", "item", "inventory"],
            ),
            (
                "character_creation",
                &[
                    "character creation",
                    "ability scores",
                    "background",
                    "class",
                    "race",
                    "ancestry",
                ],
            ),
            (
                "magic",
                &["spell", "magic", "casting", "ritual", "arcane", "divine"],
            ),
            (
                "movement",
                &["movement", "speed", "travel", "distance", "terrain"],
            ),
            (
                "social",
                &[
                    "persuade",
                    "intimidate",
                    "deceive",
                    "diplomacy",
                    "social",
                    "charisma",
                ],
            ),
        ]
    });

// ============================================================================
// QUERY EXPANSION FUNCTIONS
// ============================================================================

/// Expand a query term using abbreviations and synonyms
pub fn expand_query_term(term: &str) -> Vec<String> {
    let term_lower = term.to_lowercase();
    let mut expanded = vec![term_lower.clone()];

    // Check abbreviation expansions
    for (abbr, expansions) in QUERY_EXPANSIONS.iter() {
        if term_lower == *abbr {
            expanded.extend(expansions.iter().map(|s| s.to_string()));
            break;
        }
    }

    // Check synonym groups
    for group in QUERY_SYNONYMS.iter() {
        if group.contains(&term_lower.as_str()) {
            expanded.extend(
                group
                    .iter()
                    .filter(|s| **s != term_lower)
                    .map(|s| s.to_string()),
            );
            break;
        }
    }

    expanded
}

/// Expand a full query string
pub fn expand_query(query: &str) -> String {
    let words: Vec<&str> = query.split_whitespace().collect();
    let mut expanded_terms: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for word in words {
        for term in expand_query_term(word) {
            if !seen.contains(&term) {
                seen.insert(term.clone());
                expanded_terms.push(term);
            }
        }
    }

    expanded_terms.join(" ")
}

// ============================================================================
// MECHANIC DETECTION
// ============================================================================

/// Detect mechanic type from content
pub fn detect_mechanic_type(text: &str) -> Option<&'static str> {
    let text_lower = text.to_lowercase();
    let mut best_match: Option<(&'static str, usize)> = None;

    for (mechanic_type, keywords) in MECHANIC_TYPE_KEYWORDS.iter() {
        let count = keywords.iter().filter(|kw| text_lower.contains(*kw)).count();

        if count > 0 {
            match best_match {
                None => best_match = Some((mechanic_type, count)),
                Some((_, best_count)) if count > best_count => {
                    best_match = Some((mechanic_type, count));
                }
                _ => {}
            }
        }
    }

    best_match.map(|(t, _)| t)
}

/// Extract semantic keywords from content for embedding boost
pub fn extract_semantic_keywords(text: &str, max_keywords: usize) -> Vec<String> {
    let text_lower = text.to_lowercase();
    let mut keywords = Vec::new();

    // Check for TTRPG vocabulary terms
    for term in TTRPG_CORE_VOCABULARY.iter() {
        if text_lower.contains(term) && !keywords.contains(&term.to_string()) {
            keywords.push(term.to_string());
            if keywords.len() >= max_keywords {
                break;
            }
        }
    }

    // Also check damage types, conditions, creature types from D&D vocabulary
    let dnd_vocab = DnD5eVocabulary;
    for term in dnd_vocab.damage_types() {
        if text_lower.contains(term) && !keywords.contains(&term.to_string()) {
            keywords.push(term.to_string());
            if keywords.len() >= max_keywords {
                break;
            }
        }
    }

    for term in dnd_vocab.conditions() {
        if text_lower.contains(term) && !keywords.contains(&term.to_string()) {
            keywords.push(term.to_string());
            if keywords.len() >= max_keywords {
                break;
            }
        }
    }

    keywords
}

// ============================================================================
// FUZZY MATCHING
// ============================================================================

/// Simple fuzzy match using edit distance
/// Returns similarity score (0.0 to 1.0)
pub fn fuzzy_match(a: &str, b: &str) -> f32 {
    if a == b {
        return 1.0;
    }

    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    if a_lower == b_lower {
        return 1.0;
    }

    // Simple Levenshtein-based similarity
    let distance = levenshtein_distance(&a_lower, &b_lower);
    let max_len = a_lower.len().max(b_lower.len());

    if max_len == 0 {
        return 1.0;
    }

    1.0 - (distance as f32 / max_len as f32)
}

/// Calculate Levenshtein edit distance between two strings
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
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

    for (i, row) in matrix.iter_mut().enumerate().take(a_len + 1) {
        row[0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[a_len][b_len]
}

// ============================================================================
// SPELL CORRECTION
// ============================================================================

/// Correct spelling using fuzzy matching against TTRPG vocabulary
/// Returns corrected word if a close match is found, otherwise original
pub fn correct_spelling(word: &str, threshold: f32) -> String {
    let word_lower = word.to_lowercase();

    // Already in vocabulary
    if TTRPG_CORE_VOCABULARY.contains(word_lower.as_str()) {
        return word_lower;
    }

    // Find closest match
    let mut best_match: Option<(&str, f32)> = None;

    for term in TTRPG_CORE_VOCABULARY.iter() {
        let similarity = fuzzy_match(&word_lower, term);
        if similarity >= threshold {
            match best_match {
                None => best_match = Some((term, similarity)),
                Some((_, best_sim)) if similarity > best_sim => {
                    best_match = Some((term, similarity));
                }
                _ => {}
            }
        }
    }

    best_match
        .map(|(term, _)| term.to_string())
        .unwrap_or(word_lower)
}

/// Correct spelling in a query string
pub fn correct_query_spelling(query: &str, threshold: f32) -> String {
    query
        .split_whitespace()
        .map(|word| correct_spelling(word, threshold))
        .collect::<Vec<_>>()
        .join(" ")
}

// ============================================================================
// STOP WORDS
// ============================================================================

/// Common English stop words filtered out of BM25 indexing
/// Based on MDMAI config.py STOP_WORDS
pub static BM25_STOP_WORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut words = HashSet::new();
    words.extend([
        // Articles
        "a", "an", "the", // Pronouns
        "i", "me", "my", "myself", "we", "our", "ours", "ourselves", "you", "your", "yours",
        "yourself", "yourselves", "he", "him", "his", "himself", "she", "her", "hers", "herself",
        "it", "its", "itself", "they", "them", "their", "theirs", "themselves", "what", "which",
        "who", "whom", "this", "that", "these", "those", // Verbs (common)
        "am", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "having",
        "do", "does", "did", "doing", // Prepositions
        "at", "by", "for", "with", "about", "against", "between", "into", "through", "during",
        "before", "after", "above", "below", "to", "from", "up", "down", "in", "out", "on", "off",
        "over", "under", // Conjunctions
        "and", "but", "if", "or", "because", "as", "until", "while", "of", "so", "than", "too",
        "very", "just", "can", "will", "should", // Other
        "s", "t", "now", "here", "there", "when", "where", "why", "how", "all", "each", "few",
        "more", "most", "other", "some", "such", "no", "nor", "not", "only", "own", "same", "then",
        "again", "further", "once",
    ]);
    words
});

/// Check if a word is a stop word
pub fn is_stop_word(word: &str) -> bool {
    BM25_STOP_WORDS.contains(word.to_lowercase().as_str())
}

/// Filter stop words from a list of tokens
pub fn filter_stop_words<'a>(tokens: &[&'a str]) -> Vec<&'a str> {
    tokens.iter().filter(|w| !is_stop_word(w)).copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_query_term_abbreviation() {
        let expanded = expand_query_term("ac");
        assert!(expanded.contains(&"armor class".to_string()));
        assert!(expanded.contains(&"ac".to_string()));
    }

    #[test]
    fn test_expand_query_term_synonym() {
        let expanded = expand_query_term("monster");
        assert!(expanded.contains(&"monster".to_string()));
        assert!(expanded.contains(&"creature".to_string()));
    }

    #[test]
    fn test_expand_query() {
        let expanded = expand_query("ac damage");
        assert!(expanded.contains("armor class"));
        assert!(expanded.contains("damage"));
    }

    #[test]
    fn test_detect_mechanic_type_combat() {
        let text = "Make an attack roll against the target's armor class. On a hit, deal damage.";
        assert_eq!(detect_mechanic_type(text), Some("combat"));
    }

    #[test]
    fn test_detect_mechanic_type_sanity() {
        let text = "Roll a sanity check. On failure, lose 1d6 sanity points.";
        assert_eq!(detect_mechanic_type(text), Some("sanity"));
    }

    #[test]
    fn test_extract_semantic_keywords() {
        let text = "The wizard casts fireball dealing fire damage to all creatures in the area.";
        let keywords = extract_semantic_keywords(text, 5);
        assert!(!keywords.is_empty());
        assert!(
            keywords.contains(&"fireball".to_string()) || keywords.contains(&"fire".to_string())
        );
    }

    #[test]
    fn test_fuzzy_match_exact() {
        assert_eq!(fuzzy_match("fireball", "fireball"), 1.0);
        assert_eq!(fuzzy_match("FIREBALL", "fireball"), 1.0);
    }

    #[test]
    fn test_fuzzy_match_similar() {
        let similarity = fuzzy_match("firebll", "fireball");
        assert!(similarity > 0.7); // Should be close match
    }

    #[test]
    fn test_fuzzy_match_different() {
        let similarity = fuzzy_match("hello", "world");
        assert!(similarity < 0.5); // Should be distant
    }

    #[test]
    fn test_correct_spelling() {
        // Typo should correct to vocabulary term
        let corrected = correct_spelling("firebll", 0.7);
        assert_eq!(corrected, "fireball");
    }

    #[test]
    fn test_correct_spelling_unchanged() {
        // Word not in vocabulary should remain
        let corrected = correct_spelling("xyzabc", 0.7);
        assert_eq!(corrected, "xyzabc");
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("abc", "abd"), 1);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_is_stop_word() {
        assert!(is_stop_word("the"));
        assert!(is_stop_word("THE"));
        assert!(is_stop_word("and"));
        assert!(!is_stop_word("fireball"));
        assert!(!is_stop_word("dragon"));
    }

    #[test]
    fn test_filter_stop_words() {
        let tokens = vec!["the", "dragon", "is", "a", "creature"];
        let filtered = filter_stop_words(&tokens);
        assert_eq!(filtered, vec!["dragon", "creature"]);
    }
}
