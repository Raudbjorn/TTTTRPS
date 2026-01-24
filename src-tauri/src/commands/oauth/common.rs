//! Common OAuth Infrastructure
//!
//! Shared types and traits for OAuth providers.

use serde::{Deserialize, Serialize};

// Re-export unified gate types
pub use crate::gate::{OAuthFlowState as GateOAuthFlowState, TokenInfo as GateTokenInfo};

/// Common storage backend enum used by all OAuth providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum StorageBackend {
    /// File-based storage
    File,
    /// System keyring storage
    Keyring,
    /// Auto-select (keyring if available, else file)
    #[default]
    Auto,
}


impl std::fmt::Display for StorageBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File => write!(f, "file"),
            Self::Keyring => write!(f, "keyring"),
            Self::Auto => write!(f, "auto"),
        }
    }
}

impl std::str::FromStr for StorageBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "file" => Ok(Self::File),
            "keyring" => Ok(Self::Keyring),
            "auto" => Ok(Self::Auto),
            _ => Err(format!("Unknown storage backend: {}. Valid options: file, keyring, auto", s)),
        }
    }
}
