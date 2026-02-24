//! Meilisearch Index Configurations for NPC Generation
//!
//! Defines and manages specialized Meilisearch indexes for:
//! - Vocabulary banks (phrases by culture, role, race, category)
//! - Name components (prefixes, roots, suffixes by culture and gender)
//! - Exclamation templates (interjections and exclamations by culture)
//!
//! These indexes enable fast, filtered search across NPC generation data.
//! Uses embedded meilisearch-lib (no HTTP server required).

use std::collections::BTreeSet;

use crate::core::wilysearch::engine::Engine;
use crate::core::wilysearch::traits::{Indexes, SettingsApi, System};
use crate::core::wilysearch::types::{CreateIndexRequest, Settings};
use serde::{Deserialize, Serialize};

// ============================================================================
// Error Type
// ============================================================================

/// Errors that can occur during NPC index operations.
///
/// Field is named `detail` (not `source`) to avoid thiserror's automatic
/// `#[source]` inference, since the inner value is a `String`, not an `Error`.
#[derive(Debug, thiserror::Error)]
pub enum NpcIndexError {
    #[error("Failed to check index '{index}': {detail}")]
    Check { index: String, detail: String },

    #[error("Failed to create index '{index}': {detail}")]
    Create { index: String, detail: String },

    #[error("Failed to update settings for '{index}': {detail}")]
    Settings { index: String, detail: String },

    #[error("Task wait failed for index '{index}': {detail}")]
    TaskFailed { index: String, detail: String },

    #[error("Failed to clear index '{index}': {detail}")]
    Clear { index: String, detail: String },
}

impl From<NpcIndexError> for String {
    fn from(e: NpcIndexError) -> Self {
        e.to_string()
    }
}

// ============================================================================
// Index Names
// ============================================================================

/// Index for vocabulary banks (phrases, greetings, farewells, etc.)
pub const INDEX_VOCABULARY_BANKS: &str = "ttrpg_vocabulary_banks";

/// Index for name components (prefixes, roots, suffixes)
pub const INDEX_NAME_COMPONENTS: &str = "ttrpg_name_components";

/// Index for exclamation templates
pub const INDEX_EXCLAMATION_TEMPLATES: &str = "ttrpg_exclamation_templates";

// ============================================================================
// Index Document Types
// ============================================================================

/// A vocabulary phrase stored in the search index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabularyPhraseDocument {
    /// Unique document ID (format: "{bank_id}_{category}_{index}")
    pub id: String,
    /// The phrase text
    pub phrase: String,
    /// Vocabulary bank this phrase belongs to
    pub bank_id: String,
    /// Category of the phrase (greeting, farewell, exclamation, etc.)
    pub category: String,
    /// Formality level (formal, casual, hostile)
    pub formality: String,
    /// Culture this phrase is associated with
    pub culture: Option<String>,
    /// NPC role this phrase is suited for
    pub role: Option<String>,
    /// Race this phrase is suited for
    pub race: Option<String>,
    /// Usage frequency weight (0.0-1.0)
    pub frequency: f32,
    /// Tags for additional categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

/// A name component stored in the search index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameComponentDocument {
    /// Unique document ID (format: "{culture}_{type}_{index}")
    pub id: String,
    /// The component text (e.g., "Aer", "iel", "dor")
    pub component: String,
    /// Culture this component belongs to
    pub culture: String,
    /// Type of component (prefix, root, suffix, title, epithet)
    pub component_type: String,
    /// Gender affinity (male, female, neutral, any)
    pub gender: String,
    /// Usage frequency weight (0.0-1.0)
    pub frequency: f32,
    /// Optional meaning or etymology
    pub meaning: Option<String>,
    /// Phonetic compatibility hints
    #[serde(default)]
    pub phonetic_tags: Vec<String>,
}

/// An exclamation template stored in the search index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExclamationTemplateDocument {
    /// Unique document ID
    pub id: String,
    /// The exclamation template (may contain {placeholders})
    pub template: String,
    /// Culture this exclamation is associated with
    pub culture: String,
    /// Intensity level (mild, moderate, strong)
    pub intensity: String,
    /// Emotional context (surprise, anger, joy, fear, etc.)
    pub emotion: String,
    /// Whether this includes religious/deity references
    pub religious: bool,
    /// Usage frequency weight (0.0-1.0)
    pub frequency: f32,
}

// ============================================================================
// Index Settings
// ============================================================================

/// Build settings for the vocabulary banks index.
fn vocabulary_settings() -> Settings {
    Settings {
        searchable_attributes: Some(vec![
            "phrase".to_string(),
            "category".to_string(),
            "bank_id".to_string(),
            "tags".to_string(),
        ]),
        filterable_attributes: Some(vec![
            "culture".to_string(),
            "role".to_string(),
            "race".to_string(),
            "category".to_string(),
            "formality".to_string(),
            "bank_id".to_string(),
            "tags".to_string(),
        ]),
        sortable_attributes: Some(vec!["frequency".to_string()]),
        ..Default::default()
    }
}

/// Build settings for the name components index.
fn name_components_settings() -> Settings {
    Settings {
        searchable_attributes: Some(vec![
            "component".to_string(),
            "meaning".to_string(),
            "phonetic_tags".to_string(),
        ]),
        filterable_attributes: Some(vec![
            "culture".to_string(),
            "component_type".to_string(),
            "gender".to_string(),
            "phonetic_tags".to_string(),
        ]),
        sortable_attributes: Some(vec!["frequency".to_string()]),
        ..Default::default()
    }
}

/// Build settings for the exclamation templates index.
fn exclamation_settings() -> Settings {
    Settings {
        searchable_attributes: Some(vec![
            "template".to_string(),
            "emotion".to_string(),
        ]),
        filterable_attributes: Some(vec![
            "culture".to_string(),
            "intensity".to_string(),
            "emotion".to_string(),
            "religious".to_string(),
        ]),
        sortable_attributes: Some(vec!["frequency".to_string()]),
        ..Default::default()
    }
}

// ============================================================================
// Index Management
// ============================================================================

/// Create an index if it doesn't exist, then apply settings.
///
/// This is idempotent: calling it multiple times is safe. If the index
/// already exists, only settings are updated.
fn ensure_single_index(
    meili: &Engine,
    uid: &str,
    settings: Settings,
) -> Result<(), NpcIndexError> {
    match meili.get_index(uid) {
        Ok(_) => {
            meili.update_settings(uid, &settings).map_err(|e| NpcIndexError::Settings {
                index: uid.to_string(),
                detail: e.to_string(),
            })?;
        }
        Err(crate::core::wilysearch::error::Error::IndexNotFound(_)) => {
            log::info!("Index '{}' not found, creating...", uid);
            let req = CreateIndexRequest {
                uid: uid.to_string(),
                primary_key: Some("id".to_string()),
            };
            meili.create_index(&req).map_err(|e| NpcIndexError::Create {
                index: uid.to_string(),
                detail: e.to_string(),
            })?;
            meili.update_settings(uid, &settings).map_err(|e| NpcIndexError::Settings {
                index: uid.to_string(),
                detail: e.to_string(),
            })?;
        }
        Err(e) => return Err(NpcIndexError::Check {
            index: uid.to_string(),
            detail: e.to_string(),
        }),
    }

    log::debug!("Configured index '{}'", uid);
    Ok(())
}

/// Ensures all NPC-related indexes exist with proper settings.
///
/// This function is idempotent - it can be called multiple times safely.
/// Indexes that already exist will have their settings updated.
pub fn ensure_npc_indexes(meili: &Engine) -> Result<(), NpcIndexError> {
    log::info!("Ensuring NPC generation indexes exist...");

    ensure_single_index(meili, INDEX_VOCABULARY_BANKS, vocabulary_settings())?;
    ensure_single_index(meili, INDEX_NAME_COMPONENTS, name_components_settings())?;
    ensure_single_index(meili, INDEX_EXCLAMATION_TEMPLATES, exclamation_settings())?;

    log::info!("NPC generation indexes ready");
    Ok(())
}

// ============================================================================
// Index Statistics
// ============================================================================

/// Statistics about NPC generation indexes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NpcIndexStats {
    /// Number of vocabulary phrases indexed
    pub vocabulary_phrase_count: u64,
    /// Number of name components indexed
    pub name_component_count: u64,
    /// Number of exclamation templates indexed
    pub exclamation_template_count: u64,
    /// List of cultures with indexed data.
    // TODO: Populate via faceted search once culture facets are configured.
    pub indexed_cultures: Vec<String>,
}

/// Get statistics about NPC generation indexes.
pub fn get_npc_index_stats(meili: &Engine) -> Result<NpcIndexStats, NpcIndexError> {
    let mut stats = NpcIndexStats::default();

    match meili.index_stats(INDEX_VOCABULARY_BANKS) {
        Ok(index_stats) => {
            stats.vocabulary_phrase_count = index_stats.number_of_documents as u64;
        }
        Err(e) => {
            log::debug!(
                "Index '{}' not found or inaccessible: {}",
                INDEX_VOCABULARY_BANKS,
                e
            );
        }
    }

    match meili.index_stats(INDEX_NAME_COMPONENTS) {
        Ok(index_stats) => {
            stats.name_component_count = index_stats.number_of_documents as u64;
        }
        Err(e) => {
            log::debug!(
                "Index '{}' not found or inaccessible: {}",
                INDEX_NAME_COMPONENTS,
                e
            );
        }
    }

    match meili.index_stats(INDEX_EXCLAMATION_TEMPLATES) {
        Ok(index_stats) => {
            stats.exclamation_template_count = index_stats.number_of_documents as u64;
        }
        Err(e) => {
            log::debug!(
                "Index '{}' not found or inaccessible: {}",
                INDEX_EXCLAMATION_TEMPLATES,
                e
            );
        }
    }

    Ok(stats)
}

/// Clear all NPC generation indexes by deleting and recreating them.
///
/// # Warning
/// This will delete all indexed NPC generation data!
pub fn clear_npc_indexes(meili: &Engine) -> Result<(), NpcIndexError> {
    log::warn!("Clearing all NPC generation indexes!");

    for index_name in [
        INDEX_VOCABULARY_BANKS,
        INDEX_NAME_COMPONENTS,
        INDEX_EXCLAMATION_TEMPLATES,
    ] {
        match meili.get_index(index_name) {
            Ok(_) => {
                log::info!("Deleting index '{}'...", index_name);
                meili.delete_index(index_name).map_err(|e| NpcIndexError::Clear {
                    index: index_name.to_string(),
                    detail: e.to_string(),
                })?;
                log::info!("Deleted index '{}' successfully", index_name);
            }
            Err(_) => {
                log::debug!("Index '{}' doesn't exist, skipping", index_name);
            }
        }
    }

    // Recreate indexes with settings
    ensure_npc_indexes(meili)?;

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vocabulary_phrase_document_serialization() {
        let doc = VocabularyPhraseDocument {
            id: "tavern_greeting_0".to_string(),
            phrase: "Well met, traveler!".to_string(),
            bank_id: "tavern".to_string(),
            category: "greeting".to_string(),
            formality: "casual".to_string(),
            culture: Some("common".to_string()),
            role: Some("merchant".to_string()),
            race: None,
            frequency: 0.8,
            tags: vec!["friendly".to_string()],
        };

        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("Well met, traveler!"));
        assert!(json.contains("tavern_greeting_0"));

        let parsed: VocabularyPhraseDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.phrase, doc.phrase);
        assert_eq!(parsed.frequency, 0.8);
    }

    #[test]
    fn test_name_component_document_serialization() {
        let doc = NameComponentDocument {
            id: "elvish_prefix_0".to_string(),
            component: "Ael".to_string(),
            culture: "elvish".to_string(),
            component_type: "prefix".to_string(),
            gender: "neutral".to_string(),
            frequency: 0.6,
            meaning: Some("star".to_string()),
            phonetic_tags: vec!["vowel_start".to_string()],
        };

        let json = serde_json::to_string(&doc).unwrap();
        let parsed: NameComponentDocument = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.component, "Ael");
        assert_eq!(parsed.meaning, Some("star".to_string()));
    }

    #[test]
    fn test_exclamation_template_document_serialization() {
        let doc = ExclamationTemplateDocument {
            id: "dwarvish_surprise_0".to_string(),
            template: "By {deity}'s beard!".to_string(),
            culture: "dwarvish".to_string(),
            intensity: "moderate".to_string(),
            emotion: "surprise".to_string(),
            religious: true,
            frequency: 0.7,
        };

        let json = serde_json::to_string(&doc).unwrap();
        let parsed: ExclamationTemplateDocument = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.template, "By {deity}'s beard!");
        assert!(parsed.religious);
    }

    #[test]
    fn test_index_names() {
        assert_eq!(INDEX_VOCABULARY_BANKS, "ttrpg_vocabulary_banks");
        assert_eq!(INDEX_NAME_COMPONENTS, "ttrpg_name_components");
        assert_eq!(INDEX_EXCLAMATION_TEMPLATES, "ttrpg_exclamation_templates");
    }

    #[test]
    fn test_npc_index_stats_default() {
        let stats = NpcIndexStats::default();
        assert_eq!(stats.vocabulary_phrase_count, 0);
        assert_eq!(stats.name_component_count, 0);
        assert_eq!(stats.exclamation_template_count, 0);
        assert!(stats.indexed_cultures.is_empty());
    }
}
