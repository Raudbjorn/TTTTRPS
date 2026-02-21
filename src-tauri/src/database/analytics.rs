//! Usage analytics database operations
//!
//! This module provides operations for tracking LLM usage (tokens, costs).

use super::models::{UsageRecord, UsageStats, ProviderUsageStats};
use super::Database;
use sqlx::Row;

/// Extension trait for usage tracking database operations
pub trait UsageOps {
    fn record_usage(&self, usage: &UsageRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_total_usage(&self) -> impl std::future::Future<Output = Result<UsageStats, sqlx::Error>> + Send;
    fn get_usage_by_provider(&self) -> impl std::future::Future<Output = Result<Vec<ProviderUsageStats>, sqlx::Error>> + Send;
    fn reset_usage_stats(&self) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
}

impl UsageOps for Database {
    async fn record_usage(&self, usage: &UsageRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO usage_logs
            (id, provider, model, input_tokens, output_tokens, estimated_cost_usd, timestamp)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&usage.id)
        .bind(&usage.provider)
        .bind(&usage.model)
        .bind(usage.input_tokens as i64)
        .bind(usage.output_tokens as i64)
        .bind(usage.estimated_cost_usd)
        .bind(&usage.timestamp)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_total_usage(&self) -> Result<UsageStats, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                COALESCE(SUM(input_tokens), 0) as total_input,
                COALESCE(SUM(output_tokens), 0) as total_output,
                COUNT(*) as total_requests,
                COALESCE(SUM(estimated_cost_usd), 0.0) as total_cost
            FROM usage_logs
            "#
        )
        .fetch_one(self.pool())
        .await?;

        Ok(UsageStats {
            total_input_tokens: row.get::<i64, _>("total_input") as u64,
            total_output_tokens: row.get::<i64, _>("total_output") as u64,
            total_requests: row.get::<i64, _>("total_requests") as u32,
            estimated_cost_usd: row.get("total_cost"),
        })
    }

    async fn get_usage_by_provider(&self) -> Result<Vec<ProviderUsageStats>, sqlx::Error> {
        sqlx::query_as::<_, ProviderUsageStats>(
            r#"
            SELECT
                provider,
                COALESCE(SUM(input_tokens), 0) as input_tokens,
                COALESCE(SUM(output_tokens), 0) as output_tokens,
                COUNT(*) as requests,
                COALESCE(SUM(estimated_cost_usd), 0.0) as estimated_cost_usd
            FROM usage_logs
            GROUP BY provider
            "#
        )
        .fetch_all(self.pool())
        .await
    }

    async fn reset_usage_stats(&self) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM usage_logs")
            .execute(self.pool())
            .await?;
        Ok(())
    }
}
