//! Meilisearch Index Configuration for Personality System
//!
//! Defines the Meilisearch indexes, settings, and operations for the
//! personality template and blend rule storage.

use super::errors::{PersonalityExtensionError, TemplateError, BlendRuleError};
use super::types::{
    BlendRule, BlendRuleDocument, SettingPersonalityTemplate, TemplateDocument, TemplateId,
    BlendRuleId,
};
use meilisearch_sdk::client::Client;
use meilisearch_sdk::indexes::Index;
use meilisearch_sdk::search::SearchResults;
use meilisearch_sdk::settings::Settings;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ============================================================================
// Index Constants
// ============================================================================

/// Index name for personality templates.
pub const INDEX_PERSONALITY_TEMPLATES: &str = "ttrpg_personality_templates";

/// Index name for blend rules.
pub const INDEX_BLEND_RULES: &str = "ttrpg_blend_rules";

/// Default timeout for index operations (30 seconds).
pub const INDEX_TASK_TIMEOUT_SECS: u64 = 30;

/// Polling interval for task completion (100ms).
pub const INDEX_TASK_POLL_MS: u64 = 100;

// ============================================================================
// Filter Safety
// ============================================================================

/// Escape a value for safe use in Meilisearch filter expressions.
/// Escapes backslashes and double quotes to prevent filter injection.
fn escape_filter_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

// ============================================================================
// Index Settings
// ============================================================================

/// Get the settings configuration for the personality templates index.
pub fn personality_templates_settings() -> Settings {
    Settings::new()
        // Searchable attributes for full-text search
        .with_searchable_attributes([
            "name",
            "description",
            "vocabularyKeys",
            "commonPhrases",
        ])
        // Filterable attributes for faceted search
        .with_filterable_attributes([
            "gameSystem",
            "settingName",
            "isBuiltin",
            "tags",
            "campaignId",
        ])
        // Sortable attributes for ordering
        .with_sortable_attributes([
            "name",
            "createdAt",
            "updatedAt",
        ])
}

/// Get the settings configuration for the blend rules index.
pub fn blend_rules_settings() -> Settings {
    Settings::new()
        // Searchable attributes for full-text search
        .with_searchable_attributes([
            "name",
            "description",
        ])
        // Filterable attributes for faceted search
        .with_filterable_attributes([
            "context",
            "enabled",
            "isBuiltin",
            "tags",
            "campaignId",
        ])
        // Sortable attributes for ordering
        .with_sortable_attributes([
            "name",
            "priority",
            "createdAt",
            "updatedAt",
        ])
}

// ============================================================================
// Personality Index Manager
// ============================================================================

/// Manages Meilisearch indexes for the personality system.
pub struct PersonalityIndexManager {
    client: Client,
    #[allow(dead_code)]
    host: String,
    #[allow(dead_code)]
    api_key: Option<String>,
}

impl PersonalityIndexManager {
    /// Create a new index manager.
    pub fn new(host: &str, api_key: Option<&str>) -> Self {
        Self {
            client: Client::new(host, api_key).expect("Failed to create Meilisearch client"),
            host: host.to_string(),
            api_key: api_key.map(|s| s.to_string()),
        }
    }

    /// Get the Meilisearch host URL.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Get the underlying Meilisearch client.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Initialize both personality indexes with appropriate settings.
    ///
    /// This should be called during application startup.
    pub async fn initialize_indexes(&self) -> Result<(), PersonalityExtensionError> {
        // Create/update templates index
        self.ensure_templates_index().await?;

        // Create/update blend rules index
        self.ensure_blend_rules_index().await?;

        log::info!(
            "Initialized personality indexes: {}, {}",
            INDEX_PERSONALITY_TEMPLATES,
            INDEX_BLEND_RULES
        );

        Ok(())
    }

    /// Ensure the templates index exists with correct settings.
    async fn ensure_templates_index(&self) -> Result<Index, PersonalityExtensionError> {
        let index = self.ensure_index(INDEX_PERSONALITY_TEMPLATES).await?;

        let settings = personality_templates_settings();
        let task = index.set_settings(&settings).await.map_err(|e| {
            TemplateError::MeilisearchError {
                template_id: String::new(),
                message: format!("Failed to set index settings: {}", e),
            }
        })?;

        task.wait_for_completion(
            &self.client,
            Some(Duration::from_millis(INDEX_TASK_POLL_MS)),
            Some(Duration::from_secs(INDEX_TASK_TIMEOUT_SECS)),
        )
        .await
        .map_err(|e| TemplateError::MeilisearchError {
            template_id: String::new(),
            message: format!("Failed to wait for settings task: {}", e),
        })?;

        Ok(index)
    }

    /// Ensure the blend rules index exists with correct settings.
    async fn ensure_blend_rules_index(&self) -> Result<Index, PersonalityExtensionError> {
        let index = self.ensure_index(INDEX_BLEND_RULES).await?;

        let settings = blend_rules_settings();
        let task = index.set_settings(&settings).await.map_err(|e| {
            BlendRuleError::MeilisearchError {
                rule_id: String::new(),
                message: format!("Failed to set index settings: {}", e),
            }
        })?;

        task.wait_for_completion(
            &self.client,
            Some(Duration::from_millis(INDEX_TASK_POLL_MS)),
            Some(Duration::from_secs(INDEX_TASK_TIMEOUT_SECS)),
        )
        .await
        .map_err(|e| BlendRuleError::MeilisearchError {
            rule_id: String::new(),
            message: format!("Failed to wait for settings task: {}", e),
        })?;

        Ok(index)
    }

    /// Ensure an index exists, creating it if necessary.
    async fn ensure_index(&self, name: &str) -> Result<Index, PersonalityExtensionError> {
        match self.client.get_index(name).await {
            Ok(idx) => Ok(idx),
            Err(_) => {
                let task = self
                    .client
                    .create_index(name, Some("id"))
                    .await
                    .map_err(|e| {
                        PersonalityExtensionError::internal(format!(
                            "Failed to create index '{}': {}",
                            name, e
                        ))
                    })?;

                task.wait_for_completion(
                    &self.client,
                    Some(Duration::from_millis(INDEX_TASK_POLL_MS)),
                    Some(Duration::from_secs(INDEX_TASK_TIMEOUT_SECS)),
                )
                .await
                .map_err(|e| {
                    PersonalityExtensionError::internal(format!(
                        "Failed to wait for index creation '{}': {}",
                        name, e
                    ))
                })?;

                Ok(self.client.index(name))
            }
        }
    }

    // ========================================================================
    // Template Operations
    // ========================================================================

    /// Add or update a personality template in the index.
    pub async fn upsert_template(
        &self,
        template: &SettingPersonalityTemplate,
    ) -> Result<(), PersonalityExtensionError> {
        let index = self.client.index(INDEX_PERSONALITY_TEMPLATES);
        let doc: TemplateDocument = template.clone().into();

        let task = index.add_documents(&[doc], Some("id")).await.map_err(|e| {
            TemplateError::MeilisearchError {
                template_id: template.id.to_string(),
                message: format!("Failed to add template: {}", e),
            }
        })?;

        task.wait_for_completion(
            &self.client,
            Some(Duration::from_millis(INDEX_TASK_POLL_MS)),
            Some(Duration::from_secs(INDEX_TASK_TIMEOUT_SECS)),
        )
        .await
        .map_err(|e| TemplateError::MeilisearchError {
            template_id: template.id.to_string(),
            message: format!("Failed to wait for add task: {}", e),
        })?;

        log::debug!("Upserted template: {} ({})", template.name, template.id);
        Ok(())
    }

    /// Get a template by ID.
    pub async fn get_template(
        &self,
        id: &TemplateId,
    ) -> Result<Option<TemplateDocument>, PersonalityExtensionError> {
        let index = self.client.index(INDEX_PERSONALITY_TEMPLATES);

        match index.get_document::<TemplateDocument>(id.as_str()).await {
            Ok(doc) => Ok(Some(doc)),
            Err(meilisearch_sdk::errors::Error::Meilisearch(e))
                if e.error_code == meilisearch_sdk::errors::ErrorCode::DocumentNotFound =>
            {
                Ok(None)
            }
            Err(e) => Err(TemplateError::MeilisearchError {
                template_id: id.to_string(),
                message: format!("Failed to get template: {}", e),
            }
            .into()),
        }
    }

    /// Delete a template by ID.
    pub async fn delete_template(&self, id: &TemplateId) -> Result<(), PersonalityExtensionError> {
        let index = self.client.index(INDEX_PERSONALITY_TEMPLATES);

        let task = index.delete_document(id.as_str()).await.map_err(|e| {
            TemplateError::MeilisearchError {
                template_id: id.to_string(),
                message: format!("Failed to delete template: {}", e),
            }
        })?;

        task.wait_for_completion(
            &self.client,
            Some(Duration::from_millis(INDEX_TASK_POLL_MS)),
            Some(Duration::from_secs(INDEX_TASK_TIMEOUT_SECS)),
        )
        .await
        .map_err(|e| TemplateError::MeilisearchError {
            template_id: id.to_string(),
            message: format!("Failed to wait for delete task: {}", e),
        })?;

        log::debug!("Deleted template: {}", id);
        Ok(())
    }

    /// Search templates with optional filters.
    pub async fn search_templates(
        &self,
        query: &str,
        filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<TemplateDocument>, PersonalityExtensionError> {
        let index = self.client.index(INDEX_PERSONALITY_TEMPLATES);

        let mut search = index.search();
        search.with_query(query).with_limit(limit);

        if let Some(f) = filter {
            search.with_filter(f);
        }

        let results: SearchResults<TemplateDocument> =
            search.execute().await.map_err(|e| {
                PersonalityExtensionError::internal(format!("Template search failed: {}", e))
            })?;

        Ok(results.hits.into_iter().map(|h| h.result).collect())
    }

    /// List all templates with optional filter.
    pub async fn list_templates(
        &self,
        filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<TemplateDocument>, PersonalityExtensionError> {
        self.search_templates("", filter, limit).await
    }

    /// List templates by game system.
    pub async fn list_templates_by_game_system(
        &self,
        game_system: &str,
        limit: usize,
    ) -> Result<Vec<TemplateDocument>, PersonalityExtensionError> {
        let filter = format!("gameSystem = \"{}\"", escape_filter_value(game_system));
        self.list_templates(Some(&filter), limit).await
    }

    /// List templates by campaign.
    pub async fn list_templates_by_campaign(
        &self,
        campaign_id: &str,
        limit: usize,
    ) -> Result<Vec<TemplateDocument>, PersonalityExtensionError> {
        let filter = format!("campaignId = \"{}\"", escape_filter_value(campaign_id));
        self.list_templates(Some(&filter), limit).await
    }

    /// List built-in templates.
    pub async fn list_builtin_templates(
        &self,
        limit: usize,
    ) -> Result<Vec<TemplateDocument>, PersonalityExtensionError> {
        self.list_templates(Some("isBuiltin = true"), limit).await
    }

    // ========================================================================
    // Blend Rule Operations
    // ========================================================================

    /// Add or update a blend rule in the index.
    pub async fn upsert_blend_rule(&self, rule: &BlendRule) -> Result<(), PersonalityExtensionError> {
        let index = self.client.index(INDEX_BLEND_RULES);
        let doc: BlendRuleDocument = rule.clone().into();

        let task = index.add_documents(&[doc], Some("id")).await.map_err(|e| {
            BlendRuleError::MeilisearchError {
                rule_id: rule.id.to_string(),
                message: format!("Failed to add rule: {}", e),
            }
        })?;

        task.wait_for_completion(
            &self.client,
            Some(Duration::from_millis(INDEX_TASK_POLL_MS)),
            Some(Duration::from_secs(INDEX_TASK_TIMEOUT_SECS)),
        )
        .await
        .map_err(|e| BlendRuleError::MeilisearchError {
            rule_id: rule.id.to_string(),
            message: format!("Failed to wait for add task: {}", e),
        })?;

        log::debug!("Upserted blend rule: {} ({})", rule.name, rule.id);
        Ok(())
    }

    /// Get a blend rule by ID.
    pub async fn get_blend_rule(
        &self,
        id: &BlendRuleId,
    ) -> Result<Option<BlendRuleDocument>, PersonalityExtensionError> {
        let index = self.client.index(INDEX_BLEND_RULES);

        match index.get_document::<BlendRuleDocument>(id.as_str()).await {
            Ok(doc) => Ok(Some(doc)),
            Err(meilisearch_sdk::errors::Error::Meilisearch(e))
                if e.error_code == meilisearch_sdk::errors::ErrorCode::DocumentNotFound =>
            {
                Ok(None)
            }
            Err(e) => Err(BlendRuleError::MeilisearchError {
                rule_id: id.to_string(),
                message: format!("Failed to get rule: {}", e),
            }
            .into()),
        }
    }

    /// Delete a blend rule by ID.
    pub async fn delete_blend_rule(&self, id: &BlendRuleId) -> Result<(), PersonalityExtensionError> {
        let index = self.client.index(INDEX_BLEND_RULES);

        let task = index.delete_document(id.as_str()).await.map_err(|e| {
            BlendRuleError::MeilisearchError {
                rule_id: id.to_string(),
                message: format!("Failed to delete rule: {}", e),
            }
        })?;

        task.wait_for_completion(
            &self.client,
            Some(Duration::from_millis(INDEX_TASK_POLL_MS)),
            Some(Duration::from_secs(INDEX_TASK_TIMEOUT_SECS)),
        )
        .await
        .map_err(|e| BlendRuleError::MeilisearchError {
            rule_id: id.to_string(),
            message: format!("Failed to wait for delete task: {}", e),
        })?;

        log::debug!("Deleted blend rule: {}", id);
        Ok(())
    }

    /// Search blend rules with optional filters.
    pub async fn search_blend_rules(
        &self,
        query: &str,
        filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<BlendRuleDocument>, PersonalityExtensionError> {
        let index = self.client.index(INDEX_BLEND_RULES);

        let mut search = index.search();
        search.with_query(query).with_limit(limit);

        if let Some(f) = filter {
            search.with_filter(f);
        }

        let results: SearchResults<BlendRuleDocument> =
            search.execute().await.map_err(|e| {
                PersonalityExtensionError::internal(format!("Blend rule search failed: {}", e))
            })?;

        Ok(results.hits.into_iter().map(|h| h.result).collect())
    }

    /// List all blend rules with optional filter.
    pub async fn list_blend_rules(
        &self,
        filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<BlendRuleDocument>, PersonalityExtensionError> {
        self.search_blend_rules("", filter, limit).await
    }

    /// List blend rules for a specific context.
    pub async fn list_rules_by_context(
        &self,
        context: &str,
        limit: usize,
    ) -> Result<Vec<BlendRuleDocument>, PersonalityExtensionError> {
        let filter = format!("context = \"{}\"", context);
        self.list_blend_rules(Some(&filter), limit).await
    }

    /// List enabled blend rules ordered by priority.
    pub async fn list_enabled_rules(
        &self,
        limit: usize,
    ) -> Result<Vec<BlendRuleDocument>, PersonalityExtensionError> {
        let index = self.client.index(INDEX_BLEND_RULES);

        let results: SearchResults<BlendRuleDocument> = index
            .search()
            .with_query("")
            .with_filter("enabled = true")
            .with_sort(&["priority:desc"])
            .with_limit(limit)
            .execute()
            .await
            .map_err(|e| {
                PersonalityExtensionError::internal(format!("List enabled rules failed: {}", e))
            })?;

        Ok(results.hits.into_iter().map(|h| h.result).collect())
    }

    /// List blend rules by campaign.
    pub async fn list_rules_by_campaign(
        &self,
        campaign_id: &str,
        limit: usize,
    ) -> Result<Vec<BlendRuleDocument>, PersonalityExtensionError> {
        let filter = format!("campaignId = \"{}\"", campaign_id);
        self.list_blend_rules(Some(&filter), limit).await
    }

    // ========================================================================
    // Utility Operations
    // ========================================================================

    /// Get document counts for both indexes.
    pub async fn get_stats(&self) -> Result<PersonalityIndexStats, PersonalityExtensionError> {
        let templates_index = self.client.index(INDEX_PERSONALITY_TEMPLATES);
        let rules_index = self.client.index(INDEX_BLEND_RULES);

        let templates_stats = templates_index.get_stats().await.map_err(|e| {
            PersonalityExtensionError::internal(format!("Failed to get template stats: {}", e))
        })?;

        let rules_stats = rules_index.get_stats().await.map_err(|e| {
            PersonalityExtensionError::internal(format!("Failed to get rules stats: {}", e))
        })?;

        Ok(PersonalityIndexStats {
            template_count: templates_stats.number_of_documents as u64,
            rule_count: rules_stats.number_of_documents as u64,
        })
    }

    /// Clear all documents from the templates index.
    pub async fn clear_templates(&self) -> Result<(), PersonalityExtensionError> {
        let index = self.client.index(INDEX_PERSONALITY_TEMPLATES);

        let task = index.delete_all_documents().await.map_err(|e| {
            PersonalityExtensionError::internal(format!("Failed to clear templates: {}", e))
        })?;

        task.wait_for_completion(
            &self.client,
            Some(Duration::from_millis(INDEX_TASK_POLL_MS)),
            Some(Duration::from_secs(INDEX_TASK_TIMEOUT_SECS)),
        )
        .await
        .map_err(|e| {
            PersonalityExtensionError::internal(format!(
                "Failed to wait for clear templates task: {}",
                e
            ))
        })?;

        log::info!("Cleared all templates");
        Ok(())
    }

    /// Clear all documents from the blend rules index.
    pub async fn clear_blend_rules(&self) -> Result<(), PersonalityExtensionError> {
        let index = self.client.index(INDEX_BLEND_RULES);

        let task = index.delete_all_documents().await.map_err(|e| {
            PersonalityExtensionError::internal(format!("Failed to clear blend rules: {}", e))
        })?;

        task.wait_for_completion(
            &self.client,
            Some(Duration::from_millis(INDEX_TASK_POLL_MS)),
            Some(Duration::from_secs(INDEX_TASK_TIMEOUT_SECS)),
        )
        .await
        .map_err(|e| {
            PersonalityExtensionError::internal(format!(
                "Failed to wait for clear rules task: {}",
                e
            ))
        })?;

        log::info!("Cleared all blend rules");
        Ok(())
    }

    /// Delete both personality indexes entirely.
    ///
    /// Logs warnings for individual index deletion failures but continues
    /// to attempt all deletions. This is intentional since cleanup should
    /// be best-effort - partial cleanup is better than failing early.
    pub async fn delete_indexes(&self) -> Result<(), PersonalityExtensionError> {
        // Delete templates index
        match self.client.delete_index(INDEX_PERSONALITY_TEMPLATES).await {
            Ok(task) => {
                if let Err(e) = task
                    .wait_for_completion(
                        &self.client,
                        Some(Duration::from_millis(INDEX_TASK_POLL_MS)),
                        Some(Duration::from_secs(INDEX_TASK_TIMEOUT_SECS)),
                    )
                    .await
                {
                    log::warn!("Failed to wait for templates index deletion: {}", e);
                }
            }
            Err(e) => {
                log::warn!("Failed to delete templates index: {}", e);
            }
        }

        // Delete rules index
        match self.client.delete_index(INDEX_BLEND_RULES).await {
            Ok(task) => {
                if let Err(e) = task
                    .wait_for_completion(
                        &self.client,
                        Some(Duration::from_millis(INDEX_TASK_POLL_MS)),
                        Some(Duration::from_secs(INDEX_TASK_TIMEOUT_SECS)),
                    )
                    .await
                {
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
        // Settings should be configured (we can't easily inspect them, but ensure no panic)
        let _ = settings;
    }

    #[test]
    fn test_blend_rules_settings() {
        let settings = blend_rules_settings();
        // Settings should be configured (we can't easily inspect them, but ensure no panic)
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
    fn test_index_manager_creation() {
        // This test only creates the manager object (no network calls)
        let manager = PersonalityIndexManager::new("http://localhost:7700", None);
        assert_eq!(manager.host(), "http://localhost:7700");
    }
}
