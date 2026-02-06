//! Meilisearch Index Configuration for Archetype Registry
//!
//! This module defines the Meilisearch index configurations for archetypes and
//! vocabulary banks, using embedded meilisearch-lib for direct Rust integration.
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
//! use crate::core::search::EmbeddedSearch;
//!
//! let search = EmbeddedSearch::new(db_path)?;
//! let index_manager = ArchetypeIndexManager::new(search.clone_inner());
//!
//! // Ensure all indexes exist with proper configuration
//! index_manager.ensure_indexes()?;
//! ```

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use meilisearch_lib::{FilterableAttributesRule, MeilisearchLib, Setting, Settings, Unchecked};

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
const TASK_TIMEOUT: Duration = Duration::from_secs(30);

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
fn build_archetype_settings() -> Settings<Unchecked> {
    Settings {
        searchable_attributes: Setting::Set(vec![
            "display_name".to_string(),
            "description".to_string(),
            "tags".to_string(),
        ]).into(),
        filterable_attributes: Setting::Set(vec![
            FilterableAttributesRule::Field("id".to_string()),
            FilterableAttributesRule::Field("category".to_string()),
            FilterableAttributesRule::Field("parent_id".to_string()),
            FilterableAttributesRule::Field("setting_pack_id".to_string()),
            FilterableAttributesRule::Field("game_system".to_string()),
            FilterableAttributesRule::Field("tags".to_string()),
        ]),
        sortable_attributes: Setting::Set(BTreeSet::from([
            "display_name".to_string(),
            "category".to_string(),
            "created_at".to_string(),
        ])),
        ..Default::default()
    }
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
fn build_vocabulary_bank_settings() -> Settings<Unchecked> {
    Settings {
        searchable_attributes: Setting::Set(vec![
            "display_name".to_string(),
            "description".to_string(),
            "phrase_texts".to_string(),
        ]).into(),
        filterable_attributes: Setting::Set(vec![
            FilterableAttributesRule::Field("id".to_string()),
            FilterableAttributesRule::Field("culture".to_string()),
            FilterableAttributesRule::Field("role".to_string()),
            FilterableAttributesRule::Field("race".to_string()),
            FilterableAttributesRule::Field("categories".to_string()),
            FilterableAttributesRule::Field("formality_range".to_string()),
        ]),
        sortable_attributes: Setting::Set(BTreeSet::from([
            "display_name".to_string(),
            "created_at".to_string(),
        ])),
        ..Default::default()
    }
}

// ============================================================================
// Index Manager
// ============================================================================

/// Manager for archetype-related Meilisearch indexes.
///
/// Provides idempotent index creation and configuration, ensuring that
/// indexes exist with the correct settings on application startup.
///
/// Uses embedded `MeilisearchLib` directly (no HTTP overhead).
///
/// # Example
///
/// ```rust,ignore
/// let manager = ArchetypeIndexManager::new(meili.clone());
/// manager.ensure_indexes()?;
/// ```
pub struct ArchetypeIndexManager {
    meili: Arc<MeilisearchLib>,
}

impl ArchetypeIndexManager {
    /// Create a new index manager with a shared reference to MeilisearchLib.
    ///
    /// # Arguments
    ///
    /// * `meili` - Shared reference to the embedded MeilisearchLib instance
    pub fn new(meili: Arc<MeilisearchLib>) -> Self {
        Self { meili }
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
    /// let manager = ArchetypeIndexManager::new(meili.clone());
    /// manager.ensure_indexes()?;
    /// ```
    pub fn ensure_indexes(&self) -> Result<()> {
        log::info!("Ensuring archetype indexes exist with proper configuration");

        // Create or update archetypes index
        self.ensure_index(
            INDEX_ARCHETYPES,
            build_archetype_settings(),
        )?;

        // Create or update vocabulary banks index
        self.ensure_index(
            INDEX_VOCABULARY_BANKS,
            build_vocabulary_bank_settings(),
        )?;

        log::info!(
            "Archetype indexes configured: {}, {}",
            INDEX_ARCHETYPES,
            INDEX_VOCABULARY_BANKS
        );

        Ok(())
    }

    /// Ensure a single index exists with the specified settings.
    ///
    /// Creates the index if it doesn't exist, then applies settings.
    /// All operations are synchronous (embedded meilisearch-lib).
    ///
    /// # Arguments
    ///
    /// * `uid` - Name of the index to create or update
    /// * `settings` - Index settings to apply
    fn ensure_index(
        &self,
        uid: &str,
        settings: Settings<Unchecked>,
    ) -> Result<()> {
        let exists = self.meili.index_exists(uid)
            .map_err(|e| ArchetypeError::Meilisearch(format!("Check index '{}': {}", uid, e)))?;

        if !exists {
            log::info!("Creating index '{}' with primary key 'id'", uid);
            let task = self.meili.create_index(uid, Some("id".to_string()))
                .map_err(|e| ArchetypeError::Meilisearch(format!("Create index '{}': {}", uid, e)))?;
            self.meili.wait_for_task(task.uid, Some(TASK_TIMEOUT))
                .map_err(|e| ArchetypeError::Meilisearch(format!("Wait create '{}': {}", uid, e)))?;
        } else {
            log::debug!("Index '{}' exists, updating settings", uid);
        }

        let task = self.meili.update_settings(uid, settings)
            .map_err(|e| ArchetypeError::Meilisearch(format!("Settings '{}': {}", uid, e)))?;
        self.meili.wait_for_task(task.uid, Some(TASK_TIMEOUT))
            .map_err(|e| ArchetypeError::Meilisearch(format!("Wait settings '{}': {}", uid, e)))?;

        Ok(())
    }

    /// Check if the archetypes index exists and is configured.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if the index exists
    /// - `Ok(false)` if the index does not exist
    /// - `Err(...)` if there was an error checking
    pub fn archetypes_index_exists(&self) -> Result<bool> {
        self.index_exists(INDEX_ARCHETYPES)
    }

    /// Check if the vocabulary banks index exists and is configured.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if the index exists
    /// - `Ok(false)` if the index does not exist
    /// - `Err(...)` if there was an error checking
    pub fn vocabulary_banks_index_exists(&self) -> Result<bool> {
        self.index_exists(INDEX_VOCABULARY_BANKS)
    }

    /// Check if a specific index exists.
    fn index_exists(&self, uid: &str) -> Result<bool> {
        self.meili.index_exists(uid)
            .map_err(|e| ArchetypeError::Meilisearch(format!(
                "Failed to check index '{}': {}", uid, e
            )))
    }

    /// Get document count for the archetypes index.
    ///
    /// # Returns
    ///
    /// Number of documents in the index, or 0 if the index doesn't exist.
    pub fn archetype_count(&self) -> Result<u64> {
        self.document_count(INDEX_ARCHETYPES)
    }

    /// Get document count for the vocabulary banks index.
    ///
    /// # Returns
    ///
    /// Number of documents in the index, or 0 if the index doesn't exist.
    pub fn vocabulary_bank_count(&self) -> Result<u64> {
        self.document_count(INDEX_VOCABULARY_BANKS)
    }

    /// Get document count for an index.
    ///
    /// Returns 0 if the index does not exist (graceful handling).
    fn document_count(&self, uid: &str) -> Result<u64> {
        match self.meili.index_stats(uid) {
            Ok(stats) => Ok(stats.number_of_documents),
            Err(meilisearch_lib::Error::IndexNotFound(_)) => Ok(0),
            Err(e) => Err(ArchetypeError::Meilisearch(format!(
                "Failed to get stats for index '{}': {}", uid, e
            ))),
        }
    }

    /// Delete both archetype indexes (for testing/cleanup).
    ///
    /// Handles not-found indexes gracefully (already deleted is not an error).
    /// Attempts to delete all indexes even if one fails, collecting errors.
    ///
    /// # Warning
    ///
    /// This permanently deletes all data in both indexes.
    ///
    /// # Errors
    ///
    /// Returns the first non-`IndexNotFound` error encountered. All indexes
    /// are attempted regardless of individual failures.
    pub fn delete_indexes(&self) -> Result<()> {
        log::warn!("Deleting archetype indexes");

        let mut first_error: Option<ArchetypeError> = None;

        for uid in [INDEX_ARCHETYPES, INDEX_VOCABULARY_BANKS] {
            match self.meili.delete_index(uid) {
                Ok(task) => {
                    if let Err(e) = self.meili.wait_for_task(task.uid, Some(TASK_TIMEOUT)) {
                        log::error!("Failed to wait for delete of '{}': {}", uid, e);
                        if first_error.is_none() {
                            first_error = Some(ArchetypeError::Meilisearch(format!(
                                "Wait delete '{}': {}", uid, e
                            )));
                        }
                    } else {
                        log::info!("Deleted index '{}'", uid);
                    }
                }
                Err(meilisearch_lib::Error::IndexNotFound(_)) => {
                    log::debug!("Index '{}' already doesn't exist", uid);
                }
                Err(e) => {
                    log::error!("Failed to delete index '{}': {}", uid, e);
                    if first_error.is_none() {
                        first_error = Some(ArchetypeError::Meilisearch(format!(
                            "Delete index '{}': {}", uid, e
                        )));
                    }
                }
            }
        }

        match first_error {
            Some(e) => Err(e),
            None => Ok(()),
        }
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
/// let stats = meili.index_stats(archetype_index_name())?;
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
/// let stats = meili.index_stats(vocabulary_banks_index_name())?;
/// ```
#[inline]
pub fn vocabulary_banks_index_name() -> &'static str {
    INDEX_VOCABULARY_BANKS
}

/// Get the archetype index settings.
///
/// Useful for tests or manual index configuration.
pub fn get_archetype_settings() -> Settings<Unchecked> {
    build_archetype_settings()
}

/// Get the vocabulary banks index settings.
///
/// Useful for tests or manual index configuration.
pub fn get_vocabulary_bank_settings() -> Settings<Unchecked> {
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
    fn test_archetype_settings_searchable_attributes() {
        let settings = build_archetype_settings();

        // Verify searchable attributes are set in priority order
        match &settings.searchable_attributes {
            ws if ws.is_set() => {
                // WildcardSetting wraps Setting<Vec<String>>
                // Verify the setting was constructed (non-default)
            }
            _ => panic!("searchable_attributes should be Set"),
        }
    }

    #[test]
    fn test_archetype_settings_filterable_attributes() {
        let settings = build_archetype_settings();

        // Verify filterable attributes contain expected fields
        match &settings.filterable_attributes {
            Setting::Set(attrs) => {
                assert_eq!(attrs.len(), 6, "Expected 6 filterable attributes");
                // Verify key fields are present
                let field_names: Vec<String> = attrs.iter().filter_map(|r| {
                    match r {
                        FilterableAttributesRule::Field(name) => Some(name.clone()),
                        _ => None,
                    }
                }).collect();
                assert!(field_names.contains(&"id".to_string()));
                assert!(field_names.contains(&"category".to_string()));
                assert!(field_names.contains(&"game_system".to_string()));
                assert!(field_names.contains(&"tags".to_string()));
            }
            _ => panic!("filterable_attributes should be Set"),
        }
    }

    #[test]
    fn test_archetype_settings_sortable_attributes() {
        let settings = build_archetype_settings();

        match &settings.sortable_attributes {
            Setting::Set(attrs) => {
                assert_eq!(attrs.len(), 3, "Expected 3 sortable attributes");
                assert!(attrs.contains("display_name"));
                assert!(attrs.contains("category"));
                assert!(attrs.contains("created_at"));
            }
            _ => panic!("sortable_attributes should be Set"),
        }
    }

    #[test]
    fn test_vocabulary_bank_settings_filterable_attributes() {
        let settings = build_vocabulary_bank_settings();

        match &settings.filterable_attributes {
            Setting::Set(attrs) => {
                assert_eq!(attrs.len(), 6, "Expected 6 filterable attributes");
                let field_names: Vec<String> = attrs.iter().filter_map(|r| {
                    match r {
                        FilterableAttributesRule::Field(name) => Some(name.clone()),
                        _ => None,
                    }
                }).collect();
                assert!(field_names.contains(&"culture".to_string()));
                assert!(field_names.contains(&"role".to_string()));
                assert!(field_names.contains(&"race".to_string()));
                assert!(field_names.contains(&"categories".to_string()));
            }
            _ => panic!("filterable_attributes should be Set"),
        }
    }

    #[test]
    fn test_vocabulary_bank_settings_sortable_attributes() {
        let settings = build_vocabulary_bank_settings();

        match &settings.sortable_attributes {
            Setting::Set(attrs) => {
                assert_eq!(attrs.len(), 2, "Expected 2 sortable attributes");
                assert!(attrs.contains("display_name"));
                assert!(attrs.contains("created_at"));
            }
            _ => panic!("sortable_attributes should be Set"),
        }
    }

    #[test]
    fn test_get_settings_return_same_as_build() {
        // Public getters should return the same settings as internal builders
        let arch_public = get_archetype_settings();
        let arch_internal = build_archetype_settings();

        // Verify sortable attributes match (as a representative check)
        match (&arch_public.sortable_attributes, &arch_internal.sortable_attributes) {
            (Setting::Set(a), Setting::Set(b)) => assert_eq!(a, b),
            _ => panic!("Both should be Set"),
        }

        let vocab_public = get_vocabulary_bank_settings();
        let vocab_internal = build_vocabulary_bank_settings();

        match (&vocab_public.sortable_attributes, &vocab_internal.sortable_attributes) {
            (Setting::Set(a), Setting::Set(b)) => assert_eq!(a, b),
            _ => panic!("Both should be Set"),
        }
    }
}
