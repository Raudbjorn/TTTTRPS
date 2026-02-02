/**
 * MDMAI Desktop Application - Core Library
 *
 * This library provides the core functionality for the MDMAI desktop application.
 */

// Public module exports
pub mod core;
pub mod database;
pub mod ingestion;
pub mod commands;
pub mod backstory_commands;
pub mod oauth;

// Test modules
#[cfg(test)]
mod tests;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

