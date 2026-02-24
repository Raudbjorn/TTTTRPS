//! Personality Module
//!
//! Provides personality profiles and application layer for:
//! - NPC dialogue styling
//! - Narration tone matching
//! - Chat response personality injection
//! - Dynamic personality blending based on gameplay context
//! - Setting-specific personality templates
//!
//! ## Architecture
//!
//! The personality system is organized into several layers:
//!
//! ### Base Layer (personality_base)
//! Core personality profile types and extraction logic, re-exported from
//! `crate::core::personality_base`.
//!
//! ### Application Layer
//! - `application`: Personality application manager, context management
//!
//! ### Extension Layer (Phase 1)
//! - `types`: Core data models with newtype ID wrappers
//! - `errors`: Comprehensive error types for all operations
//! - `context`: GameplayContext enum for automatic switching
//! - `context_keywords`: Keyword-based context detection
//! - `meilisearch`: Meilisearch index configuration and operations
//!
//! ### Template System (Phase 2)
//! - `templates`: SettingTemplate struct with validation
//! - `template_store`: Meilisearch-backed store with LRU cache
//! - `template_loader`: YAML file loading and import/export
//!
//! ### Blending System (Phase 3)
//! - `blender`: PersonalityBlender with weighted interpolation and LRU caching
//! - `context_detector`: GameplayContextDetector with session state integration
//! - `blend_rules`: BlendRuleStore with Meilisearch backend
//!
//! ## Usage
//!
//! ```rust,ignore
//! use crate::core::personality::{
//!     // Base types
//!     PersonalityProfile, PersonalityStore, PersonalityExtractor,
//!     // Application layer
//!     PersonalityApplicationManager, ContentType,
//!     // Extension types
//!     types::{TemplateId, PersonalityId, SettingPersonalityTemplate},
//!     context::GameplayContext,
//!     context_keywords::ContextDetector,
//!     errors::PersonalityExtensionError,
//!     // Template system
//!     templates::{SettingTemplate, TemplateValidationConfig},
//!     template_store::SettingTemplateStore,
//!     template_loader::TemplateLoader,
//!     // Blending system
//!     blender::{PersonalityBlender, BlendSpec, BlendedProfile},
//!     context_detector::{GameplayContextDetector, SessionStateSnapshot},
//!     blend_rules::BlendRuleStore,
//! };
//! ```

// Re-export everything from the base personality module
pub use crate::core::personality_base::*;

// Application layer
pub mod application;
pub use application::*;

// =============================================================================
// Phase 1: Foundation Modules
// =============================================================================

/// Error types for personality extension operations.
///
/// Provides structured error handling with detailed context for:
/// - Template operations (parse, validation, I/O)
/// - Blend operations (weight validation, interpolation)
/// - Blend rule operations (conflicts, evaluation)
/// - Context detection (confidence, ambiguity)
pub mod errors;

/// Core data models for personality extensions.
///
/// Includes:
/// - Newtype ID wrappers (`TemplateId`, `PersonalityId`, `BlendRuleId`)
/// - `SettingPersonalityTemplate` for setting-specific customization
/// - `BlendRule` for context-based blending configuration
/// - Meilisearch document types for storage
pub mod types;

/// Gameplay context enum and related types.
///
/// Defines the `GameplayContext` enum with variants for:
/// - Combat encounters
/// - Social interactions
/// - Exploration
/// - Puzzle/investigation
/// - Lore exposition
/// - Downtime
/// - Rule clarification
pub mod context;

/// Context detection keywords and detector.
///
/// Provides weighted keyword matching for automatic gameplay context
/// detection based on conversation content.
pub mod context_keywords;

/// Meilisearch index configuration for personality storage.
///
/// Manages two indexes:
/// - `ttrpg_personality_templates`: Setting-specific personality templates
/// - `ttrpg_blend_rules`: Context-based blending rules
pub mod search;

// =============================================================================
// Phase 2: Template System Modules
// =============================================================================

/// Setting template struct with validation.
///
/// Provides:
/// - `SettingTemplate` struct extending `SettingPersonalityTemplate`
/// - `TemplateValidationConfig` for configurable validation rules
/// - `TemplateYaml` for YAML serialization/deserialization
/// - Builder pattern for ergonomic template construction
/// - Conversion to `PersonalityProfile` for blending
pub mod templates;

/// Setting template store with Meilisearch and LRU caching.
///
/// Provides:
/// - CRUD operations (get, list_all, save, delete)
/// - Filtering by game_system and setting_name
/// - Full-text search across name, description, vocabulary, phrases
/// - LRU cache (capacity 100) with tokio::sync::RwLock
/// - Import/export to/from YAML
pub mod template_store;

/// YAML template loader for file-based templates.
///
/// Provides:
/// - Loading templates from `assets/settings/` (built-in)
/// - Loading templates from `~/.local/share/ttrpg-assistant/templates/` (user)
/// - Validation during loading with error reporting
/// - Import/export to YAML files
pub mod template_loader;

// =============================================================================
// Phase 3: Blending System Modules
// =============================================================================

/// Personality blender with weighted interpolation.
///
/// Provides:
/// - `PersonalityBlender` for blending multiple personalities
/// - `BlendSpec` for specifying blend operations
/// - Weighted average for numeric fields (formality, trait intensities)
/// - Proportional selection for list fields
/// - LRU cache (capacity 100) with tokio::sync::Mutex
pub mod blender;

/// Enhanced context detector with session state integration.
///
/// Provides:
/// - `GameplayContextDetector` combining keywords and session signals
/// - `SessionStateSnapshot` for capturing session state
/// - History smoothing over last 5 detections
/// - Confidence scoring 0.0-1.0
pub mod context_detector;

/// Blend rule store with Meilisearch backend.
///
/// Provides:
/// - `BlendRuleStore` for CRUD operations on blend rules
/// - Unique constraint on (campaign_id, context)
/// - Filtering by campaign_id, context
/// - Default rule creation for all contexts
pub mod blend_rules;

// =============================================================================
// Phase 4: Integration Layer
// =============================================================================

/// Contextual personality integration.
///
/// Provides:
/// - `ContextualPersonalityManager` combining all Phase 1-3 components
/// - `get_contextual_personality` for automatic context-aware blending
/// - Fallback to default personality when no rules match
pub mod contextual;

// =============================================================================
// Convenience Re-exports
// =============================================================================

// Re-export commonly used extension types at the module level
pub use context::GameplayContext;
pub use context_keywords::{ContextDetector, ContextDetectionConfig};
pub use errors::{
    BlendError, BlendRuleError, ContextDetectionError, PersonalityExtensionError, TemplateError,
};
pub use search::{
    PersonalityIndexError, PersonalityIndexManager, PersonalityIndexStats,
    INDEX_BLEND_RULES, INDEX_PERSONALITY_TEMPLATES,
};
pub use types::{
    BlendComponent, BlendRule, BlendRuleDocument, BlendRuleId, BlendWeightEntry,
    ContextDetectionResult, PersonalityId, SettingPersonalityTemplate, TemplateDocument,
    TemplateId,
};

// Phase 2 re-exports
pub use templates::{SettingTemplate, TemplateValidationConfig, TemplateYaml};
pub use template_store::{
    BatchDeleteResult, BatchImportResult, BatchSaveResult, CacheStats, SettingTemplateStore,
};
pub use template_loader::{LoadError, LoadErrorKind, TemplateLoadResult, TemplateLoader};

// Phase 3 re-exports
pub use blender::{
    BlendComponentSpec, BlendedProfile, BlenderCacheStats, BlendSpec, PersonalityBlender,
    DEFAULT_CACHE_CAPACITY, WEIGHT_TOLERANCE,
};
pub use context_detector::{
    DetectionStats, GameplayContextDetector, SessionStateSnapshot,
    COMBAT_SESSION_BOOST, DEFAULT_HISTORY_SIZE, DETECTION_TARGET_MS,
};
pub use blend_rules::{
    BlendRuleStore, BulkImportResult as RuleBulkImportResult, RuleCacheStats,
    DEFAULT_RULE_CACHE_CAPACITY,
};

// Phase 4 re-exports
pub use contextual::{ContextualConfig, ContextualPersonalityManager, ContextualPersonalityResult};

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify that key types are accessible through the module
        let _id: TemplateId = TemplateId::new("test");
        let _pid: PersonalityId = PersonalityId::new("test");
        let _ctx: GameplayContext = GameplayContext::default();
        let _detector = ContextDetector::new();

        // Verify error types are accessible
        let _err: PersonalityExtensionError = TemplateError::not_found("test").into();
    }

    #[test]
    fn test_gameplay_context_default() {
        let ctx: GameplayContext = Default::default();
        assert_eq!(ctx, GameplayContext::Unknown);
    }

    #[test]
    fn test_context_detector_smoke() {
        let detector = ContextDetector::new();
        let result = detector.detect("I roll for initiative and attack the goblin!");

        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.context, GameplayContext::CombatEncounter);
    }

    // =========================================================================
    // Phase 2: Template System Tests
    // =========================================================================

    #[test]
    fn test_phase2_exports() {
        // Verify Phase 2 types are accessible
        let _config = TemplateValidationConfig::default();
        let _template = SettingTemplate::new("Test", PersonalityId::new("base"));
    }

    #[test]
    fn test_setting_template_builder() {
        let template = SettingTemplate::builder("Test Template", "storyteller")
            .game_system("dnd5e")
            .setting_name("Test Setting")
            .vocabulary("test term", 0.05)
            .common_phrase("Test phrase")
            .tag("test")
            .build();

        assert_eq!(template.name, "Test Template");
        assert_eq!(template.game_system, Some("dnd5e".to_string()));
        assert!(!template.vocabulary_keys.is_empty());
    }

    #[test]
    fn test_validation_config_presets() {
        let default_config = TemplateValidationConfig::default();
        assert_eq!(default_config.min_vocabulary_entries, 10);
        assert_eq!(default_config.max_name_length, 100);
        assert_eq!(default_config.min_description_length, 10);
        assert_eq!(default_config.max_description_length, 500);

        let lenient = TemplateValidationConfig::lenient();
        assert_eq!(lenient.min_vocabulary_entries, 0);

        let strict = TemplateValidationConfig::strict();
        assert!(strict.require_game_system);
        assert!(strict.require_setting_name);
    }

    #[test]
    fn test_load_error_kinds() {
        assert_ne!(LoadErrorKind::IoError, LoadErrorKind::ParseError);
        assert_ne!(LoadErrorKind::ParseError, LoadErrorKind::ValidationError);
    }

    // =========================================================================
    // Phase 3: Blending System Tests
    // =========================================================================

    #[test]
    fn test_phase3_exports() {
        // Verify Phase 3 types are accessible
        let _blender = PersonalityBlender::new();
        let _detector = GameplayContextDetector::new();
        let _snapshot = SessionStateSnapshot::empty();
    }

    #[test]
    fn test_blend_spec_creation() {
        let components = vec![
            BlendComponent::new(PersonalityId::new("a"), 0.6),
            BlendComponent::new(PersonalityId::new("b"), 0.4),
        ];

        let spec = BlendSpec::new(components).unwrap();
        assert!(spec.is_valid());
    }

    #[test]
    fn test_session_state_snapshot() {
        let snapshot = SessionStateSnapshot::combat(4, 1);
        assert!(snapshot.combat_active);
        assert!(snapshot.suggests_combat());

        let social = SessionStateSnapshot::social("npc_001");
        assert!(social.suggests_social());
        assert!(!social.combat_active);
    }

    #[test]
    fn test_gameplay_context_detector() {
        let mut detector = GameplayContextDetector::new();

        let result = detector
            .detect_text_only("Roll for initiative! Attack the goblin!")
            .unwrap();

        // Note: ContextDetectionResult.context is a String (for JSON serialization)
        assert_eq!(result.context, "combat_encounter");
        assert!(result.confidence > 0.3);
    }

    #[test]
    fn test_default_blend_rules() {
        let rules = BlendRuleStore::create_default_rules();

        // Should have rules for all defined contexts
        assert_eq!(rules.len(), GameplayContext::all_defined().len());

        // All should be builtin
        for rule in &rules {
            assert!(rule.is_builtin);
        }
    }

    // =========================================================================
    // Phase 4: Contextual Integration Tests
    // =========================================================================

    #[test]
    fn test_phase4_exports() {
        // Verify Phase 4 types are accessible
        let _config = ContextualConfig::default();
    }

    #[test]
    fn test_contextual_config_defaults() {
        let config = ContextualConfig::default();
        assert_eq!(config.min_confidence_threshold, 0.3);
        assert!(config.use_blend_rules);
        assert!(config.enable_caching);
        assert!(config.default_personality_id.is_none());
    }
}
