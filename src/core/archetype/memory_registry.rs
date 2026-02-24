//! In-Memory Archetype Registry backed by YAML assets.
//!
//! Drop-in replacement for [`ArchetypeRegistry`] that uses bundled YAML assets
//! instead of Meilisearch for persistence. All data lives in memory — the
//! ~50KB of YAML assets is loaded at construction time.
//!
//! # API Compatibility
//!
//! Provides the same public API surface as the Meilisearch-backed registry:
//! - CRUD: `register()`, `get()`, `list()`, `update()`, `delete()`, `count()`, `exists()`
//! - Setting packs: `register_setting_pack()`, `activate_setting_pack()`, etc.
//! - Cache: `get_cached()`, `cache_resolved()`, `invalidate_cache_for_archetype()`, etc.
//! - Resolver accessors: `archetypes()`, `setting_packs()`, `active_packs()`
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::core::archetype::InMemoryArchetypeRegistry;
//!
//! let registry = InMemoryArchetypeRegistry::new().await;
//! let all = registry.list(None).await;
//! println!("Loaded {} archetypes from YAML assets", all.len());
//! ```

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

use lru::LruCache;
use tokio::sync::RwLock;

use super::error::{ArchetypeError, Result};
use super::registry::{ArchetypeEvent, CacheStats};
use super::resolution::{ResolutionQuery, ResolvedArchetype};
use super::setting_pack::{SettingPack, SettingPackSummary};
use super::types::{Archetype, ArchetypeCategory, ArchetypeId, ArchetypeSummary};
use crate::core::assets::AssetLoader;

// ============================================================================
// Constants
// ============================================================================

/// Default cache capacity for resolved archetypes.
const DEFAULT_CACHE_CAPACITY: usize = 256;

// ============================================================================
// InMemoryArchetypeRegistry
// ============================================================================

/// In-memory archetype registry backed by YAML asset files.
///
/// This registry loads all archetypes, vocabulary banks, and setting packs from
/// the bundled YAML assets at construction time. No external persistence layer
/// is required.
///
/// # Thread Safety
///
/// All mutable state is protected by `tokio::sync::RwLock` for async-safe access.
pub struct InMemoryArchetypeRegistry {
    /// All registered archetypes indexed by ID.
    archetypes: Arc<RwLock<HashMap<String, Archetype>>>,

    /// Loaded setting packs (not necessarily active).
    setting_packs: Arc<RwLock<HashMap<String, SettingPack>>>,

    /// Active setting pack per campaign (campaign_id → pack_id).
    active_packs: Arc<RwLock<HashMap<String, String>>>,

    /// Resolution cache with LRU eviction.
    cache: Arc<RwLock<LruCache<String, ResolvedArchetype>>>,

    /// Event listeners (stub for future event system).
    event_listeners: Arc<RwLock<Vec<Box<dyn Fn(ArchetypeEvent) + Send + Sync>>>>,
}

impl InMemoryArchetypeRegistry {
    /// Create a new registry pre-loaded with bundled YAML assets.
    ///
    /// Loads all archetypes and setting packs from `AssetLoader`. Parse failures
    /// are logged and skipped — the registry will contain whatever parsed successfully.
    pub async fn new() -> Self {
        Self::with_cache_capacity(DEFAULT_CACHE_CAPACITY).await
    }

    /// Create a new registry with custom cache capacity.
    pub async fn with_cache_capacity(cache_capacity: usize) -> Self {
        let mut archetype_map = HashMap::new();
        let mut pack_map = HashMap::new();

        // Load archetypes from bundled YAML
        for archetype in AssetLoader::load_archetypes() {
            archetype_map.insert(archetype.id.to_string(), archetype);
        }

        // Load setting packs from bundled YAML
        for pack in AssetLoader::load_setting_packs() {
            pack_map.insert(pack.id.clone(), pack);
        }

        log::info!(
            "InMemoryArchetypeRegistry initialized: {} archetypes, {} setting packs",
            archetype_map.len(),
            pack_map.len()
        );

        Self {
            archetypes: Arc::new(RwLock::new(archetype_map)),
            setting_packs: Arc::new(RwLock::new(pack_map)),
            active_packs: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(cache_capacity.max(1)).unwrap(),
            ))),
            event_listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create an empty registry (for testing).
    pub fn empty() -> Self {
        Self {
            archetypes: Arc::new(RwLock::new(HashMap::new())),
            setting_packs: Arc::new(RwLock::new(HashMap::new())),
            active_packs: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(DEFAULT_CACHE_CAPACITY).unwrap(),
            ))),
            event_listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    // ========================================================================
    // CRUD Operations
    // ========================================================================

    /// Register a new archetype.
    ///
    /// Validates the archetype and checks for duplicate IDs.
    /// No persistence — changes live only in memory.
    pub async fn register(&self, archetype: Archetype) -> Result<ArchetypeId> {
        archetype.validate()?;

        let id = archetype.id.clone();

        {
            let mut archetypes = self.archetypes.write().await;

            if archetypes.contains_key(id.as_str()) {
                return Err(ArchetypeError::DuplicateArchetypeId(id.to_string()));
            }

            if let Some(ref parent_id) = archetype.parent_id {
                if !archetypes.contains_key(parent_id.as_str()) {
                    return Err(ArchetypeError::ParentNotFound(parent_id.to_string()));
                }
            }

            archetypes.insert(id.to_string(), archetype);
        }

        self.emit_event(ArchetypeEvent::Created { id: id.clone() }).await;
        log::info!("Registered archetype: {}", id);

        Ok(id)
    }

    /// Get an archetype by ID (without resolution).
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

    /// List all archetypes, optionally filtered by category (summary view).
    pub async fn list(&self, filter: Option<ArchetypeCategory>) -> Vec<ArchetypeSummary> {
        let archetypes = self.archetypes.read().await;
        archetypes
            .values()
            .filter(|a| filter.is_none() || filter.as_ref() == Some(&a.category))
            .map(ArchetypeSummary::from)
            .collect()
    }

    /// List all archetypes with full detail, optionally filtered by category.
    pub async fn list_full(&self, filter: Option<ArchetypeCategory>) -> Vec<Archetype> {
        let archetypes = self.archetypes.read().await;
        archetypes
            .values()
            .filter(|a| filter.is_none() || filter.as_ref() == Some(&a.category))
            .cloned()
            .collect()
    }

    /// Update an existing archetype.
    pub async fn update(&self, archetype: Archetype) -> Result<()> {
        archetype.validate()?;

        let id = archetype.id.clone();

        {
            let archetypes = self.archetypes.read().await;
            if !archetypes.contains_key(id.as_str()) {
                return Err(ArchetypeError::NotFound {
                    id: id.to_string(),
                    layers_checked: vec!["update_lookup".to_string()],
                });
            }
        }

        if let Some(ref parent_id) = archetype.parent_id {
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

        let affected_children = self.find_children(&id).await;

        {
            let mut archetypes = self.archetypes.write().await;
            archetypes.insert(id.to_string(), archetype);
        }

        self.invalidate_cache_for_archetype(&id).await;
        for child_id in &affected_children {
            self.invalidate_cache_for_archetype(child_id).await;
        }

        self.emit_event(ArchetypeEvent::Modified {
            id: id.clone(),
            affected_children,
        })
        .await;

        log::info!("Updated archetype: {}", id);
        Ok(())
    }

    /// Delete an archetype.
    pub async fn delete(&self, id: &str) -> Result<()> {
        {
            let archetypes = self.archetypes.read().await;
            if !archetypes.contains_key(id) {
                return Err(ArchetypeError::NotFound {
                    id: id.to_string(),
                    layers_checked: vec!["delete_lookup".to_string()],
                });
            }
        }

        let children = self.find_children(&ArchetypeId::new(id)).await;
        if !children.is_empty() {
            return Err(ArchetypeError::HasDependentChildren {
                child_ids: children.iter().map(|c| c.to_string()).collect(),
            });
        }

        {
            let mut archetypes = self.archetypes.write().await;
            archetypes.remove(id);
        }

        self.invalidate_cache_for_archetype(&ArchetypeId::new(id)).await;
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
    pub async fn register_setting_pack(&self, pack: SettingPack) -> Result<String> {
        pack.validate()?;

        let id = pack.id.clone();
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

        self.invalidate_cache_for_campaign(campaign_id).await;

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
    pub async fn deactivate_setting_pack(&self, campaign_id: &str) -> Result<()> {
        let old_pack = {
            let mut active = self.active_packs.write().await;
            active.remove(campaign_id)
        };

        if let Some(pack_id) = old_pack {
            self.invalidate_cache_for_campaign(campaign_id).await;
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

    /// Emit an event to all listeners.
    async fn emit_event(&self, event: ArchetypeEvent) {
        let listeners = self.event_listeners.read().await;
        for listener in listeners.iter() {
            listener(event.clone());
        }
    }

    // ========================================================================
    // Resolver Accessors
    // ========================================================================

    /// Get a reference to the archetypes map for the resolver.
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
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_loads_assets() {
        let registry = InMemoryArchetypeRegistry::new().await;
        let count = registry.count().await;
        assert!(count > 0, "should load archetypes from YAML assets");
        assert!(count >= 20, "expected at least 20 archetypes, got {}", count);
    }

    #[tokio::test]
    async fn test_empty_registry() {
        let registry = InMemoryArchetypeRegistry::empty();
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_get_existing_archetype() {
        let registry = InMemoryArchetypeRegistry::new().await;
        let fighter = registry.get("fighter").await;
        assert!(fighter.is_ok(), "fighter should exist");
        assert_eq!(fighter.unwrap().display_name.as_ref(), "Fighter");
    }

    #[tokio::test]
    async fn test_get_nonexistent_archetype() {
        let registry = InMemoryArchetypeRegistry::new().await;
        let result = registry.get("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_all() {
        let registry = InMemoryArchetypeRegistry::new().await;
        let all = registry.list(None).await;
        assert!(!all.is_empty());
    }

    #[tokio::test]
    async fn test_list_by_category() {
        let registry = InMemoryArchetypeRegistry::new().await;

        let classes = registry.list(Some(ArchetypeCategory::Class)).await;
        assert!(!classes.is_empty(), "should have class archetypes");

        let races = registry.list(Some(ArchetypeCategory::Race)).await;
        assert!(!races.is_empty(), "should have race archetypes");

        let roles = registry.list(Some(ArchetypeCategory::Role)).await;
        assert!(!roles.is_empty(), "should have role archetypes");
    }

    #[tokio::test]
    async fn test_register_new_archetype() {
        let registry = InMemoryArchetypeRegistry::empty();

        let archetype = Archetype::new("test_knight", "Test Knight", ArchetypeCategory::Class);
        let id = registry.register(archetype).await.unwrap();
        assert_eq!(id.as_str(), "test_knight");
        assert!(registry.exists("test_knight").await);
    }

    #[tokio::test]
    async fn test_register_duplicate_fails() {
        let registry = InMemoryArchetypeRegistry::new().await;

        let archetype = Archetype::new("fighter", "Duplicate Fighter", ArchetypeCategory::Class);
        let result = registry.register(archetype).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_archetype() {
        let registry = InMemoryArchetypeRegistry::empty();

        let archetype = Archetype::new("deleteme", "Delete Me", ArchetypeCategory::Role);
        registry.register(archetype).await.unwrap();
        assert!(registry.exists("deleteme").await);

        registry.delete("deleteme").await.unwrap();
        assert!(!registry.exists("deleteme").await);
    }

    #[tokio::test]
    async fn test_setting_packs_loaded() {
        let registry = InMemoryArchetypeRegistry::new().await;
        let packs = registry.list_setting_packs().await;
        assert!(!packs.is_empty(), "should load setting packs from YAML");
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let registry = InMemoryArchetypeRegistry::new().await;

        let query = ResolutionQuery::single("fighter");
        let resolved = ResolvedArchetype::new();

        // Cache miss
        assert!(registry.get_cached(&query).await.is_none());

        // Cache put + hit
        registry.cache_resolved(&query, resolved.clone()).await;
        assert!(registry.get_cached(&query).await.is_some());

        // Invalidate
        registry
            .invalidate_cache_for_archetype(&ArchetypeId::new("fighter"))
            .await;
        assert!(registry.get_cached(&query).await.is_none());
    }

    #[tokio::test]
    async fn test_resolver_accessors() {
        let registry = InMemoryArchetypeRegistry::new().await;

        let archetypes = registry.archetypes();
        let packs = registry.setting_packs();
        let active = registry.active_packs();

        let map = archetypes.read().await;
        assert!(!map.is_empty());
        drop(map);

        let packs_map = packs.read().await;
        assert!(!packs_map.is_empty());
        drop(packs_map);

        let active_map = active.read().await;
        assert!(active_map.is_empty(), "no active packs by default");
    }
}
