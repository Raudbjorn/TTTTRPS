#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod native_features;

use ttrpg_assistant::commands;

use tauri::Manager;
use std::sync::Mutex as StdMutex;
use native_features::NativeFeaturesState;

fn main() {
    // Initialize logging
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app.state::<NativeFeaturesState>().inner().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = state.initialize(&handle).await {
                    eprintln!("Failed to initialize native features: {}", e);
                }
            });
            Ok(())
        })
        // Native features (DragDrop, Dialogs)
        .manage(NativeFeaturesState::new())
        // New AppState
        .manage(commands::AppState {
            llm_client: StdMutex::new(None),
            llm_config: StdMutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            native_features::show_native_file_dialog,
            native_features::show_save_dialog,
            native_features::send_native_notification,
            native_features::handle_drag_drop_event,
            // LLM Commands
            commands::configure_llm,
            commands::chat,
            commands::check_llm_health,
            commands::get_llm_config,
            // Document Commands
            commands::ingest_document,
            commands::search,
            // Character Commands
            commands::generate_character,
            // Campaign Commands
            commands::create_campaign,
            commands::list_campaigns,
            commands::get_campaign,
            commands::delete_campaign,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
