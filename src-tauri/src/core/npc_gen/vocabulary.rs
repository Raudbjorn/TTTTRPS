//! Vocabulary Bank Data Models
//!
//! Defines data structures for NPC vocabulary banks that store categorized
//! phrases for dynamic speech generation. Banks can be loaded from YAML files
//! and provide frequency-weighted phrase selection.
//!
//! # Architecture
//!
//! ```text
//! VocabularyBank
//!   +-- id: String
//!   +-- culture: Option<String>
//!   +-- greetings: PhraseCategory
//!   |     +-- formal: Vec<PhraseEntry>
//!   |     +-- casual: Vec<PhraseEntry>
//!   |     +-- hostile: Vec<PhraseEntry>
//!   +-- farewells: PhraseCategory
//!   +-- exclamations: Vec<PhraseEntry>
//!   +-- negotiation: Vec<PhraseEntry>
//!   +-- combat: Vec<PhraseEntry>
//!   +-- cultural_references: Vec<String>
//!   +-- speech_patterns: Vec<SpeechPattern>
//! ```

use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::errors::VocabularyError;

// ============================================================================
// Core Types
// ============================================================================

/// A single phrase entry with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhraseEntry {
    /// The phrase text (may contain {placeholders} for variable substitution)
    pub text: String,

    /// Usage frequency weight (0.0-1.0, higher = more common)
    /// Defaults to 1.0 if not specified
    #[serde(default = "default_frequency")]
    pub frequency: f32,

    /// Tags for additional categorization (e.g., "nervous", "confident", "drunk")
    #[serde(default)]
    pub tags: Vec<String>,

    /// Optional context hint for when to use this phrase
    #[serde(default)]
    pub context: Option<String>,
}

fn default_frequency() -> f32 {
    1.0
}

impl PhraseEntry {
    /// Create a new phrase entry with default frequency.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            frequency: 1.0,
            tags: Vec::new(),
            context: None,
        }
    }

    /// Create a phrase entry with custom frequency.
    pub fn with_frequency(text: impl Into<String>, frequency: f32) -> Self {
        Self {
            text: text.into(),
            frequency: frequency.clamp(0.0, 1.0),
            tags: Vec::new(),
            context: None,
        }
    }

    /// Add tags to this phrase entry.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Add context hint to this phrase entry.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Check if this phrase has a specific tag.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t.eq_ignore_ascii_case(tag))
    }

    /// Substitute placeholders in the phrase text.
    ///
    /// Placeholders are in the format `{name}` and are replaced with
    /// values from the provided map.
    pub fn substitute(&self, values: &HashMap<String, String>) -> String {
        let mut result = self.text.clone();
        for (key, value) in values {
            let placeholder = format!("{{{}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }
}

impl Default for PhraseEntry {
    fn default() -> Self {
        Self {
            text: String::new(),
            frequency: 1.0,
            tags: Vec::new(),
            context: None,
        }
    }
}

// ============================================================================
// Phrase Categories
// ============================================================================

/// A category of phrases organized by formality level.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PhraseCategory {
    /// Formal phrases (for speaking to nobility, authority, etc.)
    #[serde(default)]
    pub formal: Vec<PhraseEntry>,

    /// Casual phrases (for speaking to peers, friends)
    #[serde(default)]
    pub casual: Vec<PhraseEntry>,

    /// Hostile phrases (for speaking to enemies, threats)
    #[serde(default)]
    pub hostile: Vec<PhraseEntry>,
}

impl PhraseCategory {
    /// Create an empty phrase category.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a formal phrase.
    pub fn add_formal(&mut self, phrase: PhraseEntry) {
        self.formal.push(phrase);
    }

    /// Add a casual phrase.
    pub fn add_casual(&mut self, phrase: PhraseEntry) {
        self.casual.push(phrase);
    }

    /// Add a hostile phrase.
    pub fn add_hostile(&mut self, phrase: PhraseEntry) {
        self.hostile.push(phrase);
    }

    /// Get phrases by formality level.
    pub fn get_by_formality(&self, formality: Formality) -> &[PhraseEntry] {
        match formality {
            Formality::Formal => &self.formal,
            Formality::Casual => &self.casual,
            Formality::Hostile => &self.hostile,
        }
    }

    /// Get all phrases across all formality levels.
    pub fn all_phrases(&self) -> impl Iterator<Item = &PhraseEntry> {
        self.formal
            .iter()
            .chain(self.casual.iter())
            .chain(self.hostile.iter())
    }

    /// Check if this category has any phrases.
    pub fn is_empty(&self) -> bool {
        self.formal.is_empty() && self.casual.is_empty() && self.hostile.is_empty()
    }

    /// Get total phrase count across all formality levels.
    pub fn len(&self) -> usize {
        self.formal.len() + self.casual.len() + self.hostile.len()
    }

    /// Select a random phrase based on frequency weights.
    pub fn select_weighted(&self, formality: Formality, rng: &mut impl Rng) -> Option<&PhraseEntry> {
        let phrases = self.get_by_formality(formality);
        select_weighted_phrase(phrases, rng)
    }

    /// Select a random phrase with tag filtering.
    pub fn select_with_tags(
        &self,
        formality: Formality,
        required_tags: &[&str],
        rng: &mut impl Rng,
    ) -> Option<&PhraseEntry> {
        let phrases = self.get_by_formality(formality);
        let filtered: Vec<_> = phrases
            .iter()
            .filter(|p| required_tags.iter().all(|tag| p.has_tag(tag)))
            .collect();

        if filtered.is_empty() {
            return None;
        }

        let total_weight: f32 = filtered.iter().map(|p| p.frequency).sum();
        if total_weight <= 0.0 {
            return filtered.choose(rng).copied();
        }

        let mut choice = rng.gen::<f32>() * total_weight;
        for phrase in &filtered {
            choice -= phrase.frequency;
            if choice <= 0.0 {
                return Some(phrase);
            }
        }

        filtered.last().copied()
    }
}

/// Formality level for phrase selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Formality {
    Formal,
    Casual,
    Hostile,
}

impl Default for Formality {
    fn default() -> Self {
        Self::Casual
    }
}

impl Formality {
    /// Parse formality from a string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "formal" | "polite" | "respectful" => Self::Formal,
            "hostile" | "aggressive" | "threatening" => Self::Hostile,
            _ => Self::Casual,
        }
    }

    /// Convert to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Formal => "formal",
            Self::Casual => "casual",
            Self::Hostile => "hostile",
        }
    }
}

// ============================================================================
// Speech Patterns
// ============================================================================

/// A speech pattern that defines recurring stylistic elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechPattern {
    /// Description of the pattern (for documentation/UI)
    pub description: String,

    /// Example phrases demonstrating this pattern
    #[serde(default)]
    pub examples: Vec<String>,

    /// How often to apply this pattern (0.0-1.0)
    #[serde(default = "default_pattern_frequency")]
    pub frequency: f32,

    /// Pattern type (prefix, suffix, interjection, structure)
    #[serde(default)]
    pub pattern_type: PatternType,
}

fn default_pattern_frequency() -> f32 {
    0.3
}

impl SpeechPattern {
    /// Create a new speech pattern.
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            examples: Vec::new(),
            frequency: 0.3,
            pattern_type: PatternType::General,
        }
    }

    /// Add an example to this pattern.
    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }

    /// Set the frequency for this pattern.
    pub fn with_frequency(mut self, frequency: f32) -> Self {
        self.frequency = frequency.clamp(0.0, 1.0);
        self
    }

    /// Set the pattern type.
    pub fn with_type(mut self, pattern_type: PatternType) -> Self {
        self.pattern_type = pattern_type;
        self
    }

    /// Check if this pattern should be applied based on frequency.
    pub fn should_apply(&self, rng: &mut impl Rng) -> bool {
        rng.gen::<f32>() < self.frequency
    }
}

/// Type of speech pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PatternType {
    /// Pattern applied at the start of speech
    Prefix,
    /// Pattern applied at the end of speech
    Suffix,
    /// Pattern inserted during speech
    Interjection,
    /// Pattern affecting sentence structure
    Structure,
    /// General/unspecified pattern
    #[default]
    General,
}

// ============================================================================
// Vocabulary Bank
// ============================================================================

/// A complete vocabulary bank containing categorized phrases for NPC speech.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabularyBank {
    /// Unique identifier for this bank
    pub id: String,

    /// Human-readable name
    #[serde(default)]
    pub name: String,

    /// Description of this vocabulary bank
    #[serde(default)]
    pub description: String,

    /// Culture this bank is associated with (e.g., "dwarvish", "elvish")
    #[serde(default)]
    pub culture: Option<String>,

    /// NPC roles this bank is suited for (e.g., "merchant", "guard")
    #[serde(default)]
    pub roles: Vec<String>,

    /// Greeting phrases
    #[serde(default)]
    pub greetings: PhraseCategory,

    /// Farewell phrases
    #[serde(default)]
    pub farewells: PhraseCategory,

    /// Exclamations and interjections
    #[serde(default)]
    pub exclamations: Vec<PhraseEntry>,

    /// Negotiation and bargaining phrases
    #[serde(default)]
    pub negotiation: Vec<PhraseEntry>,

    /// Combat and threat phrases
    #[serde(default)]
    pub combat: Vec<PhraseEntry>,

    /// Cultural references (deities, places, sayings)
    #[serde(default)]
    pub cultural_references: Vec<String>,

    /// Proverbs and wisdom sayings
    #[serde(default)]
    pub proverbs: Vec<String>,

    /// Speech patterns specific to this bank
    #[serde(default)]
    pub speech_patterns: Vec<SpeechPattern>,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,

    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

impl VocabularyBank {
    /// Create a new empty vocabulary bank with the given ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: String::new(),
            description: String::new(),
            culture: None,
            roles: Vec::new(),
            greetings: PhraseCategory::default(),
            farewells: PhraseCategory::default(),
            exclamations: Vec::new(),
            negotiation: Vec::new(),
            combat: Vec::new(),
            cultural_references: Vec::new(),
            proverbs: Vec::new(),
            speech_patterns: Vec::new(),
            metadata: HashMap::new(),
            tags: Vec::new(),
        }
    }

    /// Create a vocabulary bank with a name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the culture for this bank.
    pub fn with_culture(mut self, culture: impl Into<String>) -> Self {
        self.culture = Some(culture.into());
        self
    }

    /// Add a role this bank is suited for.
    pub fn add_role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }

    /// Get a random greeting phrase.
    pub fn get_greeting(
        &self,
        formality: Formality,
        rng: &mut impl Rng,
    ) -> Option<&PhraseEntry> {
        self.greetings.select_weighted(formality, rng)
    }

    /// Get a random farewell phrase.
    pub fn get_farewell(
        &self,
        formality: Formality,
        rng: &mut impl Rng,
    ) -> Option<&PhraseEntry> {
        self.farewells.select_weighted(formality, rng)
    }

    /// Get a random exclamation.
    pub fn get_exclamation(&self, rng: &mut impl Rng) -> Option<&PhraseEntry> {
        select_weighted_phrase(&self.exclamations, rng)
    }

    /// Get a random negotiation phrase.
    pub fn get_negotiation_phrase(&self, rng: &mut impl Rng) -> Option<&PhraseEntry> {
        select_weighted_phrase(&self.negotiation, rng)
    }

    /// Get a random combat phrase.
    pub fn get_combat_phrase(&self, rng: &mut impl Rng) -> Option<&PhraseEntry> {
        select_weighted_phrase(&self.combat, rng)
    }

    /// Get a random proverb.
    pub fn get_proverb(&self, rng: &mut impl Rng) -> Option<&str> {
        self.proverbs.choose(rng).map(|s| s.as_str())
    }

    /// Get a random cultural reference.
    pub fn get_cultural_reference(&self, rng: &mut impl Rng) -> Option<&str> {
        self.cultural_references.choose(rng).map(|s| s.as_str())
    }

    /// Get speech patterns that should be applied based on their frequencies.
    pub fn get_active_patterns(&self, rng: &mut impl Rng) -> Vec<&SpeechPattern> {
        self.speech_patterns
            .iter()
            .filter(|p| p.should_apply(rng))
            .collect()
    }

    /// Check if this bank matches a given culture.
    pub fn matches_culture(&self, culture: &str) -> bool {
        self.culture
            .as_ref()
            .map(|c| c.eq_ignore_ascii_case(culture))
            .unwrap_or(false)
    }

    /// Check if this bank is suited for a given role.
    pub fn matches_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r.eq_ignore_ascii_case(role))
    }

    /// Validate the vocabulary bank structure.
    pub fn validate(&self) -> Result<(), VocabularyError> {
        if self.id.is_empty() {
            return Err(VocabularyError::InvalidStructure {
                bank_id: self.id.clone(),
                reason: "Bank ID cannot be empty".to_string(),
            });
        }

        // Check for invalid frequency values across all phrase categories
        for phrase in self.greetings.all_phrases() {
            if !(0.0..=1.0).contains(&phrase.frequency) {
                return Err(VocabularyError::InvalidStructure {
                    bank_id: self.id.clone(),
                    reason: format!(
                        "Invalid frequency {} in greeting phrase",
                        phrase.frequency
                    ),
                });
            }
        }

        for phrase in self.farewells.all_phrases() {
            if !(0.0..=1.0).contains(&phrase.frequency) {
                return Err(VocabularyError::InvalidStructure {
                    bank_id: self.id.clone(),
                    reason: format!(
                        "Invalid frequency {} in farewell phrase",
                        phrase.frequency
                    ),
                });
            }
        }

        for phrase in &self.exclamations {
            if !(0.0..=1.0).contains(&phrase.frequency) {
                return Err(VocabularyError::InvalidStructure {
                    bank_id: self.id.clone(),
                    reason: format!(
                        "Invalid frequency {} in exclamation phrase",
                        phrase.frequency
                    ),
                });
            }
        }

        for phrase in &self.negotiation {
            if !(0.0..=1.0).contains(&phrase.frequency) {
                return Err(VocabularyError::InvalidStructure {
                    bank_id: self.id.clone(),
                    reason: format!(
                        "Invalid frequency {} in negotiation phrase",
                        phrase.frequency
                    ),
                });
            }
        }

        for phrase in &self.combat {
            if !(0.0..=1.0).contains(&phrase.frequency) {
                return Err(VocabularyError::InvalidStructure {
                    bank_id: self.id.clone(),
                    reason: format!(
                        "Invalid frequency {} in combat phrase",
                        phrase.frequency
                    ),
                });
            }
        }

        for pattern in &self.speech_patterns {
            if !(0.0..=1.0).contains(&pattern.frequency) {
                return Err(VocabularyError::InvalidStructure {
                    bank_id: self.id.clone(),
                    reason: format!(
                        "Invalid frequency {} in speech pattern",
                        pattern.frequency
                    ),
                });
            }
        }

        Ok(())
    }

    /// Get total phrase count across all categories.
    pub fn total_phrases(&self) -> usize {
        self.greetings.len()
            + self.farewells.len()
            + self.exclamations.len()
            + self.negotiation.len()
            + self.combat.len()
    }
}

impl Default for VocabularyBank {
    fn default() -> Self {
        Self::new("default")
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Select a phrase from a list based on frequency weights.
fn select_weighted_phrase<'a>(
    phrases: &'a [PhraseEntry],
    rng: &mut impl Rng,
) -> Option<&'a PhraseEntry> {
    if phrases.is_empty() {
        return None;
    }

    let total_weight: f32 = phrases.iter().map(|p| p.frequency).sum();
    if total_weight <= 0.0 {
        return phrases.choose(rng);
    }

    let mut choice = rng.gen::<f32>() * total_weight;
    for phrase in phrases {
        choice -= phrase.frequency;
        if choice <= 0.0 {
            return Some(phrase);
        }
    }

    phrases.last()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phrase_entry_creation() {
        let phrase = PhraseEntry::new("Hello there!");
        assert_eq!(phrase.text, "Hello there!");
        assert_eq!(phrase.frequency, 1.0);
        assert!(phrase.tags.is_empty());
    }

    #[test]
    fn test_phrase_entry_with_frequency() {
        let phrase = PhraseEntry::with_frequency("Rare greeting", 0.2);
        assert_eq!(phrase.frequency, 0.2);
    }

    #[test]
    fn test_phrase_entry_frequency_clamping() {
        let phrase = PhraseEntry::with_frequency("Too high", 1.5);
        assert_eq!(phrase.frequency, 1.0);

        let phrase = PhraseEntry::with_frequency("Too low", -0.5);
        assert_eq!(phrase.frequency, 0.0);
    }

    #[test]
    fn test_phrase_entry_has_tag() {
        let phrase = PhraseEntry::new("Test")
            .with_tags(vec!["nervous".to_string(), "formal".to_string()]);

        assert!(phrase.has_tag("nervous"));
        assert!(phrase.has_tag("NERVOUS")); // Case insensitive
        assert!(!phrase.has_tag("casual"));
    }

    #[test]
    fn test_phrase_entry_substitute() {
        let phrase = PhraseEntry::new("By {deity}'s name, {player}!");
        let mut values = HashMap::new();
        values.insert("deity".to_string(), "Moradin".to_string());
        values.insert("player".to_string(), "friend".to_string());

        let result = phrase.substitute(&values);
        assert_eq!(result, "By Moradin's name, friend!");
    }

    #[test]
    fn test_phrase_category() {
        let mut category = PhraseCategory::new();
        category.add_formal(PhraseEntry::new("Good day, sir."));
        category.add_casual(PhraseEntry::new("Hey there!"));
        category.add_hostile(PhraseEntry::new("What do YOU want?"));

        assert_eq!(category.len(), 3);
        assert!(!category.is_empty());

        assert_eq!(category.get_by_formality(Formality::Formal).len(), 1);
        assert_eq!(category.get_by_formality(Formality::Casual).len(), 1);
        assert_eq!(category.get_by_formality(Formality::Hostile).len(), 1);
    }

    #[test]
    fn test_phrase_category_weighted_selection() {
        let mut category = PhraseCategory::new();
        category.add_casual(PhraseEntry::with_frequency("Common", 1.0));
        category.add_casual(PhraseEntry::with_frequency("Rare", 0.0));

        let mut rng = rand::thread_rng();

        // With frequency 0, "Rare" should never be selected via weighted selection
        // (though it can be if all frequencies are 0)
        for _ in 0..10 {
            let selected = category.select_weighted(Formality::Casual, &mut rng);
            assert!(selected.is_some());
            assert_eq!(selected.unwrap().text, "Common");
        }
    }

    #[test]
    fn test_formality_from_str() {
        assert_eq!(Formality::from_str("formal"), Formality::Formal);
        assert_eq!(Formality::from_str("POLITE"), Formality::Formal);
        assert_eq!(Formality::from_str("hostile"), Formality::Hostile);
        assert_eq!(Formality::from_str("aggressive"), Formality::Hostile);
        assert_eq!(Formality::from_str("casual"), Formality::Casual);
        assert_eq!(Formality::from_str("unknown"), Formality::Casual);
    }

    #[test]
    fn test_speech_pattern() {
        let pattern = SpeechPattern::new("Ends sentences with 'ye know?'")
            .with_example("Nice day, ye know?")
            .with_frequency(0.5)
            .with_type(PatternType::Suffix);

        assert_eq!(pattern.examples.len(), 1);
        assert_eq!(pattern.frequency, 0.5);
        assert_eq!(pattern.pattern_type, PatternType::Suffix);
    }

    #[test]
    fn test_vocabulary_bank_creation() {
        let bank = VocabularyBank::new("tavern")
            .with_name("Tavern Keeper Vocabulary")
            .with_culture("common")
            .add_role("merchant")
            .add_role("innkeeper");

        assert_eq!(bank.id, "tavern");
        assert_eq!(bank.name, "Tavern Keeper Vocabulary");
        assert_eq!(bank.culture, Some("common".to_string()));
        assert_eq!(bank.roles.len(), 2);
    }

    #[test]
    fn test_vocabulary_bank_culture_matching() {
        let bank = VocabularyBank::new("elvish").with_culture("elvish");

        assert!(bank.matches_culture("elvish"));
        assert!(bank.matches_culture("ELVISH"));
        assert!(!bank.matches_culture("dwarvish"));
    }

    #[test]
    fn test_vocabulary_bank_role_matching() {
        let bank = VocabularyBank::new("merchant").add_role("merchant").add_role("shopkeeper");

        assert!(bank.matches_role("merchant"));
        assert!(bank.matches_role("SHOPKEEPER"));
        assert!(!bank.matches_role("guard"));
    }

    #[test]
    fn test_vocabulary_bank_validation() {
        let valid_bank = VocabularyBank::new("valid");
        assert!(valid_bank.validate().is_ok());

        let invalid_bank = VocabularyBank::new("");
        assert!(invalid_bank.validate().is_err());
    }

    #[test]
    fn test_vocabulary_bank_phrase_retrieval() {
        let mut bank = VocabularyBank::new("test");
        bank.greetings.add_casual(PhraseEntry::new("Hello!"));
        bank.farewells.add_casual(PhraseEntry::new("Goodbye!"));
        bank.exclamations.push(PhraseEntry::new("Wow!"));
        bank.proverbs.push("A penny saved is a penny earned.".to_string());

        let mut rng = rand::thread_rng();

        assert!(bank.get_greeting(Formality::Casual, &mut rng).is_some());
        assert!(bank.get_farewell(Formality::Casual, &mut rng).is_some());
        assert!(bank.get_exclamation(&mut rng).is_some());
        assert!(bank.get_proverb(&mut rng).is_some());

        // Empty category should return None
        assert!(bank.get_greeting(Formality::Formal, &mut rng).is_none());
    }

    #[test]
    fn test_vocabulary_bank_yaml_roundtrip() {
        let mut bank = VocabularyBank::new("test")
            .with_name("Test Bank")
            .with_culture("human");

        bank.greetings
            .add_casual(PhraseEntry::with_frequency("Hello!", 0.8));
        bank.proverbs.push("Test proverb".to_string());

        let yaml = serde_yaml_ng::to_string(&bank).unwrap();
        let parsed: VocabularyBank = serde_yaml_ng::from_str(&yaml).unwrap();

        assert_eq!(parsed.id, "test");
        assert_eq!(parsed.name, "Test Bank");
        assert_eq!(parsed.culture, Some("human".to_string()));
        assert_eq!(parsed.greetings.casual.len(), 1);
        assert_eq!(parsed.proverbs.len(), 1);
    }

    #[test]
    fn test_total_phrases() {
        let mut bank = VocabularyBank::new("test");
        bank.greetings.add_casual(PhraseEntry::new("Hi"));
        bank.greetings.add_formal(PhraseEntry::new("Hello"));
        bank.farewells.add_casual(PhraseEntry::new("Bye"));
        bank.exclamations.push(PhraseEntry::new("Wow"));
        bank.negotiation.push(PhraseEntry::new("Deal"));
        bank.combat.push(PhraseEntry::new("Attack!"));

        assert_eq!(bank.total_phrases(), 6);
    }
}
