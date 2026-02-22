//! Synonym Expansion for TTRPG Search
//!
//! Expands search terms with domain-specific synonyms to improve
//! search recall without relying on external search engine features.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use serde::{Deserialize, Serialize};

use super::error::{PreprocessError, PreprocessResult};

/// Bidirectional synonym map supporting multi-way and one-way synonyms.
///
/// # Multi-way synonyms
/// All terms in a group are interchangeable:
/// - "hp" ↔ "hit points" ↔ "health"
///
/// # One-way synonyms
/// Source expands to targets but not reverse:
/// - "dragon" → ["wyrm", "drake"] (but "wyrm" doesn't expand to "dragon")
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SynonymMap {
    /// Multi-way synonym groups: all terms interchangeable
    multi_way: Vec<HashSet<String>>,
    /// One-way synonyms: source → targets only
    one_way: HashMap<String, Vec<String>>,
    /// Maximum expansions per term (prevents query explosion)
    max_expansions: usize,
}

impl SynonymMap {
    /// Create empty map with expansion limit.
    pub fn new(max_expansions: usize) -> Self {
        Self {
            multi_way: Vec::new(),
            one_way: HashMap::new(),
            max_expansions,
        }
    }

    /// Load from TOML configuration file.
    ///
    /// Expected format:
    /// ```toml
    /// max_expansions = 5
    ///
    /// [multi_way]
    /// hp = ["hit points", "health", "life"]
    /// ac = ["armor class"]
    ///
    /// [one_way]
    /// dragon = ["wyrm", "drake"]
    /// ```
    pub fn from_toml_file(path: &Path) -> PreprocessResult<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_toml_str(&content)
    }

    /// Load from TOML string.
    pub fn from_toml_str(content: &str) -> PreprocessResult<Self> {
        #[derive(Deserialize)]
        struct TomlSynonyms {
            #[serde(default = "default_max_expansions")]
            max_expansions: usize,
            #[serde(default)]
            multi_way: HashMap<String, Vec<String>>,
            #[serde(default)]
            one_way: HashMap<String, Vec<String>>,
        }

        fn default_max_expansions() -> usize { 5 }

        let parsed: TomlSynonyms = toml::from_str(content)
            .map_err(|e| PreprocessError::SynonymParse(e.to_string()))?;

        let mut map = SynonymMap::new(parsed.max_expansions);

        // Convert multi_way entries to sets
        for (key, values) in parsed.multi_way {
            let mut group = HashSet::new();
            group.insert(key.to_lowercase());
            for v in values {
                group.insert(v.to_lowercase());
            }
            map.multi_way.push(group);
        }

        // Add one-way synonyms
        for (source, targets) in parsed.one_way {
            map.one_way.insert(
                source.to_lowercase(),
                targets.into_iter().map(|t| t.to_lowercase()).collect()
            );
        }

        Ok(map)
    }

    /// Add a multi-way synonym group where all terms are interchangeable.
    pub fn add_multi_way(&mut self, terms: &[&str]) {
        let group: HashSet<String> = terms.iter().map(|t| t.to_lowercase()).collect();
        if group.len() > 1 {
            self.multi_way.push(group);
        }
    }

    /// Add a one-way synonym mapping.
    pub fn add_one_way(&mut self, source: &str, targets: &[&str]) {
        let source_lower = source.to_lowercase();
        let targets_lower: Vec<String> = targets.iter().map(|t| t.to_lowercase()).collect();

        self.one_way
            .entry(source_lower)
            .or_insert_with(Vec::new)
            .extend(targets_lower);
    }

    /// Expand a single term to its synonyms.
    ///
    /// Returns a vector containing the original term plus its synonyms,
    /// limited by max_expansions.
    pub fn expand_term(&self, term: &str) -> Vec<String> {
        let term_lower = term.to_lowercase();
        let mut expansions = vec![term_lower.clone()];

        // Check multi-way groups
        for group in &self.multi_way {
            if group.contains(&term_lower) {
                for synonym in group {
                    if *synonym != term_lower && !expansions.contains(synonym) {
                        expansions.push(synonym.clone());
                        if expansions.len() >= self.max_expansions {
                            return expansions;
                        }
                    }
                }
            }
        }

        // Check one-way synonyms
        if let Some(targets) = self.one_way.get(&term_lower) {
            for target in targets {
                if !expansions.contains(target) {
                    expansions.push(target.clone());
                    if expansions.len() >= self.max_expansions {
                        return expansions;
                    }
                }
            }
        }

        expansions
    }

    /// Expand all terms in a query.
    pub fn expand_query(&self, query: &str) -> ExpandedQuery {
        let terms: Vec<&str> = query.split_whitespace().collect();
        let term_groups: Vec<Vec<String>> = terms
            .iter()
            .map(|term| self.expand_term(term))
            .collect();

        ExpandedQuery {
            original: query.to_string(),
            term_groups,
        }
    }

    /// Merge another synonym map into this one.
    pub fn merge(&mut self, other: &SynonymMap) {
        // Merge multi-way groups
        for group in &other.multi_way {
            // Check if this group overlaps with an existing one
            let mut found_overlap = false;
            for existing in &mut self.multi_way {
                if group.intersection(existing).next().is_some() {
                    // Merge the groups
                    existing.extend(group.iter().cloned());
                    found_overlap = true;
                    break;
                }
            }
            if !found_overlap {
                self.multi_way.push(group.clone());
            }
        }

        // Merge one-way synonyms
        for (source, targets) in &other.one_way {
            self.one_way
                .entry(source.clone())
                .or_insert_with(Vec::new)
                .extend(targets.iter().cloned());
        }
    }
}

/// Result of synonym expansion.
#[derive(Debug, Clone)]
pub struct ExpandedQuery {
    /// Original query string
    pub original: String,
    /// Term groups: [["hp", "hit points", "health"], ["restore", "heal"]]
    pub term_groups: Vec<Vec<String>>,
}

impl ExpandedQuery {
    /// Generate SurrealDB FTS query with OR-expanded synonyms using indexed match.
    ///
    /// Generates a query like:
    /// `(content @1@ 'fireball' OR content @1@ 'fire bolt') AND (content @1@ 'damage')`
    ///
    /// **Note**: Using indexed `@N@` operators allows `search::score(N)` and
    /// `search::highlight(..., N)` but each reference must be unique. If you
    /// have multiple OR clauses using the same reference, SurrealDB will error
    /// with "Duplicated Match reference". For OR-expanded queries, use
    /// `to_surrealdb_fts_plain()` instead.
    pub fn to_surrealdb_fts(&self, field: &str, analyzer_ref: u32) -> String {
        let mut group_clauses = Vec::new();

        for group in &self.term_groups {
            if group.is_empty() {
                continue;
            }

            if group.len() == 1 {
                // Single term, no OR needed
                group_clauses.push(format!(
                    "{} @{}@ '{}'",
                    field, analyzer_ref, escape_fts_term(&group[0])
                ));
            } else {
                // Multiple synonyms, join with OR
                let or_terms: Vec<String> = group
                    .iter()
                    .map(|term| format!("{} @{}@ '{}'", field, analyzer_ref, escape_fts_term(term)))
                    .collect();
                group_clauses.push(format!("({})", or_terms.join(" OR ")));
            }
        }

        if group_clauses.is_empty() {
            return String::new();
        }

        group_clauses.join(" AND ")
    }

    /// Generate SurrealDB FTS query with OR-expanded synonyms using internal boolean syntax.
    ///
    /// Generates a query using SurrealDB's internal FTS boolean operators:
    /// `content @@ '(fireball | "fire bolt") AND damage'`
    ///
    /// This uses a **single** `@@` operator with boolean logic inside the search string:
    /// - `|` for OR between synonyms
    /// - `AND` for required terms between groups
    /// - Double quotes for multi-word phrases
    ///
    /// This avoids "Duplicated Match reference" errors that occur when using
    /// multiple `@@` operators (each `@@` implicitly uses reference 1).
    ///
    /// Use this for hybrid search where vector scores provide ranking.
    pub fn to_surrealdb_fts_plain(&self, field: &str) -> String {
        let mut group_clauses = Vec::new();

        for group in &self.term_groups {
            if group.is_empty() {
                continue;
            }

            if group.len() == 1 {
                // Single term - quote if multi-word
                let term = &group[0];
                if term.contains(' ') {
                    group_clauses.push(format!("\"{}\"", escape_fts_phrase(term)));
                } else {
                    group_clauses.push(escape_fts_term(term));
                }
            } else {
                // Multiple synonyms - join with | (SurrealDB's internal OR)
                let or_terms: Vec<String> = group
                    .iter()
                    .map(|term| {
                        if term.contains(' ') {
                            format!("\"{}\"", escape_fts_phrase(term))
                        } else {
                            escape_fts_term(term)
                        }
                    })
                    .collect();
                group_clauses.push(format!("({})", or_terms.join(" | ")));
            }
        }

        if group_clauses.is_empty() {
            return String::new();
        }

        // Combine all groups with AND, wrap in single @@ expression
        let inner_query = group_clauses.join(" AND ");
        format!("{} @@ '{}'", field, inner_query)
    }

    /// Generate SQLite FTS5 MATCH expression (for future sqlite-vec support).
    ///
    /// Generates a query like:
    /// `("fireball" OR "fire bolt") AND ("damage" OR "harm")`
    pub fn to_fts5_match(&self) -> String {
        let mut group_clauses = Vec::new();

        for group in &self.term_groups {
            if group.is_empty() {
                continue;
            }

            if group.len() == 1 {
                group_clauses.push(format!("\"{}\"", escape_fts5_term(&group[0])));
            } else {
                let or_terms: Vec<String> = group
                    .iter()
                    .map(|term| format!("\"{}\"", escape_fts5_term(term)))
                    .collect();
                group_clauses.push(format!("({})", or_terms.join(" OR ")));
            }
        }

        if group_clauses.is_empty() {
            return String::new();
        }

        group_clauses.join(" AND ")
    }

    /// Get the corrected query text suitable for embedding generation.
    /// This returns the original terms joined (not expanded) to avoid
    /// introducing noise into vector search.
    pub fn text_for_embedding(&self) -> String {
        self.term_groups
            .iter()
            .filter_map(|g| g.first())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Escape a term for SurrealDB FTS query (single-quoted context)
fn escape_fts_term(term: &str) -> String {
    term.replace('\'', "''")
}

/// Escape a phrase for SurrealDB FTS query (double-quoted context inside single quotes)
fn escape_fts_phrase(term: &str) -> String {
    // Inside a single-quoted string, double quotes don't need escaping
    // but single quotes do (doubled), and backslashes need escaping
    term.replace('\\', "\\\\").replace('\'', "''")
}

/// Escape a term for FTS5 MATCH expression
fn escape_fts5_term(term: &str) -> String {
    term.replace('"', "\"\"")
}

/// Build the default TTRPG synonym map with comprehensive game terminology.
pub fn build_default_ttrpg_synonyms() -> SynonymMap {
    let mut map = SynonymMap::new(5);

    // Stat abbreviations
    map.add_multi_way(&["hp", "hit points", "health", "life"]);
    map.add_multi_way(&["ac", "armor class", "armour class"]);
    map.add_multi_way(&["str", "strength"]);
    map.add_multi_way(&["dex", "dexterity"]);
    map.add_multi_way(&["con", "constitution"]);
    map.add_multi_way(&["int", "intelligence"]);
    map.add_multi_way(&["wis", "wisdom"]);
    map.add_multi_way(&["cha", "charisma"]);

    // Game mechanics
    map.add_multi_way(&["dc", "difficulty class"]);
    map.add_multi_way(&["cr", "challenge rating"]);
    map.add_multi_way(&["xp", "experience points", "experience"]);
    map.add_multi_way(&["aoo", "attack of opportunity", "opportunity attack"]);
    map.add_multi_way(&["crit", "critical hit", "nat 20", "natural 20"]);
    map.add_multi_way(&["dm", "dungeon master", "game master", "gm"]);
    map.add_multi_way(&["pc", "player character"]);
    map.add_multi_way(&["npc", "non-player character"]);
    map.add_multi_way(&["init", "initiative"]);
    map.add_multi_way(&["proficiency", "prof", "proficiency bonus"]);

    // Conditions
    map.add_multi_way(&["prone", "knocked down", "lying down"]);
    map.add_multi_way(&["grappled", "grabbed", "held"]);
    map.add_multi_way(&["stunned", "stun"]);
    map.add_multi_way(&["frightened", "scared", "afraid"]);
    map.add_multi_way(&["paralyzed", "paralysed"]);
    map.add_multi_way(&["invisible", "invis"]);
    map.add_multi_way(&["unconscious", "ko", "knocked out"]);

    // Book abbreviations
    map.add_multi_way(&["phb", "player's handbook", "players handbook"]);
    map.add_multi_way(&["dmg", "dungeon master's guide", "dm guide"]);
    map.add_multi_way(&["mm", "monster manual"]);
    map.add_multi_way(&["xge", "xanathar's guide", "xanathar"]);
    map.add_multi_way(&["tce", "tasha's cauldron", "tasha"]);
    map.add_multi_way(&["vgm", "volo's guide", "volo"]);
    map.add_multi_way(&["mtof", "mordenkainen's tome", "mordenkainen"]);

    // Damage types
    map.add_multi_way(&["fire damage", "flame damage", "burning"]);
    map.add_multi_way(&["cold damage", "frost damage", "ice damage"]);
    map.add_multi_way(&["lightning damage", "electric damage", "shock"]);
    map.add_multi_way(&["thunder damage", "sonic damage"]);
    map.add_multi_way(&["necrotic damage", "necrotic"]);
    map.add_multi_way(&["radiant damage", "radiant", "holy damage"]);
    map.add_multi_way(&["psychic damage", "psychic", "mental damage"]);
    map.add_multi_way(&["poison damage", "poison", "toxic damage"]);
    map.add_multi_way(&["force damage", "force"]);

    // Creature types (one-way: broader term expands to specific)
    map.add_one_way("undead", &["zombie", "skeleton", "vampire", "lich", "ghost", "wight"]);
    map.add_one_way("dragon", &["wyrm", "drake", "wyvern"]);
    map.add_one_way("demon", &["fiend", "devil"]);
    map.add_one_way("elemental", &["fire elemental", "water elemental", "earth elemental", "air elemental"]);

    // Common spells (multi-word terms)
    map.add_multi_way(&["magic missile", "mm"]);
    map.add_multi_way(&["fireball", "fire ball"]);
    map.add_multi_way(&["lightning bolt", "lb"]);
    map.add_multi_way(&["shield", "shield spell"]);
    map.add_multi_way(&["cure wounds", "healing spell"]);

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_way_expansion() {
        let mut map = SynonymMap::new(5);
        map.add_multi_way(&["hp", "hit points", "health"]);

        let expansions = map.expand_term("hp");
        assert!(expansions.contains(&"hp".to_string()));
        assert!(expansions.contains(&"hit points".to_string()));
        assert!(expansions.contains(&"health".to_string()));
    }

    #[test]
    fn test_one_way_expansion() {
        let mut map = SynonymMap::new(5);
        map.add_one_way("dragon", &["wyrm", "drake"]);

        // dragon expands to wyrm, drake
        let dragon_exp = map.expand_term("dragon");
        assert!(dragon_exp.contains(&"dragon".to_string()));
        assert!(dragon_exp.contains(&"wyrm".to_string()));
        assert!(dragon_exp.contains(&"drake".to_string()));

        // wyrm does NOT expand to dragon
        let wyrm_exp = map.expand_term("wyrm");
        assert_eq!(wyrm_exp, vec!["wyrm".to_string()]);
    }

    #[test]
    fn test_max_expansions_limit() {
        let mut map = SynonymMap::new(2);
        map.add_multi_way(&["a", "b", "c", "d", "e"]);

        let expansions = map.expand_term("a");
        assert_eq!(expansions.len(), 2); // Limited to max_expansions
    }

    #[test]
    fn test_case_insensitive() {
        let mut map = SynonymMap::new(5);
        map.add_multi_way(&["HP", "Hit Points"]);

        let expansions = map.expand_term("hp");
        assert!(expansions.contains(&"hit points".to_string()));

        let expansions2 = map.expand_term("HIT POINTS");
        assert!(expansions2.contains(&"hp".to_string()));
    }

    #[test]
    fn test_surrealdb_fts_query() {
        let mut map = SynonymMap::new(5);
        map.add_multi_way(&["hp", "hit points"]);

        let expanded = map.expand_query("restore hp");
        let fts = expanded.to_surrealdb_fts("content", 1);

        assert!(fts.contains("content @1@ 'restore'"));
        assert!(fts.contains("content @1@ 'hp'"));
        assert!(fts.contains("content @1@ 'hit points'"));
        assert!(fts.contains(" OR "));
        assert!(fts.contains(" AND "));
    }

    #[test]
    fn test_fts5_match_query() {
        let mut map = SynonymMap::new(5);
        map.add_multi_way(&["hp", "hit points"]);

        let expanded = map.expand_query("restore hp");
        let fts5 = expanded.to_fts5_match();

        assert!(fts5.contains("\"restore\""));
        assert!(fts5.contains("\"hp\""));
        assert!(fts5.contains("\"hit points\""));
    }

    #[test]
    fn test_surrealdb_fts_plain_query() {
        let mut map = SynonymMap::new(5);
        map.add_multi_way(&["hp", "hit points"]);

        let expanded = map.expand_query("restore hp");
        let fts = expanded.to_surrealdb_fts_plain("content");

        // Should be a single @@ with internal boolean logic
        // Expected: content @@ 'restore AND (hp | "hit points")'
        assert!(fts.starts_with("content @@ '"), "Should use single @@ operator: {}", fts);
        assert!(fts.ends_with("'"), "Should end with single quote: {}", fts);
        assert!(fts.contains("restore"), "Should contain 'restore': {}", fts);
        assert!(fts.contains("hp"), "Should contain 'hp': {}", fts);
        assert!(fts.contains("hit points"), "Should contain 'hit points': {}", fts);
        assert!(fts.contains(" | "), "Should use | for OR between synonyms: {}", fts);
        assert!(fts.contains(" AND "), "Should use AND between groups: {}", fts);
        // Should NOT have multiple @@ operators
        assert_eq!(fts.matches("@@").count(), 1, "Should have exactly one @@ operator: {}", fts);
    }

    #[test]
    fn test_surrealdb_fts_plain_single_term() {
        let map = SynonymMap::new(5); // No synonyms

        let expanded = map.expand_query("fireball");
        let fts = expanded.to_surrealdb_fts_plain("content");

        // Single term without synonyms
        assert_eq!(fts, "content @@ 'fireball'");
    }

    #[test]
    fn test_default_ttrpg_synonyms() {
        let map = build_default_ttrpg_synonyms();

        // Test stat abbreviations
        let hp_exp = map.expand_term("hp");
        assert!(hp_exp.contains(&"hit points".to_string()));

        // Test book abbreviations
        let phb_exp = map.expand_term("phb");
        assert!(phb_exp.contains(&"player's handbook".to_string()));

        // Test one-way creature types
        let undead_exp = map.expand_term("undead");
        assert!(undead_exp.contains(&"zombie".to_string()));
    }

    #[test]
    fn test_toml_parsing() {
        let toml = r#"
max_expansions = 3

[multi_way]
hp = ["hit points", "health"]
ac = ["armor class"]

[one_way]
dragon = ["wyrm", "drake"]
"#;
        let map = SynonymMap::from_toml_str(toml).unwrap();

        let hp_exp = map.expand_term("hp");
        assert!(hp_exp.contains(&"hit points".to_string()));
        assert!(hp_exp.contains(&"health".to_string()));

        let dragon_exp = map.expand_term("dragon");
        assert!(dragon_exp.contains(&"wyrm".to_string()));
    }

    #[test]
    fn test_text_for_embedding() {
        let mut map = SynonymMap::new(5);
        map.add_multi_way(&["hp", "hit points"]);

        let expanded = map.expand_query("restore hp now");
        let embedding_text = expanded.text_for_embedding();

        // Should use first term from each group (the original terms)
        assert_eq!(embedding_text, "restore hp now");
    }
}
