//! Meilisearch Index Configurations for NPC Generation
//!
//! Defines and manages specialized Meilisearch indexes for:
//! - Vocabulary banks (phrases by culture, role, race, category)
//! - Name components (prefixes, roots, suffixes by culture and gender)
//! - Exclamation templates (interjections and exclamations by culture)
//!
//! These indexes enable fast, filtered search across NPC generation data.

use meilisearch_sdk::client::Client;
use meilisearch_sdk::indexes::Index;
use meilisearch_sdk::settings::Settings;
use serde::{Deserialize, Serialize};

// ============================================================================
// Index Names
// ============================================================================

/// Index for vocabulary banks (phrases, greetings, farewells, etc.)
pub const INDEX_VOCABULARY_BANKS: &str = "ttrpg_vocabulary_banks";

/// Index for name components (prefixes, roots, suffixes)
pub const INDEX_NAME_COMPONENTS: &str = "ttrpg_name_components";

/// Index for exclamation templates
pub const INDEX_EXCLAMATION_TEMPLATES: &str = "ttrpg_exclamation_templates";

/// Default task timeout for index operations (30 seconds)
const INDEX_TASK_TIMEOUT_SECS: u64 = 30;

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
// Index Configuration
// ============================================================================

/// Ensures all NPC-related indexes exist with proper settings.
///
/// This function is idempotent - it can be called multiple times safely.
/// Indexes that already exist will have their settings updated.
///
/// # Arguments
/// * `client` - Meilisearch client
///
/// # Returns
/// * `Ok(())` - All indexes created/verified successfully
/// * `Err(String)` - If index creation or configuration fails
///
/// # Example
/// ```ignore
/// use meilisearch_sdk::client::Client;
/// use crate::core::npc_gen::indexes::ensure_npc_indexes;
///
/// async fn setup() -> Result<(), String> {
///     let client = Client::new("http://localhost:7700", Some("api_key"));
///     ensure_npc_indexes(&client).await?;
///     Ok(())
/// }
/// ```
pub async fn ensure_npc_indexes(client: &Client) -> Result<(), String> {
    log::info!("Ensuring NPC generation indexes exist...");

    // Create vocabulary banks index
    ensure_vocabulary_bank_index(client).await?;

    // Create name components index
    ensure_name_components_index(client).await?;

    // Create exclamation templates index
    ensure_exclamation_templates_index(client).await?;

    log::info!("NPC generation indexes ready");
    Ok(())
}

/// Ensure the vocabulary banks index exists with proper configuration.
async fn ensure_vocabulary_bank_index(client: &Client) -> Result<Index, String> {
    let index = ensure_index(client, INDEX_VOCABULARY_BANKS, "id").await?;

    let settings = Settings::new()
        .with_searchable_attributes([
            "phrase",
            "category",
            "bank_id",
            "tags",
        ])
        .with_filterable_attributes([
            "culture",
            "role",
            "race",
            "category",
            "formality",
            "bank_id",
            "tags",
        ])
        .with_sortable_attributes([
            "frequency",
        ]);

    apply_settings(&index, &settings, client).await?;

    log::debug!("Configured index '{}'", INDEX_VOCABULARY_BANKS);
    Ok(index)
}

/// Ensure the name components index exists with proper configuration.
async fn ensure_name_components_index(client: &Client) -> Result<Index, String> {
    let index = ensure_index(client, INDEX_NAME_COMPONENTS, "id").await?;

    let settings = Settings::new()
        .with_searchable_attributes([
            "component",
            "meaning",
            "phonetic_tags",
        ])
        .with_filterable_attributes([
            "culture",
            "component_type",
            "gender",
            "phonetic_tags",
        ])
        .with_sortable_attributes([
            "frequency",
        ]);

    apply_settings(&index, &settings, client).await?;

    log::debug!("Configured index '{}'", INDEX_NAME_COMPONENTS);
    Ok(index)
}

/// Ensure the exclamation templates index exists with proper configuration.
async fn ensure_exclamation_templates_index(client: &Client) -> Result<Index, String> {
    let index = ensure_index(client, INDEX_EXCLAMATION_TEMPLATES, "id").await?;

    let settings = Settings::new()
        .with_searchable_attributes([
            "template",
            "emotion",
        ])
        .with_filterable_attributes([
            "culture",
            "intensity",
            "emotion",
            "religious",
        ])
        .with_sortable_attributes([
            "frequency",
        ]);

    apply_settings(&index, &settings, client).await?;

    log::debug!("Configured index '{}'", INDEX_EXCLAMATION_TEMPLATES);
    Ok(index)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Ensure an index exists, creating it if necessary.
async fn ensure_index(client: &Client, name: &str, primary_key: &str) -> Result<Index, String> {
    match client.get_index(name).await {
        Ok(idx) => Ok(idx),
        Err(meilisearch_sdk::errors::Error::Meilisearch(err))
            if err.error_code == meilisearch_sdk::errors::ErrorCode::IndexNotFound =>
        {
            // Index doesn't exist, create it
            log::info!("Index '{}' not found, creating...", name);
            let task = client
                .create_index(name, Some(primary_key))
                .await
                .map_err(|e| format!("Failed to create index '{}': {}", name, e))?;

            task.wait_for_completion(
                client,
                Some(std::time::Duration::from_millis(100)),
                Some(std::time::Duration::from_secs(INDEX_TASK_TIMEOUT_SECS)),
            )
            .await
            .map_err(|e| format!("Timeout waiting for index '{}' creation: {}", name, e))?;

            Ok(client.index(name))
        }
        Err(e) => {
            // Other errors (connectivity, auth, etc.) should be surfaced
            Err(format!("Failed to get index '{}': {}", name, e))
        }
    }
}

/// Apply settings to an index.
async fn apply_settings(
    index: &Index,
    settings: &Settings,
    client: &Client,
) -> Result<(), String> {
    let task = index
        .set_settings(settings)
        .await
        .map_err(|e| format!("Failed to set settings on '{}': {}", index.uid, e))?;

    task.wait_for_completion(
        client,
        Some(std::time::Duration::from_millis(100)),
        Some(std::time::Duration::from_secs(INDEX_TASK_TIMEOUT_SECS)),
    )
    .await
    .map_err(|e| format!("Timeout waiting for settings on '{}': {}", index.uid, e))?;

    Ok(())
}

// ============================================================================
// Index Statistics
// ============================================================================

/// Statistics about NPC generation indexes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct NpcIndexStats {
    /// Number of vocabulary phrases indexed
    pub vocabulary_phrase_count: u64,
    /// Number of name components indexed
    pub name_component_count: u64,
    /// Number of exclamation templates indexed
    pub exclamation_template_count: u64,
    /// List of cultures with indexed data
    pub indexed_cultures: Vec<String>,
}


/// Get statistics about NPC generation indexes.
///
/// # Arguments
/// * `client` - Meilisearch client
///
/// # Returns
/// * `Ok(NpcIndexStats)` - Statistics about indexed data
/// * `Err(String)` - If statistics cannot be retrieved
pub async fn get_npc_index_stats(client: &Client) -> Result<NpcIndexStats, String> {
    let mut stats = NpcIndexStats::default();

    // Get vocabulary phrase count
    match client.get_index(INDEX_VOCABULARY_BANKS).await {
        Ok(index) => match index.get_stats().await {
            Ok(index_stats) => {
                stats.vocabulary_phrase_count = index_stats.number_of_documents as u64;
            }
            Err(e) => {
                log::warn!(
                    "Failed to get stats for index '{}': {}",
                    INDEX_VOCABULARY_BANKS,
                    e
                );
            }
        },
        Err(e) => {
            // Log but don't fail - index may not exist yet
            log::debug!(
                "Index '{}' not found or inaccessible: {}",
                INDEX_VOCABULARY_BANKS,
                e
            );
        }
    }

    // Get name component count
    match client.get_index(INDEX_NAME_COMPONENTS).await {
        Ok(index) => match index.get_stats().await {
            Ok(index_stats) => {
                stats.name_component_count = index_stats.number_of_documents as u64;
            }
            Err(e) => {
                log::warn!(
                    "Failed to get stats for index '{}': {}",
                    INDEX_NAME_COMPONENTS,
                    e
                );
            }
        },
        Err(e) => {
            log::debug!(
                "Index '{}' not found or inaccessible: {}",
                INDEX_NAME_COMPONENTS,
                e
            );
        }
    }

    // Get exclamation template count
    match client.get_index(INDEX_EXCLAMATION_TEMPLATES).await {
        Ok(index) => match index.get_stats().await {
            Ok(index_stats) => {
                stats.exclamation_template_count = index_stats.number_of_documents as u64;
            }
            Err(e) => {
                log::warn!(
                    "Failed to get stats for index '{}': {}",
                    INDEX_EXCLAMATION_TEMPLATES,
                    e
                );
            }
        },
        Err(e) => {
            log::debug!(
                "Index '{}' not found or inaccessible: {}",
                INDEX_EXCLAMATION_TEMPLATES,
                e
            );
        }
    }

    // TODO: Retrieve unique cultures from indexes using faceted search

    Ok(stats)
}

/// Clear all NPC generation indexes (for testing/reset).
///
/// # Warning
/// This will delete all indexed NPC generation data!
pub async fn clear_npc_indexes(client: &Client) -> Result<(), String> {
    log::warn!("Clearing all NPC generation indexes!");

    for index_name in [
        INDEX_VOCABULARY_BANKS,
        INDEX_NAME_COMPONENTS,
        INDEX_EXCLAMATION_TEMPLATES,
    ] {
        if let Ok(index) = client.get_index(index_name).await {
            let task = index
                .delete_all_documents()
                .await
                .map_err(|e| format!("Failed to clear '{}': {}", index_name, e))?;

            task.wait_for_completion(
                client,
                Some(std::time::Duration::from_millis(100)),
                Some(std::time::Duration::from_secs(INDEX_TASK_TIMEOUT_SECS)),
            )
            .await
            .map_err(|e| format!("Timeout clearing '{}': {}", index_name, e))?;

            log::info!("Cleared index '{}'", index_name);
        }
    }

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
