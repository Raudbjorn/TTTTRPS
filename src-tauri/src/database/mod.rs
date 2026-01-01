//! SQLite Database Module
//!
//! Provides structured data storage for campaigns, sessions, characters,
//! usage tracking, and application state.

mod migrations;
mod models;
mod backup;

pub use migrations::run_migrations;
pub use models::*;
pub use backup::{create_backup, restore_backup, list_backups, BackupInfo};

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions, SqliteConnectOptions};
use sqlx::Row;
use std::path::PathBuf;
use std::str::FromStr;

/// Database connection pool
#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
    path: PathBuf,
}

impl Database {
    /// Create a new database connection
    pub async fn new(data_dir: &std::path::Path) -> Result<Self, sqlx::Error> {
        let db_path = data_dir.join("ttrpg_assistant.db");

        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}?mode=rwc", db_path.display()))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .busy_timeout(std::time::Duration::from_secs(30));

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .min_connections(1)
            .connect_with(options)
            .await?;

        let db = Self { pool, path: db_path };

        // Run migrations
        migrations::run_migrations(&db.pool).await?;

        Ok(db)
    }

    /// Get the underlying pool for direct queries
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Get database file path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    // =========================================================================
    // Campaign Operations
    // =========================================================================

    pub async fn create_campaign(&self, campaign: &CampaignRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO campaigns (id, name, system, description, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&campaign.id)
        .bind(&campaign.name)
        .bind(&campaign.system)
        .bind(&campaign.description)
        .bind(&campaign.created_at)
        .bind(&campaign.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_campaign(&self, id: &str) -> Result<Option<CampaignRecord>, sqlx::Error> {
        sqlx::query_as::<_, CampaignRecord>(
            "SELECT * FROM campaigns WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_campaigns(&self) -> Result<Vec<CampaignRecord>, sqlx::Error> {
        sqlx::query_as::<_, CampaignRecord>(
            "SELECT * FROM campaigns ORDER BY updated_at DESC"
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn update_campaign(&self, campaign: &CampaignRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE campaigns
            SET name = ?, system = ?, description = ?, updated_at = ?
            WHERE id = ?
            "#
        )
        .bind(&campaign.name)
        .bind(&campaign.system)
        .bind(&campaign.description)
        .bind(&campaign.updated_at)
        .bind(&campaign.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_campaign(&self, id: &str) -> Result<(), sqlx::Error> {
        // Delete related data first
        sqlx::query("DELETE FROM sessions WHERE campaign_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM campaign_snapshots WHERE campaign_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM campaigns WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // =========================================================================
    // Session Operations
    // =========================================================================

    pub async fn create_session(&self, session: &SessionRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO sessions (id, campaign_id, session_number, status, started_at, ended_at, notes)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&session.id)
        .bind(&session.campaign_id)
        .bind(session.session_number)
        .bind(&session.status)
        .bind(&session.started_at)
        .bind(&session.ended_at)
        .bind(&session.notes)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_session(&self, id: &str) -> Result<Option<SessionRecord>, sqlx::Error> {
        sqlx::query_as::<_, SessionRecord>(
            "SELECT * FROM sessions WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_sessions(&self, campaign_id: &str) -> Result<Vec<SessionRecord>, sqlx::Error> {
        sqlx::query_as::<_, SessionRecord>(
            "SELECT * FROM sessions WHERE campaign_id = ? ORDER BY session_number DESC"
        )
        .bind(campaign_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_active_session(&self, campaign_id: &str) -> Result<Option<SessionRecord>, sqlx::Error> {
        sqlx::query_as::<_, SessionRecord>(
            "SELECT * FROM sessions WHERE campaign_id = ? AND status = 'active' LIMIT 1"
        )
        .bind(campaign_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn update_session(&self, session: &SessionRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE sessions
            SET status = ?, ended_at = ?, notes = ?
            WHERE id = ?
            "#
        )
        .bind(&session.status)
        .bind(&session.ended_at)
        .bind(&session.notes)
        .bind(&session.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // =========================================================================
    // Character Operations
    // =========================================================================

    pub async fn save_character(&self, character: &CharacterRecord) -> Result<(), sqlx::Error> {
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
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_character(&self, id: &str) -> Result<Option<CharacterRecord>, sqlx::Error> {
        sqlx::query_as::<_, CharacterRecord>(
            "SELECT * FROM characters WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_characters(&self, campaign_id: Option<&str>) -> Result<Vec<CharacterRecord>, sqlx::Error> {
        if let Some(cid) = campaign_id {
            sqlx::query_as::<_, CharacterRecord>(
                "SELECT * FROM characters WHERE campaign_id = ? ORDER BY name"
            )
            .bind(cid)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, CharacterRecord>(
                "SELECT * FROM characters ORDER BY name"
            )
            .fetch_all(&self.pool)
            .await
        }
    }

    pub async fn delete_character(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM characters WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // =========================================================================
    // Usage Tracking Operations
    // =========================================================================

    pub async fn record_usage(&self, usage: &UsageRecord) -> Result<(), sqlx::Error> {
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
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_total_usage(&self) -> Result<UsageStats, sqlx::Error> {
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
        .fetch_one(&self.pool)
        .await?;

        Ok(UsageStats {
            total_input_tokens: row.get::<i64, _>("total_input") as u64,
            total_output_tokens: row.get::<i64, _>("total_output") as u64,
            total_requests: row.get::<i64, _>("total_requests") as u32,
            estimated_cost_usd: row.get("total_cost"),
        })
    }

    pub async fn get_usage_by_provider(&self) -> Result<Vec<ProviderUsageStats>, sqlx::Error> {
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
        .fetch_all(&self.pool)
        .await
    }

    pub async fn reset_usage_stats(&self) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM usage_logs")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // =========================================================================
    // Document/Source Operations
    // =========================================================================

    pub async fn save_document(&self, doc: &DocumentRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO documents
            (id, name, source_type, file_path, page_count, chunk_count, status, ingested_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&doc.id)
        .bind(&doc.name)
        .bind(&doc.source_type)
        .bind(&doc.file_path)
        .bind(doc.page_count)
        .bind(doc.chunk_count)
        .bind(&doc.status)
        .bind(&doc.ingested_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_documents(&self) -> Result<Vec<DocumentRecord>, sqlx::Error> {
        sqlx::query_as::<_, DocumentRecord>(
            "SELECT * FROM documents ORDER BY ingested_at DESC"
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn delete_document(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM documents WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // =========================================================================
    // Settings Operations
    // =========================================================================

    pub async fn get_setting(&self, key: &str) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.get("value")))
    }

    pub async fn set_setting(&self, key: &str, value: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?, ?, datetime('now'))"
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_setting(&self, key: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM settings WHERE key = ?")
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // =========================================================================
    // NPC Conversation Operations
    // =========================================================================

    pub async fn get_npc_conversation(&self, npc_id: &str) -> Result<Option<NpcConversation>, sqlx::Error> {
        sqlx::query_as::<_, NpcConversation>(
            "SELECT * FROM npc_conversations WHERE npc_id = ?"
        )
        .bind(npc_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_npc_conversations(&self, campaign_id: &str) -> Result<Vec<NpcConversation>, sqlx::Error> {
        sqlx::query_as::<_, NpcConversation>(
            "SELECT * FROM npc_conversations WHERE campaign_id = ? ORDER BY last_message_at DESC"
        )
        .bind(campaign_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn save_npc_conversation(&self, conversation: &NpcConversation) -> Result<(), sqlx::Error> {
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
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    // =========================================================================
    // Personality Operations
    // =========================================================================

    pub async fn save_personality(&self, record: &PersonalityRecord) -> Result<(), sqlx::Error> {
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
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_personality(&self, id: &str) -> Result<Option<PersonalityRecord>, sqlx::Error> {
        sqlx::query_as::<_, PersonalityRecord>(
            "SELECT * FROM personalities WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_personalities(&self) -> Result<Vec<PersonalityRecord>, sqlx::Error> {
        sqlx::query_as::<_, PersonalityRecord>(
            "SELECT * FROM personalities ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn delete_personality(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM personalities WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
