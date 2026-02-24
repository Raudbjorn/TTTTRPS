//! Setting Template Store with Meilisearch and LRU Cache (TASK-PERS-006)
//!
//! Provides the `SettingTemplateStore` for managing setting templates with:
//! - CRUD operations (get, list_all, save, delete)
//! - Filtering by game_system and setting_name
//! - Full-text search across name, description, vocabulary, phrases
//! - LRU cache (capacity 100) with `tokio::sync::RwLock` for async safety
//! - Cache invalidation on write operations
//!
//! ## Architecture
//!
//! ```text
//! SettingTemplateStore
//! ├── PersonalityIndexManager (embedded meilisearch_lib, sync operations)
//! └── LRU Cache (in-memory, capacity 100)
//!     └── RwLock<LruCache<TemplateId, SettingTemplate>>
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! // Create from an existing PersonalityIndexManager
//! let store = SettingTemplateStore::from_manager(index_manager);
//!
//! // Save a template
//! store.save(&template).await?;
//!
//! // Get by ID (checks cache first)
//! let template = store.get(&template_id).await?;
//!
//! // Search
//! let results = store.search("forgotten realms sage")?;
//!
//! // Filter by game system
//! let dnd_templates = store.filter_by_game_system("dnd5e")?;
//! ```

use super::errors::{PersonalityExtensionError, TemplateError};

use super::templates::SettingTemplate;
use super::types::{PersonalityId, TemplateDocument, TemplateId};
use super::search::{PersonalityIndexManager, escape_filter_value};
use crate::core::personality_base::PersonalityProfile;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Constants
// ============================================================================

/// Default LRU cache capacity.
const DEFAULT_CACHE_CAPACITY: usize = 100;

/// Maximum results to return from search/filter operations.
const DEFAULT_SEARCH_LIMIT: usize = 100;

// ============================================================================
// Setting Template Store
// ============================================================================

/// Store for managing setting templates with Meilisearch and LRU caching.
pub struct SettingTemplateStore {
    /// Meilisearch index manager.
    index_manager: Arc<PersonalityIndexManager>,

    /// LRU cache for recently accessed templates.
    cache: RwLock<LruCache<TemplateId, SettingTemplate>>,

    /// Cache capacity for monitoring.
    cache_capacity: usize,
}

impl SettingTemplateStore {
    /// Create a store from an existing index manager.
    ///
    /// This is the primary constructor. The `PersonalityIndexManager` holds
    /// an `Arc<MeilisearchLib>` used for embedded search operations.
    pub fn from_manager(index_manager: Arc<PersonalityIndexManager>) -> Self {
        let capacity =
            NonZeroUsize::new(DEFAULT_CACHE_CAPACITY).unwrap();

        Self {
            index_manager,
            cache: RwLock::new(LruCache::new(capacity)),
            cache_capacity: DEFAULT_CACHE_CAPACITY,
        }
    }

    /// Create a store from an existing index manager with custom cache capacity.
    pub fn from_manager_with_capacity(
        index_manager: Arc<PersonalityIndexManager>,
        cache_capacity: usize,
    ) -> Self {
        let capacity = NonZeroUsize::new(cache_capacity.max(1))
            .unwrap_or(NonZeroUsize::new(DEFAULT_CACHE_CAPACITY).unwrap());

        Self {
            index_manager,
            cache: RwLock::new(LruCache::new(capacity)),
            cache_capacity,
        }
    }

    /// Get the underlying index manager.
    pub fn index_manager(&self) -> &Arc<PersonalityIndexManager> {
        &self.index_manager
    }

    // ========================================================================
    // CRUD Operations
    // ========================================================================

    /// Get a template by ID.
    ///
    /// Checks the cache first, then falls back to Meilisearch.
    pub async fn get(&self, id: &TemplateId) -> Result<Option<SettingTemplate>, PersonalityExtensionError> {
        // Check cache first
        {
            let mut cache = self.cache.write().await;
            if let Some(template) = cache.get(id) {
                log::debug!("Cache hit for template: {}", id);
                return Ok(Some(template.clone()));
            }
        }

        // Fall back to Meilisearch
        log::debug!("Cache miss for template: {}, fetching from Meilisearch", id);
        let doc = self.index_manager.get_template(id)?;

        if let Some(doc) = doc {
            // Convert document to full template
            let template = self.document_to_template(&doc)?;

            // Update cache
            {
                let mut cache = self.cache.write().await;
                cache.put(id.clone(), template.clone());
            }

            Ok(Some(template))
        } else {
            Ok(None)
        }
    }

    /// List all templates.
    pub async fn list_all(&self) -> Result<Vec<SettingTemplate>, PersonalityExtensionError> {
        self.list_with_limit(DEFAULT_SEARCH_LIMIT).await
    }

    /// List templates with a custom limit.
    pub async fn list_with_limit(
        &self,
        limit: usize,
    ) -> Result<Vec<SettingTemplate>, PersonalityExtensionError> {
        let docs = self.index_manager.list_templates(None, limit)?;

        let mut templates = Vec::with_capacity(docs.len());
        for doc in docs {
            let template = self.document_to_template(&doc)?;
            templates.push(template);
        }

        Ok(templates)
    }

    /// Save a template (creates or updates).
    ///
    /// Automatically updates vocabulary_keys and invalidates cache.
    ///
    /// NOTE: `upsert_template` is a synchronous disk I/O call via embedded
    /// Meilisearch. This is acceptable because the in-process engine uses
    /// mmap-backed storage, making typical writes sub-millisecond.
    pub async fn save(&self, template: &SettingTemplate) -> Result<(), PersonalityExtensionError> {
        // Prepare template for indexing
        let mut template = template.clone();
        template.update_vocabulary_keys();
        template.touch();

        // Convert to SettingPersonalityTemplate for indexing
        let spt: super::types::SettingPersonalityTemplate = template.clone().into();

        // Save to Meilisearch (sync I/O, fast for embedded engine)
        self.index_manager.upsert_template(&spt)?;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.put(template.id.clone(), template);
        }

        log::debug!("Saved template: {}", spt.id);
        Ok(())
    }

    /// Delete a template by ID.
    ///
    /// Invalidates cache entry.
    pub async fn delete(&self, id: &TemplateId) -> Result<(), PersonalityExtensionError> {
        // Delete from Meilisearch
        self.index_manager.delete_template(id)?;

        // Remove from cache
        {
            let mut cache = self.cache.write().await;
            cache.pop(id);
        }

        log::debug!("Deleted template: {}", id);
        Ok(())
    }

    // ========================================================================
    // Filtering Operations
    // ========================================================================

    /// Filter templates by game system.
    pub fn filter_by_game_system(
        &self,
        game_system: &str,
    ) -> Result<Vec<SettingTemplate>, PersonalityExtensionError> {
        let docs = self
            .index_manager
            .list_templates_by_game_system(game_system, DEFAULT_SEARCH_LIMIT)?;

        self.documents_to_templates(docs)
    }

    /// Filter templates by setting name.
    pub fn filter_by_setting(
        &self,
        setting_name: &str,
    ) -> Result<Vec<SettingTemplate>, PersonalityExtensionError> {
        let filter = format!("settingName = \"{}\"", escape_filter_value(setting_name));
        let docs = self
            .index_manager
            .list_templates(Some(&filter), DEFAULT_SEARCH_LIMIT)?;

        self.documents_to_templates(docs)
    }

    /// Filter templates by both game system and setting.
    pub fn filter_by_game_system_and_setting(
        &self,
        game_system: &str,
        setting_name: &str,
    ) -> Result<Vec<SettingTemplate>, PersonalityExtensionError> {
        let filter = format!(
            "gameSystem = \"{}\" AND settingName = \"{}\"",
            escape_filter_value(game_system),
            escape_filter_value(setting_name),
        );
        let docs = self
            .index_manager
            .list_templates(Some(&filter), DEFAULT_SEARCH_LIMIT)?;

        self.documents_to_templates(docs)
    }

    /// List built-in templates only.
    pub fn list_builtin(&self) -> Result<Vec<SettingTemplate>, PersonalityExtensionError> {
        let docs = self
            .index_manager
            .list_builtin_templates(DEFAULT_SEARCH_LIMIT)?;

        self.documents_to_templates(docs)
    }

    /// List templates for a specific campaign.
    pub fn filter_by_campaign(
        &self,
        campaign_id: &str,
    ) -> Result<Vec<SettingTemplate>, PersonalityExtensionError> {
        let docs = self
            .index_manager
            .list_templates_by_campaign(campaign_id, DEFAULT_SEARCH_LIMIT)?;

        self.documents_to_templates(docs)
    }

    /// Filter templates by tag.
    pub fn filter_by_tag(
        &self,
        tag: &str,
    ) -> Result<Vec<SettingTemplate>, PersonalityExtensionError> {
        let filter = format!("tags = \"{}\"", escape_filter_value(tag));
        let docs = self
            .index_manager
            .list_templates(Some(&filter), DEFAULT_SEARCH_LIMIT)?;

        self.documents_to_templates(docs)
    }

    // ========================================================================
    // Search Operations
    // ========================================================================

    /// Search templates by keyword.
    ///
    /// Searches across name, description, vocabulary keys, and common phrases.
    pub fn search(&self, keyword: &str) -> Result<Vec<SettingTemplate>, PersonalityExtensionError> {
        self.search_with_limit(keyword, DEFAULT_SEARCH_LIMIT)
    }

    /// Search templates with a custom limit.
    pub fn search_with_limit(
        &self,
        keyword: &str,
        limit: usize,
    ) -> Result<Vec<SettingTemplate>, PersonalityExtensionError> {
        let docs = self
            .index_manager
            .search_templates(keyword, None, limit)?;

        self.documents_to_templates(docs)
    }

    /// Search templates with a filter.
    pub fn search_filtered(
        &self,
        keyword: &str,
        filter: &str,
    ) -> Result<Vec<SettingTemplate>, PersonalityExtensionError> {
        let docs = self
            .index_manager
            .search_templates(keyword, Some(filter), DEFAULT_SEARCH_LIMIT)?;

        self.documents_to_templates(docs)
    }

    // ========================================================================
    // Template Instantiation
    // ========================================================================

    /// Instantiate a template to create a PersonalityProfile.
    ///
    /// Requires a base profile to apply the template overrides to.
    pub async fn instantiate_template(
        &self,
        template_id: &TemplateId,
        base_profile: &PersonalityProfile,
    ) -> Result<PersonalityProfile, PersonalityExtensionError> {
        let template = self.get(template_id).await?.ok_or_else(|| {
            TemplateError::not_found(template_id.to_string())
        })?;

        Ok(template.to_personality_profile(base_profile))
    }

    /// Instantiate a template by looking up the base profile by ID.
    ///
    /// This requires a profile resolver function since we don't have direct
    /// access to the PersonalityStore.
    pub async fn instantiate_template_with_resolver<F>(
        &self,
        template_id: &TemplateId,
        resolve_profile: F,
    ) -> Result<PersonalityProfile, PersonalityExtensionError>
    where
        F: FnOnce(&PersonalityId) -> Option<PersonalityProfile>,
    {
        let template = self.get(template_id).await?.ok_or_else(|| {
            TemplateError::not_found(template_id.to_string())
        })?;

        let base_profile = resolve_profile(&template.base_profile).ok_or_else(|| {
            TemplateError::base_profile_not_found(
                template_id.to_string(),
                template.base_profile.to_string(),
            )
        })?;

        Ok(template.to_personality_profile(&base_profile))
    }

    // ========================================================================
    // Cache Management
    // ========================================================================

    /// Clear the cache.
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        log::debug!("Cleared template cache");
    }

    /// Get cache statistics.
    pub async fn cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        CacheStats {
            capacity: self.cache_capacity,
            current_size: cache.len(),
        }
    }

    /// Invalidate a specific cache entry.
    pub async fn invalidate_cache_entry(&self, id: &TemplateId) {
        let mut cache = self.cache.write().await;
        cache.pop(id);
    }

    /// Pre-warm the cache with specific templates.
    pub async fn warm_cache(&self, ids: &[TemplateId]) -> Result<usize, PersonalityExtensionError> {
        let mut loaded = 0;

        for id in ids {
            if let Some(_) = self.get(id).await? {
                loaded += 1;
            }
        }

        log::debug!("Warmed cache with {} templates", loaded);
        Ok(loaded)
    }

    // ========================================================================
    // Batch Operations
    // ========================================================================

    /// Save multiple templates.
    pub async fn save_batch(
        &self,
        templates: &[SettingTemplate],
    ) -> Result<BatchSaveResult, PersonalityExtensionError> {
        let mut saved = 0;
        let mut failed = Vec::new();

        for template in templates {
            match self.save(template).await {
                Ok(_) => saved += 1,
                Err(e) => {
                    failed.push((template.id.clone(), e.to_string()));
                }
            }
        }

        Ok(BatchSaveResult { saved, failed })
    }

    /// Delete multiple templates.
    pub async fn delete_batch(
        &self,
        ids: &[TemplateId],
    ) -> Result<BatchDeleteResult, PersonalityExtensionError> {
        let mut deleted = 0;
        let mut failed = Vec::new();

        for id in ids {
            match self.delete(id).await {
                Ok(_) => deleted += 1,
                Err(e) => {
                    failed.push((id.clone(), e.to_string()));
                }
            }
        }

        Ok(BatchDeleteResult { deleted, failed })
    }

    // ========================================================================
    // Utility Operations
    // ========================================================================

    /// Check if a template exists by ID.
    pub async fn exists(&self, id: &TemplateId) -> Result<bool, PersonalityExtensionError> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if cache.peek(id).is_some() {
                return Ok(true);
            }
        }

        // Check Meilisearch
        let doc = self.index_manager.get_template(id)?;
        Ok(doc.is_some())
    }

    /// Get the total count of templates.
    pub async fn count(&self) -> Result<u64, PersonalityExtensionError> {
        let stats = self.index_manager.get_stats()?;
        Ok(stats.template_count)
    }

    /// Clear all templates (use with caution).
    pub async fn clear_all(&self) -> Result<(), PersonalityExtensionError> {
        self.index_manager.clear_templates()?;
        self.clear_cache().await;
        log::info!("Cleared all templates");
        Ok(())
    }

    // ========================================================================
    // Import/Export Operations (TASK-PERS-008)
    // ========================================================================

    /// Export a template to YAML string.
    ///
    /// Serializes the template to YAML format suitable for file storage or sharing.
    pub fn export_to_yaml(&self, template: &SettingTemplate) -> Result<String, PersonalityExtensionError> {
        use super::templates::TemplateYaml;

        let yaml: TemplateYaml = template.clone().into();
        serde_yaml_ng::to_string(&yaml).map_err(|e| {
            PersonalityExtensionError::internal(format!("Failed to serialize template to YAML: {}", e))
        })
    }

    /// Import a template from YAML string.
    ///
    /// Parses and validates the YAML content, returning a `SettingTemplate`.
    /// Does NOT save to the store - call `save()` separately.
    pub fn import_from_yaml(&self, yaml: &str) -> Result<SettingTemplate, PersonalityExtensionError> {
        use super::templates::{TemplateValidationConfig, TemplateYaml};

        let template_yaml: TemplateYaml = serde_yaml_ng::from_str(yaml).map_err(|e| {
            TemplateError::ParseError {
                file: "<string>".to_string(),
                line: e.location().map(|loc| loc.line()).unwrap_or(0),
                message: e.to_string(),
                source: Some(Box::new(e)),
            }
        })?;

        let mut template: SettingTemplate = template_yaml.try_into()?;
        template.update_vocabulary_keys();
        template.validate_with_config(&TemplateValidationConfig::lenient())?;

        Ok(template)
    }

    /// Import a template from YAML with a new generated ID.
    ///
    /// Use this when importing templates that may conflict with existing ones.
    pub fn import_from_yaml_new_id(&self, yaml: &str) -> Result<SettingTemplate, PersonalityExtensionError> {
        let mut template = self.import_from_yaml(yaml)?;
        template.id = TemplateId::generate();
        template.touch();
        Ok(template)
    }

    /// Import and save a template from YAML.
    ///
    /// Combines import and save into a single operation.
    pub async fn import_and_save(&self, yaml: &str) -> Result<SettingTemplate, PersonalityExtensionError> {
        let template = self.import_from_yaml(yaml)?;
        self.save(&template).await?;
        Ok(template)
    }

    /// Import and save a template from YAML with a new ID.
    ///
    /// Useful when importing templates that may have duplicate IDs.
    pub async fn import_and_save_new_id(&self, yaml: &str) -> Result<SettingTemplate, PersonalityExtensionError> {
        let template = self.import_from_yaml_new_id(yaml)?;
        self.save(&template).await?;
        Ok(template)
    }

    /// Check if a template with the given name already exists.
    ///
    /// Returns true if a template with the same name exists in the store.
    pub async fn check_duplicate_name(&self, name: &str) -> Result<bool, PersonalityExtensionError> {
        let templates = self.list_all().await?;
        Ok(templates.iter().any(|t| t.name == name))
    }

    /// Import a template with duplicate name checking.
    ///
    /// Returns an error if a template with the same name already exists.
    pub async fn import_and_save_checked(
        &self,
        yaml: &str,
    ) -> Result<SettingTemplate, PersonalityExtensionError> {
        let template = self.import_from_yaml(yaml)?;

        if self.check_duplicate_name(&template.name).await? {
            return Err(TemplateError::ValidationError {
                template_id: template.id.to_string(),
                message: format!("A template with name '{}' already exists", template.name),
            }.into());
        }

        self.save(&template).await?;
        Ok(template)
    }

    /// Export multiple templates to YAML.
    ///
    /// Returns a map of template ID to YAML content.
    pub fn export_batch_to_yaml(
        &self,
        templates: &[SettingTemplate],
    ) -> Result<std::collections::HashMap<String, String>, PersonalityExtensionError> {
        let mut result = std::collections::HashMap::new();

        for template in templates {
            let yaml = self.export_to_yaml(template)?;
            result.insert(template.id.to_string(), yaml);
        }

        Ok(result)
    }

    /// Import multiple templates from YAML strings.
    ///
    /// Continues processing even if some imports fail.
    pub async fn import_batch_from_yaml(
        &self,
        yamls: &[String],
    ) -> Result<BatchImportResult, PersonalityExtensionError> {
        let mut result = BatchImportResult {
            imported: 0,
            failed: Vec::new(),
        };

        for (index, yaml) in yamls.iter().enumerate() {
            match self.import_and_save(yaml).await {
                Ok(_) => result.imported += 1,
                Err(e) => {
                    result.failed.push((index, e.to_string()));
                }
            }
        }

        Ok(result)
    }

    // ========================================================================
    // Internal Helpers
    // ========================================================================

    /// Convert a TemplateDocument to a full SettingTemplate.
    ///
    /// Since TemplateDocument doesn't contain all fields (like full vocabulary map),
    /// we reconstruct from the document fields available.
    fn document_to_template(
        &self,
        doc: &TemplateDocument,
    ) -> Result<SettingTemplate, PersonalityExtensionError> {
        // For now, we reconstruct from the document fields we have.
        // In a production system, you might store the full template JSON in Meilisearch
        // or maintain a separate storage for full templates.

        // Reconstruct vocabulary from keys (frequencies are lost in document conversion)
        // This is a limitation - consider storing full template in Meilisearch
        let vocabulary: std::collections::HashMap<String, f32> = doc
            .vocabulary_keys
            .iter()
            .map(|k| (k.clone(), 0.05)) // Default frequency
            .collect();

        Ok(SettingTemplate {
            id: TemplateId::new(&doc.id),
            name: doc.name.clone(),
            description: doc.description.clone(),
            game_system: doc.game_system.clone(),
            setting_name: doc.setting_name.clone(),
            is_builtin: doc.is_builtin,
            base_profile: PersonalityId::new(&doc.base_profile),
            vocabulary,
            common_phrases: doc.common_phrases.clone(),
            deity_references: Vec::new(), // Not stored in document
            tags: doc.tags.clone(),
            tone_overrides: std::collections::HashMap::new(), // Not stored in document
            cultural_markers: Vec::new(), // Not stored in document
            campaign_id: doc.campaign_id.clone(),
            vocabulary_keys: doc.vocabulary_keys.clone(),
            created_at: doc.created_at.clone(),
            updated_at: doc.updated_at.clone(),
        })
    }

    /// Convert multiple documents to templates.
    fn documents_to_templates(
        &self,
        docs: Vec<TemplateDocument>,
    ) -> Result<Vec<SettingTemplate>, PersonalityExtensionError> {
        let mut templates = Vec::with_capacity(docs.len());
        for doc in docs {
            let template = self.document_to_template(&doc)?;
            templates.push(template);
        }
        Ok(templates)
    }
}

// ============================================================================
// Result Types
// ============================================================================

/// Cache statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    /// Maximum cache capacity.
    pub capacity: usize,

    /// Current number of cached entries.
    pub current_size: usize,
}

/// Result of a batch save operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchSaveResult {
    /// Number of templates successfully saved.
    pub saved: usize,

    /// Templates that failed to save with error messages.
    pub failed: Vec<(TemplateId, String)>,
}

/// Result of a batch delete operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchDeleteResult {
    /// Number of templates successfully deleted.
    pub deleted: usize,

    /// Templates that failed to delete with error messages.
    pub failed: Vec<(TemplateId, String)>,
}

/// Result of a batch import operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchImportResult {
    /// Number of templates successfully imported.
    pub imported: usize,

    /// Indices and error messages for failed imports.
    pub failed: Vec<(usize, String)>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Set up a test store with a temporary Meilisearch instance.
    ///
    /// Returns `(TempDir, SettingTemplateStore)`. The `TempDir` must be kept
    /// alive for the duration of the test to prevent premature cleanup.
    fn setup_test_store() -> (tempfile::TempDir, SettingTemplateStore) {
        setup_test_store_with_capacity(DEFAULT_CACHE_CAPACITY)
    }

    fn setup_test_store_with_capacity(
        capacity: usize,
    ) -> (tempfile::TempDir, SettingTemplateStore) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let options = crate::core::wilysearch::core::MeilisearchOptions {
            db_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        let meili = Arc::new(crate::core::wilysearch::engine::Engine::new(options).unwrap());
        let index_manager = Arc::new(PersonalityIndexManager::new(meili));
        index_manager.initialize_indexes().unwrap();
        let store = SettingTemplateStore::from_manager_with_capacity(index_manager, capacity);
        (temp_dir, store)
    }

    fn sample_template() -> SettingTemplate {
        let mut vocab = HashMap::new();
        for i in 0..10 {
            vocab.insert(format!("term{}", i), 0.05);
        }

        let phrases: Vec<String> = (0..5).map(|i| format!("Phrase {}", i)).collect();

        SettingTemplate::builder("Test Template", "storyteller")
            .game_system("dnd5e")
            .setting_name("Test Setting")
            .vocabulary_map(vocab)
            .common_phrases(phrases)
            .deity_reference("Test Deity")
            .tag("test")
            .build()
    }

    #[test]
    fn test_cache_stats_serialization() {
        let stats = CacheStats {
            capacity: 100,
            current_size: 50,
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"capacity\":100"));
        assert!(json.contains("\"currentSize\":50"));
    }

    #[test]
    fn test_batch_save_result_serialization() {
        let result = BatchSaveResult {
            saved: 5,
            failed: vec![(TemplateId::new("id1"), "error message".to_string())],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"saved\":5"));
        assert!(json.contains("\"failed\""));
    }

    #[test]
    fn test_batch_delete_result_serialization() {
        let result = BatchDeleteResult {
            deleted: 3,
            failed: vec![],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"deleted\":3"));
    }

    #[tokio::test]
    #[ignore = "Requires embedded Meilisearch instance"]
    async fn test_store_crud_operations() {
        let (_tmp, store) = setup_test_store();
        let template = sample_template();

        // Save
        store.save(&template).await.unwrap();

        // Get
        let retrieved = store.get(&template.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, template.name);

        // Exists
        assert!(store.exists(&template.id).await.unwrap());

        // Delete
        store.delete(&template.id).await.unwrap();
        assert!(!store.exists(&template.id).await.unwrap());
    }

    #[tokio::test]
    #[ignore = "Requires embedded Meilisearch instance"]
    async fn test_store_cache_behavior() {
        let (_tmp, store) = setup_test_store_with_capacity(10);
        let template = sample_template();
        store.save(&template).await.unwrap();

        // First get should hit Meilisearch
        let _ = store.get(&template.id).await.unwrap();

        // Second get should hit cache
        let stats = store.cache_stats().await;
        assert_eq!(stats.current_size, 1);

        // Clear cache
        store.clear_cache().await;
        let stats = store.cache_stats().await;
        assert_eq!(stats.current_size, 0);

        // Cleanup
        store.delete(&template.id).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "Requires embedded Meilisearch instance"]
    async fn test_store_search() {
        let (_tmp, store) = setup_test_store();
        let template = SettingTemplate::builder("Forgotten Realms Sage", "storyteller")
            .game_system("dnd5e")
            .setting_name("Forgotten Realms")
            .vocabulary("ancient texts", 0.05)
            .common_phrase("As the annals of Candlekeep record")
            .deity_reference("Mystra")
            .tag("sage")
            .build();

        store.save(&template).await.unwrap();

        // Wait for indexing
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Search by name
        let results = store.search("Forgotten Realms").unwrap();
        assert!(!results.is_empty());

        // Cleanup
        store.delete(&template.id).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "Requires embedded Meilisearch instance"]
    async fn test_store_filter_by_game_system() {
        let (_tmp, store) = setup_test_store();
        let template = sample_template();
        store.save(&template).await.unwrap();

        // Wait for indexing
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let results = store.filter_by_game_system("dnd5e").unwrap();
        assert!(!results.is_empty());

        // Cleanup
        store.delete(&template.id).await.unwrap();
    }
}
