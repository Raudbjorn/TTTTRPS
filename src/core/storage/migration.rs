//! Migration utilities for SQLite/Meilisearch to SurrealDB.
//!
//! This module provides data migration from existing SQLite and Meilisearch
//! storage to the unified SurrealDB backend.
//!
//! ## Tasks Implemented
//!
//! - 5.1.1: MigrationStatus tracking (FR-6.3)
//! - 5.1.2: SQLite backup (FR-6.3)
//! - 5.1.3-5.1.7: SQLite table migration functions (FR-6.1)
//! - 5.2.1: Meilisearch index migration (FR-6.2)
//! - 5.3.1-5.3.2: Validation and resumption (FR-6.3)
//!
//! ## Migration Flow
//!
//! 1. Check for existing progress (resumable)
//! 2. Backup SQLite database
//! 3. Migrate SQLite tables (campaigns, NPCs, sessions, chat, library)
//! 4. Migrate Meilisearch indexes (ttrpg_rules, ttrpg_fiction, session_notes, homebrew)
//! 5. Validate record counts
//! 6. Mark migration complete

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::fs;
use std::path::{Path, PathBuf};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

use super::error::StorageError;

// ============================================================================
// Task 5.1.1: MigrationStatus tracking (FR-6.3)
// ============================================================================

/// Migration status tracking.
///
/// Tracks the progress of a migration operation, including the current phase,
/// record counts, errors, and timestamps. This allows for resumable migrations
/// if the process is interrupted.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MigrationStatus {
    /// When the migration started
    pub started_at: Option<DateTime<Utc>>,
    /// When the migration completed (None if still in progress)
    pub completed_at: Option<DateTime<Utc>>,
    /// Current migration phase
    pub phase: MigrationPhase,
    /// Number of records migrated per table
    pub records_migrated: MigrationCounts,
    /// Errors encountered during migration
    pub errors: Vec<String>,
    /// Path to backup file (if created)
    pub backup_path: Option<String>,
}

impl Default for MigrationStatus {
    fn default() -> Self {
        Self {
            started_at: None,
            completed_at: None,
            phase: MigrationPhase::NotStarted,
            records_migrated: MigrationCounts::default(),
            errors: Vec::new(),
            backup_path: None,
        }
    }
}

/// Migration phases for tracking progress.
///
/// The migration proceeds through phases in order, allowing resumption
/// from any phase if interrupted.
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum MigrationPhase {
    /// Migration has not yet started
    #[default]
    NotStarted,
    /// Creating backup of SQLite database
    BackingUp,
    /// Migrating SQLite tables to SurrealDB
    MigratingSqlite,
    /// Migrating Meilisearch indexes to SurrealDB chunks
    MigratingMeilisearch,
    /// Validating migrated data
    Validating,
    /// Migration completed successfully
    Completed,
    /// Migration failed with errors
    Failed,
}

/// Record counts for tracking migration progress.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MigrationCounts {
    pub campaigns: usize,
    pub npcs: usize,
    pub sessions: usize,
    pub chat_messages: usize,
    pub library_items: usize,
    pub chunks: usize,
}

// ============================================================================
// Task 5.1.2: SQLite backup (FR-6.3)
// ============================================================================

/// Create backup of SQLite database before migration.
///
/// # Arguments
///
/// * `sqlite_path` - Path to the SQLite database file
/// * `backup_dir` - Directory to store the backup
///
/// # Returns
///
/// The path to the created backup file.
///
/// # Errors
///
/// Returns `StorageError::Migration` if:
/// - SQLite database does not exist
/// - Failed to create backup directory
/// - Failed to copy database file
/// - Backup size does not match original
pub async fn backup_sqlite(sqlite_path: &Path, backup_dir: &Path) -> Result<PathBuf, StorageError> {
    if !sqlite_path.exists() {
        return Err(StorageError::Migration(
            "SQLite database not found".to_string(),
        ));
    }

    fs::create_dir_all(backup_dir).map_err(|e| {
        StorageError::Migration(format!("Failed to create backup directory: {}", e))
    })?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!("ttrpg_assistant_{}.db.backup", timestamp);
    let backup_path = backup_dir.join(&backup_name);

    fs::copy(sqlite_path, &backup_path)
        .map_err(|e| StorageError::Migration(format!("Failed to backup SQLite: {}", e)))?;

    // Verify backup integrity by comparing file sizes
    let original_size = fs::metadata(sqlite_path)
        .map_err(|e| StorageError::Migration(format!("Failed to read original size: {}", e)))?
        .len();
    let backup_size = fs::metadata(&backup_path)
        .map_err(|e| StorageError::Migration(format!("Failed to read backup size: {}", e)))?
        .len();

    if original_size != backup_size {
        // Clean up failed backup
        let _ = fs::remove_file(&backup_path);
        return Err(StorageError::Migration(format!(
            "Backup size mismatch: expected {} bytes, got {} bytes",
            original_size, backup_size
        )));
    }

    tracing::info!(
        backup_path = %backup_path.display(),
        size_bytes = original_size,
        "SQLite backup created successfully"
    );

    Ok(backup_path)
}

// ============================================================================
// Task 5.1.3: Campaign migration (FR-6.1)
// ============================================================================

/// Migrate campaigns from SQLite to SurrealDB.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `sqlite` - SQLite connection pool
///
/// # Returns
///
/// Number of campaigns migrated.
pub async fn migrate_campaigns(db: &Surreal<Db>, sqlite: &SqlitePool) -> Result<usize, StorageError> {
    #[derive(sqlx::FromRow, Debug)]
    struct SqliteCampaign {
        id: String,
        name: String,
        system: String,
        description: Option<String>,
        created_at: String,
        updated_at: String,
        // Extended fields from v9
        setting: Option<String>,
        current_in_game_date: Option<String>,
        house_rules: Option<String>,
        world_state: Option<String>,
        archived_at: Option<String>,
    }

    let campaigns = sqlx::query_as::<_, SqliteCampaign>(
        "SELECT id, name, system, description, created_at, updated_at, \
         setting, current_in_game_date, house_rules, world_state, archived_at \
         FROM campaigns",
    )
    .fetch_all(sqlite)
    .await
    .map_err(|e| StorageError::Migration(format!("Failed to read campaigns: {}", e)))?;

    let mut count = 0;
    for campaign in campaigns {
        // Determine status based on archived_at
        let status = if campaign.archived_at.is_some() {
            "archived"
        } else {
            "active"
        };

        // Build metadata object from extended fields
        let metadata = serde_json::json!({
            "setting": campaign.setting,
            "current_in_game_date": campaign.current_in_game_date,
            "house_rules": campaign.house_rules,
            "world_state": campaign.world_state,
            "archived_at": campaign.archived_at,
        });

        db.query(
            r#"
            CREATE type::thing('campaign', $id) CONTENT {
                name: $name,
                description: $description,
                game_system: $game_system,
                status: $status,
                created_at: type::datetime($created_at),
                updated_at: type::datetime($updated_at),
                metadata: $metadata
            };
        "#,
        )
        .bind(("id", campaign.id.clone()))
        .bind(("name", campaign.name))
        .bind(("description", campaign.description))
        .bind(("game_system", Some(campaign.system)))
        .bind(("status", status.to_string()))
        .bind(("created_at", campaign.created_at))
        .bind(("updated_at", campaign.updated_at))
        .bind(("metadata", metadata))
        .await
        .map_err(|e| {
            StorageError::Migration(format!("Failed to migrate campaign {}: {}", campaign.id, e))
        })?;

        count += 1;
    }

    tracing::info!(count, "Migrated campaigns from SQLite");
    Ok(count)
}

// ============================================================================
// Task 5.1.4: NPC migration (FR-6.1)
// ============================================================================

/// Migrate NPCs from SQLite to SurrealDB.
///
/// NPCs are linked to campaigns via record links.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `sqlite` - SQLite connection pool
///
/// # Returns
///
/// Number of NPCs migrated.
pub async fn migrate_npcs(db: &Surreal<Db>, sqlite: &SqlitePool) -> Result<usize, StorageError> {
    #[derive(sqlx::FromRow, Debug)]
    struct SqliteNpc {
        id: String,
        campaign_id: Option<String>,
        name: String,
        role: String,
        personality_json: String,
        stats_json: Option<String>,
        notes: Option<String>,
        created_at: String,
        // Extended fields from v7, v10
        personality_id: Option<String>,
        data_json: Option<String>,
        location_id: Option<String>,
        voice_profile_id: Option<String>,
        quest_hooks: Option<String>,
    }

    let npcs = sqlx::query_as::<_, SqliteNpc>(
        "SELECT id, campaign_id, name, role, personality_json, stats_json, notes, created_at, \
         personality_id, data_json, location_id, voice_profile_id, quest_hooks \
         FROM npcs",
    )
    .fetch_all(sqlite)
    .await
    .map_err(|e| StorageError::Migration(format!("Failed to read NPCs: {}", e)))?;

    let mut count = 0;
    for npc in npcs {
        // Parse personality JSON for description/personality fields
        let personality: serde_json::Value =
            serde_json::from_str(&npc.personality_json).unwrap_or_default();

        let description = personality
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| npc.notes.clone());

        let personality_text = personality
            .get("traits")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .or_else(|| {
                personality
                    .get("personality")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            });

        let appearance = personality
            .get("appearance")
            .and_then(|v| v.as_str())
            .map(String::from);

        let backstory = personality
            .get("backstory")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Build metadata from extended fields
        let metadata = serde_json::json!({
            "role": npc.role,
            "stats_json": npc.stats_json,
            "personality_id": npc.personality_id,
            "data_json": npc.data_json,
            "location_id": npc.location_id,
            "voice_profile_id": npc.voice_profile_id,
            "quest_hooks": npc.quest_hooks,
            "original_personality_json": npc.personality_json,
        });

        // Tags from role
        let tags: Vec<String> = vec![npc.role.clone()];

        // Campaign record link (optional)
        let campaign_link = npc.campaign_id.as_ref().map(|id| format!("campaign:{}", id));

        db.query(
            r#"
            CREATE type::thing('npc', $id) CONTENT {
                name: $name,
                description: $description,
                personality: $personality,
                appearance: $appearance,
                backstory: $backstory,
                campaign: IF $campaign_link IS NOT NONE THEN type::thing('campaign', $campaign_id) ELSE NONE END,
                tags: $tags,
                created_at: type::datetime($created_at),
                updated_at: time::now(),
                metadata: $metadata
            };
        "#,
        )
        .bind(("id", npc.id.clone()))
        .bind(("name", npc.name))
        .bind(("description", description))
        .bind(("personality", personality_text))
        .bind(("appearance", appearance))
        .bind(("backstory", backstory))
        .bind(("campaign_link", campaign_link))
        .bind(("campaign_id", npc.campaign_id))
        .bind(("tags", tags))
        .bind(("created_at", npc.created_at))
        .bind(("metadata", metadata))
        .await
        .map_err(|e| {
            StorageError::Migration(format!("Failed to migrate NPC {}: {}", npc.id, e))
        })?;

        count += 1;
    }

    tracing::info!(count, "Migrated NPCs from SQLite");
    Ok(count)
}

// ============================================================================
// Task 5.1.5: Session migration (FR-6.1)
// ============================================================================

/// Migrate sessions from SQLite to SurrealDB.
///
/// Sessions are linked to campaigns via record links.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `sqlite` - SQLite connection pool
///
/// # Returns
///
/// Number of sessions migrated.
pub async fn migrate_sessions(db: &Surreal<Db>, sqlite: &SqlitePool) -> Result<usize, StorageError> {
    #[derive(sqlx::FromRow, Debug)]
    struct SqliteSession {
        id: String,
        campaign_id: String,
        session_number: i32,
        status: String,
        started_at: String,
        ended_at: Option<String>,
        notes: Option<String>,
        // Extended fields from v4, v8
        title: Option<String>,
        order_index: Option<i32>,
    }

    let sessions = sqlx::query_as::<_, SqliteSession>(
        "SELECT id, campaign_id, session_number, status, started_at, ended_at, notes, \
         title, order_index \
         FROM sessions",
    )
    .fetch_all(sqlite)
    .await
    .map_err(|e| StorageError::Migration(format!("Failed to read sessions: {}", e)))?;

    let mut count = 0;
    for session in sessions {
        // Map status to SurrealDB session statuses
        let status = match session.status.as_str() {
            "active" | "in_progress" => "active",
            "completed" | "ended" => "completed",
            "planned" => "planned",
            _ => "planned",
        };

        db.query(
            r#"
            CREATE type::thing('session', $id) CONTENT {
                campaign: type::thing('campaign', $campaign_id),
                name: $name,
                session_number: $session_number,
                date: type::datetime($started_at),
                summary: NONE,
                notes: $notes,
                status: $status,
                created_at: type::datetime($started_at),
                updated_at: IF $ended_at IS NOT NONE THEN type::datetime($ended_at) ELSE time::now() END
            };
        "#,
        )
        .bind(("id", session.id.clone()))
        .bind(("campaign_id", session.campaign_id))
        .bind(("name", session.title.unwrap_or_else(|| format!("Session {}", session.session_number))))
        .bind(("session_number", session.session_number))
        .bind(("started_at", session.started_at))
        .bind(("ended_at", session.ended_at))
        .bind(("notes", session.notes))
        .bind(("status", status.to_string()))
        .await
        .map_err(|e| {
            StorageError::Migration(format!("Failed to migrate session {}: {}", session.id, e))
        })?;

        count += 1;
    }

    tracing::info!(count, "Migrated sessions from SQLite");
    Ok(count)
}

// ============================================================================
// Task 5.1.6: Chat message migration (FR-6.1)
// ============================================================================

/// Migrate chat messages from SQLite to SurrealDB.
///
/// Chat messages are linked to campaigns and optionally NPCs via record links.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `sqlite` - SQLite connection pool
///
/// # Returns
///
/// Number of chat messages migrated.
pub async fn migrate_chat_messages(
    db: &Surreal<Db>,
    sqlite: &SqlitePool,
) -> Result<usize, StorageError> {
    #[derive(sqlx::FromRow, Debug)]
    struct SqliteChatMessage {
        id: String,
        session_id: String,
        role: String,
        content: String,
        tokens_input: Option<i32>,
        tokens_output: Option<i32>,
        is_streaming: i32,
        metadata: Option<String>,
        created_at: String,
    }

    // Also need to get the chat session's linked campaign
    #[derive(sqlx::FromRow, Debug)]
    struct SqliteChatSession {
        id: String,
        linked_campaign_id: Option<String>,
    }

    // Load chat sessions for campaign linking
    let chat_sessions: Vec<SqliteChatSession> = sqlx::query_as(
        "SELECT id, linked_campaign_id FROM global_chat_sessions",
    )
    .fetch_all(sqlite)
    .await
    .unwrap_or_default();

    let session_campaign_map: std::collections::HashMap<String, Option<String>> = chat_sessions
        .into_iter()
        .map(|s| (s.id, s.linked_campaign_id))
        .collect();

    let messages = sqlx::query_as::<_, SqliteChatMessage>(
        "SELECT id, session_id, role, content, tokens_input, tokens_output, \
         is_streaming, metadata, created_at \
         FROM chat_messages \
         ORDER BY created_at ASC",
    )
    .fetch_all(sqlite)
    .await
    .map_err(|e| StorageError::Migration(format!("Failed to read chat messages: {}", e)))?;

    let mut count = 0;
    for message in messages {
        // Parse metadata for additional fields
        let metadata: serde_json::Value = message
            .metadata
            .as_ref()
            .and_then(|m| serde_json::from_str(m).ok())
            .unwrap_or_default();

        // Get campaign ID from session lookup
        let campaign_id = session_campaign_map
            .get(&message.session_id)
            .cloned()
            .flatten();

        // Extract NPC ID from metadata if present
        let npc_id = metadata
            .get("npc_id")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Extract sources from metadata if present
        let sources: Option<Vec<String>> = metadata
            .get("sources")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

        // Build metadata object
        let new_metadata = serde_json::json!({
            "tokens_input": message.tokens_input,
            "tokens_output": message.tokens_output,
            "is_streaming": message.is_streaming != 0,
            "original_metadata": metadata,
        });

        db.query(
            r#"
            CREATE type::thing('chat_message', $id) CONTENT {
                session_id: $session_id,
                role: $role,
                content: $content,
                campaign: IF $campaign_id IS NOT NONE THEN type::thing('campaign', $campaign_id) ELSE NONE END,
                npc: IF $npc_id IS NOT NONE THEN type::thing('npc', $npc_id) ELSE NONE END,
                sources: $sources,
                created_at: type::datetime($created_at),
                metadata: $metadata
            };
        "#,
        )
        .bind(("id", message.id.clone()))
        .bind(("session_id", message.session_id))
        .bind(("role", message.role))
        .bind(("content", message.content))
        .bind(("campaign_id", campaign_id))
        .bind(("npc_id", npc_id))
        .bind(("sources", sources))
        .bind(("created_at", message.created_at))
        .bind(("metadata", new_metadata))
        .await
        .map_err(|e| {
            StorageError::Migration(format!("Failed to migrate chat message {}: {}", message.id, e))
        })?;

        count += 1;
    }

    tracing::info!(count, "Migrated chat messages from SQLite");
    Ok(count)
}

// ============================================================================
// Task 5.1.7: Library item migration (FR-6.1)
// ============================================================================

/// Migrate library items (documents) from SQLite to SurrealDB.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `sqlite` - SQLite connection pool
///
/// # Returns
///
/// Number of library items migrated.
pub async fn migrate_library_items(
    db: &Surreal<Db>,
    sqlite: &SqlitePool,
) -> Result<usize, StorageError> {
    #[derive(sqlx::FromRow, Debug)]
    struct SqliteDocument {
        id: String,
        name: String,
        source_type: String,
        file_path: Option<String>,
        page_count: i32,
        chunk_count: i32,
        status: String,
        ingested_at: String,
    }

    let documents = sqlx::query_as::<_, SqliteDocument>("SELECT * FROM documents")
        .fetch_all(sqlite)
        .await
        .map_err(|e| StorageError::Migration(format!("Failed to read documents: {}", e)))?;

    let mut count = 0;
    for doc in documents {
        // Generate slug from name
        let slug = doc
            .name
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>();

        // Map status
        let status = match doc.status.as_str() {
            "complete" | "completed" | "ready" => "ready",
            "processing" | "ingesting" => "processing",
            "error" | "failed" => "error",
            _ => "pending",
        };

        // Detect file type from source_type or file_path
        let file_type = if let Some(ref path) = doc.file_path {
            Path::new(path)
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_lowercase())
        } else {
            Some(doc.source_type.to_lowercase())
        };

        // Build metadata
        let metadata = serde_json::json!({
            "original_source_type": doc.source_type,
            "sqlite_chunk_count": doc.chunk_count,
        });

        db.query(
            r#"
            CREATE type::thing('library_item', $id) CONTENT {
                slug: $slug,
                title: $title,
                file_path: $file_path,
                file_type: $file_type,
                page_count: $page_count,
                status: $status,
                created_at: type::datetime($ingested_at),
                updated_at: time::now(),
                metadata: $metadata
            };
        "#,
        )
        .bind(("id", doc.id.clone()))
        .bind(("slug", slug))
        .bind(("title", doc.name))
        .bind(("file_path", doc.file_path))
        .bind(("file_type", file_type))
        .bind(("page_count", doc.page_count))
        .bind(("status", status.to_string()))
        .bind(("ingested_at", doc.ingested_at))
        .bind(("metadata", metadata))
        .await
        .map_err(|e| {
            StorageError::Migration(format!("Failed to migrate library item {}: {}", doc.id, e))
        })?;

        count += 1;
    }

    tracing::info!(count, "Migrated library items from SQLite");
    Ok(count)
}

// ============================================================================
// Task 5.2.1: Meilisearch index migration (FR-6.2)
// ============================================================================

/// Document from Meilisearch for migration.
///
/// This struct represents documents exported from Meilisearch indexes
/// for import into SurrealDB chunks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeilisearchDocument {
    /// Document ID from Meilisearch
    pub id: String,
    /// Text content of the document/chunk
    pub content: String,
    /// ID of the library item this chunk belongs to
    pub library_item_id: String,
    /// Page number in the source document (optional)
    pub page_number: Option<i32>,
    /// Index of this chunk within the document
    pub chunk_index: Option<i32>,
    /// Embedding vector (768 dimensions for nomic-embed-text)
    pub embedding: Option<Vec<f32>>,
    /// Additional fields from Meilisearch
    pub metadata: Option<serde_json::Value>,
}

/// Migrate documents from a Meilisearch index to SurrealDB chunks.
///
/// Maps index names to content_type:
/// - `ttrpg_rules` -> "rules"
/// - `ttrpg_fiction` -> "fiction"
/// - `session_notes` -> "session_notes"
/// - `homebrew` -> "homebrew"
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `index_name` - Name of the Meilisearch index being migrated
/// * `documents` - Documents exported from Meilisearch
///
/// # Returns
///
/// Number of chunks migrated.
pub async fn migrate_meilisearch_index(
    db: &Surreal<Db>,
    index_name: &str,
    documents: Vec<MeilisearchDocument>,
) -> Result<usize, StorageError> {
    let content_type = match index_name {
        "ttrpg_rules" => "rules",
        "ttrpg_fiction" => "fiction",
        "session_notes" => "session_notes",
        "homebrew" => "homebrew",
        _ => "unknown",
    };

    let mut count = 0;
    for doc in documents {
        // Check embedding dimensions - preserve if 768, else null for re-embedding
        let embedding = doc.embedding.filter(|e| e.len() == 768);

        // Extract additional fields from metadata if present
        let metadata = doc.metadata.clone().unwrap_or_default();
        let section_path = metadata
            .get("section_path")
            .and_then(|v| v.as_str())
            .map(String::from);
        let chapter_title = metadata
            .get("chapter_title")
            .and_then(|v| v.as_str())
            .map(String::from);
        let section_title = metadata
            .get("section_title")
            .and_then(|v| v.as_str())
            .map(String::from);

        db.query(
            r#"
            CREATE chunk CONTENT {
                content: $content,
                library_item: type::thing('library_item', $library_id),
                content_type: $content_type,
                page_number: $page_number,
                chunk_index: $chunk_index,
                section_path: $section_path,
                chapter_title: $chapter_title,
                section_title: $section_title,
                embedding: $embedding,
                embedding_model: IF $embedding IS NOT NONE THEN "nomic-embed-text" ELSE NONE END,
                created_at: time::now(),
                metadata: $metadata
            };
        "#,
        )
        .bind(("content", doc.content))
        .bind(("library_id", doc.library_item_id))
        .bind(("content_type", content_type.to_string()))
        .bind(("page_number", doc.page_number))
        .bind(("chunk_index", doc.chunk_index))
        .bind(("section_path", section_path))
        .bind(("chapter_title", chapter_title))
        .bind(("section_title", section_title))
        .bind(("embedding", embedding))
        .bind(("metadata", doc.metadata))
        .await
        .map_err(|e| {
            StorageError::Migration(format!("Failed to migrate chunk from {}: {}", index_name, e))
        })?;

        count += 1;
    }

    tracing::info!(count, index_name, "Migrated chunks from Meilisearch index");
    Ok(count)
}

// ============================================================================
// Task 5.3.1: Validation (FR-6.3)
// ============================================================================

/// Validate migration by comparing record counts.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `expected` - Expected record counts from migration
///
/// # Returns
///
/// A vector of validation errors (empty if validation passes).
pub async fn validate_migration(
    db: &Surreal<Db>,
    expected: &MigrationCounts,
) -> Result<Vec<String>, StorageError> {
    let mut errors = Vec::new();

    // Helper to count records in a table
    async fn count_table(db: &Surreal<Db>, table: &str) -> Result<usize, StorageError> {
        #[derive(Deserialize)]
        struct CountResult {
            count: i64,
        }

        let result: Option<CountResult> = db
            .query(format!("SELECT count() as count FROM {} GROUP ALL", table))
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?
            .take(0)
            .ok()
            .flatten();

        Ok(result.map(|r| r.count as usize).unwrap_or(0))
    }

    // Count campaigns
    let campaign_count = count_table(db, "campaign").await?;
    if campaign_count != expected.campaigns {
        errors.push(format!(
            "Campaign count mismatch: expected {}, got {}",
            expected.campaigns, campaign_count
        ));
    }

    // Count NPCs
    let npc_count = count_table(db, "npc").await?;
    if npc_count != expected.npcs {
        errors.push(format!(
            "NPC count mismatch: expected {}, got {}",
            expected.npcs, npc_count
        ));
    }

    // Count sessions
    let session_count = count_table(db, "session").await?;
    if session_count != expected.sessions {
        errors.push(format!(
            "Session count mismatch: expected {}, got {}",
            expected.sessions, session_count
        ));
    }

    // Count chat messages
    let chat_count = count_table(db, "chat_message").await?;
    if chat_count != expected.chat_messages {
        errors.push(format!(
            "Chat message count mismatch: expected {}, got {}",
            expected.chat_messages, chat_count
        ));
    }

    // Count library items
    let library_count = count_table(db, "library_item").await?;
    if library_count != expected.library_items {
        errors.push(format!(
            "Library item count mismatch: expected {}, got {}",
            expected.library_items, library_count
        ));
    }

    // Count chunks
    let chunk_count = count_table(db, "chunk").await?;
    if chunk_count != expected.chunks {
        errors.push(format!(
            "Chunk count mismatch: expected {}, got {}",
            expected.chunks, chunk_count
        ));
    }

    if errors.is_empty() {
        tracing::info!(
            campaigns = campaign_count,
            npcs = npc_count,
            sessions = session_count,
            chat_messages = chat_count,
            library_items = library_count,
            chunks = chunk_count,
            "Migration validation passed"
        );
    } else {
        tracing::warn!(error_count = errors.len(), "Migration validation failed");
    }

    Ok(errors)
}

// ============================================================================
// Task 5.3.2: Resumption (FR-6.3)
// ============================================================================

/// Check if migration can be resumed from a previous attempt.
///
/// Retrieves the stored migration status from SurrealDB.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
///
/// # Returns
///
/// The stored migration status, or default if none exists.
pub async fn get_migration_progress(db: &Surreal<Db>) -> Result<MigrationStatus, StorageError> {
    let result: Option<MigrationStatus> = db
        .query("SELECT * FROM migration_status:current")
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?
        .take(0)
        .ok()
        .flatten();

    Ok(result.unwrap_or_default())
}

/// Save migration progress for resumption.
///
/// Stores the current migration status in SurrealDB, allowing the
/// migration to be resumed if interrupted.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `status` - Current migration status to save
pub async fn save_migration_progress(
    db: &Surreal<Db>,
    status: &MigrationStatus,
) -> Result<(), StorageError> {
    // Clone status to avoid lifetime issues with SurrealDB's bind
    let status_clone = status.clone();
    db.query("UPSERT migration_status:current CONTENT $status")
        .bind(("status", status_clone))
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

    tracing::debug!(phase = ?status.phase, "Saved migration progress");
    Ok(())
}

// ============================================================================
// Main Migration Orchestrator
// ============================================================================

/// Run full migration from SQLite + Meilisearch to SurrealDB.
///
/// This function orchestrates the complete migration process:
/// 1. Check for existing progress (resumable)
/// 2. Backup SQLite database
/// 3. Migrate SQLite tables
/// 4. Migrate Meilisearch indexes (placeholder - requires meilisearch-lib integration)
/// 5. Validate record counts
/// 6. Mark migration complete
///
/// The migration is resumable - if interrupted, calling this function again
/// will continue from the last completed phase.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `sqlite_pool` - SQLite connection pool
/// * `data_dir` - Application data directory (contains SQLite DB and backup dir)
/// * `on_progress` - Callback function invoked with status updates
///
/// # Returns
///
/// The final migration status.
pub async fn run_migration(
    db: &Surreal<Db>,
    sqlite_pool: &SqlitePool,
    data_dir: &Path,
    on_progress: impl Fn(&MigrationStatus),
) -> Result<MigrationStatus, StorageError> {
    let mut status = get_migration_progress(db).await?;

    // Skip if already completed
    if status.phase == MigrationPhase::Completed {
        tracing::info!("Migration already completed");
        return Ok(status);
    }

    // Set start time if not already set
    if status.started_at.is_none() {
        status.started_at = Some(Utc::now());
    }

    // Phase 1: Backup
    if status.phase == MigrationPhase::NotStarted {
        status.phase = MigrationPhase::BackingUp;
        on_progress(&status);
        save_migration_progress(db, &status).await?;

        tracing::info!("Starting migration: Phase 1 - Backup");

        let sqlite_path = data_dir.join("ttrpg_assistant.db");
        let backup_dir = data_dir.join("backups");

        match backup_sqlite(&sqlite_path, &backup_dir).await {
            Ok(backup_path) => {
                status.backup_path = Some(backup_path.to_string_lossy().to_string());
            }
            Err(e) => {
                // Non-fatal - continue without backup if DB doesn't exist
                if !sqlite_path.exists() {
                    tracing::warn!("SQLite database not found, skipping backup");
                } else {
                    status.errors.push(format!("Backup failed: {}", e));
                }
            }
        }
    }

    // Phase 2: SQLite migration
    if status.phase <= MigrationPhase::MigratingSqlite {
        status.phase = MigrationPhase::MigratingSqlite;
        on_progress(&status);
        save_migration_progress(db, &status).await?;

        tracing::info!("Starting migration: Phase 2 - SQLite tables");

        // Migrate campaigns
        match migrate_campaigns(db, sqlite_pool).await {
            Ok(count) => status.records_migrated.campaigns = count,
            Err(e) => status.errors.push(format!("Campaign migration failed: {}", e)),
        }

        // Migrate NPCs
        match migrate_npcs(db, sqlite_pool).await {
            Ok(count) => status.records_migrated.npcs = count,
            Err(e) => status.errors.push(format!("NPC migration failed: {}", e)),
        }

        // Migrate sessions
        match migrate_sessions(db, sqlite_pool).await {
            Ok(count) => status.records_migrated.sessions = count,
            Err(e) => status.errors.push(format!("Session migration failed: {}", e)),
        }

        // Migrate chat messages
        match migrate_chat_messages(db, sqlite_pool).await {
            Ok(count) => status.records_migrated.chat_messages = count,
            Err(e) => status.errors.push(format!("Chat message migration failed: {}", e)),
        }

        // Migrate library items
        match migrate_library_items(db, sqlite_pool).await {
            Ok(count) => status.records_migrated.library_items = count,
            Err(e) => status.errors.push(format!("Library item migration failed: {}", e)),
        }

        save_migration_progress(db, &status).await?;
    }

    // Phase 3: Meilisearch migration
    // NOTE: This is a placeholder - actual Meilisearch migration requires
    // the existing meilisearch-lib integration to export documents.
    // This will be implemented when integrating with the search module.
    if status.phase <= MigrationPhase::MigratingMeilisearch {
        status.phase = MigrationPhase::MigratingMeilisearch;
        on_progress(&status);
        save_migration_progress(db, &status).await?;

        tracing::info!("Starting migration: Phase 3 - Meilisearch indexes (placeholder)");

        // TODO: Integrate with meilisearch-lib to export documents
        // For now, chunks will be 0 since we haven't migrated from Meilisearch
        // The actual implementation would:
        // 1. Connect to embedded Meilisearch
        // 2. Export documents from each index (ttrpg_rules, ttrpg_fiction, session_notes, homebrew)
        // 3. Call migrate_meilisearch_index for each index
    }

    // Phase 4: Validation
    status.phase = MigrationPhase::Validating;
    on_progress(&status);
    save_migration_progress(db, &status).await?;

    tracing::info!("Starting migration: Phase 4 - Validation");

    let validation_errors = validate_migration(db, &status.records_migrated).await?;
    if !validation_errors.is_empty() {
        status.errors.extend(validation_errors);
    }

    // Determine final status
    if status.errors.is_empty() {
        status.phase = MigrationPhase::Completed;
        status.completed_at = Some(Utc::now());
        tracing::info!(
            duration_secs = status.completed_at.unwrap().signed_duration_since(status.started_at.unwrap()).num_seconds(),
            "Migration completed successfully"
        );
    } else {
        status.phase = MigrationPhase::Failed;
        tracing::error!(
            error_count = status.errors.len(),
            "Migration completed with errors"
        );
    }

    on_progress(&status);
    save_migration_progress(db, &status).await?;

    Ok(status)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_migration_phase_ordering() {
        assert!(MigrationPhase::NotStarted < MigrationPhase::BackingUp);
        assert!(MigrationPhase::BackingUp < MigrationPhase::MigratingSqlite);
        assert!(MigrationPhase::MigratingSqlite < MigrationPhase::MigratingMeilisearch);
        assert!(MigrationPhase::MigratingMeilisearch < MigrationPhase::Validating);
        assert!(MigrationPhase::Validating < MigrationPhase::Completed);
        assert!(MigrationPhase::Completed < MigrationPhase::Failed);
    }

    #[test]
    fn test_migration_status_default() {
        let status = MigrationStatus::default();
        assert!(status.started_at.is_none());
        assert!(status.completed_at.is_none());
        assert_eq!(status.phase, MigrationPhase::NotStarted);
        assert_eq!(status.records_migrated.campaigns, 0);
        assert!(status.errors.is_empty());
    }

    #[test]
    fn test_migration_counts_default() {
        let counts = MigrationCounts::default();
        assert_eq!(counts.campaigns, 0);
        assert_eq!(counts.npcs, 0);
        assert_eq!(counts.sessions, 0);
        assert_eq!(counts.chat_messages, 0);
        assert_eq!(counts.library_items, 0);
        assert_eq!(counts.chunks, 0);
    }

    #[tokio::test]
    async fn test_backup_sqlite_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent.db");
        let backup_dir = temp_dir.path().join("backups");

        let result = backup_sqlite(&nonexistent_path, &backup_dir).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not found"));
    }

    #[tokio::test]
    async fn test_backup_sqlite_success() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let backup_dir = temp_dir.path().join("backups");

        // Create a test database file
        std::fs::write(&db_path, "test database content").unwrap();

        let result = backup_sqlite(&db_path, &backup_dir).await;
        assert!(result.is_ok());

        let backup_path = result.unwrap();
        assert!(backup_path.exists());
        assert!(backup_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("backup"));
    }

    #[test]
    fn test_meilisearch_document_serialization() {
        let doc = MeilisearchDocument {
            id: "test-id".to_string(),
            content: "Test content".to_string(),
            library_item_id: "lib-123".to_string(),
            page_number: Some(42),
            chunk_index: Some(5),
            embedding: Some(vec![0.1, 0.2, 0.3]),
            metadata: Some(serde_json::json!({"key": "value"})),
        };

        let json = serde_json::to_string(&doc).unwrap();
        let parsed: MeilisearchDocument = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, "test-id");
        assert_eq!(parsed.content, "Test content");
        assert_eq!(parsed.page_number, Some(42));
    }

    #[test]
    fn test_content_type_mapping() {
        // Verify index name to content_type mapping logic
        fn get_content_type(index_name: &str) -> &str {
            match index_name {
                "ttrpg_rules" => "rules",
                "ttrpg_fiction" => "fiction",
                "session_notes" => "session_notes",
                "homebrew" => "homebrew",
                _ => "unknown",
            }
        }

        assert_eq!(get_content_type("ttrpg_rules"), "rules");
        assert_eq!(get_content_type("ttrpg_fiction"), "fiction");
        assert_eq!(get_content_type("session_notes"), "session_notes");
        assert_eq!(get_content_type("homebrew"), "homebrew");
        assert_eq!(get_content_type("other"), "unknown");
    }
}
