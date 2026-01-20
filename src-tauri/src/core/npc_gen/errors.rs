//! NPC Generation Error Types
//!
//! Defines comprehensive error types for vocabulary, naming, and dialect systems.
//! Uses thiserror for ergonomic error handling with rich context fields.

use std::path::PathBuf;
use thiserror::Error;

// ============================================================================
// Vocabulary Errors
// ============================================================================

/// Errors that can occur when working with vocabulary banks.
#[derive(Error, Debug)]
pub enum VocabularyError {
    /// Failed to load a vocabulary bank from file.
    #[error("Failed to load vocabulary bank '{bank_id}' from {path}: {source}")]
    LoadFailed {
        bank_id: String,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse vocabulary bank YAML.
    #[error("Failed to parse vocabulary bank '{bank_id}': {source}")]
    ParseFailed {
        bank_id: String,
        #[source]
        source: serde_yaml_ng::Error,
    },

    /// Requested vocabulary bank not found.
    #[error("Vocabulary bank '{bank_id}' not found")]
    NotFound { bank_id: String },

    /// Missing required vocabulary bank (critical for operation).
    #[error("Required vocabulary bank '{bank_id}' is missing - system cannot function without it")]
    MissingRequired { bank_id: String },

    /// Invalid vocabulary bank structure.
    #[error("Invalid vocabulary bank structure in '{bank_id}': {reason}")]
    InvalidStructure { bank_id: String, reason: String },

    /// Phrase category not found in bank.
    #[error("Category '{category}' not found in vocabulary bank '{bank_id}'")]
    CategoryNotFound { bank_id: String, category: String },

    /// Empty phrase collection where content was expected.
    #[error("Empty phrase collection in bank '{bank_id}', category '{category}'")]
    EmptyCollection { bank_id: String, category: String },

    /// Cache operation failed.
    #[error("Vocabulary cache operation failed: {reason}")]
    CacheError { reason: String },

    /// Index operation failed.
    #[error("Vocabulary index operation failed: {source}")]
    IndexError {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl VocabularyError {
    /// Create a LoadFailed error.
    pub fn load_failed(bank_id: impl Into<String>, path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::LoadFailed {
            bank_id: bank_id.into(),
            path: path.into(),
            source,
        }
    }

    /// Create a ParseFailed error.
    pub fn parse_failed(bank_id: impl Into<String>, source: serde_yaml_ng::Error) -> Self {
        Self::ParseFailed {
            bank_id: bank_id.into(),
            source,
        }
    }

    /// Create a NotFound error.
    pub fn not_found(bank_id: impl Into<String>) -> Self {
        Self::NotFound {
            bank_id: bank_id.into(),
        }
    }

    /// Create a CategoryNotFound error.
    pub fn category_not_found(bank_id: impl Into<String>, category: impl Into<String>) -> Self {
        Self::CategoryNotFound {
            bank_id: bank_id.into(),
            category: category.into(),
        }
    }

    /// Check if this error is recoverable (can continue with fallback).
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::NotFound { .. }
                | Self::CategoryNotFound { .. }
                | Self::EmptyCollection { .. }
                | Self::CacheError { .. }
        )
    }
}

// ============================================================================
// Name Generation Errors
// ============================================================================

/// Errors that can occur during name generation.
#[derive(Error, Debug)]
pub enum NameGenerationError {
    /// Failed to load name components from file.
    #[error("Failed to load name components for culture '{culture}' from {path}: {source}")]
    LoadFailed {
        culture: String,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse name components YAML.
    #[error("Failed to parse name components for culture '{culture}': {source}")]
    ParseFailed {
        culture: String,
        #[source]
        source: serde_yaml_ng::Error,
    },

    /// Culture not found in name generator.
    #[error("Culture '{culture}' not found in name generator")]
    CultureNotFound { culture: String },

    /// Invalid name pattern specification.
    #[error("Invalid name pattern '{pattern}' for culture '{culture}': {reason}")]
    InvalidPattern {
        culture: String,
        pattern: String,
        reason: String,
    },

    /// Component type not available for culture.
    #[error("Component type '{component_type}' not available for culture '{culture}'")]
    ComponentNotAvailable {
        culture: String,
        component_type: String,
    },

    /// Name generation constraints cannot be satisfied.
    #[error("Cannot satisfy name generation constraints for culture '{culture}': {reason}")]
    ConstraintUnsatisfiable { culture: String, reason: String },

    /// Gender-specific components not available.
    #[error("Gender-specific components for '{gender}' not available in culture '{culture}'")]
    GenderNotAvailable { culture: String, gender: String },

    /// Index operation failed.
    #[error("Name component index operation failed: {source}")]
    IndexError {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl NameGenerationError {
    /// Create a LoadFailed error.
    pub fn load_failed(culture: impl Into<String>, path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::LoadFailed {
            culture: culture.into(),
            path: path.into(),
            source,
        }
    }

    /// Create a ParseFailed error.
    pub fn parse_failed(culture: impl Into<String>, source: serde_yaml_ng::Error) -> Self {
        Self::ParseFailed {
            culture: culture.into(),
            source,
        }
    }

    /// Create a CultureNotFound error.
    pub fn culture_not_found(culture: impl Into<String>) -> Self {
        Self::CultureNotFound {
            culture: culture.into(),
        }
    }

    /// Create an InvalidPattern error.
    pub fn invalid_pattern(
        culture: impl Into<String>,
        pattern: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::InvalidPattern {
            culture: culture.into(),
            pattern: pattern.into(),
            reason: reason.into(),
        }
    }

    /// Check if this error is recoverable.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::CultureNotFound { .. }
                | Self::ComponentNotAvailable { .. }
                | Self::GenderNotAvailable { .. }
        )
    }
}

// ============================================================================
// Dialect Errors
// ============================================================================

/// Errors that can occur during dialect transformation.
#[derive(Error, Debug)]
pub enum DialectError {
    /// Failed to load dialect definition from file.
    #[error("Failed to load dialect '{dialect_id}' from {path}: {source}")]
    LoadFailed {
        dialect_id: String,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse dialect definition YAML.
    #[error("Failed to parse dialect '{dialect_id}': {source}")]
    ParseFailed {
        dialect_id: String,
        #[source]
        source: serde_yaml_ng::Error,
    },

    /// Dialect not found.
    #[error("Dialect '{dialect_id}' not found")]
    NotFound { dialect_id: String },

    /// Invalid regex pattern in dialect rule.
    #[error("Invalid regex pattern in dialect '{dialect_id}', rule '{rule_id}': {source}")]
    InvalidRegex {
        dialect_id: String,
        rule_id: String,
        #[source]
        source: regex::Error,
    },

    /// Transformation produced invalid result.
    #[error("Dialect transformation '{dialect_id}' produced invalid result: {reason}")]
    TransformFailed { dialect_id: String, reason: String },

    /// Dialect intensity out of valid range.
    #[error("Dialect intensity {intensity} out of valid range (0.0-1.0) for '{dialect_id}'")]
    InvalidIntensity { dialect_id: String, intensity: f32 },

    /// Circular dialect dependency detected.
    #[error("Circular dialect dependency detected: {chain}")]
    CircularDependency { chain: String },

    /// Cache operation failed.
    #[error("Dialect cache operation failed: {reason}")]
    CacheError { reason: String },
}

impl DialectError {
    /// Create a LoadFailed error.
    pub fn load_failed(dialect_id: impl Into<String>, path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::LoadFailed {
            dialect_id: dialect_id.into(),
            path: path.into(),
            source,
        }
    }

    /// Create a ParseFailed error.
    pub fn parse_failed(dialect_id: impl Into<String>, source: serde_yaml_ng::Error) -> Self {
        Self::ParseFailed {
            dialect_id: dialect_id.into(),
            source,
        }
    }

    /// Create a NotFound error.
    pub fn not_found(dialect_id: impl Into<String>) -> Self {
        Self::NotFound {
            dialect_id: dialect_id.into(),
        }
    }

    /// Create an InvalidRegex error.
    pub fn invalid_regex(
        dialect_id: impl Into<String>,
        rule_id: impl Into<String>,
        source: regex::Error,
    ) -> Self {
        Self::InvalidRegex {
            dialect_id: dialect_id.into(),
            rule_id: rule_id.into(),
            source,
        }
    }

    /// Create an InvalidIntensity error.
    pub fn invalid_intensity(dialect_id: impl Into<String>, intensity: f32) -> Self {
        Self::InvalidIntensity {
            dialect_id: dialect_id.into(),
            intensity,
        }
    }

    /// Check if this error is recoverable.
    pub fn is_recoverable(&self) -> bool {
        matches!(self, Self::NotFound { .. } | Self::CacheError { .. })
    }
}

// ============================================================================
// File Utility Errors
// ============================================================================

/// Errors that can occur during async file operations.
#[derive(Error, Debug)]
pub enum FileError {
    /// File not found.
    #[error("File not found: {path}")]
    NotFound { path: PathBuf },

    /// Failed to read file.
    #[error("Failed to read file {path}: {source}")]
    ReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse file content.
    #[error("Failed to parse {path} as {format}: {source}")]
    ParseFailed {
        path: PathBuf,
        format: String,
        #[source]
        source: serde_yaml_ng::Error,
    },

    /// Directory scan failed.
    #[error("Failed to scan directory {path}: {source}")]
    ScanFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Invalid file extension.
    #[error("Invalid file extension for {path}: expected {expected}")]
    InvalidExtension { path: PathBuf, expected: String },
}

impl FileError {
    /// Create a NotFound error.
    pub fn not_found(path: impl Into<PathBuf>) -> Self {
        Self::NotFound { path: path.into() }
    }

    /// Create a ReadFailed error.
    pub fn read_failed(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::ReadFailed {
            path: path.into(),
            source,
        }
    }

    /// Create a ParseFailed error.
    pub fn parse_failed(path: impl Into<PathBuf>, format: impl Into<String>, source: serde_yaml_ng::Error) -> Self {
        Self::ParseFailed {
            path: path.into(),
            format: format.into(),
            source,
        }
    }

    /// Create a ScanFailed error.
    pub fn scan_failed(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::ScanFailed {
            path: path.into(),
            source,
        }
    }
}

// ============================================================================
// Unified NPC Extension Error
// ============================================================================

/// Unified error type for all NPC extension operations.
#[derive(Error, Debug)]
pub enum NpcExtensionError {
    #[error(transparent)]
    Vocabulary(#[from] VocabularyError),

    #[error(transparent)]
    NameGeneration(#[from] NameGenerationError),

    #[error(transparent)]
    Dialect(#[from] DialectError),

    #[error(transparent)]
    File(#[from] FileError),

    /// General NPC generation error (legacy compatibility).
    #[error("NPC generation failed: {0}")]
    GenerationFailed(String),

    /// LLM operation error.
    #[error("LLM error: {0}")]
    LLMError(String),

    /// Invalid parameters provided.
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
}

/// Type alias for Result with NpcExtensionError.
pub type Result<T> = std::result::Result<T, NpcExtensionError>;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vocabulary_error_recoverable() {
        let err = VocabularyError::not_found("test_bank");
        assert!(err.is_recoverable());

        let err = VocabularyError::MissingRequired {
            bank_id: "required".to_string(),
        };
        assert!(!err.is_recoverable());
    }

    #[test]
    fn test_name_generation_error_recoverable() {
        let err = NameGenerationError::culture_not_found("elvish");
        assert!(err.is_recoverable());

        let err = NameGenerationError::invalid_pattern("elvish", "{invalid}", "syntax error");
        assert!(!err.is_recoverable());
    }

    #[test]
    fn test_dialect_error_recoverable() {
        let err = DialectError::not_found("scottish");
        assert!(err.is_recoverable());

        let err = DialectError::invalid_intensity("scottish", 1.5);
        assert!(!err.is_recoverable());
    }

    #[test]
    fn test_error_display() {
        let err = VocabularyError::category_not_found("tavern", "greetings");
        let msg = format!("{}", err);
        assert!(msg.contains("greetings"));
        assert!(msg.contains("tavern"));
    }

    #[test]
    fn test_unified_error_from() {
        let vocab_err = VocabularyError::not_found("test");
        let unified: NpcExtensionError = vocab_err.into();
        assert!(matches!(unified, NpcExtensionError::Vocabulary(_)));

        let name_err = NameGenerationError::culture_not_found("test");
        let unified: NpcExtensionError = name_err.into();
        assert!(matches!(unified, NpcExtensionError::NameGeneration(_)));
    }
}
