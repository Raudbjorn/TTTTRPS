//! Archetype Command Types
//!
//! Request and response types for archetype-related commands.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::core::archetype::{
    Archetype, ArchetypeCategory, ArchetypeRegistry, ArchetypeSummary,
    ResolvedArchetype, SettingPackSummary, VocabularyBankManager, VocabularyBankSummary,
};
use crate::commands::AppState;

// ============================================================================
// Request/Response Types for Archetype Commands
// ============================================================================

/// Request payload for creating a new archetype.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateArchetypeRequest {
    /// Unique identifier for the archetype (e.g., "dwarf_merchant").
    pub id: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Category: "role", "race", "class", or "setting".
    pub category: String,
    /// Optional parent archetype ID for inheritance.
    pub parent_id: Option<String>,
    /// Optional description text.
    pub description: Option<String>,
    /// Personality trait affinities.
    #[serde(default)]
    pub personality_affinity: Vec<PersonalityAffinityInput>,
    /// NPC role mappings.
    #[serde(default)]
    pub npc_role_mapping: Vec<NpcRoleMappingInput>,
    /// Naming culture weights.
    #[serde(default)]
    pub naming_cultures: Vec<NamingCultureWeightInput>,
    /// Optional vocabulary bank ID reference.
    pub vocabulary_bank_id: Option<String>,
    /// Optional stat tendencies.
    pub stat_tendencies: Option<StatTendenciesInput>,
    /// Tags for categorization and search.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Input type for personality affinity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalityAffinityInput {
    pub trait_id: String,
    pub weight: f32,
}

/// Input type for NPC role mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcRoleMappingInput {
    pub role: String,
    pub weight: f32,
}

/// Input type for naming culture weight.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NamingCultureWeightInput {
    pub culture: String,
    pub weight: f32,
}

/// Input type for stat tendencies.
///
/// Uses HashMaps to support arbitrary stat names for different game systems.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatTendenciesInput {
    /// Stat modifiers (e.g., {"strength": 2, "charisma": -1}).
    #[serde(default)]
    pub modifiers: std::collections::HashMap<String, i32>,
    /// Minimum stat values (e.g., {"constitution": 12}).
    #[serde(default)]
    pub minimums: std::collections::HashMap<String, u8>,
    /// Priority order for stat allocation (e.g., ["strength", "constitution"]).
    #[serde(default)]
    pub priority_order: Vec<String>,
}

/// Response for archetype operations that return an archetype.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchetypeResponse {
    pub id: String,
    pub display_name: String,
    pub category: String,
    pub parent_id: Option<String>,
    pub description: Option<String>,
    pub personality_affinity: Vec<PersonalityAffinityInput>,
    pub npc_role_mapping: Vec<NpcRoleMappingInput>,
    pub naming_cultures: Vec<NamingCultureWeightInput>,
    pub vocabulary_bank_id: Option<String>,
    pub stat_tendencies: Option<StatTendenciesInput>,
    pub tags: Vec<String>,
}

impl From<Archetype> for ArchetypeResponse {
    fn from(a: Archetype) -> Self {
        Self {
            id: a.id.to_string(),
            display_name: a.display_name.to_string(),
            category: format!("{:?}", a.category).to_lowercase(),
            parent_id: a.parent_id.map(|p| p.to_string()),
            description: a.description.map(|d| d.to_string()),
            personality_affinity: a.personality_affinity.into_iter()
                .map(|p| PersonalityAffinityInput {
                    trait_id: p.trait_id,
                    weight: p.weight,
                })
                .collect(),
            npc_role_mapping: a.npc_role_mapping.into_iter()
                .map(|m| NpcRoleMappingInput {
                    role: m.role,
                    weight: m.weight,
                })
                .collect(),
            naming_cultures: a.naming_cultures.into_iter()
                .map(|c| NamingCultureWeightInput {
                    culture: c.culture,
                    weight: c.weight,
                })
                .collect(),
            vocabulary_bank_id: a.vocabulary_bank_id,
            stat_tendencies: a.stat_tendencies.map(|s| StatTendenciesInput {
                modifiers: s.modifiers,
                minimums: s.minimums,
                priority_order: s.priority_order,
            }),
            tags: a.tags,
        }
    }
}

/// Response for archetype list operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchetypeSummaryResponse {
    pub id: String,
    pub display_name: String,
    pub category: String,
    pub tags: Vec<String>,
}

impl From<ArchetypeSummary> for ArchetypeSummaryResponse {
    fn from(s: ArchetypeSummary) -> Self {
        Self {
            id: s.id.to_string(),
            display_name: s.display_name.to_string(),
            category: format!("{:?}", s.category).to_lowercase(),
            tags: s.tags,
        }
    }
}

/// Request for resolution query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionQueryRequest {
    /// Direct archetype ID to resolve.
    pub archetype_id: Option<String>,
    /// NPC role for role-based resolution layer.
    pub npc_role: Option<String>,
    /// Race for race-based resolution layer.
    pub race: Option<String>,
    /// Class for class-based resolution layer.
    pub class: Option<String>,
    /// Setting pack ID for setting overrides.
    pub setting: Option<String>,
    /// Campaign ID for campaign-specific setting pack.
    pub campaign_id: Option<String>,
}

/// Response for resolved archetype.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedArchetypeResponse {
    pub id: Option<String>,
    pub display_name: Option<String>,
    pub category: Option<String>,
    pub personality_affinity: Vec<PersonalityAffinityInput>,
    pub npc_role_mapping: Vec<NpcRoleMappingInput>,
    pub naming_cultures: Vec<NamingCultureWeightInput>,
    pub vocabulary_bank_id: Option<String>,
    pub stat_tendencies: Option<StatTendenciesInput>,
    pub tags: Vec<String>,
    pub resolution_metadata: Option<ResolutionMetadataResponse>,
}

/// Metadata about the resolution process.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionMetadataResponse {
    pub layers_checked: Vec<String>,
    pub merge_operations: usize,
    pub resolution_time_ms: Option<u64>,
    pub cache_hit: bool,
}

impl From<ResolvedArchetype> for ResolvedArchetypeResponse {
    fn from(r: ResolvedArchetype) -> Self {
        Self {
            id: r.id.map(|id| id.to_string()),
            display_name: r.display_name.map(|n| n.to_string()),
            category: r.category.map(|c| format!("{:?}", c).to_lowercase()),
            personality_affinity: r.personality_affinity.into_iter()
                .map(|p| PersonalityAffinityInput {
                    trait_id: p.trait_id,
                    weight: p.weight,
                })
                .collect(),
            npc_role_mapping: r.npc_role_mapping.into_iter()
                .map(|m| NpcRoleMappingInput {
                    role: m.role,
                    weight: m.weight,
                })
                .collect(),
            naming_cultures: r.naming_cultures.into_iter()
                .map(|c| NamingCultureWeightInput {
                    culture: c.culture,
                    weight: c.weight,
                })
                .collect(),
            vocabulary_bank_id: r.vocabulary_bank_id,
            stat_tendencies: r.stat_tendencies.map(|s| StatTendenciesInput {
                modifiers: s.modifiers,
                minimums: s.minimums,
                priority_order: s.priority_order,
            }),
            tags: r.tags,
            resolution_metadata: r.resolution_metadata.map(|m| ResolutionMetadataResponse {
                layers_checked: m.layers_checked,
                merge_operations: m.merge_operations,
                resolution_time_ms: m.resolution_time_ms,
                cache_hit: m.cache_hit,
            }),
        }
    }
}

/// Response for setting pack summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingPackSummaryResponse {
    pub id: String,
    pub name: String,
    pub version: String,
    pub game_system: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
}

impl From<SettingPackSummary> for SettingPackSummaryResponse {
    fn from(s: SettingPackSummary) -> Self {
        Self {
            id: s.id,
            name: s.name,
            version: s.version,
            game_system: s.game_system,
            author: s.author,
            tags: s.tags,
        }
    }
}

/// Response for vocabulary bank summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VocabularyBankSummaryResponse {
    pub id: String,
    pub display_name: String,
    pub culture: Option<String>,
    pub role: Option<String>,
    pub is_builtin: bool,
    pub category_count: usize,
    pub phrase_count: usize,
}

impl From<VocabularyBankSummary> for VocabularyBankSummaryResponse {
    fn from(s: VocabularyBankSummary) -> Self {
        Self {
            id: s.id,
            display_name: s.display_name,
            culture: s.culture,
            role: s.role,
            is_builtin: s.is_builtin,
            category_count: s.category_count,
            phrase_count: s.phrase_count,
        }
    }
}

/// Request for creating a vocabulary bank.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVocabularyBankRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub culture: Option<String>,
    pub role: Option<String>,
    #[serde(default)]
    pub phrases: Vec<PhraseInput>,
}

/// Input type for a phrase in vocabulary bank.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhraseInput {
    pub text: String,
    pub category: String,
    #[serde(default = "default_formality")]
    pub formality: u8,
    pub tones: Option<Vec<String>>,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_formality() -> u8 {
    5
}

/// Response for vocabulary bank.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VocabularyBankResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub culture: Option<String>,
    pub role: Option<String>,
    pub phrases: Vec<PhraseOutput>,
}

/// Output type for a phrase.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhraseOutput {
    pub text: String,
    pub category: String,
    pub formality: u8,
    /// All tone markers for this phrase (preserves multi-tone phrases)
    pub tones: Vec<String>,
    pub tags: Vec<String>,
}

/// Filter options for listing phrases.
///
/// Note: category is passed as a separate required parameter to get_phrases.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhraseFilterRequest {
    pub formality_min: Option<u8>,
    pub formality_max: Option<u8>,
    pub tone: Option<String>,
}

/// Response for cache statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchetypeCacheStatsResponse {
    pub current_size: usize,
    pub capacity: usize,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse category string to ArchetypeCategory enum.
pub(crate) fn parse_category(s: &str) -> Result<ArchetypeCategory, String> {
    match s.to_lowercase().as_str() {
        "role" => Ok(ArchetypeCategory::Role),
        "race" => Ok(ArchetypeCategory::Race),
        "class" => Ok(ArchetypeCategory::Class),
        "setting" => Ok(ArchetypeCategory::Setting),
        _ => Err(format!("Invalid category: {}. Must be 'role', 'race', 'class', or 'setting'", s)),
    }
}

/// Get the archetype registry from state, returning error if not initialized.
pub(crate) async fn get_registry(state: &AppState) -> Result<Arc<ArchetypeRegistry>, String> {
    state.archetype_registry
        .read()
        .await
        .as_ref()
        .cloned()
        .ok_or_else(|| "Archetype registry not initialized. Please wait for Meilisearch to start.".to_string())
}

/// Get the vocabulary manager from state, returning error if not initialized.
pub(crate) async fn get_vocabulary_manager(state: &AppState) -> Result<Arc<VocabularyBankManager>, String> {
    state.vocabulary_manager
        .read()
        .await
        .as_ref()
        .cloned()
        .ok_or_else(|| "Vocabulary manager not initialized. Please wait for Meilisearch to start.".to_string())
}
