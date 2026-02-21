//! SurrealDB-backed RAG (Retrieval-Augmented Generation) commands.
//!
//! Implements Task 4.2.3 from the migration spec: Update RAG Tauri commands (FR-8.3).
//!
//! These commands provide RAG functionality using SurrealDB for context retrieval,
//! combining search results with LLM responses for TTRPG assistant queries.
//!
//! ## Architecture
//!
//! ```text
//! Frontend                     Backend                      SurrealDB / LLM
//! --------                     -------                      ---------------
//!    |                            |                              |
//!    |-- rag_query_surrealdb -->  |                              |
//!    |                            |-- hybrid_search() ---------->| SurrealDB
//!    |                            |<-- SearchResults ------------|
//!    |                            |                              |
//!    |                            |-- format_context() -------->-|
//!    |                            |                              |
//!    |                            |-- llm.chat() --------------->| LLM Router
//!    |<-- RagResponse ------------|<-- Response -----------------|
//! ```
//!
//! ## Streaming Support
//!
//! The streaming command (`rag_query_stream_surrealdb`) emits Tauri events:
//! - `rag-surreal-chunk-{stream_id}`: Content chunks as LLM generates response
//! - `rag-surreal-complete-{stream_id}`: Final event with source citations
//! - `rag-surreal-error-{stream_id}`: Error event if something fails

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{Emitter, State, Window};

use crate::commands::state::AppState;
use crate::core::llm::router::{ChatMessage, ChatRequest};
use crate::core::storage::{
    prepare_rag_context, retrieve_rag_context, RagConfig, RagContext, RagSource, SearchFilter,
    SurrealStorage,
};

// ============================================================================
// TYPES
// ============================================================================

/// RAG query options.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurrealRagOptions {
    /// Maximum chunks to include in context (default: 8)
    pub max_chunks: Option<usize>,
    /// Maximum context size in bytes (default: 4000)
    pub max_bytes: Option<usize>,
    /// Semantic ratio for search (0.0 = keyword only, 1.0 = semantic only)
    pub semantic_ratio: Option<f32>,
    /// Filter by content type
    pub content_type: Option<String>,
    /// Filter by library item slug
    pub library_item: Option<String>,
    /// Custom system prompt template (use {{context}} placeholder)
    pub system_template: Option<String>,
    /// Include source citations in response
    #[serde(default = "default_include_sources")]
    pub include_sources: bool,
}

fn default_include_sources() -> bool {
    true
}

impl Default for SurrealRagOptions {
    fn default() -> Self {
        Self {
            max_chunks: None,
            max_bytes: None,
            semantic_ratio: None,
            content_type: None,
            library_item: None,
            system_template: None,
            include_sources: default_include_sources(),
        }
    }
}

/// RAG query response.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurrealRagResponse {
    /// LLM-generated response content
    pub content: String,
    /// Source citations
    pub sources: Vec<SurrealRagSourcePayload>,
    /// Context bytes used
    pub context_used: usize,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// RAG source citation payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurrealRagSourcePayload {
    /// Chunk identifier
    pub id: String,
    /// Source document title/slug
    pub title: String,
    /// Page number within source
    pub page: Option<i32>,
    /// Relevance score
    pub relevance: f32,
}

impl From<RagSource> for SurrealRagSourcePayload {
    fn from(s: RagSource) -> Self {
        Self {
            id: s.id,
            title: s.title,
            page: s.page,
            relevance: s.relevance,
        }
    }
}

/// RAG streaming chunk payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurrealRagChunkPayload {
    /// Stream identifier
    pub stream_id: String,
    /// Content delta
    pub delta: String,
    /// Chunk index for ordering
    pub index: u32,
}

/// RAG streaming completion payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurrealRagCompletePayload {
    /// Stream identifier
    pub stream_id: String,
    /// Source citations
    pub sources: Vec<SurrealRagSourcePayload>,
    /// Total context bytes used
    pub context_used: usize,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Get SurrealDB storage from app state.
fn get_storage(state: &AppState) -> Result<Arc<SurrealStorage>, String> {
    state
        .surreal_storage
        .as_ref()
        .cloned()
        .ok_or_else(|| "SurrealDB storage not initialized".to_string())
}

/// Build RagConfig from options.
fn build_rag_config(opts: &SurrealRagOptions) -> RagConfig {
    let mut config = if let Some(ratio) = opts.semantic_ratio {
        RagConfig::with_semantic_ratio(ratio)
    } else {
        RagConfig::default()
    };

    if let Some(max_chunks) = opts.max_chunks {
        config = config.with_max_chunks(max_chunks);
    }

    if let Some(max_bytes) = opts.max_bytes {
        config = config.with_max_bytes(max_bytes);
    }

    config = config.with_sources(opts.include_sources);

    if let Some(ref template) = opts.system_template {
        config = config.with_template(template);
    }

    config
}

/// Build SearchFilter from options.
fn build_filter(opts: &SurrealRagOptions) -> Option<SearchFilter> {
    let mut filter = SearchFilter::new();
    let mut has_filter = false;

    if let Some(ref ct) = opts.content_type {
        filter = filter.content_type(ct);
        has_filter = true;
    }

    if let Some(ref li) = opts.library_item {
        filter = filter.library_item(li);
        has_filter = true;
    }

    if has_filter {
        Some(filter)
    } else {
        None
    }
}

// ============================================================================
// RAG QUERY COMMANDS (Task 4.2.3)
// ============================================================================

/// Execute a RAG query using SurrealDB for retrieval.
///
/// This command:
/// 1. Searches SurrealDB for relevant context using hybrid search
/// 2. Formats the context into a system prompt
/// 3. Calls the LLM router for response generation
/// 4. Returns the response with source citations
///
/// # Arguments
///
/// * `question` - The user's question
/// * `embedding` - Query embedding vector (768 dimensions)
/// * `options` - RAG configuration options
/// * `state` - Application state with storage and LLM router
///
/// # Returns
///
/// RAG response with generated content and source citations.
///
/// # Example (Frontend)
///
/// ```typescript
/// const embedding = await generateEmbedding(question);
/// const response = await invoke('rag_query_surrealdb', {
///   question: 'How does flanking work in D&D 5e?',
///   embedding: embedding,
///   options: {
///     maxChunks: 10,
///     semanticRatio: 0.7,
///     contentType: 'rules'
///   }
/// });
/// console.log(response.content);
/// console.log(`Sources: ${response.sources.map(s => s.title).join(', ')}`);
/// ```
#[tauri::command]
pub async fn rag_query_surrealdb(
    question: String,
    embedding: Vec<f32>,
    options: Option<SurrealRagOptions>,
    state: State<'_, AppState>,
) -> Result<SurrealRagResponse, String> {
    let start = std::time::Instant::now();
    let opts = options.unwrap_or_default();

    log::info!(
        "[rag_query_surrealdb] Question: '{}', embedding_dims: {}",
        question,
        embedding.len()
    );

    // Validate embedding dimensions
    if embedding.len() != 768 {
        return Err(format!(
            "Invalid embedding dimensions: expected 768, got {}",
            embedding.len()
        ));
    }

    // Get storage
    let storage = get_storage(&state)?;
    let db = storage.db();

    // Build config and filter
    let config = build_rag_config(&opts);
    let filter = build_filter(&opts);

    // Retrieve context
    let (system_prompt, sources) =
        retrieve_rag_context(db, &question, embedding, &config, filter.as_ref())
            .await
            .map_err(|e| format!("Context retrieval failed: {}", e))?;

    // Get context size (approximate from system prompt)
    let context_used = system_prompt.len();

    // Call LLM router for response
    let llm_router = state.llm_router.read().await;

    // Build chat request with system prompt and user question
    let request = ChatRequest::new(vec![ChatMessage::user(&question)])
        .with_system(&system_prompt);

    let response = llm_router
        .chat(request)
        .await
        .map_err(|e| format!("LLM error: {}", e))?;

    let response_content = response.content;

    let processing_time_ms = start.elapsed().as_millis() as u64;

    log::info!(
        "[rag_query_surrealdb] Completed: {} sources, {}ms",
        sources.len(),
        processing_time_ms
    );

    Ok(SurrealRagResponse {
        content: response_content,
        sources: sources.into_iter().map(SurrealRagSourcePayload::from).collect(),
        context_used,
        processing_time_ms,
    })
}

/// Execute a streaming RAG query using SurrealDB.
///
/// Similar to `rag_query_surrealdb` but streams the LLM response via Tauri events.
///
/// # Arguments
///
/// * `window` - Tauri window for event emission
/// * `question` - The user's question
/// * `embedding` - Query embedding vector (768 dimensions)
/// * `stream_id` - Unique identifier for this stream
/// * `options` - RAG configuration options
/// * `state` - Application state with storage and LLM router
///
/// # Events Emitted
///
/// - `rag-surreal-chunk-{stream_id}`: Content chunks
/// - `rag-surreal-complete-{stream_id}`: Final event with sources
/// - `rag-surreal-error-{stream_id}`: Error event
///
/// # Example (Frontend)
///
/// ```typescript
/// import { listen } from '@tauri-apps/api/event';
///
/// const streamId = crypto.randomUUID();
///
/// await listen(`rag-surreal-chunk-${streamId}`, (e) => {
///   console.log('Chunk:', e.payload.delta);
/// });
///
/// await listen(`rag-surreal-complete-${streamId}`, (e) => {
///   console.log('Complete! Sources:', e.payload.sources);
/// });
///
/// await invoke('rag_query_stream_surrealdb', {
///   question: 'Explain opportunity attacks',
///   embedding: queryEmbedding,
///   streamId: streamId,
///   options: { contentType: 'rules' }
/// });
/// ```
#[tauri::command]
pub async fn rag_query_stream_surrealdb(
    window: Window,
    question: String,
    embedding: Vec<f32>,
    stream_id: String,
    options: Option<SurrealRagOptions>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let opts = options.unwrap_or_default();

    log::info!(
        "[rag_query_stream_surrealdb] Starting stream '{}' for: '{}'",
        stream_id,
        question
    );

    // Validate embedding dimensions
    if embedding.len() != 768 {
        let _ = window.emit(
            &format!("rag-surreal-error-{}", stream_id),
            format!("Invalid embedding dimensions: expected 768, got {}", embedding.len()),
        );
        return Err("Invalid embedding dimensions".to_string());
    }

    // Get storage
    let storage = get_storage(&state)?;

    // Clone what we need for the spawned task
    let storage_clone = storage.clone();
    let llm_router = state.llm_router.read().await.clone();
    let stream_id_clone = stream_id.clone();
    let question_clone = question.clone();
    let config = build_rag_config(&opts);
    let filter = build_filter(&opts);

    // Spawn async task for streaming
    tokio::spawn(async move {
        let db = storage_clone.db();

        // Prepare context
        let context: RagContext = match prepare_rag_context(
            db,
            &question_clone,
            embedding,
            &config,
            filter.as_ref(),
        )
        .await
        {
            Ok(c) => c,
            Err(e) => {
                let _ = window.emit(
                    &format!("rag-surreal-error-{}", stream_id_clone),
                    format!("Context preparation failed: {}", e),
                );
                return;
            }
        };

        let sources: Vec<SurrealRagSourcePayload> = context
            .sources
            .iter()
            .map(|s| SurrealRagSourcePayload::from(s.clone()))
            .collect();
        let context_bytes = context.context_bytes;

        // Build chat request with system prompt
        let request = ChatRequest::new(vec![ChatMessage::user(&context.query)])
            .with_system(&context.system_prompt);

        // Stream LLM response
        let mut chunk_index: u32 = 0;

        match llm_router.stream_chat(request).await {
            Ok(mut receiver) => {
                while let Some(chunk_result) = receiver.recv().await {
                    match chunk_result {
                        Ok(chunk) => {
                            chunk_index += 1;
                            let payload = SurrealRagChunkPayload {
                                stream_id: stream_id_clone.clone(),
                                delta: chunk.content,
                                index: chunk_index,
                            };
                            let _ = window.emit(
                                &format!("rag-surreal-chunk-{}", stream_id_clone),
                                &payload,
                            );

                            if chunk.is_final {
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = window.emit(
                                &format!("rag-surreal-error-{}", stream_id_clone),
                                format!("Stream error: {}", e),
                            );
                            log::error!(
                                "[rag_query_stream_surrealdb] Stream '{}' chunk error: {}",
                                stream_id_clone,
                                e
                            );
                            return;
                        }
                    }
                }

                // Send completion event with sources
                let complete_payload = SurrealRagCompletePayload {
                    stream_id: stream_id_clone.clone(),
                    sources,
                    context_used: context_bytes,
                };
                let _ = window.emit(
                    &format!("rag-surreal-complete-{}", stream_id_clone),
                    &complete_payload,
                );
                log::info!(
                    "[rag_query_stream_surrealdb] Stream '{}' completed with {} chunks",
                    stream_id_clone,
                    chunk_index
                );
            }
            Err(e) => {
                let _ = window.emit(
                    &format!("rag-surreal-error-{}", stream_id_clone),
                    format!("LLM streaming error: {}", e),
                );
                log::error!(
                    "[rag_query_stream_surrealdb] Stream '{}' failed: {}",
                    stream_id_clone,
                    e
                );
            }
        }
    });

    Ok(())
}

/// Get RAG configuration presets for TTRPG content types.
///
/// Returns recommended RAG configurations for different content types.
///
/// # Example (Frontend)
///
/// ```typescript
/// const presets = await invoke('get_rag_presets_surrealdb');
/// // Returns: { rules: {...}, lore: {...}, sessionNotes: {...} }
/// ```
#[tauri::command]
pub async fn get_rag_presets_surrealdb() -> Result<SurrealRagPresets, String> {
    Ok(SurrealRagPresets {
        rules: RagPresetInfo {
            name: "Rules & Mechanics".to_string(),
            description: "Optimized for exact rules lookup. Favors keyword matching.".to_string(),
            semantic_ratio: 0.4,
            max_chunks: 10,
            max_bytes: 6000,
        },
        lore: RagPresetInfo {
            name: "Lore & Fiction".to_string(),
            description: "Optimized for narrative content. Favors semantic similarity.".to_string(),
            semantic_ratio: 0.7,
            max_chunks: 6,
            max_bytes: 4000,
        },
        session_notes: RagPresetInfo {
            name: "Session Notes".to_string(),
            description: "Balanced for mixed content. Good for campaign queries.".to_string(),
            semantic_ratio: 0.5,
            max_chunks: 12,
            max_bytes: 5000,
        },
    })
}

/// RAG presets response.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurrealRagPresets {
    pub rules: RagPresetInfo,
    pub lore: RagPresetInfo,
    pub session_notes: RagPresetInfo,
}

/// RAG preset configuration info.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagPresetInfo {
    pub name: String,
    pub description: String,
    pub semantic_ratio: f32,
    pub max_chunks: usize,
    pub max_bytes: usize,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rag_options_defaults() {
        let opts = SurrealRagOptions::default();
        assert!(opts.max_chunks.is_none());
        assert!(opts.semantic_ratio.is_none());
        assert!(opts.include_sources);
    }

    #[test]
    fn test_build_rag_config_defaults() {
        let opts = SurrealRagOptions::default();
        let config = build_rag_config(&opts);
        assert_eq!(config.max_context_chunks, 8);
        assert!(config.include_sources);
    }

    #[test]
    fn test_build_rag_config_custom() {
        let opts = SurrealRagOptions {
            max_chunks: Some(15),
            max_bytes: Some(8000),
            semantic_ratio: Some(0.8),
            include_sources: false,
            ..Default::default()
        };
        let config = build_rag_config(&opts);
        assert_eq!(config.max_context_chunks, 15);
        assert_eq!(config.max_context_bytes, 8000);
        assert!(!config.include_sources);
        // Semantic ratio affects search_config
        assert!((config.search_config.semantic_weight - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_build_filter_empty() {
        let opts = SurrealRagOptions::default();
        assert!(build_filter(&opts).is_none());
    }

    #[test]
    fn test_build_filter_with_content_type() {
        let opts = SurrealRagOptions {
            content_type: Some("rules".to_string()),
            ..Default::default()
        };
        let filter = build_filter(&opts);
        assert!(filter.is_some());
    }

    #[test]
    fn test_rag_source_conversion() {
        let source = RagSource {
            id: "chunk:abc".to_string(),
            title: "PHB 2024".to_string(),
            page: Some(251),
            relevance: 0.95,
        };
        let payload: SurrealRagSourcePayload = source.into();
        assert_eq!(payload.id, "chunk:abc");
        assert_eq!(payload.title, "PHB 2024");
        assert_eq!(payload.page, Some(251));
        assert_eq!(payload.relevance, 0.95);
    }

    #[test]
    fn test_rag_response_serialization() {
        let response = SurrealRagResponse {
            content: "Flanking gives advantage.".to_string(),
            sources: vec![SurrealRagSourcePayload {
                id: "chunk:1".to_string(),
                title: "PHB".to_string(),
                page: Some(251),
                relevance: 0.9,
            }],
            context_used: 1500,
            processing_time_ms: 250,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"content\":\"Flanking gives advantage.\""));
        assert!(json.contains("\"contextUsed\":1500"));
        assert!(json.contains("\"processingTimeMs\":250"));
    }

    #[test]
    fn test_rag_presets() {
        let presets = SurrealRagPresets {
            rules: RagPresetInfo {
                name: "Rules".to_string(),
                description: "For rules".to_string(),
                semantic_ratio: 0.4,
                max_chunks: 10,
                max_bytes: 6000,
            },
            lore: RagPresetInfo {
                name: "Lore".to_string(),
                description: "For lore".to_string(),
                semantic_ratio: 0.7,
                max_chunks: 6,
                max_bytes: 4000,
            },
            session_notes: RagPresetInfo {
                name: "Notes".to_string(),
                description: "For notes".to_string(),
                semantic_ratio: 0.5,
                max_chunks: 12,
                max_bytes: 5000,
            },
        };

        // Rules should favor keyword (lower semantic ratio)
        assert!(presets.rules.semantic_ratio < presets.lore.semantic_ratio);
    }
}
