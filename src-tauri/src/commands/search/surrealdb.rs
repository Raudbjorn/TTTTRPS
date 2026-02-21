//! SurrealDB-backed search commands.
//!
//! These commands provide search functionality using SurrealDB as the backend,
//! implementing Tasks 6.1.1, 6.1.2, and 6.1.3 from the migration spec.
//!
//! ## Migration Status
//!
//! These commands run alongside existing Meilisearch commands during the migration
//! period. Once migration is complete, the Meilisearch commands will be removed
//! and these will become the primary search implementation.
//!
//! ## Architecture
//!
//! The commands use the storage module (`core::storage`) which provides:
//! - `hybrid_search()` - Combined vector + BM25 search
//! - `vector_search()` - KNN search with HNSW index
//! - `fulltext_search()` - BM25 search with highlighting
//!
//! Embeddings must be generated externally (via LLM provider) before calling
//! vector or hybrid search.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::state::AppState;
use crate::core::storage::{
    fulltext_search, hybrid_search, vector_search, HybridSearchConfig, SearchFilter, SearchResult,
    SurrealStorage,
};

// ============================================================================
// TYPES (Task 6.1.1 - Compatible with existing frontend)
// ============================================================================

/// Search hit response (compatible with existing frontend).
///
/// Maps from `SearchResult` (storage layer) to frontend payload format.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurrealSearchHit {
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

impl From<SearchResult> for SurrealSearchHit {
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

/// Search response with metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurrealSearchResponse {
    /// Search results
    pub results: Vec<SurrealSearchHit>,
    /// Total estimated hits
    pub total_hits: usize,
    /// Original query string
    pub query: String,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Search type used
    pub search_type: String,
    /// Performance hints
    pub hints: Vec<String>,
}

/// Search options for SurrealDB search commands.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurrealSearchOptions {
    /// Maximum results to return (default: 10)
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Semantic ratio for hybrid search (0.0 = keyword only, 1.0 = semantic only)
    /// Default: 0.6 (60% semantic, 40% keyword)
    pub semantic_ratio: Option<f32>,
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

impl Default for SurrealSearchOptions {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            semantic_ratio: None,
            content_type: None,
            library_item: None,
            page_min: None,
            page_max: None,
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Get SurrealDB storage from app state.
///
/// Returns an error if SurrealDB storage is not initialized.
fn get_storage(state: &AppState) -> Result<Arc<SurrealStorage>, String> {
    state
        .surreal_storage
        .as_ref()
        .cloned()
        .ok_or_else(|| "SurrealDB storage not initialized".to_string())
}

/// Build SearchFilter from options.
fn build_filter(opts: &SurrealSearchOptions) -> Option<SearchFilter> {
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
// SEARCH COMMANDS (Task 6.1.1 - Hybrid Search)
// ============================================================================

/// Perform hybrid search using SurrealDB.
///
/// Combines vector (semantic) and full-text (keyword) search with configurable
/// weights. Requires pre-computed query embedding for the semantic component.
///
/// If no embedding is provided, falls back to keyword-only search.
///
/// # Arguments
///
/// * `query` - Search query string
/// * `embedding` - Query embedding vector (768 dimensions for BGE-base)
/// * `options` - Search options (limit, filters, semantic ratio)
/// * `state` - Application state with SurrealDB connection
///
/// # Returns
///
/// Search response with results, metadata, and performance info.
///
/// # Example (Frontend)
///
/// ```typescript
/// const embedding = await generateEmbedding(query); // From LLM provider
/// const response = await invoke('search_surrealdb', {
///   query: 'How does flanking work?',
///   embedding: embedding,
///   options: {
///     limit: 10,
///     semanticRatio: 0.7,
///     contentType: 'rules'
///   }
/// });
/// ```
#[tauri::command]
pub async fn search_surrealdb(
    query: String,
    embedding: Option<Vec<f32>>,
    options: Option<SurrealSearchOptions>,
    state: State<'_, AppState>,
) -> Result<SurrealSearchResponse, String> {
    let start = std::time::Instant::now();
    let opts = options.unwrap_or_default();

    log::info!(
        "[search_surrealdb] Query: '{}', has_embedding: {}, limit: {}",
        query,
        embedding.is_some(),
        opts.limit
    );

    // Get storage
    let storage = get_storage(&state)?;
    let db = storage.db();

    // Build filter
    let filter = build_filter(&opts);
    let filter_str = filter.as_ref().and_then(|f| f.to_surql());

    let has_embedding = embedding.as_ref().map(|e| !e.is_empty()).unwrap_or(false);

    let (results, search_type) = match embedding {
        Some(emb) if !emb.is_empty() => {
            // Hybrid search with embedding
            let config = if let Some(ratio) = opts.semantic_ratio {
                HybridSearchConfig::from_semantic_ratio(ratio).with_limit(opts.limit)
            } else {
                HybridSearchConfig::default().with_limit(opts.limit)
            };

            let res = hybrid_search(db, &query, emb, &config, filter_str.as_deref())
                .await
                .map_err(|e| format!("Hybrid search failed: {}", e))?;

            (res, "hybrid")
        }
        _ => {
            // Keyword-only search (no embedding)
            let res = fulltext_search(db, &query, opts.limit, filter_str.as_deref())
                .await
                .map_err(|e| format!("Fulltext search failed: {}", e))?;

            (res, "keyword")
        }
    };

    let processing_time_ms = start.elapsed().as_millis() as u64;
    let total_hits = results.len();

    // Build hints
    let mut hints = Vec::new();
    if search_type == "keyword" && !has_embedding {
        hints.push("No embedding provided - using keyword-only search".to_string());
    }
    if processing_time_ms > 300 {
        hints.push(format!("Search took {}ms (target: <300ms)", processing_time_ms));
    }

    log::info!(
        "[search_surrealdb] Completed {} search: {} results in {}ms",
        search_type,
        total_hits,
        processing_time_ms
    );

    Ok(SurrealSearchResponse {
        results: results.into_iter().map(SurrealSearchHit::from).collect(),
        total_hits,
        query,
        processing_time_ms,
        search_type: search_type.to_string(),
        hints,
    })
}

/// Perform vector-only search using SurrealDB.
///
/// Uses HNSW KNN search for pure semantic matching. Best for conceptual
/// queries where exact keyword matching is less important.
///
/// # Arguments
///
/// * `embedding` - Query embedding vector (768 dimensions)
/// * `limit` - Maximum results to return (default: 10)
/// * `options` - Search options (filters only, no semantic_ratio)
/// * `state` - Application state with SurrealDB connection
///
/// # Returns
///
/// Search response with results ordered by vector similarity.
#[tauri::command]
pub async fn vector_search_surrealdb(
    embedding: Vec<f32>,
    limit: Option<usize>,
    options: Option<SurrealSearchOptions>,
    state: State<'_, AppState>,
) -> Result<SurrealSearchResponse, String> {
    let start = std::time::Instant::now();
    let opts = options.unwrap_or_default();
    let limit = limit.unwrap_or(opts.limit);

    log::info!(
        "[vector_search_surrealdb] Embedding dims: {}, limit: {}",
        embedding.len(),
        limit
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

    // Build filter
    let filter = build_filter(&opts);
    let filter_str = filter.as_ref().and_then(|f| f.to_surql());

    let results = vector_search(db, embedding, limit, filter_str.as_deref())
        .await
        .map_err(|e| format!("Vector search failed: {}", e))?;

    let processing_time_ms = start.elapsed().as_millis() as u64;
    let total_hits = results.len();

    log::info!(
        "[vector_search_surrealdb] Completed: {} results in {}ms",
        total_hits,
        processing_time_ms
    );

    Ok(SurrealSearchResponse {
        results: results.into_iter().map(SurrealSearchHit::from).collect(),
        total_hits,
        query: "<vector query>".to_string(),
        processing_time_ms,
        search_type: "vector".to_string(),
        hints: vec![],
    })
}

/// Perform keyword-only search using SurrealDB.
///
/// Uses BM25 ranking with the ttrpg_analyzer (English stemming, ASCII normalization).
/// Includes search highlighting.
///
/// # Arguments
///
/// * `query` - Search query string
/// * `limit` - Maximum results to return (default: 10)
/// * `options` - Search options (filters only)
/// * `state` - Application state with SurrealDB connection
///
/// # Returns
///
/// Search response with results and highlights.
#[tauri::command]
pub async fn keyword_search_surrealdb(
    query: String,
    limit: Option<usize>,
    options: Option<SurrealSearchOptions>,
    state: State<'_, AppState>,
) -> Result<SurrealSearchResponse, String> {
    let start = std::time::Instant::now();
    let opts = options.unwrap_or_default();
    let limit = limit.unwrap_or(opts.limit);

    log::info!(
        "[keyword_search_surrealdb] Query: '{}', limit: {}",
        query,
        limit
    );

    // Get storage
    let storage = get_storage(&state)?;
    let db = storage.db();

    // Build filter
    let filter = build_filter(&opts);
    let filter_str = filter.as_ref().and_then(|f| f.to_surql());

    let results = fulltext_search(db, &query, limit, filter_str.as_deref())
        .await
        .map_err(|e| format!("Keyword search failed: {}", e))?;

    let processing_time_ms = start.elapsed().as_millis() as u64;
    let total_hits = results.len();

    log::info!(
        "[keyword_search_surrealdb] Completed: {} results in {}ms",
        total_hits,
        processing_time_ms
    );

    Ok(SurrealSearchResponse {
        results: results.into_iter().map(SurrealSearchHit::from).collect(),
        total_hits,
        query,
        processing_time_ms,
        search_type: "keyword".to_string(),
        hints: vec![],
    })
}

// ============================================================================
// SUGGESTIONS (Task 6.1.2)
// ============================================================================

/// Get search suggestions for autocomplete.
///
/// Returns word suggestions that start with the given prefix, extracted from
/// indexed content.
///
/// # Arguments
///
/// * `prefix` - Partial word to complete
/// * `limit` - Maximum suggestions to return (default: 10)
/// * `state` - Application state with SurrealDB connection
///
/// # Returns
///
/// List of suggested completions.
///
/// # Example (Frontend)
///
/// ```typescript
/// const suggestions = await invoke('get_suggestions_surrealdb', {
///   prefix: 'flank',
///   limit: 5
/// });
/// // Returns: ['flanking', 'flank', 'flanked']
/// ```
#[tauri::command]
pub async fn get_suggestions_surrealdb(
    prefix: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let limit = limit.unwrap_or(10);

    log::debug!(
        "[get_suggestions_surrealdb] Prefix: '{}', limit: {}",
        prefix,
        limit
    );

    if prefix.len() < 2 {
        return Ok(vec![]);
    }

    // Get storage
    let storage = get_storage(&state)?;
    let db = storage.db();

    // Query for content containing the prefix and extract words
    // We use a simple approach: search for chunks containing the prefix,
    // then extract and filter words from the content
    let query_str = r#"
        SELECT content
        FROM chunk
        WHERE content @@ $prefix
        LIMIT 100;
    "#;

    let mut response = db
        .query(query_str)
        .bind(("prefix", prefix.clone()))
        .await
        .map_err(|e| format!("Suggestion query failed: {}", e))?;

    #[derive(Deserialize)]
    struct ContentRow {
        content: String,
    }

    let rows: Vec<ContentRow> = response
        .take(0)
        .map_err(|e| format!("Failed to parse suggestions: {}", e))?;

    // Extract words that start with the prefix
    let prefix_lower = prefix.to_lowercase();
    let mut suggestions: Vec<String> = rows
        .into_iter()
        .flat_map(|row| {
            row.content
                .split(|c: char| !c.is_alphanumeric())
                .filter(|w| w.len() >= prefix.len())
                .filter(|w| w.to_lowercase().starts_with(&prefix_lower))
                .map(|w| w.to_lowercase())
                .collect::<Vec<_>>()
        })
        .collect();

    // Deduplicate and sort by length (shorter = more likely to be the base word)
    suggestions.sort();
    suggestions.dedup();
    suggestions.sort_by_key(|s| s.len());
    suggestions.truncate(limit);

    log::debug!(
        "[get_suggestions_surrealdb] Found {} suggestions for '{}'",
        suggestions.len(),
        prefix
    );

    Ok(suggestions)
}

// ============================================================================
// HEALTH CHECK (Task 6.1.3)
// ============================================================================

/// Check SurrealDB search health status.
///
/// Verifies the database is accessible and returns status information.
///
/// # Returns
///
/// Health status with database info.
#[tauri::command]
pub async fn check_surrealdb_health(
    state: State<'_, AppState>,
) -> Result<SurrealHealthStatus, String> {
    log::debug!("[check_surrealdb_health] Checking SurrealDB status");

    let storage = get_storage(&state)?;

    // Perform health check
    storage
        .health_check()
        .await
        .map_err(|e| format!("Health check failed: {}", e))?;

    // Get configuration
    let config = storage.config().await;

    // Get chunk count
    let db = storage.db();
    let count_result: Result<Vec<serde_json::Value>, _> = db
        .query("SELECT count() FROM chunk GROUP ALL")
        .await
        .and_then(|mut r| r.take(0));

    let chunk_count = count_result
        .ok()
        .and_then(|v| v.first().cloned())
        .and_then(|v| v.get("count").and_then(|c| c.as_u64()))
        .unwrap_or(0);

    Ok(SurrealHealthStatus {
        healthy: true,
        namespace: config.namespace,
        database: config.database,
        chunk_count,
        vector_dimensions: config.default_vector_dimensions,
    })
}

/// SurrealDB health status response.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurrealHealthStatus {
    /// Whether the database is healthy
    pub healthy: bool,
    /// Active namespace
    pub namespace: String,
    /// Active database
    pub database: String,
    /// Number of indexed chunks
    pub chunk_count: u64,
    /// Configured vector dimensions
    pub vector_dimensions: u32,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_options_defaults() {
        let opts = SurrealSearchOptions::default();
        assert_eq!(opts.limit, 10);
        assert!(opts.semantic_ratio.is_none());
        assert!(opts.content_type.is_none());
    }

    #[test]
    fn test_build_filter_empty() {
        let opts = SurrealSearchOptions::default();
        assert!(build_filter(&opts).is_none());
    }

    #[test]
    fn test_build_filter_with_content_type() {
        let opts = SurrealSearchOptions {
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
    fn test_build_filter_with_page_range() {
        let opts = SurrealSearchOptions {
            page_min: Some(100),
            page_max: Some(200),
            ..Default::default()
        };
        let filter = build_filter(&opts);
        assert!(filter.is_some());
        let surql = filter.unwrap().to_surql().unwrap();
        assert!(surql.contains("page_number >= 100"));
        assert!(surql.contains("page_number <= 200"));
    }

    #[test]
    fn test_search_hit_from_search_result() {
        let result = SearchResult {
            id: "chunk:abc".to_string(),
            content: "Test content".to_string(),
            score: 0.85,
            linear_score: None,
            source: "test-doc".to_string(),
            page_number: Some(42),
            section_path: Some("Chapter 1/Section 2".to_string()),
            content_type: "rules".to_string(),
            highlights: Some("<mark>Test</mark> content".to_string()),
        };

        let hit: SurrealSearchHit = result.into();
        assert_eq!(hit.id, "chunk:abc");
        assert_eq!(hit.content, "Test content");
        assert_eq!(hit.score, 0.85);
        assert_eq!(hit.source, "test-doc");
        assert_eq!(hit.page_number, Some(42));
        assert_eq!(hit.content_type, "rules");
        assert!(hit.highlights.is_some());
    }

    #[test]
    fn test_health_status_serialization() {
        let status = SurrealHealthStatus {
            healthy: true,
            namespace: "ttrpg".to_string(),
            database: "main".to_string(),
            chunk_count: 1000,
            vector_dimensions: 768,
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"healthy\":true"));
        assert!(json.contains("\"namespace\":\"ttrpg\""));
        assert!(json.contains("\"chunkCount\":1000"));
    }
}
