//! Unified Archetype Registry for TTRPG Assistant
//!
//! The Archetype Registry serves as the single source of truth for character
//! archetype data across the application. It consolidates personality affinity,
//! NPC role mapping, vocabulary banks, and naming cultures into a unified system.
//!
//! # Overview
//!
//! This module provides:
//!
//! - **Core Data Models**: [`Archetype`], [`ArchetypeId`], [`ArchetypeCategory`]
//! - **Hierarchical Resolution**: Role -> Race -> Class -> Setting -> Direct ID
//! - **Setting Packs**: Content packs that modify archetype behavior for specific settings
//! - **Vocabulary Banks**: Phrase collections for NPC dialogue generation
//! - **Caching**: LRU cache for resolved archetypes with dependency-based invalidation
//!
//! # Architecture
//!
//! ```text
//!                     +---------------------------+
//!                     |    ArchetypeRegistry      |
//!                     |  (Central Coordination)   |
//!                     +---------------------------+
//!                                |
//!           +--------------------+--------------------+
//!           |                    |                    |
//!           v                    v                    v
//!   +---------------+    +---------------+    +---------------+
//!   | Personality   |    | NPCGenerator  |    | NameGenerator |
//!   | Blender       |    |               |    |               |
//!   +---------------+    +---------------+    +---------------+
//! ```
//!
//! # Integration Points
//!
//! - **PersonalityBlender**: Queries registry for trait affinities
//! - **NPCGenerator**: Queries registry for role mappings and stat tendencies
//! - **NameGenerator**: Queries registry for naming culture weights
//!
//! # Usage Examples
//!
//! ## Creating an Archetype
//!
//! ```rust,ignore
//! use crate::core::archetype::{Archetype, ArchetypeCategory, PersonalityAffinity};
//!
//! let archetype = Archetype::new("dwarf_merchant", "Dwarf Merchant", ArchetypeCategory::Role)
//!     .with_parent("dwarf")
//!     .with_personality_affinity(vec![
//!         PersonalityAffinity::new("stubborn", 0.8),
//!         PersonalityAffinity::new("loyal", 0.7),
//!     ])
//!     .with_naming_cultures(vec![
//!         NamingCultureWeight::new("dwarvish", 1.0),
//!     ]);
//! ```
//!
//! ## Querying for Resolution
//!
//! ```rust,ignore
//! use crate::core::archetype::{ResolutionQuery, ResolvedArchetype};
//!
//! // Direct lookup
//! let query = ResolutionQuery::single("knight_errant");
//!
//! // Hierarchical resolution
//! let query = ResolutionQuery::for_npc("merchant")
//!     .with_race("dwarf")
//!     .with_class("fighter")
//!     .with_setting("forgotten_realms");
//!
//! // Resolve through registry
//! let resolved: ResolvedArchetype = registry.resolve(&query).await?;
//! ```
//!
//! ## Creating a Setting Pack
//!
//! ```rust,ignore
//! use crate::core::archetype::{SettingPack, ArchetypeOverride, PersonalityAffinity};
//!
//! let pack = SettingPack::new("forgotten_realms", "Forgotten Realms", "dnd5e", "1.0.0")
//!     .with_description("Setting pack for the Forgotten Realms campaign setting")
//!     .with_archetype_override(
//!         "dwarf",
//!         ArchetypeOverride::new()
//!             .with_display_name("Shield Dwarf")
//!             .with_affinity_additions(vec![
//!                 PersonalityAffinity::new("clan_loyalty", 0.9),
//!             ]),
//!     );
//! ```
//!
//! # Meilisearch Indexes
//!
//! | Index | Primary Key | Purpose |
//! |-------|-------------|---------|
//! | `ttrpg_archetypes` | `id` | Character archetype definitions |
//! | `ttrpg_npc_vocabulary_banks` | `id` | NPC phrase collections |
//!
//! # Module Structure
//!
//! - [`error`]: Error types for all archetype operations
//! - [`types`]: Core data models (Archetype, ArchetypeCategory, etc.)
//! - [`setting_pack`]: Setting pack types for content customization
//! - [`resolution`]: Query and result types for archetype resolution
//! - [`meilisearch`]: Meilisearch index configuration and management

// ============================================================================
// Module Declarations
// ============================================================================

pub mod cache;
pub mod error;
pub mod integration;
pub mod memory_registry;
pub mod search;
pub mod registry;
pub mod resolution;
pub mod resolver;
pub mod setting_pack;
pub mod setting_pack_loader;
pub mod types;
pub mod vocabulary;

// ============================================================================
// Re-exports: Error Types
// ============================================================================

pub use error::{ArchetypeError, Result};

// ============================================================================
// Re-exports: Meilisearch
// ============================================================================

pub use search::{
    archetype_index_name,
    get_archetype_settings,
    get_vocabulary_bank_settings,
    vocabulary_banks_index_name,
    ArchetypeIndexManager,
    INDEX_ARCHETYPES,
    INDEX_VOCABULARY_BANKS,
};

// ============================================================================
// Re-exports: Core Types
// ============================================================================

pub use types::{
    // Main archetype types
    Archetype,
    ArchetypeCategory,
    ArchetypeId,
    ArchetypeSummary,

    // Affinity and mapping types
    NamingCultureWeight,
    NamingPatternOverrides,
    NpcRoleMapping,
    PersonalityAffinity,
    StatTendencies,
};

// ============================================================================
// Re-exports: Setting Pack Types
// ============================================================================

pub use setting_pack::{
    // Main setting pack types
    SettingPack,
    SettingPackStats,
    SettingPackSummary,

    // Override types
    ArchetypeOverride,
    VocabularyBankOverride,

    // Vocabulary types
    PhraseDefinition,
    VocabularyBankDefinition,

    // Naming culture types
    CustomNamingCulture,

    // Helper functions
    compare_semver,
    is_valid_semver,
    parse_semver,
};

// ============================================================================
// Re-exports: Resolution Types
// ============================================================================

pub use resolution::{
    // Query types
    ResolutionQuery,
    ResolutionQueryBuilder,

    // Result types
    ResolvedArchetype,
    ResolutionMetadata,
    ResolutionResult,
};

// ============================================================================
// Re-exports: Registry and Resolver
// ============================================================================

pub use memory_registry::InMemoryArchetypeRegistry;
pub use registry::{ArchetypeEvent, ArchetypeRegistry};
pub use resolver::ArchetypeResolver;

// ============================================================================
// Re-exports: Setting Pack Loader
// ============================================================================

pub use setting_pack_loader::{SettingPackEvent, SettingPackLoader};

// ============================================================================
// Re-exports: Cache Types
// ============================================================================

pub use cache::{CacheConfig, CacheManager, CacheStats};

// ============================================================================
// Re-exports: Vocabulary Types
// ============================================================================

pub use vocabulary::{
    BankListFilter,
    PhraseFilterOptions,
    VocabularyBank,
    VocabularyBankManager,
    VocabularyBankSummary,
};

// ============================================================================
// Re-exports: Integration Types
// ============================================================================

pub use integration::{
    // PersonalityBlender integration
    blend_for_archetype,
    blend_for_resolved,
    PersonalityAffinityEntry,
    PersonalityContext,
    PersonalityContextSource,
    PersonalityConsumer,
    SpeechPatterns,

    // NPCGenerator integration
    npc_context_for_archetype,
    npc_context_with_overlays,
    NpcGenerationConsumer,
    NpcGenerationContext,
    NpcGenerationContextSource,

    // NameGenerator integration
    naming_context_for_archetype,
    naming_context_from_resolved,
    select_naming_culture,
    NamingConsumer,
    NamingContext,
    NamingContextSource,
};

// ============================================================================
// Prelude Module
// ============================================================================

/// Convenient imports for common archetype operations.
///
/// ```rust,ignore
/// use crate::core::archetype::prelude::*;
/// ```
pub mod prelude {
    pub use super::{
        // Core types
        Archetype,
        ArchetypeCategory,
        ArchetypeId,
        ArchetypeSummary,

        // Component types
        NamingCultureWeight,
        NpcRoleMapping,
        PersonalityAffinity,
        StatTendencies,

        // Resolution
        ResolutionQuery,
        ResolvedArchetype,
        ResolutionMetadata,

        // Setting packs
        ArchetypeOverride,
        SettingPack,
        SettingPackLoader,

        // Registry and Resolver
        ArchetypeRegistry,
        ArchetypeResolver,

        // Cache
        CacheConfig,
        CacheManager,
        CacheStats,

        // Vocabulary
        VocabularyBank,
        VocabularyBankManager,
        PhraseFilterOptions,

        // Integration
        blend_for_archetype,
        npc_context_for_archetype,
        naming_context_for_archetype,
        PersonalityContext,
        NpcGenerationContext,
        NamingContext,

        // Errors
        ArchetypeError,
        Result,
    };
}

// ============================================================================
// Module Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prelude_imports() {
        // Verify all prelude types are accessible
        let _id: ArchetypeId = ArchetypeId::new("test");
        let _category: ArchetypeCategory = ArchetypeCategory::Role;
        let _affinity = PersonalityAffinity::new("test", 0.5);
        let _mapping = NpcRoleMapping::new("test", 0.5);
        let _culture = NamingCultureWeight::new("test", 1.0);
        let _query = ResolutionQuery::single("test");
        let _resolved = ResolvedArchetype::new();
    }

    #[test]
    fn test_archetype_creation_via_exports() {
        let archetype = Archetype::new(
            "test_archetype",
            "Test Archetype",
            ArchetypeCategory::Role,
        );

        assert_eq!(archetype.id.as_str(), "test_archetype");
    }

    #[test]
    fn test_setting_pack_creation_via_exports() {
        let pack = SettingPack::new(
            "test_pack",
            "Test Pack",
            "generic",
            "1.0.0",
        );

        assert_eq!(pack.id, "test_pack");
    }

    #[test]
    fn test_resolution_query_via_exports() {
        let query = ResolutionQuery::for_npc("merchant")
            .with_race("dwarf");

        assert_eq!(query.npc_role, Some("merchant".to_string()));
        assert_eq!(query.race, Some("dwarf".to_string()));
    }

    #[test]
    fn test_semver_helpers_accessible() {
        assert!(is_valid_semver("1.0.0"));
        assert_eq!(parse_semver("1.2.3"), Some((1, 2, 3)));
    }

    #[test]
    fn test_archetype_event_exported() {
        // Verify ArchetypeEvent is accessible
        let event = ArchetypeEvent::Created {
            id: ArchetypeId::new("test"),
        };
        match event {
            ArchetypeEvent::Created { id } => {
                assert_eq!(id.as_str(), "test");
            }
            _ => panic!("Expected Created event"),
        }
    }

    #[test]
    fn test_cache_stats_exported() {
        // Verify CacheStats is accessible (using new cache module)
        let stats = CacheStats {
            hits: 10,
            misses: 5,
            evictions: 2,
            invalidations: 1,
            stale_hits: 0,
            current_size: 50,
            capacity: 256,
        };
        assert_eq!(stats.hits, 10);
        assert_eq!(stats.capacity, 256);
        assert!((stats.hit_rate() - 0.666).abs() < 0.01); // 10/(10+5) = 0.666
    }

    #[test]
    fn test_vocabulary_exports() {
        // Verify vocabulary types are accessible
        let filter = PhraseFilterOptions::for_category("greetings")
            .with_formality(3, 7);
        assert_eq!(filter.category, "greetings");
        assert_eq!(filter.formality_range, Some((3, 7)));
    }

    #[test]
    fn test_cache_config_exported() {
        // Verify CacheConfig is accessible
        let config = CacheConfig::default();
        assert_eq!(config.capacity, 256);
        assert_eq!(config.ttl_seconds, 3600);
    }

    #[tokio::test]
    async fn test_setting_pack_loader_exported() {
        // Verify SettingPackLoader is accessible
        let loader = SettingPackLoader::new();
        assert_eq!(loader.count().await, 0);

        // Load a pack
        let pack = SettingPack::new("test", "Test Pack", "dnd5e", "1.0.0");
        let vkey = loader.load_pack(pack).await.unwrap();
        assert_eq!(vkey, "test@1.0.0");
        assert_eq!(loader.count().await, 1);
    }

    #[test]
    fn test_setting_pack_event_exported() {
        // Verify SettingPackEvent is accessible
        let event = SettingPackEvent::Loaded {
            pack_id: "test".to_string(),
            version: "1.0.0".to_string(),
        };
        match event {
            SettingPackEvent::Loaded { pack_id, version } => {
                assert_eq!(pack_id, "test");
                assert_eq!(version, "1.0.0");
            }
            _ => panic!("Expected Loaded event"),
        }
    }
}
