//! NPC Generation Module
//!
//! Comprehensive NPC generation system for TTRPG applications with support for:
//!
//! - **Vocabulary Banks**: Categorized phrases for dynamic NPC speech
//! - **Cultural Naming**: Culturally-appropriate name generation with multiple structures
//! - **Dialect Transformation**: Text transformation for regional speech patterns
//! - **Meilisearch Integration**: Fast indexed search for vocabulary and names
//!
//! # Module Structure
//!
//! - [`errors`]: Error types for vocabulary, naming, and dialect operations
//! - [`file_utils`]: Async file I/O utilities for YAML loading
//! - [`indexes`]: Meilisearch index configurations and document types
//! - [`vocabulary`]: Vocabulary bank data models and phrase selection
//! - [`names`]: Cultural naming rules and name component models
//! - [`dialects`]: Dialect transformation engine and rules
//! - [`generator`]: Core NPC generation logic (legacy, being extended)
//!
//! # Example
//!
//! ```ignore
//! use crate::core::npc_gen::{
//!     vocabulary::{VocabularyBank, Formality},
//!     names::{CulturalNamingRules, NameStructure},
//!     dialects::{DialectDefinition, DialectTransformer, Intensity},
//! };
//!
//! // Load a vocabulary bank
//! let bank = VocabularyBank::new("tavern")
//!     .with_name("Tavern Keeper Vocabulary")
//!     .with_culture("common");
//!
//! // Get a random greeting
//! let mut rng = rand::thread_rng();
//! if let Some(greeting) = bank.get_greeting(Formality::Casual, &mut rng) {
//!     println!("NPC says: {}", greeting.text);
//! }
//!
//! // Apply dialect transformation
//! let dialect = DialectDefinition::new("scottish")
//!     .add_phonetic_rule(PhoneticRule::new("th_to_d", "th", "d"));
//! let transformer = DialectTransformer::new(dialect).with_intensity(Intensity::Moderate);
//! let result = transformer.transform("The weather is fine!", &mut rng);
//! println!("Transformed: {}", result.transformed);
//! ```
//!
//! # Architecture
//!
//! The NPC generation system follows a layered architecture:
//!
//! 1. **Data Layer**: YAML files defining vocabulary banks, naming rules, dialects
//! 2. **Loading Layer**: Async file utilities with caching
//! 3. **Index Layer**: Meilisearch indexes for fast filtered search
//! 4. **Model Layer**: Rust structs with validation and selection logic
//! 5. **Transformation Layer**: Dialect engine with regex caching
//! 6. **Generation Layer**: NPC generator combining all layers

// ============================================================================
// Submodules
// ============================================================================

/// Error types for NPC generation operations.
pub mod errors;

/// Async file loading utilities for YAML resources.
pub mod file_utils;

/// Meilisearch index configurations for NPC data.
pub mod indexes;

/// Vocabulary bank data models and phrase selection.
pub mod vocabulary;

/// Cultural naming rules and name component models.
pub mod names;

/// Dialect transformation engine and rules.
pub mod dialects;

/// Core NPC generator (legacy implementation).
mod generator;

// ============================================================================
// Re-exports
// ============================================================================

// Error types
pub use errors::{
    DialectError, FileError, NameGenerationError, NpcExtensionError, Result, VocabularyError,
};

// File utilities
pub use file_utils::{
    directory_exists, extract_stem, file_exists, get_dialects_dir, get_names_dir,
    get_npc_data_dir, get_vocabulary_dir, load_all_yaml_files, load_yaml_file,
    load_yaml_file_or_default, load_yaml_file_or_else, resolve_path, scan_yaml_directory,
    FileResult,
};

// Index types
pub use indexes::{
    ensure_npc_indexes, get_npc_index_stats, clear_npc_indexes,
    ExclamationTemplateDocument, NameComponentDocument, NpcIndexError, NpcIndexStats,
    VocabularyPhraseDocument, INDEX_EXCLAMATION_TEMPLATES, INDEX_NAME_COMPONENTS,
    INDEX_VOCABULARY_BANKS,
};

// Vocabulary types
pub use vocabulary::{
    Formality, PatternType, PhraseCategory, PhraseEntry, SpeechPattern, VocabularyBank,
};

// Name types
pub use names::{
    ComponentType, CulturalNamingRules, Gender, GenderRules, NameComponent, NameComponents,
    NamePattern, NameStructure, PhoneticRule as NamePhoneticRule,
};

// Dialect types
pub use dialects::{
    clear_pattern_cache, DialectDefinition, DialectTransformResult, DialectTransformer,
    Emotion, ExclamationIntensity, ExclamationTemplate, GrammaticalRule, Intensity,
    PhoneticRule,
};

// Legacy generator (re-exported from submodule for backward compatibility)
pub use generator::{
    AppearanceDescription, NPC, NPCGenerationOptions, NPCGenerator, NPCGenError,
    NPCPersonality, NPCRelationship, NPCRole, NPCStore, PersonalityDepth, PlotHook,
    PlotHookType, Urgency, VoiceDescription,
};

// ============================================================================
// Integration Types
// ============================================================================

/// Configuration for NPC voice generation.
#[derive(Debug, Clone, Default)]
pub struct NPCVoiceConfig {
    /// Base vocabulary bank ID
    pub vocabulary_bank_id: Option<String>,

    /// Dialect to apply (if any)
    pub dialect_id: Option<String>,

    /// Dialect intensity
    pub dialect_intensity: Intensity,

    /// Culture for name generation
    pub culture_id: Option<String>,

    /// Preferred formality level
    pub formality: Formality,
}

impl NPCVoiceConfig {
    /// Create a new voice configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the vocabulary bank.
    pub fn with_vocabulary(mut self, bank_id: impl Into<String>) -> Self {
        self.vocabulary_bank_id = Some(bank_id.into());
        self
    }

    /// Set the dialect.
    pub fn with_dialect(mut self, dialect_id: impl Into<String>, intensity: Intensity) -> Self {
        self.dialect_id = Some(dialect_id.into());
        self.dialect_intensity = intensity;
        self
    }

    /// Set the culture for naming.
    pub fn with_culture(mut self, culture_id: impl Into<String>) -> Self {
        self.culture_id = Some(culture_id.into());
        self
    }

    /// Set the default formality level.
    pub fn with_formality(mut self, formality: Formality) -> Self {
        self.formality = formality;
        self
    }
}

// ============================================================================
// Module-Level Functions
// ============================================================================

/// Initialize the NPC generation system.
///
/// This ensures all Meilisearch indexes exist and are properly configured.
/// Should be called during application startup.
///
/// # Arguments
/// * `meilisearch_client` - Meilisearch client instance
///
/// # Returns
/// * `Ok(())` - Initialization successful
/// * `Err(String)` - If index creation fails
pub fn initialize_npc_system(
    meili: &meilisearch_lib::MeilisearchLib,
) -> std::result::Result<(), String> {
    log::info!("Initializing NPC generation system...");

    // Ensure indexes exist
    ensure_npc_indexes(meili)?;

    log::info!("NPC generation system initialized successfully");
    Ok(())
}

/// Get statistics about the NPC generation data.
pub fn get_system_stats(
    meili: &meilisearch_lib::MeilisearchLib,
) -> std::result::Result<NpcIndexStats, String> {
    get_npc_index_stats(meili).map_err(|e| e.to_string())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npc_voice_config() {
        let config = NPCVoiceConfig::new()
            .with_vocabulary("tavern")
            .with_dialect("scottish", Intensity::Moderate)
            .with_culture("human")
            .with_formality(Formality::Casual);

        assert_eq!(config.vocabulary_bank_id, Some("tavern".to_string()));
        assert_eq!(config.dialect_id, Some("scottish".to_string()));
        assert_eq!(config.dialect_intensity, Intensity::Moderate);
        assert_eq!(config.culture_id, Some("human".to_string()));
        assert_eq!(config.formality, Formality::Casual);
    }

    #[test]
    fn test_module_exports() {
        // Verify key types are accessible
        let _ = VocabularyBank::new("test");
        let _ = DialectDefinition::new("test");
        let _ = CulturalNamingRules::new("test");
        let _ = PhraseEntry::new("test");
        let _ = NPCGenerator::new();
    }
}
