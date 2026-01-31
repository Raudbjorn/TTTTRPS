//! Search analytics database operations
//!
//! This module provides operations for tracking and analyzing search behavior.

use super::models::{SearchAnalyticsRecord, SearchSelectionRecord};
use super::Database;
use sqlx::Row;
use sqlx::FromRow;

/// Summary of search analytics for a time period
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchAnalyticsSummary {
    pub total_searches: u32,
    pub zero_result_searches: u32,
    pub click_through_rate: f64,
    pub avg_results_per_search: f64,
    pub avg_execution_time_ms: f64,
    pub top_queries: Vec<(String, u32)>,
    pub failed_queries: Vec<String>,
    pub cache_stats: SearchCacheStats,
    pub by_search_type: std::collections::HashMap<String, u32>,
    pub period_start: String,
    pub period_end: String,
}

/// Cache statistics
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SearchCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub avg_time_saved_ms: f64,
    pub total_time_saved_ms: u64,
    pub top_cached_queries: Vec<(String, u32)>,
}

/// Popular query record from database
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, FromRow)]
pub struct PopularQueryRecord {
    pub query: String,
    pub count: i64,
    pub clicks: i64,
    pub avg_result_count: f64,
    pub last_searched: String,
}

/// Extension trait for search analytics database operations
pub trait SearchAnalyticsOps {
    fn record_search(&self, record: &SearchAnalyticsRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn update_search_selection(&self, search_id: &str, selected_result_id: Option<&str>, selected_result_index: Option<i32>) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn record_search_selection(&self, selection: &SearchSelectionRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_search_analytics(&self, hours: i64) -> impl std::future::Future<Output = Result<Vec<SearchAnalyticsRecord>, sqlx::Error>> + Send;
    fn get_search_analytics_summary(&self, hours: i64) -> impl std::future::Future<Output = Result<SearchAnalyticsSummary, sqlx::Error>> + Send;
    fn get_popular_queries(&self, limit: usize) -> impl std::future::Future<Output = Result<Vec<PopularQueryRecord>, sqlx::Error>> + Send;
    fn get_cache_stats(&self) -> impl std::future::Future<Output = Result<SearchCacheStats, sqlx::Error>> + Send;
    fn get_trending_queries(&self, limit: usize) -> impl std::future::Future<Output = Result<Vec<String>, sqlx::Error>> + Send;
    fn get_zero_result_queries(&self, hours: i64) -> impl std::future::Future<Output = Result<Vec<String>, sqlx::Error>> + Send;
    fn get_click_distribution(&self) -> impl std::future::Future<Output = Result<std::collections::HashMap<i32, u32>, sqlx::Error>> + Send;
    fn cleanup_search_analytics(&self, days: i64) -> impl std::future::Future<Output = Result<u64, sqlx::Error>> + Send;
}

impl SearchAnalyticsOps for Database {
    async fn record_search(&self, record: &SearchAnalyticsRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO search_analytics
            (id, query, results_count, selected_result_id, selected_result_index,
             response_time_ms, cache_hit, search_type, source_filter, campaign_id, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&record.id)
        .bind(&record.query)
        .bind(record.results_count)
        .bind(&record.selected_result_id)
        .bind(record.selected_result_index)
        .bind(record.response_time_ms)
        .bind(record.cache_hit)
        .bind(&record.search_type)
        .bind(&record.source_filter)
        .bind(&record.campaign_id)
        .bind(&record.created_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn update_search_selection(
        &self,
        search_id: &str,
        selected_result_id: Option<&str>,
        selected_result_index: Option<i32>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE search_analytics
            SET selected_result_id = ?, selected_result_index = ?
            WHERE id = ?
            "#
        )
        .bind(selected_result_id)
        .bind(selected_result_index)
        .bind(search_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn record_search_selection(&self, selection: &SearchSelectionRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO search_selections
            (id, search_id, query, result_index, source, was_helpful, selection_delay_ms, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&selection.id)
        .bind(&selection.search_id)
        .bind(&selection.query)
        .bind(selection.result_index)
        .bind(&selection.source)
        .bind(selection.was_helpful)
        .bind(selection.selection_delay_ms)
        .bind(&selection.created_at)
        .execute(self.pool())
        .await?;

        // Update the search record
        self.update_search_selection(&selection.search_id, None, Some(selection.result_index)).await?;

        Ok(())
    }

    async fn get_search_analytics(&self, hours: i64) -> Result<Vec<SearchAnalyticsRecord>, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours);
        sqlx::query_as::<_, SearchAnalyticsRecord>(
            r#"
            SELECT * FROM search_analytics
            WHERE created_at > ?
            ORDER BY created_at DESC
            "#
        )
        .bind(cutoff.to_rfc3339())
        .fetch_all(self.pool())
        .await
    }

    async fn get_search_analytics_summary(&self, hours: i64) -> Result<SearchAnalyticsSummary, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours);
        let cutoff_str = cutoff.to_rfc3339();

        // Get basic stats
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total_searches,
                COALESCE(SUM(CASE WHEN results_count = 0 THEN 1 ELSE 0 END), 0) as zero_result_searches,
                COALESCE(SUM(CASE WHEN selected_result_index IS NOT NULL THEN 1 ELSE 0 END), 0) as clicked_searches,
                COALESCE(AVG(results_count), 0) as avg_results_per_search,
                COALESCE(AVG(response_time_ms), 0) as avg_execution_time_ms,
                COALESCE(SUM(CASE WHEN cache_hit = 1 THEN 1 ELSE 0 END), 0) as cache_hits,
                COALESCE(SUM(CASE WHEN cache_hit = 0 THEN 1 ELSE 0 END), 0) as cache_misses
            FROM search_analytics
            WHERE created_at > ?
            "#
        )
        .bind(&cutoff_str)
        .fetch_one(self.pool())
        .await?;

        let total_searches: i64 = row.get("total_searches");
        let clicked_searches: i64 = row.get("clicked_searches");
        let zero_result_searches: i64 = row.get("zero_result_searches");
        let avg_results: f64 = row.get("avg_results_per_search");
        let avg_time: f64 = row.get("avg_execution_time_ms");
        let cache_hits: i64 = row.get("cache_hits");
        let cache_misses: i64 = row.get("cache_misses");

        let click_through_rate = if total_searches > 0 {
            clicked_searches as f64 / total_searches as f64
        } else {
            0.0
        };

        // Get top queries
        let top_queries: Vec<(String, u32)> = sqlx::query(
            r#"
            SELECT LOWER(TRIM(query)) as query, COUNT(*) as count
            FROM search_analytics
            WHERE created_at > ?
            GROUP BY LOWER(TRIM(query))
            ORDER BY count DESC
            LIMIT 10
            "#
        )
        .bind(&cutoff_str)
        .fetch_all(self.pool())
        .await?
        .into_iter()
        .map(|r| (r.get::<String, _>("query"), r.get::<i64, _>("count") as u32))
        .collect();

        // Get failed queries
        let failed_queries: Vec<String> = sqlx::query(
            r#"
            SELECT DISTINCT query
            FROM search_analytics
            WHERE created_at > ? AND results_count = 0
            LIMIT 20
            "#
        )
        .bind(&cutoff_str)
        .fetch_all(self.pool())
        .await?
        .into_iter()
        .map(|r| r.get("query"))
        .collect();

        // Get search type breakdown
        let type_rows = sqlx::query(
            r#"
            SELECT search_type, COUNT(*) as count
            FROM search_analytics
            WHERE created_at > ?
            GROUP BY search_type
            "#
        )
        .bind(&cutoff_str)
        .fetch_all(self.pool())
        .await?;

        let mut by_search_type = std::collections::HashMap::new();
        for row in type_rows {
            let t: String = row.get("search_type");
            let c: i64 = row.get("count");
            by_search_type.insert(t, c as u32);
        }

        // Calculate cache stats
        let total_cache_ops = cache_hits + cache_misses;
        let hit_rate = if total_cache_ops > 0 {
            cache_hits as f64 / total_cache_ops as f64
        } else {
            0.0
        };

        // Estimate time saved (avg time * hit rate factor)
        let avg_time_saved = avg_time * 0.8; // Assume 80% time saved on cache hits

        Ok(SearchAnalyticsSummary {
            total_searches: total_searches as u32,
            zero_result_searches: zero_result_searches as u32,
            click_through_rate,
            avg_results_per_search: avg_results,
            avg_execution_time_ms: avg_time,
            top_queries,
            failed_queries,
            cache_stats: SearchCacheStats {
                hits: cache_hits as u64,
                misses: cache_misses as u64,
                hit_rate,
                avg_time_saved_ms: avg_time_saved,
                total_time_saved_ms: (cache_hits as f64 * avg_time_saved) as u64,
                top_cached_queries: Vec::new(), // Populated separately if needed
            },
            by_search_type,
            period_start: cutoff_str,
            period_end: chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn get_popular_queries(&self, limit: usize) -> Result<Vec<PopularQueryRecord>, sqlx::Error> {
        sqlx::query_as::<_, PopularQueryRecord>(
            r#"
            SELECT
                LOWER(TRIM(query)) as query,
                COUNT(*) as count,
                COALESCE(SUM(CASE WHEN selected_result_index IS NOT NULL THEN 1 ELSE 0 END), 0) as clicks,
                COALESCE(AVG(results_count), 0) as avg_result_count,
                MAX(created_at) as last_searched
            FROM search_analytics
            GROUP BY LOWER(TRIM(query))
            ORDER BY count DESC
            LIMIT ?
            "#
        )
        .bind(limit as i64)
        .fetch_all(self.pool())
        .await
    }

    async fn get_cache_stats(&self) -> Result<SearchCacheStats, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN cache_hit = 1 THEN 1 ELSE 0 END), 0) as hits,
                COALESCE(SUM(CASE WHEN cache_hit = 0 THEN 1 ELSE 0 END), 0) as misses,
                COALESCE(AVG(CASE WHEN cache_hit = 0 THEN response_time_ms END), 0) as avg_uncached_time
            FROM search_analytics
            "#
        )
        .fetch_one(self.pool())
        .await?;

        let hits: i64 = row.get("hits");
        let misses: i64 = row.get("misses");
        let avg_uncached_time: f64 = row.get("avg_uncached_time");
        let total = hits + misses;

        let hit_rate = if total > 0 { hits as f64 / total as f64 } else { 0.0 };
        let avg_time_saved = avg_uncached_time * 0.8;
        let total_time_saved = (hits as f64 * avg_time_saved) as u64;

        // Get top cached queries
        let top_cached: Vec<(String, u32)> = sqlx::query(
            r#"
            SELECT LOWER(TRIM(query)) as query_norm, COUNT(*) as count
            FROM search_analytics
            WHERE cache_hit = 1
            GROUP BY query_norm
            ORDER BY count DESC
            LIMIT 10
            "#
        )
        .fetch_all(self.pool())
        .await?
        .into_iter()
        .map(|r| (r.get::<String, _>("query_norm"), r.get::<i64, _>("count") as u32))
        .collect();

        Ok(SearchCacheStats {
            hits: hits as u64,
            misses: misses as u64,
            hit_rate,
            avg_time_saved_ms: avg_time_saved,
            total_time_saved_ms: total_time_saved,
            top_cached_queries: top_cached,
        })
    }

    async fn get_trending_queries(&self, limit: usize) -> Result<Vec<String>, sqlx::Error> {
        let now = chrono::Utc::now();
        let recent_cutoff = (now - chrono::Duration::hours(24)).to_rfc3339();
        let older_cutoff = (now - chrono::Duration::hours(168)).to_rfc3339();

        // This query finds queries with higher recent activity vs historical
        let queries: Vec<String> = sqlx::query(
            r#"
            WITH recent AS (
                SELECT LOWER(TRIM(query)) as q, COUNT(*) as cnt
                FROM search_analytics
                WHERE created_at > ?
                GROUP BY q
            ),
            older AS (
                SELECT LOWER(TRIM(query)) as q, COUNT(*) as cnt
                FROM search_analytics
                WHERE created_at > ? AND created_at <= ?
                GROUP BY q
            )
            SELECT recent.q as query,
                   CAST(recent.cnt AS REAL) / COALESCE(NULLIF(older.cnt, 0), 1) * 7.0 as trend_score
            FROM recent
            LEFT JOIN older ON recent.q = older.q
            ORDER BY trend_score DESC
            LIMIT ?
            "#
        )
        .bind(&recent_cutoff)
        .bind(&older_cutoff)
        .bind(&recent_cutoff)
        .bind(limit as i64)
        .fetch_all(self.pool())
        .await?
        .into_iter()
        .map(|r| r.get("query"))
        .collect();

        Ok(queries)
    }

    async fn get_zero_result_queries(&self, hours: i64) -> Result<Vec<String>, sqlx::Error> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::hours(hours)).to_rfc3339();

        let queries: Vec<String> = sqlx::query(
            r#"
            SELECT DISTINCT query
            FROM search_analytics
            WHERE created_at > ? AND results_count = 0
            ORDER BY created_at DESC
            "#
        )
        .bind(&cutoff)
        .fetch_all(self.pool())
        .await?
        .into_iter()
        .map(|r| r.get("query"))
        .collect();

        Ok(queries)
    }

    async fn get_click_distribution(&self) -> Result<std::collections::HashMap<i32, u32>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT selected_result_index, COUNT(*) as count
            FROM search_analytics
            WHERE selected_result_index IS NOT NULL
            GROUP BY selected_result_index
            ORDER BY selected_result_index
            "#
        )
        .fetch_all(self.pool())
        .await?;

        let mut distribution = std::collections::HashMap::new();
        for row in rows {
            let idx: i32 = row.get("selected_result_index");
            let count: i64 = row.get("count");
            distribution.insert(idx, count as u32);
        }

        Ok(distribution)
    }

    async fn cleanup_search_analytics(&self, days: i64) -> Result<u64, sqlx::Error> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339();

        let analytics_result = sqlx::query("DELETE FROM search_analytics WHERE created_at < ?")
            .bind(&cutoff)
            .execute(self.pool())
            .await?;

        let selections_result = sqlx::query("DELETE FROM search_selections WHERE created_at < ?")
            .bind(&cutoff)
            .execute(self.pool())
            .await?;

        Ok(analytics_result.rows_affected() + selections_result.rows_affected())
    }
}
