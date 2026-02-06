//! Search and Library Commands Module
//!
//! Commands for search, document ingestion, library management,
//! TTRPG document queries, search analytics, embeddings configuration,
//! and extraction settings.
//!
//! ## SurrealDB Migration
//!
//! The `surrealdb` and `rag_surrealdb` modules contain SurrealDB-backed
//! implementations that will replace the Meilisearch commands after migration.
//! These are registered alongside existing commands during the transition period.
//!
//! ## Query Preprocessing (REQ-QP-003)
//!
//! The `preprocessing` module provides search commands with automatic typo
//! correction and synonym expansion. These commands use the QueryPipeline
//! from AppState to preprocess queries before searching.

pub mod query;
pub mod suggestions;
pub mod library;
pub mod ingestion;
pub mod extraction;
pub mod ttrpg_docs;
pub mod embeddings;
pub mod analytics;
pub mod meilisearch;
pub mod types;

// SurrealDB migration modules (Tasks 6.1.1-6.1.3, 4.2.3)
pub mod surrealdb;
pub mod rag_surrealdb;

// Query preprocessing module (REQ-QP-003)
pub mod preprocessing;

// Re-export all commands using glob to include Tauri __cmd__ macros
pub use query::*;
pub use suggestions::*;
pub use library::*;
pub use ingestion::*;
pub use extraction::*;
pub use ttrpg_docs::*;
pub use embeddings::*;
pub use analytics::*;
pub use meilisearch::*;
pub use types::*;

// Re-export SurrealDB commands
pub use surrealdb::*;
pub use rag_surrealdb::*;

// Re-export preprocessing commands
pub use preprocessing::*;
