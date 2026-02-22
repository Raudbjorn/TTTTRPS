//! Storage module for SurrealDB-based unified storage layer.
//!
//! This module provides a unified storage interface using SurrealDB, combining:
//! - Full-text search capabilities
//! - Vector search for semantic queries
//! - Graph relationships for TTRPG entities
//! - Document storage and retrieval
//!
//! # Architecture
//!
//! The storage layer replaces the previous SQLite + Meilisearch architecture with
//! a single SurrealDB instance that handles all data persistence and search needs.
//!
//! # Modules
//!
//! - `surrealdb` - Core SurrealStorage wrapper for database operations
//! - `error` - Error types for storage operations
//! - `schema` - Database schema definitions using SurrealQL
//! - `search` - Full-text and vector search functions
//! - `rag` - RAG (Retrieval-Augmented Generation) pipeline
//! - `ingestion` - Document ingestion and chunking
//! - `migration` - SQLite/Meilisearch to SurrealDB migration utilities
//! - `models` - Data models for storage operations

pub mod surrealdb;
pub mod error;
pub mod schema;
pub mod search;
pub mod rag;
pub mod ingestion;
pub mod migration;
pub mod models;

pub use error::StorageError;
pub use surrealdb::SurrealStorage;

// Migration types and functions (Task 5.1.1-5.3.2)
pub use migration::{
    MigrationStatus, MigrationPhase, MigrationCounts, MeilisearchDocument,
    backup_sqlite, migrate_campaigns, migrate_npcs, migrate_sessions,
    migrate_chat_messages, migrate_library_items, migrate_meilisearch_index,
    validate_migration, get_migration_progress, save_migration_progress,
    run_migration,
};

// Search functions and types (Task 2.1, 2.2, 2.3, Task 11)
pub use search::{
    SearchResult,
    HybridSearchConfig,
    ScoreNormalization,
    SearchFilter,
    PreprocessedSearchResult,
    vector_search,
    fulltext_search,
    fulltext_search_with_highlights,
    hybrid_search,
    hybrid_search_with_preprocessing,
};

pub use ingestion::{
    ChunkData,
    ingest_chunks,
    ingest_chunks_with_embeddings,
    delete_library_chunks,
    get_chunk_count,
    update_chunk_embeddings,
};

// Library item models and CRUD (Task 3.2.1, 3.2.2)
pub use models::{
    count_library_items, create_library_item, delete_library_item, get_library_item,
    get_library_item_by_slug, get_library_items, update_library_item, update_library_item_status,
    LibraryItem, LibraryItemBuilder, LibraryItemWithCount,
};

// RAG pipeline types and functions (Task 4.1, 4.2)
pub use rag::{
    RagConfig, RagSource, RagResponse, RagContext, FormattedContext,
    format_context, build_system_prompt, retrieve_rag_context, prepare_rag_context,
};
