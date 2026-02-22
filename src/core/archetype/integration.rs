//! Integration interfaces for the Archetype Registry.
//!
//! This module defines the interfaces and traits that allow other systems
//! (PersonalityBlender, NPCGenerator, NameGenerator) to query archetype data.
//!
//! # Design Philosophy
//!
//! These interfaces follow a "registry-as-source-of-truth" pattern:
//! - Systems query the registry for archetype-derived data
//! - All data flows through a unified resolution pipeline
//! - Graceful fallbacks are provided when the registry is unavailable
//!
//! # Integration Points
//!
//! ```text
//!                     ArchetypeRegistry
//!                            |
//!            +---------------+---------------+
//!            |               |               |
//!            v               v               v
//!   PersonalityBlender  NPCGenerator   NameGenerator
//!        |                  |               |
//!   personality_for()  generate_with()  generate_for()
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::core::archetype::integration::*;
//!
//! // For PersonalityBlender
//! let affinities = blend_for_archetype(&registry, "dwarf_merchant").await?;
//!
//! // For NPCGenerator
//! let context = npc_context_for_archetype(&registry, "guard").await?;
//!
//! // For NameGenerator
//! let naming = naming_context_for_archetype(&registry, "elf").await?;
//! ```

use serde::{Deserialize, Serialize};

use super::error::Result;
use super::registry::ArchetypeRegistry;
use super::resolution::{ResolutionQuery, ResolvedArchetype};
use super::types::{
    NamingCultureWeight, NamingPatternOverrides, NpcRoleMapping, PersonalityAffinity,
    StatTendencies,
};
use super::vocabulary::{VocabularyBank, VocabularyBankManager};

// ============================================================================
// PersonalityBlender Integration
// ============================================================================

/// Personality affinities resolved for character generation.
///
/// This struct packages the personality data that `PersonalityBlender`
/// needs to generate a character's personality traits.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalityContext {
    /// Trait affinities with weights and intensities.
    ///
    /// Maps trait_id -> (weight, default_intensity).
    /// Higher weights indicate traits more likely for this archetype.
    pub affinities: Vec<PersonalityAffinityEntry>,

    /// Speech patterns from vocabulary bank (if available).
    ///
    /// Provides common phrases for dialogue generation.
    pub speech_patterns: Option<SpeechPatterns>,

    /// Source information for debugging and display.
    pub source: PersonalityContextSource,
}

/// A single personality affinity entry for the blender.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalityAffinityEntry {
    /// The personality trait identifier.
    pub trait_id: String,

    /// Selection weight (0.0-1.0).
    pub weight: f32,

    /// Default intensity when this trait is selected (1-10).
    pub default_intensity: u8,
}

impl From<&PersonalityAffinity> for PersonalityAffinityEntry {
    fn from(affinity: &PersonalityAffinity) -> Self {
        Self {
            trait_id: affinity.trait_id.clone(),
            weight: affinity.weight,
            default_intensity: affinity.default_intensity,
        }
    }
}

/// Speech patterns for dialogue generation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechPatterns {
    /// Common greeting phrases.
    pub greetings: Vec<String>,

    /// Common farewell phrases.
    pub farewells: Vec<String>,

    /// Common affirmation phrases.
    pub affirmations: Vec<String>,

    /// Common denial phrases.
    pub denials: Vec<String>,

    /// Common exclamations and expressions.
    pub expressions: Vec<String>,
}

/// Source information for the personality context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct PersonalityContextSource {
    /// Archetypes that contributed to this context.
    pub contributing_archetypes: Vec<String>,

    /// Active setting pack (if any).
    pub setting_pack: Option<String>,

    /// Vocabulary bank used (if any).
    pub vocabulary_bank: Option<String>,

    /// Whether fallback defaults were used.
    pub used_fallback: bool,
}


/// Get personality affinities for a given archetype.
///
/// This is the primary integration point for `PersonalityBlender`.
///
/// # Arguments
///
/// * `registry` - The archetype registry
/// * `archetype_id` - The archetype to query
/// * `vocabulary_manager` - Optional vocabulary bank manager for speech patterns
///
/// # Returns
///
/// A `PersonalityContext` containing trait affinities and speech patterns.
///
/// # Fallback Behavior
///
/// If the archetype is not found, returns a context with default affinities
/// (balanced distribution) and sets `used_fallback = true`.
pub async fn blend_for_archetype(
    registry: &ArchetypeRegistry,
    archetype_id: &str,
    vocabulary_manager: Option<&VocabularyBankManager>,
) -> Result<PersonalityContext> {
    // Try to get the archetype
    let archetype = match registry.get(archetype_id).await {
        Ok(arch) => arch,
        Err(_) => {
            // Return fallback context
            return Ok(PersonalityContext {
                affinities: default_personality_affinities(),
                speech_patterns: None,
                source: PersonalityContextSource {
                    used_fallback: true,
                    ..Default::default()
                },
            });
        }
    };

    // Convert affinities
    let affinities: Vec<PersonalityAffinityEntry> = archetype
        .personality_affinity
        .iter()
        .map(PersonalityAffinityEntry::from)
        .collect();

    // Get speech patterns if vocabulary manager is available
    let speech_patterns = if let (Some(manager), Some(bank_id)) =
        (vocabulary_manager, &archetype.vocabulary_bank_id)
    {
        get_speech_patterns(manager, bank_id).await.ok()
    } else {
        None
    };

    Ok(PersonalityContext {
        affinities,
        speech_patterns,
        source: PersonalityContextSource {
            contributing_archetypes: vec![archetype_id.to_string()],
            vocabulary_bank: archetype.vocabulary_bank_id.clone(),
            used_fallback: false,
            ..Default::default()
        },
    })
}

/// Get personality context for a hierarchically resolved archetype.
///
/// Use this when you have multiple archetype layers (role + race + class).
///
/// # Arguments
///
/// * `registry` - The archetype registry
/// * `resolved` - The resolved archetype to extract context from
/// * `vocabulary_manager` - Optional vocabulary bank manager
pub async fn blend_for_resolved(
    _registry: &ArchetypeRegistry,
    resolved: &ResolvedArchetype,
    vocabulary_manager: Option<&VocabularyBankManager>,
) -> Result<PersonalityContext> {
    let affinities: Vec<PersonalityAffinityEntry> = resolved
        .personality_affinity
        .iter()
        .map(PersonalityAffinityEntry::from)
        .collect();

    // Get speech patterns if available
    let speech_patterns = if let (Some(manager), Some(bank_id)) =
        (vocabulary_manager, &resolved.vocabulary_bank_id)
    {
        get_speech_patterns(manager, bank_id).await.ok()
    } else {
        None
    };

    // Extract contributing archetypes from resolution metadata if available
    let contributing_archetypes = resolved
        .resolution_metadata
        .as_ref()
        .map(|m| m.layers_checked.clone())
        .unwrap_or_default();

    // Extract setting pack from resolution metadata query if available
    let setting_pack = resolved
        .resolution_metadata
        .as_ref()
        .and_then(|m| m.query.as_ref())
        .and_then(|q| q.setting.clone());

    Ok(PersonalityContext {
        affinities,
        speech_patterns,
        source: PersonalityContextSource {
            contributing_archetypes,
            setting_pack,
            vocabulary_bank: resolved.vocabulary_bank_id.clone(),
            used_fallback: false,
        },
    })
}

/// Get speech patterns from a vocabulary bank.
async fn get_speech_patterns(
    manager: &VocabularyBankManager,
    bank_id: &str,
) -> Result<SpeechPatterns> {
    let bank = manager.get_bank(bank_id).await?;

    Ok(SpeechPatterns {
        greetings: extract_phrases(&bank, "greetings"),
        farewells: extract_phrases(&bank, "farewells"),
        affirmations: extract_phrases(&bank, "affirmations"),
        denials: extract_phrases(&bank, "denials"),
        expressions: extract_phrases(&bank, "expressions"),
    })
}

/// Extract phrase texts from a vocabulary bank category.
fn extract_phrases(bank: &VocabularyBank, category: &str) -> Vec<String> {
    bank.definition
        .phrases
        .get(category)
        .map(|phrases| phrases.iter().map(|p| p.text.clone()).collect())
        .unwrap_or_default()
}

/// Default personality affinities for fallback.
fn default_personality_affinities() -> Vec<PersonalityAffinityEntry> {
    vec![
        PersonalityAffinityEntry {
            trait_id: "neutral".to_string(),
            weight: 0.5,
            default_intensity: 5,
        },
        PersonalityAffinityEntry {
            trait_id: "practical".to_string(),
            weight: 0.4,
            default_intensity: 5,
        },
        PersonalityAffinityEntry {
            trait_id: "cautious".to_string(),
            weight: 0.3,
            default_intensity: 4,
        },
    ]
}

// ============================================================================
// NPCGenerator Integration
// ============================================================================

/// Context for NPC generation from archetype data.
///
/// This struct packages the data that `NPCGenerator` needs to create
/// an NPC based on archetype specifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcGenerationContext {
    /// Role mappings with weights for role selection.
    pub role_mappings: Vec<NpcRoleMapping>,

    /// Stat generation tendencies.
    pub stat_tendencies: StatTendencies,

    /// Personality affinities for trait selection.
    pub personality_affinities: Vec<PersonalityAffinity>,

    /// Vocabulary bank ID for dialogue.
    pub vocabulary_bank_id: Option<String>,

    /// Tags for filtering and categorization.
    pub tags: Vec<String>,

    /// Source information.
    pub source: NpcGenerationContextSource,
}

/// Source information for NPC generation context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct NpcGenerationContextSource {
    /// Primary archetype ID.
    pub primary_archetype: Option<String>,

    /// Race archetype ID (if applied).
    pub race_archetype: Option<String>,

    /// Class archetype ID (if applied).
    pub class_archetype: Option<String>,

    /// Setting pack applied.
    pub setting_pack: Option<String>,

    /// Whether fallback defaults were used.
    pub used_fallback: bool,
}


/// Get NPC generation context for an archetype.
///
/// This is the primary integration point for `NPCGenerator`.
///
/// # Arguments
///
/// * `registry` - The archetype registry
/// * `archetype_id` - The primary archetype (usually a role)
///
/// # Returns
///
/// An `NpcGenerationContext` with role mappings, stats, and traits.
///
/// # Fallback Behavior
///
/// If the archetype is not found, returns a context with default values
/// and sets `used_fallback = true`.
pub async fn npc_context_for_archetype(
    registry: &ArchetypeRegistry,
    archetype_id: &str,
) -> Result<NpcGenerationContext> {
    let archetype = match registry.get(archetype_id).await {
        Ok(arch) => arch,
        Err(_) => {
            return Ok(NpcGenerationContext {
                role_mappings: default_role_mappings(),
                stat_tendencies: StatTendencies::default(),
                personality_affinities: Vec::new(),
                vocabulary_bank_id: None,
                tags: Vec::new(),
                source: NpcGenerationContextSource {
                    used_fallback: true,
                    ..Default::default()
                },
            });
        }
    };

    Ok(NpcGenerationContext {
        role_mappings: archetype.npc_role_mapping.clone(),
        stat_tendencies: archetype.stat_tendencies.clone().unwrap_or_default(),
        personality_affinities: archetype.personality_affinity.clone(),
        vocabulary_bank_id: archetype.vocabulary_bank_id.clone(),
        tags: archetype.tags.clone(),
        source: NpcGenerationContextSource {
            primary_archetype: Some(archetype_id.to_string()),
            used_fallback: false,
            ..Default::default()
        },
    })
}

/// Get NPC generation context with race and class overlays.
///
/// # Arguments
///
/// * `registry` - The archetype registry
/// * `role_id` - The role archetype ID
/// * `race_id` - Optional race archetype ID
/// * `class_id` - Optional class archetype ID
///
/// # Returns
///
/// An `NpcGenerationContext` with merged data from all layers.
pub async fn npc_context_with_overlays(
    registry: &ArchetypeRegistry,
    role_id: &str,
    race_id: Option<&str>,
    class_id: Option<&str>,
) -> Result<NpcGenerationContext> {
    // Build resolution query
    let mut query = ResolutionQuery::for_npc(role_id);

    if let Some(race) = race_id {
        query = query.with_race(race);
    }

    if let Some(class) = class_id {
        query = query.with_class(class);
    }

    // Check cache first
    if let Some(cached) = registry.get_cached(&query).await {
        return Ok(npc_context_from_resolved(&cached, race_id, class_id));
    }

    // For now, do a simple lookup of the primary archetype
    // Full hierarchical resolution would use ArchetypeResolver
    let base_context = npc_context_for_archetype(registry, role_id).await?;

    // Update source with overlay info
    Ok(NpcGenerationContext {
        source: NpcGenerationContextSource {
            primary_archetype: Some(role_id.to_string()),
            race_archetype: race_id.map(String::from),
            class_archetype: class_id.map(String::from),
            used_fallback: base_context.source.used_fallback,
            ..Default::default()
        },
        ..base_context
    })
}

/// Convert a resolved archetype to NPC generation context.
fn npc_context_from_resolved(
    resolved: &ResolvedArchetype,
    race_id: Option<&str>,
    class_id: Option<&str>,
) -> NpcGenerationContext {
    // Extract primary archetype from ID if available
    let primary_archetype = resolved.id.as_ref().map(|id| id.to_string());

    // Extract setting pack from resolution metadata query if available
    let setting_pack = resolved
        .resolution_metadata
        .as_ref()
        .and_then(|m| m.query.as_ref())
        .and_then(|q| q.setting.clone());

    NpcGenerationContext {
        role_mappings: resolved.npc_role_mapping.clone(),
        stat_tendencies: resolved.stat_tendencies.clone().unwrap_or_default(),
        personality_affinities: resolved.personality_affinity.clone(),
        vocabulary_bank_id: resolved.vocabulary_bank_id.clone(),
        tags: resolved.tags.clone(),
        source: NpcGenerationContextSource {
            primary_archetype,
            race_archetype: race_id.map(String::from),
            class_archetype: class_id.map(String::from),
            setting_pack,
            used_fallback: false,
        },
    }
}

/// Default role mappings for fallback.
fn default_role_mappings() -> Vec<NpcRoleMapping> {
    vec![NpcRoleMapping::new("commoner", 0.8)]
}

// ============================================================================
// NameGenerator Integration
// ============================================================================

/// Context for name generation from archetype data.
///
/// This struct packages the data that `NameGenerator` needs to
/// generate appropriate names for an archetype.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NamingContext {
    /// Naming cultures with weights.
    ///
    /// The generator selects a culture probabilistically based on weights.
    pub cultures: Vec<NamingCultureWeight>,

    /// Overall title probability modifier.
    ///
    /// If set, overrides individual culture settings.
    pub title_probability: Option<f32>,

    /// Overall epithet probability modifier.
    pub epithet_probability: Option<f32>,

    /// Source information.
    pub source: NamingContextSource,
}

/// Source information for naming context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct NamingContextSource {
    /// Contributing archetype IDs.
    pub contributing_archetypes: Vec<String>,

    /// Setting pack applied.
    pub setting_pack: Option<String>,

    /// Whether fallback defaults were used.
    pub used_fallback: bool,
}


/// Get naming context for an archetype.
///
/// This is the primary integration point for `NameGenerator`.
///
/// # Arguments
///
/// * `registry` - The archetype registry
/// * `archetype_id` - The archetype to query (typically a race)
///
/// # Returns
///
/// A `NamingContext` with culture weights and pattern overrides.
///
/// # Fallback Behavior
///
/// If the archetype is not found, returns a context with "common" culture
/// and sets `used_fallback = true`.
pub async fn naming_context_for_archetype(
    registry: &ArchetypeRegistry,
    archetype_id: &str,
) -> Result<NamingContext> {
    let archetype = match registry.get(archetype_id).await {
        Ok(arch) => arch,
        Err(_) => {
            return Ok(NamingContext {
                cultures: vec![NamingCultureWeight::new("common", 1.0)],
                title_probability: None,
                epithet_probability: None,
                source: NamingContextSource {
                    used_fallback: true,
                    ..Default::default()
                },
            });
        }
    };

    // Extract title and epithet probabilities from the first culture with overrides
    let (title_prob, epithet_prob) = archetype
        .naming_cultures
        .iter()
        .find_map(|c| c.pattern_overrides.as_ref())
        .map(|o| (o.title_probability, o.epithet_probability))
        .unwrap_or((None, None));

    Ok(NamingContext {
        cultures: if archetype.naming_cultures.is_empty() {
            vec![NamingCultureWeight::new("common", 1.0)]
        } else {
            archetype.naming_cultures.clone()
        },
        title_probability: title_prob,
        epithet_probability: epithet_prob,
        source: NamingContextSource {
            contributing_archetypes: vec![archetype_id.to_string()],
            used_fallback: false,
            ..Default::default()
        },
    })
}

/// Get naming context from a resolved archetype.
///
/// Use this when you have a hierarchically resolved archetype.
pub fn naming_context_from_resolved(resolved: &ResolvedArchetype) -> NamingContext {
    let (title_prob, epithet_prob) = resolved
        .naming_cultures
        .iter()
        .find_map(|c| c.pattern_overrides.as_ref())
        .map(|o| (o.title_probability, o.epithet_probability))
        .unwrap_or((None, None));

    // Extract contributing archetypes from resolution metadata if available
    let contributing_archetypes = resolved
        .resolution_metadata
        .as_ref()
        .map(|m| m.layers_checked.clone())
        .unwrap_or_default();

    // Extract setting pack from resolution metadata query if available
    let setting_pack = resolved
        .resolution_metadata
        .as_ref()
        .and_then(|m| m.query.as_ref())
        .and_then(|q| q.setting.clone());

    NamingContext {
        cultures: if resolved.naming_cultures.is_empty() {
            vec![NamingCultureWeight::new("common", 1.0)]
        } else {
            resolved.naming_cultures.clone()
        },
        title_probability: title_prob,
        epithet_probability: epithet_prob,
        source: NamingContextSource {
            contributing_archetypes,
            setting_pack,
            used_fallback: false,
        },
    }
}

/// Select a naming culture based on weights.
///
/// # Arguments
///
/// * `cultures` - Vector of cultures with weights
/// * `rng_value` - Random value from 0.0 to 1.0
///
/// # Returns
///
/// The selected culture ID and its pattern overrides.
pub fn select_naming_culture(
    cultures: &[NamingCultureWeight],
    rng_value: f32,
) -> (&str, Option<&NamingPatternOverrides>) {
    if cultures.is_empty() {
        return ("common", None);
    }

    // Normalize weights
    let total_weight: f32 = cultures.iter().map(|c| c.weight).sum();
    if total_weight <= 0.0 {
        return (&cultures[0].culture, cultures[0].pattern_overrides.as_ref());
    }

    // Select based on weight
    let mut cumulative = 0.0;
    let normalized_value = rng_value * total_weight;

    for culture in cultures {
        cumulative += culture.weight;
        if normalized_value <= cumulative {
            return (&culture.culture, culture.pattern_overrides.as_ref());
        }
    }

    // Fallback to last
    let last = cultures.last().unwrap();
    (&last.culture, last.pattern_overrides.as_ref())
}

// ============================================================================
// Trait Definitions for Implementing Systems
// ============================================================================

/// Trait for systems that consume personality data from the registry.
///
/// Implement this trait to receive personality affinities from archetypes.
#[async_trait::async_trait]
pub trait PersonalityConsumer: Send + Sync {
    /// Process personality context for character generation.
    ///
    /// # Arguments
    ///
    /// * `context` - The personality context from the registry
    ///
    /// # Returns
    ///
    /// Implementation-specific result type.
    async fn apply_personality_context(&self, context: PersonalityContext) -> Result<()>;
}

/// Trait for systems that consume NPC generation data from the registry.
///
/// Implement this trait to receive NPC context from archetypes.
#[async_trait::async_trait]
pub trait NpcGenerationConsumer: Send + Sync {
    /// Process NPC generation context.
    ///
    /// # Arguments
    ///
    /// * `context` - The NPC generation context from the registry
    ///
    /// # Returns
    ///
    /// Implementation-specific result type.
    async fn apply_npc_context(&self, context: NpcGenerationContext) -> Result<()>;
}

/// Trait for systems that consume naming data from the registry.
///
/// Implement this trait to receive naming context from archetypes.
#[async_trait::async_trait]
pub trait NamingConsumer: Send + Sync {
    /// Process naming context for name generation.
    ///
    /// # Arguments
    ///
    /// * `context` - The naming context from the registry
    ///
    /// # Returns
    ///
    /// Implementation-specific result type.
    async fn apply_naming_context(&self, context: NamingContext) -> Result<()>;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // PersonalityAffinityEntry tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_personality_affinity_entry_from() {
        let affinity = PersonalityAffinity::with_intensity("brave", 0.8, 7);
        let entry = PersonalityAffinityEntry::from(&affinity);

        assert_eq!(entry.trait_id, "brave");
        assert_eq!(entry.weight, 0.8);
        assert_eq!(entry.default_intensity, 7);
    }

    // -------------------------------------------------------------------------
    // SpeechPatterns tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_speech_patterns_default() {
        let patterns = SpeechPatterns::default();
        assert!(patterns.greetings.is_empty());
        assert!(patterns.farewells.is_empty());
    }

    // -------------------------------------------------------------------------
    // Default fallback tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_default_personality_affinities() {
        let affinities = default_personality_affinities();
        assert!(!affinities.is_empty());
        assert!(affinities.iter().any(|a| a.trait_id == "neutral"));
    }

    #[test]
    fn test_default_role_mappings() {
        let mappings = default_role_mappings();
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].role, "commoner");
    }

    // -------------------------------------------------------------------------
    // Culture selection tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_select_naming_culture_single() {
        let cultures = vec![NamingCultureWeight::new("dwarvish", 1.0)];

        let (culture, _) = select_naming_culture(&cultures, 0.5);
        assert_eq!(culture, "dwarvish");
    }

    #[test]
    fn test_select_naming_culture_multiple() {
        let cultures = vec![
            NamingCultureWeight::new("dwarvish", 0.8),
            NamingCultureWeight::new("common", 0.2),
        ];

        // Low value should select first (dwarvish)
        let (culture1, _) = select_naming_culture(&cultures, 0.1);
        assert_eq!(culture1, "dwarvish");

        // High value should select second (common)
        let (culture2, _) = select_naming_culture(&cultures, 0.9);
        assert_eq!(culture2, "common");
    }

    #[test]
    fn test_select_naming_culture_empty() {
        let cultures: Vec<NamingCultureWeight> = vec![];

        let (culture, overrides) = select_naming_culture(&cultures, 0.5);
        assert_eq!(culture, "common");
        assert!(overrides.is_none());
    }

    #[test]
    fn test_select_naming_culture_with_overrides() {
        let overrides = NamingPatternOverrides {
            title_probability: Some(0.3),
            epithet_probability: Some(0.5),
            ..Default::default()
        };

        let cultures = vec![NamingCultureWeight::with_overrides("elvish", 1.0, overrides)];

        let (culture, returned_overrides) = select_naming_culture(&cultures, 0.5);
        assert_eq!(culture, "elvish");
        assert!(returned_overrides.is_some());
        assert_eq!(returned_overrides.unwrap().title_probability, Some(0.3));
    }

    // -------------------------------------------------------------------------
    // Context source tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_personality_context_source_default() {
        let source = PersonalityContextSource::default();
        assert!(source.contributing_archetypes.is_empty());
        assert!(source.setting_pack.is_none());
        assert!(!source.used_fallback);
    }

    #[test]
    fn test_npc_generation_context_source_default() {
        let source = NpcGenerationContextSource::default();
        assert!(source.primary_archetype.is_none());
        assert!(!source.used_fallback);
    }

    #[test]
    fn test_naming_context_source_default() {
        let source = NamingContextSource::default();
        assert!(source.contributing_archetypes.is_empty());
        assert!(!source.used_fallback);
    }

    // -------------------------------------------------------------------------
    // Serialization tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_personality_context_serialization() {
        let context = PersonalityContext {
            affinities: vec![PersonalityAffinityEntry {
                trait_id: "brave".to_string(),
                weight: 0.8,
                default_intensity: 7,
            }],
            speech_patterns: Some(SpeechPatterns {
                greetings: vec!["Hello!".to_string()],
                ..Default::default()
            }),
            source: PersonalityContextSource::default(),
        };

        let json = serde_json::to_string(&context).unwrap();
        assert!(json.contains("\"traitId\""));
        assert!(json.contains("\"speechPatterns\""));

        let deserialized: PersonalityContext = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.affinities.len(), 1);
    }

    #[test]
    fn test_npc_generation_context_serialization() {
        let context = NpcGenerationContext {
            role_mappings: vec![NpcRoleMapping::new("guard", 0.9)],
            stat_tendencies: StatTendencies::default(),
            personality_affinities: Vec::new(),
            vocabulary_bank_id: Some("military".to_string()),
            tags: vec!["combat".to_string()],
            source: NpcGenerationContextSource::default(),
        };

        let json = serde_json::to_string(&context).unwrap();
        assert!(json.contains("\"roleMappings\""));
        assert!(json.contains("\"vocabularyBankId\""));

        let deserialized: NpcGenerationContext = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.role_mappings.len(), 1);
    }

    #[test]
    fn test_naming_context_serialization() {
        let context = NamingContext {
            cultures: vec![NamingCultureWeight::new("elvish", 0.9)],
            title_probability: Some(0.3),
            epithet_probability: Some(0.5),
            source: NamingContextSource::default(),
        };

        let json = serde_json::to_string(&context).unwrap();
        assert!(json.contains("\"cultures\""));
        assert!(json.contains("\"titleProbability\""));

        let deserialized: NamingContext = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.cultures.len(), 1);
        assert_eq!(deserialized.title_probability, Some(0.3));
    }
}
