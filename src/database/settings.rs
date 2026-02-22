//! Settings database operations
//!
//! This module provides key-value settings storage.

use super::Database;
use sqlx::Row;

/// Extension trait for settings database operations
pub trait SettingsOps {
    fn get_setting(&self, key: &str) -> impl std::future::Future<Output = Result<Option<String>, sqlx::Error>> + Send;
    fn set_setting(&self, key: &str, value: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn delete_setting(&self, key: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
}

impl SettingsOps for Database {
    async fn get_setting(&self, key: &str) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(self.pool())
            .await?;

        Ok(row.map(|r| r.get("value")))
    }

    async fn set_setting(&self, key: &str, value: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?, ?, datetime('now'))"
        )
        .bind(key)
        .bind(value)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn delete_setting(&self, key: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM settings WHERE key = ?")
            .bind(key)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}
