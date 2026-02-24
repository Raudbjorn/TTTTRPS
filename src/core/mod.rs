
pub mod assets;
pub mod models;
pub mod logging;
pub mod voice;
pub mod llm;
// llm_router moved to llm::router
pub mod campaign_manager;
pub mod campaign;
pub mod credentials;
pub mod personality_base;
pub mod personality;
pub mod archetype;
pub mod session_manager;
pub mod character_gen;
pub mod npc_gen;
pub mod audio;
pub mod theme;
pub mod location_gen;

// Meilisearch-based search (replaces vector_store, keyword_search, hybrid_search, embedding_pipeline)
// search_client.rs refactored into search/ module
pub mod search_pipeline;
pub mod search_chat;

// Unified search module: Meilisearch client + hybrid search + embeddings + TTRPG synonyms
pub mod search;

// TTRPG-specific search enhancement (query parsing, ranking, filtering)
pub mod ttrpg_search;

// RAG (Retrieval-Augmented Generation) configuration
pub mod rag;

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
pub mod plot_types;
pub mod session_summary;
pub mod search_analytics;
pub mod name_gen;
pub mod voice_queue;
pub mod transcription;

// TASK-022, TASK-023, TASK-024: Analytics and Security modules
pub mod usage;
pub mod security;

// Session submodules (TASK-014, TASK-015, TASK-017)
pub mod session;

// SurrealDB-based unified storage (Phase 1 of SurrealDB migration)
pub mod storage;

// Query preprocessing: typo correction + synonym expansion
pub mod preprocess;

// Embedded Meilisearch Core (Wilysearch)
pub mod wilysearch;
