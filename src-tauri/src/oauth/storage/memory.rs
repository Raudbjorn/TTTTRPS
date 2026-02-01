//! In-memory token storage for testing and ephemeral use.
//!
//! This module provides [`MemoryTokenStorage`], a thread-safe in-memory
//! token storage backend. Useful for:
//!
//! - Unit tests that need isolated token storage
//! - Short-lived applications that don't need persistence
//! - Caching layer in front of persistent storage

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::instrument;

use super::TokenStorage;
use crate::oauth::token::TokenInfo;
use crate::oauth::Result;

/// In-memory token storage.
///
/// Uses `Arc<RwLock<HashMap<String, TokenInfo>>>` for thread-safe access from
/// multiple async tasks. The storage is Clone and can be shared across
/// the application.
///
/// # Example
///
/// ```rust,ignore
/// use crate::oauth::storage::MemoryTokenStorage;
/// use crate::oauth::token::TokenInfo;
/// use crate::oauth::storage::TokenStorage;
///
/// # async fn example() -> crate::oauth::Result<()> {
/// // Create empty storage
/// let storage = MemoryTokenStorage::new();
///
/// // Or create with an initial token for a provider
/// let token = TokenInfo::new("access".into(), "refresh".into(), 3600);
/// let storage = MemoryTokenStorage::with_token("anthropic", token);
///
/// // Storage can be cloned and shared
/// let storage2 = storage.clone();
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct MemoryTokenStorage {
    /// Thread-safe token storage keyed by provider.
    inner: Arc<RwLock<HashMap<String, TokenInfo>>>,
}

impl Default for MemoryTokenStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryTokenStorage {
    /// Create a new empty MemoryTokenStorage.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use crate::oauth::storage::MemoryTokenStorage;
    ///
    /// let storage = MemoryTokenStorage::new();
    /// ```
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a MemoryTokenStorage with an initial token for a provider.
    ///
    /// Useful for testing scenarios where you want to start
    /// with a pre-populated token.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use crate::oauth::storage::MemoryTokenStorage;
    /// use crate::oauth::token::TokenInfo;
    ///
    /// let token = TokenInfo::new(
    ///     "access_token".into(),
    ///     "refresh_token".into(),
    ///     3600,
    /// );
    /// let storage = MemoryTokenStorage::with_token("anthropic", token);
    /// ```
    pub fn with_token(provider: impl Into<String>, token: TokenInfo) -> Self {
        let mut map = HashMap::new();
        map.insert(provider.into(), token);
        Self {
            inner: Arc::new(RwLock::new(map)),
        }
    }

    /// Create a MemoryTokenStorage with multiple initial tokens.
    ///
    /// Useful for testing scenarios with multiple providers.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use crate::oauth::storage::MemoryTokenStorage;
    /// use crate::oauth::token::TokenInfo;
    ///
    /// let tokens = vec![
    ///     ("anthropic", TokenInfo::new("access1".into(), "refresh1".into(), 3600)),
    ///     ("gemini", TokenInfo::new("access2".into(), "refresh2".into(), 3600)),
    /// ];
    /// let storage = MemoryTokenStorage::with_tokens(tokens);
    /// ```
    pub fn with_tokens<I, S>(tokens: I) -> Self
    where
        I: IntoIterator<Item = (S, TokenInfo)>,
        S: Into<String>,
    {
        let map: HashMap<String, TokenInfo> = tokens
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect();
        Self {
            inner: Arc::new(RwLock::new(map)),
        }
    }

    /// Get a snapshot of the current token for a provider synchronously.
    ///
    /// This method attempts a non-blocking read lock. If the lock cannot be
    /// acquired immediately (e.g., because a write is in progress), it returns `None`.
    /// This is safe to call from both sync and async contexts.
    ///
    /// For reliable access in async code, use [`Self::load`] instead.
    pub fn get_sync(&self, provider: &str) -> Option<TokenInfo> {
        self.inner
            .try_read()
            .ok()
            .and_then(|guard| guard.get(provider).cloned())
    }

    /// Get the number of stored tokens.
    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    /// Check if storage is empty.
    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.is_empty()
    }

    /// Get all provider names that have stored tokens.
    pub async fn providers(&self) -> Vec<String> {
        self.inner.read().await.keys().cloned().collect()
    }

    /// Clear all stored tokens.
    pub async fn clear(&self) {
        self.inner.write().await.clear();
    }
}

#[async_trait]
impl TokenStorage for MemoryTokenStorage {
    #[instrument(skip(self))]
    async fn load(&self, provider: &str) -> Result<Option<TokenInfo>> {
        let guard = self.inner.read().await;
        Ok(guard.get(provider).cloned())
    }

    #[instrument(skip(self, token))]
    async fn save(&self, provider: &str, token: &TokenInfo) -> Result<()> {
        let mut guard = self.inner.write().await;
        guard.insert(provider.to_string(), token.clone());
        Ok(())
    }

    #[instrument(skip(self))]
    async fn remove(&self, provider: &str) -> Result<()> {
        let mut guard = self.inner.write().await;
        guard.remove(provider);
        Ok(())
    }

    async fn exists(&self, provider: &str) -> Result<bool> {
        let guard = self.inner.read().await;
        Ok(guard.contains_key(provider))
    }

    fn name(&self) -> &str {
        "memory"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_is_empty() {
        let storage = MemoryTokenStorage::new();
        assert!(storage.load("anthropic").await.unwrap().is_none());
        assert!(!storage.exists("anthropic").await.unwrap());
        assert!(storage.is_empty().await);
    }

    #[tokio::test]
    async fn test_with_token() {
        let token = TokenInfo::new("access".into(), "refresh".into(), 3600);
        let storage = MemoryTokenStorage::with_token("anthropic", token);

        let loaded = storage.load("anthropic").await.unwrap().unwrap();
        assert_eq!(loaded.access_token, "access");
        assert!(storage.exists("anthropic").await.unwrap());
        assert!(!storage.is_empty().await);
    }

    #[tokio::test]
    async fn test_with_tokens() {
        let tokens = vec![
            ("anthropic", TokenInfo::new("access1".into(), "refresh1".into(), 3600)),
            ("gemini", TokenInfo::new("access2".into(), "refresh2".into(), 3600)),
        ];
        let storage = MemoryTokenStorage::with_tokens(tokens);

        assert_eq!(storage.len().await, 2);
        assert!(storage.exists("anthropic").await.unwrap());
        assert!(storage.exists("gemini").await.unwrap());
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let storage = MemoryTokenStorage::new();

        let token = TokenInfo::new("access".into(), "refresh".into(), 3600);
        storage.save("anthropic", &token).await.unwrap();

        let loaded = storage.load("anthropic").await.unwrap().unwrap();
        assert_eq!(loaded.access_token, "access");
        assert_eq!(loaded.refresh_token, "refresh");
    }

    #[tokio::test]
    async fn test_remove() {
        let token = TokenInfo::new("access".into(), "refresh".into(), 3600);
        let storage = MemoryTokenStorage::with_token("anthropic", token);

        assert!(storage.exists("anthropic").await.unwrap());
        storage.remove("anthropic").await.unwrap();
        assert!(!storage.exists("anthropic").await.unwrap());
        assert!(storage.load("anthropic").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_remove_empty() {
        let storage = MemoryTokenStorage::new();
        // Should not error when removing from empty storage
        storage.remove("nonexistent").await.unwrap();
    }

    #[tokio::test]
    async fn test_overwrite() {
        let storage = MemoryTokenStorage::new();

        let token1 = TokenInfo::new("access1".into(), "refresh1".into(), 3600);
        storage.save("anthropic", &token1).await.unwrap();

        let token2 = TokenInfo::new("access2".into(), "refresh2".into(), 7200);
        storage.save("anthropic", &token2).await.unwrap();

        let loaded = storage.load("anthropic").await.unwrap().unwrap();
        assert_eq!(loaded.access_token, "access2");
        assert_eq!(loaded.refresh_token, "refresh2");
    }

    #[tokio::test]
    async fn test_clone_shares_state() {
        let storage1 = MemoryTokenStorage::new();
        let storage2 = storage1.clone();

        let token = TokenInfo::new("access".into(), "refresh".into(), 3600);
        storage1.save("anthropic", &token).await.unwrap();

        // Storage2 should see the token saved via storage1
        let loaded = storage2.load("anthropic").await.unwrap().unwrap();
        assert_eq!(loaded.access_token, "access");
    }

    #[tokio::test]
    async fn test_multiple_providers() {
        let storage = MemoryTokenStorage::new();

        let token1 = TokenInfo::new("anthropic_access".into(), "refresh1".into(), 3600);
        let token2 = TokenInfo::new("gemini_access".into(), "refresh2".into(), 3600);

        storage.save("anthropic", &token1).await.unwrap();
        storage.save("gemini", &token2).await.unwrap();

        assert_eq!(storage.len().await, 2);

        let loaded1 = storage.load("anthropic").await.unwrap().unwrap();
        let loaded2 = storage.load("gemini").await.unwrap().unwrap();

        assert_eq!(loaded1.access_token, "anthropic_access");
        assert_eq!(loaded2.access_token, "gemini_access");
    }

    #[tokio::test]
    async fn test_providers() {
        let storage = MemoryTokenStorage::new();

        let token = TokenInfo::new("access".into(), "refresh".into(), 3600);
        storage.save("anthropic", &token).await.unwrap();
        storage.save("gemini", &token).await.unwrap();

        let mut providers = storage.providers().await;
        providers.sort();
        assert_eq!(providers, vec!["anthropic", "gemini"]);
    }

    #[tokio::test]
    async fn test_clear() {
        let storage = MemoryTokenStorage::new();

        let token = TokenInfo::new("access".into(), "refresh".into(), 3600);
        storage.save("anthropic", &token).await.unwrap();
        storage.save("gemini", &token).await.unwrap();

        assert_eq!(storage.len().await, 2);
        storage.clear().await;
        assert!(storage.is_empty().await);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let storage = MemoryTokenStorage::new();

        // Spawn multiple tasks that read and write concurrently
        let mut handles = vec![];

        for i in 0..10 {
            let storage = storage.clone();
            let handle = tokio::spawn(async move {
                let provider = format!("provider-{}", i);
                let token = TokenInfo::new(format!("access{}", i), "refresh".into(), 3600);
                storage.save(&provider, &token).await.unwrap();
                storage.load(&provider).await.unwrap()
            });
            handles.push(handle);
        }

        // All tasks should complete without panicking
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_some());
        }
    }

    #[tokio::test]
    async fn test_storage_name() {
        let storage = MemoryTokenStorage::new();
        assert_eq!(storage.name(), "memory");
    }

    #[test]
    fn test_default() {
        let storage = MemoryTokenStorage::default();
        // get_sync on empty should return None
        assert!(storage.get_sync("anthropic").is_none());
    }

    #[tokio::test]
    async fn test_composite_token() {
        let storage = MemoryTokenStorage::new();

        let token = TokenInfo::new("access".into(), "refresh".into(), 3600)
            .with_project_ids("proj-123", Some("managed-456"));
        storage.save("anthropic", &token).await.unwrap();

        let loaded = storage.load("anthropic").await.unwrap().unwrap();
        let (base, project, managed) = loaded.parse_refresh_parts();
        assert_eq!(base, "refresh");
        assert_eq!(project.as_deref(), Some("proj-123"));
        assert_eq!(managed.as_deref(), Some("managed-456"));
    }

    #[tokio::test]
    async fn test_load_nonexistent_provider() {
        let storage = MemoryTokenStorage::new();

        let token = TokenInfo::new("access".into(), "refresh".into(), 3600);
        storage.save("anthropic", &token).await.unwrap();

        // Loading a different provider should return None
        assert!(storage.load("gemini").await.unwrap().is_none());
        assert!(!storage.exists("gemini").await.unwrap());
    }
}
