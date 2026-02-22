//! Campaign database operations
//!
//! This module provides CRUD operations for campaigns and campaign versions.

use super::models::{CampaignRecord, CampaignVersionRecord};
use super::Database;
use sqlx::Row;

/// Extension trait for campaign-related database operations
pub trait CampaignOps {
    // Campaign CRUD
    fn create_campaign(&self, campaign: &CampaignRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_campaign(&self, id: &str) -> impl std::future::Future<Output = Result<Option<CampaignRecord>, sqlx::Error>> + Send;
    fn list_campaigns(&self) -> impl std::future::Future<Output = Result<Vec<CampaignRecord>, sqlx::Error>> + Send;
    fn update_campaign(&self, campaign: &CampaignRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn delete_campaign(&self, id: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;

    // Campaign Versions
    fn save_campaign_version(&self, version: &CampaignVersionRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_campaign_version(&self, id: &str) -> impl std::future::Future<Output = Result<Option<CampaignVersionRecord>, sqlx::Error>> + Send;
    fn list_campaign_versions(&self, campaign_id: &str) -> impl std::future::Future<Output = Result<Vec<CampaignVersionRecord>, sqlx::Error>> + Send;
    fn get_latest_version_number(&self, campaign_id: &str) -> impl std::future::Future<Output = Result<i32, sqlx::Error>> + Send;
    fn delete_campaign_version(&self, id: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
}

impl CampaignOps for Database {
    // =========================================================================
    // Campaign Operations
    // =========================================================================

    async fn create_campaign(&self, campaign: &CampaignRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO campaigns (id, name, system, description, setting, current_in_game_date,
                house_rules, world_state, created_at, updated_at, archived_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&campaign.id)
        .bind(&campaign.name)
        .bind(&campaign.system)
        .bind(&campaign.description)
        .bind(&campaign.setting)
        .bind(&campaign.current_in_game_date)
        .bind(&campaign.house_rules)
        .bind(&campaign.world_state)
        .bind(&campaign.created_at)
        .bind(&campaign.updated_at)
        .bind(&campaign.archived_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_campaign(&self, id: &str) -> Result<Option<CampaignRecord>, sqlx::Error> {
        sqlx::query_as::<_, CampaignRecord>(
            "SELECT * FROM campaigns WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
    }

    async fn list_campaigns(&self) -> Result<Vec<CampaignRecord>, sqlx::Error> {
        sqlx::query_as::<_, CampaignRecord>(
            "SELECT * FROM campaigns ORDER BY updated_at DESC"
        )
        .fetch_all(self.pool())
        .await
    }

    async fn update_campaign(&self, campaign: &CampaignRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE campaigns
            SET name = ?, system = ?, description = ?, setting = ?,
                current_in_game_date = ?, house_rules = ?, world_state = ?,
                updated_at = ?, archived_at = ?
            WHERE id = ?
            "#
        )
        .bind(&campaign.name)
        .bind(&campaign.system)
        .bind(&campaign.description)
        .bind(&campaign.setting)
        .bind(&campaign.current_in_game_date)
        .bind(&campaign.house_rules)
        .bind(&campaign.world_state)
        .bind(&campaign.updated_at)
        .bind(&campaign.archived_at)
        .bind(&campaign.id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn delete_campaign(&self, id: &str) -> Result<(), sqlx::Error> {
        // Use transaction to ensure atomic deletion of campaign and all related data
        let mut tx = self.pool().begin().await?;

        // Delete related data first (order matters for foreign key constraints)
        sqlx::query("DELETE FROM sessions WHERE campaign_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM campaign_snapshots WHERE campaign_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM campaign_versions WHERE campaign_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM campaigns WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    // =========================================================================
    // Campaign Version Operations
    // =========================================================================

    async fn save_campaign_version(&self, version: &CampaignVersionRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO campaign_versions
            (id, campaign_id, version_number, snapshot_type, description, data, diff_data, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&version.id)
        .bind(&version.campaign_id)
        .bind(version.version_number)
        .bind(&version.snapshot_type)
        .bind(&version.description)
        .bind(&version.data)
        .bind(&version.diff_data)
        .bind(&version.created_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_campaign_version(&self, id: &str) -> Result<Option<CampaignVersionRecord>, sqlx::Error> {
        sqlx::query_as::<_, CampaignVersionRecord>(
            "SELECT * FROM campaign_versions WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
    }

    async fn list_campaign_versions(&self, campaign_id: &str) -> Result<Vec<CampaignVersionRecord>, sqlx::Error> {
        sqlx::query_as::<_, CampaignVersionRecord>(
            "SELECT * FROM campaign_versions WHERE campaign_id = ? ORDER BY version_number DESC"
        )
        .bind(campaign_id)
        .fetch_all(self.pool())
        .await
    }

    async fn get_latest_version_number(&self, campaign_id: &str) -> Result<i32, sqlx::Error> {
        let result = sqlx::query(
            "SELECT MAX(version_number) as max_version FROM campaign_versions WHERE campaign_id = ?"
        )
        .bind(campaign_id)
        .fetch_optional(self.pool())
        .await?;

        Ok(result
            .and_then(|row| row.try_get::<i32, _>("max_version").ok())
            .unwrap_or(0))
    }

    async fn delete_campaign_version(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM campaign_versions WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}
