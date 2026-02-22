/// TTTTRPS - AI-Powered TTRPG Assistant (TUI Edition)
///
/// Core library providing campaign management, LLM integration,
/// document ingestion, and search for tabletop RPG game masters.

pub mod config;
pub mod core;
pub mod database;
pub mod ingestion;
pub mod oauth;
pub mod tui;

#[cfg(test)]
mod tests;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
