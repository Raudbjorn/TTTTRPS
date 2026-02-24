//! Embedded Meilisearch Wrapper
//!
//! Provides a thin wrapper around `meilisearch_lib::Meilisearch` for shared
//! access across the TTRPG Assistant application. This module enables embedded
//! search capabilities without requiring an external Meilisearch process.
//!
//! # Architecture
//!
//! The `EmbeddedSearch` struct wraps `Meilisearch` in an `Arc` for safe
//! concurrent access from multiple Tauri command handlers. This replaces the
//! previous HTTP-based `SearchClient` and `SidecarManager` approach.
//!
//! # Example
//!
//! ```rust,ignore
//! use crate::core::search::EmbeddedSearch;
//! use std::path::PathBuf;
//!
//! let db_path = PathBuf::from("~/.local/share/ttrpg-assistant/meilisearch");
//! let search = EmbeddedSearch::new(db_path)?;
//!
//! // Access the inner Meilisearch for operations
//! let meili = search.inner();
//! let results = meili.search("ttrpg_rules", SearchQuery::new("fireball"))?;
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use crate::core::wilysearch::engine::Engine;
use crate::core::wilysearch::core::MeilisearchOptions;

use super::error::{Result, SearchError};

/// Default maximum index size: 10 GiB
const DEFAULT_MAX_INDEX_SIZE: usize = 10 * 1024 * 1024 * 1024;

/// Embedded Meilisearch search engine with RAG capabilities.
///
/// Wraps `Meilisearch` in an `Arc` for thread-safe shared access across
/// Tauri command handlers and async tasks.
#[derive(Clone)]
pub struct EmbeddedSearch {
    inner: Arc<Engine>,
}

impl EmbeddedSearch {
    /// Initialize embedded Meilisearch with the specified database path.
    ///
    /// # Arguments
    ///
    /// * `db_path` - Path to the Meilisearch database directory. Will be created
    ///   if it doesn't exist. Defaults to `~/.local/share/ttrpg-assistant/meilisearch/`
    ///   in typical usage.
    ///
    /// # Errors
    ///
    /// Returns `SearchError::ConfigError` if the configuration is invalid, or
    /// `SearchError::InitError` if the database fails to initialize.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let search = EmbeddedSearch::new(PathBuf::from("/path/to/db"))?;
    /// ```
    pub fn new(db_path: PathBuf) -> Result<Self> {
        Self::with_max_index_size(db_path, DEFAULT_MAX_INDEX_SIZE)
    }

    /// Initialize embedded Meilisearch with custom maximum index size.
    ///
    /// # Arguments
    ///
    /// * `db_path` - Path to the Meilisearch database directory
    /// * `max_index_size` - Maximum size in bytes for each index (default: 10 GiB)
    ///
    /// # Errors
    ///
    /// Returns `SearchError::ConfigError` if the configuration is invalid, or
    /// `SearchError::InitError` if the database fails to initialize.
    pub fn with_max_index_size(db_path: PathBuf, max_index_size: usize) -> Result<Self> {
        let options = MeilisearchOptions {
            db_path,
            max_index_size,
            ..MeilisearchOptions::default()
        };

        let inner =
            Engine::new(options).map_err(|e| SearchError::InitError(e.to_string()))?;

        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    /// Get a reference to the inner `Engine`.
    ///
    /// Use this for synchronous operations or when you need direct access
    /// to the search engine methods.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let engine = search.inner();
    /// let indexes = engine.list_indexes()?;
    /// ```
    #[inline]
    pub fn inner(&self) -> &Engine {
        &self.inner
    }

    /// Clone the `Arc<Engine>` for sharing across async tasks.
    ///
    /// Use this when spawning tasks that need owned access to the search engine,
    /// such as streaming response handlers.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let engine = search.clone_inner();
    /// tokio::spawn(async move {
    ///     let results = engine.search("index", query)?;
    ///     // ...
    /// });
    /// ```
    #[inline]
    pub fn clone_inner(&self) -> Arc<Engine> {
        Arc::clone(&self.inner)
    }

    /// Attempt to shutdown the embedded Meilisearch instance.
    ///
    /// This attempts to gracefully shutdown if this is the last reference to the
    /// inner `Engine`. If other references exist, this method succeeds
    /// without performing shutdown - cleanup will occur when all references are dropped.
    ///
    /// # Behavior
    ///
    /// - If this is the sole owner (Arc strong count == 1), takes ownership and calls shutdown
    /// - If other references exist, logs and returns Ok - shutdown deferred to drop
    /// - Data is always safe due to LMDB's transactional guarantees
    ///
    /// # Errors
    ///
    /// Returns an error only if shutdown fails when this is the sole owner.
    pub fn shutdown(self) -> Result<()> {
        match Arc::try_unwrap(self.inner) {
            Ok(engine) => {
                tracing::info!("EmbeddedSearch: sole owner, performing shutdown");
                drop(engine);
                Ok(())
            }
            Err(_arc) => {
                tracing::debug!(
                    "EmbeddedSearch: other references exist, shutdown deferred to drop"
                );
                Ok(())
            }
        }
    }
}

impl std::fmt::Debug for EmbeddedSearch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddedSearch")
            .field("inner", &"Arc<Engine>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_embedded_search_creation() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("meilisearch");

        let search = EmbeddedSearch::new(db_path);
        assert!(
            search.is_ok(),
            "EmbeddedSearch should initialize successfully"
        );

        let search = search.unwrap();
        // Verify inner() returns a reference
        let _engine = search.inner();

        // Verify clone_inner() returns an Arc
        let engine_arc = search.clone_inner();
        assert_eq!(Arc::strong_count(&engine_arc), 2);

        // Drop the extra Arc reference so shutdown can take ownership
        drop(engine_arc);

        // Clean shutdown (takes ownership)
        search.shutdown().expect("Shutdown should succeed");
    }

    #[test]
    fn test_embedded_search_clone() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("meilisearch");

        let search1 = EmbeddedSearch::new(db_path).expect("Should create search");
        let search2 = search1.clone();

        // Both should point to the same inner
        assert!(Arc::ptr_eq(&search1.inner, &search2.inner));

        // Shutdown with multiple references - should succeed but defer actual shutdown
        search1.shutdown().expect("Shutdown should succeed");
        // search2 still holds a reference, drop it to clean up
        drop(search2);
    }

    #[test]
    fn test_custom_max_index_size() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("meilisearch");

        // 1 GiB max index size
        let max_size = 1024 * 1024 * 1024;
        let search = EmbeddedSearch::with_max_index_size(db_path, max_size);
        assert!(
            search.is_ok(),
            "EmbeddedSearch should initialize with custom size"
        );

        search.unwrap().shutdown().expect("Shutdown should succeed");
    }

    #[test]
    fn test_debug_impl() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("meilisearch");

        let search = EmbeddedSearch::new(db_path).expect("Should create search");
        let debug_str = format!("{:?}", search);
        assert!(debug_str.contains("EmbeddedSearch"));
        assert!(debug_str.contains("Arc<Engine>"));

        search.shutdown().expect("Shutdown should succeed");
    }
}
