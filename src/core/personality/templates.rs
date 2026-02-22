//! Setting Template Struct with Validation (TASK-PERS-005)
//!
//! Provides the `SettingTemplate` struct for setting-specific personality customization
//! with comprehensive validation rules ensuring data integrity.
//!
//! ## Validation Rules
//!
//! - `id`: Non-empty string
//! - `name`: Non-empty string, max 100 characters
//! - `description`: Optional, max 500 characters if present
//! - `game_system`: Optional, but recommended for filtering
//! - `setting_name`: Optional, but recommended for filtering
//! - `base_profile`: Non-empty string, must reference existing profile
//! - `vocabulary`: At least 10 entries with frequencies in 0.0-1.0 range
//! - `common_phrases`: At least 5 entries, non-empty strings
//! - `deity_references`: At least 1 entry if setting has deities
//! - `tags`: Optional but useful for search
//!
//! ## Example
//!
//! ```rust,ignore
//! use personality::templates::{SettingTemplate, TemplateValidationConfig};
//!
//! let template = SettingTemplate::builder("Forgotten Realms Sage", "storyteller")
//!     .game_system("dnd5e")
//!     .setting_name("Forgotten Realms")
//!     .vocabulary("ancient texts", 0.05)
//!     .vocabulary("Mystra's blessing", 0.03)
//!     // ... more vocabulary entries
//!     .common_phrase("As the annals of Candlekeep record")
//!     // ... more phrases
//!     .deity_reference("Mystra")
//!     .build()?;
//!
//! template.validate()?;
//! ```

use super::errors::TemplateError;
use super::types::{PersonalityId, SettingPersonalityTemplate, TemplateId};
use crate::core::personality_base::PersonalityProfile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Validation Configuration
// ============================================================================

/// Configuration for template validation rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateValidationConfig {
    /// Minimum number of vocabulary entries required.
    pub min_vocabulary_entries: usize,

    /// Minimum number of common phrases required.
    pub min_common_phrases: usize,

    /// Minimum number of deity references required (0 to disable).
    pub min_deity_references: usize,

    /// Minimum length for name field.
    pub min_name_length: usize,

    /// Maximum length for name field.
    pub max_name_length: usize,

    /// Minimum length for description field (if present).
    pub min_description_length: usize,

    /// Maximum length for description field.
    pub max_description_length: usize,

    /// Whether to require game_system field.
    pub require_game_system: bool,

    /// Whether to require setting_name field.
    pub require_setting_name: bool,
}

impl Default for TemplateValidationConfig {
    fn default() -> Self {
        Self {
            min_vocabulary_entries: 10,
            min_common_phrases: 5,
            min_deity_references: 0, // Optional by default
            min_name_length: 1,
            max_name_length: 100,
            min_description_length: 10,
            max_description_length: 500,
            require_game_system: false,
            require_setting_name: false,
        }
    }
}

impl TemplateValidationConfig {
    /// Create a minimal configuration for quick validation.
    /// - Name: 1-100 chars
    /// - Description: 10-500 chars (if present)
    /// - Vocabulary: >= 10 entries
    pub fn minimal() -> Self {
        Self {
            min_vocabulary_entries: 10,
            min_common_phrases: 0,
            min_deity_references: 0,
            min_name_length: 1,
            max_name_length: 100,
            min_description_length: 10,
            max_description_length: 500,
            require_game_system: false,
            require_setting_name: false,
        }
    }

    /// Create a lenient configuration for testing or draft templates.
    pub fn lenient() -> Self {
        Self {
            min_vocabulary_entries: 0,
            min_common_phrases: 0,
            min_deity_references: 0,
            min_name_length: 0,
            max_name_length: 500,
            min_description_length: 0,
            max_description_length: 10000,
            require_game_system: false,
            require_setting_name: false,
        }
    }

    /// Create a strict configuration for production templates.
    pub fn strict() -> Self {
        Self {
            min_vocabulary_entries: 10,
            min_common_phrases: 5,
            min_deity_references: 1,
            min_name_length: 1,
            max_name_length: 100,
            min_description_length: 10,
            max_description_length: 500,
            require_game_system: true,
            require_setting_name: true,
        }
    }
}

// ============================================================================
// Setting Template (Extended from SettingPersonalityTemplate)
// ============================================================================

/// A setting-specific personality template with full validation support.
///
/// This extends `SettingPersonalityTemplate` from the types module with:
/// - Deity references for setting-appropriate divine mentions
/// - Validation methods with configurable rules
/// - Builder pattern for ergonomic construction
/// - Conversion to `PersonalityProfile` for blending
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingTemplate {
    /// Unique identifier for the template.
    pub id: TemplateId,

    /// Human-readable name for the template.
    pub name: String,

    /// Description of the template's purpose and style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Game system this template is designed for (e.g., "dnd5e", "pf2e").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_system: Option<String>,

    /// Setting name (e.g., "Forgotten Realms", "Eberron").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setting_name: Option<String>,

    /// Whether this is a built-in template (not user-editable).
    #[serde(default)]
    pub is_builtin: bool,

    /// Base profile ID to extend (e.g., "storyteller", "rules_lawyer").
    pub base_profile: PersonalityId,

    /// Setting-specific vocabulary with usage frequency weights (0.0-1.0).
    /// Higher frequency = more likely to appear in generated text.
    #[serde(default)]
    pub vocabulary: HashMap<String, f32>,

    /// Common phrases characteristic of this setting.
    #[serde(default)]
    pub common_phrases: Vec<String>,

    /// Deity references appropriate for this setting.
    #[serde(default)]
    pub deity_references: Vec<String>,

    /// Tags for categorization and search.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Tone dimension overrides (dimension name -> intensity 0.0-1.0).
    #[serde(default)]
    pub tone_overrides: HashMap<String, f32>,

    /// Cultural markers and setting-specific elements.
    #[serde(default)]
    pub cultural_markers: Vec<String>,

    /// Campaign ID if this template is campaign-specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign_id: Option<String>,

    /// Flattened vocabulary keys for Meilisearch search.
    /// Automatically populated during validation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vocabulary_keys: Vec<String>,

    /// Creation timestamp (RFC 3339 format).
    pub created_at: String,

    /// Last update timestamp (RFC 3339 format).
    pub updated_at: String,
}

impl SettingTemplate {
    /// Create a new template with the given name and base profile.
    pub fn new(name: impl Into<String>, base_profile: impl Into<PersonalityId>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: TemplateId::generate(),
            name: name.into(),
            description: None,
            game_system: None,
            setting_name: None,
            is_builtin: false,
            base_profile: base_profile.into(),
            vocabulary: HashMap::new(),
            common_phrases: Vec::new(),
            deity_references: Vec::new(),
            tags: Vec::new(),
            tone_overrides: HashMap::new(),
            cultural_markers: Vec::new(),
            campaign_id: None,
            vocabulary_keys: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Create a builder for constructing a template.
    pub fn builder(
        name: impl Into<String>,
        base_profile: impl Into<PersonalityId>,
    ) -> SettingTemplateBuilder {
        SettingTemplateBuilder::new(name, base_profile)
    }

    /// Validate the template with the default configuration.
    ///
    /// Returns `Ok(())` if valid, or `Err(TemplateError::ValidationError)` with details.
    pub fn validate(&self) -> Result<(), TemplateError> {
        self.validate_with_config(&TemplateValidationConfig::default())
    }

    /// Validate the template with a custom configuration.
    pub fn validate_with_config(&self, config: &TemplateValidationConfig) -> Result<(), TemplateError> {
        let mut errors = Vec::new();

        // ID validation
        if self.id.as_str().is_empty() {
            errors.push("id cannot be empty".to_string());
        }

        // Name validation
        if self.name.is_empty() {
            errors.push("name cannot be empty".to_string());
        } else if self.name.len() < config.min_name_length {
            errors.push(format!(
                "name must be at least {} characters, found {}",
                config.min_name_length,
                self.name.len()
            ));
        } else if self.name.len() > config.max_name_length {
            errors.push(format!(
                "name exceeds maximum length of {} characters",
                config.max_name_length
            ));
        }

        // Description validation
        if let Some(desc) = &self.description {
            if config.min_description_length > 0 && desc.len() < config.min_description_length {
                errors.push(format!(
                    "description must be at least {} characters, found {}",
                    config.min_description_length,
                    desc.len()
                ));
            } else if desc.len() > config.max_description_length {
                errors.push(format!(
                    "description exceeds maximum length of {} characters",
                    config.max_description_length
                ));
            }
        }

        // Base profile validation
        if self.base_profile.as_str().is_empty() {
            errors.push("base_profile cannot be empty".to_string());
        }

        // Game system validation
        if config.require_game_system && self.game_system.is_none() {
            errors.push("game_system is required".to_string());
        }

        // Setting name validation
        if config.require_setting_name && self.setting_name.is_none() {
            errors.push("setting_name is required".to_string());
        }

        // Vocabulary validation
        if self.vocabulary.len() < config.min_vocabulary_entries {
            errors.push(format!(
                "vocabulary requires at least {} entries, found {}",
                config.min_vocabulary_entries,
                self.vocabulary.len()
            ));
        }

        // Validate vocabulary frequencies are in range
        for (term, freq) in &self.vocabulary {
            if *freq < 0.0 || *freq > 1.0 {
                errors.push(format!(
                    "vocabulary frequency for '{}' is out of range [0.0, 1.0]: {}",
                    term, freq
                ));
            }
        }

        // Common phrases validation
        if self.common_phrases.len() < config.min_common_phrases {
            errors.push(format!(
                "common_phrases requires at least {} entries, found {}",
                config.min_common_phrases,
                self.common_phrases.len()
            ));
        }

        // Validate phrases are non-empty
        for (i, phrase) in self.common_phrases.iter().enumerate() {
            if phrase.trim().is_empty() {
                errors.push(format!("common_phrases[{}] is empty", i));
            }
        }

        // Deity references validation
        if config.min_deity_references > 0
            && self.deity_references.len() < config.min_deity_references
        {
            errors.push(format!(
                "deity_references requires at least {} entries, found {}",
                config.min_deity_references,
                self.deity_references.len()
            ));
        }

        // Validate tone overrides are in range
        for (dimension, intensity) in &self.tone_overrides {
            if *intensity < 0.0 || *intensity > 1.0 {
                errors.push(format!(
                    "tone_override for '{}' is out of range [0.0, 1.0]: {}",
                    dimension, intensity
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(TemplateError::ValidationError {
                template_id: self.id.to_string(),
                message: errors.join("; "),
            })
        }
    }

    /// Update the vocabulary_keys field from the vocabulary map.
    ///
    /// This should be called before indexing in Meilisearch.
    pub fn update_vocabulary_keys(&mut self) {
        self.vocabulary_keys = self.vocabulary.keys().cloned().collect();
    }

    /// Touch the updated_at timestamp.
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// Mark as a built-in template.
    pub fn mark_builtin(&mut self) {
        self.is_builtin = true;
    }

    /// Convert to a `PersonalityProfile` by applying template overrides to a base profile.
    ///
    /// Creates an independent copy that can be used for blending or direct application.
    pub fn to_personality_profile(&self, base: &PersonalityProfile) -> PersonalityProfile {
        let mut profile = base.clone();

        // Update identification
        profile.id = format!("{}_{}", self.id, base.id);
        profile.name = format!("{} ({})", base.name, self.name);
        profile.source = Some(format!("Template: {}", self.id));

        // Merge common phrases
        let mut phrases = base.speech_patterns.common_phrases.clone();
        phrases.extend(self.common_phrases.clone());
        profile.speech_patterns.common_phrases = phrases;

        // Add vocabulary terms to example phrases (for LLM context)
        let vocab_examples: Vec<String> = self
            .vocabulary
            .iter()
            .filter(|(_, freq)| **freq >= 0.03) // Include frequently used terms
            .map(|(term, _)| term.clone())
            .collect();
        profile.example_phrases.extend(vocab_examples);

        // Add deity references to metadata
        if !self.deity_references.is_empty() {
            profile.metadata.insert(
                "deity_references".to_string(),
                self.deity_references.join(", "),
            );
        }

        // Add cultural markers to metadata
        if !self.cultural_markers.is_empty() {
            profile.metadata.insert(
                "cultural_markers".to_string(),
                self.cultural_markers.join(", "),
            );
        }

        // Add template info to metadata
        profile
            .metadata
            .insert("template_id".to_string(), self.id.to_string());
        profile
            .metadata
            .insert("template_name".to_string(), self.name.clone());
        if let Some(game_system) = &self.game_system {
            profile
                .metadata
                .insert("game_system".to_string(), game_system.clone());
        }
        if let Some(setting_name) = &self.setting_name {
            profile
                .metadata
                .insert("setting_name".to_string(), setting_name.clone());
        }

        // Add tags
        profile.tags.extend(self.tags.clone());
        if let Some(game_system) = &self.game_system {
            if !profile.tags.contains(game_system) {
                profile.tags.push(game_system.clone());
            }
        }
        if let Some(setting_name) = &self.setting_name {
            if !profile.tags.contains(setting_name) {
                profile.tags.push(setting_name.clone());
            }
        }

        // Update timestamps
        profile.updated_at = chrono::Utc::now().to_rfc3339();

        profile
    }
}

impl Default for SettingTemplate {
    fn default() -> Self {
        Self::new("Unnamed Template", PersonalityId::new("default"))
    }
}

// ============================================================================
// Conversion from/to SettingPersonalityTemplate
// ============================================================================

impl From<SettingPersonalityTemplate> for SettingTemplate {
    fn from(template: SettingPersonalityTemplate) -> Self {
        Self {
            id: template.id,
            name: template.name,
            description: template.description,
            game_system: template.game_system,
            setting_name: template.setting_name,
            is_builtin: template.is_builtin,
            base_profile: template.base_profile,
            vocabulary: template.vocabulary,
            common_phrases: template.common_phrases,
            deity_references: Vec::new(), // New field not in original
            tags: template.tags,
            tone_overrides: template.tone_overrides,
            cultural_markers: template.cultural_markers,
            campaign_id: template.campaign_id,
            vocabulary_keys: Vec::new(),
            created_at: template.created_at,
            updated_at: template.updated_at,
        }
    }
}

impl From<SettingTemplate> for SettingPersonalityTemplate {
    fn from(template: SettingTemplate) -> Self {
        Self {
            id: template.id,
            name: template.name,
            description: template.description,
            game_system: template.game_system,
            setting_name: template.setting_name,
            is_builtin: template.is_builtin,
            base_profile: template.base_profile,
            vocabulary: template.vocabulary,
            common_phrases: template.common_phrases,
            tags: template.tags,
            tone_overrides: template.tone_overrides,
            cultural_markers: template.cultural_markers,
            campaign_id: template.campaign_id,
            created_at: template.created_at,
            updated_at: template.updated_at,
        }
    }
}

// ============================================================================
// Template Builder
// ============================================================================

/// Builder for constructing `SettingTemplate` instances.
#[derive(Debug)]
pub struct SettingTemplateBuilder {
    template: SettingTemplate,
}

impl SettingTemplateBuilder {
    /// Create a new builder with required fields.
    pub fn new(name: impl Into<String>, base_profile: impl Into<PersonalityId>) -> Self {
        Self {
            template: SettingTemplate::new(name, base_profile),
        }
    }

    /// Set the template ID (default is auto-generated UUID).
    pub fn id(mut self, id: impl Into<TemplateId>) -> Self {
        self.template.id = id.into();
        self
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.template.description = Some(description.into());
        self
    }

    /// Set the game system.
    pub fn game_system(mut self, game_system: impl Into<String>) -> Self {
        self.template.game_system = Some(game_system.into());
        self
    }

    /// Set the setting name.
    pub fn setting_name(mut self, setting_name: impl Into<String>) -> Self {
        self.template.setting_name = Some(setting_name.into());
        self
    }

    /// Mark as built-in template.
    pub fn builtin(mut self) -> Self {
        self.template.is_builtin = true;
        self
    }

    /// Add a vocabulary entry with frequency.
    pub fn vocabulary(mut self, term: impl Into<String>, frequency: f32) -> Self {
        self.template
            .vocabulary
            .insert(term.into(), frequency.clamp(0.0, 1.0));
        self
    }

    /// Add multiple vocabulary entries from a map.
    pub fn vocabulary_map(mut self, vocab: HashMap<String, f32>) -> Self {
        for (term, freq) in vocab {
            self.template.vocabulary.insert(term, freq.clamp(0.0, 1.0));
        }
        self
    }

    /// Add a common phrase.
    pub fn common_phrase(mut self, phrase: impl Into<String>) -> Self {
        self.template.common_phrases.push(phrase.into());
        self
    }

    /// Add multiple common phrases.
    pub fn common_phrases(mut self, phrases: Vec<String>) -> Self {
        self.template.common_phrases.extend(phrases);
        self
    }

    /// Add a deity reference.
    pub fn deity_reference(mut self, deity: impl Into<String>) -> Self {
        self.template.deity_references.push(deity.into());
        self
    }

    /// Add multiple deity references.
    pub fn deity_references(mut self, deities: Vec<String>) -> Self {
        self.template.deity_references.extend(deities);
        self
    }

    /// Add a tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.template.tags.push(tag.into());
        self
    }

    /// Add multiple tags.
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.template.tags.extend(tags);
        self
    }

    /// Add a tone override.
    pub fn tone_override(mut self, dimension: impl Into<String>, intensity: f32) -> Self {
        self.template
            .tone_overrides
            .insert(dimension.into(), intensity.clamp(0.0, 1.0));
        self
    }

    /// Add a cultural marker.
    pub fn cultural_marker(mut self, marker: impl Into<String>) -> Self {
        self.template.cultural_markers.push(marker.into());
        self
    }

    /// Add multiple cultural markers.
    pub fn cultural_markers(mut self, markers: Vec<String>) -> Self {
        self.template.cultural_markers.extend(markers);
        self
    }

    /// Set the campaign ID.
    pub fn campaign_id(mut self, campaign_id: impl Into<String>) -> Self {
        self.template.campaign_id = Some(campaign_id.into());
        self
    }

    /// Build the template, updating vocabulary_keys.
    pub fn build(mut self) -> SettingTemplate {
        self.template.update_vocabulary_keys();
        self.template
    }

    /// Build and validate the template with default config.
    pub fn build_validated(self) -> Result<SettingTemplate, TemplateError> {
        let template = self.build();
        template.validate()?;
        Ok(template)
    }

    /// Build and validate the template with custom config.
    pub fn build_validated_with_config(
        self,
        config: &TemplateValidationConfig,
    ) -> Result<SettingTemplate, TemplateError> {
        let template = self.build();
        template.validate_with_config(config)?;
        Ok(template)
    }
}

// ============================================================================
// YAML Schema Types (for template_loader.rs)
// ============================================================================

/// YAML representation of a setting template.
///
/// This is the format used in `.yaml` template files.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TemplateYaml {
    /// Template identifier (used as filename base).
    pub id: String,

    /// Human-readable name.
    pub name: String,

    /// Description of the template.
    #[serde(default)]
    pub description: Option<String>,

    /// Game system (e.g., "dnd5e").
    #[serde(default)]
    pub game_system: Option<String>,

    /// Setting name (e.g., "Forgotten Realms").
    #[serde(default)]
    pub setting_name: Option<String>,

    /// Whether this is a built-in template.
    #[serde(default)]
    pub is_builtin: bool,

    /// Base profile to extend.
    pub base_profile: String,

    /// Vocabulary with frequencies.
    #[serde(default)]
    pub vocabulary: HashMap<String, f32>,

    /// Common phrases.
    #[serde(default)]
    pub common_phrases: Vec<String>,

    /// Deity references.
    #[serde(default)]
    pub deity_references: Vec<String>,

    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Tone overrides.
    #[serde(default)]
    pub tone_overrides: HashMap<String, f32>,

    /// Cultural markers.
    #[serde(default)]
    pub cultural_markers: Vec<String>,
}

impl TryFrom<TemplateYaml> for SettingTemplate {
    type Error = TemplateError;

    fn try_from(yaml: TemplateYaml) -> Result<Self, Self::Error> {
        let now = chrono::Utc::now().to_rfc3339();

        let template = SettingTemplate {
            id: TemplateId::new(yaml.id),
            name: yaml.name,
            description: yaml.description,
            game_system: yaml.game_system,
            setting_name: yaml.setting_name,
            is_builtin: yaml.is_builtin,
            base_profile: PersonalityId::new(yaml.base_profile),
            vocabulary: yaml.vocabulary,
            common_phrases: yaml.common_phrases,
            deity_references: yaml.deity_references,
            tags: yaml.tags,
            tone_overrides: yaml.tone_overrides,
            cultural_markers: yaml.cultural_markers,
            campaign_id: None,
            vocabulary_keys: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        };

        Ok(template)
    }
}

impl From<SettingTemplate> for TemplateYaml {
    fn from(template: SettingTemplate) -> Self {
        Self {
            id: template.id.into_inner(),
            name: template.name,
            description: template.description,
            game_system: template.game_system,
            setting_name: template.setting_name,
            is_builtin: template.is_builtin,
            base_profile: template.base_profile.into_inner(),
            vocabulary: template.vocabulary,
            common_phrases: template.common_phrases,
            deity_references: template.deity_references,
            tags: template.tags,
            tone_overrides: template.tone_overrides,
            cultural_markers: template.cultural_markers,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_vocabulary() -> HashMap<String, f32> {
        let mut vocab = HashMap::new();
        vocab.insert("ancient texts".to_string(), 0.05);
        vocab.insert("Mystra's blessing".to_string(), 0.03);
        vocab.insert("arcane knowledge".to_string(), 0.04);
        vocab.insert("magical weave".to_string(), 0.03);
        vocab.insert("eldritch power".to_string(), 0.02);
        vocab.insert("tome of lore".to_string(), 0.03);
        vocab.insert("mystical arts".to_string(), 0.02);
        vocab.insert("the Weave".to_string(), 0.05);
        vocab.insert("divine providence".to_string(), 0.02);
        vocab.insert("Candlekeep records".to_string(), 0.04);
        vocab
    }

    fn sample_phrases() -> Vec<String> {
        vec![
            "As the annals of Candlekeep record".to_string(),
            "The Weave reveals what it will".to_string(),
            "By Mystra's grace".to_string(),
            "The old texts speak of such things".to_string(),
            "As any sage worth their salt knows".to_string(),
        ]
    }

    #[test]
    fn test_template_builder_basic() {
        let template = SettingTemplate::builder("Test Template", "storyteller")
            .game_system("dnd5e")
            .setting_name("Test Setting")
            .tag("test")
            .build();

        assert_eq!(template.name, "Test Template");
        assert_eq!(template.base_profile.as_str(), "storyteller");
        assert_eq!(template.game_system, Some("dnd5e".to_string()));
        assert_eq!(template.setting_name, Some("Test Setting".to_string()));
        assert_eq!(template.tags, vec!["test".to_string()]);
    }

    #[test]
    fn test_template_builder_full() {
        let template = SettingTemplate::builder("Forgotten Realms Sage", "storyteller")
            .description("A scholarly personality for Forgotten Realms campaigns")
            .game_system("dnd5e")
            .setting_name("Forgotten Realms")
            .builtin()
            .vocabulary_map(sample_vocabulary())
            .common_phrases(sample_phrases())
            .deity_reference("Mystra")
            .deity_reference("Oghma")
            .tone_override("scholarly", 0.9)
            .tone_override("mysterious", 0.4)
            .cultural_marker("Uses 'tenday' instead of 'week'")
            .cultural_marker("References Candlekeep frequently")
            .tag("sage")
            .tag("lore")
            .build();

        assert_eq!(template.name, "Forgotten Realms Sage");
        assert!(template.is_builtin);
        assert_eq!(template.vocabulary.len(), 10);
        assert_eq!(template.common_phrases.len(), 5);
        assert_eq!(template.deity_references.len(), 2);
        assert_eq!(template.tone_overrides.len(), 2);
        assert_eq!(template.cultural_markers.len(), 2);
        assert_eq!(template.tags.len(), 2);
        assert_eq!(template.vocabulary_keys.len(), 10);
    }

    #[test]
    fn test_validation_success() {
        let template = SettingTemplate::builder("Valid Template", "storyteller")
            .game_system("dnd5e")
            .setting_name("Test Setting")
            .vocabulary_map(sample_vocabulary())
            .common_phrases(sample_phrases())
            .deity_reference("Test Deity")
            .build();

        assert!(template.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_name() {
        let template = SettingTemplate::builder("", "storyteller").build();

        let err = template.validate().unwrap_err();
        assert!(err.to_string().contains("name cannot be empty"));
    }

    #[test]
    fn test_validation_insufficient_vocabulary() {
        let template = SettingTemplate::builder("Test", "storyteller")
            .vocabulary("term1", 0.5)
            .common_phrases(sample_phrases())
            .deity_reference("Deity")
            .build();

        let err = template.validate().unwrap_err();
        assert!(err.to_string().contains("vocabulary requires at least"));
    }

    #[test]
    fn test_validation_insufficient_phrases() {
        let template = SettingTemplate::builder("Test", "storyteller")
            .vocabulary_map(sample_vocabulary())
            .common_phrase("Only one phrase")
            .deity_reference("Deity")
            .build();

        let err = template.validate().unwrap_err();
        assert!(err.to_string().contains("common_phrases requires at least"));
    }

    #[test]
    fn test_validation_out_of_range_frequency() {
        let mut template = SettingTemplate::builder("Test", "storyteller")
            .vocabulary_map(sample_vocabulary())
            .common_phrases(sample_phrases())
            .deity_reference("Deity")
            .build();

        // Manually insert invalid frequency
        template.vocabulary.insert("bad_term".to_string(), 1.5);

        let err = template.validate().unwrap_err();
        assert!(err.to_string().contains("out of range"));
    }

    #[test]
    fn test_validation_lenient_config() {
        let template = SettingTemplate::builder("Minimal Template", "storyteller").build();

        // Should fail with default config
        assert!(template.validate().is_err());

        // Should pass with lenient config
        assert!(template
            .validate_with_config(&TemplateValidationConfig::lenient())
            .is_ok());
    }

    #[test]
    fn test_validation_strict_config() {
        let template = SettingTemplate::builder("Test", "storyteller")
            .vocabulary_map(sample_vocabulary())
            .common_phrases(sample_phrases())
            .deity_reference("Deity")
            .build();

        // Should pass with default config (no game_system/setting_name required)
        assert!(template.validate().is_ok());

        // Should fail with strict config (requires game_system/setting_name)
        assert!(template
            .validate_with_config(&TemplateValidationConfig::strict())
            .is_err());
    }

    #[test]
    fn test_build_validated() {
        let result = SettingTemplate::builder("Valid", "storyteller")
            .vocabulary_map(sample_vocabulary())
            .common_phrases(sample_phrases())
            .deity_reference("Deity")
            .build_validated();

        assert!(result.is_ok());
    }

    #[test]
    fn test_build_validated_failure() {
        let result = SettingTemplate::builder("", "storyteller").build_validated();

        assert!(result.is_err());
    }

    #[test]
    fn test_update_vocabulary_keys() {
        let mut template = SettingTemplate::new("Test", "base");
        template.vocabulary.insert("key1".to_string(), 0.5);
        template.vocabulary.insert("key2".to_string(), 0.3);

        assert!(template.vocabulary_keys.is_empty());

        template.update_vocabulary_keys();

        assert_eq!(template.vocabulary_keys.len(), 2);
        assert!(template.vocabulary_keys.contains(&"key1".to_string()));
        assert!(template.vocabulary_keys.contains(&"key2".to_string()));
    }

    #[test]
    fn test_yaml_roundtrip() {
        let template = SettingTemplate::builder("Test Template", "storyteller")
            .game_system("dnd5e")
            .setting_name("Test Setting")
            .vocabulary("term1", 0.5)
            .common_phrase("Test phrase")
            .deity_reference("Test Deity")
            .tag("test")
            .build();

        let yaml: TemplateYaml = template.clone().into();
        let roundtrip: SettingTemplate = yaml.try_into().unwrap();

        assert_eq!(roundtrip.name, template.name);
        assert_eq!(roundtrip.base_profile.as_str(), template.base_profile.as_str());
        assert_eq!(roundtrip.game_system, template.game_system);
        assert_eq!(roundtrip.vocabulary.len(), template.vocabulary.len());
    }

    #[test]
    fn test_conversion_from_setting_personality_template() {
        let spt = SettingPersonalityTemplate::new("Old Template", PersonalityId::new("base"))
            .with_game_system("dnd5e")
            .with_vocabulary("term", 0.5)
            .with_common_phrase("phrase");

        let template: SettingTemplate = spt.into();

        assert_eq!(template.name, "Old Template");
        assert_eq!(template.game_system, Some("dnd5e".to_string()));
        assert!(template.deity_references.is_empty()); // New field
    }

    #[test]
    fn test_conversion_to_setting_personality_template() {
        let template = SettingTemplate::builder("New Template", "base")
            .game_system("pf2e")
            .deity_reference("Pharasma")
            .build();

        let spt: SettingPersonalityTemplate = template.into();

        assert_eq!(spt.name, "New Template");
        assert_eq!(spt.game_system, Some("pf2e".to_string()));
        // deity_references is not in SettingPersonalityTemplate
    }
}
