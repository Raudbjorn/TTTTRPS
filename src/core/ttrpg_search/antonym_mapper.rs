//! Antonym Mapper Module
//!
//! Maps semantic opposites for penalty scoring in search results.

use std::collections::HashMap;

use crate::ingestion::ttrpg::GameVocabulary;

// ============================================================================
// Antonym Mapper
// ============================================================================

/// Maps TTRPG antonyms for penalty scoring
#[derive(Debug, Clone)]
pub struct AntonymMapper {
    /// Maps attribute to its antonyms
    antonyms: HashMap<String, Vec<String>>,
}

impl Default for AntonymMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl AntonymMapper {
    /// Create a new antonym mapper with default pairs
    pub fn new() -> Self {
        let mut mapper = Self {
            antonyms: HashMap::new(),
        };

        // Add default antonym pairs
        mapper.add_pair("fire", "cold");
        mapper.add_pair("radiant", "necrotic");
        mapper.add_pair("lawful", "chaotic");
        mapper.add_pair("good", "evil");
        mapper.add_pair("light", "darkness");
        mapper.add_pair("positive", "negative");

        // Loose associations
        mapper.add_pair("lightning", "thunder");
        mapper.add_pair("acid", "poison"); // loosely related

        mapper
    }

    /// Create from a GameVocabulary's antonym pairs
    pub fn from_vocabulary(vocab: &dyn GameVocabulary) -> Self {
        let mut mapper = Self {
            antonyms: HashMap::new(),
        };

        for (a, b) in vocab.antonym_pairs() {
            mapper.add_pair(a, b);
        }

        mapper
    }

    /// Add a bidirectional antonym pair
    pub fn add_pair(&mut self, a: &str, b: &str) {
        self.antonyms
            .entry(a.to_lowercase())
            .or_default()
            .push(b.to_lowercase());

        self.antonyms
            .entry(b.to_lowercase())
            .or_default()
            .push(a.to_lowercase());
    }

    /// Get antonyms for an attribute
    pub fn get_antonyms(&self, attr: &str) -> Option<&Vec<String>> {
        self.antonyms.get(&attr.to_lowercase())
    }

    /// Check if two attributes are antonyms
    pub fn are_antonyms(&self, a: &str, b: &str) -> bool {
        let a_lower = a.to_lowercase();
        let b_lower = b.to_lowercase();

        if let Some(antonyms) = self.antonyms.get(&a_lower) {
            return antonyms.contains(&b_lower);
        }
        false
    }

    /// Calculate penalty multiplier for antonym presence
    ///
    /// # Returns
    /// * 1.0 = no penalty (no antonyms found)
    /// * 0.5 = moderate penalty (one antonym present)
    /// * 0.1 = heavy penalty (multiple antonyms present)
    pub fn calculate_penalty(
        &self,
        query_attrs: &[String],
        result_attrs: &[String],
    ) -> f32 {
        let mut antonym_count = 0;

        for query_attr in query_attrs {
            if let Some(antonyms) = self.get_antonyms(query_attr) {
                for ant in antonyms {
                    if result_attrs.iter().any(|r| r.to_lowercase() == *ant) {
                        antonym_count += 1;
                    }
                }
            }
        }

        match antonym_count {
            0 => 1.0,      // No penalty
            1 => 0.5,      // Moderate penalty
            2 => 0.25,     // Heavy penalty
            _ => 0.1,      // Very heavy penalty
        }
    }

    /// Check if any result attributes conflict with excluded attributes
    pub fn has_excluded(
        &self,
        excluded: &[String],
        result_attrs: &[String],
    ) -> bool {
        for excl in excluded {
            let excl_lower = excl.to_lowercase();
            if result_attrs.iter().any(|r| r.to_lowercase() == excl_lower) {
                return true;
            }
        }
        false
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_antonym_pairs() {
        let mapper = AntonymMapper::new();

        assert!(mapper.are_antonyms("fire", "cold"));
        assert!(mapper.are_antonyms("cold", "fire"));
        assert!(mapper.are_antonyms("radiant", "necrotic"));
        assert!(mapper.are_antonyms("lawful", "chaotic"));
    }

    #[test]
    fn test_get_antonyms() {
        let mapper = AntonymMapper::new();

        let antonyms = mapper.get_antonyms("fire").unwrap();
        assert!(antonyms.contains(&"cold".to_string()));
    }

    #[test]
    fn test_not_antonyms() {
        let mapper = AntonymMapper::new();

        assert!(!mapper.are_antonyms("fire", "fire"));
        assert!(!mapper.are_antonyms("fire", "radiant"));
    }

    #[test]
    fn test_calculate_penalty_no_conflict() {
        let mapper = AntonymMapper::new();

        let query = vec!["fire".to_string()];
        let result = vec!["fire".to_string(), "bludgeoning".to_string()];

        assert_eq!(mapper.calculate_penalty(&query, &result), 1.0);
    }

    #[test]
    fn test_calculate_penalty_one_conflict() {
        let mapper = AntonymMapper::new();

        let query = vec!["fire".to_string()];
        let result = vec!["cold".to_string()];

        assert_eq!(mapper.calculate_penalty(&query, &result), 0.5);
    }

    #[test]
    fn test_calculate_penalty_multiple_conflicts() {
        let mapper = AntonymMapper::new();

        let query = vec!["fire".to_string(), "radiant".to_string()];
        let result = vec!["cold".to_string(), "necrotic".to_string()];

        assert_eq!(mapper.calculate_penalty(&query, &result), 0.25);
    }

    #[test]
    fn test_has_excluded() {
        let mapper = AntonymMapper::new();

        let excluded = vec!["undead".to_string()];
        let result = vec!["undead".to_string(), "humanoid".to_string()];

        assert!(mapper.has_excluded(&excluded, &result));

        let result_ok = vec!["humanoid".to_string()];
        assert!(!mapper.has_excluded(&excluded, &result_ok));
    }

    #[test]
    fn test_case_insensitive() {
        let mapper = AntonymMapper::new();

        assert!(mapper.are_antonyms("FIRE", "cold"));
        assert!(mapper.are_antonyms("fire", "COLD"));
    }
}
