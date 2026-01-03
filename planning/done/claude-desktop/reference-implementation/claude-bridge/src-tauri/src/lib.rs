//! Claude Bridge Tauri application library.

pub mod commands;

use commands::AppState;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize logging with tracing.
pub fn init_logging() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("claude_bridge=debug".parse().unwrap()))
        .init();
}

/// Create and run the Tauri application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::connect,
            commands::disconnect,
            commands::get_status,
            commands::send_message,
            commands::new_conversation,
            commands::get_conversation,
            commands::update_config,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
