//! Dialect Data Models
//!
//! Defines data structures for dialect transformation systems that modify
//! NPC speech to match cultural or regional speech patterns.
//!
//! # Features
//!
//! - Phonetic substitutions (e.g., "th" -> "d" for certain accents)
//! - Grammatical transformations (e.g., double negatives, word order)
//! - Intensity levels (light, moderate, heavy)
//! - Cached regex patterns for performance
//!
//! # Architecture
//!
//! ```text
//! DialectDefinition
//!   +-- id: String
//!   +-- phonetic_rules: Vec<PhoneticRule>
//!   +-- grammatical_rules: Vec<GrammaticalRule>
//!   +-- vocabulary_replacements: HashMap<String, Vec<String>>
//!   +-- exclamation_templates: Vec<ExclamationTemplate>
//!   +-- intensity_config: IntensityConfig
//! ```

use once_cell::sync::Lazy;
use rand::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

use super::errors::DialectError;

// ============================================================================
// Intensity Levels
// ============================================================================

/// Intensity level for dialect application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Intensity {
    /// Subtle hints of dialect (10-30% of rules applied)
    Light,
    /// Noticeable but readable dialect (40-60% of rules applied)
    #[default]
    Moderate,
    /// Heavy dialect that may require effort to read (70-90% of rules applied)
    Heavy,
}

impl Intensity {
    /// Get the application probability for this intensity level.
    pub fn probability(&self) -> f32 {
        match self {
            Self::Light => 0.2,
            Self::Moderate => 0.5,
            Self::Heavy => 0.8,
        }
    }

    /// Parse intensity from a string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "light" | "subtle" | "mild" => Self::Light,
            "heavy" | "thick" | "strong" => Self::Heavy,
            _ => Self::Moderate,
        }
    }
}

// ============================================================================
// Phonetic Rules
// ============================================================================

/// A phonetic substitution rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneticRule {
    /// Rule identifier (for debugging and caching)
    pub id: String,

    /// Pattern to match (can be regex or literal)
    pub from: String,

    /// Replacement string
    pub to: String,

    /// Probability of applying this rule (0.0-1.0)
    #[serde(default = "default_frequency")]
    pub frequency: f32,

    /// Only apply at word boundaries
    #[serde(default)]
    pub word_boundary: bool,

    /// Only apply at start of words
    #[serde(default)]
    pub word_start: bool,

    /// Only apply at end of words
    #[serde(default)]
    pub word_end: bool,

    /// Case-sensitive matching
    #[serde(default)]
    pub case_sensitive: bool,

    /// Description of this rule
    #[serde(default)]
    pub description: String,
}

fn default_frequency() -> f32 {
    1.0
}

impl PhoneticRule {
    /// Create a new phonetic rule.
    pub fn new(id: impl Into<String>, from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            frequency: 1.0,
            word_boundary: false,
            word_start: false,
            word_end: false,
            case_sensitive: false,
            description: String::new(),
        }
    }

    /// Set the frequency for this rule.
    pub fn with_frequency(mut self, frequency: f32) -> Self {
        self.frequency = frequency.clamp(0.0, 1.0);
        self
    }

    /// Set word boundary matching.
    pub fn at_word_boundary(mut self) -> Self {
        self.word_boundary = true;
        self
    }

    /// Set word start matching.
    pub fn at_word_start(mut self) -> Self {
        self.word_start = true;
        self
    }

    /// Set word end matching.
    pub fn at_word_end(mut self) -> Self {
        self.word_end = true;
        self
    }

    /// Build the regex pattern for this rule.
    pub fn build_pattern(&self) -> Result<Regex, DialectError> {
        let mut pattern = regex::escape(&self.from);

        if self.word_start {
            pattern = format!(r"\b{}", pattern);
        }
        if self.word_end {
            pattern = format!(r"{}\b", pattern);
        }
        if self.word_boundary && !self.word_start && !self.word_end {
            pattern = format!(r"\b{}\b", pattern);
        }

        let flags = if self.case_sensitive { "" } else { "(?i)" };
        let full_pattern = format!("{}{}", flags, pattern);

        Regex::new(&full_pattern).map_err(|e| DialectError::invalid_regex("unknown", &self.id, e))
    }

    /// Check if this rule should be applied based on frequency and intensity.
    pub fn should_apply(&self, intensity: Intensity, rng: &mut impl Rng) -> bool {
        let effective_probability = self.frequency * intensity.probability();
        rng.gen::<f32>() < effective_probability
    }
}

impl Default for PhoneticRule {
    fn default() -> Self {
        Self {
            id: String::new(),
            from: String::new(),
            to: String::new(),
            frequency: 1.0,
            word_boundary: false,
            word_start: false,
            word_end: false,
            case_sensitive: false,
            description: String::new(),
        }
    }
}

// ============================================================================
// Grammatical Rules
// ============================================================================

/// A grammatical transformation rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammaticalRule {
    /// Rule identifier
    pub id: String,

    /// Regex pattern to match
    pub pattern: String,

    /// Replacement string (may include capture groups like $1, $2)
    pub replacement: String,

    /// Probability of applying this rule (0.0-1.0)
    #[serde(default = "default_frequency")]
    pub frequency: f32,

    /// Description of what this rule does
    #[serde(default)]
    pub description: String,

    /// Order of application (lower = earlier)
    #[serde(default)]
    pub order: i32,
}

impl GrammaticalRule {
    /// Create a new grammatical rule.
    pub fn new(
        id: impl Into<String>,
        pattern: impl Into<String>,
        replacement: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            pattern: pattern.into(),
            replacement: replacement.into(),
            frequency: 1.0,
            description: String::new(),
            order: 0,
        }
    }

    /// Set the frequency for this rule.
    pub fn with_frequency(mut self, frequency: f32) -> Self {
        self.frequency = frequency.clamp(0.0, 1.0);
        self
    }

    /// Set the order of application.
    pub fn with_order(mut self, order: i32) -> Self {
        self.order = order;
        self
    }

    /// Build the regex pattern for this rule.
    pub fn build_pattern(&self) -> Result<Regex, DialectError> {
        Regex::new(&self.pattern).map_err(|e| DialectError::invalid_regex("unknown", &self.id, e))
    }

    /// Check if this rule should be applied based on frequency and intensity.
    pub fn should_apply(&self, intensity: Intensity, rng: &mut impl Rng) -> bool {
        let effective_probability = self.frequency * intensity.probability();
        rng.gen::<f32>() < effective_probability
    }
}

impl Default for GrammaticalRule {
    fn default() -> Self {
        Self {
            id: String::new(),
            pattern: String::new(),
            replacement: String::new(),
            frequency: 1.0,
            description: String::new(),
            order: 0,
        }
    }
}

// ============================================================================
// Exclamation Templates
// ============================================================================

/// A template for culturally-specific exclamations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExclamationTemplate {
    /// Template text (may contain {placeholders})
    pub template: String,

    /// Emotion this exclamation conveys
    #[serde(default)]
    pub emotion: Emotion,

    /// Intensity of the exclamation
    #[serde(default)]
    pub intensity: ExclamationIntensity,

    /// Usage frequency weight
    #[serde(default = "default_frequency")]
    pub frequency: f32,

    /// Whether this template references deities/religion
    #[serde(default)]
    pub religious: bool,

    /// Available placeholder values
    #[serde(default)]
    pub placeholders: HashMap<String, Vec<String>>,
}

impl ExclamationTemplate {
    /// Create a new exclamation template.
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
            emotion: Emotion::Surprise,
            intensity: ExclamationIntensity::Moderate,
            frequency: 1.0,
            religious: false,
            placeholders: HashMap::new(),
        }
    }

    /// Set the emotion for this template.
    pub fn with_emotion(mut self, emotion: Emotion) -> Self {
        self.emotion = emotion;
        self
    }

    /// Set the intensity for this template.
    pub fn with_intensity(mut self, intensity: ExclamationIntensity) -> Self {
        self.intensity = intensity;
        self
    }

    /// Mark this template as religious.
    pub fn religious(mut self) -> Self {
        self.religious = true;
        self
    }

    /// Add placeholder values.
    pub fn with_placeholder(mut self, key: impl Into<String>, values: Vec<String>) -> Self {
        self.placeholders.insert(key.into(), values);
        self
    }

    /// Expand the template with placeholder values.
    pub fn expand(&self, rng: &mut impl Rng) -> String {
        let mut result = self.template.clone();

        for (key, values) in &self.placeholders {
            if let Some(value) = values.choose(rng) {
                let placeholder = format!("{{{}}}", key);
                result = result.replace(&placeholder, value);
            }
        }

        result
    }
}

/// Emotion conveyed by an exclamation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Emotion {
    #[default]
    Surprise,
    Anger,
    Joy,
    Fear,
    Disgust,
    Sadness,
    Frustration,
    Awe,
    Pain,
    Relief,
}

/// Intensity of an exclamation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExclamationIntensity {
    Mild,
    #[default]
    Moderate,
    Strong,
}

// ============================================================================
// Dialect Definition
// ============================================================================

/// Complete definition of a dialect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialectDefinition {
    /// Unique identifier for this dialect
    pub id: String,

    /// Human-readable name
    #[serde(default)]
    pub name: String,

    /// Description of this dialect
    #[serde(default)]
    pub description: String,

    /// Culture(s) this dialect is associated with
    #[serde(default)]
    pub cultures: Vec<String>,

    /// Phonetic substitution rules
    #[serde(default)]
    pub phonetic_rules: Vec<PhoneticRule>,

    /// Grammatical transformation rules
    #[serde(default)]
    pub grammatical_rules: Vec<GrammaticalRule>,

    /// Word-level vocabulary replacements
    #[serde(default)]
    pub vocabulary_replacements: HashMap<String, Vec<String>>,

    /// Culturally-specific exclamation templates
    #[serde(default)]
    pub exclamation_templates: Vec<ExclamationTemplate>,

    /// Proverbs and sayings in this dialect
    #[serde(default)]
    pub proverbs: Vec<String>,

    /// Common interjections
    #[serde(default)]
    pub interjections: Vec<String>,

    /// Default intensity level
    #[serde(default)]
    pub default_intensity: Intensity,

    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl DialectDefinition {
    /// Create a new dialect definition.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: String::new(),
            description: String::new(),
            cultures: Vec::new(),
            phonetic_rules: Vec::new(),
            grammatical_rules: Vec::new(),
            vocabulary_replacements: HashMap::new(),
            exclamation_templates: Vec::new(),
            proverbs: Vec::new(),
            interjections: Vec::new(),
            default_intensity: Intensity::Moderate,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set the dialect name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add a culture association.
    pub fn add_culture(mut self, culture: impl Into<String>) -> Self {
        self.cultures.push(culture.into());
        self
    }

    /// Add a phonetic rule.
    pub fn add_phonetic_rule(mut self, rule: PhoneticRule) -> Self {
        self.phonetic_rules.push(rule);
        self
    }

    /// Add a grammatical rule.
    pub fn add_grammatical_rule(mut self, rule: GrammaticalRule) -> Self {
        self.grammatical_rules.push(rule);
        self
    }

    /// Add a vocabulary replacement.
    pub fn add_replacement(
        mut self,
        word: impl Into<String>,
        alternatives: Vec<String>,
    ) -> Self {
        self.vocabulary_replacements.insert(word.into(), alternatives);
        self
    }

    /// Validate the dialect definition.
    pub fn validate(&self) -> Result<(), DialectError> {
        if self.id.is_empty() {
            return Err(DialectError::TransformFailed {
                dialect_id: self.id.clone(),
                reason: "Dialect ID cannot be empty".to_string(),
            });
        }

        // Validate all phonetic rule patterns
        for rule in &self.phonetic_rules {
            rule.build_pattern()?;
        }

        // Validate all grammatical rule patterns
        for rule in &self.grammatical_rules {
            rule.build_pattern()?;
        }

        Ok(())
    }

    /// Check if this dialect is associated with a culture.
    pub fn matches_culture(&self, culture: &str) -> bool {
        self.cultures.iter().any(|c| c.eq_ignore_ascii_case(culture))
    }

    /// Get a random exclamation for the given emotion.
    pub fn get_exclamation(
        &self,
        emotion: Option<Emotion>,
        rng: &mut impl Rng,
    ) -> Option<String> {
        let filtered: Vec<_> = self
            .exclamation_templates
            .iter()
            .filter(|t| emotion.map(|e| t.emotion == e).unwrap_or(true))
            .collect();

        if filtered.is_empty() {
            return None;
        }

        // Weighted random selection
        let total_weight: f32 = filtered.iter().map(|t| t.frequency).sum();
        if total_weight <= 0.0 {
            return filtered.choose(rng).map(|t| t.expand(rng));
        }

        let mut choice = rng.gen::<f32>() * total_weight;
        for template in &filtered {
            choice -= template.frequency;
            if choice <= 0.0 {
                return Some(template.expand(rng));
            }
        }

        filtered.last().map(|t| t.expand(rng))
    }

    /// Get a random proverb.
    pub fn get_proverb(&self, rng: &mut impl Rng) -> Option<&str> {
        self.proverbs.choose(rng).map(|s| s.as_str())
    }

    /// Get a random interjection.
    pub fn get_interjection(&self, rng: &mut impl Rng) -> Option<&str> {
        self.interjections.choose(rng).map(|s| s.as_str())
    }
}

impl Default for DialectDefinition {
    fn default() -> Self {
        Self::new("default")
    }
}

// ============================================================================
// Dialect Transformer
// ============================================================================

/// Thread-safe regex pattern cache.
static PATTERN_CACHE: Lazy<RwLock<HashMap<String, Regex>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Transforms text according to dialect rules.
#[derive(Debug, Clone)]
pub struct DialectTransformer {
    dialect: DialectDefinition,
    intensity: Intensity,
}

impl DialectTransformer {
    /// Create a new transformer for a dialect.
    pub fn new(dialect: DialectDefinition) -> Self {
        let intensity = dialect.default_intensity;
        Self { dialect, intensity }
    }

    /// Set the intensity level.
    pub fn with_intensity(mut self, intensity: Intensity) -> Self {
        self.intensity = intensity;
        self
    }

    /// Transform text according to dialect rules.
    pub fn transform(&self, text: &str, rng: &mut impl Rng) -> DialectTransformResult {
        let mut result = text.to_string();
        let mut applied_rules = Vec::new();

        // Apply vocabulary replacements first
        result = self.apply_vocabulary_replacements(&result, rng);

        // Apply phonetic rules
        for rule in &self.dialect.phonetic_rules {
            if rule.should_apply(self.intensity, rng) {
                if let Ok(transformed) = self.apply_phonetic_rule(&result, rule) {
                    if transformed != result {
                        applied_rules.push(rule.id.clone());
                        result = transformed;
                    }
                }
            }
        }

        // Sort grammatical rules by order
        let mut sorted_rules: Vec<_> = self.dialect.grammatical_rules.iter().collect();
        sorted_rules.sort_by_key(|r| r.order);

        // Apply grammatical rules
        for rule in sorted_rules {
            if rule.should_apply(self.intensity, rng) {
                if let Ok(transformed) = self.apply_grammatical_rule(&result, rule) {
                    if transformed != result {
                        applied_rules.push(rule.id.clone());
                        result = transformed;
                    }
                }
            }
        }

        DialectTransformResult {
            original: text.to_string(),
            transformed: result,
            dialect_id: self.dialect.id.clone(),
            intensity: self.intensity,
            applied_rules,
        }
    }

    /// Apply a phonetic rule to the text.
    fn apply_phonetic_rule(&self, text: &str, rule: &PhoneticRule) -> Result<String, DialectError> {
        // Use composite key to avoid collisions across dialects
        let cache_key = format!("{}:{}", self.dialect.id, rule.id);
        let pattern = self.get_or_compile_pattern(&cache_key, || rule.build_pattern())?;
        Ok(pattern.replace_all(text, rule.to.as_str()).into_owned())
    }

    /// Apply a grammatical rule to the text.
    fn apply_grammatical_rule(
        &self,
        text: &str,
        rule: &GrammaticalRule,
    ) -> Result<String, DialectError> {
        // Use composite key to avoid collisions across dialects
        let cache_key = format!("{}:{}", self.dialect.id, rule.id);
        let pattern = self.get_or_compile_pattern(&cache_key, || rule.build_pattern())?;
        Ok(pattern
            .replace_all(text, rule.replacement.as_str())
            .into_owned())
    }

    /// Apply vocabulary replacements.
    fn apply_vocabulary_replacements(&self, text: &str, rng: &mut impl Rng) -> String {
        let mut result = text.to_string();

        for (word, alternatives) in &self.dialect.vocabulary_replacements {
            if let Some(replacement) = alternatives.choose(rng) {
                // Use the pattern cache for vocabulary replacements to improve performance
                let cache_key = format!("vocab:{}", word);
                if let Ok(re) = self.get_or_compile_pattern(&cache_key, || {
                    let pattern_str = format!(r"(?i)\b{}\b", regex::escape(word));
                    Regex::new(&pattern_str)
                        .map_err(|e| DialectError::invalid_regex(&self.dialect.id, &cache_key, e))
                }) {
                    result = re.replace_all(&result, replacement.as_str()).into_owned();
                }
            }
        }

        result
    }

    /// Get a compiled pattern from cache or compile it.
    fn get_or_compile_pattern<F>(
        &self,
        cache_key: &str,
        compile: F,
    ) -> Result<Regex, DialectError>
    where
        F: FnOnce() -> Result<Regex, DialectError>,
    {
        // Check read cache first
        if let Some(pattern) = PATTERN_CACHE.read().unwrap().get(cache_key) {
            return Ok(pattern.clone());
        }

        // Acquire write lock
        let mut cache = PATTERN_CACHE.write().unwrap();
        // Double-check in case another thread compiled it while waiting for write lock
        if let Some(pattern) = cache.get(cache_key) {
            return Ok(pattern.clone());
        }

        // Compile and cache
        let pattern = compile()?;
        cache.insert(cache_key.to_string(), pattern.clone());
        Ok(pattern)
    }

    /// Get the dialect definition.
    pub fn dialect(&self) -> &DialectDefinition {
        &self.dialect
    }
}

// ============================================================================
// Transform Result
// ============================================================================

/// Result of a dialect transformation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialectTransformResult {
    /// Original input text
    pub original: String,

    /// Transformed output text
    pub transformed: String,

    /// ID of the dialect used
    pub dialect_id: String,

    /// Intensity level used
    pub intensity: Intensity,

    /// IDs of rules that were applied
    pub applied_rules: Vec<String>,
}

impl DialectTransformResult {
    /// Check if any transformation was applied.
    pub fn was_transformed(&self) -> bool {
        self.original != self.transformed
    }

    /// Get the number of rules applied.
    pub fn rules_applied(&self) -> usize {
        self.applied_rules.len()
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Clear the pattern cache (for testing).
pub fn clear_pattern_cache() {
    let mut cache = PATTERN_CACHE.write().unwrap();
    cache.clear();
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intensity_probability() {
        assert!(Intensity::Light.probability() < Intensity::Moderate.probability());
        assert!(Intensity::Moderate.probability() < Intensity::Heavy.probability());
    }

    #[test]
    fn test_intensity_from_str() {
        assert_eq!(Intensity::from_str("light"), Intensity::Light);
        assert_eq!(Intensity::from_str("HEAVY"), Intensity::Heavy);
        assert_eq!(Intensity::from_str("unknown"), Intensity::Moderate);
    }

    #[test]
    fn test_phonetic_rule_creation() {
        let rule = PhoneticRule::new("th_to_d", "th", "d")
            .with_frequency(0.8)
            .at_word_start();

        assert_eq!(rule.id, "th_to_d");
        assert_eq!(rule.from, "th");
        assert_eq!(rule.to, "d");
        assert_eq!(rule.frequency, 0.8);
        assert!(rule.word_start);
        assert!(!rule.word_end);
    }

    #[test]
    fn test_phonetic_rule_pattern_building() {
        let rule = PhoneticRule::new("test", "th", "d").at_word_start();

        let pattern = rule.build_pattern().unwrap();
        assert!(pattern.is_match("the"));
        assert!(pattern.is_match("THE"));
        assert!(!pattern.is_match("bathe"));
    }

    #[test]
    fn test_phonetic_rule_word_end() {
        let rule = PhoneticRule::new("test", "ing", "in'").at_word_end();

        let pattern = rule.build_pattern().unwrap();
        assert!(pattern.is_match("walking"));
        assert!(!pattern.is_match("ingress"));
    }

    #[test]
    fn test_grammatical_rule_creation() {
        let rule = GrammaticalRule::new("double_neg", r"don't (\w+)", "don't $1 none")
            .with_frequency(0.6)
            .with_order(10);

        assert_eq!(rule.id, "double_neg");
        assert_eq!(rule.frequency, 0.6);
        assert_eq!(rule.order, 10);
    }

    #[test]
    fn test_grammatical_rule_pattern() {
        let rule = GrammaticalRule::new("test", r"(\w+)ing\b", "${1}in'");

        let pattern = rule.build_pattern().unwrap();
        let result = pattern.replace_all("walking and talking", "${1}in'");
        assert_eq!(result, "walkin' and talkin'");
    }

    #[test]
    fn test_exclamation_template() {
        let template = ExclamationTemplate::new("By {deity}'s beard!")
            .with_emotion(Emotion::Surprise)
            .with_placeholder("deity", vec!["Moradin".to_string(), "Odin".to_string()]);

        assert_eq!(template.emotion, Emotion::Surprise);
        assert!(!template.religious);

        let mut rng = rand::thread_rng();
        let expanded = template.expand(&mut rng);
        assert!(expanded.contains("Moradin") || expanded.contains("Odin"));
    }

    #[test]
    fn test_dialect_definition_creation() {
        let dialect = DialectDefinition::new("scottish")
            .with_name("Scottish Accent")
            .add_culture("human")
            .add_phonetic_rule(PhoneticRule::new("th_to_d", "th", "d"))
            .add_grammatical_rule(GrammaticalRule::new(
                "aye",
                r"\byes\b",
                "aye",
            ));

        assert_eq!(dialect.id, "scottish");
        assert_eq!(dialect.name, "Scottish Accent");
        assert!(dialect.matches_culture("human"));
        assert!(!dialect.matches_culture("elf"));
        assert_eq!(dialect.phonetic_rules.len(), 1);
        assert_eq!(dialect.grammatical_rules.len(), 1);
    }

    #[test]
    fn test_dialect_definition_validation() {
        let valid = DialectDefinition::new("valid");
        assert!(valid.validate().is_ok());

        let empty_id = DialectDefinition::new("");
        assert!(empty_id.validate().is_err());

        // Use grammatical rule for invalid regex test since phonetic rules escape their patterns
        let mut invalid_regex = DialectDefinition::new("invalid");
        invalid_regex.grammatical_rules.push(GrammaticalRule::new(
            "bad",
            r"(?P<unclosed",  // Invalid regex - named capture group is missing closing '>' and pattern
            "replacement",
        ));
        assert!(invalid_regex.validate().is_err());
    }

    #[test]
    fn test_dialect_transformer() {
        clear_pattern_cache();

        let dialect = DialectDefinition::new("test")
            .add_phonetic_rule(
                PhoneticRule::new("ing_to_in", "ing", "in'")
                    .at_word_end()
                    .with_frequency(1.0),
            )
            .add_replacement("yes", vec!["aye".to_string()]);

        let transformer = DialectTransformer::new(dialect).with_intensity(Intensity::Heavy);

        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let result = transformer.transform("I am walking and talking, yes!", &mut rng);

        // With heavy intensity and frequency 1.0, transformations should be applied
        assert!(result.transformed.contains("in'") || result.transformed.contains("aye"));
    }

    #[test]
    fn test_dialect_transform_result() {
        let result = DialectTransformResult {
            original: "hello".to_string(),
            transformed: "hallo".to_string(),
            dialect_id: "test".to_string(),
            intensity: Intensity::Moderate,
            applied_rules: vec!["rule1".to_string()],
        };

        assert!(result.was_transformed());
        assert_eq!(result.rules_applied(), 1);

        let unchanged = DialectTransformResult {
            original: "hello".to_string(),
            transformed: "hello".to_string(),
            dialect_id: "test".to_string(),
            intensity: Intensity::Light,
            applied_rules: vec![],
        };

        assert!(!unchanged.was_transformed());
    }

    #[test]
    fn test_vocabulary_replacements() {
        clear_pattern_cache();

        let dialect = DialectDefinition::new("test")
            .add_replacement("hello", vec!["'ello".to_string()])
            .add_replacement("friend", vec!["mate".to_string()]);

        let transformer = DialectTransformer::new(dialect);

        let mut rng = rand::thread_rng();
        let result = transformer.transform("Hello, friend!", &mut rng);

        // Vocabulary replacements should always apply
        assert!(result.transformed.contains("'ello") || result.transformed.contains("mate"));
    }

    #[test]
    fn test_yaml_roundtrip() {
        let dialect = DialectDefinition::new("test")
            .with_name("Test Dialect")
            .add_culture("human")
            .add_phonetic_rule(PhoneticRule::new("test_rule", "th", "d").with_frequency(0.5));

        let yaml = serde_yaml_ng::to_string(&dialect).unwrap();
        let parsed: DialectDefinition = serde_yaml_ng::from_str(&yaml).unwrap();

        assert_eq!(parsed.id, "test");
        assert_eq!(parsed.name, "Test Dialect");
        assert_eq!(parsed.phonetic_rules.len(), 1);
        assert_eq!(parsed.phonetic_rules[0].frequency, 0.5);
    }

    #[test]
    fn test_get_exclamation() {
        let dialect = DialectDefinition::new("test").add_culture("test");

        // Manually add exclamation templates since there's no builder for this
        let mut dialect = dialect;
        dialect.exclamation_templates.push(
            ExclamationTemplate::new("Blimey!")
                .with_emotion(Emotion::Surprise)
                .with_intensity(ExclamationIntensity::Moderate),
        );

        let mut rng = rand::thread_rng();

        let exclamation = dialect.get_exclamation(Some(Emotion::Surprise), &mut rng);
        assert!(exclamation.is_some());
        assert_eq!(exclamation.unwrap(), "Blimey!");

        // No matching emotion
        let no_match = dialect.get_exclamation(Some(Emotion::Anger), &mut rng);
        assert!(no_match.is_none());
    }
}
