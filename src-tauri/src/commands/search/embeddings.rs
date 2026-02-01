//! Embeddings Configuration Commands
//!
//! Commands for configuring vector embeddings for semantic search.

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
pub async fn get_vector_store_status(state: State<'_, AppState>) -> Result<String, String> {
    if state.search_client.health_check().await {
        Ok("Meilisearch Ready".to_string())
    } else {
        Ok("Meilisearch Unhealthy".to_string())
    }
}

/// Configure Meilisearch embedder for semantic/vector search
#[tauri::command]
pub async fn configure_meilisearch_embedder(
    index_name: String,
    config: EmbedderConfigRequest,
    state: State<'_, AppState>,
) -> Result<String, String> {
    use crate::core::search::EmbedderConfig;

    let embedder_config = match config.provider.as_str() {
        "openAi" | "openai" => {
            let api_key = config.api_key.ok_or("OpenAI API key required")?;
            EmbedderConfig::OpenAI {
                api_key,
                model: config.model,
                dimensions: config.dimensions,
            }
        }
        "ollama" => {
            let url = config.url.unwrap_or_else(|| "http://localhost:11434".to_string());
            let model = config.model.unwrap_or_else(|| "nomic-embed-text".to_string());
            EmbedderConfig::Ollama { url, model }
        }
        "huggingFace" | "huggingface" => {
            let model = config.model.unwrap_or_else(|| "BAAI/bge-base-en-v1.5".to_string());
            EmbedderConfig::HuggingFace { model }
        }
        other => return Err(format!("Unknown provider: {}. Use 'openAi', 'ollama', or 'huggingFace'", other)),
    };

    state.search_client
        .configure_embedder(&index_name, &config.name, &embedder_config)
        .await
        .map_err(|e| format!("Failed to configure embedder: {}", e))?;

    Ok(format!("Configured embedder '{}' for index '{}'", config.name, index_name))
}

/// Setup Ollama embeddings on all content indexes using REST embedder
///
/// This configures Meilisearch to use Ollama for AI-powered semantic search.
/// The embedder is configured as a REST source for maximum compatibility.
#[tauri::command]
pub async fn setup_ollama_embeddings(
    host: String,
    model: String,
    state: State<'_, AppState>,
) -> Result<SetupEmbeddingsResult, String> {
    let configured = state.search_client
        .setup_ollama_embeddings(&host, &model)
        .await
        .map_err(|e| format!("Failed to setup embeddings: {}", e))?;

    let dimensions = crate::core::search::ollama_embedding_dimensions(&model);

    Ok(SetupEmbeddingsResult {
        indexes_configured: configured,
        model: model.clone(),
        dimensions,
        host: host.clone(),
    })
}

/// Setup Copilot embeddings on all content indexes via direct API access
///
/// This configures Meilisearch to use GitHub Copilot for AI-powered semantic search.
/// The embedder is configured as a REST source calling the Copilot API directly at
/// https://api.githubcopilot.com/embeddings with the OAuth token in the Authorization header.
///
/// **Note:** Copilot API tokens are short-lived (~30 minutes). If the token expires,
/// you will need to call this command again to refresh the configuration.
#[tauri::command]
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
    let api_key = state.copilot.get_valid_token().await
        .map_err(|e| format!("Failed to get Copilot API token: {}", e))?;

    log::info!("Retrieved valid Copilot API token for Meilisearch embeddings");

    // Configure Meilisearch to call the Copilot API directly
    let configured = state.search_client
        .setup_copilot_embeddings(&model, dims, &api_key)
        .await
        .map_err(|e| format!("Failed to setup Copilot embeddings: {}", e))?;

    log::info!(
        "Configured Copilot embeddings on {} indexes with model '{}' ({} dimensions)",
        configured.len(),
        model,
        dims
    );

    Ok(SetupCopilotEmbeddingsResult {
        indexes_configured: configured,
        model,
        dimensions: dims,
        api_url: "https://api.githubcopilot.com/embeddings".to_string(),
    })
}

/// Get embedder configuration for an index
#[tauri::command]
pub async fn get_embedder_status(
    index_name: String,
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>, String> {
    state.search_client
        .get_embedder_settings(&index_name)
        .await
        .map_err(|e| format!("Failed to get embedder status: {}", e))
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
#[tauri::command]
pub async fn setup_local_embeddings(
    model: String,
    state: State<'_, AppState>,
) -> Result<SetupEmbeddingsResult, String> {
    use crate::core::search::EmbedderConfig;

    // Get dimensions for the model
    let dimensions = huggingface_embedding_dimensions(&model);

    // Configure HuggingFace embedder on all content indexes
    let indexes = vec!["documents", "chat_history", "rules", "campaigns"];
    let mut configured = Vec::new();

    for index_name in indexes {
        let config = EmbedderConfig::HuggingFace {
            model: model.clone(),
        };

        match state.search_client
            .configure_embedder(index_name, "default", &config)
            .await
        {
            Ok(_) => {
                configured.push(index_name.to_string());
                log::info!("Configured HuggingFace embedder on index '{}'", index_name);
            }
            Err(e) => {
                log::warn!("Failed to configure embedder on '{}': {}", index_name, e);
            }
        }
    }

    Ok(SetupEmbeddingsResult {
        indexes_configured: configured,
        model: model.clone(),
        dimensions,
        host: "local".to_string(),
    })
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
