//! Meilisearch Index Configuration for Personality System
//!
//! Defines the Meilisearch indexes, settings, and operations for the
//! personality template and blend rule storage.
//!
//! All operations use the embedded `meilisearch_lib` (synchronous, no HTTP).

use super::errors::PersonalityExtensionError;
use super::types::{
    BlendRule, BlendRuleDocument, SettingPersonalityTemplate, TemplateDocument, TemplateId,
    BlendRuleId,
};
use meilisearch_lib::{FilterableAttributesRule, MeilisearchLib, SearchQuery, Setting, Settings, Unchecked};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Error Type
// ============================================================================

/// Errors that can occur during personality index operations.
///
/// Uses `detail` instead of `source` for the error message string to avoid
/// conflict with `thiserror`'s automatic `#[source]` attribute on fields
/// named `source` (which requires `std::error::Error`).
#[derive(Debug, thiserror::Error)]
pub enum PersonalityIndexError {
    #[error("Failed to check index '{index}': {detail}")]
    Check { index: String, detail: String },

    #[error("Failed to create index '{index}': {detail}")]
    Create { index: String, detail: String },

    #[error("Failed to update settings for '{index}': {detail}")]
    Settings { index: String, detail: String },

    #[error("Task wait failed for index '{index}': {detail}")]
    TaskWaitFailed { index: String, detail: String },

    #[error("Failed to get stats for '{index}': {detail}")]
    Stats { index: String, detail: String },

    #[error("Failed to add document to '{index}': {detail}")]
    AddDocuments { index: String, detail: String },

    #[error("Failed to get document '{doc_id}' from '{index}': {detail}")]
    GetDocument { index: String, doc_id: String, detail: String },

    #[error("Failed to delete document '{doc_id}' from '{index}': {detail}")]
    DeleteDocument { index: String, doc_id: String, detail: String },

    #[error("Search failed on '{index}': {detail}")]
    Search { index: String, detail: String },

    #[error("Failed to clear index '{index}': {detail}")]
    Clear { index: String, detail: String },
}

impl From<PersonalityIndexError> for String {
    fn from(e: PersonalityIndexError) -> Self {
        e.to_string()
    }
}

impl From<PersonalityIndexError> for PersonalityExtensionError {
    fn from(e: PersonalityIndexError) -> Self {
        PersonalityExtensionError::Internal(e.to_string())
    }
}

// ============================================================================
// Index Constants
// ============================================================================

/// Index name for personality templates.
pub const INDEX_PERSONALITY_TEMPLATES: &str = "ttrpg_personality_templates";

/// Index name for blend rules.
pub const INDEX_BLEND_RULES: &str = "ttrpg_blend_rules";

/// Default timeout for index operations (30 seconds).
const INDEX_TIMEOUT: Duration = Duration::from_secs(30);

// ============================================================================
// Filter Safety
// ============================================================================

/// Escape a value for safe use in Meilisearch filter expressions.
/// Escapes backslashes and double quotes to prevent filter injection.
pub(crate) fn escape_filter_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

// ============================================================================
// Index Settings
// ============================================================================

/// Get the settings configuration for the personality templates index.
pub fn personality_templates_settings() -> Settings<Unchecked> {
    let filterable: Vec<FilterableAttributesRule> = vec![
        FilterableAttributesRule::Field("gameSystem".to_string()),
        FilterableAttributesRule::Field("settingName".to_string()),
        FilterableAttributesRule::Field("isBuiltin".to_string()),
        FilterableAttributesRule::Field("tags".to_string()),
        FilterableAttributesRule::Field("campaignId".to_string()),
    ];

    let sortable = BTreeSet::from([
        "name".to_string(),
        "createdAt".to_string(),
        "updatedAt".to_string(),
    ]);

    Settings {
        searchable_attributes: Setting::Set(vec![
            "name".to_string(),
            "description".to_string(),
            "vocabularyKeys".to_string(),
            "commonPhrases".to_string(),
        ])
        .into(),
        filterable_attributes: Setting::Set(filterable),
        sortable_attributes: Setting::Set(sortable),
        ..Default::default()
    }
}

/// Get the settings configuration for the blend rules index.
pub fn blend_rules_settings() -> Settings<Unchecked> {
    let filterable: Vec<FilterableAttributesRule> = vec![
        FilterableAttributesRule::Field("context".to_string()),
        FilterableAttributesRule::Field("enabled".to_string()),
        FilterableAttributesRule::Field("isBuiltin".to_string()),
        FilterableAttributesRule::Field("tags".to_string()),
        FilterableAttributesRule::Field("campaignId".to_string()),
    ];

    let sortable = BTreeSet::from([
        "name".to_string(),
        "priority".to_string(),
        "createdAt".to_string(),
        "updatedAt".to_string(),
    ]);

    Settings {
        searchable_attributes: Setting::Set(vec![
            "name".to_string(),
            "description".to_string(),
        ])
        .into(),
        filterable_attributes: Setting::Set(filterable),
        sortable_attributes: Setting::Set(sortable),
        ..Default::default()
    }
}

// ============================================================================
// Index Management Helpers
// ============================================================================

/// Create an index if it doesn't exist, then apply settings.
///
/// This is idempotent: calling it multiple times is safe. If the index
/// already exists, only settings are updated.
fn ensure_single_index(
    meili: &MeilisearchLib,
    uid: &str,
    settings: Settings<Unchecked>,
) -> Result<(), PersonalityIndexError> {
    let exists = meili
        .index_exists(uid)
        .map_err(|e| PersonalityIndexError::Check {
            index: uid.to_string(),
            detail: e.to_string(),
        })?;

    if !exists {
        log::info!("Index '{}' not found, creating...", uid);
        let task = meili
            .create_index(uid, Some("id".to_string()))
            .map_err(|e| PersonalityIndexError::Create {
                index: uid.to_string(),
                detail: e.to_string(),
            })?;
        meili
            .wait_for_task(task.uid, Some(INDEX_TIMEOUT))
            .map_err(|e| PersonalityIndexError::TaskWaitFailed {
                index: uid.to_string(),
                detail: e.to_string(),
            })?;
    }

    let task = meili
        .update_settings(uid, settings)
        .map_err(|e| PersonalityIndexError::Settings {
            index: uid.to_string(),
            detail: e.to_string(),
        })?;
    meili
        .wait_for_task(task.uid, Some(INDEX_TIMEOUT))
        .map_err(|e| PersonalityIndexError::TaskWaitFailed {
            index: uid.to_string(),
            detail: e.to_string(),
        })?;

    log::debug!("Configured index '{}'", uid);
    Ok(())
}

/// Get the document count for an index.
///
/// Returns `Ok(0)` if the index does not exist, propagates errors from
/// `index_exists` and `index_stats` calls.
fn get_document_count(meili: &MeilisearchLib, uid: &str) -> Result<u64, PersonalityIndexError> {
    if !meili.index_exists(uid).map_err(|e| PersonalityIndexError::Check {
        index: uid.to_string(),
        detail: e.to_string(),
    })? {
        return Ok(0);
    }

    meili
        .index_stats(uid)
        .map(|stats| stats.number_of_documents)
        .map_err(|e| PersonalityIndexError::Stats {
            index: uid.to_string(),
            detail: e.to_string(),
        })
}

// ============================================================================
// Personality Index Manager
// ============================================================================

/// Manages Meilisearch indexes for the personality system.
///
/// Uses embedded `MeilisearchLib` for direct synchronous access without HTTP.
pub struct PersonalityIndexManager {
    meili: Arc<MeilisearchLib>,
}

impl PersonalityIndexManager {
    /// Create a new index manager from a shared `MeilisearchLib` instance.
    pub fn new(meili: Arc<MeilisearchLib>) -> Self {
        Self { meili }
    }

    /// Get a reference to the underlying `MeilisearchLib`.
    pub fn meili(&self) -> &MeilisearchLib {
        &self.meili
    }

    /// Initialize both personality indexes with appropriate settings.
    ///
    /// This should be called during application startup. It is idempotent:
    /// calling it multiple times is safe.
    pub fn initialize_indexes(&self) -> Result<(), PersonalityExtensionError> {
        ensure_single_index(&self.meili, INDEX_PERSONALITY_TEMPLATES, personality_templates_settings())?;
        ensure_single_index(&self.meili, INDEX_BLEND_RULES, blend_rules_settings())?;

        log::info!(
            "Initialized personality indexes: {}, {}",
            INDEX_PERSONALITY_TEMPLATES,
            INDEX_BLEND_RULES
        );

        Ok(())
    }

    // ========================================================================
    // Template Operations
    // ========================================================================

    /// Add or update a personality template in the index.
    pub fn upsert_template(
        &self,
        template: &SettingPersonalityTemplate,
    ) -> Result<(), PersonalityExtensionError> {
        let doc: TemplateDocument = template.clone().into();
        let doc_value = serde_json::to_value(&doc).map_err(|e| {
            PersonalityExtensionError::internal(format!("Failed to serialize template: {}", e))
        })?;

        let task = self
            .meili
            .add_documents(INDEX_PERSONALITY_TEMPLATES, vec![doc_value], Some("id".to_string()))
            .map_err(|e| PersonalityIndexError::AddDocuments {
                index: INDEX_PERSONALITY_TEMPLATES.to_string(),
                detail: e.to_string(),
            })?;

        self.meili
            .wait_for_task(task.uid, Some(INDEX_TIMEOUT))
            .map_err(|e| PersonalityIndexError::TaskWaitFailed {
                index: INDEX_PERSONALITY_TEMPLATES.to_string(),
                detail: e.to_string(),
            })?;

        log::debug!("Upserted template: {} ({})", template.name, template.id);
        Ok(())
    }

    /// Get a template by ID.
    pub fn get_template(
        &self,
        id: &TemplateId,
    ) -> Result<Option<TemplateDocument>, PersonalityExtensionError> {
        match self.meili.get_document(INDEX_PERSONALITY_TEMPLATES, id.as_str()) {
            Ok(value) => {
                let doc: TemplateDocument = serde_json::from_value(value).map_err(|e| {
                    PersonalityExtensionError::internal(format!(
                        "Failed to deserialize template '{}': {}",
                        id, e
                    ))
                })?;
                Ok(Some(doc))
            }
            Err(meilisearch_lib::Error::DocumentNotFound(_)) => Ok(None),
            Err(e) => Err(PersonalityIndexError::GetDocument {
                index: INDEX_PERSONALITY_TEMPLATES.to_string(),
                doc_id: id.to_string(),
                detail: e.to_string(),
            }
            .into()),
        }
    }

    /// Delete a template by ID.
    pub fn delete_template(&self, id: &TemplateId) -> Result<(), PersonalityExtensionError> {
        let task = self
            .meili
            .delete_document(INDEX_PERSONALITY_TEMPLATES, id.as_str())
            .map_err(|e| PersonalityIndexError::DeleteDocument {
                index: INDEX_PERSONALITY_TEMPLATES.to_string(),
                doc_id: id.to_string(),
                detail: e.to_string(),
            })?;

        self.meili
            .wait_for_task(task.uid, Some(INDEX_TIMEOUT))
            .map_err(|e| PersonalityIndexError::TaskWaitFailed {
                index: INDEX_PERSONALITY_TEMPLATES.to_string(),
                detail: e.to_string(),
            })?;

        log::debug!("Deleted template: {}", id);
        Ok(())
    }

    /// Search templates with optional filters.
    pub fn search_templates(
        &self,
        query: &str,
        filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<TemplateDocument>, PersonalityExtensionError> {
        let mut search_query = if query.is_empty() {
            SearchQuery::empty()
        } else {
            SearchQuery::new(query)
        };

        search_query = search_query.with_pagination(0, limit);

        if let Some(f) = filter {
            search_query = search_query.with_filter(serde_json::Value::String(f.to_string()));
        }

        let result = self
            .meili
            .search(INDEX_PERSONALITY_TEMPLATES, search_query)
            .map_err(|e| PersonalityIndexError::Search {
                index: INDEX_PERSONALITY_TEMPLATES.to_string(),
                detail: e.to_string(),
            })?;

        let mut docs = Vec::with_capacity(result.hits.len());
        for hit in result.hits {
            match serde_json::from_value::<TemplateDocument>(hit.document) {
                Ok(doc) => docs.push(doc),
                Err(e) => {
                    log::error!("Failed to deserialize template search hit: {}", e);
                }
            }
        }

        Ok(docs)
    }

    /// List all templates with optional filter.
    pub fn list_templates(
        &self,
        filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<TemplateDocument>, PersonalityExtensionError> {
        self.search_templates("", filter, limit)
    }

    /// List templates by game system.
    pub fn list_templates_by_game_system(
        &self,
        game_system: &str,
        limit: usize,
    ) -> Result<Vec<TemplateDocument>, PersonalityExtensionError> {
        let filter = format!("gameSystem = \"{}\"", escape_filter_value(game_system));
        self.list_templates(Some(&filter), limit)
    }

    /// List templates by campaign.
    pub fn list_templates_by_campaign(
        &self,
        campaign_id: &str,
        limit: usize,
    ) -> Result<Vec<TemplateDocument>, PersonalityExtensionError> {
        let filter = format!("campaignId = \"{}\"", escape_filter_value(campaign_id));
        self.list_templates(Some(&filter), limit)
    }

    /// List built-in templates.
    pub fn list_builtin_templates(
        &self,
        limit: usize,
    ) -> Result<Vec<TemplateDocument>, PersonalityExtensionError> {
        self.list_templates(Some("isBuiltin = true"), limit)
    }

    // ========================================================================
    // Blend Rule Operations
    // ========================================================================

    /// Add or update a blend rule in the index.
    pub fn upsert_blend_rule(&self, rule: &BlendRule) -> Result<(), PersonalityExtensionError> {
        let doc: BlendRuleDocument = rule.clone().into();
        let doc_value = serde_json::to_value(&doc).map_err(|e| {
            PersonalityExtensionError::internal(format!("Failed to serialize blend rule: {}", e))
        })?;

        let task = self
            .meili
            .add_documents(INDEX_BLEND_RULES, vec![doc_value], Some("id".to_string()))
            .map_err(|e| PersonalityIndexError::AddDocuments {
                index: INDEX_BLEND_RULES.to_string(),
                detail: e.to_string(),
            })?;

        self.meili
            .wait_for_task(task.uid, Some(INDEX_TIMEOUT))
            .map_err(|e| PersonalityIndexError::TaskWaitFailed {
                index: INDEX_BLEND_RULES.to_string(),
                detail: e.to_string(),
            })?;

        log::debug!("Upserted blend rule: {} ({})", rule.name, rule.id);
        Ok(())
    }

    /// Get a blend rule by ID.
    pub fn get_blend_rule(
        &self,
        id: &BlendRuleId,
    ) -> Result<Option<BlendRuleDocument>, PersonalityExtensionError> {
        match self.meili.get_document(INDEX_BLEND_RULES, id.as_str()) {
            Ok(value) => {
                let doc: BlendRuleDocument = serde_json::from_value(value).map_err(|e| {
                    PersonalityExtensionError::internal(format!(
                        "Failed to deserialize blend rule '{}': {}",
                        id, e
                    ))
                })?;
                Ok(Some(doc))
            }
            Err(meilisearch_lib::Error::DocumentNotFound(_)) => Ok(None),
            Err(e) => Err(PersonalityIndexError::GetDocument {
                index: INDEX_BLEND_RULES.to_string(),
                doc_id: id.to_string(),
                detail: e.to_string(),
            }
            .into()),
        }
    }

    /// Delete a blend rule by ID.
    pub fn delete_blend_rule(&self, id: &BlendRuleId) -> Result<(), PersonalityExtensionError> {
        let task = self
            .meili
            .delete_document(INDEX_BLEND_RULES, id.as_str())
            .map_err(|e| PersonalityIndexError::DeleteDocument {
                index: INDEX_BLEND_RULES.to_string(),
                doc_id: id.to_string(),
                detail: e.to_string(),
            })?;

        self.meili
            .wait_for_task(task.uid, Some(INDEX_TIMEOUT))
            .map_err(|e| PersonalityIndexError::TaskWaitFailed {
                index: INDEX_BLEND_RULES.to_string(),
                detail: e.to_string(),
            })?;

        log::debug!("Deleted blend rule: {}", id);
        Ok(())
    }

    /// Search blend rules with optional filters.
    pub fn search_blend_rules(
        &self,
        query: &str,
        filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<BlendRuleDocument>, PersonalityExtensionError> {
        let mut search_query = if query.is_empty() {
            SearchQuery::empty()
        } else {
            SearchQuery::new(query)
        };

        search_query = search_query.with_pagination(0, limit);

        if let Some(f) = filter {
            search_query = search_query.with_filter(serde_json::Value::String(f.to_string()));
        }

        let result = self
            .meili
            .search(INDEX_BLEND_RULES, search_query)
            .map_err(|e| PersonalityIndexError::Search {
                index: INDEX_BLEND_RULES.to_string(),
                detail: e.to_string(),
            })?;

        let mut docs = Vec::with_capacity(result.hits.len());
        for hit in result.hits {
            match serde_json::from_value::<BlendRuleDocument>(hit.document) {
                Ok(doc) => docs.push(doc),
                Err(e) => {
                    log::error!("Failed to deserialize blend rule search hit: {}", e);
                }
            }
        }

        Ok(docs)
    }

    /// List all blend rules with optional filter.
    pub fn list_blend_rules(
        &self,
        filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<BlendRuleDocument>, PersonalityExtensionError> {
        self.search_blend_rules("", filter, limit)
    }

    /// List blend rules for a specific context.
    pub fn list_rules_by_context(
        &self,
        context: &str,
        limit: usize,
    ) -> Result<Vec<BlendRuleDocument>, PersonalityExtensionError> {
        let filter = format!("context = \"{}\"", escape_filter_value(context));
        self.list_blend_rules(Some(&filter), limit)
    }

    /// List blend rules that are enabled (i.e., `enabled = true`), ordered by
    /// priority (descending). Disabled rules are excluded from results.
    pub fn list_enabled_rules(
        &self,
        limit: usize,
    ) -> Result<Vec<BlendRuleDocument>, PersonalityExtensionError> {
        let search_query = SearchQuery::empty()
            .with_filter(serde_json::Value::String("enabled = true".to_string()))
            .with_sort(vec!["priority:desc".to_string()])
            .with_pagination(0, limit);

        let result = self
            .meili
            .search(INDEX_BLEND_RULES, search_query)
            .map_err(|e| PersonalityIndexError::Search {
                index: INDEX_BLEND_RULES.to_string(),
                detail: e.to_string(),
            })?;

        let mut docs = Vec::with_capacity(result.hits.len());
        for hit in result.hits {
            match serde_json::from_value::<BlendRuleDocument>(hit.document) {
                Ok(doc) => docs.push(doc),
                Err(e) => {
                    log::error!("Failed to deserialize enabled rule search hit: {}", e);
                }
            }
        }

        Ok(docs)
    }

    /// List blend rules by campaign.
    pub fn list_rules_by_campaign(
        &self,
        campaign_id: &str,
        limit: usize,
    ) -> Result<Vec<BlendRuleDocument>, PersonalityExtensionError> {
        let filter = format!("campaignId = \"{}\"", escape_filter_value(campaign_id));
        self.list_blend_rules(Some(&filter), limit)
    }

    // ========================================================================
    // Utility Operations
    // ========================================================================

    /// Get document counts for both indexes.
    pub fn get_stats(&self) -> Result<PersonalityIndexStats, PersonalityExtensionError> {
        Ok(PersonalityIndexStats {
            template_count: get_document_count(&self.meili, INDEX_PERSONALITY_TEMPLATES)?,
            rule_count: get_document_count(&self.meili, INDEX_BLEND_RULES)?,
        })
    }

    /// Clear all documents from the templates index.
    pub fn clear_templates(&self) -> Result<(), PersonalityExtensionError> {
        let task = self
            .meili
            .delete_all_documents(INDEX_PERSONALITY_TEMPLATES)
            .map_err(|e| PersonalityIndexError::Clear {
                index: INDEX_PERSONALITY_TEMPLATES.to_string(),
                detail: e.to_string(),
            })?;

        self.meili
            .wait_for_task(task.uid, Some(INDEX_TIMEOUT))
            .map_err(|e| PersonalityIndexError::TaskWaitFailed {
                index: INDEX_PERSONALITY_TEMPLATES.to_string(),
                detail: e.to_string(),
            })?;

        log::info!("Cleared all templates");
        Ok(())
    }

    /// Clear all documents from the blend rules index.
    pub fn clear_blend_rules(&self) -> Result<(), PersonalityExtensionError> {
        let task = self
            .meili
            .delete_all_documents(INDEX_BLEND_RULES)
            .map_err(|e| PersonalityIndexError::Clear {
                index: INDEX_BLEND_RULES.to_string(),
                detail: e.to_string(),
            })?;

        self.meili
            .wait_for_task(task.uid, Some(INDEX_TIMEOUT))
            .map_err(|e| PersonalityIndexError::TaskWaitFailed {
                index: INDEX_BLEND_RULES.to_string(),
                detail: e.to_string(),
            })?;

        log::info!("Cleared all blend rules");
        Ok(())
    }

    /// Delete both personality indexes entirely.
    ///
    /// Logs warnings for individual index deletion failures but continues
    /// to attempt all deletions. This is intentional since cleanup should
    /// be best-effort - partial cleanup is better than failing early.
    pub fn delete_indexes(&self) -> Result<(), PersonalityExtensionError> {
        // Delete templates index
        match self.meili.delete_index(INDEX_PERSONALITY_TEMPLATES) {
            Ok(task) => {
                if let Err(e) = self.meili.wait_for_task(task.uid, Some(INDEX_TIMEOUT)) {
                    log::warn!("Failed to wait for templates index deletion: {}", e);
                }
            }
            Err(e) => {
                log::warn!("Failed to delete templates index: {}", e);
            }
        }

        // Delete rules index
        match self.meili.delete_index(INDEX_BLEND_RULES) {
            Ok(task) => {
                if let Err(e) = self.meili.wait_for_task(task.uid, Some(INDEX_TIMEOUT)) {
                    log::warn!("Failed to wait for blend rules index deletion: {}", e);
                }
            }
            Err(e) => {
                log::warn!("Failed to delete blend rules index: {}", e);
            }
        }

        log::info!("Deleted personality indexes");
        Ok(())
    }
}

// ============================================================================
// Stats Type
// ============================================================================

/// Statistics for personality indexes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalityIndexStats {
    /// Number of templates in the index.
    pub template_count: u64,

    /// Number of blend rules in the index.
    pub rule_count: u64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_constants() {
        assert_eq!(INDEX_PERSONALITY_TEMPLATES, "ttrpg_personality_templates");
        assert_eq!(INDEX_BLEND_RULES, "ttrpg_blend_rules");
    }

    #[test]
    fn test_personality_templates_settings() {
        let settings = personality_templates_settings();
        // Settings should be configured (verify no panic)
        let _ = settings;
    }

    #[test]
    fn test_blend_rules_settings() {
        let settings = blend_rules_settings();
        // Settings should be configured (verify no panic)
        let _ = settings;
    }

    #[test]
    fn test_personality_index_stats_serialization() {
        let stats = PersonalityIndexStats {
            template_count: 10,
            rule_count: 5,
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"templateCount\""));
        assert!(json.contains("\"ruleCount\""));

        let parsed: PersonalityIndexStats = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.template_count, 10);
        assert_eq!(parsed.rule_count, 5);
    }

    #[test]
    fn test_escape_filter_value() {
        assert_eq!(escape_filter_value("simple"), "simple");
        assert_eq!(escape_filter_value(r#"with "quotes""#), r#"with \"quotes\""#);
        assert_eq!(escape_filter_value(r"with \backslash"), r"with \\backslash");
    }

    #[test]
    fn test_personality_index_error_display() {
        let err = PersonalityIndexError::Check {
            index: "test_index".to_string(),
            detail: "connection failed".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Failed to check index 'test_index': connection failed"
        );

        let err = PersonalityIndexError::AddDocuments {
            index: "test_index".to_string(),
            detail: "serialization error".to_string(),
        };
        assert!(err.to_string().contains("test_index"));
        assert!(err.to_string().contains("serialization error"));

        let err = PersonalityIndexError::GetDocument {
            index: "test_index".to_string(),
            doc_id: "doc-123".to_string(),
            detail: "not found".to_string(),
        };
        assert!(err.to_string().contains("doc-123"));

        let err = PersonalityIndexError::TaskWaitFailed {
            index: "test_index".to_string(),
            detail: "timeout exceeded".to_string(),
        };
        assert!(err.to_string().contains("test_index"));
        assert!(err.to_string().contains("timeout exceeded"));
    }

    #[test]
    fn test_personality_index_error_to_string() {
        let err = PersonalityIndexError::Search {
            index: "test".to_string(),
            detail: "query failed".to_string(),
        };
        let s: String = err.into();
        assert!(s.contains("Search failed"));
    }
}
