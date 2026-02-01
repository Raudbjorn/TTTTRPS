//! Storage adapter for Copilot tokens.
//!
//! This module adapts the shared gate storage infrastructure for
//! storing Copilot authentication tokens (GitHub OAuth and Copilot API tokens).

use async_trait::async_trait;

use crate::oauth::copilot::error::{Error, Result};
use crate::oauth::copilot::models::TokenInfo;
use crate::oauth::storage::TokenStorage as GateStorage;

/// Provider identifier for Copilot/GitHub tokens.
pub const COPILOT_PROVIDER_ID: &str = "copilot";

/// Trait for storing and retrieving Copilot tokens.
///
/// This trait is implemented by storage backends (file, keyring, memory)
/// to persist authentication state across sessions.
#[async_trait]
pub trait CopilotTokenStorage: Send + Sync + std::fmt::Debug + 'static {
    /// Loads the stored token info, if any.
    async fn load(&self) -> Result<Option<TokenInfo>>;

    /// Saves token info to storage.
    async fn save(&self, token: &TokenInfo) -> Result<()>;

    /// Removes any stored token info.
    async fn remove(&self) -> Result<()>;
}

// =============================================================================
// Gate Storage Adapter
// =============================================================================

/// Adapter that wraps a gate TokenStorage to implement CopilotTokenStorage.
///
/// This allows the Copilot client to use the shared storage infrastructure
/// while maintaining its own token format.
#[derive(Debug)]
pub struct GateStorageAdapter<S: GateStorage> {
    storage: S,
}

impl<S: GateStorage> GateStorageAdapter<S> {
    /// Creates a new adapter wrapping the given storage.
    pub fn new(storage: S) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl<S: GateStorage + std::fmt::Debug + 'static> CopilotTokenStorage for GateStorageAdapter<S> {
    async fn load(&self) -> Result<Option<TokenInfo>> {
        match self.storage.load(COPILOT_PROVIDER_ID).await {
            Ok(Some(gate_token)) => {
                // Convert from gate TokenInfo to copilot TokenInfo
                // Gate stores: access_token = github_token, refresh_token = copilot_token
                // Gate expires_at is i64 (unix timestamp)
                let copilot_token = TokenInfo {
                    token_type: "github".to_string(),
                    github_token: gate_token.access_token,
                    copilot_token: if gate_token.refresh_token.is_empty() {
                        None
                    } else {
                        Some(gate_token.refresh_token)
                    },
                    copilot_expires_at: if gate_token.expires_at > 0 {
                        Some(gate_token.expires_at)
                    } else {
                        None
                    },
                };
                Ok(Some(copilot_token))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(Error::Storage(e.to_string())),
        }
    }

    async fn save(&self, token: &TokenInfo) -> Result<()> {
        // Convert from copilot TokenInfo to gate TokenInfo
        // Gate expects: access_token, refresh_token (both String), expires_at (i64)
        let gate_token = crate::oauth::token::TokenInfo {
            token_type: "Bearer".to_string(),
            access_token: token.github_token.clone(),
            refresh_token: token.copilot_token.clone().unwrap_or_default(),
            expires_at: token.copilot_expires_at.unwrap_or(0),
            provider: Some(COPILOT_PROVIDER_ID.to_string()),
        };

        self.storage
            .save(COPILOT_PROVIDER_ID, &gate_token)
            .await
            .map_err(|e| Error::Storage(e.to_string()))
    }

    async fn remove(&self) -> Result<()> {
        self.storage
            .remove(COPILOT_PROVIDER_ID)
            .await
            .map_err(|e| Error::Storage(e.to_string()))
    }
}

// =============================================================================
// Memory Storage
// =============================================================================

/// In-memory token storage for testing and development.
#[derive(Debug, Default)]
pub struct MemoryTokenStorage {
    token: std::sync::RwLock<Option<TokenInfo>>,
}

impl MemoryTokenStorage {
    /// Creates a new empty memory storage.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a memory storage with an initial token.
    #[must_use]
    pub fn with_token(token: TokenInfo) -> Self {
        Self {
            token: std::sync::RwLock::new(Some(token)),
        }
    }
}

#[async_trait]
impl CopilotTokenStorage for MemoryTokenStorage {
    async fn load(&self) -> Result<Option<TokenInfo>> {
        let guard = self
            .token
            .read()
            .map_err(|e| Error::Storage(format!("Lock poisoned: {e}")))?;
        Ok(guard.clone())
    }

    async fn save(&self, token: &TokenInfo) -> Result<()> {
        let mut guard = self
            .token
            .write()
            .map_err(|e| Error::Storage(format!("Lock poisoned: {e}")))?;
        *guard = Some(token.clone());
        Ok(())
    }

    async fn remove(&self) -> Result<()> {
        let mut guard = self
            .token
            .write()
            .map_err(|e| Error::Storage(format!("Lock poisoned: {e}")))?;
        *guard = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_storage_empty() {
        let storage = MemoryTokenStorage::new();
        let result = storage.load().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_memory_storage_save_load() {
        let storage = MemoryTokenStorage::new();
        let token = TokenInfo::new("gho_test123");

        storage.save(&token).await.unwrap();

        let loaded = storage.load().await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().github_token, "gho_test123");
    }

    #[tokio::test]
    async fn test_memory_storage_remove() {
        let storage = MemoryTokenStorage::with_token(TokenInfo::new("gho_test"));

        assert!(storage.load().await.unwrap().is_some());

        storage.remove().await.unwrap();

        assert!(storage.load().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_memory_storage_overwrite() {
        let storage = MemoryTokenStorage::new();

        let token1 = TokenInfo::new("gho_first");
        storage.save(&token1).await.unwrap();

        let token2 = TokenInfo::new("gho_second");
        storage.save(&token2).await.unwrap();

        let loaded = storage.load().await.unwrap().unwrap();
        assert_eq!(loaded.github_token, "gho_second");
    }
}
