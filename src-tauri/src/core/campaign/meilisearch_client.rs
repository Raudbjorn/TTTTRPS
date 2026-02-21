//! Meilisearch Campaign Client
//!
//! Typed client wrapper for embedded Meilisearch operations specific to campaign generation.
//! Provides clean API over meilisearch-lib with retry logic and batch operations.
//!
//! Migrated from `meilisearch_sdk` to embedded `meilisearch_lib` (TASK-CAMP-004).

use meilisearch_lib::{MeilisearchLib, SearchQuery, Settings, Unchecked};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

use super::meilisearch_indexes::{
    get_index_configs, INDEX_CAMPAIGN_ARCS, INDEX_PLOT_POINTS, INDEX_SESSION_PLANS,
};

// ============================================================================
// Constants
// ============================================================================

/// Maximum batch size for document operations
pub const MEILISEARCH_BATCH_SIZE: usize = 1000;

/// Default timeout for short operations (single entity CRUD)
pub const TASK_TIMEOUT_SHORT_SECS: u64 = 30;

/// Default timeout for long operations (batch, index creation)
pub const TASK_TIMEOUT_LONG_SECS: u64 = 300;

/// Primary key used by all campaign indexes (see [`IndexConfig::primary_key`]).
///
/// Centralised here so that `upsert_document`/`upsert_documents` and
/// `delete_by_filter` share a single source of truth.
const CAMPAIGN_PRIMARY_KEY: &str = "id";

/// Maximum retry attempts for transient errors
const MAX_RETRY_ATTEMPTS: u32 = 3;

/// Base delay for exponential backoff (milliseconds)
const RETRY_BASE_DELAY_MS: u64 = 100;

// ============================================================================
// Filter Safety
// ============================================================================

/// Escape a value for safe use in Meilisearch filter expressions.
/// Escapes backslashes and double quotes to prevent filter injection.
fn escape_filter_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum MeilisearchCampaignError {
    #[error("Meilisearch connection error: {0}")]
    ConnectionError(String),

    #[error("Meilisearch operation error: {0}")]
    OperationError(String),

    #[error("Document not found: {index}/{id}")]
    DocumentNotFound { index: String, id: String },

    #[error("Index not found: {0}")]
    IndexNotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Task timeout: operation did not complete within {0} seconds")]
    TaskTimeout(u64),

    #[error("Health check failed: Meilisearch is not available")]
    HealthCheckFailed,

    #[error("Batch operation failed: {0}")]
    BatchOperationFailed(String),
}

impl From<meilisearch_lib::Error> for MeilisearchCampaignError {
    fn from(e: meilisearch_lib::Error) -> Self {
        MeilisearchCampaignError::OperationError(e.to_string())
    }
}

impl From<serde_json::Error> for MeilisearchCampaignError {
    fn from(e: serde_json::Error) -> Self {
        MeilisearchCampaignError::SerializationError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, MeilisearchCampaignError>;

// ============================================================================
// Meilisearch Campaign Client
// ============================================================================

/// Client for campaign generation Meilisearch operations.
///
/// Uses embedded `MeilisearchLib` for direct Rust integration without HTTP.
/// All operations are synchronous.
pub struct MeilisearchCampaignClient {
    meili: Arc<MeilisearchLib>,
}

impl MeilisearchCampaignClient {
    /// Create a new campaign client wrapping an embedded MeilisearchLib instance
    pub fn new(meili: Arc<MeilisearchLib>) -> Self {
        Self { meili }
    }

    // ========================================================================
    // Health Check (REC-MEIL-003)
    // ========================================================================

    /// Check if Meilisearch is healthy.
    ///
    /// For embedded instances this should always be `true` once initialized,
    /// but we check the reported status rather than assuming availability.
    pub fn health_check(&self) -> bool {
        let health = self.meili.health();
        health.status == "available"
    }

    /// Wait for Meilisearch to become healthy with timeout.
    ///
    /// For embedded instances this returns immediately since the engine is
    /// always available once initialized.
    pub fn wait_for_health(&self, _timeout_secs: u64) -> bool {
        self.health_check()
    }

    // ========================================================================
    // Index Management (TASK-CAMP-005)
    // ========================================================================

    /// Ensure all campaign generation indexes exist with correct settings
    pub fn ensure_indexes(&self) -> Result<()> {
        for config in get_index_configs() {
            self.ensure_index(config.name, config.primary_key, config.settings)?;
        }

        log::info!(
            "Ensured campaign generation indexes: {}, {}, {}",
            INDEX_CAMPAIGN_ARCS,
            INDEX_SESSION_PLANS,
            INDEX_PLOT_POINTS
        );

        Ok(())
    }

    /// Ensure a single index exists with the given settings
    fn ensure_index(
        &self,
        name: &str,
        primary_key: &str,
        settings: Settings<Unchecked>,
    ) -> Result<()> {
        let exists = self.meili.index_exists(name)?;

        if exists {
            // Index exists, update settings
            let task = self.meili.update_settings(name, settings)?;
            self.meili
                .wait_for_task(task.uid, Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)))?;
            log::debug!("Updated settings for index '{}'", name);
        } else {
            // Create new index
            let task = self
                .meili
                .create_index(name, Some(primary_key.to_string()))?;
            self.meili
                .wait_for_task(task.uid, Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)))?;

            // Apply settings
            let task = self.meili.update_settings(name, settings)?;
            self.meili
                .wait_for_task(task.uid, Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)))?;

            log::info!("Created index '{}' with settings", name);
        }

        Ok(())
    }

    /// Delete an index (idempotent — treats `IndexNotFound` as success).
    pub fn delete_index(&self, name: &str) -> Result<()> {
        match self.meili.delete_index(name) {
            Ok(task) => {
                self.meili
                    .wait_for_task(task.uid, Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)))?;
                log::info!("Deleted index '{}'", name);
                Ok(())
            }
            Err(meilisearch_lib::Error::IndexNotFound(_)) => {
                log::debug!("Index '{}' already doesn't exist", name);
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    // ========================================================================
    // Generic CRUD Operations with Retry
    // ========================================================================

    /// Add or update a single document with retry.
    ///
    /// Uses [`CAMPAIGN_PRIMARY_KEY`] as the primary key for all campaign indexes.
    pub fn upsert_document<T: Serialize>(
        &self,
        index_name: &str,
        document: &T,
    ) -> Result<()> {
        self.with_retry_blocking(|| {
            let value = serde_json::to_value(document)?;
            let task = self.meili.add_documents(
                index_name,
                vec![value],
                Some(CAMPAIGN_PRIMARY_KEY.to_string()),
            )?;
            self.meili
                .wait_for_task(task.uid, Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)))?;
            Ok(())
        })
    }

    /// Add or update multiple documents in batches (REC-MEIL-002)
    ///
    /// Uses [`CAMPAIGN_PRIMARY_KEY`] as the primary key for all campaign indexes.
    pub fn upsert_documents<T: Serialize>(
        &self,
        index_name: &str,
        documents: &[T],
    ) -> Result<()> {
        if documents.is_empty() {
            return Ok(());
        }

        // Process in batches
        for chunk in documents.chunks(MEILISEARCH_BATCH_SIZE) {
            self.with_retry_blocking(|| {
                let values: Vec<serde_json::Value> = chunk
                    .iter()
                    .map(|doc| serde_json::to_value(doc))
                    .collect::<std::result::Result<Vec<_>, _>>()?;

                let task = self.meili.add_documents(
                    index_name,
                    values,
                    Some(CAMPAIGN_PRIMARY_KEY.to_string()),
                )?;
                self.meili
                    .wait_for_task(task.uid, Some(Duration::from_secs(TASK_TIMEOUT_LONG_SECS)))?;
                Ok(())
            })?;
        }

        log::info!(
            "Upserted {} documents to index '{}'",
            documents.len(),
            index_name
        );
        Ok(())
    }

    /// Get a document by ID
    pub fn get_document<T: DeserializeOwned>(
        &self,
        index_name: &str,
        id: &str,
    ) -> Result<Option<T>> {
        match self.meili.get_document(index_name, id) {
            Ok(doc) => {
                let deserialized: T = serde_json::from_value(doc)?;
                Ok(Some(deserialized))
            }
            Err(meilisearch_lib::Error::DocumentNotFound(_)) => Ok(None),
            Err(e) => Err(MeilisearchCampaignError::OperationError(e.to_string())),
        }
    }

    /// Delete a document by ID
    pub fn delete_document(&self, index_name: &str, id: &str) -> Result<()> {
        self.with_retry_blocking(|| {
            let task = self.meili.delete_document(index_name, id)?;
            self.meili
                .wait_for_task(task.uid, Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)))?;
            Ok(())
        })
    }

    /// Delete multiple documents by IDs
    pub fn delete_documents(&self, index_name: &str, ids: &[&str]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        self.with_retry_blocking(|| {
            let id_strings: Vec<String> = ids.iter().map(|s| s.to_string()).collect();
            let task = self
                .meili
                .delete_documents_batch(index_name, id_strings)?;
            self.meili
                .wait_for_task(task.uid, Some(Duration::from_secs(TASK_TIMEOUT_LONG_SECS)))?;
            Ok(())
        })
    }

    /// Delete documents matching a filter
    pub fn delete_by_filter(&self, index_name: &str, filter: &str) -> Result<usize> {
        let mut total_deleted = 0;

        // Loop until no more matching documents
        loop {
            // Search for matching documents
            let query = SearchQuery::empty()
                .with_filter(Value::String(filter.to_string()))
                .with_pagination(0, MEILISEARCH_BATCH_SIZE);

            let results = self.meili.search(index_name, query)?;

            if results.hits.is_empty() {
                break;
            }

            // Extract IDs by primary key
            let ids: Vec<String> = results
                .hits
                .iter()
                .filter_map(|hit| hit.document.get(CAMPAIGN_PRIMARY_KEY).and_then(|v| v.as_str()))
                .map(|s| s.to_string())
                .collect();

            let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
            let count = id_refs.len();

            self.delete_documents(index_name, &id_refs)?;
            total_deleted += count;

            // If we got fewer than batch size, we're done
            if count < MEILISEARCH_BATCH_SIZE {
                break;
            }
        }

        if total_deleted > 0 {
            log::info!(
                "Deleted {} documents from '{}' matching filter",
                total_deleted,
                index_name
            );
        }
        Ok(total_deleted)
    }

    // ========================================================================
    // Search Operations
    // ========================================================================

    /// Search documents with filter and sorting
    pub fn search<T: DeserializeOwned>(
        &self,
        index_name: &str,
        query: &str,
        filter: Option<&str>,
        sort: Option<&[&str]>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<T>> {
        let mut search_query = if query.is_empty() {
            SearchQuery::empty()
        } else {
            SearchQuery::new(query)
        };

        search_query = search_query.with_pagination(offset, limit);

        if let Some(f) = filter {
            search_query = search_query.with_filter(Value::String(f.to_string()));
        }

        if let Some(s) = sort {
            let sort_vec: Vec<String> = s.iter().map(|v| v.to_string()).collect();
            search_query = search_query.with_sort(sort_vec);
        }

        let results = self.meili.search(index_name, search_query)?;

        results
            .hits
            .into_iter()
            .map(|hit| serde_json::from_value(hit.document).map_err(Into::into))
            .collect()
    }

    /// List all documents matching a filter (no text search)
    pub fn list<T: DeserializeOwned>(
        &self,
        index_name: &str,
        filter: Option<&str>,
        sort: Option<&[&str]>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<T>> {
        self.search(index_name, "", filter, sort, limit, offset)
    }

    /// Count documents matching a filter.
    ///
    /// Uses `with_pagination(0, 0)` (offset=0, limit=0) to request zero
    /// document bodies, relying on `estimated_total_hits` for the count.
    pub fn count(&self, index_name: &str, filter: Option<&str>) -> Result<usize> {
        let mut search_query = SearchQuery::empty().with_pagination(0, 0);

        if let Some(f) = filter {
            search_query = search_query.with_filter(Value::String(f.to_string()));
        }

        let results = self.meili.search(index_name, search_query)?;

        // Clamp to usize::MAX to avoid truncation on 32-bit targets
        let estimated = results.estimated_total_hits.unwrap_or(0);
        Ok(usize::try_from(estimated).unwrap_or(usize::MAX))
    }

    // ========================================================================
    // Campaign Arcs Operations (Typed)
    // ========================================================================

    /// Get a campaign arc by ID
    pub fn get_arc<T: DeserializeOwned>(&self, id: &str) -> Result<Option<T>> {
        self.get_document(INDEX_CAMPAIGN_ARCS, id)
    }

    /// List arcs for a campaign
    pub fn list_arcs<T: DeserializeOwned>(
        &self,
        campaign_id: &str,
    ) -> Result<Vec<T>> {
        let filter = format!("campaign_id = \"{}\"", escape_filter_value(campaign_id));
        self.list(
            INDEX_CAMPAIGN_ARCS,
            Some(&filter),
            Some(&["created_at:desc"]),
            1000,
            0,
        )
    }

    /// Save a campaign arc
    pub fn save_arc<T: Serialize>(&self, arc: &T) -> Result<()> {
        self.upsert_document(INDEX_CAMPAIGN_ARCS, arc)
    }

    /// Delete a campaign arc
    pub fn delete_arc(&self, id: &str) -> Result<()> {
        self.delete_document(INDEX_CAMPAIGN_ARCS, id)
    }

    // ========================================================================
    // Session Plans Operations (Typed)
    // ========================================================================

    /// Get a session plan by ID
    pub fn get_plan<T: DeserializeOwned>(&self, id: &str) -> Result<Option<T>> {
        self.get_document(INDEX_SESSION_PLANS, id)
    }

    /// Get session plan for a specific session
    pub fn get_plan_for_session<T: DeserializeOwned>(
        &self,
        session_id: &str,
    ) -> Result<Option<T>> {
        let filter = format!("session_id = \"{}\"", escape_filter_value(session_id));
        let results: Vec<T> = self.list(INDEX_SESSION_PLANS, Some(&filter), None, 1, 0)?;
        Ok(results.into_iter().next())
    }

    /// List plans for a campaign
    pub fn list_plans<T: DeserializeOwned>(
        &self,
        campaign_id: &str,
        include_templates: bool,
    ) -> Result<Vec<T>> {
        let escaped_id = escape_filter_value(campaign_id);
        let filter = if include_templates {
            format!("campaign_id = \"{}\"", escaped_id)
        } else {
            format!("campaign_id = \"{}\" AND is_template = false", escaped_id)
        };
        self.list(
            INDEX_SESSION_PLANS,
            Some(&filter),
            Some(&["session_number:desc"]),
            1000,
            0,
        )
    }

    /// List plan templates for a campaign
    pub fn list_plan_templates<T: DeserializeOwned>(
        &self,
        campaign_id: &str,
    ) -> Result<Vec<T>> {
        let filter = format!(
            "campaign_id = \"{}\" AND is_template = true",
            escape_filter_value(campaign_id)
        );
        self.list(
            INDEX_SESSION_PLANS,
            Some(&filter),
            Some(&["title:asc"]),
            1000,
            0,
        )
    }

    /// Save a session plan
    pub fn save_plan<T: Serialize>(&self, plan: &T) -> Result<()> {
        self.upsert_document(INDEX_SESSION_PLANS, plan)
    }

    /// Delete a session plan
    pub fn delete_plan(&self, id: &str) -> Result<()> {
        self.delete_document(INDEX_SESSION_PLANS, id)
    }

    // ========================================================================
    // Plot Points Operations (Typed)
    // ========================================================================

    /// Get a plot point by ID
    pub fn get_plot_point<T: DeserializeOwned>(&self, id: &str) -> Result<Option<T>> {
        self.get_document(INDEX_PLOT_POINTS, id)
    }

    /// List plot points for a campaign
    pub fn list_plot_points<T: DeserializeOwned>(
        &self,
        campaign_id: &str,
    ) -> Result<Vec<T>> {
        let filter = format!("campaign_id = \"{}\"", escape_filter_value(campaign_id));
        self.list(
            INDEX_PLOT_POINTS,
            Some(&filter),
            Some(&["created_at:desc"]),
            1000,
            0,
        )
    }

    /// List plot points by activation state
    pub fn list_plot_points_by_state<T: DeserializeOwned>(
        &self,
        campaign_id: &str,
        activation_state: &str,
    ) -> Result<Vec<T>> {
        let filter = format!(
            "campaign_id = \"{}\" AND activation_state = \"{}\"",
            escape_filter_value(campaign_id),
            escape_filter_value(activation_state)
        );
        self.list(
            INDEX_PLOT_POINTS,
            Some(&filter),
            Some(&["tension_level:desc", "urgency:desc"]),
            1000,
            0,
        )
    }

    /// List plot points by arc
    pub fn list_plot_points_by_arc<T: DeserializeOwned>(
        &self,
        arc_id: &str,
    ) -> Result<Vec<T>> {
        let filter = format!("arc_id = \"{}\"", escape_filter_value(arc_id));
        self.list(
            INDEX_PLOT_POINTS,
            Some(&filter),
            Some(&["created_at:asc"]),
            1000,
            0,
        )
    }

    /// Save a plot point
    pub fn save_plot_point<T: Serialize>(&self, plot_point: &T) -> Result<()> {
        self.upsert_document(INDEX_PLOT_POINTS, plot_point)
    }

    /// Delete a plot point
    pub fn delete_plot_point(&self, id: &str) -> Result<()> {
        self.delete_document(INDEX_PLOT_POINTS, id)
    }

    // ========================================================================
    // Retry Logic (REC-MEIL-001)
    // ========================================================================

    /// Execute an operation with exponential backoff retry.
    ///
    /// **Blocking**: uses `std::thread::sleep` for backoff delays. In async
    /// contexts (e.g. Tauri command handlers), call via
    /// `tokio::task::spawn_blocking` to avoid stalling the runtime thread pool.
    fn with_retry_blocking<F, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Result<T>,
    {
        let mut attempt = 0;
        let mut last_error = None;

        while attempt < MAX_RETRY_ATTEMPTS {
            match operation() {
                Ok(result) => return Ok(result),
                Err(e) => {
                    // Only retry on transient errors
                    if Self::is_transient_error(&e) {
                        attempt += 1;
                        if attempt < MAX_RETRY_ATTEMPTS {
                            let delay = RETRY_BASE_DELAY_MS * (2_u64.pow(attempt));
                            log::warn!(
                                "Meilisearch operation failed (attempt {}/{}), retrying in {}ms: {}",
                                attempt,
                                MAX_RETRY_ATTEMPTS,
                                delay,
                                e
                            );
                            std::thread::sleep(Duration::from_millis(delay));
                        }
                        last_error = Some(e);
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(last_error.unwrap_or(MeilisearchCampaignError::OperationError(
            "Unknown error after retries".to_string(),
        )))
    }

    /// Check if an error is transient and should be retried
    fn is_transient_error(error: &MeilisearchCampaignError) -> bool {
        matches!(
            error,
            MeilisearchCampaignError::ConnectionError(_)
                | MeilisearchCampaignError::TaskTimeout(_)
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(MEILISEARCH_BATCH_SIZE, 1000);
        assert!(TASK_TIMEOUT_SHORT_SECS < TASK_TIMEOUT_LONG_SECS);
    }

    #[test]
    fn test_is_transient_error() {
        assert!(MeilisearchCampaignClient::is_transient_error(
            &MeilisearchCampaignError::ConnectionError("test".to_string())
        ));

        assert!(MeilisearchCampaignClient::is_transient_error(
            &MeilisearchCampaignError::TaskTimeout(30)
        ));

        assert!(!MeilisearchCampaignClient::is_transient_error(
            &MeilisearchCampaignError::DocumentNotFound {
                index: "test".to_string(),
                id: "123".to_string()
            }
        ));

        assert!(!MeilisearchCampaignClient::is_transient_error(
            &MeilisearchCampaignError::SerializationError("test".to_string())
        ));
    }

    #[test]
    fn test_error_display() {
        let err = MeilisearchCampaignError::DocumentNotFound {
            index: "arcs".to_string(),
            id: "abc123".to_string(),
        };
        assert!(err.to_string().contains("arcs"));
        assert!(err.to_string().contains("abc123"));
    }

    #[test]
    fn test_escape_filter_value() {
        assert_eq!(escape_filter_value("simple"), "simple");
        assert_eq!(escape_filter_value("has\"quotes"), "has\\\"quotes");
        assert_eq!(escape_filter_value("has\\backslash"), "has\\\\backslash");
        assert_eq!(
            escape_filter_value("both\"and\\here"),
            "both\\\"and\\\\here"
        );
    }
}
