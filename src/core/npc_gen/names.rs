//! Name Component Data Models
//!
//! Defines data structures for cultural naming systems with support for:
//! - Multiple name structures (Given-Family, Family-Given, Epithet-based, etc.)
//! - Gender-specific and gender-neutral components
//! - Phonetic compatibility rules
//! - Cultural naming conventions and constraints
//!
//! # Architecture
//!
//! ```text
//! CulturalNamingRules
//!   +-- culture_id: String
//!   +-- name_structure: NameStructure
//!   +-- components: NameComponents
//!   |     +-- prefixes: Vec<NameComponent>
//!   |     +-- roots: Vec<NameComponent>
//!   |     +-- suffixes: Vec<NameComponent>
//!   |     +-- titles: Vec<NameComponent>
//!   |     +-- epithets: Vec<NameComponent>
//!   +-- gender_rules: GenderRules
//!   +-- phonetic_rules: Vec<PhoneticRule>
//! ```

use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::errors::NameGenerationError;

// ============================================================================
// Name Structure
// ============================================================================

/// Defines how name components are arranged for a culture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NameStructure {
    /// Western style: "John Smith" (given name first, family name last)
    #[default]
    GivenFamily,

    /// Eastern style: "Smith John" (family name first, given name last)
    FamilyGiven,

    /// Epithet style: "Grak the Crusher" (given name + epithet)
    GivenEpithet,

    /// Synthetic style: "Aelrielien" (prefix + root + suffix combined)
    PrefixRootSuffix,

    /// Clan descriptor style: "Thorin Oakenshield" (given + descriptor)
    ClanDescriptor,

    /// Patronymic style: "Bjorn Eriksson" (given + parent-derived)
    Patronymic,

    /// Matronymic style: "Bjorn Helgasdottir" (given + mother-derived)
    Matronymic,

    /// Single name only: "Gandalf"
    SingleName,

    /// Title-based: "Lord Blackwood" (title + family/epithet)
    TitleBased,
}

impl NameStructure {
    /// Get a human-readable description of this structure.
    pub fn description(&self) -> &'static str {
        match self {
            Self::GivenFamily => "Given name followed by family name",
            Self::FamilyGiven => "Family name followed by given name",
            Self::GivenEpithet => "Given name followed by an epithet",
            Self::PrefixRootSuffix => "Synthetic name from combined syllables",
            Self::ClanDescriptor => "Given name with clan or deed descriptor",
            Self::Patronymic => "Given name with father-derived surname",
            Self::Matronymic => "Given name with mother-derived surname",
            Self::SingleName => "Single name only (no surname)",
            Self::TitleBased => "Title followed by family name or epithet",
        }
    }

    /// Get the format pattern for this structure.
    ///
    /// Placeholders:
    /// - `{given}` - Given/first name
    /// - `{family}` - Family/last name
    /// - `{title}` - Title (Lord, Lady, etc.)
    /// - `{epithet}` - Descriptive epithet
    /// - `{prefix}` - Name prefix syllable
    /// - `{root}` - Name root syllable
    /// - `{suffix}` - Name suffix syllable
    /// - `{parent}` - Parent's name (for patronymic/matronymic)
    pub fn format_pattern(&self) -> &'static str {
        match self {
            Self::GivenFamily => "{given} {family}",
            Self::FamilyGiven => "{family} {given}",
            Self::GivenEpithet => "{given} {epithet}",
            Self::PrefixRootSuffix => "{prefix}{root}{suffix}",
            Self::ClanDescriptor => "{given} {epithet}",
            Self::Patronymic => "{given} {parent}son",
            Self::Matronymic => "{given} {parent}daughter",
            Self::SingleName => "{given}",
            Self::TitleBased => "{title} {family}",
        }
    }
}

// ============================================================================
// Component Types
// ============================================================================

/// Type of name component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentType {
    /// Start of a synthetic name ("Ael", "Thor")
    Prefix,
    /// Middle part of a synthetic name ("ri", "an")
    Root,
    /// End of a synthetic name ("ion", "dor")
    Suffix,
    /// Given/first name ("John", "Elena")
    Given,
    /// Family/last name ("Smith", "Ironforge")
    Family,
    /// Honorific title ("Lord", "Lady", "Sir")
    Title,
    /// Descriptive epithet ("the Bold", "Dragonslayer")
    Epithet,
    /// Clan or tribe name ("Battlehammer", "Moonwhisper")
    Clan,
    /// Nickname ("Red", "Lucky")
    Nickname,
}

impl ComponentType {
    /// Get all component types.
    pub fn all() -> &'static [ComponentType] {
        &[
            ComponentType::Prefix,
            ComponentType::Root,
            ComponentType::Suffix,
            ComponentType::Given,
            ComponentType::Family,
            ComponentType::Title,
            ComponentType::Epithet,
            ComponentType::Clan,
            ComponentType::Nickname,
        ]
    }
}

// ============================================================================
// Gender
// ============================================================================

/// Gender affinity for name components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Gender {
    Male,
    Female,
    #[default]
    Neutral,
    /// Can be used for any gender
    Any,
}

impl Gender {
    /// Parse gender from a string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "male" | "m" | "masculine" => Self::Male,
            "female" | "f" | "feminine" => Self::Female,
            "neutral" | "n" | "nonbinary" | "nb" => Self::Neutral,
            "any" | "all" | "*" => Self::Any,
            _ => Self::Neutral,
        }
    }

    /// Check if this gender is compatible with another.
    pub fn is_compatible_with(&self, other: &Gender) -> bool {
        match (self, other) {
            (Self::Any, _) | (_, Self::Any) => true,
            (Self::Neutral, _) | (_, Self::Neutral) => true,
            (a, b) => a == b,
        }
    }
}

// ============================================================================
// Name Component
// ============================================================================

/// A single name component (syllable, word, or phrase).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameComponent {
    /// The component text
    pub text: String,

    /// Type of component
    pub component_type: ComponentType,

    /// Gender affinity
    #[serde(default)]
    pub gender: Gender,

    /// Usage frequency weight (0.0-1.0)
    #[serde(default = "default_frequency")]
    pub frequency: f32,

    /// Optional meaning or etymology
    #[serde(default)]
    pub meaning: Option<String>,

    /// Phonetic tags for compatibility checking
    #[serde(default)]
    pub phonetic_tags: Vec<String>,

    /// Tags for additional categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_frequency() -> f32 {
    1.0
}

impl NameComponent {
    /// Create a new name component.
    pub fn new(text: impl Into<String>, component_type: ComponentType) -> Self {
        Self {
            text: text.into(),
            component_type,
            gender: Gender::Neutral,
            frequency: 1.0,
            meaning: None,
            phonetic_tags: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Set the gender for this component.
    pub fn with_gender(mut self, gender: Gender) -> Self {
        self.gender = gender;
        self
    }

    /// Set the frequency for this component.
    pub fn with_frequency(mut self, frequency: f32) -> Self {
        self.frequency = frequency.clamp(0.0, 1.0);
        self
    }

    /// Set the meaning for this component.
    pub fn with_meaning(mut self, meaning: impl Into<String>) -> Self {
        self.meaning = Some(meaning.into());
        self
    }

    /// Add phonetic tags to this component.
    pub fn with_phonetic_tags(mut self, tags: Vec<String>) -> Self {
        self.phonetic_tags = tags;
        self
    }

    /// Check if this component has a specific phonetic tag.
    pub fn has_phonetic_tag(&self, tag: &str) -> bool {
        self.phonetic_tags
            .iter()
            .any(|t| t.eq_ignore_ascii_case(tag))
    }

    /// Check if this component is compatible with a given gender.
    pub fn is_compatible_with_gender(&self, gender: &Gender) -> bool {
        self.gender.is_compatible_with(gender)
    }
}

impl Default for NameComponent {
    fn default() -> Self {
        Self {
            text: String::new(),
            component_type: ComponentType::Given,
            gender: Gender::Neutral,
            frequency: 1.0,
            meaning: None,
            phonetic_tags: Vec::new(),
            tags: Vec::new(),
        }
    }
}

// ============================================================================
// Name Components Collection
// ============================================================================

/// Collection of name components organized by type.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NameComponents {
    /// Prefix syllables for synthetic names
    #[serde(default)]
    pub prefixes: Vec<NameComponent>,

    /// Root syllables for synthetic names
    #[serde(default)]
    pub roots: Vec<NameComponent>,

    /// Suffix syllables for synthetic names
    #[serde(default)]
    pub suffixes: Vec<NameComponent>,

    /// Complete given names
    #[serde(default)]
    pub given_names: Vec<NameComponent>,

    /// Family/last names
    #[serde(default)]
    pub family_names: Vec<NameComponent>,

    /// Titles and honorifics
    #[serde(default)]
    pub titles: Vec<NameComponent>,

    /// Epithets and descriptors
    #[serde(default)]
    pub epithets: Vec<NameComponent>,

    /// Clan or tribe names
    #[serde(default)]
    pub clans: Vec<NameComponent>,

    /// Nicknames
    #[serde(default)]
    pub nicknames: Vec<NameComponent>,
}

impl NameComponents {
    /// Create an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get components by type.
    pub fn get_by_type(&self, component_type: ComponentType) -> &[NameComponent] {
        match component_type {
            ComponentType::Prefix => &self.prefixes,
            ComponentType::Root => &self.roots,
            ComponentType::Suffix => &self.suffixes,
            ComponentType::Given => &self.given_names,
            ComponentType::Family => &self.family_names,
            ComponentType::Title => &self.titles,
            ComponentType::Epithet => &self.epithets,
            ComponentType::Clan => &self.clans,
            ComponentType::Nickname => &self.nicknames,
        }
    }

    /// Get mutable reference to components by type.
    pub fn get_by_type_mut(&mut self, component_type: ComponentType) -> &mut Vec<NameComponent> {
        match component_type {
            ComponentType::Prefix => &mut self.prefixes,
            ComponentType::Root => &mut self.roots,
            ComponentType::Suffix => &mut self.suffixes,
            ComponentType::Given => &mut self.given_names,
            ComponentType::Family => &mut self.family_names,
            ComponentType::Title => &mut self.titles,
            ComponentType::Epithet => &mut self.epithets,
            ComponentType::Clan => &mut self.clans,
            ComponentType::Nickname => &mut self.nicknames,
        }
    }

    /// Add a component to the appropriate collection.
    pub fn add(&mut self, component: NameComponent) {
        self.get_by_type_mut(component.component_type)
            .push(component);
    }

    /// Select a random component of the given type, filtered by gender.
    pub fn select_random(
        &self,
        component_type: ComponentType,
        gender: Option<&Gender>,
        rng: &mut impl Rng,
    ) -> Option<&NameComponent> {
        let components = self.get_by_type(component_type);

        let filtered: Vec<_> = components
            .iter()
            .filter(|c| {
                gender
                    .map(|g| c.is_compatible_with_gender(g))
                    .unwrap_or(true)
            })
            .collect();

        if filtered.is_empty() {
            return None;
        }

        // Weighted random selection
        let total_weight: f32 = filtered.iter().map(|c| c.frequency).sum();
        if total_weight <= 0.0 {
            return filtered.choose(rng).copied();
        }

        let mut choice = rng.gen::<f32>() * total_weight;
        for component in &filtered {
            choice -= component.frequency;
            if choice <= 0.0 {
                return Some(component);
            }
        }

        filtered.last().copied()
    }

    /// Check if any components exist for a given type.
    pub fn has_type(&self, component_type: ComponentType) -> bool {
        !self.get_by_type(component_type).is_empty()
    }

    /// Get total component count across all types.
    pub fn total_count(&self) -> usize {
        self.prefixes.len()
            + self.roots.len()
            + self.suffixes.len()
            + self.given_names.len()
            + self.family_names.len()
            + self.titles.len()
            + self.epithets.len()
            + self.clans.len()
            + self.nicknames.len()
    }
}

// ============================================================================
// Gender Rules
// ============================================================================

/// Rules for gender-specific name generation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenderRules {
    /// Whether to use gender-specific suffixes
    #[serde(default)]
    pub use_gendered_suffixes: bool,

    /// Suffix to append for male names (e.g., "son" for patronymics)
    #[serde(default)]
    pub male_suffix: Option<String>,

    /// Suffix to append for female names (e.g., "dottir" for matronymics)
    #[serde(default)]
    pub female_suffix: Option<String>,

    /// Suffix for neutral names
    #[serde(default)]
    pub neutral_suffix: Option<String>,

    /// Whether all components must match the requested gender
    #[serde(default)]
    pub strict_gender_matching: bool,

    /// Default gender when none specified
    #[serde(default)]
    pub default_gender: Gender,
}

impl GenderRules {
    /// Create new gender rules.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable gendered suffixes.
    pub fn with_gendered_suffixes(mut self, male: &str, female: &str) -> Self {
        self.use_gendered_suffixes = true;
        self.male_suffix = Some(male.to_string());
        self.female_suffix = Some(female.to_string());
        self
    }

    /// Get the appropriate suffix for a gender.
    pub fn get_suffix(&self, gender: &Gender) -> Option<&str> {
        if !self.use_gendered_suffixes {
            return None;
        }

        match gender {
            Gender::Male => self.male_suffix.as_deref(),
            Gender::Female => self.female_suffix.as_deref(),
            Gender::Neutral | Gender::Any => self.neutral_suffix.as_deref(),
        }
    }
}

// ============================================================================
// Phonetic Rules
// ============================================================================

/// Rule for phonetic compatibility between name components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneticRule {
    /// Description of the rule
    pub description: String,

    /// Tags that this rule applies to
    #[serde(default)]
    pub applies_to: Vec<String>,

    /// Tags that are compatible with the applies_to tags
    #[serde(default)]
    pub compatible_with: Vec<String>,

    /// Tags that are incompatible with the applies_to tags
    #[serde(default)]
    pub incompatible_with: Vec<String>,
}

impl PhoneticRule {
    /// Create a new phonetic rule.
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            applies_to: Vec::new(),
            compatible_with: Vec::new(),
            incompatible_with: Vec::new(),
        }
    }

    /// Check if a component pair satisfies this rule.
    pub fn is_satisfied(&self, first: &NameComponent, second: &NameComponent) -> bool {
        // If the first component doesn't have any of the applies_to tags, rule doesn't apply
        if !self
            .applies_to
            .iter()
            .any(|t| first.has_phonetic_tag(t))
        {
            return true;
        }

        // Check for incompatible tags
        for tag in &self.incompatible_with {
            if second.has_phonetic_tag(tag) {
                return false;
            }
        }

        // If there are compatible tags, at least one must match
        if !self.compatible_with.is_empty() {
            return self
                .compatible_with
                .iter()
                .any(|t| second.has_phonetic_tag(t));
        }

        true
    }
}

// ============================================================================
// Cultural Naming Rules
// ============================================================================

/// Complete naming rules for a culture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CulturalNamingRules {
    /// Unique identifier for this culture
    pub culture_id: String,

    /// Human-readable culture name
    #[serde(default)]
    pub culture_name: String,

    /// Description of this culture's naming conventions
    #[serde(default)]
    pub description: String,

    /// Primary name structure for this culture
    #[serde(default)]
    pub name_structure: NameStructure,

    /// Alternative structures that can be used
    #[serde(default)]
    pub alternative_structures: Vec<NameStructure>,

    /// Name components
    #[serde(default)]
    pub components: NameComponents,

    /// Gender rules
    #[serde(default)]
    pub gender_rules: GenderRules,

    /// Phonetic compatibility rules
    #[serde(default)]
    pub phonetic_rules: Vec<PhoneticRule>,

    /// Minimum name length (characters)
    #[serde(default)]
    pub min_length: Option<usize>,

    /// Maximum name length (characters)
    #[serde(default)]
    pub max_length: Option<usize>,

    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl CulturalNamingRules {
    /// Create new naming rules for a culture.
    pub fn new(culture_id: impl Into<String>) -> Self {
        Self {
            culture_id: culture_id.into(),
            culture_name: String::new(),
            description: String::new(),
            name_structure: NameStructure::default(),
            alternative_structures: Vec::new(),
            components: NameComponents::default(),
            gender_rules: GenderRules::default(),
            phonetic_rules: Vec::new(),
            min_length: None,
            max_length: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set the culture name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.culture_name = name.into();
        self
    }

    /// Set the name structure.
    pub fn with_structure(mut self, structure: NameStructure) -> Self {
        self.name_structure = structure;
        self
    }

    /// Add an alternative structure.
    pub fn add_alternative_structure(mut self, structure: NameStructure) -> Self {
        self.alternative_structures.push(structure);
        self
    }

    /// Validate the naming rules.
    pub fn validate(&self) -> Result<(), NameGenerationError> {
        if self.culture_id.is_empty() {
            return Err(NameGenerationError::InvalidPattern {
                culture: self.culture_id.clone(),
                pattern: "culture_id".to_string(),
                reason: "Culture ID cannot be empty".to_string(),
            });
        }

        // Check that required components exist for the name structure
        match self.name_structure {
            NameStructure::GivenFamily => {
                // GivenFamily requires both Given and Family components
                if !self.components.has_type(ComponentType::Given) {
                    return Err(NameGenerationError::ComponentNotAvailable {
                        culture: self.culture_id.clone(),
                        component_type: "given".to_string(),
                    });
                }
                if !self.components.has_type(ComponentType::Family) {
                    return Err(NameGenerationError::ComponentNotAvailable {
                        culture: self.culture_id.clone(),
                        component_type: "family".to_string(),
                    });
                }
            }
            NameStructure::PrefixRootSuffix => {
                if !self.components.has_type(ComponentType::Prefix) {
                    return Err(NameGenerationError::ComponentNotAvailable {
                        culture: self.culture_id.clone(),
                        component_type: "prefix".to_string(),
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Check if a component pair is phonetically compatible.
    pub fn is_compatible(&self, first: &NameComponent, second: &NameComponent) -> bool {
        self.phonetic_rules
            .iter()
            .all(|rule| rule.is_satisfied(first, second))
    }

    /// Get a random structure (primary or alternative).
    pub fn random_structure(&self, rng: &mut impl Rng) -> NameStructure {
        if self.alternative_structures.is_empty() || rng.gen_bool(0.7) {
            self.name_structure
        } else {
            *self
                .alternative_structures
                .choose(rng)
                .unwrap_or(&self.name_structure)
        }
    }
}

impl Default for CulturalNamingRules {
    fn default() -> Self {
        Self::new("default")
    }
}

// ============================================================================
// Name Pattern
// ============================================================================

/// A specific pattern for generating names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamePattern {
    /// Pattern identifier
    pub id: String,

    /// Pattern template (e.g., "{prefix}{root}{suffix}")
    pub template: String,

    /// Required component types
    pub required_components: Vec<ComponentType>,

    /// Usage frequency weight
    #[serde(default = "default_frequency")]
    pub frequency: f32,

    /// Description of this pattern
    #[serde(default)]
    pub description: String,
}

impl NamePattern {
    /// Create a new name pattern.
    pub fn new(id: impl Into<String>, template: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            template: template.into(),
            required_components: Vec::new(),
            frequency: 1.0,
            description: String::new(),
        }
    }

    /// Add a required component type.
    pub fn require(mut self, component_type: ComponentType) -> Self {
        self.required_components.push(component_type);
        self
    }

    /// Check if this pattern can be satisfied by the given components.
    pub fn can_be_satisfied(&self, components: &NameComponents) -> bool {
        self.required_components
            .iter()
            .all(|ct| components.has_type(*ct))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_structure_description() {
        assert!(!NameStructure::GivenFamily.description().is_empty());
        assert!(!NameStructure::PrefixRootSuffix.description().is_empty());
    }

    #[test]
    fn test_name_structure_format_pattern() {
        assert_eq!(NameStructure::GivenFamily.format_pattern(), "{given} {family}");
        assert_eq!(
            NameStructure::PrefixRootSuffix.format_pattern(),
            "{prefix}{root}{suffix}"
        );
    }

    #[test]
    fn test_gender_from_str() {
        assert_eq!(Gender::from_str("male"), Gender::Male);
        assert_eq!(Gender::from_str("FEMALE"), Gender::Female);
        assert_eq!(Gender::from_str("neutral"), Gender::Neutral);
        assert_eq!(Gender::from_str("any"), Gender::Any);
        assert_eq!(Gender::from_str("unknown"), Gender::Neutral);
    }

    #[test]
    fn test_gender_compatibility() {
        assert!(Gender::Any.is_compatible_with(&Gender::Male));
        assert!(Gender::Any.is_compatible_with(&Gender::Female));
        assert!(Gender::Neutral.is_compatible_with(&Gender::Male));
        assert!(Gender::Male.is_compatible_with(&Gender::Male));
        assert!(!Gender::Male.is_compatible_with(&Gender::Female));
    }

    #[test]
    fn test_name_component_creation() {
        let component = NameComponent::new("Ael", ComponentType::Prefix)
            .with_gender(Gender::Neutral)
            .with_frequency(0.8)
            .with_meaning("star");

        assert_eq!(component.text, "Ael");
        assert_eq!(component.component_type, ComponentType::Prefix);
        assert_eq!(component.gender, Gender::Neutral);
        assert_eq!(component.frequency, 0.8);
        assert_eq!(component.meaning, Some("star".to_string()));
    }

    #[test]
    fn test_name_component_phonetic_tags() {
        let component = NameComponent::new("Ael", ComponentType::Prefix)
            .with_phonetic_tags(vec!["vowel_start".to_string(), "soft".to_string()]);

        assert!(component.has_phonetic_tag("vowel_start"));
        assert!(component.has_phonetic_tag("SOFT")); // Case-insensitive
        assert!(!component.has_phonetic_tag("consonant_start"));
    }

    #[test]
    fn test_name_components_collection() {
        let mut components = NameComponents::new();

        components.add(NameComponent::new("Ael", ComponentType::Prefix));
        components.add(NameComponent::new("ri", ComponentType::Root));
        components.add(NameComponent::new("ion", ComponentType::Suffix));

        assert!(components.has_type(ComponentType::Prefix));
        assert!(components.has_type(ComponentType::Root));
        assert!(components.has_type(ComponentType::Suffix));
        assert!(!components.has_type(ComponentType::Title));

        assert_eq!(components.total_count(), 3);
    }

    #[test]
    fn test_name_components_random_selection() {
        let mut components = NameComponents::new();

        components.add(
            NameComponent::new("Ael", ComponentType::Prefix)
                .with_gender(Gender::Neutral)
                .with_frequency(1.0),
        );
        components.add(
            NameComponent::new("Thor", ComponentType::Prefix)
                .with_gender(Gender::Male)
                .with_frequency(1.0),
        );

        let mut rng = rand::thread_rng();

        // Select without gender filter
        let selected = components.select_random(ComponentType::Prefix, None, &mut rng);
        assert!(selected.is_some());

        // Select with male gender filter - should get both (neutral is compatible)
        let selected = components.select_random(ComponentType::Prefix, Some(&Gender::Male), &mut rng);
        assert!(selected.is_some());

        // Select from empty type
        let selected = components.select_random(ComponentType::Title, None, &mut rng);
        assert!(selected.is_none());
    }

    #[test]
    fn test_gender_rules() {
        let rules = GenderRules::new().with_gendered_suffixes("son", "dottir");

        assert!(rules.use_gendered_suffixes);
        assert_eq!(rules.get_suffix(&Gender::Male), Some("son"));
        assert_eq!(rules.get_suffix(&Gender::Female), Some("dottir"));
    }

    #[test]
    fn test_phonetic_rule() {
        let rule = PhoneticRule {
            description: "Vowel endings prefer consonant starts".to_string(),
            applies_to: vec!["vowel_end".to_string()],
            compatible_with: vec!["consonant_start".to_string()],
            incompatible_with: vec!["vowel_start".to_string()],
        };

        let vowel_end = NameComponent::new("a", ComponentType::Root)
            .with_phonetic_tags(vec!["vowel_end".to_string()]);

        let consonant_start = NameComponent::new("tion", ComponentType::Suffix)
            .with_phonetic_tags(vec!["consonant_start".to_string()]);

        let vowel_start = NameComponent::new("ion", ComponentType::Suffix)
            .with_phonetic_tags(vec!["vowel_start".to_string()]);

        assert!(rule.is_satisfied(&vowel_end, &consonant_start));
        assert!(!rule.is_satisfied(&vowel_end, &vowel_start));
    }

    #[test]
    fn test_cultural_naming_rules() {
        let mut rules = CulturalNamingRules::new("elvish")
            .with_name("Elvish Names")
            .with_structure(NameStructure::PrefixRootSuffix);

        rules
            .components
            .add(NameComponent::new("Ael", ComponentType::Prefix));
        rules
            .components
            .add(NameComponent::new("ri", ComponentType::Root));
        rules
            .components
            .add(NameComponent::new("ion", ComponentType::Suffix));

        assert_eq!(rules.culture_id, "elvish");
        assert_eq!(rules.name_structure, NameStructure::PrefixRootSuffix);
        assert!(rules.validate().is_ok());
    }

    #[test]
    fn test_cultural_naming_rules_validation() {
        let empty_id = CulturalNamingRules::new("");
        assert!(empty_id.validate().is_err());

        let missing_prefix = CulturalNamingRules::new("test")
            .with_structure(NameStructure::PrefixRootSuffix);
        assert!(missing_prefix.validate().is_err());
    }

    #[test]
    fn test_name_pattern() {
        let pattern = NamePattern::new("synthetic", "{prefix}{root}{suffix}")
            .require(ComponentType::Prefix)
            .require(ComponentType::Suffix);

        let mut components = NameComponents::new();
        components.add(NameComponent::new("Ael", ComponentType::Prefix));
        components.add(NameComponent::new("ion", ComponentType::Suffix));

        assert!(pattern.can_be_satisfied(&components));

        components = NameComponents::new();
        components.add(NameComponent::new("Ael", ComponentType::Prefix));
        // Missing suffix

        assert!(!pattern.can_be_satisfied(&components));
    }

    #[test]
    fn test_yaml_roundtrip() {
        let mut rules = CulturalNamingRules::new("test")
            .with_name("Test Culture")
            .with_structure(NameStructure::GivenFamily);

        rules.components.add(
            NameComponent::new("John", ComponentType::Given)
                .with_gender(Gender::Male)
                .with_frequency(0.5),
        );

        let yaml = serde_yaml_ng::to_string(&rules).unwrap();
        let parsed: CulturalNamingRules = serde_yaml_ng::from_str(&yaml).unwrap();

        assert_eq!(parsed.culture_id, "test");
        assert_eq!(parsed.culture_name, "Test Culture");
        assert_eq!(parsed.name_structure, NameStructure::GivenFamily);
        assert_eq!(parsed.components.given_names.len(), 1);
    }
}
