//! SurrealDB storage wrapper for TTRPG Assistant.
//!
//! Provides a thread-safe wrapper around SurrealDB with RocksDB persistence,
//! handling initialization, schema application, and shared access across async tasks.

use std::path::PathBuf;
use std::sync::Arc;

use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;
use tokio::sync::RwLock;

use super::error::StorageError;
use super::schema::SCHEMA_V1;

/// Storage configuration for SurrealDB.
#[derive(Clone, Debug)]
pub struct StorageConfig {
    /// SurrealDB namespace (default: "ttrpg")
    pub namespace: String,
    /// SurrealDB database name (default: "main")
    pub database: String,
    /// Default vector dimensions for embeddings (default: 768 for BGE-base)
    pub default_vector_dimensions: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            namespace: "ttrpg".to_string(),
            database: "main".to_string(),
            default_vector_dimensions: 768,
        }
    }
}

/// Embedded SurrealDB storage with unified document, vector, and graph capabilities.
///
/// This wrapper provides:
/// - Thread-safe access via `Arc<Surreal<Db>>`
/// - Automatic schema initialization on startup
/// - Configuration management with sensible TTRPG defaults
///
/// # Example
///
/// ```no_run
/// use std::path::PathBuf;
/// use ttrpg_assistant::core::storage::SurrealStorage;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let storage = SurrealStorage::new(PathBuf::from("./data/surrealdb")).await?;
///
/// // Get database reference for queries
/// let db = storage.db();
///
/// // Clone Arc for async tasks
/// let db_clone = storage.clone_db();
/// tokio::spawn(async move {
///     // Use db_clone in background task
/// });
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct SurrealStorage {
    /// Thread-safe database handle
    db: Arc<Surreal<Db>>,
    /// Configuration settings
    config: Arc<RwLock<StorageConfig>>,
}

impl SurrealStorage {
    /// Initialize SurrealDB with RocksDB persistence.
    ///
    /// Creates the database directory if it doesn't exist, connects to SurrealDB
    /// with the RocksDB storage engine, selects the namespace and database,
    /// and applies the schema.
    ///
    /// # Arguments
    ///
    /// * `db_path` - Path where RocksDB will store data files
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Init` if directory creation fails,
    /// `StorageError::Database` if connection fails, or
    /// `StorageError::Migration` if schema application fails.
    pub async fn new(db_path: PathBuf) -> Result<Self, StorageError> {
        // Create directory if it doesn't exist
        if !db_path.exists() {
            std::fs::create_dir_all(&db_path).map_err(|e| {
                StorageError::Init(format!(
                    "Failed to create database directory at {}: {}",
                    db_path.display(),
                    e
                ))
            })?;
        }

        // Connect to SurrealDB with RocksDB storage
        let db = Surreal::new::<RocksDb>(db_path.clone())
            .await
            .map_err(|e| StorageError::Database(format!("Failed to connect to SurrealDB: {}", e)))?;

        // Select namespace and database
        db.use_ns("ttrpg")
            .use_db("main")
            .await
            .map_err(|e| StorageError::Database(format!("Failed to select namespace/database: {}", e)))?;

        let storage = Self {
            db: Arc::new(db),
            config: Arc::new(RwLock::new(StorageConfig::default())),
        };

        // Apply schema
        storage.apply_schema().await?;

        tracing::info!(
            path = %db_path.display(),
            namespace = "ttrpg",
            database = "main",
            "SurrealDB storage initialized"
        );

        Ok(storage)
    }

    /// Initialize SurrealDB with custom configuration.
    ///
    /// Like `new()`, but allows specifying a custom namespace and database.
    ///
    /// # Arguments
    ///
    /// * `db_path` - Path where RocksDB will store data files
    /// * `config` - Custom storage configuration
    pub async fn with_config(db_path: PathBuf, config: StorageConfig) -> Result<Self, StorageError> {
        // Create directory if it doesn't exist
        if !db_path.exists() {
            std::fs::create_dir_all(&db_path).map_err(|e| {
                StorageError::Init(format!(
                    "Failed to create database directory at {}: {}",
                    db_path.display(),
                    e
                ))
            })?;
        }

        // Connect to SurrealDB with RocksDB storage
        let db = Surreal::new::<RocksDb>(db_path.clone())
            .await
            .map_err(|e| StorageError::Database(format!("Failed to connect to SurrealDB: {}", e)))?;

        // Select namespace and database from config
        db.use_ns(&config.namespace)
            .use_db(&config.database)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to select namespace/database: {}", e)))?;

        let storage = Self {
            db: Arc::new(db),
            config: Arc::new(RwLock::new(config.clone())),
        };

        // Apply schema
        storage.apply_schema().await?;

        tracing::info!(
            path = %db_path.display(),
            namespace = %config.namespace,
            database = %config.database,
            "SurrealDB storage initialized with custom config"
        );

        Ok(storage)
    }

    /// Apply database schema (tables, indexes, analyzers).
    ///
    /// This is called automatically during initialization. The schema
    /// uses `DEFINE ... IF NOT EXISTS` statements to be idempotent.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Migration` if schema application fails.
    pub async fn apply_schema(&self) -> Result<(), StorageError> {
        self.db
            .query(SCHEMA_V1)
            .await
            .map_err(|e| StorageError::Migration(format!("Schema application failed: {}", e)))?;

        tracing::debug!("Schema V1 applied successfully");
        Ok(())
    }

    /// Get direct database reference for queries.
    ///
    /// Use this for executing SurrealQL queries directly.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use ttrpg_assistant::core::storage::SurrealStorage;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = SurrealStorage::new(PathBuf::from("./data")).await?;
    /// let result: Vec<serde_json::Value> = storage.db()
    ///     .query("SELECT * FROM document LIMIT 10")
    ///     .await?
    ///     .take(0)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn db(&self) -> &Surreal<Db> {
        &self.db
    }

    /// Clone Arc for sharing across async tasks.
    ///
    /// Returns an `Arc<Surreal<Db>>` that can be moved into async tasks
    /// or threads for concurrent database access.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use ttrpg_assistant::core::storage::SurrealStorage;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let storage = SurrealStorage::new(PathBuf::from("./data")).await?;
    /// let db = storage.clone_db();
    ///
    /// tokio::spawn(async move {
    ///     let _result = db.query("SELECT count() FROM document").await;
    /// });
    /// # Ok(())
    /// # }
    /// ```
    pub fn clone_db(&self) -> Arc<Surreal<Db>> {
        Arc::clone(&self.db)
    }

    /// Get current configuration.
    ///
    /// Returns a clone of the current storage configuration.
    pub async fn config(&self) -> StorageConfig {
        self.config.read().await.clone()
    }

    /// Update configuration.
    ///
    /// Note: This does not change the active namespace/database connection.
    /// Use this primarily for updating `default_vector_dimensions`.
    pub async fn set_config(&self, config: StorageConfig) {
        let mut cfg = self.config.write().await;
        *cfg = config;
    }

    /// Check if the database is healthy and accessible.
    ///
    /// Executes a simple query to verify connectivity.
    pub async fn health_check(&self) -> Result<(), StorageError> {
        self.db
            .query("RETURN 1")
            .await
            .map_err(|e| StorageError::Database(format!("Health check failed: {}", e)))?;
        Ok(())
    }
}

impl std::fmt::Debug for SurrealStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SurrealStorage")
            .field("db", &"<Surreal<Db>>")
            .field("config", &self.config)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_surreal_storage_creation() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SurrealStorage::new(temp_dir.path().to_path_buf()).await;
        assert!(storage.is_ok(), "Storage creation failed: {:?}", storage.err());
    }

    #[tokio::test]
    async fn test_storage_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("nested").join("db");

        assert!(!db_path.exists());
        let storage = SurrealStorage::new(db_path.clone()).await;
        assert!(storage.is_ok(), "Storage creation failed: {:?}", storage.err());
        assert!(db_path.exists(), "Directory was not created");
    }

    #[tokio::test]
    async fn test_storage_default_config() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SurrealStorage::new(temp_dir.path().to_path_buf())
            .await
            .expect("Storage creation failed");

        let config = storage.config().await;
        assert_eq!(config.namespace, "ttrpg");
        assert_eq!(config.database, "main");
        assert_eq!(config.default_vector_dimensions, 768);
    }

    #[tokio::test]
    async fn test_storage_custom_config() {
        let temp_dir = TempDir::new().unwrap();
        let custom_config = StorageConfig {
            namespace: "test_ns".to_string(),
            database: "test_db".to_string(),
            default_vector_dimensions: 1024,
        };

        let storage = SurrealStorage::with_config(temp_dir.path().to_path_buf(), custom_config)
            .await
            .expect("Storage creation failed");

        let config = storage.config().await;
        assert_eq!(config.namespace, "test_ns");
        assert_eq!(config.database, "test_db");
        assert_eq!(config.default_vector_dimensions, 1024);
    }

    #[tokio::test]
    async fn test_storage_health_check() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SurrealStorage::new(temp_dir.path().to_path_buf())
            .await
            .expect("Storage creation failed");

        let result = storage.health_check().await;
        assert!(result.is_ok(), "Health check failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_storage_db_query() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SurrealStorage::new(temp_dir.path().to_path_buf())
            .await
            .expect("Storage creation failed");

        // Test basic query
        let result: Result<Vec<serde_json::Value>, _> = storage
            .db()
            .query("RETURN 'hello'")
            .await
            .and_then(|mut res| res.take(0));

        assert!(result.is_ok(), "Query failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_storage_clone_db() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SurrealStorage::new(temp_dir.path().to_path_buf())
            .await
            .expect("Storage creation failed");

        let db_clone = storage.clone_db();

        // Use cloned db in a spawned task
        let handle = tokio::spawn(async move {
            let result: Result<Vec<serde_json::Value>, _> =
                db_clone.query("RETURN 42").await.and_then(|mut res| res.take(0));
            result.is_ok()
        });

        let success = handle.await.expect("Task panicked");
        assert!(success, "Cloned db query failed");
    }

    #[tokio::test]
    async fn test_storage_schema_applied() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SurrealStorage::new(temp_dir.path().to_path_buf())
            .await
            .expect("Storage creation failed");

        // Query for schema_version table which should exist after schema application
        let result: Result<Vec<serde_json::Value>, _> = storage
            .db()
            .query("INFO FOR TABLE schema_version")
            .await
            .and_then(|mut res| res.take(0));

        assert!(result.is_ok(), "Schema table not found: {:?}", result.err());
    }
}
