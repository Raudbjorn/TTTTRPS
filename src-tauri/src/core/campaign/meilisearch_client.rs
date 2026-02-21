//! Meilisearch Campaign Client
//!
//! Typed client wrapper for Meilisearch operations specific to campaign generation.
//! Provides clean API over raw Meilisearch SDK with retry logic and batch operations.
//!
//! TASK-CAMP-004

use meilisearch_sdk::client::Client;
use meilisearch_sdk::search::SearchResults;
use serde::{de::DeserializeOwned, Serialize};
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

impl From<meilisearch_sdk::errors::Error> for MeilisearchCampaignError {
    fn from(e: meilisearch_sdk::errors::Error) -> Self {
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

/// Client for campaign generation Meilisearch operations
pub struct MeilisearchCampaignClient {
    client: Client,
    #[allow(dead_code)]
    host: String,
    #[allow(dead_code)]
    api_key: Option<String>,
}

impl MeilisearchCampaignClient {
    /// Create a new campaign client
    pub fn new(host: &str, api_key: Option<&str>) -> Result<Self> {
        let client = Client::new(host, api_key)
            .map_err(|e| MeilisearchCampaignError::ConnectionError(e.to_string()))?;

        Ok(Self {
            client,
            host: host.to_string(),
            api_key: api_key.map(|s| s.to_string()),
        })
    }

    /// Get the host URL
    pub fn host(&self) -> &str {
        &self.host
    }

    // ========================================================================
    // Health Check (REC-MEIL-003)
    // ========================================================================

    /// Check if Meilisearch is healthy
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/health", self.host);
        let client = reqwest::Client::new();
        match client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Wait for Meilisearch to become healthy with timeout
    pub async fn wait_for_health(&self, timeout_secs: u64) -> bool {
        let start = std::time::Instant::now();
        let duration = Duration::from_secs(timeout_secs);

        while start.elapsed() < duration {
            if self.health_check().await {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        false
    }

    // ========================================================================
    // Index Management (TASK-CAMP-005)
    // ========================================================================

    /// Ensure all campaign generation indexes exist with correct settings
    pub async fn ensure_indexes(&self) -> Result<()> {
        for config in get_index_configs() {
            self.ensure_index(config.name, config.primary_key, config.settings)
                .await?;
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
    async fn ensure_index(
        &self,
        name: &str,
        primary_key: &str,
        settings: meilisearch_sdk::settings::Settings,
    ) -> Result<()> {
        // Try to get existing index first
        match self.client.get_index(name).await {
            Ok(index) => {
                // Index exists, update settings
                let task = index.set_settings(&settings).await?;
                task.wait_for_completion(
                    &self.client,
                    Some(Duration::from_millis(100)),
                    Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)),
                )
                .await?;
                log::debug!("Updated settings for index '{}'", name);
            }
            Err(_) => {
                // Create new index
                let task = self.client.create_index(name, Some(primary_key)).await?;
                task.wait_for_completion(
                    &self.client,
                    Some(Duration::from_millis(100)),
                    Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)),
                )
                .await?;

                // Apply settings
                let index = self.client.index(name);
                let task = index.set_settings(&settings).await?;
                task.wait_for_completion(
                    &self.client,
                    Some(Duration::from_millis(100)),
                    Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)),
                )
                .await?;

                log::info!("Created index '{}' with settings", name);
            }
        }

        Ok(())
    }

    /// Delete an index
    pub async fn delete_index(&self, name: &str) -> Result<()> {
        match self.client.delete_index(name).await {
            Ok(task) => {
                task.wait_for_completion(
                    &self.client,
                    Some(Duration::from_millis(100)),
                    Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)),
                )
                .await?;
                log::info!("Deleted index '{}'", name);
                Ok(())
            }
            Err(meilisearch_sdk::errors::Error::Meilisearch(err))
                if err.error_code == meilisearch_sdk::errors::ErrorCode::IndexNotFound =>
            {
                // Index doesn't exist - that's fine for deletion
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
    /// All campaign indexes use `"id"` as primary key (see [`IndexConfig::primary_key`]).
    pub async fn upsert_document<T: Serialize + Send + Sync>(
        &self,
        index_name: &str,
        document: &T,
    ) -> Result<()> {
        self.with_retry(|| async {
            let index = self.client.index(index_name);
            let task = index.add_documents(&[document], Some("id")).await?;
            task.wait_for_completion(
                &self.client,
                Some(Duration::from_millis(100)),
                Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)),
            )
            .await?;
            Ok(())
        })
        .await
    }

    /// Add or update multiple documents in batches (REC-MEIL-002)
    pub async fn upsert_documents<T: Serialize + Clone + Send + Sync>(
        &self,
        index_name: &str,
        documents: &[T],
    ) -> Result<()> {
        if documents.is_empty() {
            return Ok(());
        }

        let index = self.client.index(index_name);

        // Process in batches
        for chunk in documents.chunks(MEILISEARCH_BATCH_SIZE) {
            self.with_retry(|| async {
                let task = index.add_documents(chunk, Some("id")).await?;
                task.wait_for_completion(
                    &self.client,
                    Some(Duration::from_millis(100)),
                    Some(Duration::from_secs(TASK_TIMEOUT_LONG_SECS)),
                )
                .await?;
                Ok(())
            })
            .await?;
        }

        log::info!(
            "Upserted {} documents to index '{}'",
            documents.len(),
            index_name
        );
        Ok(())
    }

    /// Get a document by ID
    pub async fn get_document<T: DeserializeOwned + Send + Sync + 'static>(
        &self,
        index_name: &str,
        id: &str,
    ) -> Result<Option<T>> {
        let index = self.client.index(index_name);

        match index.get_document::<T>(id).await {
            Ok(doc) => Ok(Some(doc)),
            Err(meilisearch_sdk::errors::Error::Meilisearch(err))
                if err.error_code == meilisearch_sdk::errors::ErrorCode::DocumentNotFound =>
            {
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Delete a document by ID
    pub async fn delete_document(&self, index_name: &str, id: &str) -> Result<()> {
        self.with_retry(|| async {
            let index = self.client.index(index_name);
            let task = index.delete_document(id).await?;
            task.wait_for_completion(
                &self.client,
                Some(Duration::from_millis(100)),
                Some(Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)),
            )
            .await?;
            Ok(())
        })
        .await
    }

    /// Delete multiple documents by IDs
    pub async fn delete_documents(&self, index_name: &str, ids: &[&str]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        self.with_retry(|| async {
            let index = self.client.index(index_name);
            let task = index.delete_documents(ids).await?;
            task.wait_for_completion(
                &self.client,
                Some(Duration::from_millis(100)),
                Some(Duration::from_secs(TASK_TIMEOUT_LONG_SECS)),
            )
            .await?;
            Ok(())
        })
        .await
    }

    /// Delete documents matching a filter
    pub async fn delete_by_filter(&self, index_name: &str, filter: &str) -> Result<usize> {
        let index = self.client.index(index_name);
        let mut total_deleted = 0;

        // Loop until no more matching documents
        loop {
            // Search for matching documents
            let results: SearchResults<serde_json::Value> = index
                .search()
                .with_filter(filter)
                .with_limit(MEILISEARCH_BATCH_SIZE)
                .execute()
                .await?;

            if results.hits.is_empty() {
                break;
            }

            // Extract IDs and delete
            let ids: Vec<String> = results
                .hits
                .iter()
                .filter_map(|hit| hit.result.get("id").and_then(|v| v.as_str()))
                .map(|s| s.to_string())
                .collect();

            let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
            let count = id_refs.len();

            self.delete_documents(index_name, &id_refs).await?;
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
    pub async fn search<T: DeserializeOwned + Send + Sync + 'static>(
        &self,
        index_name: &str,
        query: &str,
        filter: Option<&str>,
        sort: Option<&[&str]>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<T>> {
        let index = self.client.index(index_name);

        let mut search = index.search();
        search.with_query(query).with_limit(limit).with_offset(offset);

        if let Some(f) = filter {
            search.with_filter(f);
        }

        if let Some(s) = sort {
            search.with_sort(s);
        }

        let results: SearchResults<T> = search.execute().await?;

        Ok(results.hits.into_iter().map(|h| h.result).collect())
    }

    /// List all documents matching a filter (no text search)
    pub async fn list<T: DeserializeOwned + Send + Sync + 'static>(
        &self,
        index_name: &str,
        filter: Option<&str>,
        sort: Option<&[&str]>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<T>> {
        self.search(index_name, "", filter, sort, limit, offset)
            .await
    }

    /// Count documents matching a filter.
    ///
    /// Uses `limit(0)` to return zero hits while still populating
    /// `estimated_total_hits`, which gives the matching document count
    /// without transferring document bodies.
    pub async fn count(&self, index_name: &str, filter: Option<&str>) -> Result<usize> {
        let index = self.client.index(index_name);

        let mut search = index.search();
        search.with_query("").with_limit(0);

        if let Some(f) = filter {
            search.with_filter(f);
        }

        let results: SearchResults<serde_json::Value> = search.execute().await?;

        Ok(results.estimated_total_hits.unwrap_or(0))
    }

    // ========================================================================
    // Campaign Arcs Operations (Typed)
    // ========================================================================

    /// Get a campaign arc by ID
    pub async fn get_arc<T: DeserializeOwned + Send + Sync + 'static>(&self, id: &str) -> Result<Option<T>> {
        self.get_document(INDEX_CAMPAIGN_ARCS, id).await
    }

    /// List arcs for a campaign
    pub async fn list_arcs<T: DeserializeOwned + Send + Sync + 'static>(
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
        .await
    }

    /// Save a campaign arc
    pub async fn save_arc<T: Serialize + Send + Sync>(&self, arc: &T) -> Result<()> {
        self.upsert_document(INDEX_CAMPAIGN_ARCS, arc).await
    }

    /// Delete a campaign arc
    pub async fn delete_arc(&self, id: &str) -> Result<()> {
        self.delete_document(INDEX_CAMPAIGN_ARCS, id).await
    }

    // ========================================================================
    // Session Plans Operations (Typed)
    // ========================================================================

    /// Get a session plan by ID
    pub async fn get_plan<T: DeserializeOwned + Send + Sync + 'static>(&self, id: &str) -> Result<Option<T>> {
        self.get_document(INDEX_SESSION_PLANS, id).await
    }

    /// Get session plan for a specific session
    pub async fn get_plan_for_session<T: DeserializeOwned + Send + Sync + 'static>(
        &self,
        session_id: &str,
    ) -> Result<Option<T>> {
        let filter = format!("session_id = \"{}\"", escape_filter_value(session_id));
        let results: Vec<T> = self
            .list(INDEX_SESSION_PLANS, Some(&filter), None, 1, 0)
            .await?;
        Ok(results.into_iter().next())
    }

    /// List plans for a campaign
    pub async fn list_plans<T: DeserializeOwned + Send + Sync + 'static>(
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
        .await
    }

    /// List plan templates for a campaign
    pub async fn list_plan_templates<T: DeserializeOwned + Send + Sync + 'static>(
        &self,
        campaign_id: &str,
    ) -> Result<Vec<T>> {
        let filter = format!("campaign_id = \"{}\" AND is_template = true", escape_filter_value(campaign_id));
        self.list(
            INDEX_SESSION_PLANS,
            Some(&filter),
            Some(&["title:asc"]),
            1000,
            0,
        )
        .await
    }

    /// Save a session plan
    pub async fn save_plan<T: Serialize + Send + Sync>(&self, plan: &T) -> Result<()> {
        self.upsert_document(INDEX_SESSION_PLANS, plan).await
    }

    /// Delete a session plan
    pub async fn delete_plan(&self, id: &str) -> Result<()> {
        self.delete_document(INDEX_SESSION_PLANS, id).await
    }

    // ========================================================================
    // Plot Points Operations (Typed)
    // ========================================================================

    /// Get a plot point by ID
    pub async fn get_plot_point<T: DeserializeOwned + Send + Sync + 'static>(&self, id: &str) -> Result<Option<T>> {
        self.get_document(INDEX_PLOT_POINTS, id).await
    }

    /// List plot points for a campaign
    pub async fn list_plot_points<T: DeserializeOwned + Send + Sync + 'static>(
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
        .await
    }

    /// List plot points by activation state
    pub async fn list_plot_points_by_state<T: DeserializeOwned + Send + Sync + 'static>(
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
        .await
    }

    /// List plot points by arc
    pub async fn list_plot_points_by_arc<T: DeserializeOwned + Send + Sync + 'static>(
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
        .await
    }

    /// Save a plot point
    pub async fn save_plot_point<T: Serialize + Send + Sync>(&self, plot_point: &T) -> Result<()> {
        self.upsert_document(INDEX_PLOT_POINTS, plot_point).await
    }

    /// Delete a plot point
    pub async fn delete_plot_point(&self, id: &str) -> Result<()> {
        self.delete_document(INDEX_PLOT_POINTS, id).await
    }

    // ========================================================================
    // Retry Logic (REC-MEIL-001)
    // ========================================================================

    /// Execute an operation with exponential backoff retry
    async fn with_retry<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut last_error = None;

        while attempt < MAX_RETRY_ATTEMPTS {
            match operation().await {
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
                            tokio::time::sleep(Duration::from_millis(delay)).await;
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
}
