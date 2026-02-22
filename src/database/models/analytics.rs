//! Analytics Models
//!
//! Database records for search analytics, selection tracking, and usage statistics.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================================================
// Search Analytics Record
// ============================================================================

/// Search analytics database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SearchAnalyticsRecord {
    pub id: String,
    pub query: String,
    pub results_count: i32,
    pub selected_result_id: Option<String>,
    pub selected_result_index: Option<i32>,
    pub response_time_ms: i32,
    pub cache_hit: bool,
    pub search_type: String,  // "semantic", "keyword", "hybrid"
    pub source_filter: Option<String>,
    pub campaign_id: Option<String>,
    pub created_at: String,
}

impl SearchAnalyticsRecord {
    pub fn new(
        query: String,
        results_count: i32,
        response_time_ms: i32,
        search_type: String,
        cache_hit: bool,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            query,
            results_count,
            selected_result_id: None,
            selected_result_index: None,
            response_time_ms,
            cache_hit,
            search_type,
            source_filter: None,
            campaign_id: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Link to a campaign
    pub fn with_campaign(mut self, campaign_id: String) -> Self {
        self.campaign_id = Some(campaign_id);
        self
    }

    /// Set source filter
    pub fn with_source_filter(mut self, filter: String) -> Self {
        self.source_filter = Some(filter);
        self
    }

    /// Record a selection
    pub fn record_selection(&mut self, result_id: String, index: i32) {
        self.selected_result_id = Some(result_id);
        self.selected_result_index = Some(index);
    }

    /// Check if query returned no results
    pub fn is_zero_result(&self) -> bool {
        self.results_count == 0
    }

    /// Check if a result was selected
    pub fn has_selection(&self) -> bool {
        self.selected_result_index.is_some()
    }
}

// ============================================================================
// Search Selection Record
// ============================================================================

/// Search selection record for tracking user clicks on search results
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SearchSelectionRecord {
    pub id: String,
    pub search_id: String,
    pub query: String,
    pub result_index: i32,
    pub source: String,
    pub was_helpful: Option<bool>,
    pub selection_delay_ms: i64,
    pub created_at: String,
}

impl SearchSelectionRecord {
    pub fn new(
        search_id: String,
        query: String,
        result_index: i32,
        source: String,
        selection_delay_ms: i64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            search_id,
            query,
            result_index,
            source,
            was_helpful: None,
            selection_delay_ms,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Record if the result was helpful
    pub fn with_helpfulness(mut self, helpful: bool) -> Self {
        self.was_helpful = Some(helpful);
        self
    }
}

// ============================================================================
// Search Query Stats Record
// ============================================================================

/// Aggregated query statistics record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SearchQueryStatsRecord {
    pub query_normalized: String,
    pub total_count: i32,
    pub total_clicks: i32,
    pub avg_results: f64,
    pub avg_time_ms: f64,
    pub last_searched_at: String,
}

impl SearchQueryStatsRecord {
    /// Calculate click-through rate
    pub fn click_through_rate(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            self.total_clicks as f64 / self.total_count as f64
        }
    }
}

