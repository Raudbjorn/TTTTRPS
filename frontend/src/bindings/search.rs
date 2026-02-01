use serde::{Deserialize, Serialize};
use super::core::{invoke, invoke_void, invoke_no_args};

// ============================================================================
// Meilisearch
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeilisearchStatus {
    pub healthy: bool,
    pub host: String,
    pub document_counts: Option<std::collections::HashMap<String, u64>>,
}

pub async fn check_meilisearch_health() -> Result<MeilisearchStatus, String> {
    invoke_no_args("check_meilisearch_health").await
}

pub async fn reindex_library(index_name: Option<String>) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        index_name: Option<String>,
    }
    invoke("reindex_library", &Args { index_name }).await
}

// ============================================================================
// Embedder Configuration
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupEmbeddingsResult {
    pub indexes_configured: Vec<String>,
    pub model: String,
    pub dimensions: u32,
    pub host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaEmbeddingModel {
    pub name: String,
    pub size: String,
    pub dimensions: u32,
}

/// Setup Ollama embeddings on all content indexes
pub async fn setup_ollama_embeddings(host: String, model: String) -> Result<SetupEmbeddingsResult, String> {
    #[derive(Serialize)]
    struct Args {
        host: String,
        model: String,
    }
    invoke("setup_ollama_embeddings", &Args { host, model }).await
}

/// Result from setting up Copilot embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupCopilotEmbeddingsResult {
    pub indexes_configured: Vec<String>,
    pub model: String,
    pub dimensions: u32,
    pub api_url: String,
}

/// Setup Copilot embeddings on all content indexes via direct Copilot API access
pub async fn setup_copilot_embeddings(
    model: String,
    dimensions: Option<u32>,
) -> Result<SetupCopilotEmbeddingsResult, String> {
    #[derive(Serialize)]
    struct Args {
        model: String,
        dimensions: Option<u32>,
    }
    invoke("setup_copilot_embeddings", &Args { model, dimensions }).await
}

/// Get embedder configuration for an index
pub async fn get_embedder_status(index_name: String) -> Result<Option<serde_json::Value>, String> {
    #[derive(Serialize)]
    struct Args {
        index_name: String,
    }
    invoke("get_embedder_status", &Args { index_name }).await
}

/// List available Ollama embedding models
pub async fn list_ollama_embedding_models(host: String) -> Result<Vec<OllamaEmbeddingModel>, String> {
    #[derive(Serialize)]
    struct Args {
        host: String,
    }
    invoke("list_ollama_embedding_models", &Args { host }).await
}

/// Local embedding model info (HuggingFace/ONNX - runs locally via Meilisearch)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalEmbeddingModel {
    pub id: String,
    pub name: String,
    pub dimensions: u32,
    pub description: String,
}

/// List available local embedding models (HuggingFace/ONNX - no external service required)
pub async fn list_local_embedding_models() -> Result<Vec<LocalEmbeddingModel>, String> {
    invoke_no_args("list_local_embedding_models").await
}

/// Setup local embeddings on all content indexes using HuggingFace embedder
pub async fn setup_local_embeddings(model: String) -> Result<SetupEmbeddingsResult, String> {
    #[derive(Serialize)]
    struct Args {
        model: String,
    }
    invoke("setup_local_embeddings", &Args { model }).await
}

pub async fn get_vector_store_status() -> Result<String, String> {
    invoke_no_args("get_vector_store_status").await
}

// ============================================================================
// Search Types and Commands
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    pub limit: usize,
    pub source_type: Option<String>,
    pub campaign_id: Option<String>,
    pub index: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultPayload {
    pub content: String,
    pub source: String,
    pub source_type: String,
    pub page_number: Option<u32>,
    pub score: f32,
    pub index: String,
}

pub async fn search(query: String, options: Option<SearchOptions>) -> Result<Vec<SearchResultPayload>, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
        options: Option<SearchOptions>,
    }
    invoke("search", &Args { query, options }).await
}

pub async fn semantic_search(query: String, limit: usize) -> Result<Vec<SearchResultPayload>, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
        limit: usize,
    }
    invoke("semantic_search", &Args { query, limit }).await
}

pub async fn keyword_search(query: String, limit: usize) -> Result<Vec<SearchResultPayload>, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
        limit: usize,
    }
    invoke("keyword_search", &Args { query, limit }).await
}

// ============================================================================
// Hybrid Search Types and Commands
// ============================================================================

/// Options for hybrid search
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HybridSearchOptions {
    #[serde(default)]
    pub limit: usize,
    pub source_type: Option<String>,
    pub campaign_id: Option<String>,
    pub index: Option<String>,
    pub semantic_weight: Option<f32>,
    pub keyword_weight: Option<f32>,
}

/// Hybrid search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResultPayload {
    pub content: String,
    pub source: String,
    pub source_type: String,
    pub page_number: Option<u32>,
    pub score: f32,
    pub index: String,
    pub keyword_rank: Option<usize>,
    pub semantic_rank: Option<usize>,
}

/// Hybrid search response with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResponse {
    pub results: Vec<HybridSearchResultPayload>,
    pub total_hits: usize,
    pub original_query: String,
    pub expanded_query: Option<String>,
    pub corrected_query: Option<String>,
    pub processing_time_ms: u64,
    pub hints: Vec<String>,
}

/// Query expansion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExpansionResult {
    pub original: String,
    pub expanded_query: String,
    pub was_expanded: bool,
    pub expansions: Vec<ExpansionInfo>,
    pub hints: Vec<String>,
}

/// Expansion info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpansionInfo {
    pub original: String,
    pub expanded_to: Vec<String>,
    pub category: String,
}

/// Spell correction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionResult {
    pub original_query: String,
    pub corrected_query: String,
    pub corrections: Vec<SpellingSuggestion>,
    pub has_corrections: bool,
}

/// Spelling suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellingSuggestion {
    pub original: String,
    pub suggestion: String,
    pub distance: usize,
    pub confidence: f64,
}

/// Perform hybrid search with RRF fusion
pub async fn hybrid_search(query: String, options: Option<HybridSearchOptions>) -> Result<HybridSearchResponse, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
        options: Option<HybridSearchOptions>,
    }
    invoke("hybrid_search", &Args { query, options }).await
}

/// Get search suggestions for autocomplete
pub async fn get_search_suggestions(partial: String) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args {
        partial: String,
    }
    invoke("get_search_suggestions", &Args { partial }).await
}

/// Get search hints for a query
pub async fn get_search_hints(query: String) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
    }
    invoke("get_search_hints", &Args { query }).await
}

/// Expand a query with TTRPG synonyms
pub async fn expand_query(query: String) -> Result<QueryExpansionResult, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
    }
    invoke("expand_query", &Args { query }).await
}

/// Correct spelling in a query
pub async fn correct_query(query: String) -> Result<CorrectionResult, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
    }
    invoke("correct_query", &Args { query }).await
}

/// Copy text to system clipboard
pub async fn copy_to_clipboard(text: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        text: String,
    }
    invoke("copy_to_clipboard", &Args { text }).await
}

// ============================================================================
// Search Analytics
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAnalyticsSummary {
    pub total_searches: u32,
    pub zero_result_searches: u32,
    pub click_through_rate: f64,
    pub avg_results_per_search: f64,
    pub avg_execution_time_ms: f64,
    pub top_queries: Vec<(String, u32)>,
    pub failed_queries: Vec<String>,
    pub cache_stats: CacheStats,
    pub by_search_type: std::collections::HashMap<String, u32>,
    pub period_start: String,
    pub period_end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PopularQuery {
    pub query: String,
    pub count: u32,
    pub click_through_rate: f64,
    pub avg_result_count: f64,
    pub last_searched: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub avg_time_saved_ms: f64,
    pub total_time_saved_ms: u64,
    pub top_cached_queries: Vec<(String, u32)>,
}

pub async fn get_search_analytics(hours: i64) -> Result<SearchAnalyticsSummary, String> {
    #[derive(Serialize)]
    struct Args { hours: i64 }
    invoke("get_search_analytics", &Args { hours }).await
}

pub async fn get_popular_queries(limit: usize) -> Result<Vec<PopularQuery>, String> {
    #[derive(Serialize)]
    struct Args { limit: usize }
    invoke("get_popular_queries", &Args { limit }).await
}

pub async fn get_cache_stats() -> Result<CacheStats, String> {
    invoke_no_args("get_cache_stats").await
}

pub async fn get_trending_queries(limit: usize) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args { limit: usize }
    invoke("get_trending_queries", &Args { limit }).await
}

pub async fn get_zero_result_queries(hours: i64) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args { hours: i64 }
    invoke("get_zero_result_queries", &Args { hours }).await
}

pub async fn get_click_distribution() -> Result<std::collections::HashMap<usize, u32>, String> {
    invoke_no_args("get_click_distribution").await
}

pub async fn record_search_selection(
    search_id: String,
    query: String,
    result_index: usize,
    source: String,
    selection_delay_ms: u64,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        search_id: String,
        query: String,
        result_index: usize,
        source: String,
        selection_delay_ms: u64,
    }
    invoke_void("record_search_selection", &Args {
        search_id, query, result_index, source, selection_delay_ms
    }).await
}

// --- Database-Backed Analytics (Persistent) ---

pub async fn get_search_analytics_db(hours: i64) -> Result<SearchAnalyticsSummary, String> {
    #[derive(Serialize)]
    struct Args { hours: i64 }
    invoke("get_search_analytics_db", &Args { hours }).await
}

pub async fn get_popular_queries_db(limit: usize) -> Result<Vec<PopularQuery>, String> {
    #[derive(Serialize)]
    struct Args { limit: usize }
    invoke("get_popular_queries_db", &Args { limit }).await
}

pub async fn get_cache_stats_db() -> Result<CacheStats, String> {
    invoke_no_args("get_cache_stats_db").await
}

pub async fn get_trending_queries_db(limit: usize) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args { limit: usize }
    invoke("get_trending_queries_db", &Args { limit }).await
}

pub async fn get_zero_result_queries_db(hours: i64) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args { hours: i64 }
    invoke("get_zero_result_queries_db", &Args { hours }).await
}

pub async fn get_click_distribution_db() -> Result<std::collections::HashMap<usize, u32>, String> {
    invoke_no_args("get_click_distribution_db").await
}

pub async fn record_search_event(
    query: String,
    result_count: usize,
    execution_time_ms: u64,
    search_type: String,
    from_cache: bool,
    source_filter: Option<String>,
    campaign_id: Option<String>,
) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
        result_count: usize,
        execution_time_ms: u64,
        search_type: String,
        from_cache: bool,
        source_filter: Option<String>,
        campaign_id: Option<String>,
    }
    invoke("record_search_event", &Args {
        query, result_count, execution_time_ms, search_type, from_cache, source_filter, campaign_id
    }).await
}

pub async fn record_search_selection_db(
    search_id: String,
    query: String,
    result_index: usize,
    source: String,
    selection_delay_ms: u64,
    was_helpful: Option<bool>,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        search_id: String,
        query: String,
        result_index: usize,
        source: String,
        selection_delay_ms: u64,
        was_helpful: Option<bool>,
    }
    invoke_void("record_search_selection_db", &Args {
        search_id, query, result_index, source, selection_delay_ms, was_helpful
    }).await
}

pub async fn cleanup_search_analytics(days: i64) -> Result<u64, String> {
    #[derive(Serialize)]
    struct Args { days: i64 }
    invoke("cleanup_search_analytics", &Args { days }).await
}
