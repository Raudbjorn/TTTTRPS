//! Query Parser Module
//!
//! Parses user queries to extract constraints, negations, and named entities.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::AntonymMapper;
use crate::ingestion::ttrpg::{GameVocabulary, DnD5eVocabulary};

// ============================================================================
// Types
// ============================================================================

/// A required attribute with optional constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredAttribute {
    /// Attribute category (damage_type, creature_type, etc.)
    pub category: String,
    /// Attribute value
    pub value: String,
    /// Whether this is a hard requirement
    pub required: bool,
}

/// Parsed query constraints
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryConstraints {
    /// Original user query
    pub original_query: String,
    /// Cleaned query for semantic search
    pub semantic_query: String,
    /// Expanded query with antonym hints
    pub expanded_query: String,
    /// Required attributes found in query
    pub required_attributes: Vec<RequiredAttribute>,
    /// Excluded attributes (from negations)
    pub excluded_attributes: Vec<String>,
    /// Challenge rating range
    pub cr_range: Option<(f32, f32)>,
    /// Level range
    pub level_range: Option<(u32, u32)>,
    /// Exact match entities (quoted strings)
    pub exact_match_entities: Vec<String>,
}

// ============================================================================
// Query Parser
// ============================================================================

/// Parses TTRPG queries to extract structured constraints
pub struct QueryParser {
    vocabulary: Box<dyn GameVocabulary>,
    antonym_mapper: AntonymMapper,
    negation_pattern: Regex,
    cr_pattern: Regex,
    level_pattern: Regex,
    quoted_pattern: Regex,
}

impl Default for QueryParser {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryParser {
    /// Create a new query parser with D&D 5e vocabulary
    pub fn new() -> Self {
        Self::with_vocabulary(Box::new(DnD5eVocabulary))
    }

    /// Create a query parser with custom vocabulary
    pub fn with_vocabulary(vocabulary: Box<dyn GameVocabulary>) -> Self {
        Self {
            antonym_mapper: AntonymMapper::from_vocabulary(&*vocabulary),
            vocabulary,
            negation_pattern: Regex::new(
                r"(?i)\b(not|without|except|excluding|no)\s+(\w+)"
            ).unwrap(),
            cr_pattern: Regex::new(
                r"(?i)\bcr\s*(\d+(?:/\d+)?)\s*(?:to|-|–)\s*(\d+(?:/\d+)?)|cr\s*(\d+(?:/\d+)?)"
            ).unwrap(),
            level_pattern: Regex::new(
                r"(?i)\blevel\s*(\d+)\s*(?:to|-|–)\s*(\d+)|\blevel\s*(\d+)"
            ).unwrap(),
            quoted_pattern: Regex::new(r#""([^"]+)""#).unwrap(),
        }
    }

    /// Parse a query string into structured constraints
    pub fn parse(&self, query: &str) -> QueryConstraints {
        let mut constraints = QueryConstraints {
            original_query: query.to_string(),
            semantic_query: String::new(),
            expanded_query: String::new(),
            ..Default::default()
        };

        // Extract exact match entities (quoted strings)
        constraints.exact_match_entities = self.extract_quoted(query);

        // Extract negations
        constraints.excluded_attributes = self.extract_negations(query);

        // Extract CR range
        constraints.cr_range = self.extract_cr_range(query);

        // Extract level range
        constraints.level_range = self.extract_level_range(query);

        // Extract required attributes from vocabulary
        constraints.required_attributes = self.extract_attributes(query);

        // Build semantic query (remove negations and constraints)
        constraints.semantic_query = self.build_semantic_query(query);

        // Build expanded query with antonym hints
        constraints.expanded_query = self.build_expanded_query(&constraints);

        constraints
    }

    /// Extract quoted strings for exact matching
    fn extract_quoted(&self, query: &str) -> Vec<String> {
        self.quoted_pattern
            .captures_iter(query)
            .map(|cap| cap.get(1).unwrap().as_str().to_string())
            .collect()
    }

    /// Extract negated terms
    fn extract_negations(&self, query: &str) -> Vec<String> {
        self.negation_pattern
            .captures_iter(query)
            .map(|cap| cap.get(2).unwrap().as_str().to_lowercase())
            .collect()
    }

    /// Extract challenge rating range
    fn extract_cr_range(&self, query: &str) -> Option<(f32, f32)> {
        if let Some(caps) = self.cr_pattern.captures(query) {
            // Range pattern: "cr 1 to 5"
            if let (Some(min), Some(max)) = (caps.get(1), caps.get(2)) {
                let min_cr = Self::parse_cr(min.as_str())?;
                let max_cr = Self::parse_cr(max.as_str())?;
                return Some((min_cr, max_cr));
            }
            // Single value: "cr 3"
            if let Some(val) = caps.get(3) {
                let cr = Self::parse_cr(val.as_str())?;
                return Some((cr, cr));
            }
        }
        None
    }

    /// Parse CR string (handles fractions like 1/4)
    fn parse_cr(cr_str: &str) -> Option<f32> {
        if cr_str.contains('/') {
            let parts: Vec<&str> = cr_str.split('/').collect();
            if parts.len() == 2 {
                let num = parts[0].parse::<f32>().ok()?;
                let den = parts[1].parse::<f32>().ok()?;
                return Some(num / den);
            }
        }
        cr_str.parse().ok()
    }

    /// Extract level range
    fn extract_level_range(&self, query: &str) -> Option<(u32, u32)> {
        if let Some(caps) = self.level_pattern.captures(query) {
            // Range pattern: "level 1 to 5"
            if let (Some(min), Some(max)) = (caps.get(1), caps.get(2)) {
                let min_lvl = min.as_str().parse().ok()?;
                let max_lvl = max.as_str().parse().ok()?;
                return Some((min_lvl, max_lvl));
            }
            // Single value: "level 3"
            if let Some(val) = caps.get(3) {
                let lvl = val.as_str().parse().ok()?;
                return Some((lvl, lvl));
            }
        }
        None
    }

    /// Extract TTRPG attributes from query
    fn extract_attributes(&self, query: &str) -> Vec<RequiredAttribute> {
        let query_lower = query.to_lowercase();
        let mut attrs = Vec::new();
        let mut seen = HashSet::new();

        // Check damage types
        for damage in self.vocabulary.damage_types() {
            let pattern = format!(r"\b{}\b", regex::escape(damage));
            if let Ok(re) = Regex::new(&pattern) {
                if re.is_match(&query_lower) && !seen.contains(*damage) {
                    seen.insert(*damage);
                    attrs.push(RequiredAttribute {
                        category: "damage_type".to_string(),
                        value: damage.to_string(),
                        required: true,
                    });
                }
            }
        }

        // Check creature types
        for creature in self.vocabulary.creature_types() {
            let pattern = format!(r"\b{}\b", regex::escape(creature));
            if let Ok(re) = Regex::new(&pattern) {
                if re.is_match(&query_lower) && !seen.contains(*creature) {
                    seen.insert(*creature);
                    attrs.push(RequiredAttribute {
                        category: "creature_type".to_string(),
                        value: creature.to_string(),
                        required: true,
                    });
                }
            }
        }

        // Check conditions
        for condition in self.vocabulary.conditions() {
            let pattern = format!(r"\b{}\b", regex::escape(condition));
            if let Ok(re) = Regex::new(&pattern) {
                if re.is_match(&query_lower) && !seen.contains(*condition) {
                    seen.insert(*condition);
                    attrs.push(RequiredAttribute {
                        category: "condition".to_string(),
                        value: condition.to_string(),
                        required: true,
                    });
                }
            }
        }

        // Check sizes
        for size in self.vocabulary.sizes() {
            let pattern = format!(r"\b{}\b", regex::escape(size));
            if let Ok(re) = Regex::new(&pattern) {
                if re.is_match(&query_lower) && !seen.contains(*size) {
                    seen.insert(*size);
                    attrs.push(RequiredAttribute {
                        category: "size".to_string(),
                        value: size.to_string(),
                        required: true,
                    });
                }
            }
        }

        attrs
    }

    /// Build clean semantic query without constraints
    fn build_semantic_query(&self, query: &str) -> String {
        let mut result = query.to_string();

        // Remove negation patterns
        result = self.negation_pattern.replace_all(&result, "").to_string();

        // Remove CR patterns
        result = self.cr_pattern.replace_all(&result, "").to_string();

        // Remove level patterns
        result = self.level_pattern.replace_all(&result, "").to_string();

        // Remove quotes
        result = self.quoted_pattern.replace_all(&result, "$1").to_string();

        // Clean up whitespace
        result.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Build expanded query with antonym hints
    fn build_expanded_query(&self, constraints: &QueryConstraints) -> String {
        let mut expanded = constraints.semantic_query.clone();

        // Add antonym hints for better semantic search
        for attr in &constraints.required_attributes {
            if let Some(antonyms) = self.antonym_mapper.get_antonyms(&attr.value) {
                for ant in antonyms {
                    expanded.push_str(&format!(" (NOT {})", ant));
                }
            }
        }

        expanded
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_query() {
        let parser = QueryParser::new();
        let result = parser.parse("fire damage");

        assert!(!result.required_attributes.is_empty());
        assert!(result.required_attributes.iter().any(|a| a.value == "fire"));
    }

    #[test]
    fn test_parse_negations() {
        let parser = QueryParser::new();
        let result = parser.parse("creature not undead");

        assert!(result.excluded_attributes.contains(&"undead".to_string()));
    }

    #[test]
    fn test_parse_cr_range() {
        let parser = QueryParser::new();

        let result = parser.parse("monsters cr 1 to 5");
        assert_eq!(result.cr_range, Some((1.0, 5.0)));

        let result = parser.parse("cr 1/4");
        assert_eq!(result.cr_range, Some((0.25, 0.25)));
    }

    #[test]
    fn test_parse_level_range() {
        let parser = QueryParser::new();

        let result = parser.parse("spells level 3 to 5");
        assert_eq!(result.level_range, Some((3, 5)));

        let result = parser.parse("level 9 spell");
        assert_eq!(result.level_range, Some((9, 9)));
    }

    #[test]
    fn test_parse_quoted_strings() {
        let parser = QueryParser::new();
        let result = parser.parse(r#"find "Goblin King" monster"#);

        assert!(result.exact_match_entities.contains(&"Goblin King".to_string()));
    }

    #[test]
    fn test_semantic_query_cleaned() {
        let parser = QueryParser::new();
        let result = parser.parse("fire damage not cold cr 5");

        // Semantic query should have constraint patterns removed
        assert!(!result.semantic_query.contains("cr 5"));
        assert!(!result.semantic_query.contains("not cold"));
        assert!(result.semantic_query.contains("fire") || result.semantic_query.contains("damage"));
    }
}
