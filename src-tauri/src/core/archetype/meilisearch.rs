//! Meilisearch Index Configuration for Archetype Registry
//!
//! This module defines the Meilisearch index configurations for archetypes and
//! vocabulary banks, following the Meilisearch-First Persistence Strategy.
//!
//! # Indexes
//!
//! - `ttrpg_archetypes`: Character archetype definitions with personality affinities,
//!   NPC role mappings, vocabulary references, and naming culture weights.
//! - `ttrpg_npc_vocabulary_banks`: Phrase collections organized by culture, role,
//!   and race for NPC dialogue generation.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::core::archetype::meilisearch::ArchetypeIndexManager;
//! use crate::core::search::SearchClient;
//!
//! let search_client = SearchClient::new("http://localhost:7700", None);
//! let index_manager = ArchetypeIndexManager::new(search_client);
//!
//! // Ensure all indexes exist with proper configuration
//! index_manager.ensure_indexes().await?;
//! ```

use meilisearch_sdk::client::Client;
use meilisearch_sdk::settings::Settings;
use std::time::Duration;

use super::error::{ArchetypeError, Result};

// ============================================================================
// Index Constants
// ============================================================================

/// Index name for character archetypes.
///
/// Primary key: `id`
///
/// This index stores archetype definitions including:
/// - Personality trait affinities
/// - NPC role mappings
/// - Vocabulary bank references
/// - Naming culture weights
/// - Stat tendencies
pub const INDEX_ARCHETYPES: &str = "ttrpg_archetypes";

/// Index name for NPC vocabulary banks.
///
/// Primary key: `id`
///
/// This index stores phrase collections organized by:
/// - Culture (e.g., "dwarvish", "elvish")
/// - Role (e.g., "merchant", "guard")
/// - Race (e.g., "dwarf", "elf")
/// - Formality levels
pub const INDEX_VOCABULARY_BANKS: &str = "ttrpg_npc_vocabulary_banks";

/// Default timeout for index operations (30 seconds).
const TASK_TIMEOUT_SECS: u64 = 30;

/// Polling interval for task completion checks (100ms).
const TASK_POLL_INTERVAL_MS: u64 = 100;

// ============================================================================
// Index Settings Builders
// ============================================================================

/// Build the settings for the archetypes index.
///
/// # Searchable Attributes
///
/// - `display_name`: Human-readable archetype name
/// - `description`: Full archetype description
/// - `tags`: User-defined categorization tags
///
/// # Filterable Attributes
///
/// - `id`: Unique archetype identifier
/// - `category`: Archetype type (role, race, class, setting, custom)
/// - `parent_id`: Reference to parent archetype for inheritance
/// - `setting_pack_id`: Associated setting pack
/// - `game_system`: Game system this archetype is designed for
/// - `tags`: User-defined tags for filtering
///
/// # Sortable Attributes
///
/// - `display_name`: Alphabetical sorting
/// - `category`: Group by type
/// - `created_at`: Chronological sorting
fn build_archetype_settings() -> Settings {
    Settings::new()
        .with_searchable_attributes([
            "display_name",
            "description",
            "tags",
        ])
        .with_filterable_attributes([
            "id",
            "category",
            "parent_id",
            "setting_pack_id",
            "game_system",
            "tags",
        ])
        .with_sortable_attributes([
            "display_name",
            "category",
            "created_at",
        ])
}

/// Build the settings for the vocabulary banks index.
///
/// # Searchable Attributes
///
/// - `display_name`: Human-readable bank name
/// - `description`: Bank description
/// - `phrase_texts`: Flattened array of all phrase text for full-text search
///
/// # Filterable Attributes
///
/// - `id`: Unique bank identifier
/// - `culture`: Cultural context (e.g., "dwarvish", "common")
/// - `role`: NPC role context (e.g., "merchant", "guard")
/// - `race`: Race context (e.g., "dwarf", "elf")
/// - `categories`: Phrase categories included (e.g., "greetings", "threats")
/// - `formality_range`: Min/max formality levels
///
/// # Sortable Attributes
///
/// - `display_name`: Alphabetical sorting
/// - `created_at`: Chronological sorting
fn build_vocabulary_bank_settings() -> Settings {
    Settings::new()
        .with_searchable_attributes([
            "display_name",
            "description",
            "phrase_texts",
        ])
        .with_filterable_attributes([
            "id",
            "culture",
            "role",
            "race",
            "categories",
            "formality_range",
        ])
        .with_sortable_attributes([
            "display_name",
            "created_at",
        ])
}

// ============================================================================
// Index Manager
// ============================================================================

/// Manager for archetype-related Meilisearch indexes.
///
/// Provides idempotent index creation and configuration, ensuring that
/// indexes exist with the correct settings on application startup.
///
/// # Example
///
/// ```rust,ignore
/// let manager = ArchetypeIndexManager::new(&client);
/// manager.ensure_indexes().await?;
/// ```
pub struct ArchetypeIndexManager<'a> {
    client: &'a Client,
}

impl<'a> ArchetypeIndexManager<'a> {
    /// Create a new index manager with a reference to the Meilisearch client.
    ///
    /// # Arguments
    ///
    /// * `client` - Reference to an initialized Meilisearch client
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Ensure all archetype-related indexes exist with proper configuration.
    ///
    /// This method is idempotent - it will create indexes if they don't exist,
    /// or update settings if they already exist.
    ///
    /// # Indexes Created
    ///
    /// - `ttrpg_archetypes`: Character archetype definitions
    /// - `ttrpg_npc_vocabulary_banks`: NPC phrase collections
    ///
    /// # Errors
    ///
    /// Returns `ArchetypeError::Meilisearch` if index creation or configuration fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let manager = ArchetypeIndexManager::new(&client);
    /// manager.ensure_indexes().await?;
    /// ```
    pub async fn ensure_indexes(&self) -> Result<()> {
        log::info!("Ensuring archetype indexes exist with proper configuration");

        // Create or update archetypes index
        self.ensure_index(
            INDEX_ARCHETYPES,
            build_archetype_settings(),
        ).await?;

        // Create or update vocabulary banks index
        self.ensure_index(
            INDEX_VOCABULARY_BANKS,
            build_vocabulary_bank_settings(),
        ).await?;

        log::info!(
            "Archetype indexes configured: {}, {}",
            INDEX_ARCHETYPES,
            INDEX_VOCABULARY_BANKS
        );

        Ok(())
    }

    /// Ensure a single index exists with the specified settings.
    ///
    /// This method handles both creation of new indexes and updating
    /// settings on existing indexes.
    ///
    /// # Arguments
    ///
    /// * `index_name` - Name of the index to create or update
    /// * `settings` - Index settings to apply
    async fn ensure_index(
        &self,
        index_name: &str,
        settings: Settings,
    ) -> Result<()> {
        // Try to get existing index first
        let index_result = self.client.get_index(index_name).await;

        match index_result {
            Ok(index) => {
                // Index exists, update settings
                log::debug!("Index '{}' exists, updating settings", index_name);
                let task = index.set_settings(&settings).await?;
                task.wait_for_completion(
                    self.client,
                    Some(Duration::from_millis(TASK_POLL_INTERVAL_MS)),
                    Some(Duration::from_secs(TASK_TIMEOUT_SECS)),
                ).await?;
            }
            Err(meilisearch_sdk::errors::Error::Meilisearch(e))
                if e.error_code == meilisearch_sdk::errors::ErrorCode::IndexNotFound =>
            {
                // Index doesn't exist, create it
                log::info!("Creating index '{}' with primary key 'id'", index_name);
                let task = self.client
                    .create_index(index_name, Some("id"))
                    .await?;
                task.wait_for_completion(
                    self.client,
                    Some(Duration::from_millis(TASK_POLL_INTERVAL_MS)),
                    Some(Duration::from_secs(TASK_TIMEOUT_SECS)),
                ).await?;

                // Apply settings to new index
                let index = self.client.index(index_name);
                let task = index.set_settings(&settings).await?;
                task.wait_for_completion(
                    self.client,
                    Some(Duration::from_millis(TASK_POLL_INTERVAL_MS)),
                    Some(Duration::from_secs(TASK_TIMEOUT_SECS)),
                ).await?;
            }
            Err(e) => {
                return Err(ArchetypeError::Meilisearch(format!(
                    "Failed to access index '{}': {}",
                    index_name, e
                )));
            }
        }

        Ok(())
    }

    /// Check if the archetypes index exists and is configured.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if the index exists
    /// - `Ok(false)` if the index does not exist
    /// - `Err(...)` if there was an error checking
    pub async fn archetypes_index_exists(&self) -> Result<bool> {
        self.index_exists(INDEX_ARCHETYPES).await
    }

    /// Check if the vocabulary banks index exists and is configured.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if the index exists
    /// - `Ok(false)` if the index does not exist
    /// - `Err(...)` if there was an error checking
    pub async fn vocabulary_banks_index_exists(&self) -> Result<bool> {
        self.index_exists(INDEX_VOCABULARY_BANKS).await
    }

    /// Check if a specific index exists.
    async fn index_exists(&self, index_name: &str) -> Result<bool> {
        match self.client.get_index(index_name).await {
            Ok(_) => Ok(true),
            Err(meilisearch_sdk::errors::Error::Meilisearch(e))
                if e.error_code == meilisearch_sdk::errors::ErrorCode::IndexNotFound =>
            {
                Ok(false)
            }
            Err(e) => Err(ArchetypeError::Meilisearch(format!(
                "Failed to check index '{}': {}",
                index_name, e
            ))),
        }
    }

    /// Get document count for the archetypes index.
    ///
    /// # Returns
    ///
    /// Number of documents in the index, or 0 if the index doesn't exist.
    pub async fn archetype_count(&self) -> Result<u64> {
        self.document_count(INDEX_ARCHETYPES).await
    }

    /// Get document count for the vocabulary banks index.
    ///
    /// # Returns
    ///
    /// Number of documents in the index, or 0 if the index doesn't exist.
    pub async fn vocabulary_bank_count(&self) -> Result<u64> {
        self.document_count(INDEX_VOCABULARY_BANKS).await
    }

    /// Get document count for an index.
    async fn document_count(&self, index_name: &str) -> Result<u64> {
        match self.client.get_index(index_name).await {
            Ok(index) => {
                let stats = index.get_stats().await?;
                Ok(stats.number_of_documents as u64)
            }
            Err(meilisearch_sdk::errors::Error::Meilisearch(e))
                if e.error_code == meilisearch_sdk::errors::ErrorCode::IndexNotFound =>
            {
                Ok(0)
            }
            Err(e) => Err(ArchetypeError::Meilisearch(format!(
                "Failed to get stats for index '{}': {}",
                index_name, e
            ))),
        }
    }

    /// Delete both archetype indexes (for testing/cleanup).
    ///
    /// # Warning
    ///
    /// This permanently deletes all data in both indexes.
    pub async fn delete_indexes(&self) -> Result<()> {
        log::warn!("Deleting archetype indexes");

        for index_name in [INDEX_ARCHETYPES, INDEX_VOCABULARY_BANKS] {
            match self.client.delete_index(index_name).await {
                Ok(task) => {
                    task.wait_for_completion(
                        self.client,
                        Some(Duration::from_millis(TASK_POLL_INTERVAL_MS)),
                        Some(Duration::from_secs(TASK_TIMEOUT_SECS)),
                    ).await?;
                    log::info!("Deleted index '{}'", index_name);
                }
                Err(meilisearch_sdk::errors::Error::Meilisearch(e))
                    if e.error_code == meilisearch_sdk::errors::ErrorCode::IndexNotFound =>
                {
                    log::debug!("Index '{}' already doesn't exist", index_name);
                }
                Err(e) => {
                    return Err(ArchetypeError::Meilisearch(format!(
                        "Failed to delete index '{}': {}",
                        index_name, e
                    )));
                }
            }
        }

        Ok(())
    }
}

// ============================================================================
// Public Convenience Functions
// ============================================================================

/// Get the archetype index name.
///
/// Use this for direct index access when needed:
///
/// ```rust,ignore
/// let index = search_client.index(archetype_index_name());
/// ```
#[inline]
pub fn archetype_index_name() -> &'static str {
    INDEX_ARCHETYPES
}

/// Get the vocabulary banks index name.
///
/// Use this for direct index access when needed:
///
/// ```rust,ignore
/// let index = search_client.index(vocabulary_banks_index_name());
/// ```
#[inline]
pub fn vocabulary_banks_index_name() -> &'static str {
    INDEX_VOCABULARY_BANKS
}

/// Get the archetype index settings.
///
/// Useful for tests or manual index configuration.
pub fn get_archetype_settings() -> Settings {
    build_archetype_settings()
}

/// Get the vocabulary banks index settings.
///
/// Useful for tests or manual index configuration.
pub fn get_vocabulary_bank_settings() -> Settings {
    build_vocabulary_bank_settings()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_constants() {
        assert_eq!(INDEX_ARCHETYPES, "ttrpg_archetypes");
        assert_eq!(INDEX_VOCABULARY_BANKS, "ttrpg_npc_vocabulary_banks");
    }

    #[test]
    fn test_convenience_functions() {
        assert_eq!(archetype_index_name(), INDEX_ARCHETYPES);
        assert_eq!(vocabulary_banks_index_name(), INDEX_VOCABULARY_BANKS);
    }

    #[test]
    fn test_archetype_settings() {
        let settings = build_archetype_settings();
        // Settings are built without error
        // We can't easily inspect Settings internals, but we verify it builds
        let _ = settings;
    }

    #[test]
    fn test_vocabulary_bank_settings() {
        let settings = build_vocabulary_bank_settings();
        // Settings are built without error
        let _ = settings;
    }

    #[test]
    fn test_get_settings_functions() {
        // These should return valid Settings objects
        let arch_settings = get_archetype_settings();
        let vocab_settings = get_vocabulary_bank_settings();
        let _ = (arch_settings, vocab_settings);
    }
}
