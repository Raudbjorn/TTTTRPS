//! Resolution Query and Result types for the Archetype Registry.
//!
//! This module defines types for querying and receiving resolved archetypes:
//!
//! - [`ResolutionQuery`]: Query parameters for hierarchical archetype resolution
//! - [`ResolvedArchetype`]: The result of resolving a query through multiple layers
//! - [`ResolutionMetadata`]: Debugging information about the resolution process
//!
//! # Resolution Process
//!
//! The archetype resolution follows a strict priority order, with later layers
//! overriding earlier ones:
//!
//! ```text
//! Priority (lowest to highest):
//! 1. Generic Fallback (built-in defaults)
//! 2. Role-Specific (e.g., "merchant" archetype)
//! 3. Race-Specific (e.g., "dwarf" archetype)
//! 4. Class-Specific (e.g., "knight_errant" archetype)
//! 5. Setting Pack Override (e.g., "forgotten_realms" overrides)
//! 6. Direct Archetype ID (explicit archetype takes final precedence)
//! ```
//!
//! # Examples
//!
//! ```rust,ignore
//! use crate::core::archetype::resolution::{ResolutionQuery, ResolvedArchetype};
//!
//! // Query for a specific archetype
//! let query = ResolutionQuery::single("knight_errant");
//!
//! // Query for NPC generation with multiple layers
//! let query = ResolutionQuery::for_npc("merchant")
//!     .with_race("dwarf")
//!     .with_class("fighter")
//!     .with_setting("forgotten_realms");
//!
//! // Resolve through registry
//! let resolved: ResolvedArchetype = registry.resolve(&query).await?;
//! ```

use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::setting_pack::VocabularyBankDefinition;
use super::types::{
    ArchetypeCategory, ArchetypeId, NamingCultureWeight, NpcRoleMapping,
    PersonalityAffinity, StatTendencies,
};

// ============================================================================
// ResolutionQuery - Query for archetype resolution
// ============================================================================

/// Query parameters for archetype resolution.
///
/// Supports multiple resolution modes:
/// - **Direct lookup**: Specify `archetype_id` for single archetype retrieval
/// - **Hierarchical**: Specify `npc_role`, `race`, `class`, `setting` for layered resolution
/// - **Campaign-aware**: Add `campaign_id` to use active setting pack
///
/// # Cache Key
///
/// `ResolutionQuery` implements `Hash` and `Eq` for use as cache keys.
/// All fields contribute to the hash.
///
/// # Examples
///
/// ```rust
/// use ttrpg_assistant::core::archetype::resolution::ResolutionQuery;
///
/// // Direct archetype lookup
/// let query = ResolutionQuery::single("knight_errant");
///
/// // NPC-focused query
/// let query = ResolutionQuery::for_npc("merchant")
///     .with_race("dwarf")
///     .with_setting("forgotten_realms");
///
/// // Full builder pattern
/// let query = ResolutionQuery::builder()
///     .npc_role("guard")
///     .race("human")
///     .class("fighter")
///     .setting("waterdeep")
///     .campaign("my_campaign")
///     .build();
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionQuery {
    /// Direct archetype ID (highest priority if specified).
    ///
    /// When set, this archetype is resolved (with inheritance) and
    /// merged on top of any role/race/class/setting layers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archetype_id: Option<String>,

    /// NPC role for role-based lookup.
    ///
    /// Examples: "merchant", "guard", "innkeeper", "noble"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub npc_role: Option<String>,

    /// Race for race-based overlay.
    ///
    /// Examples: "dwarf", "elf", "human", "halfling"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub race: Option<String>,

    /// Class for class-based overlay.
    ///
    /// Examples: "fighter", "wizard", "rogue", "cleric"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,

    /// Setting ID for setting pack overlay.
    ///
    /// References a registered setting pack by ID.
    /// Examples: "forgotten_realms", "eberron", "homebrew_1"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setting: Option<String>,

    /// Campaign ID for active setting pack lookup.
    ///
    /// When set, the registry will look up the active setting pack
    /// for this campaign (if any) and apply its overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub campaign_id: Option<String>,

    /// DM personality ID for final tone overlay.
    ///
    /// When set, the resolved archetype may have additional
    /// adjustments based on the DM's personality preferences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dm_personality_id: Option<String>,
}

impl ResolutionQuery {
    /// Create an empty query.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a query builder.
    pub fn builder() -> ResolutionQueryBuilder {
        ResolutionQueryBuilder::new()
    }

    /// Create a query for a single archetype lookup.
    ///
    /// # Arguments
    ///
    /// * `id` - The archetype ID to look up
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ttrpg_assistant::core::archetype::resolution::ResolutionQuery;
    /// let query = ResolutionQuery::single("knight_errant");
    /// assert!(query.archetype_id.is_some());
    /// ```
    pub fn single(id: impl Into<String>) -> Self {
        Self {
            archetype_id: Some(id.into()),
            npc_role: None,
            race: None,
            class: None,
            setting: None,
            campaign_id: None,
            dm_personality_id: None,
        }
    }

    /// Create a query for NPC generation starting with a role.
    ///
    /// # Arguments
    ///
    /// * `role` - The NPC role (e.g., "merchant", "guard")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ttrpg_assistant::core::archetype::resolution::ResolutionQuery;
    /// let query = ResolutionQuery::for_npc("merchant")
    ///     .with_race("dwarf");
    /// ```
    pub fn for_npc(role: impl Into<String>) -> Self {
        Self {
            archetype_id: None,
            npc_role: Some(role.into()),
            race: None,
            class: None,
            setting: None,
            campaign_id: None,
            dm_personality_id: None,
        }
    }

    /// Add race to the query (builder pattern).
    pub fn with_race(mut self, race: impl Into<String>) -> Self {
        self.race = Some(race.into());
        self
    }

    /// Add class to the query (builder pattern).
    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.class = Some(class.into());
        self
    }

    /// Add setting to the query (builder pattern).
    pub fn with_setting(mut self, setting: impl Into<String>) -> Self {
        self.setting = Some(setting.into());
        self
    }

    /// Add campaign ID to the query (builder pattern).
    pub fn with_campaign(mut self, campaign_id: impl Into<String>) -> Self {
        self.campaign_id = Some(campaign_id.into());
        self
    }

    /// Add DM personality ID to the query (builder pattern).
    pub fn with_dm_personality(mut self, personality_id: impl Into<String>) -> Self {
        self.dm_personality_id = Some(personality_id.into());
        self
    }

    /// Add direct archetype ID to the query (builder pattern).
    pub fn with_archetype(mut self, id: impl Into<String>) -> Self {
        self.archetype_id = Some(id.into());
        self
    }

    /// Check if this is a direct lookup query (only archetype_id set).
    pub fn is_direct_lookup(&self) -> bool {
        self.archetype_id.is_some()
            && self.npc_role.is_none()
            && self.race.is_none()
            && self.class.is_none()
            && self.setting.is_none()
    }

    /// Check if this is a hierarchical query (has role/race/class/setting).
    pub fn is_hierarchical(&self) -> bool {
        self.npc_role.is_some()
            || self.race.is_some()
            || self.class.is_some()
            || self.setting.is_some()
    }

    /// Check if the query is empty (no parameters set).
    pub fn is_empty(&self) -> bool {
        self.archetype_id.is_none()
            && self.npc_role.is_none()
            && self.race.is_none()
            && self.class.is_none()
            && self.setting.is_none()
            && self.campaign_id.is_none()
            && self.dm_personality_id.is_none()
    }

    /// Get a list of layers that will be checked during resolution.
    ///
    /// This is useful for debugging and for the `layers_checked` field
    /// in `ResolutionMetadata`.
    pub fn expected_layers(&self) -> Vec<String> {
        let mut layers = Vec::new();

        if let Some(ref role) = self.npc_role {
            layers.push(format!("role:{}", role));
        }
        if let Some(ref race) = self.race {
            layers.push(format!("race:{}", race));
        }
        if let Some(ref class) = self.class {
            layers.push(format!("class:{}", class));
        }
        if let Some(ref setting) = self.setting {
            layers.push(format!("setting:{}", setting));
        }
        if let Some(ref id) = self.archetype_id {
            layers.push(format!("direct:{}", id));
        }

        layers
    }

    /// Create a cache key string for this query.
    ///
    /// Format: `archetype_id|npc_role|race|class|setting|campaign_id|dm_personality_id`
    /// Empty fields are represented as empty strings.
    /// Fields containing the delimiter are escaped.
    pub fn cache_key(&self) -> String {
        fn escape_field(s: &str) -> String {
            s.replace('\\', "\\\\").replace('|', "\\|")
        }
        format!(
            "{}|{}|{}|{}|{}|{}|{}",
            self.archetype_id.as_deref().map(escape_field).unwrap_or_default(),
            self.npc_role.as_deref().map(escape_field).unwrap_or_default(),
            self.race.as_deref().map(escape_field).unwrap_or_default(),
            self.class.as_deref().map(escape_field).unwrap_or_default(),
            self.setting.as_deref().map(escape_field).unwrap_or_default(),
            self.campaign_id.as_deref().map(escape_field).unwrap_or_default(),
            self.dm_personality_id.as_deref().map(escape_field).unwrap_or_default(),
        )
    }
}

// Implement Hash manually for cache key usage
impl Hash for ResolutionQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.archetype_id.hash(state);
        self.npc_role.hash(state);
        self.race.hash(state);
        self.class.hash(state);
        self.setting.hash(state);
        self.campaign_id.hash(state);
        self.dm_personality_id.hash(state);
    }
}

// Implement Eq for cache key usage
impl Eq for ResolutionQuery {}

impl PartialEq for ResolutionQuery {
    fn eq(&self, other: &Self) -> bool {
        self.archetype_id == other.archetype_id
            && self.npc_role == other.npc_role
            && self.race == other.race
            && self.class == other.class
            && self.setting == other.setting
            && self.campaign_id == other.campaign_id
            && self.dm_personality_id == other.dm_personality_id
    }
}

// ============================================================================
// ResolutionQueryBuilder - Fluent builder for complex queries
// ============================================================================

/// Fluent builder for constructing `ResolutionQuery` instances.
///
/// Provides a clean API for building complex queries with multiple parameters.
///
/// # Examples
///
/// ```rust
/// use ttrpg_assistant::core::archetype::resolution::ResolutionQuery;
/// let query = ResolutionQuery::builder()
///     .npc_role("guard")
///     .race("dwarf")
///     .class("fighter")
///     .setting("forgotten_realms")
///     .campaign("my_campaign")
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct ResolutionQueryBuilder {
    query: ResolutionQuery,
}

impl ResolutionQueryBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the direct archetype ID.
    pub fn archetype_id(mut self, id: impl Into<String>) -> Self {
        self.query.archetype_id = Some(id.into());
        self
    }

    /// Set the NPC role.
    pub fn npc_role(mut self, role: impl Into<String>) -> Self {
        self.query.npc_role = Some(role.into());
        self
    }

    /// Set the race.
    pub fn race(mut self, race: impl Into<String>) -> Self {
        self.query.race = Some(race.into());
        self
    }

    /// Set the class.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.query.class = Some(class.into());
        self
    }

    /// Set the setting.
    pub fn setting(mut self, setting: impl Into<String>) -> Self {
        self.query.setting = Some(setting.into());
        self
    }

    /// Set the campaign ID.
    pub fn campaign(mut self, campaign_id: impl Into<String>) -> Self {
        self.query.campaign_id = Some(campaign_id.into());
        self
    }

    /// Set the DM personality ID.
    pub fn dm_personality(mut self, personality_id: impl Into<String>) -> Self {
        self.query.dm_personality_id = Some(personality_id.into());
        self
    }

    /// Build the query.
    pub fn build(self) -> ResolutionQuery {
        self.query
    }
}

// ============================================================================
// ResolvedArchetype - Result of archetype resolution
// ============================================================================

/// The result of resolving an archetype query through multiple layers.
///
/// Contains the merged data from all applicable archetype layers,
/// ready for use by NPC generators, personality blenders, and name generators.
///
/// # Field Sources
///
/// Each field in a `ResolvedArchetype` may come from different layers:
/// - `id`: Usually from the most specific layer (or synthetic for pure merges)
/// - `personality_affinity`: Merged from all layers
/// - `vocabulary_bank`: From the highest-priority layer that specifies it
/// - etc.
///
/// The `resolution_metadata` field provides debugging information about
/// which layers contributed to the final result.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedArchetype {
    /// Resolved archetype ID.
    ///
    /// May be:
    /// - The direct archetype ID if a single archetype was resolved
    /// - A synthetic ID for merged results (e.g., "merged_merchant_dwarf")
    /// - `None` if only partial resolution occurred
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<ArchetypeId>,

    /// Display name from the highest-priority layer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<Arc<str>>,

    /// Category from the highest-priority layer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<ArchetypeCategory>,

    /// Merged personality affinities from all layers.
    ///
    /// Later layers override earlier layers for the same trait_id.
    /// Total weight is normalized to not exceed 2.0 (per AR-104.16).
    #[serde(default)]
    pub personality_affinity: Vec<PersonalityAffinity>,

    /// Merged NPC role mappings from all layers.
    #[serde(default)]
    pub npc_role_mapping: Vec<NpcRoleMapping>,

    /// Resolved vocabulary bank (if any).
    ///
    /// May be a merged bank from multiple sources.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vocabulary_bank: Option<VocabularyBankDefinition>,

    /// Resolved vocabulary bank ID (for reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vocabulary_bank_id: Option<String>,

    /// Resolved naming cultures from all layers.
    #[serde(default)]
    pub naming_cultures: Vec<NamingCultureWeight>,

    /// Merged stat tendencies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stat_tendencies: Option<StatTendencies>,

    /// Combined tags from all layers.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Resolution metadata for debugging and tracing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution_metadata: Option<ResolutionMetadata>,
}

impl ResolvedArchetype {
    /// Create a new empty resolved archetype.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with a specific ID.
    pub fn with_id(id: impl Into<ArchetypeId>) -> Self {
        Self {
            id: Some(id.into()),
            ..Default::default()
        }
    }

    /// Check if the resolved archetype is empty (no useful data).
    ///
    /// An archetype is considered empty if it has no ID and no
    /// meaningful data in any of its fields.
    pub fn is_empty(&self) -> bool {
        self.id.is_none()
            && self.display_name.is_none()
            && self.personality_affinity.is_empty()
            && self.npc_role_mapping.is_empty()
            && self.vocabulary_bank.is_none()
            && self.vocabulary_bank_id.is_none()
            && self.naming_cultures.is_empty()
            && self.stat_tendencies.is_none()
    }

    /// Check if this result was a cache hit.
    pub fn was_cache_hit(&self) -> bool {
        self.resolution_metadata
            .as_ref()
            .map(|m| m.cache_hit)
            .unwrap_or(false)
    }

    /// Get the number of merge operations performed.
    pub fn merge_count(&self) -> usize {
        self.resolution_metadata
            .as_ref()
            .map(|m| m.merge_operations)
            .unwrap_or(0)
    }

    /// Get the layers that were checked during resolution.
    pub fn layers_checked(&self) -> Vec<String> {
        self.resolution_metadata
            .as_ref()
            .map(|m| m.layers_checked.clone())
            .unwrap_or_default()
    }

    /// Get personality affinity for a specific trait.
    pub fn get_affinity(&self, trait_id: &str) -> Option<&PersonalityAffinity> {
        self.personality_affinity
            .iter()
            .find(|a| a.trait_id == trait_id)
    }

    /// Get role mapping for a specific role.
    pub fn get_role_mapping(&self, role: &str) -> Option<&NpcRoleMapping> {
        self.npc_role_mapping.iter().find(|m| m.role == role)
    }

    /// Get naming culture by culture ID.
    pub fn get_naming_culture(&self, culture: &str) -> Option<&NamingCultureWeight> {
        self.naming_cultures.iter().find(|c| c.culture == culture)
    }

    /// Get the primary (highest weight) naming culture.
    pub fn primary_naming_culture(&self) -> Option<&NamingCultureWeight> {
        self.naming_cultures
            .iter()
            .max_by(|a, b| a.weight.total_cmp(&b.weight))
    }

    /// Get the primary (highest weight) NPC role.
    pub fn primary_role(&self) -> Option<&NpcRoleMapping> {
        self.npc_role_mapping
            .iter()
            .max_by(|a, b| a.weight.total_cmp(&b.weight))
    }

    /// Builder method to set metadata.
    pub fn with_metadata(mut self, metadata: ResolutionMetadata) -> Self {
        self.resolution_metadata = Some(metadata);
        self
    }

    /// Builder method to add personality affinities.
    pub fn with_personality_affinity(mut self, affinities: Vec<PersonalityAffinity>) -> Self {
        self.personality_affinity = affinities;
        self
    }

    /// Builder method to add NPC role mappings.
    pub fn with_npc_role_mapping(mut self, mappings: Vec<NpcRoleMapping>) -> Self {
        self.npc_role_mapping = mappings;
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

    /// Builder method to set vocabulary bank ID.
    pub fn with_vocabulary_bank_id(mut self, bank_id: impl Into<String>) -> Self {
        self.vocabulary_bank_id = Some(bank_id.into());
        self
    }
}

// ============================================================================
// ResolutionMetadata - Debugging information
// ============================================================================

/// Metadata about the resolution process.
///
/// Provides debugging and tracing information about how an archetype
/// was resolved, including which layers were checked and whether
/// the result came from cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionMetadata {
    /// Layers checked during resolution.
    ///
    /// Format: `"layer_type:layer_id"` (e.g., "role:merchant", "race:dwarf")
    pub layers_checked: Vec<String>,

    /// Number of merge operations performed.
    ///
    /// Higher counts indicate more complex resolution paths.
    /// Exceeding 50 operations triggers `ResolutionTooComplex` error.
    pub merge_operations: usize,

    /// Whether this result was retrieved from cache.
    pub cache_hit: bool,

    /// When the resolution was performed.
    pub resolved_at: chrono::DateTime<chrono::Utc>,

    /// Time taken for resolution in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution_time_ms: Option<u64>,

    /// The original query that produced this result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<ResolutionQuery>,

    /// Any warnings generated during resolution.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl ResolutionMetadata {
    /// Create new metadata for a fresh resolution.
    pub fn new(layers_checked: Vec<String>, merge_operations: usize) -> Self {
        Self {
            layers_checked,
            merge_operations,
            cache_hit: false,
            resolved_at: chrono::Utc::now(),
            resolution_time_ms: None,
            query: None,
            warnings: Vec::new(),
        }
    }

    /// Create metadata for a cache hit.
    pub fn cache_hit() -> Self {
        Self {
            layers_checked: Vec::new(),
            merge_operations: 0,
            cache_hit: true,
            resolved_at: chrono::Utc::now(),
            resolution_time_ms: Some(0),
            query: None,
            warnings: Vec::new(),
        }
    }

    /// Set the resolution time.
    pub fn with_time(mut self, time_ms: u64) -> Self {
        self.resolution_time_ms = Some(time_ms);
        self
    }

    /// Set the original query.
    pub fn with_query(mut self, query: ResolutionQuery) -> Self {
        self.query = Some(query);
        self
    }

    /// Add a warning.
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Check if any warnings were generated.
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

impl Default for ResolutionMetadata {
    fn default() -> Self {
        Self::new(Vec::new(), 0)
    }
}

// ============================================================================
// ResolutionResult - Wrapper for resolution outcome
// ============================================================================

/// Wrapper type for resolution results that includes both the resolved
/// archetype and any additional context.
///
/// This is useful when you need to distinguish between different
/// resolution outcomes or carry additional data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionResult {
    /// The resolved archetype data.
    pub archetype: ResolvedArchetype,

    /// Whether this was a partial resolution (some layers missing).
    pub is_partial: bool,

    /// Source archetypes that contributed to this result.
    #[serde(default)]
    pub source_archetypes: Vec<ArchetypeId>,

    /// Active setting pack ID (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_setting_pack: Option<String>,
}

impl ResolutionResult {
    /// Create a new resolution result.
    pub fn new(archetype: ResolvedArchetype) -> Self {
        Self {
            archetype,
            is_partial: false,
            source_archetypes: Vec::new(),
            active_setting_pack: None,
        }
    }

    /// Create a partial resolution result.
    pub fn partial(archetype: ResolvedArchetype) -> Self {
        Self {
            archetype,
            is_partial: true,
            source_archetypes: Vec::new(),
            active_setting_pack: None,
        }
    }

    /// Add source archetype.
    pub fn with_source(mut self, id: ArchetypeId) -> Self {
        self.source_archetypes.push(id);
        self
    }

    /// Set active setting pack.
    pub fn with_setting_pack(mut self, pack_id: impl Into<String>) -> Self {
        self.active_setting_pack = Some(pack_id.into());
        self
    }
}

impl From<ResolvedArchetype> for ResolutionResult {
    fn from(archetype: ResolvedArchetype) -> Self {
        Self::new(archetype)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // ResolutionQuery tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_resolution_query_single() {
        let query = ResolutionQuery::single("knight_errant");

        assert_eq!(query.archetype_id, Some("knight_errant".to_string()));
        assert!(query.is_direct_lookup());
        assert!(!query.is_hierarchical());
        assert!(!query.is_empty());
    }

    #[test]
    fn test_resolution_query_for_npc() {
        let query = ResolutionQuery::for_npc("merchant")
            .with_race("dwarf")
            .with_class("fighter")
            .with_setting("forgotten_realms");

        assert_eq!(query.npc_role, Some("merchant".to_string()));
        assert_eq!(query.race, Some("dwarf".to_string()));
        assert_eq!(query.class, Some("fighter".to_string()));
        assert_eq!(query.setting, Some("forgotten_realms".to_string()));
        assert!(!query.is_direct_lookup());
        assert!(query.is_hierarchical());
    }

    #[test]
    fn test_resolution_query_builder() {
        let query = ResolutionQuery::builder()
            .npc_role("guard")
            .race("human")
            .class("warrior")
            .setting("waterdeep")
            .campaign("my_campaign")
            .dm_personality("gruff")
            .build();

        assert_eq!(query.npc_role, Some("guard".to_string()));
        assert_eq!(query.race, Some("human".to_string()));
        assert_eq!(query.class, Some("warrior".to_string()));
        assert_eq!(query.setting, Some("waterdeep".to_string()));
        assert_eq!(query.campaign_id, Some("my_campaign".to_string()));
        assert_eq!(query.dm_personality_id, Some("gruff".to_string()));
    }

    #[test]
    fn test_resolution_query_empty() {
        let query = ResolutionQuery::new();
        assert!(query.is_empty());
        assert!(!query.is_direct_lookup());
        assert!(!query.is_hierarchical());
    }

    #[test]
    fn test_resolution_query_expected_layers() {
        let query = ResolutionQuery::for_npc("merchant")
            .with_race("dwarf")
            .with_setting("fr");

        let layers = query.expected_layers();
        assert_eq!(layers.len(), 3);
        assert!(layers.contains(&"role:merchant".to_string()));
        assert!(layers.contains(&"race:dwarf".to_string()));
        assert!(layers.contains(&"setting:fr".to_string()));
    }

    #[test]
    fn test_resolution_query_cache_key() {
        let query = ResolutionQuery::for_npc("merchant").with_race("dwarf");
        let key = query.cache_key();

        assert!(key.contains("merchant"));
        assert!(key.contains("dwarf"));
    }

    #[test]
    fn test_resolution_query_hash_and_eq() {
        use std::collections::HashSet;

        let query1 = ResolutionQuery::for_npc("merchant").with_race("dwarf");
        let query2 = ResolutionQuery::for_npc("merchant").with_race("dwarf");
        let query3 = ResolutionQuery::for_npc("guard").with_race("dwarf");

        assert_eq!(query1, query2);
        assert_ne!(query1, query3);

        // Test as hash key
        let mut set = HashSet::new();
        set.insert(query1.clone());
        assert!(set.contains(&query2));
        assert!(!set.contains(&query3));
    }

    // -------------------------------------------------------------------------
    // ResolvedArchetype tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_resolved_archetype_empty() {
        let resolved = ResolvedArchetype::new();
        assert!(resolved.is_empty());
    }

    #[test]
    fn test_resolved_archetype_with_id() {
        let resolved = ResolvedArchetype::with_id("test_archetype");
        assert!(!resolved.is_empty());
        assert_eq!(resolved.id.as_ref().unwrap().as_str(), "test_archetype");
    }

    #[test]
    fn test_resolved_archetype_builder() {
        let resolved = ResolvedArchetype::with_id("test")
            .with_personality_affinity(vec![
                PersonalityAffinity::new("curious", 0.8),
                PersonalityAffinity::new("cautious", 0.6),
            ])
            .with_npc_role_mapping(vec![NpcRoleMapping::new("merchant", 0.9)])
            .with_naming_cultures(vec![NamingCultureWeight::new("dwarvish", 1.0)])
            .with_stat_tendencies(StatTendencies::default())
            .with_vocabulary_bank_id("dwarvish_merchant");

        assert_eq!(resolved.personality_affinity.len(), 2);
        assert_eq!(resolved.npc_role_mapping.len(), 1);
        assert_eq!(resolved.naming_cultures.len(), 1);
        assert!(resolved.stat_tendencies.is_some());
        assert_eq!(resolved.vocabulary_bank_id, Some("dwarvish_merchant".to_string()));
    }

    #[test]
    fn test_resolved_archetype_get_affinity() {
        let resolved = ResolvedArchetype::new().with_personality_affinity(vec![
            PersonalityAffinity::new("curious", 0.8),
            PersonalityAffinity::new("brave", 0.7),
        ]);

        let affinity = resolved.get_affinity("curious");
        assert!(affinity.is_some());
        assert_eq!(affinity.unwrap().weight, 0.8);

        assert!(resolved.get_affinity("nonexistent").is_none());
    }

    #[test]
    fn test_resolved_archetype_primary_role() {
        let resolved = ResolvedArchetype::new().with_npc_role_mapping(vec![
            NpcRoleMapping::new("merchant", 0.5),
            NpcRoleMapping::new("guard", 0.9),
            NpcRoleMapping::new("informant", 0.3),
        ]);

        let primary = resolved.primary_role();
        assert!(primary.is_some());
        assert_eq!(primary.unwrap().role, "guard");
    }

    #[test]
    fn test_resolved_archetype_primary_naming_culture() {
        let resolved = ResolvedArchetype::new().with_naming_cultures(vec![
            NamingCultureWeight::new("common", 0.3),
            NamingCultureWeight::new("dwarvish", 0.7),
        ]);

        let primary = resolved.primary_naming_culture();
        assert!(primary.is_some());
        assert_eq!(primary.unwrap().culture, "dwarvish");
    }

    #[test]
    fn test_resolved_archetype_cache_hit() {
        let resolved = ResolvedArchetype::new()
            .with_metadata(ResolutionMetadata::cache_hit());

        assert!(resolved.was_cache_hit());
    }

    // -------------------------------------------------------------------------
    // ResolutionMetadata tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_resolution_metadata_new() {
        let metadata = ResolutionMetadata::new(
            vec!["role:merchant".to_string(), "race:dwarf".to_string()],
            3,
        );

        assert_eq!(metadata.layers_checked.len(), 2);
        assert_eq!(metadata.merge_operations, 3);
        assert!(!metadata.cache_hit);
    }

    #[test]
    fn test_resolution_metadata_cache_hit() {
        let metadata = ResolutionMetadata::cache_hit();
        assert!(metadata.cache_hit);
        assert_eq!(metadata.resolution_time_ms, Some(0));
    }

    #[test]
    fn test_resolution_metadata_warnings() {
        let mut metadata = ResolutionMetadata::new(vec![], 0);
        assert!(!metadata.has_warnings());

        metadata.add_warning("Test warning");
        assert!(metadata.has_warnings());
        assert_eq!(metadata.warnings.len(), 1);
    }

    #[test]
    fn test_resolution_metadata_builder() {
        let query = ResolutionQuery::for_npc("merchant");
        let metadata = ResolutionMetadata::new(vec![], 1)
            .with_time(42)
            .with_query(query);

        assert_eq!(metadata.resolution_time_ms, Some(42));
        assert!(metadata.query.is_some());
    }

    // -------------------------------------------------------------------------
    // ResolutionResult tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_resolution_result_new() {
        let archetype = ResolvedArchetype::with_id("test");
        let result = ResolutionResult::new(archetype);

        assert!(!result.is_partial);
        assert!(result.source_archetypes.is_empty());
    }

    #[test]
    fn test_resolution_result_partial() {
        let archetype = ResolvedArchetype::new();
        let result = ResolutionResult::partial(archetype);

        assert!(result.is_partial);
    }

    #[test]
    fn test_resolution_result_with_sources() {
        let archetype = ResolvedArchetype::with_id("merged");
        let result = ResolutionResult::new(archetype)
            .with_source(ArchetypeId::new("dwarf"))
            .with_source(ArchetypeId::new("merchant"))
            .with_setting_pack("forgotten_realms");

        assert_eq!(result.source_archetypes.len(), 2);
        assert_eq!(result.active_setting_pack, Some("forgotten_realms".to_string()));
    }

    #[test]
    fn test_resolution_result_from_archetype() {
        let archetype = ResolvedArchetype::with_id("test");
        let result: ResolutionResult = archetype.into();

        assert!(!result.is_partial);
    }

    // -------------------------------------------------------------------------
    // Serialization tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_resolution_query_serialization() {
        let query = ResolutionQuery::for_npc("merchant")
            .with_race("dwarf")
            .with_campaign("campaign_1");

        let json = serde_json::to_string(&query).unwrap();

        // Verify camelCase
        assert!(json.contains("npcRole"));
        assert!(json.contains("campaignId"));

        // Roundtrip
        let deserialized: ResolutionQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, query);
    }

    #[test]
    fn test_resolved_archetype_serialization() {
        let resolved = ResolvedArchetype::with_id("test")
            .with_personality_affinity(vec![PersonalityAffinity::new("curious", 0.8)])
            .with_metadata(ResolutionMetadata::new(vec!["role:test".to_string()], 1));

        let json = serde_json::to_string_pretty(&resolved).unwrap();

        // Verify camelCase
        assert!(json.contains("personalityAffinity"));
        assert!(json.contains("resolutionMetadata"));
        assert!(json.contains("layersChecked"));
        assert!(json.contains("mergeOperations"));

        // Roundtrip
        let deserialized: ResolvedArchetype = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id.as_ref().unwrap().as_str(), "test");
        assert_eq!(deserialized.personality_affinity.len(), 1);
    }

    #[test]
    fn test_resolution_metadata_serialization() {
        let metadata = ResolutionMetadata::new(
            vec!["role:merchant".to_string()],
            2,
        ).with_time(100);

        let json = serde_json::to_string(&metadata).unwrap();

        assert!(json.contains("layersChecked"));
        assert!(json.contains("mergeOperations"));
        assert!(json.contains("cacheHit"));
        assert!(json.contains("resolvedAt"));
        assert!(json.contains("resolutionTimeMs"));

        let deserialized: ResolutionMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.merge_operations, 2);
        assert_eq!(deserialized.resolution_time_ms, Some(100));
    }
}
