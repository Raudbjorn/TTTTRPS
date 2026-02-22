//! Attribute Extraction Module
//!
//! Extracts TTRPG-specific attributes from text with confidence scoring.
//! Supports damage types, creature types, conditions, alignments, and more.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::vocabulary::{GameVocabulary, DnD5eVocabulary};

// ============================================================================
// Types
// ============================================================================

/// Source of an attribute match, used for confidence scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttributeSource {
    /// Exact word-boundary match (highest confidence)
    ExactMatch,
    /// Pattern/regex match (medium-high confidence)
    PatternMatch,
    /// Inferred from context (lower confidence)
    Inferred,
    /// From parsed structured data (high confidence)
    StructuredData,
}

impl AttributeSource {
    /// Get the base confidence for this source type.
    pub fn base_confidence(&self) -> f32 {
        match self {
            Self::ExactMatch => 1.0,
            Self::StructuredData => 0.95,
            Self::PatternMatch => 0.8,
            Self::Inferred => 0.6,
        }
    }
}

/// A single attribute match with confidence information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeMatch {
    /// The matched/extracted value
    pub value: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// How the match was found
    pub source: AttributeSource,
    /// Position in source text (if available)
    pub position: Option<usize>,
}

impl AttributeMatch {
    /// Create a new attribute match.
    pub fn new(value: String, confidence: f32, source: AttributeSource) -> Self {
        Self {
            value,
            confidence,
            source,
            position: None,
        }
    }

    /// Create an exact match with full confidence.
    pub fn exact(value: String) -> Self {
        Self::new(value, 1.0, AttributeSource::ExactMatch)
    }

    /// Create a pattern match with medium-high confidence.
    pub fn pattern(value: String, confidence: f32) -> Self {
        Self::new(value, confidence, AttributeSource::PatternMatch)
    }
}

/// Extracted TTRPG attributes with confidence tracking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TTRPGAttributes {
    /// Damage types (fire, cold, etc.)
    pub damage_types: Vec<AttributeMatch>,
    /// Creature types (humanoid, undead, etc.)
    pub creature_types: Vec<AttributeMatch>,
    /// Conditions (poisoned, frightened, etc.)
    pub conditions: Vec<AttributeMatch>,
    /// Alignments (lawful good, etc.)
    pub alignments: Vec<AttributeMatch>,
    /// Rarities (common, rare, etc.)
    pub rarities: Vec<AttributeMatch>,
    /// Size categories (small, large, etc.)
    pub sizes: Vec<AttributeMatch>,
    /// Spell schools
    pub spell_schools: Vec<AttributeMatch>,
    /// Challenge Rating or Level
    pub cr_level: Option<AttributeMatch>,
    /// Named entities (spell names, creature names, etc.)
    pub named_entities: Vec<AttributeMatch>,
}

impl TTRPGAttributes {
    /// Get all damage types above a confidence threshold.
    pub fn confident_damage_types(&self, min_confidence: f32) -> Vec<&str> {
        self.damage_types
            .iter()
            .filter(|m| m.confidence >= min_confidence)
            .map(|m| m.value.as_str())
            .collect()
    }

    /// Get all creature types above a confidence threshold.
    pub fn confident_creature_types(&self, min_confidence: f32) -> Vec<&str> {
        self.creature_types
            .iter()
            .filter(|m| m.confidence >= min_confidence)
            .map(|m| m.value.as_str())
            .collect()
    }

    /// Check if any attributes were extracted.
    pub fn is_empty(&self) -> bool {
        self.damage_types.is_empty()
            && self.creature_types.is_empty()
            && self.conditions.is_empty()
            && self.alignments.is_empty()
            && self.rarities.is_empty()
            && self.sizes.is_empty()
            && self.spell_schools.is_empty()
            && self.cr_level.is_none()
            && self.named_entities.is_empty()
    }

    /// Convert to filterable fields for Meilisearch indexing.
    pub fn to_filterable_fields(&self) -> FilterableFields {
        FilterableFields {
            damage_types: self.damage_types.iter().map(|m| m.value.clone()).collect(),
            creature_types: self.creature_types.iter().map(|m| m.value.clone()).collect(),
            conditions: self.conditions.iter().map(|m| m.value.clone()).collect(),
            alignments: self.alignments.iter().map(|m| m.value.clone()).collect(),
            rarities: self.rarities.iter().map(|m| m.value.clone()).collect(),
            sizes: self.sizes.iter().map(|m| m.value.clone()).collect(),
            spell_schools: self.spell_schools.iter().map(|m| m.value.clone()).collect(),
            challenge_rating: self.cr_level.as_ref().and_then(|m| m.value.parse().ok()),
        }
    }
}

/// Flat structure for Meilisearch filterable fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilterableFields {
    pub damage_types: Vec<String>,
    pub creature_types: Vec<String>,
    pub conditions: Vec<String>,
    pub alignments: Vec<String>,
    pub rarities: Vec<String>,
    pub sizes: Vec<String>,
    pub spell_schools: Vec<String>,
    pub challenge_rating: Option<f32>,
}

// ============================================================================
// Extractor
// ============================================================================

/// Extracts TTRPG attributes from text content.
pub struct AttributeExtractor {
    vocabulary: Box<dyn GameVocabulary>,
    cr_pattern: Regex,
    level_pattern: Regex,
}

impl Default for AttributeExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl AttributeExtractor {
    /// Create a new attribute extractor with D&D 5e vocabulary.
    pub fn new() -> Self {
        Self::with_vocabulary(Box::new(DnD5eVocabulary))
    }

    /// Create an attribute extractor with custom vocabulary.
    pub fn with_vocabulary(vocabulary: Box<dyn GameVocabulary>) -> Self {
        Self {
            vocabulary,
            cr_pattern: Regex::new(r"(?i)\bchallenge\s+(?:rating\s+)?(\d+(?:/\d+)?)\b").unwrap(),
            level_pattern: Regex::new(r"(?i)\blevel\s+(\d+)\b").unwrap(),
        }
    }

    /// Extract all TTRPG attributes from text.
    pub fn extract(&self, text: &str) -> TTRPGAttributes {
        let text_lower = text.to_lowercase();
        let mut attrs = TTRPGAttributes::default();

        // Extract each category
        attrs.damage_types = self.extract_from_list(&text_lower, self.vocabulary.damage_types());
        attrs.creature_types = self.extract_from_list(&text_lower, self.vocabulary.creature_types());
        attrs.conditions = self.extract_from_list(&text_lower, self.vocabulary.conditions());
        attrs.alignments = self.extract_alignments(&text_lower);
        attrs.rarities = self.extract_from_list(&text_lower, self.vocabulary.rarities());
        attrs.sizes = self.extract_from_list(&text_lower, self.vocabulary.sizes());
        attrs.spell_schools = self.extract_from_list(&text_lower, self.vocabulary.spell_schools());
        attrs.cr_level = self.extract_cr_level(&text_lower);

        attrs
    }

    /// Extract terms from a vocabulary list with word-boundary matching.
    fn extract_from_list(&self, text: &str, terms: &[&str]) -> Vec<AttributeMatch> {
        let mut matches = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for term in terms {
            if seen.contains(*term) {
                continue;
            }

            // Word-boundary matching
            let pattern = format!(r"\b{}\b", regex::escape(term));
            if let Ok(re) = Regex::new(&pattern) {
                if let Some(m) = re.find(text) {
                    seen.insert(*term);
                    let mut attr_match = AttributeMatch::exact(term.to_string());
                    attr_match.position = Some(m.start());
                    matches.push(attr_match);
                }
            }
        }

        matches
    }

    /// Extract alignments with special handling for compound alignments.
    fn extract_alignments(&self, text: &str) -> Vec<AttributeMatch> {
        let mut matches = Vec::new();

        // Check for full alignment strings first
        for alignment in self.vocabulary.alignments() {
            let pattern = format!(r"\b{}\b", regex::escape(alignment));
            if let Ok(re) = Regex::new(&pattern) {
                if re.is_match(text) {
                    matches.push(AttributeMatch::exact(alignment.to_string()));
                }
            }
        }

        // Check for alignment components (lawful/chaotic/good/evil)
        let components = ["lawful", "chaotic", "neutral", "good", "evil"];
        for comp in components {
            let pattern = format!(r"\b{}\b", comp);
            if let Ok(re) = Regex::new(&pattern) {
                if re.is_match(text) && !matches.iter().any(|m| m.value.contains(comp)) {
                    matches.push(AttributeMatch::pattern(comp.to_string(), 0.7));
                }
            }
        }

        matches
    }

    /// Extract Challenge Rating or Level.
    fn extract_cr_level(&self, text: &str) -> Option<AttributeMatch> {
        // Try CR first
        if let Some(caps) = self.cr_pattern.captures(text) {
            let cr_str = caps.get(1).unwrap().as_str();
            let cr_value = if cr_str.contains('/') {
                // Handle fractional CR (1/4, 1/2, etc.)
                cr_str.to_string()
            } else {
                cr_str.to_string()
            };
            return Some(AttributeMatch::exact(cr_value));
        }

        // Try level
        if let Some(caps) = self.level_pattern.captures(text) {
            let level = caps.get(1).unwrap().as_str().to_string();
            return Some(AttributeMatch::pattern(level, 0.8));
        }

        None
    }

    /// Get antonyms for extracted attributes.
    pub fn get_antonyms(&self, attrs: &TTRPGAttributes) -> HashMap<String, Vec<String>> {
        let mut antonyms = HashMap::new();
        let pairs = self.vocabulary.antonym_pairs();

        // Check damage types
        for attr in &attrs.damage_types {
            for (a, b) in pairs {
                if attr.value == *a {
                    antonyms.entry(attr.value.clone())
                        .or_insert_with(Vec::new)
                        .push(b.to_string());
                } else if attr.value == *b {
                    antonyms.entry(attr.value.clone())
                        .or_insert_with(Vec::new)
                        .push(a.to_string());
                }
            }
        }

        // Check alignments
        for attr in &attrs.alignments {
            for (a, b) in pairs {
                if attr.value.contains(a) {
                    antonyms.entry(attr.value.clone())
                        .or_insert_with(Vec::new)
                        .push(b.to_string());
                } else if attr.value.contains(b) {
                    antonyms.entry(attr.value.clone())
                        .or_insert_with(Vec::new)
                        .push(a.to_string());
                }
            }
        }

        antonyms
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_damage_types() {
        let extractor = AttributeExtractor::new();
        let text = "This creature deals fire damage and is resistant to cold damage.";

        let attrs = extractor.extract(text);

        assert!(!attrs.damage_types.is_empty());
        let types: Vec<&str> = attrs.damage_types.iter().map(|m| m.value.as_str()).collect();
        assert!(types.contains(&"fire"));
        assert!(types.contains(&"cold"));
    }

    #[test]
    fn test_extract_creature_types() {
        let extractor = AttributeExtractor::new();
        let text = "The undead creature approaches. It is clearly a humanoid in origin.";

        let attrs = extractor.extract(text);

        let types: Vec<&str> = attrs.creature_types.iter().map(|m| m.value.as_str()).collect();
        assert!(types.contains(&"undead"));
        assert!(types.contains(&"humanoid"));
    }

    #[test]
    fn test_extract_cr() {
        let extractor = AttributeExtractor::new();

        let text1 = "Challenge 5 (1,800 XP)";
        let attrs1 = extractor.extract(text1);
        assert_eq!(attrs1.cr_level.as_ref().map(|m| m.value.as_str()), Some("5"));

        let text2 = "Challenge Rating 1/4";
        let attrs2 = extractor.extract(text2);
        assert_eq!(attrs2.cr_level.as_ref().map(|m| m.value.as_str()), Some("1/4"));
    }

    #[test]
    fn test_extract_alignments() {
        let extractor = AttributeExtractor::new();
        let text = "The paladin, a paragon of lawful good, faces the neutral evil lich.";

        let attrs = extractor.extract(text);

        let alignments: Vec<&str> = attrs.alignments.iter().map(|m| m.value.as_str()).collect();
        assert!(alignments.contains(&"lawful good"));
        assert!(alignments.contains(&"neutral evil"));
    }

    #[test]
    fn test_confident_damage_types() {
        let extractor = AttributeExtractor::new();
        let text = "Deals fire damage.";

        let attrs = extractor.extract(text);

        let confident = attrs.confident_damage_types(0.9);
        assert!(confident.contains(&"fire"));
    }

    #[test]
    fn test_get_antonyms() {
        let extractor = AttributeExtractor::new();
        let text = "This spell deals fire damage.";

        let attrs = extractor.extract(text);
        let antonyms = extractor.get_antonyms(&attrs);

        assert!(antonyms.get("fire").map(|v| v.contains(&"cold".to_string())).unwrap_or(false));
    }

    #[test]
    fn test_filterable_fields() {
        let extractor = AttributeExtractor::new();
        let text = "A small undead creature with fire resistance. Challenge 3.";

        let attrs = extractor.extract(text);
        let fields = attrs.to_filterable_fields();

        assert!(fields.creature_types.contains(&"undead".to_string()));
        assert!(fields.sizes.contains(&"small".to_string()));
        assert!(fields.damage_types.contains(&"fire".to_string()));
    }
}
