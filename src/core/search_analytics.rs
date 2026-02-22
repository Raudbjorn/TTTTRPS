//! Search Analytics Module
//!
//! Tracks search queries, result selections, and provides insights.
//! Supports both in-memory (fast) and SQLite-persistent storage.
//!
//! The database-backed analytics provides:
//! - Query frequency tracking
//! - Result selection recording
//! - Cache hit/miss statistics
//! - Popular search identification
//! - Search trend analysis
//! - Failed query tracking

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::database::{
    Database, SearchAnalyticsOps, SearchAnalyticsRecord, SearchSelectionRecord,
    SearchAnalyticsSummary as DbSearchAnalyticsSummary,
    SearchCacheStats as DbSearchCacheStats,
    PopularQueryRecord,
};

// ============================================================================
// Types (compatible with frontend bindings)
// ============================================================================

/// A recorded search query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRecord {
    /// Unique record ID
    pub id: String,
    /// Query text
    pub query: String,
    /// Number of results returned
    pub result_count: usize,
    /// Whether user clicked a result
    pub clicked: bool,
    /// Index of clicked result (if clicked)
    pub clicked_index: Option<usize>,
    /// Time to execute (ms)
    pub execution_time_ms: u64,
    /// Search type (semantic, keyword, hybrid)
    pub search_type: String,
    /// Whether result was served from cache
    pub from_cache: bool,
    /// Source filter used (if any)
    pub source_filter: Option<String>,
    /// Campaign context (if any)
    pub campaign_id: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl SearchRecord {
    /// Create a new search record
    pub fn new(
        query: String,
        result_count: usize,
        execution_time_ms: u64,
        search_type: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            query,
            result_count,
            clicked: false,
            clicked_index: None,
            execution_time_ms,
            search_type,
            from_cache: false,
            source_filter: None,
            campaign_id: None,
            timestamp: Utc::now(),
        }
    }

    /// Mark as cached
    pub fn with_cache(mut self, from_cache: bool) -> Self {
        self.from_cache = from_cache;
        self
    }

    /// Add source filter
    pub fn with_source_filter(mut self, filter: Option<String>) -> Self {
        self.source_filter = filter;
        self
    }

    /// Convert to database record
    pub fn to_db_record(&self) -> SearchAnalyticsRecord {
        SearchAnalyticsRecord {
            id: self.id.clone(),
            query: self.query.clone(),
            results_count: self.result_count as i32,
            selected_result_id: None,
            selected_result_index: self.clicked_index.map(|i| i as i32),
            response_time_ms: self.execution_time_ms as i32,
            cache_hit: self.from_cache,
            search_type: self.search_type.clone(),
            source_filter: self.source_filter.clone(),
            campaign_id: self.campaign_id.clone(),
            created_at: self.timestamp.to_rfc3339(),
        }
    }
}

/// Result selection record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultSelection {
    /// Search record ID
    pub search_id: String,
    /// Query that produced the result
    pub query: String,
    /// Index of selected result (0-based)
    pub result_index: usize,
    /// Source document/content
    pub source: String,
    /// Whether the result was helpful (if feedback provided)
    pub was_helpful: Option<bool>,
    /// Time between search and selection (ms)
    pub selection_delay_ms: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl ResultSelection {
    /// Convert to database record
    pub fn to_db_record(&self) -> SearchSelectionRecord {
        SearchSelectionRecord::new(
            self.search_id.clone(),
            self.query.clone(),
            self.result_index as i32,
            self.source.clone(),
            self.selection_delay_ms as i64,
        )
    }
}

/// Query statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryStats {
    /// Total times this query was searched
    pub count: u32,
    /// Total clicks on results
    pub clicks: u32,
    /// Average result count
    pub avg_results: f64,
    /// Average execution time
    pub avg_time_ms: f64,
    /// Last searched
    pub last_searched: Option<DateTime<Utc>>,
    /// Click position distribution (index -> count)
    pub click_positions: HashMap<usize, u32>,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheStats {
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Cache hit rate (0.0 - 1.0)
    pub hit_rate: f64,
    /// Average time saved per hit (ms)
    pub avg_time_saved_ms: f64,
    /// Total time saved (ms)
    pub total_time_saved_ms: u64,
    /// Most cached queries
    pub top_cached_queries: Vec<(String, u32)>,
}

impl From<DbSearchCacheStats> for CacheStats {
    fn from(db: DbSearchCacheStats) -> Self {
        Self {
            hits: db.hits,
            misses: db.misses,
            hit_rate: db.hit_rate,
            avg_time_saved_ms: db.avg_time_saved_ms,
            total_time_saved_ms: db.total_time_saved_ms,
            top_cached_queries: db.top_cached_queries,
        }
    }
}

/// Search analytics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSummary {
    /// Total searches in period
    pub total_searches: u32,
    /// Searches with zero results
    pub zero_result_searches: u32,
    /// Click-through rate
    pub click_through_rate: f64,
    /// Average results per search
    pub avg_results_per_search: f64,
    /// Average execution time
    pub avg_execution_time_ms: f64,
    /// Top queries
    pub top_queries: Vec<(String, u32)>,
    /// Queries with no results
    pub failed_queries: Vec<String>,
    /// Cache statistics
    pub cache_stats: CacheStats,
    /// Search type breakdown
    pub by_search_type: HashMap<String, u32>,
    /// Period start
    pub period_start: DateTime<Utc>,
    /// Period end
    pub period_end: DateTime<Utc>,
}

impl From<DbSearchAnalyticsSummary> for AnalyticsSummary {
    fn from(db: DbSearchAnalyticsSummary) -> Self {
        Self {
            total_searches: db.total_searches,
            zero_result_searches: db.zero_result_searches,
            click_through_rate: db.click_through_rate,
            avg_results_per_search: db.avg_results_per_search,
            avg_execution_time_ms: db.avg_execution_time_ms,
            top_queries: db.top_queries,
            failed_queries: db.failed_queries,
            cache_stats: db.cache_stats.into(),
            by_search_type: db.by_search_type,
            period_start: DateTime::parse_from_rfc3339(&db.period_start)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            period_end: DateTime::parse_from_rfc3339(&db.period_end)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        }
    }
}

/// Popular query entry with details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopularQuery {
    pub query: String,
    pub count: u32,
    pub click_through_rate: f64,
    pub avg_result_count: f64,
    pub last_searched: Option<String>,
}

impl From<PopularQueryRecord> for PopularQuery {
    fn from(record: PopularQueryRecord) -> Self {
        let ctr = if record.count > 0 {
            record.clicks as f64 / record.count as f64
        } else {
            0.0
        };
        Self {
            query: record.query,
            count: record.count as u32,
            click_through_rate: ctr,
            avg_result_count: record.avg_result_count,
            last_searched: Some(record.last_searched),
        }
    }
}

// ============================================================================
// Database-Backed Search Analytics
// ============================================================================

/// Database-backed search analytics tracker
/// Persists all analytics data to SQLite for long-term analysis
pub struct DbSearchAnalytics {
    db: Arc<Database>,
    /// In-memory cache counters for fast access
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
}

impl DbSearchAnalytics {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
        }
    }

    /// Record a search event (async, writes to database)
    pub async fn record(&self, record: SearchRecord) -> Result<(), String> {
        // Update in-memory counters
        if record.from_cache {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }

        // Write to database
        let db_record = record.to_db_record();
        self.db.record_search(&db_record).await
            .map_err(|e| format!("Failed to record search: {}", e))
    }

    /// Record a result selection (async, writes to database)
    pub async fn record_selection(&self, selection: ResultSelection) -> Result<(), String> {
        let db_record = selection.to_db_record();
        self.db.record_search_selection(&db_record).await
            .map_err(|e| format!("Failed to record selection: {}", e))
    }

    /// Get analytics summary for a time period
    pub async fn get_summary(&self, hours: i64) -> Result<AnalyticsSummary, String> {
        self.db.get_search_analytics_summary(hours).await
            .map(|s| s.into())
            .map_err(|e| format!("Failed to get analytics summary: {}", e))
    }

    /// Get popular queries with detailed stats
    pub async fn get_popular_queries_detailed(&self, limit: usize) -> Result<Vec<PopularQuery>, String> {
        self.db.get_popular_queries(limit).await
            .map(|records| records.into_iter().map(|r| r.into()).collect())
            .map_err(|e| format!("Failed to get popular queries: {}", e))
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> Result<CacheStats, String> {
        self.db.get_cache_stats().await
            .map(|s| s.into())
            .map_err(|e| format!("Failed to get cache stats: {}", e))
    }

    /// Get trending queries
    pub async fn get_trending_queries(&self, limit: usize) -> Result<Vec<String>, String> {
        self.db.get_trending_queries(limit).await
            .map_err(|e| format!("Failed to get trending queries: {}", e))
    }

    /// Get queries with zero results
    pub async fn get_zero_result_queries(&self, hours: i64) -> Result<Vec<String>, String> {
        self.db.get_zero_result_queries(hours).await
            .map_err(|e| format!("Failed to get zero result queries: {}", e))
    }

    /// Get click position distribution
    pub async fn get_click_position_distribution(&self) -> Result<HashMap<usize, u32>, String> {
        self.db.get_click_distribution().await
            .map(|m| m.into_iter().map(|(k, v)| (k as usize, v)).collect())
            .map_err(|e| format!("Failed to get click distribution: {}", e))
    }

    /// Clean up old records
    pub async fn cleanup(&self, days: i64) -> Result<u64, String> {
        self.db.cleanup_search_analytics(days).await
            .map_err(|e| format!("Failed to cleanup analytics: {}", e))
    }
}

// ============================================================================
// In-Memory Search Analytics (for backward compatibility)
// ============================================================================

/// In-memory analytics tracker (faster but not persistent)
/// Kept for backward compatibility with existing code
pub struct SearchAnalytics {
    /// Individual search records
    records: RwLock<Vec<SearchRecord>>,
    /// Result selections
    selections: RwLock<Vec<ResultSelection>>,
    /// Aggregated query stats
    query_stats: RwLock<HashMap<String, QueryStats>>,
    /// Cache hit/miss tracking
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    /// Cache time savings tracking
    cache_time_saved_ms: AtomicU64,
    /// Cached query counts
    cached_queries: RwLock<HashMap<String, u32>>,
    /// Average non-cached execution time (for estimating savings)
    avg_uncached_time_ms: RwLock<f64>,
    /// Maximum records to keep
    max_records: usize,
}

impl SearchAnalytics {
    pub fn new() -> Self {
        Self {
            records: RwLock::new(Vec::with_capacity(10000)),
            selections: RwLock::new(Vec::with_capacity(5000)),
            query_stats: RwLock::new(HashMap::new()),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            cache_time_saved_ms: AtomicU64::new(0),
            cached_queries: RwLock::new(HashMap::new()),
            avg_uncached_time_ms: RwLock::new(50.0), // Default estimate
            max_records: 100000,
        }
    }

    /// Record a search
    pub fn record(&self, record: SearchRecord) {
        let query_normalized = record.query.to_lowercase().trim().to_string();

        // Track cache statistics
        if record.from_cache {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);

            // Track cached query
            let mut cached = self.cached_queries.write().unwrap();
            *cached.entry(query_normalized.clone()).or_default() += 1;

            // Estimate time saved (avg uncached time - actual time)
            let avg = *self.avg_uncached_time_ms.read().unwrap();
            let saved = (avg - record.execution_time_ms as f64).max(0.0) as u64;
            self.cache_time_saved_ms.fetch_add(saved, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);

            // Update average uncached time
            let mut avg = self.avg_uncached_time_ms.write().unwrap();
            *avg = (*avg * 0.9) + (record.execution_time_ms as f64 * 0.1);
        }

        // Update aggregated stats
        {
            let mut stats = self.query_stats.write().unwrap();
            let entry = stats.entry(query_normalized).or_default();
            entry.count += 1;
            if record.clicked {
                entry.clicks += 1;
                if let Some(idx) = record.clicked_index {
                    *entry.click_positions.entry(idx).or_default() += 1;
                }
            }
            // Update rolling average
            let n = entry.count as f64;
            entry.avg_results = ((entry.avg_results * (n - 1.0)) + record.result_count as f64) / n;
            entry.avg_time_ms = ((entry.avg_time_ms * (n - 1.0)) + record.execution_time_ms as f64) / n;
            entry.last_searched = Some(record.timestamp);
        }

        // Store record
        {
            let mut records = self.records.write().unwrap();
            records.push(record);

            // Rotate if needed
            if records.len() > self.max_records {
                records.drain(0..10000);
            }
        }
    }

    /// Record a result selection (click)
    pub fn record_selection(&self, selection: ResultSelection) {
        let query_normalized = selection.query.to_lowercase().trim().to_string();

        // Update query stats with click position
        {
            let mut stats = self.query_stats.write().unwrap();
            if let Some(entry) = stats.get_mut(&query_normalized) {
                entry.clicks += 1;
                *entry.click_positions.entry(selection.result_index).or_default() += 1;
            }
        }

        // Update the corresponding search record
        {
            let mut records = self.records.write().unwrap();
            if let Some(record) = records.iter_mut().rev().find(|r| r.id == selection.search_id) {
                record.clicked = true;
                record.clicked_index = Some(selection.result_index);
            }
        }

        // Store selection
        {
            let mut selections = self.selections.write().unwrap();
            selections.push(selection);

            // Rotate if needed
            if selections.len() > self.max_records / 2 {
                selections.drain(0..5000);
            }
        }
    }

    /// Record a click on a search result (simplified version)
    pub fn record_click(&self, query: &str) {
        let query_normalized = query.to_lowercase().trim().to_string();

        let mut stats = self.query_stats.write().unwrap();
        if let Some(entry) = stats.get_mut(&query_normalized) {
            entry.clicks += 1;
        }
    }

    /// Record a click with result index
    pub fn record_click_with_index(&self, query: &str, result_index: usize) {
        let query_normalized = query.to_lowercase().trim().to_string();

        let mut stats = self.query_stats.write().unwrap();
        if let Some(entry) = stats.get_mut(&query_normalized) {
            entry.clicks += 1;
            *entry.click_positions.entry(result_index).or_default() += 1;
        }
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> CacheStats {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        let time_saved = self.cache_time_saved_ms.load(Ordering::Relaxed);

        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };

        let avg_time_saved = if hits > 0 {
            time_saved as f64 / hits as f64
        } else {
            0.0
        };

        // Get top cached queries
        let cached = self.cached_queries.read().unwrap();
        let mut cached_vec: Vec<_> = cached.iter().map(|(k, v)| (k.clone(), *v)).collect();
        cached_vec.sort_by(|a, b| b.1.cmp(&a.1));
        let top_cached: Vec<(String, u32)> = cached_vec.into_iter().take(10).collect();

        CacheStats {
            hits,
            misses,
            hit_rate,
            avg_time_saved_ms: avg_time_saved,
            total_time_saved_ms: time_saved,
            top_cached_queries: top_cached,
        }
    }

    /// Get summary for a time period
    pub fn get_summary(&self, hours: i64) -> AnalyticsSummary {
        let cutoff = Utc::now() - Duration::hours(hours);
        let records = self.records.read().unwrap();

        let relevant: Vec<&SearchRecord> = records
            .iter()
            .filter(|r| r.timestamp > cutoff)
            .collect();

        let total_searches = relevant.len() as u32;
        let zero_result_searches = relevant.iter().filter(|r| r.result_count == 0).count() as u32;
        let clicks = relevant.iter().filter(|r| r.clicked).count();

        let click_through_rate = if total_searches > 0 {
            clicks as f64 / total_searches as f64
        } else {
            0.0
        };

        let avg_results_per_search = if total_searches > 0 {
            relevant.iter().map(|r| r.result_count).sum::<usize>() as f64 / total_searches as f64
        } else {
            0.0
        };

        let avg_execution_time_ms = if total_searches > 0 {
            relevant.iter().map(|r| r.execution_time_ms).sum::<u64>() as f64 / total_searches as f64
        } else {
            0.0
        };

        // Get top queries
        let stats = self.query_stats.read().unwrap();
        let mut query_counts: Vec<(&String, &QueryStats)> = stats.iter().collect();
        query_counts.sort_by(|a, b| b.1.count.cmp(&a.1.count));
        let top_queries: Vec<(String, u32)> = query_counts
            .iter()
            .take(10)
            .map(|(q, s)| ((*q).clone(), s.count))
            .collect();

        // Get failed queries
        let failed_queries: Vec<String> = relevant
            .iter()
            .filter(|r| r.result_count == 0)
            .map(|r| r.query.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(20)
            .collect();

        // Search type breakdown
        let mut by_search_type: HashMap<String, u32> = HashMap::new();
        for record in relevant.iter() {
            *by_search_type.entry(record.search_type.clone()).or_default() += 1;
        }

        AnalyticsSummary {
            total_searches,
            zero_result_searches,
            click_through_rate,
            avg_results_per_search,
            avg_execution_time_ms,
            top_queries,
            failed_queries,
            cache_stats: self.get_cache_stats(),
            by_search_type,
            period_start: cutoff,
            period_end: Utc::now(),
        }
    }

    /// Get detailed popular queries with statistics
    pub fn get_popular_queries_detailed(&self, limit: usize) -> Vec<PopularQuery> {
        let stats = self.query_stats.read().unwrap();
        let mut query_list: Vec<(&String, &QueryStats)> = stats.iter().collect();
        query_list.sort_by(|a, b| b.1.count.cmp(&a.1.count));

        query_list
            .into_iter()
            .take(limit)
            .map(|(query, stat)| PopularQuery {
                query: query.clone(),
                count: stat.count,
                click_through_rate: if stat.count > 0 {
                    stat.clicks as f64 / stat.count as f64
                } else {
                    0.0
                },
                avg_result_count: stat.avg_results,
                last_searched: stat.last_searched.map(|t| t.to_rfc3339()),
            })
            .collect()
    }

    /// Get click position distribution across all queries
    pub fn get_click_position_distribution(&self) -> HashMap<usize, u32> {
        let stats = self.query_stats.read().unwrap();
        let mut distribution: HashMap<usize, u32> = HashMap::new();

        for stat in stats.values() {
            for (pos, count) in stat.click_positions.iter() {
                *distribution.entry(*pos).or_default() += count;
            }
        }

        distribution
    }

    /// Get selections for a specific query
    pub fn get_selections_for_query(&self, query: &str) -> Vec<ResultSelection> {
        let query_normalized = query.to_lowercase().trim().to_string();
        let selections = self.selections.read().unwrap();

        selections
            .iter()
            .filter(|s| s.query.to_lowercase() == query_normalized)
            .cloned()
            .collect()
    }

    /// Get recent selections
    pub fn get_recent_selections(&self, limit: usize) -> Vec<ResultSelection> {
        let selections = self.selections.read().unwrap();
        selections.iter().rev().take(limit).cloned().collect()
    }

    /// Get popular queries
    pub fn get_popular_queries(&self, limit: usize) -> Vec<(String, u32)> {
        let stats = self.query_stats.read().unwrap();
        let mut query_counts: Vec<(&String, &QueryStats)> = stats.iter().collect();
        query_counts.sort_by(|a, b| b.1.count.cmp(&a.1.count));

        query_counts
            .into_iter()
            .take(limit)
            .map(|(q, s)| (q.clone(), s.count))
            .collect()
    }

    /// Get queries with zero results
    pub fn get_zero_result_queries(&self, hours: i64) -> Vec<String> {
        let cutoff = Utc::now() - Duration::hours(hours);
        let records = self.records.read().unwrap();

        records
            .iter()
            .filter(|r| r.timestamp > cutoff && r.result_count == 0)
            .map(|r| r.query.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// Get trending queries (most increase in recent period)
    pub fn get_trending_queries(&self, limit: usize) -> Vec<String> {
        let now = Utc::now();
        let recent_cutoff = now - Duration::hours(24);
        let older_cutoff = now - Duration::hours(168); // Last week

        let records = self.records.read().unwrap();

        // Count queries in recent vs older period
        let mut recent_counts: HashMap<String, u32> = HashMap::new();
        let mut older_counts: HashMap<String, u32> = HashMap::new();

        for record in records.iter() {
            let query = record.query.to_lowercase();
            if record.timestamp > recent_cutoff {
                *recent_counts.entry(query).or_default() += 1;
            } else if record.timestamp > older_cutoff {
                *older_counts.entry(query).or_default() += 1;
            }
        }

        // Calculate trend score (recent count / older count)
        let mut trends: Vec<(String, f64)> = recent_counts
            .into_iter()
            .map(|(q, recent)| {
                let older = *older_counts.get(&q).unwrap_or(&1) as f64;
                let score = (recent as f64) / (older / 7.0).max(1.0); // Normalize older period
                (q, score)
            })
            .collect();

        trends.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        trends.into_iter().take(limit).map(|(q, _)| q).collect()
    }

    /// Get stats for a specific query
    pub fn get_query_stats(&self, query: &str) -> Option<QueryStats> {
        let stats = self.query_stats.read().unwrap();
        let normalized = query.to_lowercase().trim().to_string();
        stats.get(&normalized).cloned()
    }

    /// Clear old records
    pub fn cleanup(&self, days: i64) {
        let cutoff = Utc::now() - Duration::days(days);
        let mut records = self.records.write().unwrap();
        records.retain(|r| r.timestamp > cutoff);
    }
}

impl Default for SearchAnalytics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(query: &str, result_count: usize, clicked: bool) -> SearchRecord {
        SearchRecord {
            id: uuid::Uuid::new_v4().to_string(),
            query: query.to_string(),
            result_count,
            clicked,
            clicked_index: if clicked { Some(0) } else { None },
            execution_time_ms: 50,
            search_type: "hybrid".to_string(),
            from_cache: false,
            source_filter: None,
            campaign_id: None,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_record_search() {
        let analytics = SearchAnalytics::new();
        analytics.record(make_record("fireball damage", 5, true));

        let stats = analytics.get_query_stats("fireball damage").unwrap();
        assert_eq!(stats.count, 1);
        assert_eq!(stats.clicks, 1);
    }

    #[test]
    fn test_popular_queries() {
        let analytics = SearchAnalytics::new();

        for _ in 0..5 {
            analytics.record(make_record("popular query", 3, false));
        }

        analytics.record(make_record("rare query", 1, false));

        let popular = analytics.get_popular_queries(5);
        assert_eq!(popular[0].0, "popular query");
        assert_eq!(popular[0].1, 5);
    }

    #[test]
    fn test_summary() {
        let analytics = SearchAnalytics::new();
        analytics.record(make_record("test", 5, true));

        let summary = analytics.get_summary(24);
        assert_eq!(summary.total_searches, 1);
        assert!(summary.click_through_rate > 0.0);
    }

    #[test]
    fn test_cache_stats() {
        let analytics = SearchAnalytics::new();

        // Record a non-cached search
        analytics.record(make_record("query1", 5, false));

        // Record a cached search
        let mut cached_record = make_record("query2", 5, false);
        cached_record.from_cache = true;
        cached_record.execution_time_ms = 5;
        analytics.record(cached_record);

        let cache_stats = analytics.get_cache_stats();
        assert_eq!(cache_stats.hits, 1);
        assert_eq!(cache_stats.misses, 1);
        assert!((cache_stats.hit_rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_click_position_tracking() {
        let analytics = SearchAnalytics::new();

        // Record searches with different click positions
        let mut r1 = make_record("test", 5, true);
        r1.clicked_index = Some(0);
        analytics.record(r1);

        let mut r2 = make_record("test", 5, true);
        r2.clicked_index = Some(2);
        analytics.record(r2);

        let stats = analytics.get_query_stats("test").unwrap();
        assert_eq!(stats.click_positions.get(&0), Some(&1));
        assert_eq!(stats.click_positions.get(&2), Some(&1));
    }

    #[test]
    fn test_result_selection() {
        let analytics = SearchAnalytics::new();

        // Record a search
        let record = make_record("spell rules", 10, false);
        let search_id = record.id.clone();
        analytics.record(record);

        // Record a selection
        analytics.record_selection(ResultSelection {
            search_id,
            query: "spell rules".to_string(),
            result_index: 2,
            source: "phb_chapter_10".to_string(),
            was_helpful: Some(true),
            selection_delay_ms: 3000,
            timestamp: Utc::now(),
        });

        let selections = analytics.get_selections_for_query("spell rules");
        assert_eq!(selections.len(), 1);
        assert_eq!(selections[0].result_index, 2);
    }

    #[test]
    fn test_popular_queries_detailed() {
        let analytics = SearchAnalytics::new();

        for _ in 0..10 {
            analytics.record(make_record("common query", 5, false));
        }
        for _ in 0..3 {
            analytics.record(make_record("common query", 5, true));
        }

        let detailed = analytics.get_popular_queries_detailed(5);
        assert_eq!(detailed[0].query, "common query");
        assert_eq!(detailed[0].count, 13);
        // CTR = 3/13 = ~0.23
        assert!(detailed[0].click_through_rate > 0.2);
    }
}
