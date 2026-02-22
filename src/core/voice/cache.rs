//! Audio Cache System (TASK-005)
//!
//! Caches synthesized audio with intelligent LRU eviction.
//! Provides disk storage with size tracking and cache statistics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tokio::fs;
use std::future::Future;
use sha2::{Sha256, Digest};

use super::types::{VoiceProviderType, VoiceSettings, OutputFormat};

// ============================================================================
// Constants
// ============================================================================

/// Default maximum cache size: 500 MB
const DEFAULT_MAX_SIZE_BYTES: u64 = 500 * 1024 * 1024;

/// Minimum free space to maintain after eviction: 10 MB
const MIN_FREE_SPACE_BYTES: u64 = 10 * 1024 * 1024;

// ============================================================================
// Cache Types
// ============================================================================

/// A single cache entry representing a stored audio file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Unique cache key
    pub key: String,
    /// Path to the cached audio file
    pub path: PathBuf,
    /// Size of the cached file in bytes
    pub size: u64,
    /// When this entry was created
    pub created_at: DateTime<Utc>,
    /// When this entry was last accessed
    pub last_accessed: DateTime<Utc>,
    /// Number of times this entry has been accessed
    pub access_count: u32,
    /// Associated tags (session_id, npc_id, campaign_id, etc.)
    pub tags: Vec<String>,
    /// Voice profile ID used to generate this audio
    pub profile_id: Option<String>,
    /// Duration of the audio in milliseconds
    pub duration_ms: Option<u64>,
    /// Output format
    pub format: OutputFormat,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(key: String, path: PathBuf, size: u64, format: OutputFormat) -> Self {
        let now = Utc::now();
        Self {
            key,
            path,
            size,
            created_at: now,
            last_accessed: now,
            access_count: 1,
            tags: Vec::new(),
            profile_id: None,
            duration_ms: None,
            format,
        }
    }

    /// Record access to this entry
    pub fn record_access(&mut self) {
        self.last_accessed = Utc::now();
        self.access_count = self.access_count.saturating_add(1);
    }

    /// Add a tag to this entry
    pub fn add_tag(&mut self, tag: &str) {
        if !self.tags.contains(&tag.to_string()) {
            self.tags.push(tag.to_string());
        }
    }

    /// Check if this entry has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Get age in seconds since creation
    pub fn age_seconds(&self) -> i64 {
        (Utc::now() - self.created_at).num_seconds()
    }

    /// Get time since last access in seconds
    pub fn idle_seconds(&self) -> i64 {
        (Utc::now() - self.last_accessed).num_seconds()
    }
}

/// Configuration for the audio cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Maximum cache size in bytes
    pub max_size_bytes: u64,
    /// Enable automatic eviction when cache is full
    pub auto_eviction: bool,
    /// Minimum entry age (seconds) before eligible for eviction
    pub min_age_for_eviction_secs: i64,
    /// Enable cache statistics tracking
    pub track_stats: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size_bytes: DEFAULT_MAX_SIZE_BYTES,
            auto_eviction: true,
            min_age_for_eviction_secs: 60, // 1 minute
            track_stats: true,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheStats {
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Number of entries evicted
    pub evictions: u64,
    /// Total entries currently in cache
    pub entry_count: usize,
    /// Current cache size in bytes
    pub current_size_bytes: u64,
    /// Maximum cache size in bytes
    pub max_size_bytes: u64,
    /// Number of entries by format
    pub entries_by_format: HashMap<String, usize>,
    /// Cache hit rate (0.0 - 1.0)
    pub hit_rate: f64,
    /// Oldest entry age in seconds
    pub oldest_entry_age_secs: i64,
    /// Average entry size in bytes
    pub avg_entry_size_bytes: u64,
}

impl CacheStats {
    /// Calculate hit rate
    pub fn calculate_hit_rate(&mut self) {
        let total = self.hits + self.misses;
        self.hit_rate = if total > 0 {
            self.hits as f64 / total as f64
        } else {
            0.0
        };
    }
}

// ============================================================================
// Cache Key Generation
// ============================================================================

/// Parameters used to generate a cache key
#[derive(Debug, Clone)]
pub struct CacheKeyParams {
    /// The text to synthesize
    pub text: String,
    /// Voice provider type
    pub provider: VoiceProviderType,
    /// Provider-specific voice ID
    pub voice_id: String,
    /// Voice settings hash
    pub settings_hash: u64,
    /// Output format
    pub format: OutputFormat,
}

impl CacheKeyParams {
    /// Create new cache key parameters
    pub fn new(
        text: &str,
        provider: VoiceProviderType,
        voice_id: &str,
        settings: &VoiceSettings,
        format: OutputFormat,
    ) -> Self {
        Self {
            text: text.to_string(),
            provider,
            voice_id: voice_id.to_string(),
            settings_hash: hash_settings_sha256(settings),
            format,
        }
    }

    /// Generate the cache key string using SHA256
    ///
    /// Creates a deterministic hash from text + voice_id + provider + settings
    /// that can be used as a filename-safe cache key.
    pub fn to_key(&self) -> String {
        let mut hasher = Sha256::new();

        // Hash all components in a deterministic order
        hasher.update(self.text.as_bytes());
        hasher.update(b"|"); // Separator to prevent collisions
        hasher.update(self.voice_id.as_bytes());
        hasher.update(b"|");
        hasher.update(format!("{:?}", self.provider).as_bytes());
        hasher.update(b"|");
        hasher.update(self.settings_hash.to_le_bytes());
        hasher.update(b"|");
        hasher.update(format!("{:?}", self.format).as_bytes());

        let result = hasher.finalize();
        // Use first 16 bytes (32 hex chars) for a shorter but still unique key
        format!("{}.{}", hex::encode(&result[..16]), self.format.extension())
    }

    /// Generate a full SHA256 hash for verification purposes
    pub fn full_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.text.as_bytes());
        hasher.update(b"|");
        hasher.update(self.voice_id.as_bytes());
        hasher.update(b"|");
        hasher.update(format!("{:?}", self.provider).as_bytes());
        hasher.update(b"|");
        hasher.update(self.settings_hash.to_le_bytes());
        hasher.update(b"|");
        hasher.update(format!("{:?}", self.format).as_bytes());

        hex::encode(hasher.finalize())
    }
}

/// Hash voice settings to a u64 using SHA256
///
/// Converts float values to their bit representation for deterministic hashing,
/// then reduces the SHA256 output to a u64 for compact storage.
fn hash_settings_sha256(settings: &VoiceSettings) -> u64 {
    let mut hasher = Sha256::new();

    // Convert floats to bits for consistent hashing across platforms
    hasher.update(settings.stability.to_bits().to_le_bytes());
    hasher.update(settings.similarity_boost.to_bits().to_le_bytes());
    hasher.update(settings.style.to_bits().to_le_bytes());
    hasher.update([settings.use_speaker_boost as u8]);

    let result = hasher.finalize();
    // Take first 8 bytes and convert to u64
    u64::from_le_bytes(result[..8].try_into().unwrap())
}

// ============================================================================
// Audio Cache
// ============================================================================

/// Error type for cache operations
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Cache entry not found: {0}")]
    NotFound(String),

    #[error("Cache is full and eviction failed")]
    CacheFull,

    #[error("Invalid cache state: {0}")]
    InvalidState(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub type CacheResult<T> = Result<T, CacheError>;

/// Audio cache with LRU eviction and statistics tracking
pub struct AudioCache {
    /// Directory where cache files are stored
    cache_dir: PathBuf,
    /// Cache configuration
    config: CacheConfig,
    /// Current cache size in bytes
    current_size: AtomicU64,
    /// Cache entries indexed by key
    entries: RwLock<HashMap<String, CacheEntry>>,
    /// Statistics (atomic counters)
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
}

impl AudioCache {
    /// Create a new audio cache with the given directory and config
    pub async fn new(cache_dir: PathBuf, config: CacheConfig) -> CacheResult<Self> {
        // Ensure cache directory exists
        fs::create_dir_all(&cache_dir).await?;

        let cache = Self {
            cache_dir,
            config,
            current_size: AtomicU64::new(0),
            entries: RwLock::new(HashMap::new()),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        };

        // Scan existing cache directory and rebuild index
        cache.rebuild_index().await?;

        Ok(cache)
    }

    /// Create a new cache with default configuration
    pub async fn with_defaults(cache_dir: PathBuf) -> CacheResult<Self> {
        Self::new(cache_dir, CacheConfig::default()).await
    }

    /// Get the cache directory path
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Get the current cache size in bytes
    pub fn current_size(&self) -> u64 {
        self.current_size.load(Ordering::Relaxed)
    }

    /// Get the maximum cache size in bytes
    pub fn max_size(&self) -> u64 {
        self.config.max_size_bytes
    }

    /// Check if the cache contains an entry for the given key
    pub async fn contains(&self, key: &str) -> bool {
        self.entries.read().await.contains_key(key)
    }

    /// Get a cache entry by key
    pub async fn get(&self, key: &str) -> Option<PathBuf> {
        let mut entries = self.entries.write().await;

        if let Some(entry) = entries.get_mut(key) {
            // Update access stats
            entry.record_access();
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.path.clone())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Get or synthesize audio
    ///
    /// If the audio is cached, returns the cached path.
    /// Otherwise, calls the synthesize function and caches the result.
    pub async fn get_or_synthesize<F, Fut>(
        &self,
        key: &str,
        format: OutputFormat,
        tags: &[String],
        synthesize: F,
    ) -> CacheResult<PathBuf>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = CacheResult<Vec<u8>>>,
    {
        // Check cache first
        if let Some(path) = self.get(key).await {
            if path.exists() {
                return Ok(path);
            }
            // Path doesn't exist, remove stale entry
            self.remove(key).await?;
        }

        // Synthesize
        let audio_data = synthesize().await?;
        let size = audio_data.len() as u64;

        // Ensure we have space
        self.ensure_space(size).await?;

        // Write to disk
        let file_path = self.cache_dir.join(format!("{}.{}", key, format.extension()));
        fs::write(&file_path, &audio_data).await?;

        // Create entry
        let mut entry = CacheEntry::new(key.to_string(), file_path.clone(), size, format);
        for tag in tags {
            entry.add_tag(tag);
        }

        // Store entry
        {
            let mut entries = self.entries.write().await;
            entries.insert(key.to_string(), entry);
        }

        // Update size
        self.current_size.fetch_add(size, Ordering::Relaxed);

        Ok(file_path)
    }

    /// Store audio data in the cache
    pub async fn put(
        &self,
        key: &str,
        data: &[u8],
        format: OutputFormat,
        tags: &[String],
    ) -> CacheResult<PathBuf> {
        let size = data.len() as u64;

        // Ensure we have space
        self.ensure_space(size).await?;

        // Write to disk
        let file_path = self.cache_dir.join(format!("{}.{}", key, format.extension()));
        fs::write(&file_path, data).await?;

        // Create entry
        let mut entry = CacheEntry::new(key.to_string(), file_path.clone(), size, format);
        for tag in tags {
            entry.add_tag(tag);
        }

        // Store entry
        {
            let mut entries = self.entries.write().await;
            // If key already exists, remove old size first
            if let Some(old_entry) = entries.remove(key) {
                self.current_size.fetch_sub(old_entry.size, Ordering::Relaxed);
            }
            entries.insert(key.to_string(), entry);
        }

        // Update size
        self.current_size.fetch_add(size, Ordering::Relaxed);

        Ok(file_path)
    }

    /// Remove an entry from the cache
    pub async fn remove(&self, key: &str) -> CacheResult<()> {
        let mut entries = self.entries.write().await;

        if let Some(entry) = entries.remove(key) {
            // Delete file
            if entry.path.exists() {
                fs::remove_file(&entry.path).await?;
            }
            // Update size
            self.current_size.fetch_sub(entry.size, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Clear all entries with a specific tag
    pub async fn clear_by_tag(&self, tag: &str) -> CacheResult<usize> {
        let entries_to_remove: Vec<String> = {
            let entries = self.entries.read().await;
            entries
                .iter()
                .filter(|(_, e)| e.has_tag(tag))
                .map(|(k, _)| k.clone())
                .collect()
        };

        let count = entries_to_remove.len();
        for key in entries_to_remove {
            self.remove(&key).await?;
        }

        Ok(count)
    }

    /// Clear all cache entries
    pub async fn clear(&self) -> CacheResult<()> {
        let mut entries = self.entries.write().await;

        for (_, entry) in entries.drain() {
            if entry.path.exists() {
                let _ = fs::remove_file(&entry.path).await;
            }
        }

        self.current_size.store(0, Ordering::Relaxed);

        Ok(())
    }

    /// Ensure there's enough space for new data
    async fn ensure_space(&self, bytes_needed: u64) -> CacheResult<()> {
        if !self.config.auto_eviction {
            return Ok(());
        }

        let current = self.current_size.load(Ordering::Relaxed);
        let max = self.config.max_size_bytes;

        if current + bytes_needed + MIN_FREE_SPACE_BYTES <= max {
            return Ok(());
        }

        // Need to evict
        let bytes_to_free = (current + bytes_needed + MIN_FREE_SPACE_BYTES).saturating_sub(max);
        self.evict_lru(bytes_to_free).await
    }

    /// Evict entries using LRU policy until enough space is freed
    async fn evict_lru(&self, bytes_needed: u64) -> CacheResult<()> {
        let mut entries = self.entries.write().await;

        // Sort entries by last_accessed (oldest first)
        let mut sorted_entries: Vec<(String, DateTime<Utc>, u64)> = entries
            .iter()
            .filter(|(_, e)| e.idle_seconds() >= self.config.min_age_for_eviction_secs)
            .map(|(k, e)| (k.clone(), e.last_accessed, e.size))
            .collect();

        sorted_entries.sort_by_key(|(_, accessed, _)| *accessed);

        let mut freed: u64 = 0;
        let mut evicted_keys = Vec::new();

        for (key, _, size) in sorted_entries {
            if freed >= bytes_needed {
                break;
            }

            evicted_keys.push(key);
            freed += size;
        }

        // Remove entries
        for key in evicted_keys {
            if let Some(entry) = entries.remove(&key) {
                if entry.path.exists() {
                    let _ = fs::remove_file(&entry.path).await;
                }
                self.current_size.fetch_sub(entry.size, Ordering::Relaxed);
                self.evictions.fetch_add(1, Ordering::Relaxed);
            }
        }

        if freed < bytes_needed {
            return Err(CacheError::CacheFull);
        }

        Ok(())
    }

    /// Rebuild the cache index from disk
    async fn rebuild_index(&self) -> CacheResult<()> {
        let mut entries = self.entries.write().await;
        entries.clear();

        let mut total_size: u64 = 0;
        let mut read_dir = fs::read_dir(&self.cache_dir).await?;

        while let Some(dir_entry) = read_dir.next_entry().await? {
            let path = dir_entry.path();

            if !path.is_file() {
                continue;
            }

            if let Ok(metadata) = fs::metadata(&path).await {
                let size = metadata.len();
                let file_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();

                if file_name.is_empty() {
                    continue;
                }

                let extension = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("mp3");

                let format = match extension {
                    "wav" => OutputFormat::Wav,
                    "ogg" => OutputFormat::Ogg,
                    "pcm" => OutputFormat::Pcm,
                    _ => OutputFormat::Mp3,
                };

                let entry = CacheEntry::new(file_name.clone(), path, size, format);
                total_size += size;
                entries.insert(file_name, entry);
            }
        }

        self.current_size.store(total_size, Ordering::Relaxed);

        Ok(())
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let entries = self.entries.read().await;

        let mut stats = CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
            entry_count: entries.len(),
            current_size_bytes: self.current_size.load(Ordering::Relaxed),
            max_size_bytes: self.config.max_size_bytes,
            entries_by_format: HashMap::new(),
            hit_rate: 0.0,
            oldest_entry_age_secs: 0,
            avg_entry_size_bytes: 0,
        };

        stats.calculate_hit_rate();

        // Calculate format distribution
        for entry in entries.values() {
            let format_name = format!("{:?}", entry.format);
            *stats.entries_by_format.entry(format_name).or_insert(0) += 1;

            let age = entry.age_seconds();
            if age > stats.oldest_entry_age_secs {
                stats.oldest_entry_age_secs = age;
            }
        }

        // Calculate average size
        if !entries.is_empty() {
            stats.avg_entry_size_bytes = stats.current_size_bytes / entries.len() as u64;
        }

        stats
    }

    /// List all cache entries
    pub async fn list_entries(&self) -> Vec<CacheEntry> {
        self.entries.read().await.values().cloned().collect()
    }

    /// Get entries by tag
    pub async fn entries_by_tag(&self, tag: &str) -> Vec<CacheEntry> {
        self.entries
            .read()
            .await
            .values()
            .filter(|e| e.has_tag(tag))
            .cloned()
            .collect()
    }

    /// Prune entries older than the specified age
    pub async fn prune_older_than(&self, max_age_secs: i64) -> CacheResult<usize> {
        let entries_to_remove: Vec<String> = {
            let entries = self.entries.read().await;
            entries
                .iter()
                .filter(|(_, e)| e.age_seconds() > max_age_secs)
                .map(|(k, _)| k.clone())
                .collect()
        };

        let count = entries_to_remove.len();
        for key in entries_to_remove {
            self.remove(&key).await?;
        }

        Ok(count)
    }

    /// Get the total number of entries
    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }

    /// Check if cache is empty
    pub async fn is_empty(&self) -> bool {
        self.entries.read().await.is_empty()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_cache() -> (AudioCache, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let cache = AudioCache::with_defaults(temp_dir.path().to_path_buf())
            .await
            .unwrap();
        (cache, temp_dir)
    }

    #[tokio::test]
    async fn test_cache_put_and_get() {
        let (cache, _temp) = create_test_cache().await;

        let key = "test-audio-1";
        let data = vec![0u8; 1024]; // 1KB of data

        let path = cache
            .put(key, &data, OutputFormat::Mp3, &[])
            .await
            .unwrap();

        assert!(path.exists());
        assert!(cache.contains(key).await);

        let retrieved = cache.get(key).await.unwrap();
        assert_eq!(retrieved, path);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let (cache, _temp) = create_test_cache().await;

        // Put some data
        cache
            .put("key1", &vec![0u8; 1024], OutputFormat::Mp3, &[])
            .await
            .unwrap();

        // Get it (hit)
        cache.get("key1").await;

        // Try to get non-existent (miss)
        cache.get("nonexistent").await;

        let stats = cache.stats().await;
        assert_eq!(stats.entry_count, 1);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.current_size_bytes, 1024);
    }

    #[tokio::test]
    async fn test_clear_by_tag() {
        let (cache, _temp) = create_test_cache().await;

        cache
            .put(
                "session1-audio",
                &vec![0u8; 100],
                OutputFormat::Mp3,
                &["session:123".to_string()],
            )
            .await
            .unwrap();

        cache
            .put(
                "session2-audio",
                &vec![0u8; 100],
                OutputFormat::Mp3,
                &["session:456".to_string()],
            )
            .await
            .unwrap();

        assert_eq!(cache.len().await, 2);

        let removed = cache.clear_by_tag("session:123").await.unwrap();
        assert_eq!(removed, 1);
        assert_eq!(cache.len().await, 1);
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let settings = VoiceSettings::default();

        let params1 = CacheKeyParams::new(
            "Hello world",
            VoiceProviderType::OpenAI,
            "alloy",
            &settings,
            OutputFormat::Mp3,
        );

        let params2 = CacheKeyParams::new(
            "Hello world",
            VoiceProviderType::OpenAI,
            "alloy",
            &settings,
            OutputFormat::Mp3,
        );

        let params3 = CacheKeyParams::new(
            "Different text",
            VoiceProviderType::OpenAI,
            "alloy",
            &settings,
            OutputFormat::Mp3,
        );

        // Same params should produce same key
        assert_eq!(params1.to_key(), params2.to_key());

        // Different text should produce different key
        assert_ne!(params1.to_key(), params3.to_key());
    }

    #[tokio::test]
    async fn test_remove() {
        let (cache, _temp) = create_test_cache().await;

        cache
            .put("to-remove", &vec![0u8; 100], OutputFormat::Mp3, &[])
            .await
            .unwrap();

        assert!(cache.contains("to-remove").await);

        cache.remove("to-remove").await.unwrap();

        assert!(!cache.contains("to-remove").await);
    }
}
