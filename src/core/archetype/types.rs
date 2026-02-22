//! Core data models for the Archetype Registry.
//!
//! This module defines the fundamental types used throughout the archetype system:
//!
//! - [`ArchetypeId`]: Type-safe identifier for archetypes
//! - [`Archetype`]: Complete archetype definition
//! - [`ArchetypeCategory`]: Classification of archetype types
//! - [`PersonalityAffinity`]: Trait-to-weight mapping for personality generation
//! - [`NpcRoleMapping`]: Role assignments with weights
//! - [`NamingCultureWeight`]: Culture selection weights for name generation
//! - [`StatTendencies`]: Stat generation guidance
//!
//! # Design Notes
//!
//! - All types use `#[serde(rename_all = "camelCase")]` for Tauri IPC compatibility
//! - `Arc<str>` is used for `display_name` and frequently repeated strings (REC-ARCH-005)
//! - Weight fields are validated to be in range 0.0-1.0

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use super::error::{ArchetypeError, Result};

// ============================================================================
// ArchetypeId - Type-safe identifier
// ============================================================================

/// Type-safe wrapper for archetype identifiers.
///
/// Provides compile-time safety to prevent mixing archetype IDs with other strings.
///
/// # Examples
///
/// ```rust
/// use ttrpg_assistant::core::archetype::types::ArchetypeId;
///
/// let id = ArchetypeId::new("knight_errant");
/// assert_eq!(id.as_str(), "knight_errant");
///
/// // From string conversion
/// let id2: ArchetypeId = "hedge_witch".into();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArchetypeId(String);

impl ArchetypeId {
    /// Create a new archetype ID from a string.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier string (e.g., "knight_errant", "mountain_dwarf")
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the underlying string reference.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the wrapper and return the inner String.
    #[inline]
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Check if the ID is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Validate the archetype ID format.
    ///
    /// Valid IDs must:
    /// - Not be empty
    /// - Contain only alphanumeric characters, underscores, and hyphens
    /// - Start with a letter or underscore
    ///
    /// # Returns
    ///
    /// `Ok(())` if valid, `Err(ArchetypeError::ValidationFailed)` otherwise.
    pub fn validate(&self) -> Result<()> {
        if self.0.is_empty() {
            return Err(ArchetypeError::ValidationFailed {
                reason: "Archetype ID cannot be empty".to_string(),
            });
        }

        let first = self.0.chars().next().unwrap();
        if !first.is_alphabetic() && first != '_' {
            return Err(ArchetypeError::ValidationFailed {
                reason: format!(
                    "Archetype ID must start with a letter or underscore, got: '{}'",
                    first
                ),
            });
        }

        for ch in self.0.chars() {
            if !ch.is_alphanumeric() && ch != '_' && ch != '-' {
                return Err(ArchetypeError::ValidationFailed {
                    reason: format!(
                        "Archetype ID contains invalid character: '{}' in '{}'",
                        ch, self.0
                    ),
                });
            }
        }

        Ok(())
    }
}

impl fmt::Display for ArchetypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ArchetypeId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for ArchetypeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl AsRef<str> for ArchetypeId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// ArchetypeCategory - Classification enum
// ============================================================================

/// Category of archetype for filtering and resolution ordering.
///
/// Categories determine how archetypes are layered during resolution:
/// - `Role` archetypes form the base layer
/// - `Race` archetypes overlay role
/// - `Class` archetypes overlay race
/// - `Setting` archetypes overlay class
/// - `Custom` archetypes can be used anywhere in the chain
///
/// # Serialization
///
/// Serializes to snake_case for YAML/JSON compatibility:
/// - `Role` -> `"role"`
/// - `Custom("faction")` -> `{"custom": "faction"}`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ArchetypeCategory {
    /// Role-based archetype (merchant, guard, innkeeper, etc.)
    #[default]
    Role,

    /// Race/species archetype (dwarf, elf, human, etc.)
    Race,

    /// Class/profession archetype (wizard, fighter, cleric, etc.)
    Class,

    /// Setting-specific archetype (Candlekeep sage, Waterdeep noble, etc.)
    Setting,

    /// Custom user-defined category with a label.
    Custom(String),
}

impl ArchetypeCategory {
    /// Get the resolution priority for this category.
    ///
    /// Higher values override lower values during hierarchical resolution.
    /// Direct archetype ID lookups always have the highest priority (handled separately).
    ///
    /// # Priority Order (lowest to highest)
    ///
    /// 1. Role (10)
    /// 2. Race (20)
    /// 3. Class (30)
    /// 4. Setting (40)
    /// 5. Custom (25 - between Race and Class)
    pub fn resolution_priority(&self) -> u8 {
        match self {
            Self::Role => 10,
            Self::Race => 20,
            Self::Custom(_) => 25,
            Self::Class => 30,
            Self::Setting => 40,
        }
    }

    /// Check if this is a custom category.
    #[inline]
    pub fn is_custom(&self) -> bool {
        matches!(self, Self::Custom(_))
    }

    /// Get the custom category label, if applicable.
    pub fn custom_label(&self) -> Option<&str> {
        match self {
            Self::Custom(label) => Some(label),
            _ => None,
        }
    }
}


impl fmt::Display for ArchetypeCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Role => write!(f, "role"),
            Self::Race => write!(f, "race"),
            Self::Class => write!(f, "class"),
            Self::Setting => write!(f, "setting"),
            Self::Custom(label) => write!(f, "custom:{}", label),
        }
    }
}

// ============================================================================
// PersonalityAffinity - Trait weight mapping
// ============================================================================

/// Personality trait affinity mapping for character generation.
///
/// Maps a personality trait to a weight (likelihood) and default intensity.
/// Used by the PersonalityBlender to determine which traits an archetype
/// tends toward.
///
/// # Examples
///
/// ```yaml
/// personality_affinity:
///   - traitId: "curious"
///     weight: 0.8
///     defaultIntensity: 7
///   - traitId: "cautious"
///     weight: 0.6
///     defaultIntensity: 5
/// ```
///
/// # Weight Semantics
///
/// - 0.0: Never appears for this archetype
/// - 0.5: Average likelihood
/// - 1.0: Always appears for this archetype
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalityAffinity {
    /// Trait identifier matching PersonalityTrait enum or custom trait.
    pub trait_id: String,

    /// Weight from 0.0 (never) to 1.0 (always).
    ///
    /// Controls likelihood of this trait appearing when generating
    /// a character with this archetype.
    pub weight: f32,

    /// Typical intensity when this trait appears (1-10).
    ///
    /// 1 = subtle/mild manifestation
    /// 5 = moderate (default)
    /// 10 = dominant/extreme manifestation
    #[serde(default = "default_intensity")]
    pub default_intensity: u8,
}

fn default_intensity() -> u8 {
    5
}

impl PersonalityAffinity {
    /// Create a new personality affinity with default intensity.
    ///
    /// # Arguments
    ///
    /// * `trait_id` - The trait identifier
    /// * `weight` - Weight from 0.0 to 1.0
    pub fn new(trait_id: impl Into<String>, weight: f32) -> Self {
        Self {
            trait_id: trait_id.into(),
            weight,
            default_intensity: default_intensity(),
        }
    }

    /// Create with specified intensity.
    pub fn with_intensity(trait_id: impl Into<String>, weight: f32, intensity: u8) -> Self {
        Self {
            trait_id: trait_id.into(),
            weight,
            default_intensity: intensity.clamp(1, 10),
        }
    }

    /// Validate weight and intensity bounds.
    ///
    /// # Errors
    ///
    /// Returns `ArchetypeError::ValidationFailed` if:
    /// - Weight is not in range [0.0, 1.0]
    /// - Intensity is not in range [1, 10]
    pub fn validate(&self) -> Result<()> {
        if self.weight < 0.0 || self.weight > 1.0 {
            return Err(ArchetypeError::ValidationFailed {
                reason: format!(
                    "PersonalityAffinity weight must be 0.0-1.0, got {} for trait '{}'",
                    self.weight, self.trait_id
                ),
            });
        }

        if self.default_intensity < 1 || self.default_intensity > 10 {
            return Err(ArchetypeError::ValidationFailed {
                reason: format!(
                    "PersonalityAffinity intensity must be 1-10, got {} for trait '{}'",
                    self.default_intensity, self.trait_id
                ),
            });
        }

        Ok(())
    }
}

// ============================================================================
// NpcRoleMapping - Role assignment with weight
// ============================================================================

/// NPC role mapping with selection weight and context.
///
/// Maps an archetype to NPC roles, used by the NPCGenerator to determine
/// appropriate roles when generating NPCs from this archetype.
///
/// # Examples
///
/// ```yaml
/// npcRoleMapping:
///   - role: "merchant"
///     weight: 0.9
///     context: "Operates a shop or stall"
///   - role: "informant"
///     weight: 0.3
///     context: "Hears rumors from customers"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcRoleMapping {
    /// Role identifier matching NPCRole enum or custom role.
    pub role: String,

    /// Likelihood of this role (0.0-1.0).
    ///
    /// Higher weight means this role is more appropriate for the archetype.
    pub weight: f32,

    /// Additional context explaining this role mapping.
    ///
    /// Provides guidance for why this role fits the archetype.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

impl NpcRoleMapping {
    /// Create a new role mapping without context.
    pub fn new(role: impl Into<String>, weight: f32) -> Self {
        Self {
            role: role.into(),
            weight,
            context: None,
        }
    }

    /// Create a role mapping with context.
    pub fn with_context(role: impl Into<String>, weight: f32, context: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            weight,
            context: Some(context.into()),
        }
    }

    /// Validate weight bounds.
    pub fn validate(&self) -> Result<()> {
        if self.weight < 0.0 || self.weight > 1.0 {
            return Err(ArchetypeError::ValidationFailed {
                reason: format!(
                    "NpcRoleMapping weight must be 0.0-1.0, got {} for role '{}'",
                    self.weight, self.role
                ),
            });
        }
        Ok(())
    }
}

// ============================================================================
// NamingCultureWeight - Culture selection for name generation
// ============================================================================

/// Naming culture with selection weight and optional pattern overrides.
///
/// Links an archetype to naming cultures used by the NameGenerator,
/// allowing archetypes to influence name generation style.
///
/// # Examples
///
/// ```yaml
/// namingCultures:
///   - culture: "dwarvish"
///     weight: 0.8
///     patternOverrides:
///       titleProbability: 0.3
///       clanFormat: "{name} of Clan {clan}"
///   - culture: "common"
///     weight: 0.2
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NamingCultureWeight {
    /// Culture identifier matching NameCulture enum or custom culture.
    pub culture: String,

    /// Selection weight (0.0-1.0).
    ///
    /// When generating a name for this archetype, cultures are selected
    /// probabilistically based on their weights.
    pub weight: f32,

    /// Optional overrides for naming patterns in this culture.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_overrides: Option<NamingPatternOverrides>,
}

impl NamingCultureWeight {
    /// Create a culture weight without pattern overrides.
    pub fn new(culture: impl Into<String>, weight: f32) -> Self {
        Self {
            culture: culture.into(),
            weight,
            pattern_overrides: None,
        }
    }

    /// Create with pattern overrides.
    pub fn with_overrides(
        culture: impl Into<String>,
        weight: f32,
        overrides: NamingPatternOverrides,
    ) -> Self {
        Self {
            culture: culture.into(),
            weight,
            pattern_overrides: Some(overrides),
        }
    }

    /// Validate weight bounds.
    pub fn validate(&self) -> Result<()> {
        if self.weight < 0.0 || self.weight > 1.0 {
            return Err(ArchetypeError::ValidationFailed {
                reason: format!(
                    "NamingCultureWeight weight must be 0.0-1.0, got {} for culture '{}'",
                    self.weight, self.culture
                ),
            });
        }

        if let Some(ref overrides) = self.pattern_overrides {
            overrides.validate()?;
        }

        Ok(())
    }
}

/// Overrides for naming patterns within a culture.
///
/// Allows archetypes to customize how names are generated for their
/// associated naming cultures.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NamingPatternOverrides {
    /// Probability of including a title (0.0-1.0).
    ///
    /// Examples: "Sir", "Lady", "Master", "Elder"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_probability: Option<f32>,

    /// Probability of including an epithet (0.0-1.0).
    ///
    /// Examples: "the Bold", "Ironbeard", "Shadowwalker"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epithet_probability: Option<f32>,

    /// Format string for clan/family names.
    ///
    /// Placeholders: `{name}`, `{clan}`, `{family}`
    /// Example: `"{name} of Clan {clan}"` -> "Thorin of Clan Oakenshield"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clan_format: Option<String>,

    /// Additional suffix patterns specific to this archetype-culture combination.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_suffixes: Vec<String>,
}

impl NamingPatternOverrides {
    /// Validate probability bounds.
    pub fn validate(&self) -> Result<()> {
        if let Some(prob) = self.title_probability {
            if !(0.0..=1.0).contains(&prob) {
                return Err(ArchetypeError::ValidationFailed {
                    reason: format!("title_probability must be 0.0-1.0, got {}", prob),
                });
            }
        }

        if let Some(prob) = self.epithet_probability {
            if !(0.0..=1.0).contains(&prob) {
                return Err(ArchetypeError::ValidationFailed {
                    reason: format!("epithet_probability must be 0.0-1.0, got {}", prob),
                });
            }
        }

        Ok(())
    }
}

// ============================================================================
// StatTendencies - Stat generation guidance
// ============================================================================

/// Stat generation tendencies for character creation.
///
/// Provides guidance to character generators about how stats should
/// be distributed for this archetype.
///
/// # Examples
///
/// ```yaml
/// statTendencies:
///   modifiers:
///     strength: 2
///     charisma: -1
///   minimums:
///     constitution: 12
///   priorityOrder:
///     - strength
///     - constitution
///     - dexterity
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatTendencies {
    /// Stat modifiers applied during generation.
    ///
    /// Positive values increase the stat, negative decrease.
    /// Example: `{"strength": 2, "charisma": -1}`
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub modifiers: HashMap<String, i32>,

    /// Suggested minimum values for stats.
    ///
    /// Generation should attempt to meet these minimums.
    /// Example: `{"constitution": 12}`
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub minimums: HashMap<String, u8>,

    /// Priority order for point-buy or array allocation.
    ///
    /// Stats listed first receive higher values.
    /// Example: `["strength", "constitution", "dexterity"]`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub priority_order: Vec<String>,
}

impl StatTendencies {
    /// Check if the tendencies are empty (no guidance).
    pub fn is_empty(&self) -> bool {
        self.modifiers.is_empty() && self.minimums.is_empty() && self.priority_order.is_empty()
    }

    /// Get the modifier for a stat, defaulting to 0.
    pub fn get_modifier(&self, stat: &str) -> i32 {
        self.modifiers.get(stat).copied().unwrap_or(0)
    }

    /// Get the minimum for a stat, if specified.
    pub fn get_minimum(&self, stat: &str) -> Option<u8> {
        self.minimums.get(stat).copied()
    }

    /// Get the priority rank for a stat (0 = highest priority).
    ///
    /// Returns `None` if the stat is not in the priority list.
    pub fn get_priority(&self, stat: &str) -> Option<usize> {
        self.priority_order.iter().position(|s| s == stat)
    }
}

// ============================================================================
// Archetype - Complete archetype definition
// ============================================================================

/// A complete archetype definition.
///
/// Archetypes are the central data model of the registry, combining:
/// - Identity: `id`, `display_name`, `category`
/// - Inheritance: `parent_id` for archetype layering
/// - Personality: `personality_affinity` for trait generation
/// - Roles: `npc_role_mapping` for NPC generation
/// - Speech: `vocabulary_bank_id` for dialogue
/// - Names: `naming_cultures` for name generation
/// - Stats: `stat_tendencies` for character stats
///
/// # Inheritance
///
/// Archetypes can inherit from a parent using `parent_id`. Child archetypes
/// override parent fields according to the merge rules defined in the
/// resolution module.
///
/// # Examples
///
/// ```yaml
/// id: "mountain_dwarf"
/// displayName: "Mountain Dwarf"
/// category: race
/// parentId: "dwarf"
/// personalityAffinity:
///   - traitId: "stubborn"
///     weight: 0.9
///     defaultIntensity: 7
/// namingCultures:
///   - culture: "dwarvish"
///     weight: 1.0
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Archetype {
    /// Unique identifier (e.g., "mountain_dwarf", "knight_errant").
    pub id: ArchetypeId,

    /// Human-readable display name.
    ///
    /// Uses `Arc<str>` for memory efficiency when the same name
    /// is referenced multiple times.
    pub display_name: Arc<str>,

    /// Archetype category for filtering and resolution ordering.
    pub category: ArchetypeCategory,

    /// Optional parent archetype for inheritance.
    ///
    /// When set, this archetype inherits fields from the parent
    /// and can override specific values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<ArchetypeId>,

    /// Description of the archetype.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Personality trait affinities.
    ///
    /// Maps personality traits to weights and intensities for
    /// character generation.
    #[serde(default)]
    pub personality_affinity: Vec<PersonalityAffinity>,

    /// NPC roles this archetype maps to.
    ///
    /// Used by NPCGenerator to determine appropriate roles.
    #[serde(default)]
    pub npc_role_mapping: Vec<NpcRoleMapping>,

    /// Reference to vocabulary bank for speech patterns.
    ///
    /// References a VocabularyBank by ID for dialogue generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vocabulary_bank_id: Option<String>,

    /// Naming cultures with weights for name generation.
    #[serde(default)]
    pub naming_cultures: Vec<NamingCultureWeight>,

    /// Stat generation tendencies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stat_tendencies: Option<StatTendencies>,

    /// Tags for filtering and search.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Setting pack this archetype belongs to (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setting_pack_id: Option<String>,

    /// Game system this archetype is designed for.
    ///
    /// Examples: "dnd5e", "pathfinder2e", "generic"
    #[serde(default = "default_game_system")]
    pub game_system: String,

    /// Schema version for tracking format changes.
    #[serde(default = "default_version")]
    pub version: String,

    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last update timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_game_system() -> String {
    "generic".to_string()
}

impl Archetype {
    /// Create a new archetype with minimal required fields.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier
    /// * `display_name` - Human-readable name
    /// * `category` - Archetype category
    pub fn new(
        id: impl Into<ArchetypeId>,
        display_name: impl Into<Arc<str>>,
        category: ArchetypeCategory,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: id.into(),
            display_name: display_name.into(),
            category,
            parent_id: None,
            description: None,
            personality_affinity: Vec::new(),
            npc_role_mapping: Vec::new(),
            vocabulary_bank_id: None,
            naming_cultures: Vec::new(),
            stat_tendencies: None,
            tags: Vec::new(),
            setting_pack_id: None,
            game_system: default_game_system(),
            version: default_version(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Builder method to set parent archetype.
    pub fn with_parent(mut self, parent_id: impl Into<ArchetypeId>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    /// Builder method to set description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Builder method to add personality affinities.
    pub fn with_personality_affinity(mut self, affinity: Vec<PersonalityAffinity>) -> Self {
        self.personality_affinity = affinity;
        self
    }

    /// Builder method to add NPC role mappings.
    pub fn with_npc_role_mapping(mut self, mapping: Vec<NpcRoleMapping>) -> Self {
        self.npc_role_mapping = mapping;
        self
    }

    /// Builder method to set vocabulary bank reference.
    pub fn with_vocabulary_bank(mut self, bank_id: impl Into<String>) -> Self {
        self.vocabulary_bank_id = Some(bank_id.into());
        self
    }

    /// Builder method to add naming cultures.
    pub fn with_naming_cultures(mut self, cultures: Vec<NamingCultureWeight>) -> Self {
        self.naming_cultures = cultures;
        self
    }

    /// Builder method to set stat tendencies.
    pub fn with_stat_tendencies(mut self, tendencies: StatTendencies) -> Self {
        self.stat_tendencies = Some(tendencies);
        self
    }

    /// Builder method to add tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Builder method to set game system.
    pub fn with_game_system(mut self, system: impl Into<String>) -> Self {
        self.game_system = system.into();
        self
    }

    /// Validate the archetype for completeness and correctness.
    ///
    /// # Validation Rules
    ///
    /// - ID must be valid (non-empty, valid characters)
    /// - Display name must not be empty
    /// - All personality affinities must have valid weights (0.0-1.0)
    /// - Total personality affinity weights must not exceed 2.0 (AR-104.16)
    /// - All NPC role mappings must have valid weights
    /// - All naming culture weights must be valid
    pub fn validate(&self) -> Result<()> {
        // Validate ID
        self.id.validate()?;

        // Validate display name
        if self.display_name.is_empty() {
            return Err(ArchetypeError::ValidationFailed {
                reason: "Display name cannot be empty".to_string(),
            });
        }

        // Validate personality affinities
        let mut total_weight: f32 = 0.0;
        for affinity in &self.personality_affinity {
            affinity.validate()?;
            total_weight += affinity.weight;
        }

        if total_weight > 2.0 {
            return Err(ArchetypeError::InvalidTraitWeights {
                actual_sum: total_weight,
            });
        }

        // Validate NPC role mappings
        for mapping in &self.npc_role_mapping {
            mapping.validate()?;
        }

        // Validate naming cultures
        for culture in &self.naming_cultures {
            culture.validate()?;
        }

        Ok(())
    }

    /// Check if this archetype has a parent.
    #[inline]
    pub fn has_parent(&self) -> bool {
        self.parent_id.is_some()
    }

    /// Update the `updated_at` timestamp to now.
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now();
    }
}

// ============================================================================
// ArchetypeSummary - Lightweight listing type
// ============================================================================

/// Lightweight summary of an archetype for listing and search results.
///
/// Contains only the essential fields needed for display in lists,
/// without the full detail payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchetypeSummary {
    /// Unique identifier.
    pub id: ArchetypeId,

    /// Human-readable display name.
    pub display_name: Arc<str>,

    /// Archetype category.
    pub category: ArchetypeCategory,

    /// Whether this archetype has a parent.
    pub has_parent: bool,

    /// Tags for filtering.
    pub tags: Vec<String>,

    /// Game system.
    pub game_system: String,
}

impl From<&Archetype> for ArchetypeSummary {
    fn from(archetype: &Archetype) -> Self {
        Self {
            id: archetype.id.clone(),
            display_name: archetype.display_name.clone(),
            category: archetype.category.clone(),
            has_parent: archetype.parent_id.is_some(),
            tags: archetype.tags.clone(),
            game_system: archetype.game_system.clone(),
        }
    }
}

impl From<Archetype> for ArchetypeSummary {
    fn from(archetype: Archetype) -> Self {
        Self::from(&archetype)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // ArchetypeId tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_archetype_id_creation() {
        let id = ArchetypeId::new("knight_errant");
        assert_eq!(id.as_str(), "knight_errant");
        assert!(!id.is_empty());
    }

    #[test]
    fn test_archetype_id_from_str() {
        let id: ArchetypeId = "hedge_witch".into();
        assert_eq!(id.as_str(), "hedge_witch");
    }

    #[test]
    fn test_archetype_id_validation_valid() {
        assert!(ArchetypeId::new("valid_id").validate().is_ok());
        assert!(ArchetypeId::new("_private").validate().is_ok());
        assert!(ArchetypeId::new("camelCase123").validate().is_ok());
        assert!(ArchetypeId::new("kebab-case").validate().is_ok());
    }

    #[test]
    fn test_archetype_id_validation_invalid() {
        assert!(ArchetypeId::new("").validate().is_err());
        assert!(ArchetypeId::new("123starts_with_number").validate().is_err());
        assert!(ArchetypeId::new("has spaces").validate().is_err());
        assert!(ArchetypeId::new("has.dots").validate().is_err());
    }

    #[test]
    fn test_archetype_id_display() {
        let id = ArchetypeId::new("test_id");
        assert_eq!(format!("{}", id), "test_id");
    }

    // -------------------------------------------------------------------------
    // ArchetypeCategory tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_category_resolution_priority() {
        assert!(ArchetypeCategory::Role.resolution_priority()
            < ArchetypeCategory::Race.resolution_priority());
        assert!(ArchetypeCategory::Race.resolution_priority()
            < ArchetypeCategory::Class.resolution_priority());
        assert!(ArchetypeCategory::Class.resolution_priority()
            < ArchetypeCategory::Setting.resolution_priority());
    }

    #[test]
    fn test_category_custom_label() {
        let custom = ArchetypeCategory::Custom("faction".to_string());
        assert!(custom.is_custom());
        assert_eq!(custom.custom_label(), Some("faction"));

        assert!(!ArchetypeCategory::Role.is_custom());
        assert_eq!(ArchetypeCategory::Role.custom_label(), None);
    }

    #[test]
    fn test_category_serialization() {
        let json = serde_json::to_string(&ArchetypeCategory::Role).unwrap();
        assert_eq!(json, "\"role\"");

        let custom = ArchetypeCategory::Custom("test".to_string());
        let json = serde_json::to_string(&custom).unwrap();
        assert!(json.contains("custom"));
    }

    // -------------------------------------------------------------------------
    // PersonalityAffinity tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_personality_affinity_creation() {
        let affinity = PersonalityAffinity::new("curious", 0.8);
        assert_eq!(affinity.trait_id, "curious");
        assert_eq!(affinity.weight, 0.8);
        assert_eq!(affinity.default_intensity, 5); // default
    }

    #[test]
    fn test_personality_affinity_with_intensity() {
        let affinity = PersonalityAffinity::with_intensity("brave", 0.9, 8);
        assert_eq!(affinity.default_intensity, 8);
    }

    #[test]
    fn test_personality_affinity_intensity_clamping() {
        let affinity = PersonalityAffinity::with_intensity("test", 0.5, 15);
        assert_eq!(affinity.default_intensity, 10); // clamped to max

        let affinity = PersonalityAffinity::with_intensity("test", 0.5, 0);
        assert_eq!(affinity.default_intensity, 1); // clamped to min
    }

    #[test]
    fn test_personality_affinity_validation() {
        assert!(PersonalityAffinity::new("valid", 0.5).validate().is_ok());
        assert!(PersonalityAffinity::new("valid", 0.0).validate().is_ok());
        assert!(PersonalityAffinity::new("valid", 1.0).validate().is_ok());

        assert!(PersonalityAffinity::new("invalid", -0.1).validate().is_err());
        assert!(PersonalityAffinity::new("invalid", 1.1).validate().is_err());
    }

    // -------------------------------------------------------------------------
    // NpcRoleMapping tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_npc_role_mapping_creation() {
        let mapping = NpcRoleMapping::new("merchant", 0.8);
        assert_eq!(mapping.role, "merchant");
        assert_eq!(mapping.weight, 0.8);
        assert!(mapping.context.is_none());
    }

    #[test]
    fn test_npc_role_mapping_with_context() {
        let mapping = NpcRoleMapping::with_context("guard", 0.7, "City watch patrol");
        assert_eq!(mapping.context, Some("City watch patrol".to_string()));
    }

    #[test]
    fn test_npc_role_mapping_validation() {
        assert!(NpcRoleMapping::new("valid", 0.5).validate().is_ok());
        assert!(NpcRoleMapping::new("invalid", -0.1).validate().is_err());
        assert!(NpcRoleMapping::new("invalid", 1.1).validate().is_err());
    }

    // -------------------------------------------------------------------------
    // NamingCultureWeight tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_naming_culture_weight_creation() {
        let culture = NamingCultureWeight::new("dwarvish", 0.9);
        assert_eq!(culture.culture, "dwarvish");
        assert_eq!(culture.weight, 0.9);
        assert!(culture.pattern_overrides.is_none());
    }

    #[test]
    fn test_naming_culture_weight_with_overrides() {
        let overrides = NamingPatternOverrides {
            title_probability: Some(0.3),
            epithet_probability: Some(0.5),
            clan_format: Some("{name} of Clan {clan}".to_string()),
            additional_suffixes: vec!["son".to_string()],
        };
        let culture = NamingCultureWeight::with_overrides("dwarvish", 0.9, overrides);
        assert!(culture.pattern_overrides.is_some());
    }

    #[test]
    fn test_naming_pattern_overrides_validation() {
        let valid = NamingPatternOverrides {
            title_probability: Some(0.5),
            epithet_probability: Some(0.5),
            ..Default::default()
        };
        assert!(valid.validate().is_ok());

        let invalid = NamingPatternOverrides {
            title_probability: Some(1.5),
            ..Default::default()
        };
        assert!(invalid.validate().is_err());
    }

    // -------------------------------------------------------------------------
    // StatTendencies tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_stat_tendencies_empty() {
        let tendencies = StatTendencies::default();
        assert!(tendencies.is_empty());
    }

    #[test]
    fn test_stat_tendencies_modifiers() {
        let mut tendencies = StatTendencies::default();
        tendencies.modifiers.insert("strength".to_string(), 2);
        tendencies.modifiers.insert("charisma".to_string(), -1);

        assert_eq!(tendencies.get_modifier("strength"), 2);
        assert_eq!(tendencies.get_modifier("charisma"), -1);
        assert_eq!(tendencies.get_modifier("wisdom"), 0); // default
    }

    #[test]
    fn test_stat_tendencies_priority() {
        let mut tendencies = StatTendencies::default();
        tendencies.priority_order = vec![
            "strength".to_string(),
            "constitution".to_string(),
            "dexterity".to_string(),
        ];

        assert_eq!(tendencies.get_priority("strength"), Some(0));
        assert_eq!(tendencies.get_priority("constitution"), Some(1));
        assert_eq!(tendencies.get_priority("wisdom"), None);
    }

    // -------------------------------------------------------------------------
    // Archetype tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_archetype_creation() {
        let archetype = Archetype::new(
            "knight_errant",
            "Knight Errant",
            ArchetypeCategory::Class,
        );

        assert_eq!(archetype.id.as_str(), "knight_errant");
        assert_eq!(archetype.display_name.as_ref(), "Knight Errant");
        assert_eq!(archetype.category, ArchetypeCategory::Class);
        assert!(!archetype.has_parent());
    }

    #[test]
    fn test_archetype_builder_pattern() {
        let archetype = Archetype::new("dwarf_merchant", "Dwarf Merchant", ArchetypeCategory::Role)
            .with_parent("dwarf")
            .with_description("A stout merchant of dwarven heritage")
            .with_personality_affinity(vec![
                PersonalityAffinity::new("stubborn", 0.8),
                PersonalityAffinity::new("loyal", 0.7),
            ])
            .with_npc_role_mapping(vec![NpcRoleMapping::new("merchant", 0.9)])
            .with_naming_cultures(vec![NamingCultureWeight::new("dwarvish", 1.0)])
            .with_tags(vec!["dwarf".to_string(), "merchant".to_string()])
            .with_game_system("dnd5e");

        assert!(archetype.has_parent());
        assert_eq!(archetype.parent_id.as_ref().unwrap().as_str(), "dwarf");
        assert!(archetype.description.is_some());
        assert_eq!(archetype.personality_affinity.len(), 2);
        assert_eq!(archetype.npc_role_mapping.len(), 1);
        assert_eq!(archetype.naming_cultures.len(), 1);
        assert_eq!(archetype.tags.len(), 2);
        assert_eq!(archetype.game_system, "dnd5e");
    }

    #[test]
    fn test_archetype_validation_valid() {
        let archetype = Archetype::new("valid_id", "Valid Name", ArchetypeCategory::Role)
            .with_personality_affinity(vec![
                PersonalityAffinity::new("trait1", 0.5),
                PersonalityAffinity::new("trait2", 0.5),
            ]);

        assert!(archetype.validate().is_ok());
    }

    #[test]
    fn test_archetype_validation_empty_display_name() {
        let archetype = Archetype::new("valid_id", "", ArchetypeCategory::Role);
        let result = archetype.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Display name"));
    }

    #[test]
    fn test_archetype_validation_trait_weight_sum() {
        let archetype = Archetype::new("test", "Test", ArchetypeCategory::Role)
            .with_personality_affinity(vec![
                PersonalityAffinity::new("trait1", 0.9),
                PersonalityAffinity::new("trait2", 0.9),
                PersonalityAffinity::new("trait3", 0.9), // sum = 2.7 > 2.0
            ]);

        let result = archetype.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            ArchetypeError::InvalidTraitWeights { actual_sum } => {
                assert!(actual_sum > 2.0);
            }
            _ => panic!("Expected InvalidTraitWeights error"),
        }
    }

    #[test]
    fn test_archetype_touch() {
        let mut archetype = Archetype::new("test", "Test", ArchetypeCategory::Role);
        let original_updated = archetype.updated_at;

        // Small delay to ensure time difference
        std::thread::sleep(std::time::Duration::from_millis(10));
        archetype.touch();

        assert!(archetype.updated_at > original_updated);
    }

    // -------------------------------------------------------------------------
    // ArchetypeSummary tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_archetype_summary_from_archetype() {
        let archetype = Archetype::new("test", "Test Archetype", ArchetypeCategory::Class)
            .with_parent("parent")
            .with_tags(vec!["tag1".to_string()])
            .with_game_system("dnd5e");

        let summary: ArchetypeSummary = (&archetype).into();

        assert_eq!(summary.id.as_str(), "test");
        assert_eq!(summary.display_name.as_ref(), "Test Archetype");
        assert_eq!(summary.category, ArchetypeCategory::Class);
        assert!(summary.has_parent);
        assert_eq!(summary.tags, vec!["tag1".to_string()]);
        assert_eq!(summary.game_system, "dnd5e");
    }

    // -------------------------------------------------------------------------
    // Serialization roundtrip tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_archetype_serialization_roundtrip() {
        let archetype = Archetype::new("test_archetype", "Test Archetype", ArchetypeCategory::Role)
            .with_parent("parent")
            .with_description("A test archetype")
            .with_personality_affinity(vec![PersonalityAffinity::new("curious", 0.8)])
            .with_npc_role_mapping(vec![NpcRoleMapping::with_context(
                "merchant",
                0.9,
                "Test context",
            )])
            .with_naming_cultures(vec![NamingCultureWeight::new("common", 1.0)])
            .with_stat_tendencies(StatTendencies {
                modifiers: [("strength".to_string(), 2)].into_iter().collect(),
                minimums: [("constitution".to_string(), 10)].into_iter().collect(),
                priority_order: vec!["strength".to_string()],
            })
            .with_tags(vec!["test".to_string()]);

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&archetype).unwrap();

        // Verify camelCase formatting
        assert!(json.contains("displayName"));
        assert!(json.contains("parentId"));
        assert!(json.contains("personalityAffinity"));
        assert!(json.contains("npcRoleMapping"));
        assert!(json.contains("namingCultures"));
        assert!(json.contains("statTendencies"));
        assert!(json.contains("traitId"));
        assert!(json.contains("defaultIntensity"));

        // Deserialize back
        let deserialized: Archetype = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id.as_str(), archetype.id.as_str());
        assert_eq!(deserialized.display_name, archetype.display_name);
        assert_eq!(deserialized.personality_affinity.len(), 1);
        assert_eq!(deserialized.npc_role_mapping.len(), 1);
    }

    #[test]
    fn test_personality_affinity_serialization() {
        let affinity = PersonalityAffinity::with_intensity("brave", 0.9, 8);
        let json = serde_json::to_string(&affinity).unwrap();

        assert!(json.contains("\"traitId\""));
        assert!(json.contains("\"defaultIntensity\""));

        let deserialized: PersonalityAffinity = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.trait_id, "brave");
        assert_eq!(deserialized.weight, 0.9);
        assert_eq!(deserialized.default_intensity, 8);
    }

    #[test]
    fn test_naming_culture_weight_serialization() {
        let overrides = NamingPatternOverrides {
            title_probability: Some(0.3),
            clan_format: Some("{name} of {clan}".to_string()),
            ..Default::default()
        };
        let culture = NamingCultureWeight::with_overrides("elvish", 0.8, overrides);

        let json = serde_json::to_string(&culture).unwrap();
        assert!(json.contains("\"patternOverrides\""));
        assert!(json.contains("\"titleProbability\""));
        assert!(json.contains("\"clanFormat\""));

        let deserialized: NamingCultureWeight = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.culture, "elvish");
        assert!(deserialized.pattern_overrides.is_some());
    }
}
