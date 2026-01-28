//! Location database operations
//!
//! This module provides CRUD operations for campaign locations with hierarchical support.

use super::models::LocationRecord;
use super::Database;

/// Extension trait for location-related database operations
pub trait LocationOps {
    fn save_location(&self, location: &LocationRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_location(&self, id: &str) -> impl std::future::Future<Output = Result<Option<LocationRecord>, sqlx::Error>> + Send;
    fn list_locations(&self, campaign_id: &str) -> impl std::future::Future<Output = Result<Vec<LocationRecord>, sqlx::Error>> + Send;
    fn list_child_locations(&self, parent_id: &str) -> impl std::future::Future<Output = Result<Vec<LocationRecord>, sqlx::Error>> + Send;
    fn list_root_locations(&self, campaign_id: &str) -> impl std::future::Future<Output = Result<Vec<LocationRecord>, sqlx::Error>> + Send;
    fn delete_location(&self, id: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
}

impl LocationOps for Database {
    async fn save_location(&self, location: &LocationRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO locations
            (id, campaign_id, name, location_type, description, parent_id,
             connections_json, npcs_present_json, features_json, secrets_json,
             attributes_json, tags_json, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&location.id)
        .bind(&location.campaign_id)
        .bind(&location.name)
        .bind(&location.location_type)
        .bind(&location.description)
        .bind(&location.parent_id)
        .bind(&location.connections_json)
        .bind(&location.npcs_present_json)
        .bind(&location.features_json)
        .bind(&location.secrets_json)
        .bind(&location.attributes_json)
        .bind(&location.tags_json)
        .bind(&location.created_at)
        .bind(&location.updated_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_location(&self, id: &str) -> Result<Option<LocationRecord>, sqlx::Error> {
        sqlx::query_as::<_, LocationRecord>(
            "SELECT * FROM locations WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
    }

    async fn list_locations(&self, campaign_id: &str) -> Result<Vec<LocationRecord>, sqlx::Error> {
        sqlx::query_as::<_, LocationRecord>(
            "SELECT * FROM locations WHERE campaign_id = ? ORDER BY name"
        )
        .bind(campaign_id)
        .fetch_all(self.pool())
        .await
    }

    async fn list_child_locations(&self, parent_id: &str) -> Result<Vec<LocationRecord>, sqlx::Error> {
        sqlx::query_as::<_, LocationRecord>(
            "SELECT * FROM locations WHERE parent_id = ? ORDER BY name"
        )
        .bind(parent_id)
        .fetch_all(self.pool())
        .await
    }

    async fn list_root_locations(&self, campaign_id: &str) -> Result<Vec<LocationRecord>, sqlx::Error> {
        sqlx::query_as::<_, LocationRecord>(
            "SELECT * FROM locations WHERE campaign_id = ? AND parent_id IS NULL ORDER BY name"
        )
        .bind(campaign_id)
        .fetch_all(self.pool())
        .await
    }

    async fn delete_location(&self, id: &str) -> Result<(), sqlx::Error> {
        // Update children to have no parent before deleting
        sqlx::query("UPDATE locations SET parent_id = NULL WHERE parent_id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        sqlx::query("DELETE FROM locations WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}
