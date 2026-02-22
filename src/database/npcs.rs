//! NPC database operations
//!
//! This module provides CRUD operations for NPCs, NPC conversations,
//! and personality records.

use super::models::{NpcRecord, NpcConversation, PersonalityRecord};
use super::Database;

/// Extension trait for NPC-related database operations
pub trait NpcOps {
    // NPC CRUD
    fn save_npc(&self, npc: &NpcRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_npc(&self, id: &str) -> impl std::future::Future<Output = Result<Option<NpcRecord>, sqlx::Error>> + Send;
    fn list_npcs(&self, campaign_id: Option<&str>) -> impl std::future::Future<Output = Result<Vec<NpcRecord>, sqlx::Error>> + Send;
    fn delete_npc(&self, id: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;

    // NPC Conversations
    fn get_npc_conversation(&self, npc_id: &str) -> impl std::future::Future<Output = Result<Option<NpcConversation>, sqlx::Error>> + Send;
    fn list_npc_conversations(&self, campaign_id: &str) -> impl std::future::Future<Output = Result<Vec<NpcConversation>, sqlx::Error>> + Send;
    fn save_npc_conversation(&self, conversation: &NpcConversation) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;

    // Personalities
    fn save_personality(&self, record: &PersonalityRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_personality(&self, id: &str) -> impl std::future::Future<Output = Result<Option<PersonalityRecord>, sqlx::Error>> + Send;
    fn list_personalities(&self) -> impl std::future::Future<Output = Result<Vec<PersonalityRecord>, sqlx::Error>> + Send;
    fn delete_personality(&self, id: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
}

impl NpcOps for Database {
    // =========================================================================
    // NPC Operations
    // =========================================================================

    async fn save_npc(&self, npc: &NpcRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO npcs
            (id, campaign_id, name, role, personality_id, personality_json, data_json,
             stats_json, notes, location_id, voice_profile_id, quest_hooks, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&npc.id)
        .bind(&npc.campaign_id)
        .bind(&npc.name)
        .bind(&npc.role)
        .bind(&npc.personality_id)
        .bind(&npc.personality_json)
        .bind(&npc.data_json)
        .bind(&npc.stats_json)
        .bind(&npc.notes)
        .bind(&npc.location_id)
        .bind(&npc.voice_profile_id)
        .bind(&npc.quest_hooks)
        .bind(&npc.created_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_npc(&self, id: &str) -> Result<Option<NpcRecord>, sqlx::Error> {
        sqlx::query_as::<_, NpcRecord>(
            "SELECT * FROM npcs WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
    }

    async fn list_npcs(&self, campaign_id: Option<&str>) -> Result<Vec<NpcRecord>, sqlx::Error> {
        if let Some(cid) = campaign_id {
            sqlx::query_as::<_, NpcRecord>(
                "SELECT * FROM npcs WHERE campaign_id = ? ORDER BY name"
            )
            .bind(cid)
            .fetch_all(self.pool())
            .await
        } else {
            sqlx::query_as::<_, NpcRecord>(
                "SELECT * FROM npcs ORDER BY name"
            )
            .fetch_all(self.pool())
            .await
        }
    }

    async fn delete_npc(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM npcs WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    // =========================================================================
    // NPC Conversation Operations
    // =========================================================================

    async fn get_npc_conversation(&self, npc_id: &str) -> Result<Option<NpcConversation>, sqlx::Error> {
        sqlx::query_as::<_, NpcConversation>(
            "SELECT * FROM npc_conversations WHERE npc_id = ?"
        )
        .bind(npc_id)
        .fetch_optional(self.pool())
        .await
    }

    async fn list_npc_conversations(&self, campaign_id: &str) -> Result<Vec<NpcConversation>, sqlx::Error> {
        sqlx::query_as::<_, NpcConversation>(
            "SELECT * FROM npc_conversations WHERE campaign_id = ? ORDER BY last_message_at DESC"
        )
        .bind(campaign_id)
        .fetch_all(self.pool())
        .await
    }

    async fn save_npc_conversation(&self, conversation: &NpcConversation) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO npc_conversations
            (id, npc_id, campaign_id, messages_json, unread_count, last_message_at, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&conversation.id)
        .bind(&conversation.npc_id)
        .bind(&conversation.campaign_id)
        .bind(&conversation.messages_json)
        .bind(conversation.unread_count)
        .bind(&conversation.last_message_at)
        .bind(&conversation.created_at)
        .bind(&conversation.updated_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    // =========================================================================
    // Personality Operations
    // =========================================================================

    async fn save_personality(&self, record: &PersonalityRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO personalities
            (id, name, source, data_json, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&record.id)
        .bind(&record.name)
        .bind(&record.source)
        .bind(&record.data_json)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_personality(&self, id: &str) -> Result<Option<PersonalityRecord>, sqlx::Error> {
        sqlx::query_as::<_, PersonalityRecord>(
            "SELECT * FROM personalities WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
    }

    async fn list_personalities(&self) -> Result<Vec<PersonalityRecord>, sqlx::Error> {
        sqlx::query_as::<_, PersonalityRecord>(
            "SELECT * FROM personalities ORDER BY name"
        )
        .fetch_all(self.pool())
        .await
    }

    async fn delete_personality(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM personalities WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}
