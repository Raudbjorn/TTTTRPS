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
use sqlx::FromRow;
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
    // NPC Operations
    // =========================================================================

    pub async fn save_npc(&self, npc: &NpcRecord) -> Result<(), sqlx::Error> {
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
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_npc(&self, id: &str) -> Result<Option<NpcRecord>, sqlx::Error> {
        sqlx::query_as::<_, NpcRecord>(
            "SELECT * FROM npcs WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_npcs(&self, campaign_id: Option<&str>) -> Result<Vec<NpcRecord>, sqlx::Error> {
        if let Some(cid) = campaign_id {
            sqlx::query_as::<_, NpcRecord>(
                "SELECT * FROM npcs WHERE campaign_id = ? ORDER BY name"
            )
            .bind(cid)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, NpcRecord>(
                "SELECT * FROM npcs ORDER BY name"
            )
            .fetch_all(&self.pool)
            .await
        }
    }

    pub async fn delete_npc(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM npcs WHERE id = ?")
            .bind(id)
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

    // =========================================================================
    // Campaign Version Operations
    // =========================================================================

    pub async fn save_campaign_version(&self, version: &CampaignVersionRecord) -> Result<(), sqlx::Error> {
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
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_campaign_version(&self, id: &str) -> Result<Option<CampaignVersionRecord>, sqlx::Error> {
        sqlx::query_as::<_, CampaignVersionRecord>(
            "SELECT * FROM campaign_versions WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_campaign_versions(&self, campaign_id: &str) -> Result<Vec<CampaignVersionRecord>, sqlx::Error> {
        sqlx::query_as::<_, CampaignVersionRecord>(
            "SELECT * FROM campaign_versions WHERE campaign_id = ? ORDER BY version_number DESC"
        )
        .bind(campaign_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_latest_version_number(&self, campaign_id: &str) -> Result<i32, sqlx::Error> {
        let result = sqlx::query(
            "SELECT MAX(version_number) as max_version FROM campaign_versions WHERE campaign_id = ?"
        )
        .bind(campaign_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result
            .and_then(|row| row.try_get::<i32, _>("max_version").ok())
            .unwrap_or(0))
    }

    pub async fn delete_campaign_version(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM campaign_versions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // =========================================================================
    // Entity Relationship Operations
    // =========================================================================

    pub async fn save_entity_relationship(&self, rel: &EntityRelationshipRecord) -> Result<(), sqlx::Error> {
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
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_entity_relationship(&self, id: &str) -> Result<Option<EntityRelationshipRecord>, sqlx::Error> {
        sqlx::query_as::<_, EntityRelationshipRecord>(
            "SELECT * FROM entity_relationships WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_relationships_for_entity(
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
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_relationships_by_type(
        &self,
        campaign_id: &str,
        relationship_type: &str,
    ) -> Result<Vec<EntityRelationshipRecord>, sqlx::Error> {
        sqlx::query_as::<_, EntityRelationshipRecord>(
            "SELECT * FROM entity_relationships WHERE campaign_id = ? AND relationship_type = ?"
        )
        .bind(campaign_id)
        .bind(relationship_type)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_campaign_relationships(&self, campaign_id: &str) -> Result<Vec<EntityRelationshipRecord>, sqlx::Error> {
        sqlx::query_as::<_, EntityRelationshipRecord>(
            "SELECT * FROM entity_relationships WHERE campaign_id = ? ORDER BY created_at DESC"
        )
        .bind(campaign_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn delete_entity_relationship(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM entity_relationships WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_relationships_for_entity(
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
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    // =========================================================================
    // Voice Profile Operations
    // =========================================================================

    pub async fn save_voice_profile(&self, profile: &VoiceProfileRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO voice_profiles
            (id, name, provider, voice_id, settings, age_range, gender,
             personality_traits, is_preset, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&profile.id)
        .bind(&profile.name)
        .bind(&profile.provider)
        .bind(&profile.voice_id)
        .bind(&profile.settings)
        .bind(&profile.age_range)
        .bind(&profile.gender)
        .bind(&profile.personality_traits)
        .bind(profile.is_preset)
        .bind(&profile.created_at)
        .bind(&profile.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_voice_profile(&self, id: &str) -> Result<Option<VoiceProfileRecord>, sqlx::Error> {
        sqlx::query_as::<_, VoiceProfileRecord>(
            "SELECT * FROM voice_profiles WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_voice_profiles(&self) -> Result<Vec<VoiceProfileRecord>, sqlx::Error> {
        sqlx::query_as::<_, VoiceProfileRecord>(
            "SELECT * FROM voice_profiles ORDER BY is_preset DESC, name ASC"
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_voice_profile_presets(&self) -> Result<Vec<VoiceProfileRecord>, sqlx::Error> {
        sqlx::query_as::<_, VoiceProfileRecord>(
            "SELECT * FROM voice_profiles WHERE is_preset = 1 ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn delete_voice_profile(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM voice_profiles WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // =========================================================================
    // Session Notes Operations
    // =========================================================================

    pub async fn save_session_note(&self, note: &SessionNoteRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO session_notes
            (id, session_id, campaign_id, content, tags, entity_links, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&note.id)
        .bind(&note.session_id)
        .bind(&note.campaign_id)
        .bind(&note.content)
        .bind(&note.tags)
        .bind(&note.entity_links)
        .bind(&note.created_at)
        .bind(&note.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_session_note(&self, id: &str) -> Result<Option<SessionNoteRecord>, sqlx::Error> {
        sqlx::query_as::<_, SessionNoteRecord>(
            "SELECT * FROM session_notes WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_session_notes(&self, session_id: &str) -> Result<Vec<SessionNoteRecord>, sqlx::Error> {
        sqlx::query_as::<_, SessionNoteRecord>(
            "SELECT * FROM session_notes WHERE session_id = ? ORDER BY created_at DESC"
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_campaign_notes(&self, campaign_id: &str) -> Result<Vec<SessionNoteRecord>, sqlx::Error> {
        sqlx::query_as::<_, SessionNoteRecord>(
            "SELECT * FROM session_notes WHERE campaign_id = ? ORDER BY created_at DESC"
        )
        .bind(campaign_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn delete_session_note(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM session_notes WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // =========================================================================
    // Session Events (Timeline) Operations
    // =========================================================================

    pub async fn save_session_event(&self, event: &SessionEventRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO session_events
            (id, session_id, timestamp, event_type, description, entities, metadata, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&event.id)
        .bind(&event.session_id)
        .bind(&event.timestamp)
        .bind(&event.event_type)
        .bind(&event.description)
        .bind(&event.entities)
        .bind(&event.metadata)
        .bind(&event.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_session_event(&self, id: &str) -> Result<Option<SessionEventRecord>, sqlx::Error> {
        sqlx::query_as::<_, SessionEventRecord>(
            "SELECT * FROM session_events WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_session_events(&self, session_id: &str) -> Result<Vec<SessionEventRecord>, sqlx::Error> {
        sqlx::query_as::<_, SessionEventRecord>(
            "SELECT * FROM session_events WHERE session_id = ? ORDER BY timestamp ASC"
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_session_events_by_type(
        &self,
        session_id: &str,
        event_type: &str,
    ) -> Result<Vec<SessionEventRecord>, sqlx::Error> {
        sqlx::query_as::<_, SessionEventRecord>(
            "SELECT * FROM session_events WHERE session_id = ? AND event_type = ? ORDER BY timestamp ASC"
        )
        .bind(session_id)
        .bind(event_type)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn delete_session_event(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM session_events WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // =========================================================================
    // Combat State Operations
    // =========================================================================

    pub async fn save_combat_state(&self, combat: &CombatStateRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO combat_states
            (id, session_id, name, round, current_turn, is_active, combatants,
             conditions, environment, notes, created_at, updated_at, ended_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&combat.id)
        .bind(&combat.session_id)
        .bind(&combat.name)
        .bind(combat.round)
        .bind(combat.current_turn)
        .bind(combat.is_active)
        .bind(&combat.combatants)
        .bind(&combat.conditions)
        .bind(&combat.environment)
        .bind(&combat.notes)
        .bind(&combat.created_at)
        .bind(&combat.updated_at)
        .bind(&combat.ended_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_combat_state(&self, id: &str) -> Result<Option<CombatStateRecord>, sqlx::Error> {
        sqlx::query_as::<_, CombatStateRecord>(
            "SELECT * FROM combat_states WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_active_combat(&self, session_id: &str) -> Result<Option<CombatStateRecord>, sqlx::Error> {
        sqlx::query_as::<_, CombatStateRecord>(
            "SELECT * FROM combat_states WHERE session_id = ? AND is_active = 1 ORDER BY created_at DESC LIMIT 1"
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_session_combats(&self, session_id: &str) -> Result<Vec<CombatStateRecord>, sqlx::Error> {
        sqlx::query_as::<_, CombatStateRecord>(
            "SELECT * FROM combat_states WHERE session_id = ? ORDER BY created_at DESC"
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn end_combat(&self, id: &str) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE combat_states SET is_active = 0, ended_at = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_combat_state(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM combat_states WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // =========================================================================
    // Location Operations
    // =========================================================================

    pub async fn save_location(&self, location: &LocationRecord) -> Result<(), sqlx::Error> {
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
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_location(&self, id: &str) -> Result<Option<LocationRecord>, sqlx::Error> {
        sqlx::query_as::<_, LocationRecord>(
            "SELECT * FROM locations WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_locations(&self, campaign_id: &str) -> Result<Vec<LocationRecord>, sqlx::Error> {
        sqlx::query_as::<_, LocationRecord>(
            "SELECT * FROM locations WHERE campaign_id = ? ORDER BY name"
        )
        .bind(campaign_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_child_locations(&self, parent_id: &str) -> Result<Vec<LocationRecord>, sqlx::Error> {
        sqlx::query_as::<_, LocationRecord>(
            "SELECT * FROM locations WHERE parent_id = ? ORDER BY name"
        )
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_root_locations(&self, campaign_id: &str) -> Result<Vec<LocationRecord>, sqlx::Error> {
        sqlx::query_as::<_, LocationRecord>(
            "SELECT * FROM locations WHERE campaign_id = ? AND parent_id IS NULL ORDER BY name"
        )
        .bind(campaign_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn delete_location(&self, id: &str) -> Result<(), sqlx::Error> {
        // Update children to have no parent before deleting
        sqlx::query("UPDATE locations SET parent_id = NULL WHERE parent_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM locations WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // =========================================================================
    // Search Analytics Operations (TASK-023)
    // =========================================================================

    /// Record a search event
    pub async fn record_search(&self, record: &SearchAnalyticsRecord) -> Result<(), sqlx::Error> {
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
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update search record with selection info
    pub async fn update_search_selection(
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
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Record a search result selection (click)
    pub async fn record_search_selection(&self, selection: &SearchSelectionRecord) -> Result<(), sqlx::Error> {
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
        .execute(&self.pool)
        .await?;

        // Update the search record
        self.update_search_selection(&selection.search_id, None, Some(selection.result_index)).await?;

        Ok(())
    }

    /// Get search analytics for a time period (in hours)
    pub async fn get_search_analytics(&self, hours: i64) -> Result<Vec<SearchAnalyticsRecord>, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours);
        sqlx::query_as::<_, SearchAnalyticsRecord>(
            r#"
            SELECT * FROM search_analytics
            WHERE created_at > ?
            ORDER BY created_at DESC
            "#
        )
        .bind(cutoff.to_rfc3339())
        .fetch_all(&self.pool)
        .await
    }

    /// Get search analytics summary for a time period
    pub async fn get_search_analytics_summary(&self, hours: i64) -> Result<SearchAnalyticsSummary, sqlx::Error> {
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
        .fetch_one(&self.pool)
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
            SELECT query, COUNT(*) as count
            FROM search_analytics
            WHERE created_at > ?
            GROUP BY LOWER(TRIM(query))
            ORDER BY count DESC
            LIMIT 10
            "#
        )
        .bind(&cutoff_str)
        .fetch_all(&self.pool)
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
        .fetch_all(&self.pool)
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
        .fetch_all(&self.pool)
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

    /// Get popular queries with detailed stats
    pub async fn get_popular_queries(&self, limit: usize) -> Result<Vec<PopularQueryRecord>, sqlx::Error> {
        sqlx::query_as::<_, PopularQueryRecord>(
            r#"
            SELECT
                query,
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
        .fetch_all(&self.pool)
        .await
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> Result<SearchCacheStats, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN cache_hit = 1 THEN 1 ELSE 0 END), 0) as hits,
                COALESCE(SUM(CASE WHEN cache_hit = 0 THEN 1 ELSE 0 END), 0) as misses,
                COALESCE(AVG(CASE WHEN cache_hit = 0 THEN response_time_ms END), 0) as avg_uncached_time
            FROM search_analytics
            "#
        )
        .fetch_one(&self.pool)
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
            SELECT query, COUNT(*) as count
            FROM search_analytics
            WHERE cache_hit = 1
            GROUP BY LOWER(TRIM(query))
            ORDER BY count DESC
            LIMIT 10
            "#
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|r| (r.get::<String, _>("query"), r.get::<i64, _>("count") as u32))
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

    /// Get trending queries (recent surge in popularity)
    pub async fn get_trending_queries(&self, limit: usize) -> Result<Vec<String>, sqlx::Error> {
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
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|r| r.get("query"))
        .collect();

        Ok(queries)
    }

    /// Get queries with zero results
    pub async fn get_zero_result_queries(&self, hours: i64) -> Result<Vec<String>, sqlx::Error> {
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
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|r| r.get("query"))
        .collect();

        Ok(queries)
    }

    /// Get click position distribution
    pub async fn get_click_distribution(&self) -> Result<std::collections::HashMap<i32, u32>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT selected_result_index, COUNT(*) as count
            FROM search_analytics
            WHERE selected_result_index IS NOT NULL
            GROUP BY selected_result_index
            ORDER BY selected_result_index
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut distribution = std::collections::HashMap::new();
        for row in rows {
            let idx: i32 = row.get("selected_result_index");
            let count: i64 = row.get("count");
            distribution.insert(idx, count as u32);
        }

        Ok(distribution)
    }

    /// Clean up old search analytics records
    pub async fn cleanup_search_analytics(&self, days: i64) -> Result<u64, sqlx::Error> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339();

        let result = sqlx::query("DELETE FROM search_analytics WHERE created_at < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await?;

        sqlx::query("DELETE FROM search_selections WHERE created_at < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    // =========================================================================
    // Global Chat Session Operations
    // =========================================================================

    /// Create a new global chat session
    pub async fn create_chat_session(&self, session: &GlobalChatSessionRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO global_chat_sessions
            (id, status, linked_game_session_id, linked_campaign_id, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&session.id)
        .bind(&session.status)
        .bind(&session.linked_game_session_id)
        .bind(&session.linked_campaign_id)
        .bind(&session.created_at)
        .bind(&session.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get active chat session (there should only be one active at a time)
    pub async fn get_active_chat_session(&self) -> Result<Option<GlobalChatSessionRecord>, sqlx::Error> {
        sqlx::query_as::<_, GlobalChatSessionRecord>(
            "SELECT * FROM global_chat_sessions WHERE status = 'active' ORDER BY created_at DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Get chat session by ID
    pub async fn get_chat_session(&self, id: &str) -> Result<Option<GlobalChatSessionRecord>, sqlx::Error> {
        sqlx::query_as::<_, GlobalChatSessionRecord>(
            "SELECT * FROM global_chat_sessions WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Update chat session
    pub async fn update_chat_session(&self, session: &GlobalChatSessionRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE global_chat_sessions
            SET status = ?, linked_game_session_id = ?, linked_campaign_id = ?, updated_at = ?
            WHERE id = ?
            "#
        )
        .bind(&session.status)
        .bind(&session.linked_game_session_id)
        .bind(&session.linked_campaign_id)
        .bind(&session.updated_at)
        .bind(&session.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Archive a chat session
    pub async fn archive_chat_session(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE global_chat_sessions SET status = 'archived', updated_at = ? WHERE id = ?"
        )
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Link chat session to a game session
    pub async fn link_chat_session_to_game(
        &self,
        chat_session_id: &str,
        game_session_id: &str,
        campaign_id: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE global_chat_sessions
            SET linked_game_session_id = ?, linked_campaign_id = ?, updated_at = ?
            WHERE id = ?
            "#
        )
        .bind(game_session_id)
        .bind(campaign_id)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(chat_session_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get chat sessions linked to a game session
    pub async fn get_chat_sessions_by_game_session(&self, game_session_id: &str) -> Result<Vec<GlobalChatSessionRecord>, sqlx::Error> {
        sqlx::query_as::<_, GlobalChatSessionRecord>(
            "SELECT * FROM global_chat_sessions WHERE linked_game_session_id = ? ORDER BY created_at"
        )
        .bind(game_session_id)
        .fetch_all(&self.pool)
        .await
    }

    /// List all chat sessions (for history view)
    pub async fn list_chat_sessions(&self, limit: i32) -> Result<Vec<GlobalChatSessionRecord>, sqlx::Error> {
        sqlx::query_as::<_, GlobalChatSessionRecord>(
            "SELECT * FROM global_chat_sessions ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    // =========================================================================
    // Chat Message Operations
    // =========================================================================

    /// Add a chat message
    pub async fn add_chat_message(&self, message: &ChatMessageRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO chat_messages
            (id, session_id, role, content, tokens_input, tokens_output, is_streaming, metadata, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&message.id)
        .bind(&message.session_id)
        .bind(&message.role)
        .bind(&message.content)
        .bind(message.tokens_input)
        .bind(message.tokens_output)
        .bind(message.is_streaming)
        .bind(&message.metadata)
        .bind(&message.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get messages for a chat session
    pub async fn get_chat_messages(&self, session_id: &str, limit: i32) -> Result<Vec<ChatMessageRecord>, sqlx::Error> {
        sqlx::query_as::<_, ChatMessageRecord>(
            "SELECT * FROM chat_messages WHERE session_id = ? ORDER BY created_at DESC LIMIT ?"
        )
        .bind(session_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map(|mut msgs| {
            msgs.reverse(); // Return in chronological order
            msgs
        })
    }

    /// Get a single chat message by ID
    pub async fn get_chat_message(&self, id: &str) -> Result<Option<ChatMessageRecord>, sqlx::Error> {
        sqlx::query_as::<_, ChatMessageRecord>(
            "SELECT * FROM chat_messages WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Update message (e.g., when streaming completes)
    pub async fn update_chat_message(&self, message: &ChatMessageRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE chat_messages
            SET content = ?, tokens_input = ?, tokens_output = ?, is_streaming = ?, metadata = ?
            WHERE id = ?
            "#
        )
        .bind(&message.content)
        .bind(message.tokens_input)
        .bind(message.tokens_output)
        .bind(message.is_streaming)
        .bind(&message.metadata)
        .bind(&message.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete all messages in a session
    pub async fn clear_chat_messages(&self, session_id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM chat_messages WHERE session_id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Get or create active chat session
    /// Uses partial unique index to prevent race conditions - if insert fails due to
    /// constraint violation (another active session was created concurrently), retry get
    pub async fn get_or_create_active_chat_session(&self) -> Result<GlobalChatSessionRecord, sqlx::Error> {
        // First check if one already exists
        if let Some(session) = self.get_active_chat_session().await? {
            return Ok(session);
        }

        // Try to create a new active session
        let session = GlobalChatSessionRecord::new();
        match self.create_chat_session(&session).await {
            Ok(()) => Ok(session),
            Err(e) => {
                // Check if it's a unique constraint violation (race condition)
                let err_str = e.to_string();
                if err_str.contains("UNIQUE constraint failed") {
                    // Another concurrent call created an active session, fetch it
                    self.get_active_chat_session()
                        .await?
                        .ok_or_else(|| sqlx::Error::RowNotFound)
                } else {
                    Err(e)
                }
            }
        }
    }

    // =========================================================================
    // TTRPG Document Operations
    // =========================================================================

    /// Save a TTRPG document
    pub async fn save_ttrpg_document(&self, doc: &TTRPGDocumentRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO ttrpg_documents
            (id, source_document_id, name, element_type, game_system, content,
             attributes_json, challenge_rating, level, page_number, confidence,
             meilisearch_id, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&doc.id)
        .bind(&doc.source_document_id)
        .bind(&doc.name)
        .bind(&doc.element_type)
        .bind(&doc.game_system)
        .bind(&doc.content)
        .bind(&doc.attributes_json)
        .bind(doc.challenge_rating)
        .bind(doc.level)
        .bind(doc.page_number)
        .bind(doc.confidence)
        .bind(&doc.meilisearch_id)
        .bind(&doc.created_at)
        .bind(&doc.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get a TTRPG document by ID
    pub async fn get_ttrpg_document(&self, id: &str) -> Result<Option<TTRPGDocumentRecord>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGDocumentRecord>(
            "SELECT * FROM ttrpg_documents WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List TTRPG documents for a source document
    pub async fn list_ttrpg_documents_by_source(&self, source_document_id: &str) -> Result<Vec<TTRPGDocumentRecord>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGDocumentRecord>(
            "SELECT * FROM ttrpg_documents WHERE source_document_id = ? ORDER BY page_number, name"
        )
        .bind(source_document_id)
        .fetch_all(&self.pool)
        .await
    }

    /// List TTRPG documents by element type
    pub async fn list_ttrpg_documents_by_type(&self, element_type: &str) -> Result<Vec<TTRPGDocumentRecord>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGDocumentRecord>(
            "SELECT * FROM ttrpg_documents WHERE element_type = ? ORDER BY name"
        )
        .bind(element_type)
        .fetch_all(&self.pool)
        .await
    }

    /// List TTRPG documents by game system
    pub async fn list_ttrpg_documents_by_system(&self, game_system: &str) -> Result<Vec<TTRPGDocumentRecord>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGDocumentRecord>(
            "SELECT * FROM ttrpg_documents WHERE game_system = ? ORDER BY element_type, name"
        )
        .bind(game_system)
        .fetch_all(&self.pool)
        .await
    }

    /// Search TTRPG documents by name
    pub async fn search_ttrpg_documents_by_name(&self, name_pattern: &str) -> Result<Vec<TTRPGDocumentRecord>, sqlx::Error> {
        let pattern = format!("%{}%", name_pattern);
        sqlx::query_as::<_, TTRPGDocumentRecord>(
            "SELECT * FROM ttrpg_documents WHERE name LIKE ? ORDER BY name LIMIT 100"
        )
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await
    }

    /// List TTRPG documents by CR range
    pub async fn list_ttrpg_documents_by_cr(&self, min_cr: f64, max_cr: f64) -> Result<Vec<TTRPGDocumentRecord>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGDocumentRecord>(
            "SELECT * FROM ttrpg_documents WHERE challenge_rating >= ? AND challenge_rating <= ? ORDER BY challenge_rating, name"
        )
        .bind(min_cr)
        .bind(max_cr)
        .fetch_all(&self.pool)
        .await
    }

    /// Delete a TTRPG document
    pub async fn delete_ttrpg_document(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM ttrpg_documents WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete all TTRPG documents for a source document
    pub async fn delete_ttrpg_documents_by_source(&self, source_document_id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM ttrpg_documents WHERE source_document_id = ?")
            .bind(source_document_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Add attribute to a TTRPG document
    pub async fn add_ttrpg_document_attribute(
        &self,
        document_id: &str,
        attribute_type: &str,
        attribute_value: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO ttrpg_document_attributes (document_id, attribute_type, attribute_value) VALUES (?, ?, ?)"
        )
        .bind(document_id)
        .bind(attribute_type)
        .bind(attribute_value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get attributes for a TTRPG document
    pub async fn get_ttrpg_document_attributes(&self, document_id: &str) -> Result<Vec<TTRPGDocumentAttribute>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGDocumentAttribute>(
            "SELECT * FROM ttrpg_document_attributes WHERE document_id = ?"
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Find TTRPG documents by attribute
    pub async fn find_ttrpg_documents_by_attribute(
        &self,
        attribute_type: &str,
        attribute_value: &str,
    ) -> Result<Vec<TTRPGDocumentRecord>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGDocumentRecord>(
            r#"
            SELECT d.* FROM ttrpg_documents d
            JOIN ttrpg_document_attributes a ON d.id = a.document_id
            WHERE a.attribute_type = ? AND a.attribute_value = ?
            ORDER BY d.name
            "#
        )
        .bind(attribute_type)
        .bind(attribute_value)
        .fetch_all(&self.pool)
        .await
    }

    // =========================================================================
    // TTRPG Ingestion Job Operations
    // =========================================================================

    /// Create an ingestion job
    pub async fn create_ttrpg_ingestion_job(&self, job: &TTRPGIngestionJob) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO ttrpg_ingestion_jobs
            (id, document_id, status, total_pages, processed_pages, elements_found,
             errors_json, started_at, completed_at, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&job.id)
        .bind(&job.document_id)
        .bind(&job.status)
        .bind(job.total_pages)
        .bind(job.processed_pages)
        .bind(job.elements_found)
        .bind(&job.errors_json)
        .bind(&job.started_at)
        .bind(&job.completed_at)
        .bind(&job.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get an ingestion job by ID
    pub async fn get_ttrpg_ingestion_job(&self, id: &str) -> Result<Option<TTRPGIngestionJob>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGIngestionJob>(
            "SELECT * FROM ttrpg_ingestion_jobs WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Get ingestion job for a document
    pub async fn get_ttrpg_ingestion_job_by_document(&self, document_id: &str) -> Result<Option<TTRPGIngestionJob>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGIngestionJob>(
            "SELECT * FROM ttrpg_ingestion_jobs WHERE document_id = ? ORDER BY created_at DESC LIMIT 1"
        )
        .bind(document_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Update ingestion job progress
    pub async fn update_ttrpg_ingestion_job(&self, job: &TTRPGIngestionJob) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE ttrpg_ingestion_jobs
            SET status = ?, processed_pages = ?, elements_found = ?,
                errors_json = ?, started_at = ?, completed_at = ?
            WHERE id = ?
            "#
        )
        .bind(&job.status)
        .bind(job.processed_pages)
        .bind(job.elements_found)
        .bind(&job.errors_json)
        .bind(&job.started_at)
        .bind(&job.completed_at)
        .bind(&job.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// List pending ingestion jobs
    pub async fn list_pending_ttrpg_ingestion_jobs(&self) -> Result<Vec<TTRPGIngestionJob>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGIngestionJob>(
            "SELECT * FROM ttrpg_ingestion_jobs WHERE status = 'pending' ORDER BY created_at"
        )
        .fetch_all(&self.pool)
        .await
    }

    /// List active ingestion jobs
    pub async fn list_active_ttrpg_ingestion_jobs(&self) -> Result<Vec<TTRPGIngestionJob>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGIngestionJob>(
            "SELECT * FROM ttrpg_ingestion_jobs WHERE status = 'processing' ORDER BY started_at"
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Count TTRPG documents by type
    pub async fn count_ttrpg_documents_by_type(&self) -> Result<Vec<(String, i64)>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT element_type, COUNT(*) as count FROM ttrpg_documents GROUP BY element_type ORDER BY count DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter()
            .map(|row| (row.get("element_type"), row.get("count")))
            .collect())
    }

    /// Get TTRPG document statistics
    pub async fn get_ttrpg_document_stats(&self) -> Result<TTRPGDocumentStats, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total_documents,
                COUNT(DISTINCT source_document_id) as source_documents,
                COUNT(DISTINCT game_system) as game_systems,
                AVG(confidence) as avg_confidence
            FROM ttrpg_documents
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(TTRPGDocumentStats {
            total_documents: row.get::<i64, _>("total_documents") as u64,
            source_documents: row.get::<i64, _>("source_documents") as u32,
            game_systems: row.get::<i64, _>("game_systems") as u32,
            avg_confidence: row.get("avg_confidence"),
        })
    }
}

// ============================================================================
// TTRPG Document Statistics
// ============================================================================

/// Statistics about TTRPG documents in the database
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TTRPGDocumentStats {
    pub total_documents: u64,
    pub source_documents: u32,
    pub game_systems: u32,
    pub avg_confidence: Option<f64>,
}

// ============================================================================
// Search Analytics Summary Types
// ============================================================================

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
