//! Unit Tests for TTRPG Assistant Core Components
//!
//! This module contains comprehensive unit tests for:
//! - LLM Router (routing, failover, cost tracking, health monitoring)
//! - LLM Providers (Claude, OpenAI, Gemini, Ollama, etc.)
//! - Voice Manager (TTS providers, profiles, queue, cache)
//! - Security (XSS, SQL injection, path traversal, command injection)
//! - Character Generation (D&D 5e, PF2e, Call of Cthulhu)
//! - Document Ingestion (PDF, EPUB parsing)
//! - Session Manager (combat, initiative, HP, conditions, notes, timeline)
//! - Edge Cases (network errors, data errors, concurrent access, race conditions)
//!
//! Run all unit tests: `cargo test`
//! Run specific test: `cargo test llm_router`
//! Run voice tests: `cargo test voice_manager`
//! Run security tests: `cargo test security`
//! Run session tests: `cargo test session`
//! Run character gen tests: `cargo test character_gen`
//! Run edge case tests: `cargo test edge_cases`

// Modular test organization
mod session;

// Remaining monolithic test files
mod voice_manager_tests;
mod edge_cases_tests;

pub mod providers;

// Ingestion pipeline unit tests
mod ingestion;

// Character generation unit tests (Phase 4)
mod character_gen;
