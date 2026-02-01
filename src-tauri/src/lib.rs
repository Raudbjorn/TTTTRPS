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

// Logging utilities
pub mod logging {
    /// Initialize logging with the specified level
    pub fn init_logging(level: &str) -> Result<(), String> {
        let log_level = match level.to_lowercase().as_str() {
            "error" => log::LevelFilter::Error,
            "warn" => log::LevelFilter::Warn,
            "info" => log::LevelFilter::Info,
            "debug" => log::LevelFilter::Debug,
            "trace" => log::LevelFilter::Trace,
            _ => log::LevelFilter::Info,
        };

        env_logger::Builder::from_default_env()
            .filter_level(log_level)
            .format_timestamp_secs()
            .init();

        log::info!("Logging initialized at level: {}", level);
        Ok(())
    }
}
