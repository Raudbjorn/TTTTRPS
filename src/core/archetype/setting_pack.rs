//! Setting Pack data models for the Archetype Registry.
//!
//! Setting packs provide a way to bundle archetype overrides and custom content
//! for specific game settings (e.g., Forgotten Realms, Eberron, homebrew worlds).
//!
//! # Overview
//!
//! A setting pack can:
//! - Override fields on existing base archetypes
//! - Define custom archetypes scoped to the setting
//! - Add or modify vocabulary banks for NPC speech
//! - Define custom naming cultures
//!
//! # Lifecycle
//!
//! Setting packs follow a **load-then-activate** pattern:
//!
//! 1. **Load**: Parse and validate the pack from YAML/JSON
//! 2. **Register**: Make the pack available in the registry
//! 3. **Activate**: Enable the pack for a specific campaign
//!
//! Loading a pack does not affect any campaigns until explicitly activated.
//!
//! # Examples
//!
//! ```yaml
//! id: "forgotten_realms"
//! name: "Forgotten Realms"
//! description: "Setting pack for the Forgotten Realms campaign setting"
//! gameSystem: "dnd5e"
//! version: "1.0.0"
//!
//! archetypeOverrides:
//!   dwarf:
//!     displayName: "Shield Dwarf"
//!     personalityAffinityAdditions:
//!       - traitId: "clan_loyalty"
//!         weight: 0.9
//!
//! customArchetypes:
//!   - id: "waterdeep_noble"
//!     displayName: "Waterdeep Noble"
//!     category: setting
//!     # ... full archetype definition
//!
//! namingCultures:
//!   - cultureId: "chondathan"
//!     displayName: "Chondathan"
//!     prefixes: ["Aer", "Del", "Mar"]
//!     # ... name components
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::error::{ArchetypeError, Result};
use super::types::{
    Archetype, NamingCultureWeight, PersonalityAffinity, StatTendencies,
};

// ============================================================================
// SettingPack - Main container for setting-specific content
// ============================================================================

/// A setting pack containing archetype overrides and additions for a game setting.
///
/// Setting packs allow customization of archetype behavior for specific campaign
/// settings without modifying the base archetype definitions.
///
/// # Fields
///
/// - `id`: Unique identifier (e.g., "forgotten_realms", "eberron")
/// - `name`: Human-readable name
/// - `description`: Optional detailed description
/// - `game_system`: Target game system (e.g., "dnd5e", "pathfinder2e")
/// - `version`: Semantic version for tracking updates
/// - `archetype_overrides`: Modifications to base archetypes
/// - `custom_archetypes`: New archetypes scoped to this setting
/// - `vocabulary_overrides`: Modifications to vocabulary banks
/// - `vocabulary_banks`: New vocabulary banks for this setting
/// - `naming_cultures`: Custom naming cultures for this setting
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingPack {
    /// Unique identifier (e.g., "forgotten_realms", "eberron").
    pub id: String,

    /// Human-readable name.
    pub name: String,

    /// Optional description of the setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Game system this pack is designed for.
    ///
    /// Examples: "dnd5e", "pathfinder2e", "generic"
    pub game_system: String,

    /// Semantic version (MAJOR.MINOR.PATCH).
    pub version: String,

    /// Author or source of the setting pack.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// URL for more information or updates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Archetype overrides keyed by base archetype ID.
    ///
    /// Each entry modifies the corresponding base archetype when
    /// this setting pack is active.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub archetype_overrides: HashMap<String, ArchetypeOverride>,

    /// Custom archetypes scoped to this setting.
    ///
    /// These archetypes are only available when this pack is active.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_archetypes: Vec<Archetype>,

    /// Vocabulary bank overrides keyed by bank ID.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub vocabulary_overrides: HashMap<String, VocabularyBankOverride>,

    /// Custom vocabulary banks for this setting.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vocabulary_banks: Vec<VocabularyBankDefinition>,

    /// Custom naming cultures defined in this setting.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub naming_cultures: Vec<CustomNamingCulture>,

    /// Tags for categorization and search.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Last update timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl SettingPack {
    /// Create a new setting pack with minimal required fields.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier
    /// * `name` - Human-readable name
    /// * `game_system` - Target game system
    /// * `version` - Semantic version string
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        game_system: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            game_system: game_system.into(),
            version: version.into(),
            author: None,
            url: None,
            archetype_overrides: HashMap::new(),
            custom_archetypes: Vec::new(),
            vocabulary_overrides: HashMap::new(),
            vocabulary_banks: Vec::new(),
            naming_cultures: Vec::new(),
            tags: Vec::new(),
            created_at: Some(now),
            updated_at: Some(now),
        }
    }

    /// Builder method to set description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Builder method to set author.
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Builder method to add archetype overrides.
    pub fn with_archetype_override(
        mut self,
        base_id: impl Into<String>,
        override_def: ArchetypeOverride,
    ) -> Self {
        self.archetype_overrides.insert(base_id.into(), override_def);
        self
    }

    /// Builder method to add a custom archetype.
    pub fn with_custom_archetype(mut self, archetype: Archetype) -> Self {
        self.custom_archetypes.push(archetype);
        self
    }

    /// Builder method to add a custom naming culture.
    pub fn with_naming_culture(mut self, culture: CustomNamingCulture) -> Self {
        self.naming_cultures.push(culture);
        self
    }

    /// Builder method to add tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Validate the setting pack for completeness and correctness.
    ///
    /// # Validation Rules
    ///
    /// - ID must not be empty
    /// - Name must not be empty
    /// - Version must be valid semantic version (MAJOR.MINOR.PATCH)
    /// - All custom archetypes must be valid
    /// - All naming cultures must have valid IDs
    pub fn validate(&self) -> Result<()> {
        // Validate required fields
        if self.id.is_empty() {
            return Err(ArchetypeError::SettingPackInvalid {
                pack_id: "unknown".to_string(),
                reason: "Pack ID is required".to_string(),
            });
        }

        if self.name.is_empty() {
            return Err(ArchetypeError::SettingPackInvalid {
                pack_id: self.id.clone(),
                reason: "Pack name is required".to_string(),
            });
        }

        if self.game_system.is_empty() {
            return Err(ArchetypeError::SettingPackInvalid {
                pack_id: self.id.clone(),
                reason: "Game system is required".to_string(),
            });
        }

        // Validate semantic version format
        if !is_valid_semver(&self.version) {
            return Err(ArchetypeError::SettingPackInvalid {
                pack_id: self.id.clone(),
                reason: format!("Invalid version format: '{}' (expecting MAJOR.MINOR.PATCH)", self.version),
            });
        }

        // Validate custom archetypes
        for archetype in &self.custom_archetypes {
            archetype.validate().map_err(|e| ArchetypeError::SettingPackInvalid {
                pack_id: self.id.clone(),
                reason: format!("Invalid custom archetype '{}': {}", archetype.id, e),
            })?;
        }

        // Validate naming cultures
        for culture in &self.naming_cultures {
            culture.validate().map_err(|e| ArchetypeError::SettingPackInvalid {
                pack_id: self.id.clone(),
                reason: format!("Invalid naming culture '{}': {}", culture.culture_id, e),
            })?;
        }

        // Validate archetype overrides
        for (base_id, override_def) in &self.archetype_overrides {
            override_def.validate().map_err(|e| ArchetypeError::SettingPackInvalid {
                pack_id: self.id.clone(),
                reason: format!("Invalid override for archetype '{}': {}", base_id, e),
            })?;
        }

        Ok(())
    }

    /// Get the count of all content items in this pack.
    pub fn content_count(&self) -> SettingPackStats {
        SettingPackStats {
            archetype_overrides: self.archetype_overrides.len(),
            custom_archetypes: self.custom_archetypes.len(),
            vocabulary_overrides: self.vocabulary_overrides.len(),
            vocabulary_banks: self.vocabulary_banks.len(),
            naming_cultures: self.naming_cultures.len(),
        }
    }

    /// Check if the pack has any content.
    pub fn is_empty(&self) -> bool {
        self.archetype_overrides.is_empty()
            && self.custom_archetypes.is_empty()
            && self.vocabulary_overrides.is_empty()
            && self.vocabulary_banks.is_empty()
            && self.naming_cultures.is_empty()
    }

    /// Update the `updated_at` timestamp to now.
    pub fn touch(&mut self) {
        self.updated_at = Some(chrono::Utc::now());
    }
}

/// Statistics about a setting pack's content.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingPackStats {
    pub archetype_overrides: usize,
    pub custom_archetypes: usize,
    pub vocabulary_overrides: usize,
    pub vocabulary_banks: usize,
    pub naming_cultures: usize,
}

impl SettingPackStats {
    /// Get total count of all content items.
    pub fn total(&self) -> usize {
        self.archetype_overrides
            + self.custom_archetypes
            + self.vocabulary_overrides
            + self.vocabulary_banks
            + self.naming_cultures
    }
}

// ============================================================================
// ArchetypeOverride - Modifications to base archetypes
// ============================================================================

/// Override specification for a base archetype in a setting pack.
///
/// Allows selective modification of archetype fields using merge or replacement
/// semantics. Fields not specified in the override retain their base values.
///
/// # Override Semantics
///
/// - **Scalar fields** (`display_name`, `description`): Direct replacement
/// - **Affinity additions**: Merged with base, overlay values override duplicates
/// - **Affinity replacement**: Completely replaces base affinities
/// - **Nullified fields**: Explicitly removes fields from the resolved archetype
///
/// # Examples
///
/// ```yaml
/// dwarf:
///   displayName: "Shield Dwarf"  # Replaces base display name
///   personalityAffinityAdditions:
///     - traitId: "clan_loyalty"
///       weight: 0.9
///   nullifiedFields:
///     - "some_deprecated_field"
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchetypeOverride {
    /// Override for display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Override for description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Additional personality affinities to merge with base.
    ///
    /// These are added to the base affinities. If a trait_id already exists
    /// in the base, the overlay value takes precedence.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub personality_affinity_additions: Vec<PersonalityAffinity>,

    /// Complete replacement for personality affinities.
    ///
    /// If specified, completely replaces the base archetype's affinities
    /// instead of merging.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub personality_affinity_replacement: Option<Vec<PersonalityAffinity>>,

    /// Override for vocabulary bank reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vocabulary_bank_id: Option<String>,

    /// Override for naming cultures.
    ///
    /// If specified, replaces the base archetype's naming cultures.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub naming_cultures: Option<Vec<NamingCultureWeight>>,

    /// Override for stat tendencies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stat_tendencies: Option<StatTendencies>,

    /// Additional tags to add.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_tags: Vec<String>,

    /// Fields explicitly set to null (removed from resolved archetype).
    ///
    /// Valid field names:
    /// - `"vocabulary_bank"` or `"vocabularyBank"`
    /// - `"stat_tendencies"` or `"statTendencies"`
    /// - `"personality_affinity"` or `"personalityAffinity"`
    /// - `"npc_role_mapping"` or `"npcRoleMapping"`
    /// - `"naming_cultures"` or `"namingCultures"`
    /// - `"description"`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nullified_fields: Vec<String>,
}

impl ArchetypeOverride {
    /// Create an empty override (no changes).
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder method to set display name override.
    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    /// Builder method to set description override.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Builder method to add personality affinity additions.
    pub fn with_affinity_additions(mut self, affinities: Vec<PersonalityAffinity>) -> Self {
        self.personality_affinity_additions = affinities;
        self
    }

    /// Builder method to set complete affinity replacement.
    pub fn with_affinity_replacement(mut self, affinities: Vec<PersonalityAffinity>) -> Self {
        self.personality_affinity_replacement = Some(affinities);
        self
    }

    /// Builder method to set vocabulary bank override.
    pub fn with_vocabulary_bank(mut self, bank_id: impl Into<String>) -> Self {
        self.vocabulary_bank_id = Some(bank_id.into());
        self
    }

    /// Builder method to set naming culture override.
    pub fn with_naming_cultures(mut self, cultures: Vec<NamingCultureWeight>) -> Self {
        self.naming_cultures = Some(cultures);
        self
    }

    /// Builder method to add nullified fields.
    pub fn with_nullified_fields(mut self, fields: Vec<String>) -> Self {
        self.nullified_fields = fields;
        self
    }

    /// Check if this override has any effect.
    pub fn is_empty(&self) -> bool {
        self.display_name.is_none()
            && self.description.is_none()
            && self.personality_affinity_additions.is_empty()
            && self.personality_affinity_replacement.is_none()
            && self.vocabulary_bank_id.is_none()
            && self.naming_cultures.is_none()
            && self.stat_tendencies.is_none()
            && self.additional_tags.is_empty()
            && self.nullified_fields.is_empty()
    }

    /// Check if this override uses replacement semantics for affinities.
    pub fn uses_affinity_replacement(&self) -> bool {
        self.personality_affinity_replacement.is_some()
    }

    /// Validate the override.
    pub fn validate(&self) -> Result<()> {
        // Validate affinity additions
        for affinity in &self.personality_affinity_additions {
            affinity.validate()?;
        }

        // Validate affinity replacement if present
        if let Some(ref replacement) = self.personality_affinity_replacement {
            for affinity in replacement {
                affinity.validate()?;
            }
        }

        // Validate naming cultures if present
        if let Some(ref cultures) = self.naming_cultures {
            for culture in cultures {
                culture.validate()?;
            }
        }

        // Validate nullified field names
        let valid_nullifiable = [
            "vocabulary_bank", "vocabularyBank",
            "stat_tendencies", "statTendencies",
            "personality_affinity", "personalityAffinity",
            "npc_role_mapping", "npcRoleMapping",
            "naming_cultures", "namingCultures",
            "description",
        ];

        for field in &self.nullified_fields {
            if !valid_nullifiable.contains(&field.as_str()) {
                return Err(ArchetypeError::ValidationFailed {
                    reason: format!(
                        "Invalid nullified field: '{}'. Valid fields: {:?}",
                        field, valid_nullifiable
                    ),
                });
            }
        }

        Ok(())
    }

    /// Normalize field names in nullified_fields to snake_case.
    pub fn normalize_nullified_fields(&mut self) {
        self.nullified_fields = self.nullified_fields
            .iter()
            .map(|f| match f.as_str() {
                "vocabularyBank" => "vocabulary_bank".to_string(),
                "statTendencies" => "stat_tendencies".to_string(),
                "personalityAffinity" => "personality_affinity".to_string(),
                "npcRoleMapping" => "npc_role_mapping".to_string(),
                "namingCultures" => "naming_cultures".to_string(),
                other => other.to_string(),
            })
            .collect();
    }
}

// ============================================================================
// VocabularyBankOverride - Modifications to vocabulary banks
// ============================================================================

/// Override specification for a vocabulary bank in a setting pack.
///
/// Allows adding new phrases or removing existing ones from a base
/// vocabulary bank.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VocabularyBankOverride {
    /// Additional phrases by category.
    ///
    /// Key is the phrase category (e.g., "greetings", "farewells").
    /// Value is a list of phrases to add to that category.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub phrase_additions: HashMap<String, Vec<PhraseDefinition>>,

    /// Phrases to remove by text content.
    ///
    /// Key is the phrase category.
    /// Value is a list of phrase texts to remove.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub phrase_removals: HashMap<String, Vec<String>>,

    /// Override display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Override description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl VocabularyBankOverride {
    /// Create an empty override.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add phrases to a category.
    pub fn add_phrases(mut self, category: impl Into<String>, phrases: Vec<PhraseDefinition>) -> Self {
        self.phrase_additions
            .entry(category.into())
            .or_default()
            .extend(phrases);
        self
    }

    /// Remove phrases from a category by text.
    pub fn remove_phrases(mut self, category: impl Into<String>, texts: Vec<String>) -> Self {
        self.phrase_removals
            .entry(category.into())
            .or_default()
            .extend(texts);
        self
    }

    /// Check if this override has any effect.
    pub fn is_empty(&self) -> bool {
        self.phrase_additions.is_empty()
            && self.phrase_removals.is_empty()
            && self.display_name.is_none()
            && self.description.is_none()
    }
}

/// A phrase definition for vocabulary banks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhraseDefinition {
    /// The phrase text.
    pub text: String,

    /// Context tags for filtering.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_tags: Vec<String>,

    /// Formality level (1 = very casual, 10 = very formal).
    #[serde(default = "default_formality", deserialize_with = "deserialize_formality")]
    pub formality: u8,

    /// Tone markers (e.g., "angry", "friendly", "sarcastic").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tone_markers: Vec<String>,

    /// Selection priority (higher = more likely to be selected).
    #[serde(default)]
    pub priority: u8,
}

fn default_formality() -> u8 {
    5
}

fn deserialize_formality<'de, D>(deserializer: D) -> std::result::Result<u8, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = u8::deserialize(deserializer)?;
    Ok(value.clamp(1, 10))
}

impl PhraseDefinition {
    /// Create a new phrase with default settings.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            context_tags: Vec::new(),
            formality: default_formality(),
            tone_markers: Vec::new(),
            priority: 0,
        }
    }

    /// Create with specified formality.
    pub fn with_formality(mut self, formality: u8) -> Self {
        self.formality = formality.clamp(1, 10);
        self
    }

    /// Add tone markers.
    pub fn with_tone(mut self, markers: Vec<String>) -> Self {
        self.tone_markers = markers;
        self
    }
}

/// A vocabulary bank definition (for custom banks in setting packs).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VocabularyBankDefinition {
    /// Unique identifier.
    pub id: String,

    /// Human-readable name.
    pub display_name: String,

    /// Description of the vocabulary bank.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Culture context (e.g., "dwarvish", "common").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub culture: Option<String>,

    /// Role context (e.g., "merchant", "guard").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Race context (e.g., "dwarf", "elf").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub race: Option<String>,

    /// Phrases organized by category.
    #[serde(default)]
    pub phrases: HashMap<String, Vec<PhraseDefinition>>,

    /// Schema version.
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

impl VocabularyBankDefinition {
    /// Create a new vocabulary bank.
    pub fn new(id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            description: None,
            culture: None,
            role: None,
            race: None,
            phrases: HashMap::new(),
            version: default_version(),
        }
    }

    /// Add phrases to a category (extends existing phrases).
    pub fn add_phrases(mut self, category: impl Into<String>, phrases: Vec<PhraseDefinition>) -> Self {
        self.phrases
            .entry(category.into())
            .or_default()
            .extend(phrases);
        self
    }

    /// Check if the bank is empty.
    pub fn is_empty(&self) -> bool {
        self.phrases.is_empty()
    }

    /// Get total phrase count across all categories.
    pub fn phrase_count(&self) -> usize {
        self.phrases.values().map(|v| v.len()).sum()
    }
}

// ============================================================================
// CustomNamingCulture - Setting-specific naming patterns
// ============================================================================

/// Custom naming culture defined in a setting pack.
///
/// Defines name components and patterns for a culture-specific naming system.
///
/// # Examples
///
/// ```yaml
/// cultureId: "chondathan"
/// displayName: "Chondathan"
/// prefixes:
///   - "Aer"
///   - "Del"
///   - "Mar"
/// middles:
///   - "an"
///   - "en"
///   - "il"
/// suffixesMale:
///   - "us"
///   - "or"
/// suffixesFemale:
///   - "a"
///   - "ia"
/// suffixesNeutral:
///   - "is"
///   - "el"
/// familyPatterns:
///   - "{given} of {place}"
///   - "{given} {family}"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomNamingCulture {
    /// Culture identifier.
    ///
    /// Must not conflict with built-in culture IDs.
    pub culture_id: String,

    /// Human-readable display name.
    pub display_name: String,

    /// Description of the naming culture.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Name prefixes (beginning syllables).
    #[serde(default)]
    pub prefixes: Vec<String>,

    /// Name middles (middle syllables).
    #[serde(default)]
    pub middles: Vec<String>,

    /// Suffixes for masculine names.
    #[serde(default)]
    pub suffixes_male: Vec<String>,

    /// Suffixes for feminine names.
    #[serde(default)]
    pub suffixes_female: Vec<String>,

    /// Suffixes for gender-neutral names.
    #[serde(default)]
    pub suffixes_neutral: Vec<String>,

    /// Family/clan name patterns.
    ///
    /// Placeholders:
    /// - `{given}` - The given name
    /// - `{family}` - A family name
    /// - `{clan}` - A clan name
    /// - `{place}` - A place name
    /// - `{parent}` - A parent's name
    #[serde(default)]
    pub family_patterns: Vec<String>,

    /// Title patterns.
    ///
    /// List of titles that may be used with this culture.
    #[serde(default)]
    pub titles: Vec<String>,

    /// Epithet patterns.
    ///
    /// List of epithets that may be used with this culture.
    #[serde(default)]
    pub epithets: Vec<String>,

    /// Complete name examples for reference.
    #[serde(default)]
    pub examples: Vec<String>,
}

impl CustomNamingCulture {
    /// Create a new custom naming culture.
    pub fn new(culture_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            culture_id: culture_id.into(),
            display_name: display_name.into(),
            description: None,
            prefixes: Vec::new(),
            middles: Vec::new(),
            suffixes_male: Vec::new(),
            suffixes_female: Vec::new(),
            suffixes_neutral: Vec::new(),
            family_patterns: Vec::new(),
            titles: Vec::new(),
            epithets: Vec::new(),
            examples: Vec::new(),
        }
    }

    /// Builder method to set name components.
    pub fn with_components(
        mut self,
        prefixes: Vec<String>,
        middles: Vec<String>,
        suffixes_male: Vec<String>,
        suffixes_female: Vec<String>,
        suffixes_neutral: Vec<String>,
    ) -> Self {
        self.prefixes = prefixes;
        self.middles = middles;
        self.suffixes_male = suffixes_male;
        self.suffixes_female = suffixes_female;
        self.suffixes_neutral = suffixes_neutral;
        self
    }

    /// Builder method to set family patterns.
    pub fn with_family_patterns(mut self, patterns: Vec<String>) -> Self {
        self.family_patterns = patterns;
        self
    }

    /// Validate the naming culture.
    pub fn validate(&self) -> Result<()> {
        if self.culture_id.is_empty() {
            return Err(ArchetypeError::ValidationFailed {
                reason: "Culture ID cannot be empty".to_string(),
            });
        }

        if self.display_name.is_empty() {
            return Err(ArchetypeError::ValidationFailed {
                reason: "Culture display name cannot be empty".to_string(),
            });
        }

        // Check for at least some name components
        if self.prefixes.is_empty()
            && self.suffixes_male.is_empty()
            && self.suffixes_female.is_empty()
            && self.suffixes_neutral.is_empty()
        {
            return Err(ArchetypeError::ValidationFailed {
                reason: format!(
                    "Naming culture '{}' must have at least some name components (prefixes or suffixes)",
                    self.culture_id
                ),
            });
        }

        Ok(())
    }

    /// Check if the culture has masculine-specific suffixes.
    pub fn has_gendered_suffixes(&self) -> bool {
        !self.suffixes_male.is_empty() || !self.suffixes_female.is_empty()
    }

    /// Get all suffixes for a given gender.
    pub fn suffixes_for_gender(&self, gender: &str) -> Vec<&String> {
        match gender.to_lowercase().as_str() {
            "male" | "masculine" | "m" => self.suffixes_male.iter().collect(),
            "female" | "feminine" | "f" => self.suffixes_female.iter().collect(),
            _ => self.suffixes_neutral.iter().collect(),
        }
    }
}

// ============================================================================
// SettingPackSummary - Lightweight listing type
// ============================================================================

/// Lightweight summary of a setting pack for listing and search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingPackSummary {
    /// Unique identifier.
    pub id: String,

    /// Human-readable name.
    pub name: String,

    /// Game system.
    pub game_system: String,

    /// Version.
    pub version: String,

    /// Author if known.
    pub author: Option<String>,

    /// Content statistics.
    pub stats: SettingPackStats,

    /// Tags.
    pub tags: Vec<String>,
}

impl From<&SettingPack> for SettingPackSummary {
    fn from(pack: &SettingPack) -> Self {
        Self {
            id: pack.id.clone(),
            name: pack.name.clone(),
            game_system: pack.game_system.clone(),
            version: pack.version.clone(),
            author: pack.author.clone(),
            stats: pack.content_count(),
            tags: pack.tags.clone(),
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Validate a semantic version string (MAJOR.MINOR.PATCH).
///
/// # Examples
///
/// ```
/// use ttrpg_assistant::core::archetype::setting_pack::is_valid_semver;
/// assert!(is_valid_semver("1.0.0"));
/// assert!(is_valid_semver("2.10.5"));
/// assert!(!is_valid_semver("1.0"));
/// assert!(!is_valid_semver("v1.0.0"));
/// ```
pub fn is_valid_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    // Check that each part is a non-empty string of digits and parses to u32
    parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) && p.parse::<u32>().is_ok())
}

/// Parse a semantic version into (major, minor, patch) components.
///
/// # Returns
///
/// `Some((major, minor, patch))` if valid, `None` otherwise.
pub fn parse_semver(version: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return None;
    }

    let major = parts[0].parse().ok()?;
    let minor = parts[1].parse().ok()?;
    let patch = parts[2].parse().ok()?;

    Some((major, minor, patch))
}

/// Compare two semantic versions.
///
/// # Returns
///
/// - `Ordering::Less` if a < b
/// - `Ordering::Equal` if a == b
/// - `Ordering::Greater` if a > b
/// - `None` if either version is invalid
pub fn compare_semver(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let (a_maj, a_min, a_pat) = parse_semver(a)?;
    let (b_maj, b_min, b_pat) = parse_semver(b)?;

    Some((a_maj, a_min, a_pat).cmp(&(b_maj, b_min, b_pat)))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // SettingPack tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_setting_pack_creation() {
        let pack = SettingPack::new(
            "forgotten_realms",
            "Forgotten Realms",
            "dnd5e",
            "1.0.0",
        );

        assert_eq!(pack.id, "forgotten_realms");
        assert_eq!(pack.name, "Forgotten Realms");
        assert_eq!(pack.game_system, "dnd5e");
        assert_eq!(pack.version, "1.0.0");
        assert!(pack.is_empty());
    }

    #[test]
    fn test_setting_pack_builder() {
        let pack = SettingPack::new("test", "Test Pack", "generic", "1.0.0")
            .with_description("A test pack")
            .with_author("Test Author")
            .with_tags(vec!["test".to_string()]);

        assert_eq!(pack.description, Some("A test pack".to_string()));
        assert_eq!(pack.author, Some("Test Author".to_string()));
        assert_eq!(pack.tags, vec!["test".to_string()]);
    }

    #[test]
    fn test_setting_pack_validation_valid() {
        let pack = SettingPack::new("valid_pack", "Valid Pack", "dnd5e", "1.0.0");
        assert!(pack.validate().is_ok());
    }

    #[test]
    fn test_setting_pack_validation_empty_id() {
        let pack = SettingPack::new("", "Name", "dnd5e", "1.0.0");
        let result = pack.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ID"));
    }

    #[test]
    fn test_setting_pack_validation_invalid_version() {
        let pack = SettingPack::new("id", "Name", "dnd5e", "1.0");
        let result = pack.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("version"));
    }

    #[test]
    fn test_setting_pack_stats() {
        let mut pack = SettingPack::new("test", "Test", "generic", "1.0.0");
        pack.archetype_overrides.insert(
            "dwarf".to_string(),
            ArchetypeOverride::new().with_display_name("Shield Dwarf"),
        );
        pack.naming_cultures.push(
            CustomNamingCulture::new("test_culture", "Test Culture")
                .with_components(
                    vec!["Pre".to_string()],
                    vec![],
                    vec!["us".to_string()],
                    vec!["a".to_string()],
                    vec![],
                ),
        );

        let stats = pack.content_count();
        assert_eq!(stats.archetype_overrides, 1);
        assert_eq!(stats.naming_cultures, 1);
        assert_eq!(stats.total(), 2);
        assert!(!pack.is_empty());
    }

    // -------------------------------------------------------------------------
    // ArchetypeOverride tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_archetype_override_creation() {
        let override_def = ArchetypeOverride::new();
        assert!(override_def.is_empty());
    }

    #[test]
    fn test_archetype_override_builder() {
        let override_def = ArchetypeOverride::new()
            .with_display_name("Shield Dwarf")
            .with_affinity_additions(vec![PersonalityAffinity::new("clan_loyalty", 0.9)])
            .with_vocabulary_bank("dwarvish_merchant")
            .with_nullified_fields(vec!["description".to_string()]);

        assert!(!override_def.is_empty());
        assert_eq!(override_def.display_name, Some("Shield Dwarf".to_string()));
        assert_eq!(override_def.personality_affinity_additions.len(), 1);
        assert_eq!(override_def.nullified_fields.len(), 1);
    }

    #[test]
    fn test_archetype_override_affinity_replacement() {
        let override_def = ArchetypeOverride::new()
            .with_affinity_replacement(vec![PersonalityAffinity::new("new_trait", 0.8)]);

        assert!(override_def.uses_affinity_replacement());
        assert!(override_def.personality_affinity_replacement.is_some());
    }

    #[test]
    fn test_archetype_override_validation() {
        let valid = ArchetypeOverride::new()
            .with_nullified_fields(vec!["vocabulary_bank".to_string()]);
        assert!(valid.validate().is_ok());

        let invalid = ArchetypeOverride::new()
            .with_nullified_fields(vec!["invalid_field".to_string()]);
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_archetype_override_normalize_nullified() {
        let mut override_def = ArchetypeOverride::new()
            .with_nullified_fields(vec![
                "vocabularyBank".to_string(),
                "statTendencies".to_string(),
            ]);

        override_def.normalize_nullified_fields();

        assert!(override_def.nullified_fields.contains(&"vocabulary_bank".to_string()));
        assert!(override_def.nullified_fields.contains(&"stat_tendencies".to_string()));
    }

    // -------------------------------------------------------------------------
    // VocabularyBankOverride tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_vocabulary_bank_override() {
        let override_def = VocabularyBankOverride::new()
            .add_phrases("greetings", vec![PhraseDefinition::new("Hail, friend!")])
            .remove_phrases("greetings", vec!["Hello".to_string()]);

        assert!(!override_def.is_empty());
        assert_eq!(override_def.phrase_additions.len(), 1);
        assert_eq!(override_def.phrase_removals.len(), 1);
    }

    // -------------------------------------------------------------------------
    // CustomNamingCulture tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_custom_naming_culture_creation() {
        let culture = CustomNamingCulture::new("chondathan", "Chondathan")
            .with_components(
                vec!["Aer".to_string(), "Del".to_string()],
                vec!["an".to_string()],
                vec!["us".to_string()],
                vec!["a".to_string()],
                vec!["is".to_string()],
            )
            .with_family_patterns(vec!["{given} of {place}".to_string()]);

        assert_eq!(culture.culture_id, "chondathan");
        assert_eq!(culture.prefixes.len(), 2);
        assert!(culture.has_gendered_suffixes());
    }

    #[test]
    fn test_custom_naming_culture_validation() {
        let valid = CustomNamingCulture::new("test", "Test")
            .with_components(
                vec!["Pre".to_string()],
                vec![],
                vec!["us".to_string()],
                vec![],
                vec![],
            );
        assert!(valid.validate().is_ok());

        let invalid_empty_id = CustomNamingCulture::new("", "Test");
        assert!(invalid_empty_id.validate().is_err());

        let invalid_no_components = CustomNamingCulture::new("test", "Test");
        assert!(invalid_no_components.validate().is_err());
    }

    #[test]
    fn test_custom_naming_culture_suffixes_for_gender() {
        let culture = CustomNamingCulture::new("test", "Test")
            .with_components(
                vec![],
                vec![],
                vec!["us".to_string(), "or".to_string()],
                vec!["a".to_string(), "ia".to_string()],
                vec!["is".to_string()],
            );

        assert_eq!(culture.suffixes_for_gender("male").len(), 2);
        assert_eq!(culture.suffixes_for_gender("female").len(), 2);
        assert_eq!(culture.suffixes_for_gender("neutral").len(), 1);
        assert_eq!(culture.suffixes_for_gender("other").len(), 1); // defaults to neutral
    }

    // -------------------------------------------------------------------------
    // Semver helper tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_is_valid_semver() {
        assert!(is_valid_semver("1.0.0"));
        assert!(is_valid_semver("2.10.5"));
        assert!(is_valid_semver("0.0.1"));

        assert!(!is_valid_semver("1.0"));
        assert!(!is_valid_semver("v1.0.0"));
        assert!(!is_valid_semver("1.0.0.0"));
        assert!(!is_valid_semver("1.a.0"));
    }

    #[test]
    fn test_parse_semver() {
        assert_eq!(parse_semver("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_semver("0.0.0"), Some((0, 0, 0)));
        assert_eq!(parse_semver("invalid"), None);
    }

    #[test]
    fn test_compare_semver() {
        use std::cmp::Ordering;

        assert_eq!(compare_semver("1.0.0", "1.0.0"), Some(Ordering::Equal));
        assert_eq!(compare_semver("1.0.0", "2.0.0"), Some(Ordering::Less));
        assert_eq!(compare_semver("2.0.0", "1.0.0"), Some(Ordering::Greater));
        assert_eq!(compare_semver("1.1.0", "1.0.0"), Some(Ordering::Greater));
        assert_eq!(compare_semver("1.0.1", "1.0.0"), Some(Ordering::Greater));
        assert_eq!(compare_semver("invalid", "1.0.0"), None);
    }

    // -------------------------------------------------------------------------
    // Serialization tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_setting_pack_serialization_roundtrip() {
        let mut pack = SettingPack::new("test_pack", "Test Pack", "dnd5e", "1.0.0")
            .with_description("A test pack");

        pack.archetype_overrides.insert(
            "dwarf".to_string(),
            ArchetypeOverride::new()
                .with_display_name("Shield Dwarf")
                .with_affinity_additions(vec![PersonalityAffinity::new("stubborn", 0.9)]),
        );

        pack.naming_cultures.push(
            CustomNamingCulture::new("test_culture", "Test Culture")
                .with_components(
                    vec!["Pre".to_string()],
                    vec!["mid".to_string()],
                    vec!["us".to_string()],
                    vec!["a".to_string()],
                    vec!["is".to_string()],
                ),
        );

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&pack).unwrap();

        // Verify camelCase formatting
        assert!(json.contains("gameSystem"));
        assert!(json.contains("archetypeOverrides"));
        assert!(json.contains("namingCultures"));
        assert!(json.contains("displayName"));
        assert!(json.contains("cultureId"));
        assert!(json.contains("suffixesMale"));

        // Deserialize back
        let deserialized: SettingPack = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, pack.id);
        assert_eq!(deserialized.name, pack.name);
        assert_eq!(deserialized.archetype_overrides.len(), 1);
        assert_eq!(deserialized.naming_cultures.len(), 1);
    }

    #[test]
    fn test_archetype_override_serialization() {
        let override_def = ArchetypeOverride::new()
            .with_display_name("Test Override")
            .with_affinity_additions(vec![PersonalityAffinity::new("test_trait", 0.8)])
            .with_nullified_fields(vec!["vocabulary_bank".to_string()]);

        let json = serde_json::to_string(&override_def).unwrap();
        assert!(json.contains("displayName"));
        assert!(json.contains("personalityAffinityAdditions"));
        assert!(json.contains("nullifiedFields"));

        let deserialized: ArchetypeOverride = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.display_name, override_def.display_name);
    }

    #[test]
    fn test_setting_pack_summary() {
        let mut pack = SettingPack::new("test", "Test Pack", "dnd5e", "1.0.0")
            .with_author("Author")
            .with_tags(vec!["tag1".to_string()]);

        pack.archetype_overrides.insert("a".to_string(), ArchetypeOverride::new());
        pack.archetype_overrides.insert("b".to_string(), ArchetypeOverride::new());

        let summary: SettingPackSummary = (&pack).into();

        assert_eq!(summary.id, "test");
        assert_eq!(summary.name, "Test Pack");
        assert_eq!(summary.author, Some("Author".to_string()));
        assert_eq!(summary.stats.archetype_overrides, 2);
    }
}
