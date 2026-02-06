//! Query Preprocessing Search Commands
//!
//! Tauri commands for search with query preprocessing (typo correction + synonym expansion).
//! Implements REQ-QP-003 from the query preprocessing spec.
//!
//! ## Features
//!
//! - **search_with_preprocessing**: Hybrid search with automatic typo correction and
//!   synonym expansion. Returns corrections for "Did you mean?" UI.
//! - **rebuild_dictionaries**: Admin command to regenerate corpus and bigram dictionaries
//!   from indexed content.
//!
//! ## Usage (Frontend)
//!
//! ```typescript
//! // Search with preprocessing
//! const result = await invoke('search_with_preprocessing', {
//!   query: 'firball damge',  // Contains typos
//!   embedding: await generateEmbedding('fireball damage'),  // Use corrected text
//!   options: { limit: 10, semanticRatio: 0.6 }
//! });
//!
//! // Show corrections to user
//! if (result.corrections.length > 0) {
//!   console.log(`Searched for: "${result.correctedQuery}" (corrected from "${result.originalQuery}")`);
//! }
//! ```

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::RwLock as AsyncRwLock;

use crate::commands::state::AppState;
use crate::core::preprocess::{Correction, DictionaryGenerator, QueryPipeline};
use crate::core::storage::{
    hybrid_search_with_preprocessing, HybridSearchConfig, SearchFilter, SearchResult,
    SurrealStorage,
};

// ============================================================================
// TYPES
// ============================================================================

/// Correction information for frontend display.
///
/// Used to show "Did you mean..." suggestions to users.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrectionInfo {
    /// Original word from user input
    pub original: String,
    /// Corrected word
    pub corrected: String,
    /// Edit distance between original and corrected
    pub edit_distance: usize,
}

impl From<&Correction> for CorrectionInfo {
    fn from(c: &Correction) -> Self {
        Self {
            original: c.original.clone(),
            corrected: c.corrected.clone(),
            edit_distance: c.edit_distance,
        }
    }
}

/// Search result with preprocessing metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultWithCorrections {
    /// Search results
    pub results: Vec<PreprocessedSearchHit>,
    /// Total estimated hits
    pub total_hits: usize,
    /// Original query as typed by user
    pub original_query: String,
    /// Query after typo correction
    pub corrected_query: String,
    /// Query after synonym expansion (for debugging)
    pub expanded_query: Option<String>,
    /// Individual corrections made
    pub corrections: Vec<CorrectionInfo>,
    /// Human-readable summary of corrections (e.g., "firball -> fireball, damge -> damage")
    pub corrections_summary: Option<String>,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Search type used
    pub search_type: String,
    /// Performance and diagnostic hints
    pub hints: Vec<String>,
}

/// Search hit with metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreprocessedSearchHit {
    /// Unique chunk identifier
    pub id: String,
    /// Chunk content text
    pub content: String,
    /// Relevance score (0.0 - 1.0 after normalization)
    pub score: f32,
    /// Source document slug
    pub source: String,
    /// Page number within source document
    pub page_number: Option<i32>,
    /// Section path within document structure
    pub section_path: Option<String>,
    /// Content type classification
    pub content_type: String,
    /// Highlighted content with match markers (fulltext only)
    pub highlights: Option<String>,
}

impl From<SearchResult> for PreprocessedSearchHit {
    fn from(r: SearchResult) -> Self {
        Self {
            id: r.id,
            content: r.content,
            score: r.score,
            source: r.source,
            page_number: r.page_number,
            section_path: r.section_path,
            content_type: r.content_type,
            highlights: r.highlights,
        }
    }
}

/// Search options for preprocessed search.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreprocessingSearchOptions {
    /// Maximum results to return (default: 10)
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Semantic ratio for hybrid search (0.0 = keyword only, 1.0 = semantic only)
    /// Default: 0.6 (60% semantic, 40% keyword)
    pub semantic_ratio: Option<f32>,
    /// Minimum score threshold (0.0 - 1.0)
    pub min_score: Option<f32>,
    /// Filter by content type (rules, fiction, session_notes, homebrew)
    pub content_type: Option<String>,
    /// Filter by library item slug
    pub library_item: Option<String>,
    /// Minimum page number
    pub page_min: Option<i32>,
    /// Maximum page number
    pub page_max: Option<i32>,
}

fn default_limit() -> usize {
    10
}

impl Default for PreprocessingSearchOptions {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            semantic_ratio: None,
            min_score: None,
            content_type: None,
            library_item: None,
            page_min: None,
            page_max: None,
        }
    }
}

/// Statistics from dictionary rebuild operation.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RebuildStats {
    /// Number of unique words in corpus dictionary
    pub word_count: usize,
    /// Number of bigrams in bigram dictionary
    pub bigram_count: usize,
    /// Total documents (chunks) processed
    pub documents_processed: usize,
    /// Time taken in milliseconds
    pub rebuild_time_ms: u64,
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

/// Get QueryPipeline from app state.
fn get_pipeline(state: &AppState) -> Result<Arc<AsyncRwLock<QueryPipeline>>, String> {
    state
        .query_pipeline
        .as_ref()
        .cloned()
        .ok_or_else(|| "Query preprocessing pipeline not initialized".to_string())
}

/// Build SearchFilter from options.
fn build_filter(opts: &PreprocessingSearchOptions) -> Option<SearchFilter> {
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

    if opts.page_min.is_some() || opts.page_max.is_some() {
        filter = filter.page_range(opts.page_min, opts.page_max);
        has_filter = true;
    }

    if has_filter {
        Some(filter)
    } else {
        None
    }
}

// ============================================================================
// SEARCH COMMANDS (REQ-QP-003)
// ============================================================================

/// Perform hybrid search with query preprocessing.
///
/// This command applies typo correction and synonym expansion before searching:
/// 1. Corrects typos: "firball" -> "fireball"
/// 2. Expands synonyms: "hp" -> "(hp OR hit points OR health)"
///
/// The corrected query is used for embedding generation (semantic search),
/// while the expanded query is used for BM25 full-text search.
///
/// # Arguments
///
/// * `query` - User's search query (may contain typos)
/// * `embedding` - Query embedding vector (768 dimensions, generated from corrected text)
/// * `options` - Search options (limit, filters, semantic ratio)
/// * `state` - Application state with SurrealDB and QueryPipeline
///
/// # Returns
///
/// Search results with correction metadata for "Did you mean?" UI.
///
/// # Example (Frontend)
///
/// ```typescript
/// // First, get the corrected query for embedding
/// const corrected = await invoke('preprocess_query', { query });
/// const embedding = await generateEmbedding(corrected.textForEmbedding);
///
/// // Then search with preprocessing
/// const result = await invoke('search_with_preprocessing', {
///   query: 'firball damge',
///   embedding,
///   options: { limit: 10, contentType: 'rules' }
/// });
///
/// if (result.corrections.length > 0) {
///   showDidYouMean(result.correctionsSummary);
/// }
/// ```
#[tauri::command]
pub async fn search_with_preprocessing(
    query: String,
    embedding: Vec<f32>,
    options: Option<PreprocessingSearchOptions>,
    state: State<'_, AppState>,
) -> Result<SearchResultWithCorrections, String> {
    let start = std::time::Instant::now();
    let opts = options.unwrap_or_default();

    log::info!(
        "[search_with_preprocessing] Query: '{}', embedding_dims: {}, limit: {}",
        query,
        embedding.len(),
        opts.limit
    );

    // Validate embedding dimensions
    if embedding.len() != 768 {
        return Err(format!(
            "Invalid embedding dimensions: expected 768, got {}",
            embedding.len()
        ));
    }

    // Get storage and pipeline
    let storage = get_storage(&state)?;
    let pipeline_lock = get_pipeline(&state)?;
    let db = storage.db();

    // Build search config
    let mut config = if let Some(ratio) = opts.semantic_ratio {
        HybridSearchConfig::from_semantic_ratio(ratio)
    } else {
        HybridSearchConfig::default()
    };
    config = config.with_limit(opts.limit);
    if let Some(min_score) = opts.min_score {
        config = config.with_min_score(min_score);
    }

    // Build filter
    let filter = build_filter(&opts);

    // Execute search with preprocessing (acquire read lock on pipeline)
    let pipeline = pipeline_lock.read().await;
    let result = hybrid_search_with_preprocessing(
        db,
        &*pipeline,
        &query,
        embedding,
        &config,
        filter.as_ref(),
    )
    .await
    .map_err(|e| format!("Search with preprocessing failed: {}", e))?;
    drop(pipeline); // Release read lock early

    let processing_time_ms = start.elapsed().as_millis() as u64;
    let total_hits = result.results.len();

    // Build hints
    let mut hints = Vec::new();
    if result.had_corrections() {
        hints.push(format!(
            "Corrected {} typo(s)",
            result.corrections.len()
        ));
    }
    if processing_time_ms > 300 {
        hints.push(format!(
            "Search took {}ms (target: <300ms)",
            processing_time_ms
        ));
    }

    // Build expanded query string for debugging
    let expanded_query = result
        .processed_query
        .expanded
        .to_surrealdb_fts_plain("content");

    // Extract data before consuming result.results
    let original_query = result.processed_query.original.clone();
    let corrected_query = result.processed_query.corrected.clone();
    let corrections: Vec<CorrectionInfo> = result.corrections.iter().map(CorrectionInfo::from).collect();
    let corrections_summary = result.corrections_summary();

    log::info!(
        "[search_with_preprocessing] Completed: {} results, {} corrections, {}ms",
        total_hits,
        corrections.len(),
        processing_time_ms
    );

    Ok(SearchResultWithCorrections {
        results: result.results.into_iter().map(PreprocessedSearchHit::from).collect(),
        total_hits,
        original_query,
        corrected_query,
        expanded_query: if expanded_query.is_empty() {
            None
        } else {
            Some(expanded_query)
        },
        corrections,
        corrections_summary,
        processing_time_ms,
        search_type: "hybrid_preprocessed".to_string(),
        hints,
    })
}

// ============================================================================
// ADMIN COMMANDS
// ============================================================================

/// Rebuild corpus and bigram dictionaries from indexed content.
///
/// This command should be called after bulk document ingestion to update the
/// typo correction dictionaries with domain-specific vocabulary.
///
/// **Note**: This is an admin operation that may take several seconds for large
/// corpora. Consider running after significant content changes rather than
/// after every document.
///
/// # Arguments
///
/// * `state` - Application state with SurrealDB and QueryPipeline
///
/// # Returns
///
/// Statistics about the rebuild operation.
///
/// # Example (Frontend)
///
/// ```typescript
/// // After ingesting many documents, rebuild dictionaries
/// const stats = await invoke('rebuild_dictionaries');
/// console.log(`Rebuilt dictionaries: ${stats.wordCount} words, ${stats.bigramCount} bigrams`);
/// ```
#[tauri::command]
pub async fn rebuild_dictionaries(
    state: State<'_, AppState>,
) -> Result<RebuildStats, String> {
    let start = std::time::Instant::now();

    log::info!("[rebuild_dictionaries] Starting dictionary rebuild");

    // Get storage
    let storage = get_storage(&state)?;
    let db = storage.db();

    // Get all chunk content
    let chunks: Vec<ChunkContent> = db
        .query("SELECT content FROM chunk")
        .await
        .map_err(|e| format!("Failed to query chunks: {}", e))?
        .take(0)
        .map_err(|e| format!("Failed to extract chunks: {}", e))?;

    let documents_processed = chunks.len();
    log::info!(
        "[rebuild_dictionaries] Processing {} chunks",
        documents_processed
    );

    // Get dictionary paths
    let corpus_path = crate::core::preprocess::get_corpus_dictionary_path()
        .ok_or_else(|| "Could not determine corpus dictionary path".to_string())?;
    let bigram_path = crate::core::preprocess::get_bigram_dictionary_path()
        .ok_or_else(|| "Could not determine bigram dictionary path".to_string())?;

    // Ensure user data directory exists
    crate::core::preprocess::ensure_user_data_dir()
        .map_err(|e| format!("Failed to create user data directory: {}", e))?;

    // Build dictionaries
    let generator = DictionaryGenerator::default();

    let content_iter = chunks.iter().map(|c| c.content.as_str());
    let word_count = generator
        .build_corpus_dictionary_from_iter(content_iter, &corpus_path)
        .map_err(|e| format!("Failed to build corpus dictionary: {}", e))?;

    let content_iter = chunks.iter().map(|c| c.content.as_str());
    let bigram_count = generator
        .build_bigram_dictionary_from_iter(content_iter, &bigram_path)
        .map_err(|e| format!("Failed to build bigram dictionary: {}", e))?;

    let rebuild_time_ms = start.elapsed().as_millis() as u64;

    log::info!(
        "[rebuild_dictionaries] Completed: {} words, {} bigrams in {}ms",
        word_count,
        bigram_count,
        rebuild_time_ms
    );

    // Reload typo dictionaries in the query pipeline
    // This uses the AsyncRwLock to get write access and reload the SymSpell engine
    if let Some(ref pipeline_lock) = state.query_pipeline {
        match pipeline_lock.write().await.reload_typo_dictionaries() {
            Ok(()) => {
                log::info!("[rebuild_dictionaries] TypoCorrector dictionaries reloaded successfully");
            }
            Err(e) => {
                log::warn!("[rebuild_dictionaries] Failed to reload TypoCorrector dictionaries: {}", e);
            }
        }
    } else {
        log::warn!(
            "[rebuild_dictionaries] Query pipeline not initialized, dictionaries not reloaded"
        );
    }

    Ok(RebuildStats {
        word_count,
        bigram_count,
        documents_processed,
        rebuild_time_ms,
    })
}

/// Helper struct for deserializing chunk content.
#[derive(Debug, Deserialize)]
struct ChunkContent {
    content: String,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_options_defaults() {
        let opts = PreprocessingSearchOptions::default();
        assert_eq!(opts.limit, 10);
        assert!(opts.semantic_ratio.is_none());
        assert!(opts.content_type.is_none());
    }

    #[test]
    fn test_correction_info_from() {
        let correction = Correction {
            original: "firball".to_string(),
            corrected: "fireball".to_string(),
            edit_distance: 1,
        };

        let info: CorrectionInfo = (&correction).into();
        assert_eq!(info.original, "firball");
        assert_eq!(info.corrected, "fireball");
        assert_eq!(info.edit_distance, 1);
    }

    #[test]
    fn test_build_filter_empty() {
        let opts = PreprocessingSearchOptions::default();
        assert!(build_filter(&opts).is_none());
    }

    #[test]
    fn test_build_filter_with_content_type() {
        let opts = PreprocessingSearchOptions {
            content_type: Some("rules".to_string()),
            ..Default::default()
        };
        let filter = build_filter(&opts);
        assert!(filter.is_some());
        let surql = filter.unwrap().to_surql();
        assert!(surql.is_some());
        assert!(surql.unwrap().contains("content_type"));
    }

    #[test]
    fn test_build_filter_combined() {
        let opts = PreprocessingSearchOptions {
            content_type: Some("rules".to_string()),
            library_item: Some("phb-2024".to_string()),
            page_min: Some(100),
            page_max: Some(200),
            ..Default::default()
        };
        let filter = build_filter(&opts);
        assert!(filter.is_some());
        let surql = filter.unwrap().to_surql().unwrap();
        assert!(surql.contains("content_type"));
        assert!(surql.contains("library_item"));
        assert!(surql.contains("page_number >= 100"));
        assert!(surql.contains("page_number <= 200"));
    }

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResultWithCorrections {
            results: vec![],
            total_hits: 0,
            original_query: "firball".to_string(),
            corrected_query: "fireball".to_string(),
            expanded_query: Some("content @@ 'fireball'".to_string()),
            corrections: vec![CorrectionInfo {
                original: "firball".to_string(),
                corrected: "fireball".to_string(),
                edit_distance: 1,
            }],
            corrections_summary: Some("firball -> fireball".to_string()),
            processing_time_ms: 42,
            search_type: "hybrid_preprocessed".to_string(),
            hints: vec!["Corrected 1 typo(s)".to_string()],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"originalQuery\":\"firball\""));
        assert!(json.contains("\"correctedQuery\":\"fireball\""));
        assert!(json.contains("\"correctionsSummary\""));
        assert!(json.contains("\"processingTimeMs\":42"));
    }

    #[test]
    fn test_rebuild_stats_serialization() {
        let stats = RebuildStats {
            word_count: 5000,
            bigram_count: 10000,
            documents_processed: 500,
            rebuild_time_ms: 1500,
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"wordCount\":5000"));
        assert!(json.contains("\"bigramCount\":10000"));
        assert!(json.contains("\"documentsProcessed\":500"));
        assert!(json.contains("\"rebuildTimeMs\":1500"));
    }

    #[test]
    fn test_preprocessed_search_hit_from() {
        let result = SearchResult {
            id: "chunk:abc".to_string(),
            content: "Test content".to_string(),
            score: 0.85,
            linear_score: None,
            source: "test-doc".to_string(),
            page_number: Some(42),
            section_path: Some("Chapter 1".to_string()),
            content_type: "rules".to_string(),
            highlights: Some("<mark>Test</mark> content".to_string()),
        };

        let hit: PreprocessedSearchHit = result.into();
        assert_eq!(hit.id, "chunk:abc");
        assert_eq!(hit.content, "Test content");
        assert_eq!(hit.score, 0.85);
        assert_eq!(hit.source, "test-doc");
        assert_eq!(hit.page_number, Some(42));
        assert_eq!(hit.content_type, "rules");
        assert!(hit.highlights.is_some());
    }
}
