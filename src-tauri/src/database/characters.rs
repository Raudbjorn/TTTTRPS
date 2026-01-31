//! Character database operations
//!
//! This module provides CRUD operations for player and non-player characters.

use super::models::CharacterRecord;
use super::Database;

/// Extension trait for character-related database operations
pub trait CharacterOps {
    fn save_character(&self, character: &CharacterRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_character(&self, id: &str) -> impl std::future::Future<Output = Result<Option<CharacterRecord>, sqlx::Error>> + Send;
    fn list_characters(&self, campaign_id: Option<&str>) -> impl std::future::Future<Output = Result<Vec<CharacterRecord>, sqlx::Error>> + Send;
    fn delete_character(&self, id: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
}

impl CharacterOps for Database {
    async fn save_character(&self, character: &CharacterRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO characters
            (id, campaign_id, name, system, character_type, level, data_json, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&character.id)
        .bind(&character.campaign_id)
        .bind(&character.name)
        .bind(&character.system)
        .bind(&character.character_type)
        .bind(character.level)
        .bind(&character.data_json)
        .bind(&character.created_at)
        .bind(&character.updated_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_character(&self, id: &str) -> Result<Option<CharacterRecord>, sqlx::Error> {
        sqlx::query_as::<_, CharacterRecord>(
            "SELECT * FROM characters WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
    }

    async fn list_characters(&self, campaign_id: Option<&str>) -> Result<Vec<CharacterRecord>, sqlx::Error> {
        if let Some(cid) = campaign_id {
            sqlx::query_as::<_, CharacterRecord>(
                "SELECT * FROM characters WHERE campaign_id = ? ORDER BY name"
            )
            .bind(cid)
            .fetch_all(self.pool())
            .await
        } else {
            sqlx::query_as::<_, CharacterRecord>(
                "SELECT * FROM characters ORDER BY name"
            )
            .fetch_all(self.pool())
            .await
        }
    }

    async fn delete_character(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM characters WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}
