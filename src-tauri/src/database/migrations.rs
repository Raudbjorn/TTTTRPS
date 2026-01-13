//! Database Migrations
//!
//! Handles schema creation and versioned migrations.

use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use tracing::{info, warn};

/// Current database schema version
const SCHEMA_VERSION: i32 = 20;

/// Run all pending migrations
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Create migrations table if it doesn't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS _migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#
    )
    .execute(pool)
    .await?;

    // Get current version
    let current_version = get_current_version(pool).await?;

    info!(current_version, target_version = SCHEMA_VERSION, "Checking database migrations");

    if current_version < SCHEMA_VERSION {
        info!("Running database migrations from v{} to v{}", current_version, SCHEMA_VERSION);

        // Run migrations in order
        for version in (current_version + 1)..=SCHEMA_VERSION {
            run_migration(pool, version).await?;
        }

        info!("Database migrations completed successfully");
    }

    Ok(())
}

/// Get the current schema version
async fn get_current_version(pool: &SqlitePool) -> Result<i32, sqlx::Error> {
    let result = sqlx::query("SELECT MAX(version) as version FROM _migrations")
        .fetch_optional(pool)
        .await?;

    Ok(result
        .and_then(|row| row.try_get::<i32, _>("version").ok())
        .unwrap_or(0))
}

/// Run a specific migration version
async fn run_migration(pool: &SqlitePool, version: i32) -> Result<(), sqlx::Error> {
    let (name, sql) = match version {
        1 => ("initial_schema", MIGRATION_V1),
        2 => ("extended_features", MIGRATION_V2),
        3 => ("npc_conversations", MIGRATION_V3),
        4 => ("session_title", MIGRATION_V4),
        5 => ("personalities_table", MIGRATION_V5),
        6 => ("npc_personality_link", MIGRATION_V6),
        7 => ("npc_data_json", MIGRATION_V7),
        8 => ("session_ordering", MIGRATION_V8),
        9 => ("campaign_extended_fields", MIGRATION_V9),
        10 => ("npc_extended_fields", MIGRATION_V10),
        11 => ("campaign_versions", MIGRATION_V11),
        12 => ("entity_relationships", MIGRATION_V12),
        13 => ("voice_profiles", MIGRATION_V13),
        14 => ("session_notes", MIGRATION_V14),
        15 => ("session_events", MIGRATION_V15),
        16 => ("combat_states", MIGRATION_V16),
        17 => ("search_analytics", MIGRATION_V17),
        18 => ("global_chat_sessions", MIGRATION_V18),
        19 => ("chat_session_unique_active", MIGRATION_V19),
        20 => ("ttrpg_documents", MIGRATION_V20),
        _ => {
            warn!("Unknown migration version: {}", version);
            return Ok(());
        }
    };

    info!("Applying migration v{}: {}", version, name);

    // Execute migration SQL
    for statement in sql.split(";").filter(|s| !s.trim().is_empty()) {
        sqlx::query(statement.trim())
            .execute(pool)
            .await?;
    }

    // Record migration
    sqlx::query("INSERT INTO _migrations (version, name) VALUES (?, ?)")
        .bind(version)
        .bind(name)
        .execute(pool)
        .await?;

    Ok(())
}

/// Migration v1: Initial schema
const MIGRATION_V1: &str = r#"
-- Campaigns table
CREATE TABLE IF NOT EXISTS campaigns (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    system TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_campaigns_updated ON campaigns(updated_at DESC);

-- Sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    session_number INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    started_at TEXT NOT NULL,
    ended_at TEXT,
    notes TEXT,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sessions_campaign ON sessions(campaign_id);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);

-- Characters table
CREATE TABLE IF NOT EXISTS characters (
    id TEXT PRIMARY KEY,
    campaign_id TEXT,
    name TEXT NOT NULL,
    system TEXT NOT NULL,
    character_type TEXT NOT NULL DEFAULT 'player',
    level INTEGER,
    data_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_characters_campaign ON characters(campaign_id);
CREATE INDEX IF NOT EXISTS idx_characters_type ON characters(character_type);

-- NPCs table
CREATE TABLE IF NOT EXISTS npcs (
    id TEXT PRIMARY KEY,
    campaign_id TEXT,
    name TEXT NOT NULL,
    role TEXT NOT NULL,
    personality_json TEXT NOT NULL,
    stats_json TEXT,
    notes TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_npcs_campaign ON npcs(campaign_id);

-- Combat encounters table
CREATE TABLE IF NOT EXISTS combats (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    round INTEGER NOT NULL DEFAULT 1,
    current_turn INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    combatants_json TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_combats_session ON combats(session_id);
CREATE INDEX IF NOT EXISTS idx_combats_active ON combats(is_active);

-- Campaign snapshots for versioning/rollback
CREATE TABLE IF NOT EXISTS campaign_snapshots (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    description TEXT NOT NULL,
    snapshot_type TEXT NOT NULL DEFAULT 'manual',
    data_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_snapshots_campaign ON campaign_snapshots(campaign_id);
CREATE INDEX IF NOT EXISTS idx_snapshots_created ON campaign_snapshots(created_at DESC);

-- Documents (ingested sources)
CREATE TABLE IF NOT EXISTS documents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    source_type TEXT NOT NULL,
    file_path TEXT,
    page_count INTEGER NOT NULL DEFAULT 0,
    chunk_count INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending',
    ingested_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_documents_status ON documents(status);

-- Usage tracking
CREATE TABLE IF NOT EXISTS usage_logs (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    estimated_cost_usd REAL NOT NULL DEFAULT 0.0,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_usage_provider ON usage_logs(provider);
CREATE INDEX IF NOT EXISTS idx_usage_timestamp ON usage_logs(timestamp DESC);

-- Settings key-value store
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Session-specific usage (for tracking current session costs)
CREATE TABLE IF NOT EXISTS session_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_start TEXT NOT NULL,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    requests INTEGER NOT NULL DEFAULT 0,
    cost_usd REAL NOT NULL DEFAULT 0.0
);
"#;

/// Migration v2: Extended features (locations, plots, analytics, etc.)
const MIGRATION_V2: &str = r#"
-- Locations table (hierarchical location management)
CREATE TABLE IF NOT EXISTS locations (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    name TEXT NOT NULL,
    location_type TEXT NOT NULL,
    description TEXT,
    parent_id TEXT,
    connections_json TEXT NOT NULL DEFAULT '[]',
    npcs_present_json TEXT NOT NULL DEFAULT '[]',
    features_json TEXT NOT NULL DEFAULT '[]',
    secrets_json TEXT NOT NULL DEFAULT '[]',
    attributes_json TEXT NOT NULL DEFAULT '{}',
    tags_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES locations(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_locations_campaign ON locations(campaign_id);
CREATE INDEX IF NOT EXISTS idx_locations_parent ON locations(parent_id);
CREATE INDEX IF NOT EXISTS idx_locations_type ON locations(location_type);

-- Plot points table (quest and story tracking)
CREATE TABLE IF NOT EXISTS plot_points (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    priority TEXT NOT NULL DEFAULT 'side',
    involved_npcs_json TEXT NOT NULL DEFAULT '[]',
    involved_locations_json TEXT NOT NULL DEFAULT '[]',
    prerequisites_json TEXT NOT NULL DEFAULT '[]',
    unlocks_json TEXT NOT NULL DEFAULT '[]',
    consequences_json TEXT NOT NULL DEFAULT '[]',
    rewards_json TEXT NOT NULL DEFAULT '[]',
    notes_json TEXT NOT NULL DEFAULT '[]',
    tags_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    started_at TEXT,
    resolved_at TEXT,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_plot_points_campaign ON plot_points(campaign_id);
CREATE INDEX IF NOT EXISTS idx_plot_points_status ON plot_points(status);
CREATE INDEX IF NOT EXISTS idx_plot_points_priority ON plot_points(priority);

-- Plot arcs table (grouping of related plot points)
CREATE TABLE IF NOT EXISTS plot_arcs (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    plot_points_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_plot_arcs_campaign ON plot_arcs(campaign_id);

-- Session summaries table
CREATE TABLE IF NOT EXISTS session_summaries (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    campaign_id TEXT NOT NULL,
    summary TEXT NOT NULL,
    key_events_json TEXT NOT NULL DEFAULT '[]',
    combat_outcomes_json TEXT NOT NULL DEFAULT '[]',
    npcs_encountered_json TEXT NOT NULL DEFAULT '[]',
    locations_visited_json TEXT NOT NULL DEFAULT '[]',
    loot_acquired_json TEXT NOT NULL DEFAULT '[]',
    xp_awarded INTEGER,
    recap TEXT,
    generated_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_session_summaries_session ON session_summaries(session_id);
CREATE INDEX IF NOT EXISTS idx_session_summaries_campaign ON session_summaries(campaign_id);

-- Search analytics table
CREATE TABLE IF NOT EXISTS search_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    query TEXT NOT NULL,
    result_count INTEGER NOT NULL DEFAULT 0,
    clicked INTEGER NOT NULL DEFAULT 0,
    execution_time_ms INTEGER NOT NULL DEFAULT 0,
    search_type TEXT NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_search_records_timestamp ON search_records(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_search_records_query ON search_records(query);

-- Voice generation cache table
CREATE TABLE IF NOT EXISTS voice_cache (
    id TEXT PRIMARY KEY,
    text_hash TEXT NOT NULL UNIQUE,
    voice_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_accessed TEXT NOT NULL,
    access_count INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_voice_cache_hash ON voice_cache(text_hash);
CREATE INDEX IF NOT EXISTS idx_voice_cache_accessed ON voice_cache(last_accessed DESC);

-- Voice generation queue table
CREATE TABLE IF NOT EXISTS voice_queue (
    id TEXT PRIMARY KEY,
    text TEXT NOT NULL,
    voice_id TEXT NOT NULL,
    priority TEXT NOT NULL DEFAULT 'normal',
    status TEXT NOT NULL DEFAULT 'pending',
    campaign_id TEXT,
    npc_id TEXT,
    created_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT,
    result_path TEXT,
    error TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3
);

CREATE INDEX IF NOT EXISTS idx_voice_queue_status ON voice_queue(status);
CREATE INDEX IF NOT EXISTS idx_voice_queue_priority ON voice_queue(priority);
CREATE INDEX IF NOT EXISTS idx_voice_queue_campaign ON voice_queue(campaign_id);

-- Audit logs table
CREATE TABLE IF NOT EXISTS audit_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    actor TEXT,
    target TEXT,
    details_json TEXT,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_type ON audit_logs(event_type);
CREATE INDEX IF NOT EXISTS idx_audit_logs_severity ON audit_logs(severity);

-- Budget tracking table
CREATE TABLE IF NOT EXISTS budget_periods (
    id TEXT PRIMARY KEY,
    period_type TEXT NOT NULL,
    period_start TEXT NOT NULL,
    period_end TEXT NOT NULL,
    budget_limit_usd REAL NOT NULL,
    spent_usd REAL NOT NULL DEFAULT 0.0,
    provider TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_budget_period_type ON budget_periods(period_type);
CREATE INDEX IF NOT EXISTS idx_budget_period_start ON budget_periods(period_start);

-- Budget spending records
CREATE TABLE IF NOT EXISTS budget_spending (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    period_id TEXT NOT NULL,
    amount_usd REAL NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    tokens INTEGER NOT NULL DEFAULT 0,
    timestamp TEXT NOT NULL,
    FOREIGN KEY (period_id) REFERENCES budget_periods(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_budget_spending_period ON budget_spending(period_id);
CREATE INDEX IF NOT EXISTS idx_budget_spending_timestamp ON budget_spending(timestamp DESC);

-- Alerts table
CREATE TABLE IF NOT EXISTS alerts (
    id TEXT PRIMARY KEY,
    alert_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    context_json TEXT,
    acknowledged INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    acknowledged_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_alerts_type ON alerts(alert_type);
CREATE INDEX IF NOT EXISTS idx_alerts_severity ON alerts(severity);
CREATE INDEX IF NOT EXISTS idx_alerts_acknowledged ON alerts(acknowledged);
CREATE INDEX IF NOT EXISTS idx_alerts_created ON alerts(created_at DESC);

-- Cost predictions table
CREATE TABLE IF NOT EXISTS cost_predictions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    period_start TEXT NOT NULL,
    period_end TEXT NOT NULL,
    predicted_cost_usd REAL NOT NULL,
    confidence_low REAL NOT NULL,
    confidence_high REAL NOT NULL,
    usage_pattern TEXT,
    anomaly_detected INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cost_predictions_period ON cost_predictions(period_start, period_end);
"#;

/// Migration v3: NPC Conversations
const MIGRATION_V3: &str = r#"
-- NPC Conversations table
CREATE TABLE IF NOT EXISTS npc_conversations (
    id TEXT PRIMARY KEY,
    npc_id TEXT NOT NULL,
    campaign_id TEXT NOT NULL,
    messages_json TEXT NOT NULL DEFAULT '[]',
    unread_count INTEGER NOT NULL DEFAULT 0,
    last_message_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (npc_id) REFERENCES npcs(id) ON DELETE CASCADE,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_npc_conversations_npc ON npc_conversations(npc_id);
CREATE INDEX IF NOT EXISTS idx_npc_conversations_campaign ON npc_conversations(campaign_id);
CREATE INDEX IF NOT EXISTS idx_npc_conversations_last_msg ON npc_conversations(last_message_at DESC);
"#;

/// Migration v4: Session Title
const MIGRATION_V4: &str = r#"
ALTER TABLE sessions ADD COLUMN title TEXT;
"#;

/// Migration v5: Personalities table
const MIGRATION_V5: &str = r#"
CREATE TABLE IF NOT EXISTS personalities (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT,
    data_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_personalities_name ON personalities(name);
"#;



/// Migration v6: Link NPCs to Personalities
const MIGRATION_V6: &str = r#"
ALTER TABLE npcs ADD COLUMN personality_id TEXT REFERENCES personalities(id);
CREATE INDEX IF NOT EXISTS idx_npcs_personality ON npcs(personality_id);
"#;

/// Migration v7: Add full data JSON to NPCs
const MIGRATION_V7: &str = r#"
ALTER TABLE npcs ADD COLUMN data_json TEXT;
"#;

/// Migration v8: Add ordering to sessions
const MIGRATION_V8: &str = r#"
ALTER TABLE sessions ADD COLUMN order_index INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_sessions_order ON sessions(order_index);
"#;

/// Migration v9: Add extended fields to campaigns table
const MIGRATION_V9: &str = r#"
ALTER TABLE campaigns ADD COLUMN setting TEXT;
ALTER TABLE campaigns ADD COLUMN current_in_game_date TEXT;
ALTER TABLE campaigns ADD COLUMN house_rules TEXT;
ALTER TABLE campaigns ADD COLUMN world_state TEXT;
ALTER TABLE campaigns ADD COLUMN archived_at TEXT;
"#;

/// Migration v10: Add extended fields to NPCs table
/// Note: voice_profile_id and location_id references are added but FK constraints
/// are not enforced on existing columns in SQLite without table recreation
const MIGRATION_V10: &str = r#"
ALTER TABLE npcs ADD COLUMN location_id TEXT;
ALTER TABLE npcs ADD COLUMN voice_profile_id TEXT;
ALTER TABLE npcs ADD COLUMN quest_hooks TEXT;
CREATE INDEX IF NOT EXISTS idx_npcs_location ON npcs(location_id);
CREATE INDEX IF NOT EXISTS idx_npcs_voice_profile ON npcs(voice_profile_id);
"#;

/// Migration v11: Campaign versioning system
const MIGRATION_V11: &str = r#"
CREATE TABLE IF NOT EXISTS campaign_versions (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    version_number INTEGER NOT NULL,
    snapshot_type TEXT NOT NULL,
    description TEXT,
    data TEXT NOT NULL,
    diff_data TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_campaign_versions_campaign ON campaign_versions(campaign_id);
CREATE INDEX IF NOT EXISTS idx_campaign_versions_number ON campaign_versions(campaign_id, version_number DESC);
"#;

/// Migration v12: Entity relationships (many-to-many between campaign entities)
const MIGRATION_V12: &str = r#"
CREATE TABLE IF NOT EXISTS entity_relationships (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    source_entity_type TEXT NOT NULL,
    source_entity_id TEXT NOT NULL,
    target_entity_type TEXT NOT NULL,
    target_entity_id TEXT NOT NULL,
    relationship_type TEXT NOT NULL,
    description TEXT,
    strength REAL DEFAULT 1.0,
    bidirectional INTEGER DEFAULT 0,
    metadata TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id) ON DELETE CASCADE,
    UNIQUE(source_entity_type, source_entity_id, target_entity_type, target_entity_id, relationship_type)
);

CREATE INDEX IF NOT EXISTS idx_entity_relationships_campaign ON entity_relationships(campaign_id);
CREATE INDEX IF NOT EXISTS idx_entity_relationships_source ON entity_relationships(source_entity_type, source_entity_id);
CREATE INDEX IF NOT EXISTS idx_entity_relationships_target ON entity_relationships(target_entity_type, target_entity_id);
CREATE INDEX IF NOT EXISTS idx_entity_relationships_type ON entity_relationships(relationship_type);
"#;

/// Migration v13: Voice profiles for NPCs
const MIGRATION_V13: &str = r#"
CREATE TABLE IF NOT EXISTS voice_profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider TEXT NOT NULL,
    voice_id TEXT NOT NULL,
    settings TEXT,
    age_range TEXT,
    gender TEXT,
    personality_traits TEXT,
    is_preset INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_voice_profiles_name ON voice_profiles(name);
CREATE INDEX IF NOT EXISTS idx_voice_profiles_provider ON voice_profiles(provider);
"#;

/// Migration v14: Session notes
const MIGRATION_V14: &str = r#"
CREATE TABLE IF NOT EXISTS session_notes (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    campaign_id TEXT NOT NULL,
    content TEXT NOT NULL,
    tags TEXT,
    entity_links TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (campaign_id) REFERENCES campaigns(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_session_notes_session ON session_notes(session_id);
CREATE INDEX IF NOT EXISTS idx_session_notes_campaign ON session_notes(campaign_id);
CREATE INDEX IF NOT EXISTS idx_session_notes_created ON session_notes(created_at DESC);
"#;

/// Migration v15: Session events (timeline)
const MIGRATION_V15: &str = r#"
CREATE TABLE IF NOT EXISTS session_events (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    description TEXT,
    entities TEXT,
    metadata TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_session_events_session ON session_events(session_id);
CREATE INDEX IF NOT EXISTS idx_session_events_timestamp ON session_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_session_events_type ON session_events(event_type);
"#;

/// Migration v16: Combat states (distinct from existing combats table)
/// This table tracks detailed combat state for session continuity
const MIGRATION_V16: &str = r#"
CREATE TABLE IF NOT EXISTS combat_states (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    name TEXT,
    round INTEGER NOT NULL DEFAULT 1,
    current_turn INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    combatants TEXT NOT NULL,
    conditions TEXT,
    environment TEXT,
    notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    ended_at TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_combat_states_session ON combat_states(session_id);
CREATE INDEX IF NOT EXISTS idx_combat_states_active ON combat_states(is_active);
"#;

/// Migration v17: Enhanced search analytics with SQLite persistence
/// This replaces the basic search_records table from v2 with a more comprehensive schema
const MIGRATION_V17: &str = r#"
-- Enhanced search analytics table (per TASK-023 requirements)
CREATE TABLE IF NOT EXISTS search_analytics (
    id TEXT PRIMARY KEY,
    query TEXT NOT NULL,
    results_count INTEGER NOT NULL DEFAULT 0,
    selected_result_id TEXT,
    selected_result_index INTEGER,
    response_time_ms INTEGER NOT NULL DEFAULT 0,
    cache_hit INTEGER NOT NULL DEFAULT 0,
    search_type TEXT NOT NULL DEFAULT 'hybrid',
    source_filter TEXT,
    campaign_id TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_search_analytics_query ON search_analytics(query);
CREATE INDEX IF NOT EXISTS idx_search_analytics_created ON search_analytics(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_search_analytics_cache_hit ON search_analytics(cache_hit);
CREATE INDEX IF NOT EXISTS idx_search_analytics_results ON search_analytics(results_count);

-- Search result selections table (tracks which results users click)
CREATE TABLE IF NOT EXISTS search_selections (
    id TEXT PRIMARY KEY,
    search_id TEXT NOT NULL,
    query TEXT NOT NULL,
    result_index INTEGER NOT NULL,
    source TEXT NOT NULL,
    was_helpful INTEGER,
    selection_delay_ms INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (search_id) REFERENCES search_analytics(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_search_selections_search ON search_selections(search_id);
CREATE INDEX IF NOT EXISTS idx_search_selections_query ON search_selections(query);
CREATE INDEX IF NOT EXISTS idx_search_selections_created ON search_selections(created_at DESC);

-- Aggregated query statistics (updated periodically for fast retrieval)
CREATE TABLE IF NOT EXISTS search_query_stats (
    query_normalized TEXT PRIMARY KEY,
    total_count INTEGER NOT NULL DEFAULT 0,
    total_clicks INTEGER NOT NULL DEFAULT 0,
    avg_results REAL NOT NULL DEFAULT 0.0,
    avg_time_ms REAL NOT NULL DEFAULT 0.0,
    last_searched_at TEXT NOT NULL,
    click_positions_json TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_search_query_stats_count ON search_query_stats(total_count DESC);
CREATE INDEX IF NOT EXISTS idx_search_query_stats_last ON search_query_stats(last_searched_at DESC);
"#;

/// Migration v18: Global chat sessions for persistent LLM conversations
/// Allows chat history to persist across navigation and be linked to game sessions
const MIGRATION_V18: &str = r#"
-- Global chat sessions table
CREATE TABLE IF NOT EXISTS global_chat_sessions (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'active',
    linked_game_session_id TEXT,
    linked_campaign_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (linked_game_session_id) REFERENCES sessions(id) ON DELETE SET NULL,
    FOREIGN KEY (linked_campaign_id) REFERENCES campaigns(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_global_chat_sessions_status ON global_chat_sessions(status);
CREATE INDEX IF NOT EXISTS idx_global_chat_sessions_game_session ON global_chat_sessions(linked_game_session_id);
CREATE INDEX IF NOT EXISTS idx_global_chat_sessions_campaign ON global_chat_sessions(linked_campaign_id);
CREATE INDEX IF NOT EXISTS idx_global_chat_sessions_created ON global_chat_sessions(created_at DESC);

-- Chat messages table
CREATE TABLE IF NOT EXISTS chat_messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    tokens_input INTEGER,
    tokens_output INTEGER,
    is_streaming INTEGER NOT NULL DEFAULT 0,
    metadata TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES global_chat_sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_chat_messages_session ON chat_messages(session_id);
CREATE INDEX IF NOT EXISTS idx_chat_messages_created ON chat_messages(created_at);
CREATE INDEX IF NOT EXISTS idx_chat_messages_role ON chat_messages(role);
"#;

/// Migration v19: Enforce single active chat session via partial unique index
/// Prevents race condition in get_or_create_active_chat_session
const MIGRATION_V19: &str = r#"
-- Ensure only one chat session can be active at a time
CREATE UNIQUE INDEX IF NOT EXISTS idx_global_chat_sessions_single_active
ON global_chat_sessions(status) WHERE status = 'active';
"#;

/// Migration v20: TTRPG document storage for parsed PDF elements
/// Stores extracted stat blocks, spells, items, tables, and other TTRPG elements
const MIGRATION_V20: &str = r#"
-- TTRPG documents table for storing parsed game content elements
CREATE TABLE IF NOT EXISTS ttrpg_documents (
    id TEXT PRIMARY KEY,
    source_document_id TEXT NOT NULL,
    name TEXT NOT NULL,
    element_type TEXT NOT NULL,
    game_system TEXT NOT NULL,
    content TEXT NOT NULL,
    attributes_json TEXT NOT NULL DEFAULT '{}',
    challenge_rating REAL,
    level INTEGER,
    page_number INTEGER,
    confidence REAL NOT NULL DEFAULT 0.0,
    meilisearch_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (source_document_id) REFERENCES documents(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_ttrpg_documents_source ON ttrpg_documents(source_document_id);
CREATE INDEX IF NOT EXISTS idx_ttrpg_documents_type ON ttrpg_documents(element_type);
CREATE INDEX IF NOT EXISTS idx_ttrpg_documents_system ON ttrpg_documents(game_system);
CREATE INDEX IF NOT EXISTS idx_ttrpg_documents_name ON ttrpg_documents(name);
CREATE INDEX IF NOT EXISTS idx_ttrpg_documents_cr ON ttrpg_documents(challenge_rating);
CREATE INDEX IF NOT EXISTS idx_ttrpg_documents_level ON ttrpg_documents(level);

-- TTRPG document attributes for searchable metadata
CREATE TABLE IF NOT EXISTS ttrpg_document_attributes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id TEXT NOT NULL,
    attribute_type TEXT NOT NULL,
    attribute_value TEXT NOT NULL,
    FOREIGN KEY (document_id) REFERENCES ttrpg_documents(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_ttrpg_attrs_document ON ttrpg_document_attributes(document_id);
CREATE INDEX IF NOT EXISTS idx_ttrpg_attrs_type_value ON ttrpg_document_attributes(attribute_type, attribute_value);

-- TTRPG ingestion jobs for tracking parsing progress
CREATE TABLE IF NOT EXISTS ttrpg_ingestion_jobs (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    total_pages INTEGER NOT NULL DEFAULT 0,
    processed_pages INTEGER NOT NULL DEFAULT 0,
    elements_found INTEGER NOT NULL DEFAULT 0,
    errors_json TEXT,
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_ttrpg_jobs_document ON ttrpg_ingestion_jobs(document_id);
CREATE INDEX IF NOT EXISTS idx_ttrpg_jobs_status ON ttrpg_ingestion_jobs(status);
"#;
