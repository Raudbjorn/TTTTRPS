//! Post-Ingestion Dictionary Rebuild Service
//!
//! Handles automatic dictionary regeneration after document ingestion.
//! Uses debouncing to avoid rebuilding on every single document - waits for
//! batch completion before triggering a rebuild.
//!
//! # Architecture
//!
//! ```text
//! Ingestion Complete ────┐
//!                        ▼
//!              ┌─────────────────┐
//!              │ Debounce Timer  │  (5s default)
//!              └────────┬────────┘
//!                       ▼
//!              ┌─────────────────────────────────┐
//!              │ Background Task (tokio::spawn)  │
//!              │  1. Query chunks from SurrealDB │
//!              │  2. Build corpus dictionary     │
//!              │  3. Build bigram dictionary     │
//!              │  4. Reload TypoCorrector        │
//!              └─────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! // After successful ingestion:
//! rebuild_service.trigger_rebuild().await;
//!
//! // The rebuild runs in the background after debounce period
//! ```

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

use super::dictionary::DictionaryGenerator;
use super::paths::{ensure_user_data_dir, get_bigram_dictionary_path, get_corpus_dictionary_path};
use super::pipeline::QueryPipeline;

/// Statistics from a dictionary rebuild operation.
#[derive(Debug, Clone, Default)]
pub struct RebuildStats {
    /// Number of unique words in corpus dictionary
    pub word_count: usize,
    /// Number of bigrams in bigram dictionary
    pub bigram_count: usize,
    /// Number of chunks processed
    pub chunks_processed: usize,
    /// Time taken to rebuild (milliseconds)
    pub rebuild_time_ms: u64,
}

/// Configuration for the rebuild service.
#[derive(Clone, Debug)]
pub struct RebuildConfig {
    /// Minimum delay between rebuild triggers (debounce period)
    /// Default: 5 seconds
    pub debounce_duration: Duration,
    /// Minimum number of chunks to trigger a rebuild
    /// Prevents rebuilding for single document imports
    /// Default: 10 chunks
    pub min_chunks_for_rebuild: usize,
    /// Enable/disable automatic rebuilds
    pub auto_rebuild_enabled: bool,
}

impl Default for RebuildConfig {
    fn default() -> Self {
        Self {
            debounce_duration: Duration::from_secs(5),
            min_chunks_for_rebuild: 10,
            auto_rebuild_enabled: true,
        }
    }
}

/// Service for managing post-ingestion dictionary rebuilds.
///
/// Provides debounced dictionary rebuilding to avoid performance issues
/// during bulk ingestion while still keeping dictionaries up-to-date.
pub struct DictionaryRebuildService {
    /// Configuration
    config: RebuildConfig,
    /// Whether a rebuild is currently pending
    pending: AtomicBool,
    /// Whether a rebuild is currently in progress
    in_progress: AtomicBool,
    /// Timestamp of last rebuild trigger
    last_trigger: AtomicU64,
    /// Last rebuild statistics
    last_stats: Mutex<Option<RebuildStats>>,
}

impl DictionaryRebuildService {
    /// Create a new rebuild service with default configuration.
    pub fn new() -> Self {
        Self::with_config(RebuildConfig::default())
    }

    /// Create a new rebuild service with custom configuration.
    pub fn with_config(config: RebuildConfig) -> Self {
        Self {
            config,
            pending: AtomicBool::new(false),
            in_progress: AtomicBool::new(false),
            last_trigger: AtomicU64::new(0),
            last_stats: Mutex::new(None),
        }
    }

    /// Trigger a debounced dictionary rebuild.
    ///
    /// This method returns immediately. The actual rebuild happens in a background
    /// task after the debounce period expires.
    ///
    /// Multiple calls within the debounce period will be coalesced into a single rebuild.
    pub fn trigger_rebuild(&self) {
        if !self.config.auto_rebuild_enabled {
            log::debug!("Dictionary rebuild triggered but auto-rebuild is disabled");
            return;
        }

        // Mark as pending
        self.pending.store(true, Ordering::SeqCst);
        self.last_trigger.store(
            Instant::now().elapsed().as_millis() as u64,
            Ordering::SeqCst,
        );

        log::debug!(
            "Dictionary rebuild triggered, debounce period: {:?}",
            self.config.debounce_duration
        );
    }

    /// Check if a rebuild is pending.
    pub fn is_pending(&self) -> bool {
        self.pending.load(Ordering::SeqCst)
    }

    /// Check if a rebuild is currently in progress.
    pub fn is_in_progress(&self) -> bool {
        self.in_progress.load(Ordering::SeqCst)
    }

    /// Get the last rebuild statistics.
    pub async fn last_stats(&self) -> Option<RebuildStats> {
        self.last_stats.lock().await.clone()
    }

    /// Execute the rebuild if conditions are met.
    ///
    /// This is the main entry point called from the background task.
    /// Returns rebuild statistics if a rebuild was performed.
    pub async fn execute_rebuild(
        &self,
        db: &Surreal<Db>,
        query_pipeline: Option<&tokio::sync::RwLock<QueryPipeline>>,
    ) -> Option<RebuildStats> {
        // Check if rebuild is pending and not already in progress
        if !self.pending.load(Ordering::SeqCst) {
            return None;
        }

        if self.in_progress.swap(true, Ordering::SeqCst) {
            log::debug!("Dictionary rebuild already in progress, skipping");
            return None;
        }

        // Wait for debounce period
        sleep(self.config.debounce_duration).await;

        // Clear pending flag (new triggers after this will set it again)
        self.pending.store(false, Ordering::SeqCst);

        // Execute the rebuild (timing is tracked inside do_rebuild)
        let stats = match self.do_rebuild(db, query_pipeline).await {
            Ok(stats) => {
                log::info!(
                    "Dictionary rebuild complete: {} words, {} bigrams, {} chunks in {}ms",
                    stats.word_count,
                    stats.bigram_count,
                    stats.chunks_processed,
                    stats.rebuild_time_ms
                );
                Some(stats)
            }
            Err(e) => {
                log::error!("Dictionary rebuild failed: {}", e);
                None
            }
        };

        // Store stats
        if let Some(ref s) = stats {
            *self.last_stats.lock().await = Some(s.clone());
        }

        // Clear in-progress flag
        self.in_progress.store(false, Ordering::SeqCst);

        // If there was a new trigger during rebuild, schedule another
        if self.pending.load(Ordering::SeqCst) {
            log::debug!("New rebuild trigger detected during rebuild, will rebuild again");
        }

        stats
    }

    /// Perform the actual dictionary rebuild.
    async fn do_rebuild(
        &self,
        db: &Surreal<Db>,
        query_pipeline: Option<&tokio::sync::RwLock<QueryPipeline>>,
    ) -> Result<RebuildStats, String> {
        let start = Instant::now();

        // Ensure user data directory exists
        let _data_dir = ensure_user_data_dir()
            .map_err(|e| format!("Failed to create user data directory: {}", e))?;

        // Query chunk content from SurrealDB
        let contents = query_chunk_content(db).await?;
        let chunks_processed = contents.len();

        // Check minimum chunk threshold
        if chunks_processed < self.config.min_chunks_for_rebuild {
            log::debug!(
                "Skipping rebuild: {} chunks < {} minimum",
                chunks_processed,
                self.config.min_chunks_for_rebuild
            );
            return Ok(RebuildStats {
                chunks_processed,
                rebuild_time_ms: start.elapsed().as_millis() as u64,
                ..Default::default()
            });
        }

        // Build dictionaries
        let generator = DictionaryGenerator::default();
        let content_iter = contents.iter().map(|s| s.as_str());

        // Build corpus dictionary
        let corpus_path = get_corpus_dictionary_path()
            .ok_or_else(|| "Could not determine corpus dictionary path".to_string())?;
        let word_count = generator
            .build_corpus_dictionary_from_iter(content_iter.clone(), &corpus_path)
            .map_err(|e| format!("Failed to build corpus dictionary: {}", e))?;

        // Build bigram dictionary
        let bigram_path = get_bigram_dictionary_path()
            .ok_or_else(|| "Could not determine bigram dictionary path".to_string())?;
        let bigram_count = generator
            .build_bigram_dictionary_from_iter(content_iter, &bigram_path)
            .map_err(|e| format!("Failed to build bigram dictionary: {}", e))?;

        // Reload typo dictionaries in the query pipeline
        if let Some(pipeline_lock) = query_pipeline {
            match pipeline_lock.write().await.reload_typo_dictionaries() {
                Ok(()) => {
                    log::info!("TypoCorrector dictionaries reloaded successfully");
                }
                Err(e) => {
                    log::warn!("Failed to reload TypoCorrector dictionaries: {}", e);
                }
            }
        }

        let rebuild_time_ms = start.elapsed().as_millis() as u64;

        Ok(RebuildStats {
            word_count,
            bigram_count,
            chunks_processed,
            rebuild_time_ms,
        })
    }
}

impl Default for DictionaryRebuildService {
    fn default() -> Self {
        Self::new()
    }
}

/// Query all chunk content from SurrealDB.
///
/// Returns a vector of content strings from all chunks in the database.
async fn query_chunk_content(db: &Surreal<Db>) -> Result<Vec<String>, String> {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct ContentRow {
        content: String,
    }

    let result: Vec<ContentRow> = db
        .query("SELECT content FROM chunk")
        .await
        .map_err(|e| format!("Database query failed: {}", e))?
        .take(0)
        .map_err(|e| format!("Failed to extract content: {}", e))?;

    Ok(result.into_iter().map(|r| r.content).collect())
}

/// Spawn a background dictionary rebuild task.
///
/// This is the main entry point for triggering rebuilds from ingestion commands.
/// The rebuild runs asynchronously and does not block the calling code.
///
/// # Arguments
///
/// * `rebuild_service` - The rebuild service instance (shared via Arc)
/// * `db` - SurrealDB connection (shared via Arc)
/// * `query_pipeline` - Query pipeline for reloading dictionaries (optional)
///
/// # Example
///
/// ```rust,ignore
/// spawn_dictionary_rebuild(
///     rebuild_service.clone(),
///     surreal_storage.db_arc(),
///     Some(state.query_pipeline.clone()),
/// );
/// ```
pub fn spawn_dictionary_rebuild(
    rebuild_service: Arc<DictionaryRebuildService>,
    db: Arc<Surreal<Db>>,
    query_pipeline: Option<Arc<tokio::sync::RwLock<QueryPipeline>>>,
) {
    // Trigger the rebuild (sets pending flag)
    rebuild_service.trigger_rebuild();

    // Spawn background task
    tokio::spawn(async move {
        // Convert Arc<RwLock<QueryPipeline>> to reference for execute_rebuild
        // We need to handle the Option<Arc> -> Option<&RwLock> conversion
        let pipeline_ref = query_pipeline.as_ref().map(|arc| arc.as_ref());

        rebuild_service.execute_rebuild(&db, pipeline_ref).await;
    });
}

/// Force an immediate dictionary rebuild (bypasses debouncing).
///
/// Use this for explicit user-requested rebuilds or initial setup.
/// Runs synchronously (awaits completion).
pub async fn force_dictionary_rebuild(
    db: &Surreal<Db>,
    query_pipeline: Option<&tokio::sync::RwLock<QueryPipeline>>,
) -> Result<RebuildStats, String> {
    let start = Instant::now();

    // Ensure user data directory exists
    let _data_dir = ensure_user_data_dir()
        .map_err(|e| format!("Failed to create user data directory: {}", e))?;

    // Query chunk content
    let contents = query_chunk_content(db).await?;
    let chunks_processed = contents.len();

    if chunks_processed == 0 {
        log::info!("No chunks in database, skipping dictionary rebuild");
        return Ok(RebuildStats {
            rebuild_time_ms: start.elapsed().as_millis() as u64,
            ..Default::default()
        });
    }

    // Build dictionaries
    let generator = DictionaryGenerator::default();
    let content_iter = contents.iter().map(|s| s.as_str());

    // Build corpus dictionary
    let corpus_path = get_corpus_dictionary_path()
        .ok_or_else(|| "Could not determine corpus dictionary path".to_string())?;
    let word_count = generator
        .build_corpus_dictionary_from_iter(content_iter.clone(), &corpus_path)
        .map_err(|e| format!("Failed to build corpus dictionary: {}", e))?;

    log::info!("Built corpus dictionary: {} words at {:?}", word_count, corpus_path);

    // Build bigram dictionary
    let bigram_path = get_bigram_dictionary_path()
        .ok_or_else(|| "Could not determine bigram dictionary path".to_string())?;
    let bigram_count = generator
        .build_bigram_dictionary_from_iter(contents.iter().map(|s| s.as_str()), &bigram_path)
        .map_err(|e| format!("Failed to build bigram dictionary: {}", e))?;

    log::info!("Built bigram dictionary: {} bigrams at {:?}", bigram_count, bigram_path);

    // Reload typo dictionaries in the query pipeline
    if let Some(pipeline_lock) = query_pipeline {
        match pipeline_lock.write().await.reload_typo_dictionaries() {
            Ok(()) => {
                log::info!("TypoCorrector dictionaries reloaded successfully");
            }
            Err(e) => {
                log::warn!("Failed to reload TypoCorrector dictionaries: {}", e);
            }
        }
    }

    let rebuild_time_ms = start.elapsed().as_millis() as u64;

    log::info!(
        "Dictionary rebuild complete: {} words, {} bigrams from {} chunks in {}ms",
        word_count,
        bigram_count,
        chunks_processed,
        rebuild_time_ms
    );

    Ok(RebuildStats {
        word_count,
        bigram_count,
        chunks_processed,
        rebuild_time_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rebuild_config_defaults() {
        let config = RebuildConfig::default();
        assert_eq!(config.debounce_duration, Duration::from_secs(5));
        assert_eq!(config.min_chunks_for_rebuild, 10);
        assert!(config.auto_rebuild_enabled);
    }

    #[test]
    fn test_rebuild_service_creation() {
        let service = DictionaryRebuildService::new();
        assert!(!service.is_pending());
        assert!(!service.is_in_progress());
    }

    #[test]
    fn test_trigger_rebuild_sets_pending() {
        let service = DictionaryRebuildService::new();
        service.trigger_rebuild();
        assert!(service.is_pending());
    }

    #[test]
    fn test_trigger_rebuild_disabled() {
        let config = RebuildConfig {
            auto_rebuild_enabled: false,
            ..Default::default()
        };
        let service = DictionaryRebuildService::with_config(config);
        service.trigger_rebuild();
        // Should NOT set pending when disabled
        assert!(!service.is_pending());
    }

    #[test]
    fn test_rebuild_stats_default() {
        let stats = RebuildStats::default();
        assert_eq!(stats.word_count, 0);
        assert_eq!(stats.bigram_count, 0);
        assert_eq!(stats.chunks_processed, 0);
        assert_eq!(stats.rebuild_time_ms, 0);
    }
}
