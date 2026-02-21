//! Search Query Commands
//!
//! Core search functionality including basic search and hybrid search.
//! Uses embedded MeilisearchLib for direct Rust integration without HTTP.

use std::time::Instant;

use meilisearch_lib::{HybridQuery, SearchQuery};
use tauri::State;

use crate::commands::AppState;
// Re-exported from core::search::config - config module is private but items are pub
use crate::core::search::{all_indexes, select_index_for_source_type};

use super::types::{
    HybridSearchOptions, HybridSearchResponsePayload, HybridSearchResultPayload, SearchOptions,
    SearchResultPayload,
};

// ============================================================================
// Basic Search
// ============================================================================

/// Perform a keyword search across TTRPG content indexes.
///
/// This command searches using Meilisearch's BM25 ranking algorithm for fast,
/// typo-tolerant keyword matching.
///
/// # Arguments
/// * `query` - Search query string
/// * `options` - Optional search configuration (limit, filters, index)
/// * `state` - Application state containing embedded search engine
///
/// # Returns
/// Vector of search results with content, source, and relevance scores
#[tauri::command]
pub async fn search(
    query: String,
    options: Option<SearchOptions>,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResultPayload>, String> {
    let opts = options.unwrap_or_default();
    let meili = state.embedded_search.clone_inner();
    let query_clone = query.clone();

    tokio::task::spawn_blocking(move || {
        let start = Instant::now();

        // Determine which index(es) to search
        let indexes_to_search = if let Some(ref index) = opts.index {
            vec![index.as_str()]
        } else if let Some(ref source_type) = opts.source_type {
            vec![select_index_for_source_type(source_type)]
        } else {
            // Search all content indexes
            all_indexes()
        };

        // Build filter expression if we have campaign_id or source_type filters
        let filter = build_filter_expression(&opts);

        let mut all_results = Vec::new();

        for index_uid in indexes_to_search {
            // Build search query
            let mut search_query = SearchQuery::new(&query_clone);
            search_query = search_query.with_pagination(0, opts.limit);

            // Apply filter if present
            if let Some(ref filter_value) = filter {
                search_query = search_query.with_filter(filter_value.clone());
            }

            // Enable ranking scores
            search_query.show_ranking_score = true;

            // Execute search
            match meili.search(index_uid, search_query) {
                Ok(result) => {
                    for hit in result.hits {
                        if let Some(payload) = convert_hit_to_payload(&hit, index_uid) {
                            all_results.push(payload);
                        }
                    }
                }
                Err(e) => {
                    // Log error but continue with other indexes
                    log::warn!("Search error in index '{}': {}", index_uid, e);
                }
            }
        }

        // Sort by score descending and limit total results
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        all_results.truncate(opts.limit);

        log::debug!(
            "Search for '{}' returned {} results in {:?}",
            query_clone,
            all_results.len(),
            start.elapsed()
        );

        Ok(all_results)
    })
    .await
    .map_err(|e| format!("Search task failed: {}", e))?
}

// ============================================================================
// Hybrid Search
// ============================================================================

/// Perform hybrid search combining keyword and semantic matching.
///
/// Uses Meilisearch's native hybrid search which combines BM25 keyword matching
/// with vector similarity search. Results are fused using the configured
/// semantic ratio (0.0 = pure keyword, 1.0 = pure semantic).
///
/// # Arguments
/// * `query` - The search query string
/// * `options` - Optional search configuration (limit, filters, semantic_weight)
/// * `state` - Application state containing embedded search engine
///
/// # Returns
/// Search results with fused scores, timing, and query metadata
#[tauri::command]
pub async fn hybrid_search(
    query: String,
    options: Option<HybridSearchOptions>,
    state: State<'_, AppState>,
) -> Result<HybridSearchResponsePayload, String> {
    let opts = options.unwrap_or_default();
    let meili = state.embedded_search.clone_inner();
    let query_clone = query.clone();

    tokio::task::spawn_blocking(move || {
        let start = Instant::now();

        // Determine semantic ratio from options
        // Default to balanced (0.5) if not specified
        let semantic_ratio = opts.semantic_weight.unwrap_or(0.5);

        // Determine which index(es) to search
        let indexes_to_search = if let Some(ref index) = opts.index {
            vec![index.as_str()]
        } else if let Some(ref source_type) = opts.source_type {
            vec![select_index_for_source_type(source_type)]
        } else {
            // Search all content indexes for federated hybrid search
            all_indexes()
        };

        // Build filter expression
        let filter = build_hybrid_filter_expression(&opts);

        let mut all_results = Vec::new();
        let mut total_hits: usize = 0;
        let mut hints = Vec::new();

        for index_uid in indexes_to_search {
            // Build hybrid search query
            let hybrid_config = HybridQuery::new(semantic_ratio);
            let mut search_query = SearchQuery::new(&query_clone)
                .with_hybrid(hybrid_config)
                .with_pagination(0, opts.limit);

            // Apply filter if present
            if let Some(ref filter_value) = filter {
                search_query = search_query.with_filter(filter_value.clone());
            }

            // Enable ranking scores
            search_query.show_ranking_score = true;

            // Execute hybrid search
            match meili.search(index_uid, search_query) {
                Ok(result) => {
                    if let Some(estimated) = result.estimated_total_hits {
                        total_hits += estimated as usize;
                    }

                    // Track if semantic search found results
                    if let Some(semantic_count) = result.semantic_hit_count {
                        if semantic_count > 0 {
                            hints.push(format!(
                                "Found {} semantic matches in {}",
                                semantic_count, index_uid
                            ));
                        }
                    }

                    for (rank, hit) in result.hits.iter().enumerate() {
                        if let Some(payload) = convert_hit_to_hybrid_payload(&hit, index_uid, rank) {
                            all_results.push(payload);
                        }
                    }
                }
                Err(e) => {
                    // Log error but continue with other indexes
                    log::warn!("Hybrid search error in index '{}': {}", index_uid, e);
                    hints.push(format!("Search unavailable for {}: {}", index_uid, e));
                }
            }
        }

        // Sort by score descending and limit total results
        all_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all_results.truncate(opts.limit);

        let processing_time_ms = start.elapsed().as_millis() as u64;
        let within_target = processing_time_ms < 500; // Performance target: <500ms

        log::debug!(
            "Hybrid search for '{}' returned {} results in {}ms (target: {})",
            query_clone,
            all_results.len(),
            processing_time_ms,
            if within_target { "met" } else { "missed" }
        );

        Ok(HybridSearchResponsePayload {
            results: all_results,
            total_hits,
            original_query: query_clone,
            expanded_query: None, // Query expansion handled by MeilisearchLib synonyms
            corrected_query: None, // Typo tolerance handled by MeilisearchLib
            processing_time_ms,
            hints,
            within_target,
        })
    })
    .await
    .map_err(|e| format!("Hybrid search task failed: {}", e))?
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Build filter expression from SearchOptions
fn build_filter_expression(opts: &SearchOptions) -> Option<serde_json::Value> {
    let mut filters = Vec::new();

    if let Some(ref campaign_id) = opts.campaign_id {
        filters.push(format!("campaign_id = \"{}\"", campaign_id));
    }

    if let Some(ref source_type) = opts.source_type {
        filters.push(format!("source_type = \"{}\"", source_type));
    }

    if filters.is_empty() {
        None
    } else {
        Some(serde_json::Value::String(filters.join(" AND ")))
    }
}

/// Build filter expression from HybridSearchOptions
fn build_hybrid_filter_expression(opts: &HybridSearchOptions) -> Option<serde_json::Value> {
    let mut filters = Vec::new();

    if let Some(ref campaign_id) = opts.campaign_id {
        filters.push(format!("campaign_id = \"{}\"", campaign_id));
    }

    if let Some(ref source_type) = opts.source_type {
        filters.push(format!("source_type = \"{}\"", source_type));
    }

    if filters.is_empty() {
        None
    } else {
        Some(serde_json::Value::String(filters.join(" AND ")))
    }
}

/// Convert a MeilisearchLib SearchHit to frontend SearchResultPayload
fn convert_hit_to_payload(
    hit: &meilisearch_lib::SearchHit,
    index: &str,
) -> Option<SearchResultPayload> {
    let doc = &hit.document;

    // Extract content - try multiple field names
    let content = doc
        .get("content")
        .or_else(|| doc.get("text"))
        .or_else(|| doc.get("body"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Extract source - try multiple field names
    let source = doc
        .get("source")
        .or_else(|| doc.get("file_name"))
        .or_else(|| doc.get("book_title"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    // Extract source_type with fallback
    let source_type = doc
        .get("source_type")
        .or_else(|| doc.get("content_category"))
        .or_else(|| doc.get("chunk_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("document")
        .to_string();

    // Extract page number
    let page_number = doc
        .get("page_number")
        .and_then(|v| v.as_u64())
        .map(|n| n as u32);

    // Get ranking score, default to 0.0
    let score = hit.ranking_score.unwrap_or(0.0) as f32;

    Some(SearchResultPayload {
        content,
        source,
        source_type,
        page_number,
        score,
        index: index.to_string(),
    })
}

/// Convert a MeilisearchLib SearchHit to frontend HybridSearchResultPayload
fn convert_hit_to_hybrid_payload(
    hit: &meilisearch_lib::SearchHit,
    index: &str,
    rank: usize,
) -> Option<HybridSearchResultPayload> {
    let doc = &hit.document;

    // Extract content
    let content = doc
        .get("content")
        .or_else(|| doc.get("text"))
        .or_else(|| doc.get("body"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Extract source
    let source = doc
        .get("source")
        .or_else(|| doc.get("file_name"))
        .or_else(|| doc.get("book_title"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    // Extract source_type
    let source_type = doc
        .get("source_type")
        .or_else(|| doc.get("content_category"))
        .or_else(|| doc.get("chunk_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("document")
        .to_string();

    // Extract page number
    let page_number = doc
        .get("page_number")
        .and_then(|v| v.as_u64())
        .map(|n| n as u32);

    // Get ranking score
    let score = hit.ranking_score.unwrap_or(0.0) as f32;

    Some(HybridSearchResultPayload {
        content,
        source,
        source_type,
        page_number,
        score,
        index: index.to_string(),
        // Hybrid search returns a fused ranking; keyword/semantic ranks are not separable.
        keyword_rank: None,
        semantic_rank: None,
        // Native hybrid doesn't provide per-source overlap information.
        overlap_count: None,
    })
}
