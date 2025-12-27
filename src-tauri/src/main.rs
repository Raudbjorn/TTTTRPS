#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod native_features;

use ttrpg_assistant::commands;
use tauri::Manager;
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
        // Application state
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            // Native features
            native_features::show_native_file_dialog,
            native_features::show_save_dialog,
            native_features::send_native_notification,
            native_features::handle_drag_drop_event,

            // LLM Commands
            commands::configure_llm,
            commands::chat,
            commands::check_llm_health,
            commands::get_llm_config,

            // Campaign Commands
            commands::list_campaigns,
            commands::create_campaign,
            commands::get_campaign,
            commands::update_campaign,
            commands::delete_campaign,
            commands::create_snapshot,
            commands::list_snapshots,
            commands::restore_snapshot,
            commands::export_campaign,
            commands::import_campaign,

            // Campaign Notes Commands
            commands::add_campaign_note,
            commands::get_campaign_notes,
            commands::search_campaign_notes,
            commands::delete_campaign_note,

            // Session Commands
            commands::start_session,
            commands::get_session,
            commands::get_active_session,
            commands::list_sessions,
            commands::end_session,

            // Combat Commands
            commands::start_combat,
            commands::end_combat,
            commands::get_combat,
            commands::add_combatant,
            commands::remove_combatant,
            commands::next_turn,
            commands::get_current_combatant,
            commands::damage_combatant,
            commands::heal_combatant,
            commands::add_condition,
            commands::remove_condition,

            // Character Generation Commands
            commands::generate_character,
            commands::get_supported_systems,

            // NPC Commands
            commands::generate_npc,
            commands::get_npc,
            commands::list_npcs,
            commands::update_npc,
            commands::delete_npc,
            commands::search_npcs,

            // Document Ingestion Commands
            commands::ingest_pdf,

            // Voice Commands
            commands::configure_voice,
            commands::synthesize_voice,
            commands::get_voice_presets,

            // Audio Commands
            commands::get_audio_volumes,
            commands::get_sfx_categories,

            // Credential Commands
            commands::save_api_key,
            commands::get_api_key,
            commands::delete_api_key,
            commands::list_stored_providers,

            // Utility Commands
            commands::get_app_version,
            commands::get_system_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
