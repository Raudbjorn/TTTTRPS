//! Token storage backends for persisting OAuth credentials.
//!
//! This module provides the [`TokenStorage`] trait and several implementations:
//!
//! - [`FileTokenStorage`] - Stores tokens in a JSON file with secure permissions
//! - [`MemoryTokenStorage`] - In-memory storage for testing
//! - [`CallbackStorage`] - Custom storage via callbacks
//! - [`KeyringTokenStorage`] - System keyring storage (requires `keyring` feature)
//!
//! # Provider-Aware Storage
//!
//! All storage operations take a `provider` parameter to support multiple
//! LLM providers (e.g., "anthropic", "gemini") in a single storage backend.
//!
//! # Security
//!
//! - File storage uses 0600 permissions on Unix systems
//! - Tokens are never logged (use `#[instrument(skip(token))]`)
//! - All implementations are thread-safe (`Send + Sync`)
//!
//! # Example
//!
//! ```rust,ignore
//! use crate::oauth::storage::{TokenStorage, FileTokenStorage};
//!
//! # async fn example() -> crate::oauth::Result<()> {
//! // Create storage with default path
//! let storage = FileTokenStorage::default_path()?;
//!
//! // Check if token exists for a specific provider
//! if storage.exists("anthropic").await? {
//!     let token = storage.load("anthropic").await?.unwrap();
//!     println!("Token expires at: {}", token.expires_at);
//! }
//! # Ok(())
//! # }
//! ```

mod callback;
mod file;
mod memory;

#[cfg(feature = "keyring")]
mod keyring;

use async_trait::async_trait;

pub use callback::{CallbackStorage, EnvSource, FileSource};
pub use file::FileTokenStorage;
pub use memory::MemoryTokenStorage;

#[cfg(feature = "keyring")]
pub use keyring::KeyringTokenStorage;

use super::token::TokenInfo;
use super::Result;

/// Trait for token storage backends.
///
/// All storage implementations must be thread-safe (`Send + Sync`)
/// to support concurrent access from multiple tasks.
///
/// # Provider Parameter
///
/// All operations take a `provider` parameter (e.g., "anthropic", "gemini")
/// to support storing tokens for multiple LLM providers in a single backend.
///
/// # Security Notes
///
/// - Never log token values in implementations
/// - Use `#[instrument(skip(token))]` when tracing save operations
/// - Ensure file permissions are restrictive (0600 on Unix)
///
/// # Example Implementation
///
/// ```rust,ignore
/// use async_trait::async_trait;
/// use crate::oauth::{TokenStorage, TokenInfo, Result};
///
/// struct MyStorage { /* ... */ }
///
/// #[async_trait]
/// impl TokenStorage for MyStorage {
///     async fn load(&self, provider: &str) -> Result<Option<TokenInfo>> {
///         // Load token for the given provider
///         todo!()
///     }
///
///     async fn save(&self, provider: &str, token: &TokenInfo) -> Result<()> {
///         // Save token for the given provider
///         todo!()
///     }
///
///     async fn remove(&self, provider: &str) -> Result<()> {
///         // Remove token for the given provider
///         todo!()
///     }
///
///     fn name(&self) -> &str {
///         "my-storage"
///     }
/// }
/// ```
#[async_trait]
pub trait TokenStorage: Send + Sync {
    /// Load the stored token for a provider, if any.
    ///
    /// # Arguments
    ///
    /// * `provider` - Provider identifier (e.g., "anthropic", "gemini")
    ///
    /// # Returns
    ///
    /// - `Ok(Some(token))` if a token exists for the provider
    /// - `Ok(None)` if no token is stored for the provider
    /// - `Err(_)` if there's an error accessing storage
    async fn load(&self, provider: &str) -> Result<Option<TokenInfo>>;

    /// Save a token for a provider to storage.
    ///
    /// Overwrites any existing token for the provider. Implementations should
    /// ensure appropriate file permissions and atomic writes.
    ///
    /// # Arguments
    ///
    /// * `provider` - Provider identifier (e.g., "anthropic", "gemini")
    /// * `token` - The token information to save
    async fn save(&self, provider: &str, token: &TokenInfo) -> Result<()>;

    /// Remove the stored token for a provider.
    ///
    /// Returns `Ok(())` even if no token was stored for the provider.
    ///
    /// # Arguments
    ///
    /// * `provider` - Provider identifier (e.g., "anthropic", "gemini")
    async fn remove(&self, provider: &str) -> Result<()>;

    /// Check if a token exists in storage for a provider.
    ///
    /// Default implementation calls `load()`, but implementations
    /// may provide a more efficient check.
    ///
    /// # Arguments
    ///
    /// * `provider` - Provider identifier (e.g., "anthropic", "gemini")
    async fn exists(&self, provider: &str) -> Result<bool> {
        Ok(self.load(provider).await?.is_some())
    }

    /// Get the name of this storage backend.
    ///
    /// Used for logging and debugging. Default is "unknown".
    fn name(&self) -> &str {
        "unknown"
    }
}

/// Blanket implementation for `Arc<T>` where T: TokenStorage
#[async_trait]
impl<T: TokenStorage + ?Sized> TokenStorage for std::sync::Arc<T> {
    async fn load(&self, provider: &str) -> Result<Option<TokenInfo>> {
        (**self).load(provider).await
    }

    async fn save(&self, provider: &str, token: &TokenInfo) -> Result<()> {
        (**self).save(provider, token).await
    }

    async fn remove(&self, provider: &str) -> Result<()> {
        (**self).remove(provider).await
    }

    async fn exists(&self, provider: &str) -> Result<bool> {
        (**self).exists(provider).await
    }

    fn name(&self) -> &str {
        (**self).name()
    }
}

/// Blanket implementation for `Box<T>` where T: TokenStorage
#[async_trait]
impl<T: TokenStorage + ?Sized> TokenStorage for Box<T> {
    async fn load(&self, provider: &str) -> Result<Option<TokenInfo>> {
        (**self).load(provider).await
    }

    async fn save(&self, provider: &str, token: &TokenInfo) -> Result<()> {
        (**self).save(provider, token).await
    }

    async fn remove(&self, provider: &str) -> Result<()> {
        (**self).remove(provider).await
    }

    async fn exists(&self, provider: &str) -> Result<bool> {
        (**self).exists(provider).await
    }

    fn name(&self) -> &str {
        (**self).name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // Test that Arc<T> implements TokenStorage when T does
    #[tokio::test]
    async fn test_arc_storage() {
        let storage = Arc::new(MemoryTokenStorage::new());

        // Create a token
        let token = TokenInfo::new("access".into(), "refresh".into(), 3600);

        // Save via Arc
        storage.save("anthropic", &token).await.unwrap();

        // Load via Arc
        let loaded = storage.load("anthropic").await.unwrap().unwrap();
        assert_eq!(loaded.access_token, "access");

        // Check name
        assert_eq!(storage.name(), "memory");
    }

    // Test that Box<dyn TokenStorage> works
    #[tokio::test]
    async fn test_box_dyn_storage() {
        let storage: Box<dyn TokenStorage> = Box::new(MemoryTokenStorage::new());

        let token = TokenInfo::new("access".into(), "refresh".into(), 3600);
        storage.save("gemini", &token).await.unwrap();

        let loaded = storage.load("gemini").await.unwrap().unwrap();
        assert_eq!(loaded.access_token, "access");
    }

    // Test multiple providers in same storage
    #[tokio::test]
    async fn test_multiple_providers() {
        let storage = MemoryTokenStorage::new();

        let anthropic_token = TokenInfo::new("anthropic_access".into(), "refresh1".into(), 3600);
        let gemini_token = TokenInfo::new("gemini_access".into(), "refresh2".into(), 3600);

        storage.save("anthropic", &anthropic_token).await.unwrap();
        storage.save("gemini", &gemini_token).await.unwrap();

        let loaded_anthropic = storage.load("anthropic").await.unwrap().unwrap();
        let loaded_gemini = storage.load("gemini").await.unwrap().unwrap();

        assert_eq!(loaded_anthropic.access_token, "anthropic_access");
        assert_eq!(loaded_gemini.access_token, "gemini_access");

        // Remove one, verify other remains
        storage.remove("anthropic").await.unwrap();
        assert!(storage.load("anthropic").await.unwrap().is_none());
        assert!(storage.load("gemini").await.unwrap().is_some());
    }
}
