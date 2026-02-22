//! Cache management for the Archetype Registry.
//!
//! This module provides an LRU cache with dependency tracking for resolved
//! archetypes. It supports intelligent cache invalidation based on archetype
//! modifications and campaign context changes.
//!
//! # Architecture
//!
//! ```text
//!                   CacheManager
//!                        |
//!     +------------------+------------------+
//!     |                  |                  |
//!     v                  v                  v
//!  LRU Cache      Dependency Index     Cache Stats
//!  (entries)      (invalidation)       (metrics)
//! ```
//!
//! # Dependency Tracking
//!
//! The cache maintains a dependency index that maps:
//! - Archetype IDs to cache entries that depend on them
//! - Query components (role, race, class, setting) to affected entries
//! - Campaign IDs to entries that use that campaign context
//!
//! This allows for targeted invalidation when archetypes are modified.
//!
//! # Thread Safety (CRITICAL-ARCH-002)
//!
//! All mutable state is protected by `tokio::sync::RwLock` for async-safe
//! access in the Tauri async command context.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::core::archetype::cache::{CacheManager, CacheConfig};
//!
//! let config = CacheConfig::default();
//! let cache = CacheManager::new(config);
//!
//! // Store a resolved archetype
//! cache.put(query, resolved).await;
//!
//! // Retrieve from cache
//! if let Some(cached) = cache.get(&query).await {
//!     println!("Cache hit!");
//! }
//!
//! // Invalidate when archetype changes
//! cache.invalidate_for_archetype("dwarf").await;
//! ```

use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use lru::LruCache;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::resolution::{ResolvedArchetype, ResolutionQuery};

// ============================================================================
// Constants
// ============================================================================

/// Default cache capacity (256 entries per design spec).
pub const DEFAULT_CACHE_CAPACITY: usize = 256;

/// Default TTL for cache entries (3600 seconds = 1 hour per design spec).
pub const DEFAULT_TTL_SECONDS: u64 = 3600;

// ============================================================================
// CacheConfig - Configuration for cache behavior
// ============================================================================

/// Configuration options for the cache manager.
///
/// # Defaults
///
/// - `capacity`: 256 entries
/// - `ttl_seconds`: 3600 (1 hour)
/// - `track_dependencies`: true
/// - `stale_while_revalidate`: true
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheConfig {
    /// Maximum number of entries in the cache.
    ///
    /// When exceeded, least recently used entries are evicted.
    #[serde(default = "default_capacity")]
    pub capacity: usize,

    /// Time-to-live for cache entries in seconds.
    ///
    /// Entries older than this are considered stale.
    /// Set to 0 to disable TTL-based expiration.
    #[serde(default = "default_ttl")]
    pub ttl_seconds: u64,

    /// Enable dependency tracking for smart invalidation.
    ///
    /// When enabled, the cache tracks which archetypes and query
    /// components contribute to each cached result, allowing for
    /// targeted invalidation.
    #[serde(default = "default_track_dependencies")]
    pub track_dependencies: bool,

    /// Enable stale-while-revalidate behavior.
    ///
    /// When enabled, stale entries are served during invalidation
    /// while fresh data is being computed in the background.
    #[serde(default = "default_stale_while_revalidate")]
    pub stale_while_revalidate: bool,
}

fn default_capacity() -> usize {
    DEFAULT_CACHE_CAPACITY
}

fn default_ttl() -> u64 {
    DEFAULT_TTL_SECONDS
}

fn default_track_dependencies() -> bool {
    true
}

fn default_stale_while_revalidate() -> bool {
    true
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            capacity: DEFAULT_CACHE_CAPACITY,
            ttl_seconds: DEFAULT_TTL_SECONDS,
            track_dependencies: true,
            stale_while_revalidate: true,
        }
    }
}

impl CacheConfig {
    /// Create a new config with custom capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            ..Default::default()
        }
    }

    /// Create a minimal config without dependency tracking.
    ///
    /// Useful for testing or low-memory environments.
    pub fn minimal() -> Self {
        Self {
            capacity: 64,
            ttl_seconds: 0,
            track_dependencies: false,
            stale_while_revalidate: false,
        }
    }

    /// Builder method to set TTL.
    pub fn ttl(mut self, seconds: u64) -> Self {
        self.ttl_seconds = seconds;
        self
    }

    /// Builder method to enable/disable dependency tracking.
    pub fn tracking(mut self, enabled: bool) -> Self {
        self.track_dependencies = enabled;
        self
    }
}

// ============================================================================
// CacheEntry - Internal entry wrapper with metadata
// ============================================================================

/// Internal cache entry with timestamp and stale marker.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// The resolved archetype data.
    resolved: ResolvedArchetype,

    /// When this entry was created.
    created_at: Instant,

    /// Whether this entry has been marked stale but not yet evicted.
    is_stale: bool,

    /// Source archetypes that contributed to this resolution.
    /// Used for dependency-based invalidation.
    #[allow(dead_code)]
    source_archetypes: Vec<String>,
}

impl CacheEntry {
    fn new(resolved: ResolvedArchetype, sources: Vec<String>) -> Self {
        Self {
            resolved,
            created_at: Instant::now(),
            is_stale: false,
            source_archetypes: sources,
        }
    }

    fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        if ttl.is_zero() {
            false
        } else {
            self.age() > ttl
        }
    }
}

// ============================================================================
// CacheStats - Statistics for monitoring
// ============================================================================

/// Statistics about cache performance.
///
/// These metrics help monitor cache effectiveness and can be used
/// for tuning cache configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    /// Number of cache hits (successful lookups).
    pub hits: u64,

    /// Number of cache misses (failed lookups).
    pub misses: u64,

    /// Number of entries evicted due to capacity limits.
    pub evictions: u64,

    /// Number of entries invalidated explicitly.
    pub invalidations: u64,

    /// Number of stale entries served (stale-while-revalidate).
    pub stale_hits: u64,

    /// Current number of entries in the cache.
    pub current_size: usize,

    /// Maximum capacity of the cache.
    pub capacity: usize,
}

impl CacheStats {
    /// Calculate the cache hit rate as a percentage.
    ///
    /// Returns 0.0 if no lookups have been performed.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let stats = cache.stats().await;
    /// println!("Hit rate: {:.1}%", stats.hit_rate() * 100.0);
    /// ```
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Calculate utilization as a percentage of capacity.
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            self.current_size as f64 / self.capacity as f64
        }
    }
}

// ============================================================================
// DependencyIndex - Tracks what cache entries depend on what archetypes
// ============================================================================

/// Index for tracking cache entry dependencies.
///
/// Maps various identifiers to the set of cache keys that depend on them.
#[derive(Debug, Default)]
struct DependencyIndex {
    /// Maps archetype_id -> Set<cache_key>
    by_archetype: HashMap<String, HashSet<String>>,

    /// Maps "role:X" -> Set<cache_key>
    by_role: HashMap<String, HashSet<String>>,

    /// Maps "race:X" -> Set<cache_key>
    by_race: HashMap<String, HashSet<String>>,

    /// Maps "class:X" -> Set<cache_key>
    by_class: HashMap<String, HashSet<String>>,

    /// Maps "setting:X" -> Set<cache_key>
    by_setting: HashMap<String, HashSet<String>>,

    /// Maps campaign_id -> Set<cache_key>
    by_campaign: HashMap<String, HashSet<String>>,

    /// Maps vocabulary_bank_id -> Set<cache_key>
    by_vocabulary_bank: HashMap<String, HashSet<String>>,
}

impl DependencyIndex {
    fn new() -> Self {
        Self::default()
    }

    /// Index a cache entry by all its dependencies.
    fn index_entry(&mut self, cache_key: &str, query: &ResolutionQuery, sources: &[String]) {
        // Index by source archetypes
        for source in sources {
            self.by_archetype
                .entry(source.clone())
                .or_default()
                .insert(cache_key.to_string());
        }

        // Index by query components
        if let Some(ref role) = query.npc_role {
            self.by_role
                .entry(role.clone())
                .or_default()
                .insert(cache_key.to_string());
        }

        if let Some(ref race) = query.race {
            self.by_race
                .entry(race.clone())
                .or_default()
                .insert(cache_key.to_string());
        }

        if let Some(ref class) = query.class {
            self.by_class
                .entry(class.clone())
                .or_default()
                .insert(cache_key.to_string());
        }

        if let Some(ref setting) = query.setting {
            self.by_setting
                .entry(setting.clone())
                .or_default()
                .insert(cache_key.to_string());
        }

        if let Some(ref campaign_id) = query.campaign_id {
            self.by_campaign
                .entry(campaign_id.clone())
                .or_default()
                .insert(cache_key.to_string());
        }

        // Index by direct archetype ID
        if let Some(ref archetype_id) = query.archetype_id {
            self.by_archetype
                .entry(archetype_id.clone())
                .or_default()
                .insert(cache_key.to_string());
        }
    }

    /// Remove a cache key from all indexes.
    fn remove_entry(&mut self, cache_key: &str) {
        for set in self.by_archetype.values_mut() {
            set.remove(cache_key);
        }
        for set in self.by_role.values_mut() {
            set.remove(cache_key);
        }
        for set in self.by_race.values_mut() {
            set.remove(cache_key);
        }
        for set in self.by_class.values_mut() {
            set.remove(cache_key);
        }
        for set in self.by_setting.values_mut() {
            set.remove(cache_key);
        }
        for set in self.by_campaign.values_mut() {
            set.remove(cache_key);
        }
        for set in self.by_vocabulary_bank.values_mut() {
            set.remove(cache_key);
        }
    }

    /// Get all cache keys affected by an archetype change.
    fn get_affected_by_archetype(&self, archetype_id: &str) -> HashSet<String> {
        self.by_archetype
            .get(archetype_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all cache keys for a campaign.
    fn get_affected_by_campaign(&self, campaign_id: &str) -> HashSet<String> {
        self.by_campaign
            .get(campaign_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all cache keys using a vocabulary bank.
    fn get_affected_by_vocabulary_bank(&self, bank_id: &str) -> HashSet<String> {
        self.by_vocabulary_bank
            .get(bank_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Clear all indexes.
    fn clear(&mut self) {
        self.by_archetype.clear();
        self.by_role.clear();
        self.by_race.clear();
        self.by_class.clear();
        self.by_setting.clear();
        self.by_campaign.clear();
        self.by_vocabulary_bank.clear();
    }
}

// ============================================================================
// CacheManager - Main cache implementation
// ============================================================================

/// Manages the resolution cache with LRU eviction and dependency tracking.
///
/// The CacheManager provides thread-safe caching for resolved archetypes
/// with support for:
///
/// - LRU eviction when capacity is exceeded
/// - TTL-based expiration
/// - Dependency-based targeted invalidation
/// - Stale-while-revalidate for graceful invalidation
///
/// # Thread Safety
///
/// All access is protected by `tokio::sync::RwLock` for async safety.
///
/// # Example
///
/// ```rust,ignore
/// let config = CacheConfig::default();
/// let cache = CacheManager::new(config);
///
/// // Store a resolved archetype
/// cache.put(&query, resolved, vec!["dwarf".to_string()]).await;
///
/// // Retrieve from cache
/// if let Some(cached) = cache.get(&query).await {
///     println!("Hit: {:?}", cached);
/// }
///
/// // Invalidate when dwarf archetype changes
/// cache.invalidate_for_archetype("dwarf").await;
/// ```
pub struct CacheManager {
    /// LRU cache for resolved archetypes.
    cache: RwLock<LruCache<String, CacheEntry>>,

    /// Dependency index for targeted invalidation.
    dependency_index: RwLock<DependencyIndex>,

    /// Cache statistics.
    stats: RwLock<CacheStats>,

    /// Configuration.
    config: CacheConfig,
}

impl CacheManager {
    /// Create a new cache manager with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Cache configuration options
    ///
    /// # Panics
    ///
    /// Panics if capacity is 0 (should not happen with default config).
    pub fn new(config: CacheConfig) -> Self {
        let capacity = config.capacity.max(1);

        Self {
            cache: RwLock::new(LruCache::new(
                NonZeroUsize::new(capacity).expect("capacity must be > 0"),
            )),
            dependency_index: RwLock::new(DependencyIndex::new()),
            stats: RwLock::new(CacheStats {
                capacity,
                ..Default::default()
            }),
            config,
        }
    }

    /// Create a cache manager with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(CacheConfig::default())
    }

    /// Get a resolved archetype from the cache.
    ///
    /// Updates hit/miss statistics and handles TTL expiration.
    ///
    /// # Arguments
    ///
    /// * `query` - The resolution query to look up
    ///
    /// # Returns
    ///
    /// `Some(ResolvedArchetype)` if found and not expired, `None` otherwise.
    pub async fn get(&self, query: &ResolutionQuery) -> Option<ResolvedArchetype> {
        let cache_key = query.cache_key();
        let ttl = Duration::from_secs(self.config.ttl_seconds);

        let mut cache = self.cache.write().await;

        // Check if entry exists
        let entry = match cache.get(&cache_key) {
            Some(e) => e,
            None => {
                // Record miss for non-existent entry
                let mut stats = self.stats.write().await;
                stats.misses += 1;
                return None;
            }
        };

        // Check TTL expiration
        if entry.is_expired(ttl) {
            // If stale-while-revalidate is enabled and entry is just stale,
            // return it anyway
            if self.config.stale_while_revalidate && !entry.is_stale {
                let mut stats = self.stats.write().await;
                stats.stale_hits += 1;
                return Some(entry.resolved.clone());
            }

            // Entry is expired and stale-while-revalidate doesn't apply
            let mut stats = self.stats.write().await;
            stats.misses += 1;
            drop(stats);
            drop(cache);

            // Remove the expired entry
            self.remove_entry(&cache_key).await;
            return None;
        }

        // Valid cache hit
        let mut stats = self.stats.write().await;
        stats.hits += 1;

        Some(entry.resolved.clone())
    }

    /// Store a resolved archetype in the cache.
    ///
    /// # Arguments
    ///
    /// * `query` - The resolution query (used as cache key)
    /// * `resolved` - The resolved archetype to cache
    /// * `source_archetypes` - IDs of archetypes that contributed to this result
    ///
    /// # Dependency Tracking
    ///
    /// If dependency tracking is enabled, this method indexes the entry by:
    /// - Source archetypes
    /// - Query components (role, race, class, setting)
    /// - Campaign ID
    pub async fn put(
        &self,
        query: &ResolutionQuery,
        resolved: ResolvedArchetype,
        source_archetypes: Vec<String>,
    ) {
        let cache_key = query.cache_key();

        // Track dependencies if enabled
        if self.config.track_dependencies {
            let mut deps = self.dependency_index.write().await;
            deps.index_entry(&cache_key, query, &source_archetypes);
        }

        let entry = CacheEntry::new(resolved, source_archetypes);

        let mut cache = self.cache.write().await;

        // Use push to detect eviction
        let result = cache.push(cache_key.clone(), entry);

        if let Some((evicted_key, _)) = result {
            // If the evicted key is different from the inserted key, it was an eviction due to capacity
            if evicted_key != cache_key {
                let mut stats = self.stats.write().await;
                stats.evictions += 1;
            }
        }

        // Update size stat
        let mut stats = self.stats.write().await;
        stats.current_size = cache.len();
    }

    /// Invalidate all cache entries that depend on a specific archetype.
    ///
    /// This is called when an archetype is modified or deleted.
    ///
    /// # Arguments
    ///
    /// * `archetype_id` - The ID of the archetype that changed
    ///
    /// # Behavior
    ///
    /// - With dependency tracking: removes only affected entries
    /// - Without dependency tracking: clears the entire cache
    pub async fn invalidate_for_archetype(&self, archetype_id: &str) {
        if !self.config.track_dependencies {
            // Without tracking, fall back to full cache clear
            self.clear().await;
            return;
        }

        let affected = {
            let deps = self.dependency_index.read().await;
            deps.get_affected_by_archetype(archetype_id)
        };

        if affected.is_empty() {
            return;
        }

        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        if self.config.stale_while_revalidate {
            // Mark entries as stale instead of removing immediately
            for key in &affected {
                if let Some(entry) = cache.get_mut(key) {
                    entry.is_stale = true;
                }
            }
        } else {
            // Remove affected entries
            for key in affected {
                if cache.pop(&key).is_some() {
                    stats.invalidations += 1;
                }
            }
        }

        stats.current_size = cache.len();
    }

    /// Invalidate all cache entries for a campaign.
    ///
    /// This is called when a setting pack is activated or deactivated
    /// for a campaign.
    ///
    /// # Arguments
    ///
    /// * `campaign_id` - The ID of the campaign whose context changed
    pub async fn invalidate_for_campaign(&self, campaign_id: &str) {
        if !self.config.track_dependencies {
            self.clear().await;
            return;
        }

        let affected = {
            let deps = self.dependency_index.read().await;
            deps.get_affected_by_campaign(campaign_id)
        };

        if affected.is_empty() {
            return;
        }

        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        if self.config.stale_while_revalidate {
            for key in &affected {
                if let Some(entry) = cache.get_mut(key) {
                    entry.is_stale = true;
                }
            }
        } else {
            for key in affected {
                if cache.pop(&key).is_some() {
                    stats.invalidations += 1;
                }
            }
        }

        stats.current_size = cache.len();
    }

    /// Invalidate all cache entries using a specific vocabulary bank.
    ///
    /// This is called when a vocabulary bank is modified or deleted.
    ///
    /// # Arguments
    ///
    /// * `bank_id` - The ID of the vocabulary bank that changed
    pub async fn invalidate_for_vocabulary_bank(&self, bank_id: &str) {
        if !self.config.track_dependencies {
            self.clear().await;
            return;
        }

        let affected = {
            let deps = self.dependency_index.read().await;
            deps.get_affected_by_vocabulary_bank(bank_id)
        };

        if affected.is_empty() {
            return;
        }

        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        if self.config.stale_while_revalidate {
            for key in &affected {
                if let Some(entry) = cache.get_mut(key) {
                    entry.is_stale = true;
                }
            }
        } else {
            for key in affected {
                if cache.pop(&key).is_some() {
                    stats.invalidations += 1;
                }
            }
        }

        stats.current_size = cache.len();
    }

    /// Get current cache statistics.
    ///
    /// Returns a snapshot of cache performance metrics.
    pub async fn stats(&self) -> CacheStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get the cache hit rate as a percentage.
    ///
    /// Convenience method for `stats().hit_rate()`.
    pub async fn hit_rate(&self) -> f64 {
        let stats = self.stats.read().await;
        stats.hit_rate()
    }

    /// Clear the entire cache and reset statistics.
    ///
    /// This removes all entries and clears the dependency index.
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();

        if self.config.track_dependencies {
            let mut deps = self.dependency_index.write().await;
            deps.clear();
        }

        let mut stats = self.stats.write().await;
        stats.current_size = 0;
        // Don't reset hits/misses/evictions - they're cumulative
    }

    /// Check if the cache contains an entry for the given query.
    ///
    /// Does not count as a hit or miss in statistics.
    pub async fn contains(&self, query: &ResolutionQuery) -> bool {
        let cache_key = query.cache_key();
        let cache = self.cache.read().await;
        cache.contains(&cache_key)
    }

    /// Get the current number of entries in the cache.
    pub async fn len(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Check if the cache is empty.
    pub async fn is_empty(&self) -> bool {
        let cache = self.cache.read().await;
        cache.is_empty()
    }

    /// Get the cache capacity.
    pub fn capacity(&self) -> usize {
        self.config.capacity
    }

    // ========================================================================
    // Internal helpers
    // ========================================================================

    /// Remove an entry and clean up its dependency index entries.
    async fn remove_entry(&self, cache_key: &str) {
        let mut cache = self.cache.write().await;
        cache.pop(cache_key);

        if self.config.track_dependencies {
            let mut deps = self.dependency_index.write().await;
            deps.remove_entry(cache_key);
        }

        let mut stats = self.stats.write().await;
        stats.current_size = cache.len();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // CacheConfig tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.capacity, DEFAULT_CACHE_CAPACITY);
        assert_eq!(config.ttl_seconds, DEFAULT_TTL_SECONDS);
        assert!(config.track_dependencies);
        assert!(config.stale_while_revalidate);
    }

    #[test]
    fn test_cache_config_with_capacity() {
        let config = CacheConfig::with_capacity(512);
        assert_eq!(config.capacity, 512);
        assert_eq!(config.ttl_seconds, DEFAULT_TTL_SECONDS);
    }

    #[test]
    fn test_cache_config_minimal() {
        let config = CacheConfig::minimal();
        assert_eq!(config.capacity, 64);
        assert_eq!(config.ttl_seconds, 0);
        assert!(!config.track_dependencies);
        assert!(!config.stale_while_revalidate);
    }

    #[test]
    fn test_cache_config_builder() {
        let config = CacheConfig::default().ttl(600).tracking(false);
        assert_eq!(config.ttl_seconds, 600);
        assert!(!config.track_dependencies);
    }

    // -------------------------------------------------------------------------
    // CacheStats tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_cache_stats_hit_rate_zero() {
        let stats = CacheStats::default();
        assert_eq!(stats.hit_rate(), 0.0);
    }

    #[test]
    fn test_cache_stats_hit_rate_calculation() {
        let stats = CacheStats {
            hits: 75,
            misses: 25,
            ..Default::default()
        };
        assert!((stats.hit_rate() - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_cache_stats_utilization() {
        let stats = CacheStats {
            current_size: 128,
            capacity: 256,
            ..Default::default()
        };
        assert!((stats.utilization() - 0.5).abs() < 0.001);
    }

    // -------------------------------------------------------------------------
    // CacheEntry tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_cache_entry_creation() {
        let resolved = ResolvedArchetype::new();
        let entry = CacheEntry::new(resolved, vec!["dwarf".to_string()]);

        assert!(!entry.is_stale);
        assert_eq!(entry.source_archetypes, vec!["dwarf"]);
    }

    #[test]
    fn test_cache_entry_ttl_disabled() {
        let resolved = ResolvedArchetype::new();
        let entry = CacheEntry::new(resolved, vec![]);

        // Zero TTL = disabled
        assert!(!entry.is_expired(Duration::ZERO));
    }

    #[test]
    fn test_cache_entry_not_expired() {
        let resolved = ResolvedArchetype::new();
        let entry = CacheEntry::new(resolved, vec![]);

        // 1 hour TTL, just created
        assert!(!entry.is_expired(Duration::from_secs(3600)));
    }

    // -------------------------------------------------------------------------
    // DependencyIndex tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_dependency_index_archetype() {
        let mut index = DependencyIndex::new();

        let query = ResolutionQuery::single("dwarf_merchant");
        let cache_key = query.cache_key();

        index.index_entry(&cache_key, &query, &["dwarf".to_string(), "merchant".to_string()]);

        let affected = index.get_affected_by_archetype("dwarf");
        assert!(affected.contains(&cache_key));

        let affected = index.get_affected_by_archetype("merchant");
        assert!(affected.contains(&cache_key));
    }

    #[test]
    fn test_dependency_index_query_components() {
        let mut index = DependencyIndex::new();

        let query = ResolutionQuery::for_npc("merchant")
            .with_race("dwarf")
            .with_campaign("campaign_1");
        let cache_key = query.cache_key();

        index.index_entry(&cache_key, &query, &[]);

        // Check role index
        let affected = index.by_role.get("merchant");
        assert!(affected.is_some());
        assert!(affected.unwrap().contains(&cache_key));

        // Check race index
        let affected = index.by_race.get("dwarf");
        assert!(affected.is_some());
        assert!(affected.unwrap().contains(&cache_key));

        // Check campaign index
        let affected = index.get_affected_by_campaign("campaign_1");
        assert!(affected.contains(&cache_key));
    }

    #[test]
    fn test_dependency_index_remove_entry() {
        let mut index = DependencyIndex::new();

        let query = ResolutionQuery::single("dwarf");
        let cache_key = query.cache_key();

        index.index_entry(&cache_key, &query, &["dwarf".to_string()]);

        // Verify indexed
        let affected = index.get_affected_by_archetype("dwarf");
        assert!(affected.contains(&cache_key));

        // Remove
        index.remove_entry(&cache_key);

        // Verify removed
        let affected = index.get_affected_by_archetype("dwarf");
        assert!(!affected.contains(&cache_key));
    }

    #[test]
    fn test_dependency_index_clear() {
        let mut index = DependencyIndex::new();

        index.index_entry(
            "key1",
            &ResolutionQuery::single("dwarf"),
            &["dwarf".to_string()],
        );
        index.index_entry(
            "key2",
            &ResolutionQuery::for_npc("merchant"),
            &["merchant".to_string()],
        );

        index.clear();

        assert!(index.by_archetype.values().all(|s| s.is_empty()));
        assert!(index.by_role.values().all(|s| s.is_empty()));
    }

    // -------------------------------------------------------------------------
    // CacheManager tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_cache_manager_creation() {
        let cache = CacheManager::new(CacheConfig::default());
        assert_eq!(cache.capacity(), DEFAULT_CACHE_CAPACITY);
        assert!(cache.is_empty().await);
    }

    #[tokio::test]
    async fn test_cache_manager_put_and_get() {
        let cache = CacheManager::new(CacheConfig::default());
        let query = ResolutionQuery::single("dwarf");
        let resolved = ResolvedArchetype::with_id("dwarf");

        // Put
        cache.put(&query, resolved.clone(), vec!["dwarf".to_string()]).await;

        // Get
        let cached = cache.get(&query).await;
        assert!(cached.is_some());
        assert_eq!(
            cached.unwrap().id.as_ref().unwrap().as_str(),
            "dwarf"
        );
    }

    #[tokio::test]
    async fn test_cache_manager_miss() {
        let cache = CacheManager::new(CacheConfig::default());
        let query = ResolutionQuery::single("nonexistent");

        let cached = cache.get(&query).await;
        assert!(cached.is_none());

        let stats = cache.stats().await;
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_cache_manager_hit_stats() {
        let cache = CacheManager::new(CacheConfig::default());
        let query = ResolutionQuery::single("elf");
        let resolved = ResolvedArchetype::with_id("elf");

        cache.put(&query, resolved, vec![]).await;
        let _ = cache.get(&query).await;
        let _ = cache.get(&query).await;

        let stats = cache.stats().await;
        assert_eq!(stats.hits, 2);
    }

    #[tokio::test]
    async fn test_cache_manager_invalidate_for_archetype() {
        let config = CacheConfig::default().tracking(true);
        let cache = CacheManager::new(config);

        // Store entries that depend on "dwarf"
        let query1 = ResolutionQuery::single("dwarf");
        cache
            .put(
                &query1,
                ResolvedArchetype::with_id("dwarf"),
                vec!["dwarf".to_string()],
            )
            .await;

        let query2 = ResolutionQuery::for_npc("merchant").with_race("dwarf");
        cache
            .put(
                &query2,
                ResolvedArchetype::with_id("dwarf_merchant"),
                vec!["dwarf".to_string(), "merchant".to_string()],
            )
            .await;

        // Store an entry that doesn't depend on "dwarf"
        let query3 = ResolutionQuery::single("elf");
        cache
            .put(
                &query3,
                ResolvedArchetype::with_id("elf"),
                vec!["elf".to_string()],
            )
            .await;

        assert_eq!(cache.len().await, 3);

        // Invalidate "dwarf" - with stale_while_revalidate, entries are marked stale not removed
        cache.invalidate_for_archetype("dwarf").await;

        // The elf entry should still be valid
        assert!(cache.get(&query3).await.is_some());
    }

    #[tokio::test]
    async fn test_cache_manager_invalidate_for_campaign() {
        let config = CacheConfig::default().tracking(true);
        let mut mutable_config = config.clone();
        mutable_config.stale_while_revalidate = false;
        let cache = CacheManager::new(mutable_config);

        // Store entries with campaign context
        let query1 = ResolutionQuery::for_npc("guard").with_campaign("campaign_1");
        cache
            .put(&query1, ResolvedArchetype::with_id("guard"), vec![])
            .await;

        let query2 = ResolutionQuery::for_npc("merchant").with_campaign("campaign_2");
        cache
            .put(&query2, ResolvedArchetype::with_id("merchant"), vec![])
            .await;

        assert_eq!(cache.len().await, 2);

        // Invalidate campaign_1
        cache.invalidate_for_campaign("campaign_1").await;

        // campaign_1 entry should be gone
        assert!(cache.get(&query1).await.is_none());

        // campaign_2 entry should remain
        assert!(cache.get(&query2).await.is_some());
    }

    #[tokio::test]
    async fn test_cache_manager_clear() {
        let cache = CacheManager::new(CacheConfig::default());

        cache
            .put(
                &ResolutionQuery::single("dwarf"),
                ResolvedArchetype::with_id("dwarf"),
                vec![],
            )
            .await;
        cache
            .put(
                &ResolutionQuery::single("elf"),
                ResolvedArchetype::with_id("elf"),
                vec![],
            )
            .await;

        assert_eq!(cache.len().await, 2);

        cache.clear().await;

        assert!(cache.is_empty().await);
    }

    #[tokio::test]
    async fn test_cache_manager_contains() {
        let cache = CacheManager::new(CacheConfig::default());
        let query = ResolutionQuery::single("dwarf");

        assert!(!cache.contains(&query).await);

        cache
            .put(&query, ResolvedArchetype::with_id("dwarf"), vec![])
            .await;

        assert!(cache.contains(&query).await);
    }

    #[tokio::test]
    async fn test_cache_manager_without_tracking() {
        let config = CacheConfig::minimal();
        let cache = CacheManager::new(config);

        // Store some entries
        cache
            .put(
                &ResolutionQuery::single("dwarf"),
                ResolvedArchetype::with_id("dwarf"),
                vec![],
            )
            .await;
        cache
            .put(
                &ResolutionQuery::single("elf"),
                ResolvedArchetype::with_id("elf"),
                vec![],
            )
            .await;

        assert_eq!(cache.len().await, 2);

        // Without tracking, invalidation clears entire cache
        cache.invalidate_for_archetype("dwarf").await;

        assert!(cache.is_empty().await);
    }

    #[tokio::test]
    async fn test_cache_manager_eviction_stats() {
        let config = CacheConfig::with_capacity(2);
        let cache = CacheManager::new(config);

        cache
            .put(
                &ResolutionQuery::single("a"),
                ResolvedArchetype::with_id("a"),
                vec![],
            )
            .await;
        cache
            .put(
                &ResolutionQuery::single("b"),
                ResolvedArchetype::with_id("b"),
                vec![],
            )
            .await;

        // This should cause eviction
        cache
            .put(
                &ResolutionQuery::single("c"),
                ResolvedArchetype::with_id("c"),
                vec![],
            )
            .await;

        let stats = cache.stats().await;
        assert_eq!(stats.evictions, 1);
    }
}
