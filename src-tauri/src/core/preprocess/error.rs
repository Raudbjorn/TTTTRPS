//! Preprocessing Error Types

use thiserror::Error;

/// Errors that can occur during query preprocessing
#[derive(Debug, Error)]
pub enum PreprocessError {
    #[error("Dictionary load failed: {0}")]
    DictionaryLoad(String),

    #[error("Dictionary not found: {path}")]
    DictionaryNotFound { path: String },

    #[error("Synonym config parse failed: {0}")]
    SynonymParse(String),

    #[error("Dictionary generation failed: {0}")]
    DictionaryGeneration(String),

    #[error("Config parse error: {0}")]
    ConfigParse(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),
}

/// Result type alias for preprocessing operations
pub type PreprocessResult<T> = Result<T, PreprocessError>;
