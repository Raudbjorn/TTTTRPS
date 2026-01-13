//! TTRPG Element Classification Module
//!
//! Classifies extracted text content into TTRPG-specific element types
//! (stat blocks, random tables, read-aloud text, etc.) using pattern matching.

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::stat_block::StatBlockParser;
use super::random_table::RandomTableParser;

// ============================================================================
// Types
// ============================================================================

/// TTRPG element types that require special handling during processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TTRPGElementType {
    /// Creature or NPC stat block with game statistics
    StatBlock,
    /// Random/roll table with dice notation
    RandomTable,
    /// Boxed read-aloud text for GMs
    ReadAloudText,
    /// Sidebar with optional rules, tips, or variants
    Sidebar,
    /// Spell description with casting time, range, etc.
    SpellDescription,
    /// Magic item or equipment description
    ItemDescription,
    /// Class feature or ability description
    ClassFeature,
    /// Section header (various levels)
    SectionHeader,
    /// Generic body text
    GenericText,
}

impl TTRPGElementType {
    /// Get a human-readable name for this element type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StatBlock => "stat_block",
            Self::RandomTable => "random_table",
            Self::ReadAloudText => "read_aloud",
            Self::Sidebar => "sidebar",
            Self::SpellDescription => "spell",
            Self::ItemDescription => "item",
            Self::ClassFeature => "class_feature",
            Self::SectionHeader => "header",
            Self::GenericText => "text",
        }
    }

    /// Check if this element type should be kept atomic (not split).
    pub fn is_atomic(&self) -> bool {
        matches!(
            self,
            Self::StatBlock | Self::RandomTable | Self::SpellDescription | Self::ItemDescription
        )
    }
}

/// Classification result with confidence score and optional structured data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedElement {
    /// The detected element type
    pub element_type: TTRPGElementType,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// The raw text content
    pub content: String,
    /// Page number where this element was found
    pub page_number: u32,
    /// Optional parsed structured data (for stat blocks, tables, etc.)
    pub structured_data: Option<serde_json::Value>,
    /// Start position in source text (character offset)
    pub start_offset: Option<usize>,
    /// End position in source text (character offset)
    pub end_offset: Option<usize>,
}

impl ClassifiedElement {
    /// Create a new classified element.
    pub fn new(
        element_type: TTRPGElementType,
        confidence: f32,
        content: String,
        page_number: u32,
    ) -> Self {
        Self {
            element_type,
            confidence,
            content,
            page_number,
            structured_data: None,
            start_offset: None,
            end_offset: None,
        }
    }

    /// Set structured data for this element.
    pub fn with_structured_data(mut self, data: serde_json::Value) -> Self {
        self.structured_data = Some(data);
        self
    }

    /// Set the text offset range.
    pub fn with_offsets(mut self, start: usize, end: usize) -> Self {
        self.start_offset = Some(start);
        self.end_offset = Some(end);
        self
    }
}

// ============================================================================
// Classifier
// ============================================================================

/// Classifies text content into TTRPG element types.
pub struct TTRPGClassifier {
    /// Minimum confidence to classify (below falls back to GenericText)
    min_confidence: f32,
    /// Stat block parser for structured extraction
    stat_block_parser: StatBlockParser,
    /// Random table parser for structured extraction
    table_parser: RandomTableParser,
    /// Compiled regex patterns
    patterns: ClassifierPatterns,
}

struct ClassifierPatterns {
    stat_block_indicators: Vec<(&'static str, f32)>,
    spell_pattern: Regex,
    item_pattern: Regex,
    class_feature_pattern: Regex,
    read_aloud_patterns: Vec<&'static str>,
    sidebar_patterns: Vec<&'static str>,
}

impl Default for TTRPGClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl TTRPGClassifier {
    /// Create a new classifier with default settings.
    pub fn new() -> Self {
        Self::with_min_confidence(0.6)
    }

    /// Create a classifier with custom minimum confidence threshold.
    pub fn with_min_confidence(min_confidence: f32) -> Self {
        Self {
            min_confidence,
            stat_block_parser: StatBlockParser::new(),
            table_parser: RandomTableParser::new(),
            patterns: ClassifierPatterns {
                stat_block_indicators: vec![
                    ("armor class", 1.0),
                    ("hit points", 1.0),
                    ("challenge rating", 0.8),
                    ("challenge", 0.5),
                    ("str dex con int wis cha", 1.5),
                    ("saving throws", 0.5),
                    ("damage resistances", 0.6),
                    ("damage immunities", 0.6),
                    ("condition immunities", 0.6),
                    ("senses", 0.3),
                    ("languages", 0.3),
                    ("legendary actions", 0.8),
                ],
                spell_pattern: Regex::new(
                    r"(?i)^(\w[\w\s]+)\n((\d+)(?:st|nd|rd|th)[- ]level|cantrip)\s+(abjuration|conjuration|divination|enchantment|evocation|illusion|necromancy|transmutation)"
                ).unwrap(),
                item_pattern: Regex::new(
                    r"(?i)(wondrous item|armor|weapon|ring|rod|staff|wand|potion|scroll),?\s*(common|uncommon|rare|very rare|legendary|artifact)?"
                ).unwrap(),
                class_feature_pattern: Regex::new(
                    r"(?i)^(starting at|at \d+(st|nd|rd|th) level|beginning at|when you reach)"
                ).unwrap(),
                read_aloud_patterns: vec![
                    "read aloud", "boxed text", "read this", "read the following",
                ],
                sidebar_patterns: vec![
                    "sidebar", "variant:", "note:", "tip:", "optional rule:",
                    "variant rule:", "dm tip:", "gm tip:",
                ],
            },
        }
    }

    /// Classify a single text block.
    ///
    /// # Arguments
    /// * `text` - The text content to classify
    /// * `page_number` - The page number for attribution
    ///
    /// # Returns
    /// A classified element with type, confidence, and optional structured data
    pub fn classify(&self, text: &str, page_number: u32) -> ClassifiedElement {
        let text_trimmed = text.trim();
        let text_lower = text.to_lowercase();

        // Skip very short text, but allow potential headers
        if text_trimmed.len() < 20 {
            // Check if it might be a header first
            let header_result = self.classify_header(text_trimmed, page_number);
            if header_result.confidence >= self.min_confidence {
                return header_result;
            }
            return ClassifiedElement::new(
                TTRPGElementType::GenericText,
                1.0,
                text_trimmed.to_string(),
                page_number,
            );
        }

        // Try each classifier in order of specificity
        let candidates = vec![
            self.classify_stat_block(text_trimmed, &text_lower, page_number),
            self.classify_random_table(text_trimmed, page_number),
            self.classify_spell(text_trimmed, page_number),
            self.classify_item(text_trimmed, &text_lower, page_number),
            self.classify_class_feature(text_trimmed, &text_lower, page_number),
            self.classify_read_aloud(text_trimmed, &text_lower, page_number),
            self.classify_sidebar(text_trimmed, &text_lower, page_number),
            self.classify_header(text_trimmed, page_number),
        ];

        // Return highest confidence match above threshold
        candidates
            .into_iter()
            .filter(|c| c.confidence >= self.min_confidence)
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .unwrap_or_else(|| {
                ClassifiedElement::new(
                    TTRPGElementType::GenericText,
                    1.0,
                    text_trimmed.to_string(),
                    page_number,
                )
            })
    }

    /// Classify multiple text blocks from a document.
    pub fn classify_document(
        &self,
        paragraphs: &[(u32, String)],
    ) -> Vec<ClassifiedElement> {
        paragraphs
            .iter()
            .map(|(page, text)| self.classify(text, *page))
            .collect()
    }

    fn classify_stat_block(
        &self,
        text: &str,
        text_lower: &str,
        page_number: u32,
    ) -> ClassifiedElement {
        let mut score: f32 = 0.0;
        let max_score: f32 = 6.0;

        for (indicator, weight) in &self.patterns.stat_block_indicators {
            if text_lower.contains(indicator) {
                score += weight;
            }
        }

        // Check for ability score pattern (STR 18 (+4) or similar)
        let ability_pattern = Regex::new(
            r"(?i)(str|dex|con|int|wis|cha)\s+\d+\s*\([+-]?\d+\)"
        ).unwrap();
        if ability_pattern.is_match(text) {
            score += 1.5;
        }

        let confidence = (score / max_score).min(1.0);

        let mut element = ClassifiedElement::new(
            TTRPGElementType::StatBlock,
            confidence,
            text.to_string(),
            page_number,
        );

        // Try to parse structured data if confidence is high enough
        if confidence >= 0.5 {
            if let Ok(stat_block) = self.stat_block_parser.parse(text) {
                element.structured_data = serde_json::to_value(&stat_block).ok();
            }
        }

        element
    }

    fn classify_random_table(&self, text: &str, page_number: u32) -> ClassifiedElement {
        // Check for dice notation and table structure
        if let Some(table_data) = self.table_parser.parse(text) {
            let confidence = if table_data.entries.len() >= 4 { 0.9 } else { 0.7 };

            let mut element = ClassifiedElement::new(
                TTRPGElementType::RandomTable,
                confidence,
                text.to_string(),
                page_number,
            );
            element.structured_data = serde_json::to_value(&table_data).ok();
            return element;
        }

        ClassifiedElement::new(
            TTRPGElementType::RandomTable,
            0.0,
            text.to_string(),
            page_number,
        )
    }

    fn classify_spell(&self, text: &str, page_number: u32) -> ClassifiedElement {
        let confidence = if self.patterns.spell_pattern.is_match(text) {
            0.9
        } else {
            0.0
        };

        ClassifiedElement::new(
            TTRPGElementType::SpellDescription,
            confidence,
            text.to_string(),
            page_number,
        )
    }

    fn classify_item(&self, text: &str, text_lower: &str, page_number: u32) -> ClassifiedElement {
        let mut score: f32 = 0.0;

        if self.patterns.item_pattern.is_match(text_lower) {
            score += 0.7;
        }

        // Magic item indicators
        let item_keywords = ["attunement", "charges", "regain", "expended"];
        for kw in item_keywords {
            if text_lower.contains(kw) {
                score += 0.15;
            }
        }

        ClassifiedElement::new(
            TTRPGElementType::ItemDescription,
            score.min(1.0),
            text.to_string(),
            page_number,
        )
    }

    fn classify_class_feature(
        &self,
        text: &str,
        text_lower: &str,
        page_number: u32,
    ) -> ClassifiedElement {
        let mut score: f32 = 0.0;

        if self.patterns.class_feature_pattern.is_match(text_lower) {
            score += 0.6;
        }

        // Level-based feature indicators
        if text_lower.contains("level") && text_lower.contains("gain") {
            score += 0.3;
        }

        ClassifiedElement::new(
            TTRPGElementType::ClassFeature,
            score.min(1.0),
            text.to_string(),
            page_number,
        )
    }

    fn classify_read_aloud(
        &self,
        text: &str,
        text_lower: &str,
        page_number: u32,
    ) -> ClassifiedElement {
        let mut score: f32 = 0.0;

        for pattern in &self.patterns.read_aloud_patterns {
            if text_lower.contains(pattern) {
                score += 0.7;
                break;
            }
        }

        // Second-person descriptive language
        let you_count = text_lower.matches(" you ").count()
            + text_lower.matches("you see").count()
            + text_lower.matches("you hear").count();
        if you_count >= 2 {
            score += 0.3;
        }

        ClassifiedElement::new(
            TTRPGElementType::ReadAloudText,
            score.min(1.0),
            text.to_string(),
            page_number,
        )
    }

    fn classify_sidebar(
        &self,
        text: &str,
        text_lower: &str,
        page_number: u32,
    ) -> ClassifiedElement {
        let mut score: f32 = 0.0;

        for pattern in &self.patterns.sidebar_patterns {
            if text_lower.starts_with(pattern) || text_lower.contains(&format!("\n{}", pattern)) {
                score += 0.7;
                break;
            }
        }

        ClassifiedElement::new(
            TTRPGElementType::Sidebar,
            score.min(1.0),
            text.to_string(),
            page_number,
        )
    }

    fn classify_header(&self, text: &str, page_number: u32) -> ClassifiedElement {
        let text_trimmed = text.trim();

        // Header heuristics
        if text_trimmed.len() > 100 {
            return ClassifiedElement::new(
                TTRPGElementType::SectionHeader,
                0.0,
                text_trimmed.to_string(),
                page_number,
            );
        }

        // Ends with punctuation = probably not a header
        if text_trimmed.ends_with('.') || text_trimmed.ends_with(',') {
            return ClassifiedElement::new(
                TTRPGElementType::SectionHeader,
                0.0,
                text_trimmed.to_string(),
                page_number,
            );
        }

        let mut score: f32 = 0.0;

        // All caps
        let letters: Vec<char> = text_trimmed.chars().filter(|c| c.is_alphabetic()).collect();
        if !letters.is_empty() && letters.iter().all(|c| c.is_uppercase()) {
            score += 0.7;
        }

        // Chapter/section patterns
        let lower = text_trimmed.to_lowercase();
        if lower.starts_with("chapter")
            || lower.starts_with("section")
            || lower.starts_with("part")
            || lower.starts_with("appendix")
        {
            score += 0.8;
        }

        ClassifiedElement::new(
            TTRPGElementType::SectionHeader,
            score.min(1.0),
            text_trimmed.to_string(),
            page_number,
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_block_classification() {
        let classifier = TTRPGClassifier::new();
        let stat_block = r#"
            Goblin
            Small humanoid, neutral evil
            Armor Class 15 (leather armor, shield)
            Hit Points 7 (2d6)
            Speed 30 ft.
            STR 8 (-1) DEX 14 (+2) CON 10 (+0) INT 10 (+0) WIS 8 (-1) CHA 8 (-1)
            Skills Stealth +6
            Senses darkvision 60 ft.
            Languages Common, Goblin
            Challenge 1/4
        "#;

        let result = classifier.classify(stat_block, 1);
        assert_eq!(result.element_type, TTRPGElementType::StatBlock);
        assert!(result.confidence >= 0.5);
    }

    #[test]
    fn test_header_classification() {
        let classifier = TTRPGClassifier::new();

        let result = classifier.classify("CHAPTER ONE", 1);
        assert_eq!(result.element_type, TTRPGElementType::SectionHeader);
        assert!(result.confidence >= 0.6);

        let result = classifier.classify("Chapter 1: The Beginning", 1);
        assert_eq!(result.element_type, TTRPGElementType::SectionHeader);
    }

    #[test]
    fn test_generic_text_fallback() {
        let classifier = TTRPGClassifier::new();
        let text = "This is just regular body text without any special indicators. It's a normal paragraph describing something in the game world.";

        let result = classifier.classify(text, 1);
        assert_eq!(result.element_type, TTRPGElementType::GenericText);
    }

    #[test]
    fn test_element_type_is_atomic() {
        assert!(TTRPGElementType::StatBlock.is_atomic());
        assert!(TTRPGElementType::RandomTable.is_atomic());
        assert!(!TTRPGElementType::GenericText.is_atomic());
        assert!(!TTRPGElementType::SectionHeader.is_atomic());
    }
}
