//! Integration Tests Module
//!
//! Comprehensive integration tests that verify component interactions
//! and end-to-end functionality.
//!
//! # Test Categories
//!
//! ## Database Integration (`database_integration`)
//! - Full campaign lifecycle (create, update, snapshot, rollback, delete)
//! - Session with combat flow
//! - NPC conversation persistence
//! - Concurrent campaign access
//! - Backup and restore cycle
//!
//! ## Meilisearch Integration (`meilisearch_integration`)
//! - Document indexing end-to-end
//! - Search query types (typo tolerance, filtering, facets)
//! - Hybrid search (BM25 + semantic)
//! - Search analytics recording
//! - Index deletion and cleanup
//!
//! Note: Meilisearch tests requiring a running instance are marked `#[ignore]`
//!
//! ## LLM Integration (`llm_integration`)
//! - Provider failover with mock failures
//! - Streaming chunk assembly
//! - Context window management
//! - Cost tracking accumulation
//! - Circuit breaker behavior
//! - Health tracking and recovery
//!
//! ## Chat Provider Integration (`chat_provider_integration`)
//! - OpenAI provider configuration
//! - Claude provider configuration (proxy-based)
//! - Grok (xAI) provider configuration
//! - All provider chat completions
//!
//! ## Wizard Integration (`wizard_integration`)
//! - Complete wizard flow (manual mode)
//! - AI-assisted wizard flow
//! - Draft recovery scenarios
//! - Auto-save functionality
//! - Step validation and transitions
//!
//! # Running Tests
//!
//! ```bash
//! # Run all integration tests (excluding ignored)
//! cargo test integration
//!
//! # Run ignored tests (requires external services)
//! cargo test integration -- --ignored
//!
//! # Run specific integration test module
//! cargo test integration::database_integration
//! cargo test integration::meilisearch_integration
//! cargo test integration::llm_integration
//! cargo test integration::chat_provider_integration
//! cargo test integration::wizard_integration
//! ```

pub mod chat_provider_integration;
pub mod database_integration;
pub mod llm_integration;
pub mod meilisearch_integration;
pub mod wizard_integration;
