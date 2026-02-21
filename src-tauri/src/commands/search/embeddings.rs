//! Embeddings Configuration Commands
//!
//! Commands for configuring vector embeddings for semantic search.
//!
//! TODO: Phase 3 Migration - These commands need to be updated to use EmbeddedSearch/MeilisearchLib.
//! The MeilisearchLib provides embedder configuration through the settings API:
//!   - meili.update_settings(uid, settings) where settings includes embedders
//! See: meili-dev/crates/meilisearch-lib/src/settings.rs

use tauri::State;

use crate::commands::AppState;
use super::types::{
    EmbedderConfigRequest, SetupEmbeddingsResult, SetupCopilotEmbeddingsResult,
    OllamaEmbeddingModel, LocalEmbeddingModel,
};

// ============================================================================
// Embeddings Configuration Commands
// ============================================================================

#[tauri::command]
#[allow(unused_variables)]
pub async fn get_vector_store_status(state: State<'_, AppState>) -> Result<String, String> {
    // TODO: Migrate to embedded MeilisearchLib
    // Old: state.search_client.health_check().await
    // New: Use state.embedded_search.inner().health() which always returns "available"
    let meili = state.embedded_search.inner();
    let health = meili.health();

    if health.status == "available" {
        Ok("Meilisearch Ready".to_string())
    } else {
        Ok("Meilisearch Unhealthy".to_string())
    }
}

/// Configure Meilisearch embedder for semantic/vector search
///
/// TODO: Phase 3 Migration - Update to use MeilisearchLib settings API
#[tauri::command]
#[allow(unused_variables)]
pub async fn configure_meilisearch_embedder(
    index_name: String,
    config: EmbedderConfigRequest,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // TODO: Migrate to embedded MeilisearchLib
    // The old SearchClient had configure_embedder() method.
    // The new MeilisearchLib uses:
    //   meili.update_settings(uid, settings) where settings.embedders contains the config
    //
    // Access via: state.embedded_search.inner()
    let _meili = state.embedded_search.inner();

    log::warn!(
        "configure_meilisearch_embedder() called but not yet migrated to embedded MeilisearchLib. Index: {}, Provider: {}",
        index_name, config.provider
    );

    // Return error for now - full migration in Phase 3 Task 6
    Err(format!(
        "Embedder configuration not yet available - migration in progress. Index: {}",
        index_name
    ))
}

/// Setup Ollama embeddings on all content indexes using REST embedder
///
/// This configures Meilisearch to use Ollama for AI-powered semantic search.
/// The embedder is configured as a REST source for maximum compatibility.
///
/// TODO: Phase 3 Migration - Update to use MeilisearchLib settings API
#[tauri::command]
#[allow(unused_variables)]
pub async fn setup_ollama_embeddings(
    host: String,
    model: String,
    state: State<'_, AppState>,
) -> Result<SetupEmbeddingsResult, String> {
    // TODO: Migrate to embedded MeilisearchLib
    // The old SearchClient had setup_ollama_embeddings() method.
    // The new MeilisearchLib uses:
    //   meili.update_settings(uid, settings) for each index
    //
    // Access via: state.embedded_search.inner()
    let _meili = state.embedded_search.inner();

    log::warn!(
        "setup_ollama_embeddings() called but not yet migrated to embedded MeilisearchLib. Host: {}, Model: {}",
        host, model
    );

    // Return error for now - full migration in Phase 3 Task 6
    Err(format!(
        "Ollama embeddings setup not yet available - migration in progress. Model: {}",
        model
    ))
}

/// Setup Copilot embeddings on all content indexes via direct API access
///
/// This configures Meilisearch to use GitHub Copilot for AI-powered semantic search.
/// The embedder is configured as a REST source calling the Copilot API directly at
/// https://api.githubcopilot.com/embeddings with the OAuth token in the Authorization header.
///
/// **Note:** Copilot API tokens are short-lived (~30 minutes). If the token expires,
/// you will need to call this command again to refresh the configuration.
///
/// TODO: Phase 3 Migration - Update to use MeilisearchLib settings API
#[tauri::command]
#[allow(unused_variables)]
pub async fn setup_copilot_embeddings(
    model: String,
    dimensions: Option<u32>,
    state: State<'_, AppState>,
) -> Result<SetupCopilotEmbeddingsResult, String> {
    let dims = dimensions.unwrap_or_else(|| {
        crate::core::search::copilot_embedding_dimensions(&model)
    });

    // First, ensure we're authenticated with Copilot
    let is_authenticated = state.copilot.is_authenticated().await
        .map_err(|e| format!("Failed to check Copilot auth: {}", e))?;

    if !is_authenticated {
        return Err("Copilot authentication required. Please login first.".to_string());
    }

    // Get a valid Copilot API token (refreshing if needed)
    let _api_key = state.copilot.get_valid_token().await
        .map_err(|e| format!("Failed to get Copilot API token: {}", e))?;

    log::info!("Retrieved valid Copilot API token for Meilisearch embeddings");

    // TODO: Migrate to embedded MeilisearchLib
    // The old SearchClient had setup_copilot_embeddings() method.
    // The new MeilisearchLib uses:
    //   meili.update_settings(uid, settings) where settings.embedders contains the Copilot REST config
    //
    // Access via: state.embedded_search.inner()
    let _meili = state.embedded_search.inner();

    log::warn!(
        "setup_copilot_embeddings() called but not yet migrated to embedded MeilisearchLib. Model: {}",
        model
    );

    // Return error for now - full migration in Phase 3 Task 6
    Err(format!(
        "Copilot embeddings setup not yet available - migration in progress. Model: {}",
        model
    ))
}

/// Get embedder configuration for an index
///
/// TODO: Phase 3 Migration - Update to use MeilisearchLib settings API
#[tauri::command]
#[allow(unused_variables)]
pub async fn get_embedder_status(
    index_name: String,
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>, String> {
    // TODO: Migrate to embedded MeilisearchLib
    // The old SearchClient had get_embedder_settings() method.
    // The new MeilisearchLib uses:
    //   meili.get_settings(uid) -> Settings which includes embedders
    //
    // Access via: state.embedded_search.inner()
    let _meili = state.embedded_search.inner();

    log::warn!(
        "get_embedder_status() called but not yet migrated to embedded MeilisearchLib. Index: {}",
        index_name
    );

    // Return None for now - full migration in Phase 3 Task 6
    Ok(None)
}

/// List available Ollama embedding models (filters for embedding-capable models)
#[tauri::command]
pub async fn list_ollama_embedding_models(host: String) -> Result<Vec<OllamaEmbeddingModel>, String> {
    // Get all models from Ollama
    let models = crate::core::llm::LLMClient::list_ollama_models(&host)
        .await
        .map_err(|e| e.to_string())?;

    // Known embedding model patterns
    let embedding_patterns = [
        "nomic-embed",
        "mxbai-embed",
        "all-minilm",
        "bge-",
        "snowflake-arctic-embed",
        "gte-",
        "e5-",
        "embed",
    ];

    let embedding_models: Vec<OllamaEmbeddingModel> = models
        .into_iter()
        .filter(|m| {
            let name_lower = m.name.to_lowercase();
            embedding_patterns.iter().any(|p| name_lower.contains(p))
        })
        .map(|m| {
            let dimensions = crate::core::search::ollama_embedding_dimensions(&m.name);
            OllamaEmbeddingModel {
                name: m.name,
                size: m.size,
                dimensions,
            }
        })
        .collect();

    // If no embedding models found, return common defaults that user should pull
    if embedding_models.is_empty() {
        return Ok(vec![
            OllamaEmbeddingModel {
                name: "nomic-embed-text".to_string(),
                size: "274 MB".to_string(),
                dimensions: 768,
            },
            OllamaEmbeddingModel {
                name: "mxbai-embed-large".to_string(),
                size: "669 MB".to_string(),
                dimensions: 1024,
            },
            OllamaEmbeddingModel {
                name: "all-minilm".to_string(),
                size: "46 MB".to_string(),
                dimensions: 384,
            },
        ]);
    }

    Ok(embedding_models)
}

/// List available local embedding models (HuggingFace/ONNX - no external service required)
///
/// These models run locally within Meilisearch using the HuggingFace embedder.
/// No GPU required - uses ONNX runtime for CPU inference.
#[tauri::command]
pub async fn list_local_embedding_models() -> Result<Vec<LocalEmbeddingModel>, String> {
    // Curated list of recommended HuggingFace embedding models
    // These are known to work well with Meilisearch and have reasonable performance
    Ok(vec![
        LocalEmbeddingModel {
            id: "BAAI/bge-base-en-v1.5".to_string(),
            name: "BGE Base (English)".to_string(),
            dimensions: 768,
            description: "Balanced performance and quality. Good for general use.".to_string(),
        },
        LocalEmbeddingModel {
            id: "BAAI/bge-small-en-v1.5".to_string(),
            name: "BGE Small (English)".to_string(),
            dimensions: 384,
            description: "Faster, smaller. Good for limited resources.".to_string(),
        },
        LocalEmbeddingModel {
            id: "BAAI/bge-large-en-v1.5".to_string(),
            name: "BGE Large (English)".to_string(),
            dimensions: 1024,
            description: "Highest quality. Slower, needs more memory.".to_string(),
        },
        LocalEmbeddingModel {
            id: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            name: "MiniLM-L6 (Multilingual)".to_string(),
            dimensions: 384,
            description: "Fast and small. Supports 100+ languages.".to_string(),
        },
        LocalEmbeddingModel {
            id: "sentence-transformers/all-mpnet-base-v2".to_string(),
            name: "MPNet Base".to_string(),
            dimensions: 768,
            description: "High quality general-purpose embeddings.".to_string(),
        },
        LocalEmbeddingModel {
            id: "thenlper/gte-base".to_string(),
            name: "GTE Base".to_string(),
            dimensions: 768,
            description: "Excellent retrieval performance.".to_string(),
        },
        LocalEmbeddingModel {
            id: "thenlper/gte-small".to_string(),
            name: "GTE Small".to_string(),
            dimensions: 384,
            description: "Compact with good retrieval quality.".to_string(),
        },
    ])
}

/// Setup local embeddings on all content indexes using HuggingFace embedder
///
/// This configures Meilisearch to use local ONNX models for AI-powered semantic search.
/// Models are downloaded and cached automatically by Meilisearch.
/// No external service (like Ollama) is required.
///
/// TODO: Phase 3 Migration - Update to use MeilisearchLib settings API
#[tauri::command]
#[allow(unused_variables)]
pub async fn setup_local_embeddings(
    model: String,
    state: State<'_, AppState>,
) -> Result<SetupEmbeddingsResult, String> {
    // Get dimensions for the model
    let _dimensions = huggingface_embedding_dimensions(&model);

    // TODO: Migrate to embedded MeilisearchLib
    // The old SearchClient had configure_embedder() method.
    // The new MeilisearchLib uses:
    //   meili.update_settings(uid, settings) for each index
    //
    // Access via: state.embedded_search.inner()
    let _meili = state.embedded_search.inner();

    log::warn!(
        "setup_local_embeddings() called but not yet migrated to embedded MeilisearchLib. Model: {}",
        model
    );

    // Return error for now - full migration in Phase 3 Task 6
    Err(format!(
        "Local embeddings setup not yet available - migration in progress. Model: {}",
        model
    ))
}

/// Get dimensions for HuggingFace embedding models
fn huggingface_embedding_dimensions(model: &str) -> u32 {
    match model.to_lowercase().as_str() {
        m if m.contains("bge-small") => 384,
        m if m.contains("bge-base") => 768,
        m if m.contains("bge-large") => 1024,
        m if m.contains("minilm") => 384,
        m if m.contains("mpnet") => 768,
        m if m.contains("gte-small") => 384,
        m if m.contains("gte-base") => 768,
        m if m.contains("gte-large") => 1024,
        m if m.contains("e5-small") => 384,
        m if m.contains("e5-base") => 768,
        m if m.contains("e5-large") => 1024,
        _ => 768, // Default assumption
    }
}
