
pub mod models;
pub mod voice;
pub mod llm;
// llm_router moved to llm::router
pub mod campaign_manager;
pub mod campaign;
pub mod credentials;
pub mod personality_base;
pub mod personality;
pub mod session_manager;
pub mod character_gen;
pub mod npc_gen;
pub mod audio;
pub mod theme;
pub mod location_gen;

// Meilisearch-based search (replaces vector_store, keyword_search, hybrid_search, embedding_pipeline)
pub mod sidecar_manager;
pub mod search_client;
pub mod meilisearch_pipeline;
pub mod meilisearch_chat;

// Enhanced search with hybrid search, embeddings, and TTRPG synonyms
pub mod search;

// TTRPG-specific search enhancement (query parsing, ranking, filtering)
pub mod ttrpg_search;

// Extended feature modules
pub mod streaming;
pub mod budget;
pub mod alerts;
pub mod query_expansion;
pub mod input_validator;
pub mod audit;
pub mod cost_predictor;
pub mod spell_correction;
pub mod location_manager;
pub mod plot_manager;
pub mod session_summary;
pub mod search_analytics;
pub mod name_gen;
pub mod voice_queue;
pub mod transcription;

// TASK-022, TASK-023, TASK-024: Analytics and Security modules
pub mod usage;
pub mod security;

// Session sub-modules (TASK-014, TASK-015, TASK-017)
pub mod session;

// Claude Desktop CDP bridge
pub mod claude_cdp;
