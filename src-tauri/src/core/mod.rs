pub mod models;
pub mod voice;
pub mod llm;
pub mod llm_router;
pub mod campaign_manager;
pub mod credentials;
pub mod personality;
pub mod session_manager;
pub mod character_gen;
pub mod npc_gen;
pub mod audio;
pub mod theme;

// Meilisearch-based search (replaces vector_store, keyword_search, hybrid_search, embedding_pipeline)
pub mod sidecar_manager;
pub mod search_client;
pub mod meilisearch_pipeline;
pub mod meilisearch_chat;

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
