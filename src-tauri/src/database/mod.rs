//! SQLite Database Module
//!
//! Provides structured data storage for campaigns, sessions, characters,
//! usage tracking, and application state.
//!
//! # Extension Trait Pattern
//!
//! Operations are organized into domain-specific traits (e.g., `CampaignOps`,
//! `NpcOps`) implemented on the `Database` struct. Import the trait to access
//! its methods:
//!
//! ```rust,ignore
//! use crate::database::{Database, CampaignOps};
//!
//! let campaign = db.get_campaign("some-id").await?;
//! ```

// Core modules
mod migrations;
mod models;
mod backup;

// Domain-specific operation modules
mod analytics;
mod campaigns;
mod characters;
mod chat;
mod combat;
mod documents;
mod locations;
mod npcs;
pub mod quick_reference;
mod relationships;
mod search_analytics;
mod sessions;
mod settings;
mod ttrpg;
mod voice_profiles;

// Re-export existing public items
pub use migrations::run_migrations;
pub use models::*;
pub use backup::{create_backup, restore_backup, list_backups, BackupInfo};

// Re-export operation traits for ergonomic imports
pub use analytics::UsageOps;
pub use campaigns::CampaignOps;
pub use characters::CharacterOps;
pub use chat::ChatOps;
pub use combat::CombatOps;
pub use documents::DocumentOps;
pub use locations::LocationOps;
pub use npcs::NpcOps;
pub use quick_reference::QuickReferenceOps;
pub use relationships::RelationshipOps;
pub use search_analytics::SearchAnalyticsOps;
pub use sessions::SessionOps;
pub use settings::SettingsOps;
pub use ttrpg::TtrpgOps;
pub use voice_profiles::VoiceProfileOps;

// Re-export analytics summary types (used by search_analytics)
pub use search_analytics::{SearchAnalyticsSummary, SearchCacheStats, PopularQueryRecord};

// Re-export TTRPG stats type
pub use ttrpg::TTRPGDocumentStats;

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions, SqliteConnectOptions};
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
}
