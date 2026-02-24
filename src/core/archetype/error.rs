//! Error types for the Archetype Registry system.
//!
//! This module defines comprehensive error types for all archetype operations,
//! including resolution, caching, vocabulary management, and setting pack handling.

use thiserror::Error;

/// Result type alias for archetype operations.
pub type Result<T> = std::result::Result<T, ArchetypeError>;

/// Comprehensive error enum for archetype registry operations.
///
/// Each variant provides contextual information to help diagnose and
/// recover from errors in archetype resolution and management.
#[derive(Error, Debug)]
pub enum ArchetypeError {
    // =========================================================================
    // Resolution Errors
    // =========================================================================

    /// Archetype not found after checking all resolution layers.
    ///
    /// The `layers_checked` field provides debugging context showing which
    /// resolution paths were attempted before failing.
    #[error("Archetype not found: {id}, layers checked: {layers_checked:?}")]
    NotFound {
        /// The archetype ID or query that was not found
        id: String,
        /// List of resolution layers that were checked
        layers_checked: Vec<String>,
    },

    /// Circular reference detected in archetype inheritance chain.
    ///
    /// This error prevents infinite loops when resolving parent-child
    /// archetype relationships.
    #[error("Circular resolution detected in inheritance chain: {cycle_path:?}")]
    CircularResolution {
        /// The path of archetype IDs that form the cycle
        cycle_path: Vec<String>,
    },

    /// Resolution exceeded maximum allowed merge operations.
    ///
    /// This prevents runaway resolution for overly complex archetype
    /// hierarchies (limit: 50 operations per AR-206.21).
    #[error("Resolution too complex: {merge_count} merge operations exceeded limit")]
    ResolutionTooComplex {
        /// Number of merge operations attempted
        merge_count: usize,
    },

    /// Inheritance chain exceeded maximum depth limit.
    ///
    /// Maximum inheritance depth is 10 levels (per AR-103.12).
    #[error("Inheritance chain too deep for archetype '{archetype_id}': depth {depth} exceeds limit")]
    InheritanceTooDeep {
        /// The archetype that triggered the depth limit
        archetype_id: String,
        /// The depth at which the limit was exceeded
        depth: usize,
    },

    // =========================================================================
    // Validation Errors
    // =========================================================================

    /// Duplicate archetype ID detected during registration.
    #[error("Duplicate archetype ID: {0}")]
    DuplicateArchetypeId(String),

    /// Referenced parent archetype does not exist.
    #[error("Parent archetype not found: {0}")]
    ParentNotFound(String),

    /// Archetype validation failed.
    #[error("Archetype validation failed: {reason}")]
    ValidationFailed {
        /// Description of the validation failure
        reason: String,
    },

    /// Trait weights exceed allowed maximum (sum > 2.0).
    #[error("Invalid trait weights: sum {actual_sum:.2} exceeds maximum 2.0")]
    InvalidTraitWeights {
        /// The actual sum of trait weights
        actual_sum: f32,
    },

    // =========================================================================
    // Setting Pack Errors
    // =========================================================================

    /// Setting pack validation failed.
    #[error("Setting pack invalid: '{pack_id}' - {reason}")]
    SettingPackInvalid {
        /// The pack ID that failed validation
        pack_id: String,
        /// Description of what made the pack invalid
        reason: String,
    },

    /// Requested setting pack not found.
    #[error("Setting pack not found: {0}")]
    SettingPackNotFound(String),

    /// Requested pack version not found.
    #[error("Pack version not found: {pack_id}@{version}")]
    PackVersionNotFound {
        /// The pack ID
        pack_id: String,
        /// The requested version
        version: String,
    },

    /// Setting pack activation failed due to missing archetype references.
    #[error("Setting pack '{pack_id}' references missing archetypes: {missing_ids:?}")]
    SettingPackReferenceError {
        /// The pack that failed to activate
        pack_id: String,
        /// IDs of archetypes that don't exist
        missing_ids: Vec<String>,
    },

    // =========================================================================
    // Vocabulary Bank Errors
    // =========================================================================

    /// Requested vocabulary bank not found.
    #[error("Vocabulary bank not found: {0}")]
    VocabularyBankNotFound(String),

    /// Cannot delete vocabulary bank because archetypes reference it.
    #[error("Vocabulary bank '{bank_id}' is in use by archetypes: {archetype_ids:?}")]
    VocabularyBankInUse {
        /// The bank ID that cannot be deleted
        bank_id: String,
        /// IDs of archetypes that reference this bank
        archetype_ids: Vec<String>,
    },

    /// Vocabulary bank validation failed.
    #[error("Invalid vocabulary bank: {reason}")]
    InvalidVocabularyBank {
        /// Description of the validation failure
        reason: String,
    },

    // =========================================================================
    // Dependency Errors
    // =========================================================================

    /// Cannot modify archetype because it would break child archetypes.
    #[error("Modification would break child archetypes: {affected_ids:?}")]
    WouldBreakChildren {
        /// IDs of child archetypes that would be affected
        affected_ids: Vec<String>,
    },

    /// Cannot delete archetype because it has dependent children.
    #[error("Archetype has dependent children: {child_ids:?}")]
    HasDependentChildren {
        /// IDs of archetypes that depend on this one
        child_ids: Vec<String>,
    },

    // =========================================================================
    // Infrastructure Errors
    // =========================================================================

    /// Meilisearch operation failed.
    #[error("Meilisearch error: {0}")]
    Meilisearch(String),

    /// JSON serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// YAML parsing error.
    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml_ng::Error),

    /// File system I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Index does not exist and could not be created.
    #[error("Index '{index_name}' does not exist")]
    IndexNotFound {
        /// Name of the missing index
        index_name: String,
    },

    /// Task timed out waiting for completion.
    #[error("Operation timed out: {operation}")]
    Timeout {
        /// Description of the operation that timed out
        operation: String,
    },
}

impl From<crate::core::wilysearch::error::Error> for ArchetypeError {
    fn from(e: crate::core::wilysearch::error::Error) -> Self {
        ArchetypeError::Meilisearch(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_not_found() {
        let err = ArchetypeError::NotFound {
            id: "knight_errant".to_string(),
            layers_checked: vec!["role:warrior".to_string(), "race:human".to_string()],
        };
        let msg = err.to_string();
        assert!(msg.contains("knight_errant"));
        assert!(msg.contains("role:warrior"));
    }

    #[test]
    fn test_error_display_circular_resolution() {
        let err = ArchetypeError::CircularResolution {
            cycle_path: vec!["a".to_string(), "b".to_string(), "a".to_string()],
        };
        let msg = err.to_string();
        assert!(msg.contains("Circular"));
        assert!(msg.contains("a"));
        assert!(msg.contains("b"));
    }

    #[test]
    fn test_error_display_resolution_too_complex() {
        let err = ArchetypeError::ResolutionTooComplex { merge_count: 51 };
        let msg = err.to_string();
        assert!(msg.contains("51"));
        assert!(msg.contains("complex"));
    }

    #[test]
    fn test_error_display_inheritance_too_deep() {
        let err = ArchetypeError::InheritanceTooDeep {
            archetype_id: "deep_child".to_string(),
            depth: 11,
        };
        let msg = err.to_string();
        assert!(msg.contains("deep_child"));
        assert!(msg.contains("11"));
    }

    #[test]
    fn test_error_display_setting_pack_invalid() {
        let err = ArchetypeError::SettingPackInvalid {
            pack_id: "forgotten_realms".to_string(),
            reason: "missing required field 'version'".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("forgotten_realms"));
        assert!(msg.contains("version"));
    }

    #[test]
    fn test_error_display_vocabulary_bank_in_use() {
        let err = ArchetypeError::VocabularyBankInUse {
            bank_id: "dwarvish_merchant".to_string(),
            archetype_ids: vec!["dwarf_merchant".to_string(), "dwarf_smith".to_string()],
        };
        let msg = err.to_string();
        assert!(msg.contains("dwarvish_merchant"));
        assert!(msg.contains("dwarf_merchant"));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: ArchetypeError = io_err.into();
        match err {
            ArchetypeError::Io(_) => (),
            _ => panic!("Expected Io variant"),
        }
    }

    #[test]
    fn test_error_from_serde_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let err: ArchetypeError = json_err.into();
        match err {
            ArchetypeError::Serialization(_) => (),
            _ => panic!("Expected Serialization variant"),
        }
    }
}
