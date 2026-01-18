//! Token storage backends with lazy-loading callback support.
//!
//! This module provides the [`TokenStorage`] trait and several implementations
//! for storing OAuth tokens. The design uses callbacks for maximum flexibility,
//! allowing tokens to be loaded lazily (not in memory until actually needed).

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{debug, instrument, warn};

use super::error::{Error, Result};
use super::models::TokenInfo;

/// Trait for token storage backends.
///
/// Implementations of this trait provide lazy-loading token storage.
/// Tokens are not loaded into memory until the `load` method is called,
/// providing security benefits by minimizing token exposure time.
///
/// # Example
///
/// ```rust,no_run
/// use claude_gate::{TokenStorage, TokenInfo, Error};
/// use async_trait::async_trait;
///
/// struct MyStorage {
///     // Your storage backend
/// }
///
/// #[async_trait]
/// impl TokenStorage for MyStorage {
///     async fn load(&self) -> Result<Option<TokenInfo>, Error> {
///         // Load token from your backend (only called when needed)
///         Ok(None)
///     }
///
///     async fn save(&self, token: &TokenInfo) -> Result<(), Error> {
///         // Save token to your backend
///         Ok(())
///     }
///
///     async fn remove(&self) -> Result<(), Error> {
///         // Remove token from your backend
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait TokenStorage: Send + Sync {
    /// Load the token from storage.
    ///
    /// Returns `Ok(None)` if no token is stored.
    /// This method is called lazily, only when a token is actually needed.
    async fn load(&self) -> Result<Option<TokenInfo>>;

    /// Save a token to storage.
    async fn save(&self, token: &TokenInfo) -> Result<()>;

    /// Remove the token from storage.
    async fn remove(&self) -> Result<()>;

    /// Check if a token exists without loading it fully.
    ///
    /// Default implementation loads the token, but backends can override
    /// for more efficient existence checks.
    async fn exists(&self) -> Result<bool> {
        Ok(self.load().await?.is_some())
    }

    /// Get the storage backend name (for logging/debugging).
    fn name(&self) -> &str {
        "unknown"
    }
}

/// Callback-based token storage for maximum flexibility.
///
/// This allows users to provide their own load/save/remove functions,
/// enabling integration with any storage backend.
///
/// # Example
///
/// ```rust,no_run
/// use claude_gate::storage::CallbackStorage;
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
///
/// let token_holder = Arc::new(Mutex::new(None));
/// let holder_load = token_holder.clone();
/// let holder_save = token_holder.clone();
/// let holder_remove = token_holder.clone();
///
/// let storage = CallbackStorage::new(
///     move || {
///         let holder = holder_load.clone();
///         Box::pin(async move {
///             Ok(holder.lock().await.clone())
///         })
///     },
///     move |token| {
///         let holder = holder_save.clone();
///         Box::pin(async move {
///             *holder.lock().await = Some(token);
///             Ok(())
///         })
///     },
///     move || {
///         let holder = holder_remove.clone();
///         Box::pin(async move {
///             *holder.lock().await = None;
///             Ok(())
///         })
///     },
/// );
/// ```
pub struct CallbackStorage<L, S, R>
where
    L: Fn() -> Pin<Box<dyn Future<Output = Result<Option<TokenInfo>>> + Send>> + Send + Sync,
    S: Fn(TokenInfo) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync,
    R: Fn() -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync,
{
    load_fn: L,
    save_fn: S,
    remove_fn: R,
}

impl<L, S, R> CallbackStorage<L, S, R>
where
    L: Fn() -> Pin<Box<dyn Future<Output = Result<Option<TokenInfo>>> + Send>> + Send + Sync,
    S: Fn(TokenInfo) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync,
    R: Fn() -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync,
{
    /// Create a new callback-based storage.
    pub fn new(load_fn: L, save_fn: S, remove_fn: R) -> Self {
        Self {
            load_fn,
            save_fn,
            remove_fn,
        }
    }
}

#[async_trait]
impl<L, S, R> TokenStorage for CallbackStorage<L, S, R>
where
    L: Fn() -> Pin<Box<dyn Future<Output = Result<Option<TokenInfo>>> + Send>> + Send + Sync,
    S: Fn(TokenInfo) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync,
    R: Fn() -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync,
{
    async fn load(&self) -> Result<Option<TokenInfo>> {
        (self.load_fn)().await
    }

    async fn save(&self, token: &TokenInfo) -> Result<()> {
        (self.save_fn)(token.clone()).await
    }

    async fn remove(&self) -> Result<()> {
        (self.remove_fn)().await
    }

    fn name(&self) -> &str {
        "callback"
    }
}

/// Pre-wired callback functions for common storage patterns.
///
/// This module provides two ways to create storage callbacks:
///
/// 1. **Source pattern** (recommended): Create a source, then derive callbacks from it.
///    This ensures the loader and saver always use the same schema.
///
/// 2. **Direct functions**: Create individual callbacks (legacy, still supported).
///
/// # Source Pattern (Recommended)
///
/// ```rust,no_run
/// use claude_gate::storage::{CallbackStorage, callbacks::FileSource};
///
/// // Create a source - this defines the storage location and schema
/// let source = FileSource::new("~/.myapp/tokens.json");
///
/// // Get the saver first (it defines the schema)
/// let saver = source.saver();
///
/// // Get the loader from the saver (guaranteed compatible)
/// let loader = saver.loader();
///
/// // Get the remover from the source
/// let remover = source.remover();
///
/// // Use with CallbackStorage
/// let storage = CallbackStorage::new(loader, saver.into_fn(), remover);
/// ```
///
/// # Direct Functions (Legacy)
///
/// ```rust,no_run
/// use claude_gate::storage::{CallbackStorage, callbacks};
/// use std::path::PathBuf;
///
/// let path = PathBuf::from("/home/user/.myapp/tokens.json");
///
/// let storage = CallbackStorage::new(
///     callbacks::file_load(path.clone()),
///     callbacks::file_save(path.clone()),
///     callbacks::file_remove(path),
/// );
/// ```
pub mod callbacks {
    use super::*;
    use std::path::PathBuf;

    /// Type alias for the load callback function signature.
    pub type LoadFn = Box<
        dyn Fn() -> Pin<Box<dyn Future<Output = Result<Option<TokenInfo>>> + Send>> + Send + Sync,
    >;

    /// Type alias for the save callback function signature.
    pub type SaveFn = Box<
        dyn Fn(TokenInfo) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync,
    >;

    /// Type alias for the remove callback function signature.
    pub type RemoveFn =
        Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;

    /// JSON schema used by file callbacks.
    ///
    /// The token is stored under the "anthropic" key for compatibility
    /// with the Go implementation.
    #[derive(serde::Serialize, serde::Deserialize)]
    struct TokenFile {
        anthropic: TokenInfo,
    }

    // ==================== Source Pattern ====================

    /// File-based token source.
    ///
    /// This is the recommended way to create file-based callbacks.
    /// It ensures the loader and saver use the same path and JSON schema.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use claude_gate::storage::{CallbackStorage, callbacks::FileSource};
    ///
    /// let source = FileSource::new("~/.myapp/tokens.json");
    /// let saver = source.saver();
    /// let loader = saver.loader();
    /// let storage = CallbackStorage::new(loader, saver.into_fn(), source.remover());
    /// ```
    #[derive(Clone)]
    pub struct FileSource {
        path: PathBuf,
    }

    impl FileSource {
        /// Create a new file source.
        ///
        /// The path can include `~` which will be expanded to the home directory.
        #[must_use]
        pub fn new(path: impl Into<PathBuf>) -> Self {
            Self { path: path.into() }
        }

        /// Get the saver for this source.
        ///
        /// The saver is the "source of truth" - it defines the JSON schema.
        /// Use `saver.loader()` to get a compatible loader.
        #[must_use]
        pub fn saver(&self) -> FileSaver {
            FileSaver {
                path: self.path.clone(),
            }
        }

        /// Get the remover for this source.
        #[must_use]
        pub fn remover(&self) -> RemoveFn {
            file_remove(self.path.clone())
        }

        /// Get all callbacks at once (convenience method).
        ///
        /// Returns (loader, saver, remover) tuple.
        #[must_use]
        pub fn callbacks(self) -> (LoadFn, SaveFn, RemoveFn) {
            let saver = self.saver();
            let loader = saver.loader();
            let remover = self.remover();
            (loader, saver.into_fn(), remover)
        }

        /// Create a CallbackStorage directly from this source.
        #[must_use]
        pub fn into_storage(self) -> CallbackStorage<LoadFn, SaveFn, RemoveFn> {
            let (loader, saver, remover) = self.callbacks();
            CallbackStorage::new(loader, saver, remover)
        }
    }

    /// File saver that can produce a compatible loader.
    ///
    /// This ensures the loader uses the same JSON schema as the saver.
    #[derive(Clone)]
    pub struct FileSaver {
        path: PathBuf,
    }

    impl FileSaver {
        /// Get a loader that reads the same format this saver writes.
        ///
        /// This guarantees schema compatibility between load and save operations.
        #[must_use]
        pub fn loader(&self) -> LoadFn {
            file_load(self.path.clone())
        }

        /// Convert this saver into a callback function.
        #[must_use]
        pub fn into_fn(self) -> SaveFn {
            file_save(self.path)
        }

        /// Get the path this saver writes to.
        #[must_use]
        pub fn path(&self) -> &Path {
            &self.path
        }
    }

    /// Environment variable token source.
    ///
    /// Creates a read-only source that loads tokens from environment variables.
    /// Since env vars are typically read-only, use `callbacks()` to get all
    /// callbacks at once.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use claude_gate::storage::{CallbackStorage, callbacks::EnvSource};
    ///
    /// // From separate env vars
    /// let source = EnvSource::from_parts(
    ///     "CLAUDE_ACCESS_TOKEN",
    ///     "CLAUDE_REFRESH_TOKEN",
    ///     "CLAUDE_TOKEN_EXPIRES",
    /// );
    ///
    /// // Use callbacks() to get all three at once
    /// let (loader, saver, remover) = source.callbacks();
    /// let storage = CallbackStorage::new(loader, saver, remover);
    /// ```
    pub struct EnvSource {
        loader: LoadFn,
        readonly: bool,
    }

    impl EnvSource {
        /// Create a source that reads from a single JSON-encoded env var.
        ///
        /// The env var should contain the full token as JSON.
        #[must_use]
        pub fn new(var_name: impl Into<String>) -> Self {
            Self {
                loader: env_load(var_name),
                readonly: true,
            }
        }

        /// Create a source that reads from separate env vars.
        #[must_use]
        pub fn from_parts(
            access_var: impl Into<String>,
            refresh_var: impl Into<String>,
            expires_var: impl Into<String>,
        ) -> Self {
            Self {
                loader: env_load_parts(access_var, refresh_var, expires_var),
                readonly: true,
            }
        }

        /// Set whether saves should silently succeed (noop) or return an error.
        ///
        /// Default is `true` (read-only, saves return error).
        #[must_use]
        pub fn readonly(mut self, readonly: bool) -> Self {
            self.readonly = readonly;
            self
        }

        /// Get the saver for this source.
        ///
        /// By default, returns a saver that errors on write (env vars are read-only).
        /// Use `.readonly(false)` to get a no-op saver instead.
        #[must_use]
        pub fn saver(&self) -> EnvSaver {
            EnvSaver {
                readonly: self.readonly,
            }
        }

        /// Get the remover for this source.
        ///
        /// By default, returns a remover that errors (env vars are read-only).
        #[must_use]
        pub fn remover(&self) -> RemoveFn {
            if self.readonly {
                readonly_remove("Environment variable storage is read-only")
            } else {
                noop_remove()
            }
        }

        /// Get all callbacks at once.
        #[must_use]
        pub fn callbacks(self) -> (LoadFn, SaveFn, RemoveFn) {
            let remover = self.remover();
            let saver_fn = if self.readonly {
                readonly_save("Environment variable storage is read-only")
            } else {
                noop_save()
            };
            (self.loader, saver_fn, remover)
        }
    }

    /// Environment saver (placeholder for read-only sources).
    ///
    /// Since environment variables are typically read-only, this saver
    /// doesn't actually write anywhere. Use [`EnvSource::callbacks()`]
    /// to get all callbacks at once.
    pub struct EnvSaver {
        readonly: bool,
    }

    impl EnvSaver {
        /// Convert this saver into a callback function.
        ///
        /// Returns either a no-op or an error-returning callback
        /// depending on the `readonly` setting.
        #[must_use]
        pub fn into_fn(self) -> SaveFn {
            if self.readonly {
                readonly_save("Environment variable storage is read-only")
            } else {
                noop_save()
            }
        }
    }

    // ==================== Direct Functions (Legacy) ====================

    /// Create a load callback that reads tokens from a JSON file.
    ///
    /// The JSON schema matches [`FileTokenStorage`] for compatibility:
    /// ```json
    /// { "anthropic": { "type": "oauth", "access_token": "...", ... } }
    /// ```
    ///
    /// **Prefer using [`FileSource`] for new code** to ensure schema compatibility.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JSON file (will be created if it doesn't exist on save)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use claude_gate::storage::callbacks;
    /// use std::path::PathBuf;
    ///
    /// let load = callbacks::file_load(PathBuf::from("~/.myapp/auth.json"));
    /// ```
    #[must_use]
    pub fn file_load(path: PathBuf) -> LoadFn {
        Box::new(move || {
            let path = path.clone();
            Box::pin(async move {
                let path = expand_home(&path)?;

                if !path.exists() {
                    return Ok(None);
                }

                let contents = tokio::fs::read_to_string(&path).await?;
                if contents.trim().is_empty() {
                    return Ok(None);
                }

                let file: TokenFile = serde_json::from_str(&contents)?;
                Ok(Some(file.anthropic))
            })
        })
    }

    /// Create a save callback that writes tokens to a JSON file.
    ///
    /// The file is created with secure permissions (0600 on Unix).
    /// Parent directories are created if they don't exist.
    ///
    /// **Prefer using [`FileSource`] for new code** to ensure schema compatibility.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JSON file
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use claude_gate::storage::callbacks;
    /// use std::path::PathBuf;
    ///
    /// let save = callbacks::file_save(PathBuf::from("~/.myapp/auth.json"));
    /// ```
    #[must_use]
    pub fn file_save(path: PathBuf) -> SaveFn {
        Box::new(move |token| {
            let path = path.clone();
            Box::pin(async move {
                let path = expand_home(&path)?;

                // Ensure parent directory exists
                if let Some(parent) = path.parent() {
                    if !parent.exists() {
                        tokio::fs::create_dir_all(parent).await?;
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let perms = std::fs::Permissions::from_mode(0o700);
                            tokio::fs::set_permissions(parent, perms).await?;
                        }
                    }
                }

                let file = TokenFile { anthropic: token };
                let contents = serde_json::to_string_pretty(&file)?;
                tokio::fs::write(&path, &contents).await?;

                // Set secure permissions
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = std::fs::Permissions::from_mode(0o600);
                    tokio::fs::set_permissions(&path, perms).await?;
                }

                Ok(())
            })
        })
    }

    /// Create a remove callback that deletes a token file.
    ///
    /// **Prefer using [`FileSource`] for new code.**
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JSON file to remove
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use claude_gate::storage::callbacks;
    /// use std::path::PathBuf;
    ///
    /// let remove = callbacks::file_remove(PathBuf::from("~/.myapp/auth.json"));
    /// ```
    #[must_use]
    pub fn file_remove(path: PathBuf) -> RemoveFn {
        Box::new(move || {
            let path = path.clone();
            Box::pin(async move {
                let path = expand_home(&path)?;
                if path.exists() {
                    tokio::fs::remove_file(&path).await?;
                }
                Ok(())
            })
        })
    }

    /// Create all three file callbacks at once.
    ///
    /// **Prefer using [`FileSource::callbacks()`] for new code.**
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JSON file
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use claude_gate::storage::{CallbackStorage, callbacks};
    /// use std::path::PathBuf;
    ///
    /// let (load, save, remove) = callbacks::file_callbacks(
    ///     PathBuf::from("~/.myapp/auth.json")
    /// );
    /// let storage = CallbackStorage::new(load, save, remove);
    /// ```
    #[must_use]
    pub fn file_callbacks(path: PathBuf) -> (LoadFn, SaveFn, RemoveFn) {
        (
            file_load(path.clone()),
            file_save(path.clone()),
            file_remove(path),
        )
    }

    /// Create a load callback from an environment variable.
    ///
    /// Reads a JSON-encoded token from the specified environment variable.
    /// The JSON should contain the full token structure.
    ///
    /// **Prefer using [`EnvSource`] for new code.**
    ///
    /// # Arguments
    ///
    /// * `var_name` - Name of the environment variable
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use claude_gate::storage::callbacks;
    ///
    /// // Expects CLAUDE_TOKEN='{"type":"oauth","access_token":"...","refresh_token":"...","expires_at":123}'
    /// let load = callbacks::env_load("CLAUDE_TOKEN");
    /// ```
    #[must_use]
    pub fn env_load(var_name: impl Into<String>) -> LoadFn {
        let var_name = var_name.into();
        Box::new(move || {
            let var_name = var_name.clone();
            Box::pin(async move {
                match std::env::var(&var_name) {
                    Ok(json) if !json.is_empty() => {
                        let token: TokenInfo = serde_json::from_str(&json)?;
                        Ok(Some(token))
                    }
                    _ => Ok(None),
                }
            })
        })
    }

    /// Create a load callback from separate environment variables.
    ///
    /// Reads token components from individual environment variables.
    ///
    /// **Prefer using [`EnvSource::from_parts()`] for new code.**
    ///
    /// # Arguments
    ///
    /// * `access_var` - Env var for access token
    /// * `refresh_var` - Env var for refresh token
    /// * `expires_var` - Env var for expiration timestamp (Unix seconds)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use claude_gate::storage::callbacks;
    ///
    /// let load = callbacks::env_load_parts(
    ///     "CLAUDE_ACCESS_TOKEN",
    ///     "CLAUDE_REFRESH_TOKEN",
    ///     "CLAUDE_TOKEN_EXPIRES",
    /// );
    /// ```
    #[must_use]
    pub fn env_load_parts(
        access_var: impl Into<String>,
        refresh_var: impl Into<String>,
        expires_var: impl Into<String>,
    ) -> LoadFn {
        let access_var = access_var.into();
        let refresh_var = refresh_var.into();
        let expires_var = expires_var.into();

        Box::new(move || {
            let access_var = access_var.clone();
            let refresh_var = refresh_var.clone();
            let expires_var = expires_var.clone();

            Box::pin(async move {
                let access = std::env::var(&access_var).ok();
                let refresh = std::env::var(&refresh_var).ok();
                let expires = std::env::var(&expires_var)
                    .ok()
                    .and_then(|s| s.parse::<i64>().ok());

                match (access, refresh, expires) {
                    (Some(access_token), Some(refresh_token), Some(expires_at)) => {
                        Ok(Some(TokenInfo {
                            token_type: "oauth".to_string(),
                            access_token,
                            refresh_token,
                            expires_at,
                        }))
                    }
                    _ => Ok(None),
                }
            })
        })
    }

    /// Create a no-op save callback.
    ///
    /// Useful for read-only storage backends (like environment variables).
    /// Always returns `Ok(())`.
    #[must_use]
    pub fn noop_save() -> SaveFn {
        Box::new(|_| Box::pin(async { Ok(()) }))
    }

    /// Create a no-op remove callback.
    ///
    /// Useful for read-only storage backends.
    /// Always returns `Ok(())`.
    #[must_use]
    pub fn noop_remove() -> RemoveFn {
        Box::new(|| Box::pin(async { Ok(()) }))
    }

    /// Create a save callback that returns an error (for read-only storage).
    ///
    /// # Arguments
    ///
    /// * `message` - Error message to return
    #[must_use]
    pub fn readonly_save(message: impl Into<String>) -> SaveFn {
        let message = message.into();
        Box::new(move |_| {
            let message = message.clone();
            Box::pin(async move { Err(Error::storage(message)) })
        })
    }

    /// Create a remove callback that returns an error (for read-only storage).
    ///
    /// # Arguments
    ///
    /// * `message` - Error message to return
    #[must_use]
    pub fn readonly_remove(message: impl Into<String>) -> RemoveFn {
        let message = message.into();
        Box::new(move || {
            let message = message.clone();
            Box::pin(async move { Err(Error::storage(message)) })
        })
    }

    /// Expand `~` to home directory in a path.
    fn expand_home(path: &Path) -> Result<PathBuf> {
        if path.starts_with("~") {
            let home = dirs::home_dir()
                .ok_or_else(|| Error::config("Cannot determine home directory"))?;
            // Strip "~" and any leading "/" to avoid treating as absolute path
            let suffix = path
                .strip_prefix("~")
                .unwrap_or(path)
                .strip_prefix("/")
                .unwrap_or_else(|_| path.strip_prefix("~").unwrap_or(path));
            Ok(home.join(suffix))
        } else {
            Ok(path.to_path_buf())
        }
    }
}

/// File-based token storage.
///
/// Stores tokens in a JSON file with proper permissions (0600).
/// The file is only read when `load` is called, providing lazy loading.
pub struct FileTokenStorage {
    path: PathBuf,
}

impl FileTokenStorage {
    /// Create a new file-based token storage.
    ///
    /// The path can include `~` which will be expanded to the home directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the home directory cannot be determined when `~` is used.
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let expanded = if path.starts_with("~") {
            let home = dirs::home_dir().ok_or_else(|| Error::config("Cannot determine home directory"))?;
            home.join(path.strip_prefix("~").unwrap())
        } else {
            path.to_path_buf()
        };

        Ok(Self { path: expanded })
    }

    /// Get the default storage path (~/.config/cld/auth.json).
    ///
    /// # Errors
    ///
    /// Returns an error if the home directory cannot be determined.
    pub fn default_path() -> Result<Self> {
        Self::new("~/.config/cld/auth.json")
    }

    /// Get the path to the storage file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Ensure the parent directory exists.
    async fn ensure_parent_dir(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await?;
                // Set directory permissions to 0700
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = std::fs::Permissions::from_mode(0o700);
                    tokio::fs::set_permissions(parent, perms).await?;
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl TokenStorage for FileTokenStorage {
    #[instrument(skip(self), fields(path = %self.path.display()))]
    async fn load(&self) -> Result<Option<TokenInfo>> {
        if !self.path.exists() {
            debug!("Token file does not exist");
            return Ok(None);
        }

        let contents = tokio::fs::read_to_string(&self.path).await?;
        if contents.trim().is_empty() {
            debug!("Token file is empty");
            return Ok(None);
        }

        // The file format mirrors the Go version: {"provider": TokenInfo}
        // We use "anthropic" as the default provider
        let data: serde_json::Value = serde_json::from_str(&contents)?;
        let token = data
            .get("anthropic")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        debug!(token_exists = token.is_some(), "Loaded token from file");
        Ok(token)
    }

    #[instrument(skip(self, token), fields(path = %self.path.display()))]
    async fn save(&self, token: &TokenInfo) -> Result<()> {
        self.ensure_parent_dir().await?;

        // Match the Go format
        let data = serde_json::json!({
            "anthropic": token
        });

        let contents = serde_json::to_string_pretty(&data)?;
        tokio::fs::write(&self.path, &contents).await?;

        // Set file permissions to 0600 (user read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            tokio::fs::set_permissions(&self.path, perms).await?;
        }

        debug!("Saved token to file");
        Ok(())
    }

    #[instrument(skip(self), fields(path = %self.path.display()))]
    async fn remove(&self) -> Result<()> {
        if self.path.exists() {
            tokio::fs::remove_file(&self.path).await?;
            debug!("Removed token file");
        }
        Ok(())
    }

    async fn exists(&self) -> Result<bool> {
        Ok(self.path.exists())
    }

    fn name(&self) -> &str {
        "file"
    }
}

/// Keyring-based token storage using the system's secret service.
///
/// On Linux, this uses Secret Service API (GNOME Keyring, KWallet).
/// On macOS, this uses Keychain Services.
/// On Windows, this uses Credential Manager.
#[cfg(feature = "keyring")]
pub struct KeyringTokenStorage {
    service: String,
    user: String,
}

#[cfg(feature = "keyring")]
impl KeyringTokenStorage {
    /// Create a new keyring storage with the default service name.
    #[must_use]
    pub fn new() -> Self {
        Self::with_service("claude-gate")
    }

    /// Create a new keyring storage with a custom service name.
    #[must_use]
    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            user: "anthropic".to_string(),
        }
    }

    /// Check if the keyring is available on this system.
    pub fn is_available() -> bool {
        // Try to create an entry and check if it works
        keyring::Entry::new("claude-gate-test", "test").is_ok()
    }

    fn entry(&self) -> Result<keyring::Entry> {
        keyring::Entry::new(&self.service, &self.user).map_err(Error::from)
    }
}

#[cfg(feature = "keyring")]
impl Default for KeyringTokenStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "keyring")]
#[async_trait]
impl TokenStorage for KeyringTokenStorage {
    #[instrument(skip(self), fields(service = %self.service))]
    async fn load(&self) -> Result<Option<TokenInfo>> {
        let entry = self.entry()?;

        match entry.get_password() {
            Ok(json) => {
                let token: TokenInfo = serde_json::from_str(&json)?;
                debug!("Loaded token from keyring");
                Ok(Some(token))
            }
            Err(keyring::Error::NoEntry) => {
                debug!("No token in keyring");
                Ok(None)
            }
            Err(e) => Err(Error::from(e)),
        }
    }

    #[instrument(skip(self, token), fields(service = %self.service))]
    async fn save(&self, token: &TokenInfo) -> Result<()> {
        let entry = self.entry()?;
        let json = serde_json::to_string(token)?;
        entry.set_password(&json)?;
        debug!("Saved token to keyring");
        Ok(())
    }

    #[instrument(skip(self), fields(service = %self.service))]
    async fn remove(&self) -> Result<()> {
        let entry = self.entry()?;
        match entry.delete_password() {
            Ok(()) => {
                debug!("Removed token from keyring");
                Ok(())
            }
            Err(keyring::Error::NoEntry) => {
                debug!("No token to remove from keyring");
                Ok(())
            }
            Err(e) => Err(Error::from(e)),
        }
    }

    fn name(&self) -> &str {
        "keyring"
    }
}

/// In-memory token storage for testing or ephemeral use.
///
/// Tokens stored here are lost when the storage is dropped.
pub struct MemoryTokenStorage {
    token: Arc<RwLock<Option<TokenInfo>>>,
}

impl MemoryTokenStorage {
    /// Create a new in-memory storage.
    #[must_use]
    pub fn new() -> Self {
        Self {
            token: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new in-memory storage with an initial token.
    #[must_use]
    pub fn with_token(token: TokenInfo) -> Self {
        Self {
            token: Arc::new(RwLock::new(Some(token))),
        }
    }
}

impl Default for MemoryTokenStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MemoryTokenStorage {
    fn clone(&self) -> Self {
        Self {
            token: self.token.clone(),
        }
    }
}

#[async_trait]
impl TokenStorage for MemoryTokenStorage {
    async fn load(&self) -> Result<Option<TokenInfo>> {
        Ok(self.token.read().await.clone())
    }

    async fn save(&self, token: &TokenInfo) -> Result<()> {
        *self.token.write().await = Some(token.clone());
        Ok(())
    }

    async fn remove(&self) -> Result<()> {
        *self.token.write().await = None;
        Ok(())
    }

    fn name(&self) -> &str {
        "memory"
    }
}

/// Auto-selecting storage that tries keyring first, then falls back to file.
///
/// This provides the best of both worlds: secure keyring storage when available,
/// with file-based fallback for environments without a keyring (like servers).
pub struct AutoStorage {
    inner: Box<dyn TokenStorage>,
}

impl AutoStorage {
    /// Create a new auto-selecting storage.
    ///
    /// Tries keyring first (if feature enabled and available), then file.
    pub fn new() -> Result<Self> {
        #[cfg(feature = "keyring")]
        {
            if KeyringTokenStorage::is_available() {
                debug!("Using keyring storage");
                return Ok(Self {
                    inner: Box::new(KeyringTokenStorage::new()),
                });
            }
            warn!("Keyring not available, falling back to file storage");
        }

        debug!("Using file storage");
        Ok(Self {
            inner: Box::new(FileTokenStorage::default_path()?),
        })
    }

    /// Get the name of the active storage backend.
    #[must_use]
    pub fn backend_name(&self) -> &str {
        self.inner.name()
    }
}

#[async_trait]
impl TokenStorage for AutoStorage {
    async fn load(&self) -> Result<Option<TokenInfo>> {
        self.inner.load().await
    }

    async fn save(&self, token: &TokenInfo) -> Result<()> {
        self.inner.save(token).await
    }

    async fn remove(&self) -> Result<()> {
        self.inner.remove().await
    }

    async fn exists(&self) -> Result<bool> {
        self.inner.exists().await
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}

// Add the dirs crate dependency for home directory expansion
// This is handled in Cargo.toml

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_storage() {
        let storage = MemoryTokenStorage::new();
        assert!(storage.load().await.unwrap().is_none());

        let token = TokenInfo::new("access".into(), "refresh".into(), 3600);
        storage.save(&token).await.unwrap();

        let loaded = storage.load().await.unwrap().unwrap();
        assert_eq!(loaded.access_token, "access");

        storage.remove().await.unwrap();
        assert!(storage.load().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_file_storage() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test_auth.json");
        let storage = FileTokenStorage::new(&path).unwrap();

        assert!(storage.load().await.unwrap().is_none());

        let token = TokenInfo::new("access".into(), "refresh".into(), 3600);
        storage.save(&token).await.unwrap();

        let loaded = storage.load().await.unwrap().unwrap();
        assert_eq!(loaded.access_token, "access");

        storage.remove().await.unwrap();
        assert!(storage.load().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_file_callbacks() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("callback_test.json");

        // Create storage using file callbacks
        let (load, save, remove) = callbacks::file_callbacks(path.clone());
        let storage = CallbackStorage::new(load, save, remove);

        // Test load on non-existent file
        assert!(storage.load().await.unwrap().is_none());

        // Test save
        let token = TokenInfo::new("cb_access".into(), "cb_refresh".into(), 7200);
        storage.save(&token).await.unwrap();

        // Test load after save
        let loaded = storage.load().await.unwrap().unwrap();
        assert_eq!(loaded.access_token, "cb_access");
        assert_eq!(loaded.refresh_token, "cb_refresh");

        // Verify file content is valid JSON with correct schema
        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.get("anthropic").is_some());

        // Test remove
        storage.remove().await.unwrap();
        assert!(storage.load().await.unwrap().is_none());
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn test_file_callbacks_compatibility_with_file_storage() {
        // Verify that file callbacks use the same JSON schema as FileTokenStorage
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("compat_test.json");

        // Save with FileTokenStorage
        let file_storage = FileTokenStorage::new(&path).unwrap();
        let token = TokenInfo::new("compat_access".into(), "compat_refresh".into(), 3600);
        file_storage.save(&token).await.unwrap();

        // Load with file callbacks
        let load_fn = callbacks::file_load(path.clone());
        let loaded = load_fn().await.unwrap().unwrap();
        assert_eq!(loaded.access_token, "compat_access");

        // Clean up and reverse test: save with callbacks, load with FileTokenStorage
        std::fs::remove_file(&path).unwrap();

        let save_fn = callbacks::file_save(path.clone());
        let token2 = TokenInfo::new("compat2_access".into(), "compat2_refresh".into(), 3600);
        save_fn(token2).await.unwrap();

        let loaded2 = file_storage.load().await.unwrap().unwrap();
        assert_eq!(loaded2.access_token, "compat2_access");
    }

    #[tokio::test]
    async fn test_env_callbacks() {
        // Set test env vars
        std::env::set_var("TEST_CLAUDE_ACCESS", "env_access");
        std::env::set_var("TEST_CLAUDE_REFRESH", "env_refresh");
        std::env::set_var("TEST_CLAUDE_EXPIRES", "9999999999");

        let load = callbacks::env_load_parts(
            "TEST_CLAUDE_ACCESS",
            "TEST_CLAUDE_REFRESH",
            "TEST_CLAUDE_EXPIRES",
        );

        let token = load().await.unwrap().unwrap();
        assert_eq!(token.access_token, "env_access");
        assert_eq!(token.refresh_token, "env_refresh");
        assert_eq!(token.expires_at, 9999999999);

        // Clean up
        std::env::remove_var("TEST_CLAUDE_ACCESS");
        std::env::remove_var("TEST_CLAUDE_REFRESH");
        std::env::remove_var("TEST_CLAUDE_EXPIRES");
    }

    #[tokio::test]
    async fn test_noop_callbacks() {
        let save = callbacks::noop_save();
        let remove = callbacks::noop_remove();

        // Should succeed without doing anything
        let token = TokenInfo::new("x".into(), "y".into(), 100);
        save(token).await.unwrap();
        remove().await.unwrap();
    }

    #[tokio::test]
    async fn test_readonly_callbacks() {
        let save = callbacks::readonly_save("Cannot save to this storage");
        let remove = callbacks::readonly_remove("Cannot remove from this storage");

        let token = TokenInfo::new("x".into(), "y".into(), 100);
        assert!(save(token).await.is_err());
        assert!(remove().await.is_err());
    }

    #[tokio::test]
    async fn test_file_source_pattern() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("source_test.json");

        // Create a FileSource
        let source = callbacks::FileSource::new(path.clone());

        // Get saver first (it defines the schema)
        let saver = source.saver();

        // Get loader from saver (guaranteed compatible)
        let loader = saver.loader();

        // Get remover from source
        let remover = source.remover();

        // Create storage
        let storage = CallbackStorage::new(loader, saver.into_fn(), remover);

        // Test the full flow
        assert!(storage.load().await.unwrap().is_none());

        let token = TokenInfo::new("source_access".into(), "source_refresh".into(), 3600);
        storage.save(&token).await.unwrap();

        let loaded = storage.load().await.unwrap().unwrap();
        assert_eq!(loaded.access_token, "source_access");

        storage.remove().await.unwrap();
        assert!(storage.load().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_file_source_into_storage() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("into_storage_test.json");

        // Use the convenience method
        let storage = callbacks::FileSource::new(path).into_storage();

        let token = TokenInfo::new("into_access".into(), "into_refresh".into(), 3600);
        storage.save(&token).await.unwrap();

        let loaded = storage.load().await.unwrap().unwrap();
        assert_eq!(loaded.access_token, "into_access");
    }

    #[tokio::test]
    async fn test_file_source_callbacks_tuple() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("tuple_test.json");

        // Use the callbacks() method
        let (loader, saver, remover) = callbacks::FileSource::new(path).callbacks();
        let storage = CallbackStorage::new(loader, saver, remover);

        let token = TokenInfo::new("tuple_access".into(), "tuple_refresh".into(), 3600);
        storage.save(&token).await.unwrap();

        let loaded = storage.load().await.unwrap().unwrap();
        assert_eq!(loaded.access_token, "tuple_access");
    }

    #[tokio::test]
    async fn test_env_source() {
        // Set test env vars
        std::env::set_var("TEST_ENV_SRC_ACCESS", "env_src_access");
        std::env::set_var("TEST_ENV_SRC_REFRESH", "env_src_refresh");
        std::env::set_var("TEST_ENV_SRC_EXPIRES", "9999999999");

        let source = callbacks::EnvSource::from_parts(
            "TEST_ENV_SRC_ACCESS",
            "TEST_ENV_SRC_REFRESH",
            "TEST_ENV_SRC_EXPIRES",
        );

        let (loader, saver, remover) = source.callbacks();

        // Load should work
        let token = loader().await.unwrap().unwrap();
        assert_eq!(token.access_token, "env_src_access");

        // Save should error (read-only by default)
        let new_token = TokenInfo::new("x".into(), "y".into(), 100);
        assert!(saver(new_token).await.is_err());

        // Remove should error
        assert!(remover().await.is_err());

        // Clean up
        std::env::remove_var("TEST_ENV_SRC_ACCESS");
        std::env::remove_var("TEST_ENV_SRC_REFRESH");
        std::env::remove_var("TEST_ENV_SRC_EXPIRES");
    }

    #[tokio::test]
    async fn test_env_source_noop_mode() {
        let source = callbacks::EnvSource::new("NONEXISTENT_VAR")
            .readonly(false);  // Use noop instead of error

        let (loader, saver, remover) = source.callbacks();

        // Load returns None (var doesn't exist)
        assert!(loader().await.unwrap().is_none());

        // Save succeeds (noop)
        let token = TokenInfo::new("x".into(), "y".into(), 100);
        saver(token).await.unwrap();

        // Remove succeeds (noop)
        remover().await.unwrap();
    }
}
