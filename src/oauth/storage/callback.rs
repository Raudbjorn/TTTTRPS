//! Callback-based token storage for maximum flexibility.
//!
//! This module provides [`CallbackStorage`], which allows users to provide
//! their own load/save/remove functions, enabling integration with any
//! storage backend.
//!
//! Also provides pre-built sources like [`FileSource`] and [`EnvSource`]
//! for common use cases.

use async_trait::async_trait;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use super::TokenStorage;
use crate::oauth::token::TokenInfo;
use crate::oauth::Result;

/// Type alias for the load callback function signature.
pub type LoadFn = Box<
    dyn Fn(&str) -> Pin<Box<dyn Future<Output = Result<Option<TokenInfo>>> + Send>> + Send + Sync,
>;

/// Type alias for the save callback function signature.
pub type SaveFn = Box<
    dyn Fn(&str, TokenInfo) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync,
>;

/// Type alias for the remove callback function signature.
pub type RemoveFn =
    Box<dyn Fn(&str) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;

/// Callback-based token storage for maximum flexibility.
///
/// Allows users to provide their own load/save/remove functions,
/// enabling integration with any storage backend.
///
/// # Example
///
/// ```rust,ignore
/// use crate::oauth::storage::{CallbackStorage, TokenStorage};
/// use crate::oauth::TokenInfo;
/// use std::sync::Arc;
/// use tokio::sync::RwLock;
/// use std::collections::HashMap;
///
/// // Create a simple in-memory storage using callbacks
/// let tokens = Arc::new(RwLock::new(HashMap::new()));
///
/// let load_tokens = tokens.clone();
/// let save_tokens = tokens.clone();
/// let remove_tokens = tokens.clone();
///
/// let storage = CallbackStorage::new(
///     Box::new(move |provider: &str| {
///         let tokens = load_tokens.clone();
///         let provider = provider.to_string();
///         Box::pin(async move {
///             Ok(tokens.read().await.get(&provider).cloned())
///         })
///     }),
///     Box::new(move |provider: &str, token: TokenInfo| {
///         let tokens = save_tokens.clone();
///         let provider = provider.to_string();
///         Box::pin(async move {
///             tokens.write().await.insert(provider, token);
///             Ok(())
///         })
///     }),
///     Box::new(move |provider: &str| {
///         let tokens = remove_tokens.clone();
///         let provider = provider.to_string();
///         Box::pin(async move {
///             tokens.write().await.remove(&provider);
///             Ok(())
///         })
///     }),
/// );
/// ```
pub struct CallbackStorage {
    load_fn: LoadFn,
    save_fn: SaveFn,
    remove_fn: RemoveFn,
}

impl CallbackStorage {
    /// Create a new callback-based storage.
    ///
    /// # Arguments
    ///
    /// * `load_fn` - Function to load a token for a given provider
    /// * `save_fn` - Function to save a token for a given provider
    /// * `remove_fn` - Function to remove a token for a given provider
    #[must_use]
    pub fn new(load_fn: LoadFn, save_fn: SaveFn, remove_fn: RemoveFn) -> Self {
        Self {
            load_fn,
            save_fn,
            remove_fn,
        }
    }
}

#[async_trait]
impl TokenStorage for CallbackStorage {
    async fn load(&self, provider: &str) -> Result<Option<TokenInfo>> {
        (self.load_fn)(provider).await
    }

    async fn save(&self, provider: &str, token: &TokenInfo) -> Result<()> {
        (self.save_fn)(provider, token.clone()).await
    }

    async fn remove(&self, provider: &str) -> Result<()> {
        (self.remove_fn)(provider).await
    }

    fn name(&self) -> &str {
        "callback"
    }
}

/// File-based source for callback storage.
///
/// A convenience wrapper that creates load/save/remove callbacks
/// for file-based storage. Useful when you need more control over
/// the callbacks than [`FileTokenStorage`] provides.
///
/// [`FileTokenStorage`]: super::FileTokenStorage
#[derive(Clone)]
pub struct FileSource {
    path: PathBuf,
}

impl FileSource {
    /// Create a new file source.
    ///
    /// The path can include `~` which will be expanded to the home directory.
    #[must_use]
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path_str = path.as_ref().to_string_lossy();
        let expanded = if path_str.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                home.join(&path_str[2..])
            } else {
                path.as_ref().to_path_buf()
            }
        } else if path_str == "~" {
            dirs::home_dir().unwrap_or_else(|| path.as_ref().to_path_buf())
        } else {
            path.as_ref().to_path_buf()
        };

        Self { path: expanded }
    }

    /// Get the file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Environment variable source for callback storage.
///
/// Creates a read-only source that loads tokens from environment variables.
/// Since environment variables are typically read-only at runtime,
/// save and remove operations will return errors.
///
/// # Example
///
/// ```rust,ignore
/// use crate::oauth::storage::EnvSource;
///
/// // From a single JSON-encoded environment variable
/// let source = EnvSource::json("MY_TOKEN");
///
/// // From separate environment variables for each field
/// let source = EnvSource::parts(
///     "MY_ACCESS_TOKEN",
///     "MY_REFRESH_TOKEN",
///     "MY_TOKEN_EXPIRES",
/// );
/// ```
pub struct EnvSource {
    /// Environment variable configuration
    config: EnvConfig,
}

enum EnvConfig {
    /// Single JSON-encoded variable
    Json(String),
    /// Separate variables for each field
    Parts {
        access_var: String,
        refresh_var: String,
        expires_var: String,
    },
}

impl EnvSource {
    /// Create a source from a single JSON-encoded environment variable.
    ///
    /// The variable should contain the full token as JSON.
    ///
    /// # Arguments
    ///
    /// * `var_name` - Name of the environment variable
    #[must_use]
    pub fn json(var_name: impl Into<String>) -> Self {
        Self {
            config: EnvConfig::Json(var_name.into()),
        }
    }

    /// Create a source from separate environment variables.
    ///
    /// # Arguments
    ///
    /// * `access_var` - Variable containing the access token
    /// * `refresh_var` - Variable containing the refresh token
    /// * `expires_var` - Variable containing the expiration timestamp (Unix seconds)
    #[must_use]
    pub fn parts(
        access_var: impl Into<String>,
        refresh_var: impl Into<String>,
        expires_var: impl Into<String>,
    ) -> Self {
        Self {
            config: EnvConfig::Parts {
                access_var: access_var.into(),
                refresh_var: refresh_var.into(),
                expires_var: expires_var.into(),
            },
        }
    }

    /// Load a token from the environment variables.
    ///
    /// Note: The provider parameter is ignored since environment variables
    /// don't support multiple providers in a meaningful way.
    pub fn load(&self) -> Result<Option<TokenInfo>> {
        match &self.config {
            EnvConfig::Json(var_name) => {
                match std::env::var(var_name) {
                    Ok(json) if !json.is_empty() => {
                        let token: TokenInfo = serde_json::from_str(&json)?;
                        Ok(Some(token))
                    }
                    _ => Ok(None),
                }
            }
            EnvConfig::Parts {
                access_var,
                refresh_var,
                expires_var,
            } => {
                let access = std::env::var(access_var).ok();
                let refresh = std::env::var(refresh_var).ok();
                let expires = std::env::var(expires_var)
                    .ok()
                    .and_then(|s| s.parse::<i64>().ok());

                match (access, refresh, expires) {
                    (Some(access_token), Some(refresh_token), Some(expires_at)) => {
                        Ok(Some(TokenInfo {
                            token_type: "oauth".to_string(),
                            access_token,
                            refresh_token,
                            expires_at,
                            provider: None,
                        }))
                    }
                    _ => Ok(None),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_callback_storage() {
        let tokens: Arc<RwLock<HashMap<String, TokenInfo>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let load_tokens = tokens.clone();
        let save_tokens = tokens.clone();
        let remove_tokens = tokens.clone();

        let storage = CallbackStorage::new(
            Box::new(move |provider: &str| {
                let tokens = load_tokens.clone();
                let provider = provider.to_string();
                Box::pin(async move { Ok(tokens.read().await.get(&provider).cloned()) })
            }),
            Box::new(move |provider: &str, token: TokenInfo| {
                let tokens = save_tokens.clone();
                let provider = provider.to_string();
                Box::pin(async move {
                    tokens.write().await.insert(provider, token);
                    Ok(())
                })
            }),
            Box::new(move |provider: &str| {
                let tokens = remove_tokens.clone();
                let provider = provider.to_string();
                Box::pin(async move {
                    tokens.write().await.remove(&provider);
                    Ok(())
                })
            }),
        );

        // Test load on empty
        assert!(storage.load("anthropic").await.unwrap().is_none());

        // Test save and load
        let token = TokenInfo::new("access".into(), "refresh".into(), 3600);
        storage.save("anthropic", &token).await.unwrap();

        let loaded = storage.load("anthropic").await.unwrap().unwrap();
        assert_eq!(loaded.access_token, "access");

        // Test remove
        storage.remove("anthropic").await.unwrap();
        assert!(storage.load("anthropic").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_callback_storage_name() {
        let storage = CallbackStorage::new(
            Box::new(|_| Box::pin(async { Ok(None) })),
            Box::new(|_, _| Box::pin(async { Ok(()) })),
            Box::new(|_| Box::pin(async { Ok(()) })),
        );
        assert_eq!(storage.name(), "callback");
    }

    #[test]
    fn test_env_source_json() {
        // Set up environment variable
        std::env::set_var(
            "TEST_TOKEN_JSON",
            r#"{"type":"oauth","access_token":"access","refresh_token":"refresh","expires_at":1234567890}"#,
        );

        let source = EnvSource::json("TEST_TOKEN_JSON");
        let token = source.load().unwrap().unwrap();
        assert_eq!(token.access_token, "access");
        assert_eq!(token.refresh_token, "refresh");
        assert_eq!(token.expires_at, 1234567890);

        std::env::remove_var("TEST_TOKEN_JSON");
    }

    #[test]
    fn test_env_source_parts() {
        std::env::set_var("TEST_ACCESS", "access_value");
        std::env::set_var("TEST_REFRESH", "refresh_value");
        std::env::set_var("TEST_EXPIRES", "1234567890");

        let source = EnvSource::parts("TEST_ACCESS", "TEST_REFRESH", "TEST_EXPIRES");
        let token = source.load().unwrap().unwrap();
        assert_eq!(token.access_token, "access_value");
        assert_eq!(token.refresh_token, "refresh_value");
        assert_eq!(token.expires_at, 1234567890);

        std::env::remove_var("TEST_ACCESS");
        std::env::remove_var("TEST_REFRESH");
        std::env::remove_var("TEST_EXPIRES");
    }

    #[test]
    fn test_env_source_missing_var() {
        let source = EnvSource::json("NONEXISTENT_VAR");
        let token = source.load().unwrap();
        assert!(token.is_none());
    }

    #[test]
    fn test_env_source_parts_incomplete() {
        std::env::set_var("TEST_PARTIAL_ACCESS", "access");
        // Missing refresh and expires

        let source = EnvSource::parts("TEST_PARTIAL_ACCESS", "TEST_PARTIAL_REFRESH", "TEST_PARTIAL_EXPIRES");
        let token = source.load().unwrap();
        assert!(token.is_none());

        std::env::remove_var("TEST_PARTIAL_ACCESS");
    }

    #[test]
    fn test_file_source() {
        let source = FileSource::new("/path/to/tokens.json");
        assert_eq!(source.path(), Path::new("/path/to/tokens.json"));
    }
}
