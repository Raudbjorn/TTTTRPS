//! Database Unit Tests
//!
//! Comprehensive tests for SQLite database operations, split by entity type.
//! Uses an in-memory database for fast, isolated testing.
//!
//! Submodules:
//! - `campaigns`: Campaign CRUD and versioning tests
//! - `sessions`: Session lifecycle and event tests
//! - `characters`: Character save/update/delete tests
//! - `npcs`: NPC and conversation tests
//! - `usage`: Usage tracking and analytics tests
//! - `settings`: Settings CRUD tests

mod campaigns;
mod characters;
mod npcs;
mod sessions;
mod settings;
mod usage;
