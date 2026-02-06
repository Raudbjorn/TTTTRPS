//! Core Registry structure for the Archetype system.
//!
//! The [`ArchetypeRegistry`] is the central coordination point for all archetype
//! operations. It manages:
//!
//! - In-memory archetype storage with thread-safe access
//! - Setting pack registration and activation
//! - Meilisearch persistence for search and discovery
//! - Cache management for resolved archetypes
//!
//! # Thread Safety
//!
//! All mutable state is protected by `tokio::sync::RwLock` for async-safe access.
//! This is critical for proper operation in the Tauri async command context.
//!
//! # Dual-Client Architecture
//!
//! The registry uses two Meilisearch access paths:
//! - **`MeilisearchLib` (embedded)**: For index management (create, configure, delete)
//!   via [`ArchetypeIndexManager`]. No HTTP overhead.
//! - **`meilisearch_sdk::Client`**: For document CRUD operations (add, search, delete
//!   documents). Connects to the embedded instance's HTTP interface.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::core::archetype::{ArchetypeRegistry, Archetype, ArchetypeCategory};
//! use crate::core::search::EmbeddedSearch;
//! use meilisearch_sdk::Client;
//!
//! let search = EmbeddedSearch::new(db_path)?;
//! let client = Client::new("http://localhost:7700", None::<String>)?;
//! let registry = ArchetypeRegistry::new(search.clone_inner(), client).await?;
//!
//! // Register an archetype
//! let archetype = Archetype::new("knight", "Knight", ArchetypeCategory::Class);
//! registry.register(archetype).await?;
//!
//! // Get by ID
//! let knight = registry.get("knight").await?;
//!
//! // List all archetypes
//! let all = registry.list(None).await;
//! ```

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;

use lru::LruCache;
use meilisearch_lib::MeilisearchLib;
use meilisearch_sdk::client::Client;
use tokio::sync::RwLock;

use super::error::{ArchetypeError, Result};
use super::meilisearch::{ArchetypeIndexManager, INDEX_ARCHETYPES};
use super::resolution::{ResolutionQuery, ResolvedArchetype};
use super::setting_pack::{SettingPack, SettingPackSummary};
use super::types::{Archetype, ArchetypeCategory, ArchetypeId, ArchetypeSummary};

// ============================================================================
// Constants
// ============================================================================

/// Default cache capacity for resolved archetypes.
const DEFAULT_CACHE_CAPACITY: usize = 256;

/// Timeout for Meilisearch task completion (30 seconds).
const TASK_TIMEOUT_SECS: u64 = 30;

/// Polling interval for task completion checks (100ms).
const TASK_POLL_INTERVAL_MS: u64 = 100;

// ============================================================================
// Event Types (stubs for future event system)
// ============================================================================

/// Events emitted by the registry for external systems to react to.
#[derive(Debug, Clone)]
pub enum ArchetypeEvent {
    /// A new archetype was created.
    Created { id: ArchetypeId },

    /// An existing archetype was modified.
    Modified {
        id: ArchetypeId,
        /// IDs of child archetypes that may be affected.
        affected_children: Vec<ArchetypeId>,
    },

    /// An archetype was deleted.
    Deleted { id: ArchetypeId },

    /// A setting pack was activated for a campaign.
    SettingPackActivated {
        pack_id: String,
        campaign_id: String,
    },

    /// A setting pack was deactivated.
    SettingPackDeactivated {
        pack_id: String,
        campaign_id: String,
    },
}

// ============================================================================
// ArchetypeRegistry
// ============================================================================

/// Central registry for archetype data coordination.
///
/// The registry provides thread-safe access to archetypes and manages their
/// lifecycle including persistence to Meilisearch.
///
/// # Architecture
///
/// ```text
///                   ArchetypeRegistry
///                         |
///     +-------------------+-------------------+
///     |                   |                   |
/// archetypes         setting_packs       active_packs
/// (HashMap)          (HashMap)           (HashMap)
///     |                   |                   |
///     +-------------------+-------------------+
///                         |
///                   Meilisearch
///                   (persistence)
/// ```
///
/// # Thread Safety (CRITICAL-ARCH-002)
///
/// All async-accessed fields use `tokio::sync::RwLock`, NOT `std::sync::RwLock`.
/// This is critical for proper async operation and to prevent deadlocks.
pub struct ArchetypeRegistry {
    /// All registered archetypes indexed by ID.
    ///
    /// Protected by `tokio::sync::RwLock` for async-safe access.
    archetypes: Arc<RwLock<HashMap<String, Archetype>>>,

    /// Loaded setting packs (not necessarily active).
    ///
    /// Packs are loaded and validated before activation.
    setting_packs: Arc<RwLock<HashMap<String, SettingPack>>>,

    /// Active setting pack per campaign.
    ///
    /// Maps campaign_id -> pack_id.
    active_packs: Arc<RwLock<HashMap<String, String>>>,

    /// Resolution cache with LRU eviction.
    ///
    /// Caches resolved archetypes to avoid repeated resolution.
    cache: Arc<RwLock<LruCache<String, ResolvedArchetype>>>,

    /// Meilisearch client for persistence and search.
    meilisearch_client: Client,

    /// Event listeners (stub for future event system).
    event_listeners: Arc<RwLock<Vec<Box<dyn Fn(ArchetypeEvent) + Send + Sync>>>>,
}

impl ArchetypeRegistry {
    /// Create a new registry with a Meilisearch client.
    ///
    /// This constructor:
    /// 1. Ensures required Meilisearch indexes exist
    /// 2. Loads existing archetypes from Meilisearch
    /// 3. Initializes the LRU cache
    ///
    /// # Arguments
    ///
    /// * `meili` - Shared reference to embedded MeilisearchLib (for index management)
    /// * `meilisearch_client` - Initialized Meilisearch SDK client (for document CRUD)
    ///
    /// # Errors
    ///
    /// Returns `ArchetypeError::Meilisearch` if index creation or loading fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let search = EmbeddedSearch::new(db_path)?;
    /// let client = Client::new("http://localhost:7700", None::<String>)?;
    /// let registry = ArchetypeRegistry::new(search.clone_inner(), client).await?;
    /// ```
    pub async fn new(meili: Arc<MeilisearchLib>, meilisearch_client: Client) -> Result<Self> {
        Self::with_cache_capacity(meili, meilisearch_client, DEFAULT_CACHE_CAPACITY).await
    }

    /// Create a new registry with custom cache capacity.
    ///
    /// # Arguments
    ///
    /// * `meili` - Shared reference to embedded MeilisearchLib (for index management)
    /// * `meilisearch_client` - Initialized Meilisearch SDK client (for document CRUD)
    /// * `cache_capacity` - Maximum number of resolved archetypes to cache
    pub async fn with_cache_capacity(
        meili: Arc<MeilisearchLib>,
        meilisearch_client: Client,
        cache_capacity: usize,
    ) -> Result<Self> {
        // Ensure indexes exist (using embedded meilisearch-lib)
        let index_manager = ArchetypeIndexManager::new(meili);
        index_manager.ensure_indexes()?;

        let registry = Self {
            archetypes: Arc::new(RwLock::new(HashMap::new())),
            setting_packs: Arc::new(RwLock::new(HashMap::new())),
            active_packs: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(cache_capacity.max(1)).unwrap(),
            ))),
            meilisearch_client,
            event_listeners: Arc::new(RwLock::new(Vec::new())),
        };

        // Load existing archetypes from Meilisearch
        registry.load_from_meilisearch().await?;

        log::info!(
            "ArchetypeRegistry initialized with {} archetypes",
            registry.count().await
        );

        Ok(registry)
    }

    // ========================================================================
    // CRUD Operations
    // ========================================================================

    /// Register a new archetype in the registry.
    ///
    /// This method:
    /// 1. Validates the archetype
    /// 2. Checks for duplicate IDs
    /// 3. Persists to Meilisearch
    /// 4. Updates in-memory registry
    /// 5. Emits ARCHETYPE_CREATED event
    ///
    /// # Arguments
    ///
    /// * `archetype` - The archetype to register
    ///
    /// # Returns
    ///
    /// The ID of the registered archetype.
    ///
    /// # Errors
    ///
    /// - `ArchetypeError::ValidationFailed` if archetype fails validation
    /// - `ArchetypeError::DuplicateArchetypeId` if ID already exists
    /// - `ArchetypeError::ParentNotFound` if parent_id references non-existent archetype
    /// - `ArchetypeError::Meilisearch` if persistence fails
    pub async fn register(&self, archetype: Archetype) -> Result<ArchetypeId> {
        // Validate the archetype
        archetype.validate()?;

        let id = archetype.id.clone();

        // Use a single write lock to prevent TOCTOU race
        {
            let mut archetypes = self.archetypes.write().await;

            // Check for duplicate ID
            if archetypes.contains_key(id.as_str()) {
                return Err(ArchetypeError::DuplicateArchetypeId(id.to_string()));
            }

            // Validate parent exists if specified
            if let Some(ref parent_id) = archetype.parent_id {
                if !archetypes.contains_key(parent_id.as_str()) {
                    return Err(ArchetypeError::ParentNotFound(parent_id.to_string()));
                }
            }

            // Insert into in-memory registry first
            archetypes.insert(id.to_string(), archetype.clone());
        }

        // Persist to Meilisearch (after releasing lock to avoid blocking)
        if let Err(e) = self.persist_archetype(&archetype).await {
            // Rollback in-memory on persist failure
            let mut archetypes = self.archetypes.write().await;
            archetypes.remove(id.as_str());
            return Err(e);
        }

        // Emit event (stub)
        self.emit_event(ArchetypeEvent::Created { id: id.clone() })
            .await;

        log::info!("Registered archetype: {}", id);

        Ok(id)
    }

    /// Get an archetype by ID (without resolution).
    ///
    /// This returns the archetype as stored, without applying inheritance
    /// or setting pack overrides.
    ///
    /// # Arguments
    ///
    /// * `id` - The archetype ID to look up
    ///
    /// # Errors
    ///
    /// Returns `ArchetypeError::NotFound` if the archetype doesn't exist.
    pub async fn get(&self, id: &str) -> Result<Archetype> {
        let archetypes = self.archetypes.read().await;
        archetypes
            .get(id)
            .cloned()
            .ok_or_else(|| ArchetypeError::NotFound {
                id: id.to_string(),
                layers_checked: vec!["direct_lookup".to_string()],
            })
    }

    /// List all archetypes, optionally filtered by category.
    ///
    /// # Arguments
    ///
    /// * `filter` - Optional category filter
    ///
    /// # Returns
    ///
    /// Vector of archetype summaries matching the filter.
    pub async fn list(&self, filter: Option<ArchetypeCategory>) -> Vec<ArchetypeSummary> {
        let archetypes = self.archetypes.read().await;
        archetypes
            .values()
            .filter(|a| filter.is_none() || filter.as_ref() == Some(&a.category))
            .map(ArchetypeSummary::from)
            .collect()
    }

    /// Update an existing archetype.
    ///
    /// This method:
    /// 1. Validates the updated archetype
    /// 2. Checks if child archetypes would be affected
    /// 3. Persists to Meilisearch
    /// 4. Updates in-memory registry
    /// 5. Invalidates cache for affected entries
    /// 6. Emits ARCHETYPE_MODIFIED event
    ///
    /// # Arguments
    ///
    /// * `archetype` - The updated archetype (must have existing ID)
    ///
    /// # Errors
    ///
    /// - `ArchetypeError::NotFound` if archetype doesn't exist
    /// - `ArchetypeError::ValidationFailed` if archetype fails validation
    /// - `ArchetypeError::ParentNotFound` if new parent_id references non-existent archetype
    pub async fn update(&self, archetype: Archetype) -> Result<()> {
        // Validate the archetype
        archetype.validate()?;

        let id = archetype.id.clone();

        // Check archetype exists
        {
            let archetypes = self.archetypes.read().await;
            if !archetypes.contains_key(id.as_str()) {
                return Err(ArchetypeError::NotFound {
                    id: id.to_string(),
                    layers_checked: vec!["update_lookup".to_string()],
                });
            }
        }

        // Validate parent exists if specified
        if let Some(ref parent_id) = archetype.parent_id {
            // Parent cannot be self
            if parent_id.as_str() == id.as_str() {
                return Err(ArchetypeError::CircularResolution {
                    cycle_path: vec![id.to_string()],
                });
            }

            let archetypes = self.archetypes.read().await;
            if !archetypes.contains_key(parent_id.as_str()) {
                return Err(ArchetypeError::ParentNotFound(parent_id.to_string()));
            }
        }

        // Find child archetypes that inherit from this one
        let affected_children = self.find_children(&id).await;

        // Persist to Meilisearch
        self.persist_archetype(&archetype).await?;

        // Update in-memory registry
        {
            let mut archetypes = self.archetypes.write().await;
            archetypes.insert(id.to_string(), archetype);
        }

        // Invalidate cache for this archetype and all children
        self.invalidate_cache_for_archetype(&id).await;
        for child_id in &affected_children {
            self.invalidate_cache_for_archetype(child_id).await;
        }

        // Emit event
        self.emit_event(ArchetypeEvent::Modified {
            id: id.clone(),
            affected_children,
        })
        .await;

        log::info!("Updated archetype: {}", id);

        Ok(())
    }

    /// Delete an archetype from the registry.
    ///
    /// This method checks for dependent children before allowing deletion.
    ///
    /// # Arguments
    ///
    /// * `id` - The archetype ID to delete
    ///
    /// # Errors
    ///
    /// - `ArchetypeError::NotFound` if archetype doesn't exist
    /// - `ArchetypeError::HasDependentChildren` if other archetypes inherit from this one
    pub async fn delete(&self, id: &str) -> Result<()> {
        // Check archetype exists
        {
            let archetypes = self.archetypes.read().await;
            if !archetypes.contains_key(id) {
                return Err(ArchetypeError::NotFound {
                    id: id.to_string(),
                    layers_checked: vec!["delete_lookup".to_string()],
                });
            }
        }

        // Check for dependent children
        let children = self.find_children(&ArchetypeId::new(id)).await;
        if !children.is_empty() {
            return Err(ArchetypeError::HasDependentChildren {
                child_ids: children.iter().map(|c| c.to_string()).collect(),
            });
        }

        // Delete from Meilisearch
        self.delete_from_meilisearch(id).await?;

        // Remove from in-memory registry
        {
            let mut archetypes = self.archetypes.write().await;
            archetypes.remove(id);
        }

        // Invalidate cache
        self.invalidate_cache_for_archetype(&ArchetypeId::new(id))
            .await;

        // Emit event
        self.emit_event(ArchetypeEvent::Deleted {
            id: ArchetypeId::new(id),
        })
        .await;

        log::info!("Deleted archetype: {}", id);

        Ok(())
    }

    /// Get the total count of registered archetypes.
    pub async fn count(&self) -> usize {
        let archetypes = self.archetypes.read().await;
        archetypes.len()
    }

    /// Check if an archetype exists.
    pub async fn exists(&self, id: &str) -> bool {
        let archetypes = self.archetypes.read().await;
        archetypes.contains_key(id)
    }

    // ========================================================================
    // Setting Pack Operations
    // ========================================================================

    /// Register a setting pack (does not activate it).
    ///
    /// # Arguments
    ///
    /// * `pack` - The setting pack to register
    ///
    /// # Errors
    ///
    /// - `ArchetypeError::SettingPackInvalid` if pack fails validation
    pub async fn register_setting_pack(&self, pack: SettingPack) -> Result<String> {
        // Validate the pack
        pack.validate()?;

        let id = pack.id.clone();

        // Store in registry
        {
            let mut packs = self.setting_packs.write().await;
            packs.insert(id.clone(), pack);
        }

        log::info!("Registered setting pack: {}", id);

        Ok(id)
    }

    /// Get a setting pack by ID.
    pub async fn get_setting_pack(&self, id: &str) -> Result<SettingPack> {
        let packs = self.setting_packs.read().await;
        packs
            .get(id)
            .cloned()
            .ok_or_else(|| ArchetypeError::SettingPackNotFound(id.to_string()))
    }

    /// List all registered setting packs.
    pub async fn list_setting_packs(&self) -> Vec<SettingPackSummary> {
        let packs = self.setting_packs.read().await;
        packs.values().map(SettingPackSummary::from).collect()
    }

    /// Activate a setting pack for a campaign.
    ///
    /// # Arguments
    ///
    /// * `pack_id` - ID of the setting pack to activate
    /// * `campaign_id` - ID of the campaign to activate for
    ///
    /// # Errors
    ///
    /// - `ArchetypeError::SettingPackNotFound` if pack doesn't exist
    /// - `ArchetypeError::SettingPackReferenceError` if pack references missing archetypes
    pub async fn activate_setting_pack(
        &self,
        pack_id: &str,
        campaign_id: &str,
    ) -> Result<()> {
        // Verify pack exists
        let pack = {
            let packs = self.setting_packs.read().await;
            packs
                .get(pack_id)
                .cloned()
                .ok_or_else(|| ArchetypeError::SettingPackNotFound(pack_id.to_string()))?
        };

        // Validate all referenced archetypes exist
        let archetypes = self.archetypes.read().await;
        let missing: Vec<String> = pack
            .archetype_overrides
            .keys()
            .filter(|id| !archetypes.contains_key(*id))
            .cloned()
            .collect();

        if !missing.is_empty() {
            return Err(ArchetypeError::SettingPackReferenceError {
                pack_id: pack_id.to_string(),
                missing_ids: missing,
            });
        }
        drop(archetypes);

        // Deactivate any existing pack for this campaign
        self.deactivate_setting_pack(campaign_id).await?;

        // Activate new pack
        {
            let mut active = self.active_packs.write().await;
            active.insert(campaign_id.to_string(), pack_id.to_string());
        }

        // Invalidate cache for this campaign
        self.invalidate_cache_for_campaign(campaign_id).await;

        // Emit event
        self.emit_event(ArchetypeEvent::SettingPackActivated {
            pack_id: pack_id.to_string(),
            campaign_id: campaign_id.to_string(),
        })
        .await;

        log::info!(
            "Activated setting pack '{}' for campaign '{}'",
            pack_id,
            campaign_id
        );

        Ok(())
    }

    /// Deactivate the setting pack for a campaign.
    ///
    /// # Arguments
    ///
    /// * `campaign_id` - ID of the campaign to deactivate pack for
    pub async fn deactivate_setting_pack(&self, campaign_id: &str) -> Result<()> {
        let old_pack = {
            let mut active = self.active_packs.write().await;
            active.remove(campaign_id)
        };

        if let Some(pack_id) = old_pack {
            // Invalidate cache for this campaign
            self.invalidate_cache_for_campaign(campaign_id).await;

            // Emit event
            self.emit_event(ArchetypeEvent::SettingPackDeactivated {
                pack_id,
                campaign_id: campaign_id.to_string(),
            })
            .await;
        }

        Ok(())
    }

    /// Get the active setting pack for a campaign.
    pub async fn get_active_setting_pack(&self, campaign_id: &str) -> Option<SettingPack> {
        let active = self.active_packs.read().await;
        let pack_id = active.get(campaign_id)?;

        let packs = self.setting_packs.read().await;
        packs.get(pack_id).cloned()
    }

    /// Get the active setting pack ID for a campaign.
    pub async fn get_active_pack_id(&self, campaign_id: &str) -> Option<String> {
        let active = self.active_packs.read().await;
        active.get(campaign_id).cloned()
    }

    // ========================================================================
    // Cache Management
    // ========================================================================

    /// Get a cached resolved archetype if available.
    pub async fn get_cached(&self, query: &ResolutionQuery) -> Option<ResolvedArchetype> {
        let cache_key = query.cache_key();
        let mut cache = self.cache.write().await;
        cache.get(&cache_key).cloned()
    }

    /// Store a resolved archetype in the cache.
    pub async fn cache_resolved(&self, query: &ResolutionQuery, resolved: ResolvedArchetype) {
        let cache_key = query.cache_key();
        let mut cache = self.cache.write().await;
        cache.put(cache_key, resolved);
    }

    /// Invalidate all cache entries related to an archetype.
    pub async fn invalidate_cache_for_archetype(&self, id: &ArchetypeId) {
        let mut cache = self.cache.write().await;
        let id_str = id.as_str();

        // Collect keys to remove (can't modify while iterating)
        let keys_to_remove: Vec<String> = cache
            .iter()
            .filter(|(key, _)| key.contains(id_str))
            .map(|(key, _)| key.clone())
            .collect();

        for key in keys_to_remove {
            cache.pop(&key);
        }
    }

    /// Invalidate all cache entries for a campaign.
    pub async fn invalidate_cache_for_campaign(&self, campaign_id: &str) {
        let mut cache = self.cache.write().await;

        // Collect keys to remove
        let keys_to_remove: Vec<String> = cache
            .iter()
            .filter(|(key, _)| key.contains(campaign_id))
            .map(|(key, _)| key.clone())
            .collect();

        for key in keys_to_remove {
            cache.pop(&key);
        }
    }

    /// Clear the entire cache.
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get current cache statistics.
    pub async fn cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        CacheStats {
            len: cache.len(),
            cap: cache.cap().get(),
        }
    }

    // ========================================================================
    // Internal Helpers
    // ========================================================================

    /// Find all archetypes that inherit from the given archetype.
    async fn find_children(&self, parent_id: &ArchetypeId) -> Vec<ArchetypeId> {
        let archetypes = self.archetypes.read().await;
        archetypes
            .values()
            .filter(|a| a.parent_id.as_ref() == Some(parent_id))
            .map(|a| a.id.clone())
            .collect()
    }

    /// Load archetypes from Meilisearch into memory.
    async fn load_from_meilisearch(&self) -> Result<()> {
        let index = self.meilisearch_client.index(INDEX_ARCHETYPES);

        // Get all documents with pagination using search with empty query
        let mut offset = 0;
        let limit = 100;

        loop {
            let results: meilisearch_sdk::search::SearchResults<Archetype> = index
                .search()
                .with_query("")
                .with_limit(limit)
                .with_offset(offset)
                .execute()
                .await?;

            let count = results.hits.len();

            if count == 0 {
                break;
            }

            {
                let mut archetypes = self.archetypes.write().await;
                for hit in results.hits {
                    let archetype = hit.result;
                    archetypes.insert(archetype.id.to_string(), archetype);
                }
            }

            offset += count;

            // Check if we've fetched all documents
            if count < limit {
                break;
            }
        }

        Ok(())
    }

    /// Persist an archetype to Meilisearch.
    async fn persist_archetype(&self, archetype: &Archetype) -> Result<()> {
        let index = self.meilisearch_client.index(INDEX_ARCHETYPES);

        let task = index.add_documents(&[archetype], Some("id")).await?;

        task.wait_for_completion(
            &self.meilisearch_client,
            Some(Duration::from_millis(TASK_POLL_INTERVAL_MS)),
            Some(Duration::from_secs(TASK_TIMEOUT_SECS)),
        )
        .await?;

        Ok(())
    }

    /// Delete an archetype from Meilisearch.
    async fn delete_from_meilisearch(&self, id: &str) -> Result<()> {
        let index = self.meilisearch_client.index(INDEX_ARCHETYPES);

        let task = index.delete_document(id).await?;

        task.wait_for_completion(
            &self.meilisearch_client,
            Some(Duration::from_millis(TASK_POLL_INTERVAL_MS)),
            Some(Duration::from_secs(TASK_TIMEOUT_SECS)),
        )
        .await?;

        Ok(())
    }

    /// Emit an event to all listeners (stub implementation).
    async fn emit_event(&self, event: ArchetypeEvent) {
        let listeners = self.event_listeners.read().await;
        for listener in listeners.iter() {
            listener(event.clone());
        }
    }

    // ========================================================================
    // Accessors for Resolver
    // ========================================================================

    /// Get a reference to the archetypes map for the resolver.
    ///
    /// This is used by `ArchetypeResolver` to access archetypes without
    /// going through the registry's public API.
    pub(crate) fn archetypes(&self) -> Arc<RwLock<HashMap<String, Archetype>>> {
        self.archetypes.clone()
    }

    /// Get a reference to the setting packs map for the resolver.
    pub(crate) fn setting_packs(&self) -> Arc<RwLock<HashMap<String, SettingPack>>> {
        self.setting_packs.clone()
    }

    /// Get a reference to the active packs map for the resolver.
    pub(crate) fn active_packs(&self) -> Arc<RwLock<HashMap<String, String>>> {
        self.active_packs.clone()
    }
}

// ============================================================================
// CacheStats
// ============================================================================

/// Statistics about the resolution cache.
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Current number of entries in the cache.
    pub len: usize,
    /// Maximum capacity of the cache.
    pub cap: usize,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running Meilisearch instance.
    // For unit tests without Meilisearch, we test the helper methods
    // that don't require network access.

    #[test]
    fn test_cache_stats_default() {
        let stats = CacheStats { len: 10, cap: 256 };
        assert_eq!(stats.len, 10);
        assert_eq!(stats.cap, 256);
    }

    #[test]
    fn test_archetype_event_clone() {
        let event = ArchetypeEvent::Created {
            id: ArchetypeId::new("test"),
        };
        let cloned = event.clone();
        match cloned {
            ArchetypeEvent::Created { id } => assert_eq!(id.as_str(), "test"),
            _ => panic!("Expected Created event"),
        }
    }

    #[test]
    fn test_archetype_event_modified() {
        let event = ArchetypeEvent::Modified {
            id: ArchetypeId::new("parent"),
            affected_children: vec![ArchetypeId::new("child1"), ArchetypeId::new("child2")],
        };
        match event {
            ArchetypeEvent::Modified {
                id,
                affected_children,
            } => {
                assert_eq!(id.as_str(), "parent");
                assert_eq!(affected_children.len(), 2);
            }
            _ => panic!("Expected Modified event"),
        }
    }

    #[test]
    fn test_resolution_query_cache_key() {
        let query = ResolutionQuery::for_npc("merchant").with_race("dwarf");
        let key = query.cache_key();
        assert!(key.contains("merchant"));
        assert!(key.contains("dwarf"));
    }

    // Integration tests would go here with #[tokio::test] attribute
    // and require a running Meilisearch instance
}
