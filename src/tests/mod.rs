//! Test modules for TTRPG Assistant
//!
//! Run all tests: `cargo test`
//! Run integration tests (requires Meilisearch): `cargo test -- --ignored`
//! Run character generation tests: `cargo test character_gen`
//! Run property tests: `cargo test property --release`
//! Run new integration tests: `cargo test integration`
//!
//! Test Organization:
//! - `common/`: Shared fixtures, validators, and test helpers
//! - `database/`: Database CRUD tests (campaigns, sessions, characters, npcs, etc.)
//! - `unit/`: Unit tests for core components (session, security, voice, etc.)
//! - `integration/`: Integration tests requiring external services
//! - `property/`: Property-based tests

pub mod common;
mod database;
pub mod integration;
mod meilisearch_integration_tests;
pub mod mocks;
mod property;
pub mod unit;
