//! Entity relationship database operations
//!
//! This module provides CRUD operations for relationships between campaign entities
//! (NPCs, locations, factions, etc.).

use super::models::EntityRelationshipRecord;
use super::Database;

/// Extension trait for entity relationship database operations
pub trait RelationshipOps {
    fn save_entity_relationship(&self, rel: &EntityRelationshipRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_entity_relationship(&self, id: &str) -> impl std::future::Future<Output = Result<Option<EntityRelationshipRecord>, sqlx::Error>> + Send;
    fn list_relationships_for_entity(&self, entity_type: &str, entity_id: &str) -> impl std::future::Future<Output = Result<Vec<EntityRelationshipRecord>, sqlx::Error>> + Send;
    fn list_relationships_by_type(&self, campaign_id: &str, relationship_type: &str) -> impl std::future::Future<Output = Result<Vec<EntityRelationshipRecord>, sqlx::Error>> + Send;
    fn list_campaign_relationships(&self, campaign_id: &str) -> impl std::future::Future<Output = Result<Vec<EntityRelationshipRecord>, sqlx::Error>> + Send;
    fn delete_entity_relationship(&self, id: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn delete_relationships_for_entity(&self, entity_type: &str, entity_id: &str) -> impl std::future::Future<Output = Result<u64, sqlx::Error>> + Send;
}

impl RelationshipOps for Database {
    async fn save_entity_relationship(&self, rel: &EntityRelationshipRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO entity_relationships
            (id, campaign_id, source_entity_type, source_entity_id, target_entity_type,
             target_entity_id, relationship_type, description, strength, bidirectional,
             metadata, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&rel.id)
        .bind(&rel.campaign_id)
        .bind(&rel.source_entity_type)
        .bind(&rel.source_entity_id)
        .bind(&rel.target_entity_type)
        .bind(&rel.target_entity_id)
        .bind(&rel.relationship_type)
        .bind(&rel.description)
        .bind(rel.strength)
        .bind(rel.bidirectional)
        .bind(&rel.metadata)
        .bind(&rel.created_at)
        .bind(&rel.updated_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_entity_relationship(&self, id: &str) -> Result<Option<EntityRelationshipRecord>, sqlx::Error> {
        sqlx::query_as::<_, EntityRelationshipRecord>(
            "SELECT * FROM entity_relationships WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
    }

    async fn list_relationships_for_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Vec<EntityRelationshipRecord>, sqlx::Error> {
        sqlx::query_as::<_, EntityRelationshipRecord>(
            r#"
            SELECT * FROM entity_relationships
            WHERE (source_entity_type = ? AND source_entity_id = ?)
               OR (bidirectional = 1 AND target_entity_type = ? AND target_entity_id = ?)
            ORDER BY relationship_type
            "#
        )
        .bind(entity_type)
        .bind(entity_id)
        .bind(entity_type)
        .bind(entity_id)
        .fetch_all(self.pool())
        .await
    }

    async fn list_relationships_by_type(
        &self,
        campaign_id: &str,
        relationship_type: &str,
    ) -> Result<Vec<EntityRelationshipRecord>, sqlx::Error> {
        sqlx::query_as::<_, EntityRelationshipRecord>(
            "SELECT * FROM entity_relationships WHERE campaign_id = ? AND relationship_type = ?"
        )
        .bind(campaign_id)
        .bind(relationship_type)
        .fetch_all(self.pool())
        .await
    }

    async fn list_campaign_relationships(&self, campaign_id: &str) -> Result<Vec<EntityRelationshipRecord>, sqlx::Error> {
        sqlx::query_as::<_, EntityRelationshipRecord>(
            "SELECT * FROM entity_relationships WHERE campaign_id = ? ORDER BY created_at DESC"
        )
        .bind(campaign_id)
        .fetch_all(self.pool())
        .await
    }

    async fn delete_entity_relationship(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM entity_relationships WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    async fn delete_relationships_for_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM entity_relationships
            WHERE (source_entity_type = ? AND source_entity_id = ?)
               OR (target_entity_type = ? AND target_entity_id = ?)
            "#
        )
        .bind(entity_type)
        .bind(entity_id)
        .bind(entity_type)
        .bind(entity_id)
        .execute(self.pool())
        .await?;
        Ok(result.rows_affected())
    }
}
