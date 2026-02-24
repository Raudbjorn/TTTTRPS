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

use crate::core::wilysearch::engine::Engine;
use crate::core::wilysearch::traits::{Documents, Indexes, SettingsApi, Search, System};
use crate::core::wilysearch::types::{AddDocumentsQuery, SearchRequest, Settings};
use crate::core::wilysearch::error::Error as WilySearchError;

use super::error::{ArchetypeError, Result};

/// Convert a meilisearch-lib error into an `ArchetypeError::Meilisearch` with context.
fn meili_err(context: &str, uid: &str, e: WilySearchError) -> ArchetypeError {
    ArchetypeError::Meilisearch(format!("{} '{}': {}", context, uid, e))
}

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

// ============================================================================
// Index Settings Builders
// ============================================================================

/// Build the settings for the archetypes index.
fn build_archetype_settings() -> Settings {
    Settings {
        searchable_attributes: Some(vec![
            "display_name".to_string(),
            "description".to_string(),
            "tags".to_string(),
        ]),
        filterable_attributes: Some(vec![
            "id".to_string(),
            "category".to_string(),
            "parent_id".to_string(),
            "setting_pack_id".to_string(),
            "game_system".to_string(),
            "tags".to_string(),
        ]),
        sortable_attributes: Some(vec![
            "display_name".to_string(),
            "category".to_string(),
            "created_at".to_string(),
        ]),
        ..Default::default()
    }
}

/// Build the settings for the vocabulary banks index.
fn build_vocabulary_bank_settings() -> Settings {
    Settings {
        searchable_attributes: Some(vec![
            "display_name".to_string(),
            "description".to_string(),
            "phrase_texts".to_string(),
        ]),
        filterable_attributes: Some(vec![
            "id".to_string(),
            "culture".to_string(),
            "role".to_string(),
            "race".to_string(),
            "categories".to_string(),
            "formality_range".to_string(),
        ]),
        sortable_attributes: Some(vec![
            "display_name".to_string(),
            "created_at".to_string(),
        ]),
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
/// Uses embedded `Meilisearch` directly (no HTTP overhead).
pub struct ArchetypeIndexManager {
    meili: Arc<Engine>,
}

impl ArchetypeIndexManager {
    /// Create a new index manager with a shared reference to Meilisearch.
    pub fn new(meili: Arc<Engine>) -> Self {
        Self { meili }
    }

    /// Ensure all archetype-related indexes exist with proper configuration.
    ///
    /// This method is idempotent - it will create indexes if they don't exist,
    /// or update settings if they already exist.
    pub fn ensure_indexes(&self) -> Result<()> {
        log::info!("Ensuring archetype indexes exist with proper configuration");

        self.ensure_index(INDEX_ARCHETYPES, build_archetype_settings())?;
        self.ensure_index(INDEX_VOCABULARY_BANKS, build_vocabulary_bank_settings())?;

        log::info!(
            "Archetype indexes configured: {}, {}",
            INDEX_ARCHETYPES,
            INDEX_VOCABULARY_BANKS
        );

        Ok(())
    }

    /// Ensure a single index exists with the specified settings.
    fn ensure_index(&self, uid: &str, settings: Settings) -> Result<()> {
        match self.meili.get_index(uid) {
            Ok(idx) => {
                log::debug!("Index '{}' exists, reapplying settings", uid);
                self.meili.update_settings(uid, &settings)
                    .map_err(|e| meili_err("Settings", uid, e))?;
                Ok(())
            },
            Err(e) if matches!(e, WilySearchError::IndexNotFound(_)) => {
                log::info!("Creating index '{}' with primary key 'id'", uid);
                let req = crate::core::wilysearch::types::CreateIndexRequest {
                    uid: uid.to_string(),
                    primary_key: Some("id".to_string()),
                };
                self.meili.create_index(&req)
                    .map_err(|e| meili_err("Create index", uid, e))?;
                self.meili.update_settings(uid, &settings)
                    .map_err(|e| meili_err("Settings", uid, e))?;
                Ok(())
            },
            Err(e) => Err(meili_err("Get index", uid, e))
        }
    }

    /// Check if the archetypes index exists.
    pub fn archetypes_index_exists(&self) -> Result<bool> {
        Ok(self.meili.get_index(INDEX_ARCHETYPES).is_ok())
    }

    /// Check if the vocabulary banks index exists.
    pub fn vocabulary_banks_index_exists(&self) -> Result<bool> {
        Ok(self.meili.get_index(INDEX_VOCABULARY_BANKS).is_ok())
    }

    /// Get document count for the archetypes index.
    pub fn archetype_count(&self) -> Result<u64> {
        Self::get_document_count(&self.meili, INDEX_ARCHETYPES)
    }

    /// Get document count for the vocabulary banks index.
    pub fn vocabulary_bank_count(&self) -> Result<u64> {
        Self::get_document_count(&self.meili, INDEX_VOCABULARY_BANKS)
    }

    /// Get document count for an index.
    fn get_document_count(meili: &Engine, uid: &str) -> Result<u64> {
        match meili.index_stats(uid) {
            Ok(stats) => Ok(stats.number_of_documents as u64),
            Err(e) if matches!(e, WilySearchError::IndexNotFound(_)) => Ok(0),
            Err(e) => Err(meili_err("Get stats", uid, e)),
        }
    }

    /// Delete both archetype indexes (for testing/cleanup).
    pub fn delete_indexes(&self) -> Result<()> {
        log::warn!("Deleting archetype indexes");

        let mut first_error: Option<ArchetypeError> = None;
        let mut set_error = |err: ArchetypeError| {
            if first_error.is_none() {
                first_error = Some(err);
            }
        };

        for uid in [INDEX_ARCHETYPES, INDEX_VOCABULARY_BANKS] {
            match self.meili.delete_index(uid) {
                Ok(_) => {
                    log::info!("Deleted index '{}'", uid);
                }
                Err(e) if matches!(e, WilySearchError::IndexNotFound(_)) => {
                    log::debug!("Index '{}' already doesn't exist", uid);
                }
                Err(e) => {
                    log::error!("Failed to delete index '{}': {}", uid, e);
                    set_error(meili_err("Delete index", uid, e));
                }
            }
        }

        if let Some(e) = first_error {
            Err(e)
        } else {
            Ok(())
        }
    }
}

// ============================================================================
// Public Convenience Functions
// ============================================================================

/// Get the archetype index name.
#[inline]
pub fn archetype_index_name() -> &'static str {
    INDEX_ARCHETYPES
}

/// Get the vocabulary banks index name.
#[inline]
pub fn vocabulary_banks_index_name() -> &'static str {
    INDEX_VOCABULARY_BANKS
}

/// Get the archetype index settings.
pub fn get_archetype_settings() -> Settings {
    build_archetype_settings()
}

/// Get the vocabulary banks index settings.
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
    fn test_archetype_settings_searchable_attributes() {
        let settings = build_archetype_settings();
        let searchable = settings.searchable_attributes.as_ref().expect("searchable should be set");
        assert_eq!(searchable.len(), 3);
    }

    #[test]
    fn test_archetype_settings_filterable_attributes() {
        let settings = build_archetype_settings();
        let filterable = settings.filterable_attributes.as_ref().expect("filterable should be set");
        assert_eq!(filterable.len(), 6, "Expected 6 filterable attributes");
        assert!(filterable.contains(&"id".to_string()));
        assert!(filterable.contains(&"category".to_string()));
        assert!(filterable.contains(&"game_system".to_string()));
        assert!(filterable.contains(&"tags".to_string()));
    }

    #[test]
    fn test_archetype_settings_sortable_attributes() {
        let settings = build_archetype_settings();
        let sortable = settings.sortable_attributes.as_ref().expect("sortable should be set");
        assert_eq!(sortable.len(), 3, "Expected 3 sortable attributes");
        assert!(sortable.contains(&"display_name".to_string()));
        assert!(sortable.contains(&"category".to_string()));
        assert!(sortable.contains(&"created_at".to_string()));
    }

    #[test]
    fn test_vocabulary_bank_settings_filterable_attributes() {
        let settings = build_vocabulary_bank_settings();
        let filterable = settings.filterable_attributes.as_ref().expect("filterable should be set");
        assert_eq!(filterable.len(), 6, "Expected 6 filterable attributes");
        assert!(filterable.contains(&"culture".to_string()));
        assert!(filterable.contains(&"role".to_string()));
        assert!(filterable.contains(&"race".to_string()));
        assert!(filterable.contains(&"categories".to_string()));
    }

    #[test]
    fn test_vocabulary_bank_settings_sortable_attributes() {
        let settings = build_vocabulary_bank_settings();
        let sortable = settings.sortable_attributes.as_ref().expect("sortable should be set");
        assert_eq!(sortable.len(), 2, "Expected 2 sortable attributes");
        assert!(sortable.contains(&"display_name".to_string()));
        assert!(sortable.contains(&"created_at".to_string()));
    }

    #[test]
    fn test_get_settings_return_same_as_build() {
        let arch_public = get_archetype_settings();
        let arch_internal = build_archetype_settings();

        let a = arch_public.sortable_attributes.as_ref().unwrap();
        let b = arch_internal.sortable_attributes.as_ref().unwrap();
        assert_eq!(a, b);

        let a = arch_public.filterable_attributes.as_ref().unwrap();
        let b = arch_internal.filterable_attributes.as_ref().unwrap();
        assert_eq!(a.len(), b.len());

        let vocab_public = get_vocabulary_bank_settings();
        let vocab_internal = build_vocabulary_bank_settings();

        let a = vocab_public.sortable_attributes.as_ref().unwrap();
        let b = vocab_internal.sortable_attributes.as_ref().unwrap();
        assert_eq!(a, b);

        let a = vocab_public.filterable_attributes.as_ref().unwrap();
        let b = vocab_internal.filterable_attributes.as_ref().unwrap();
        assert_eq!(a.len(), b.len());
    }
}
