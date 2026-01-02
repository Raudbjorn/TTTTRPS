#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod native_features;

use ttrpg_assistant::commands;
use ttrpg_assistant::backstory_commands;
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

            // Initialize native features async
            {
                let handle = handle.clone();
                let state = app.state::<NativeFeaturesState>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = state.initialize(&handle).await {
                        eprintln!("Failed to initialize native features: {}", e);
                    }
                });
            }

            // Initialize managers (Meilisearch-based)
            // Initialize managers (Meilisearch-based)
            let (cm, sm, ns, creds, vm, sidecar_manager, search_client, personality_store, personality_manager, pipeline, _llm_router, version_manager, world_state_manager, relationship_manager, location_manager) =
                commands::AppState::init_defaults();

            // Initialize Database
            let app_handle = app.handle();
            let app_dir = app_handle.path().app_data_dir().unwrap_or(std::path::PathBuf::from("."));
            let database = tauri::async_runtime::block_on(async {
                match ttrpg_assistant::database::Database::new(&app_dir).await {
                    Ok(db) => db,
                    Err(e) => {
                        let msg = format!("Failed to initialize database: {}", e);
                        eprintln!("{}", msg);
                        // TODO: Show native error dialog if possible
                        panic!("{}", msg);
                    }
                }
            });
            log::info!("Database initialized at {:?}", database.path());

            // Start Meilisearch Sidecar
            sidecar_manager.start(handle.clone());

            // Initialize Meilisearch indexes after sidecar starts
            let sc = search_client.clone();
            tauri::async_runtime::spawn(async move {
                // Wait for Meilisearch to be ready
                if sc.wait_for_health(10).await {
                    if let Err(e) = sc.initialize_indexes().await {
                        log::error!("Failed to initialize Meilisearch indexes: {}", e);
                    } else {
                        log::info!("Meilisearch indexes initialized successfully");
                    }
                } else {
                    log::warn!("Meilisearch not ready after 10 seconds - indexes not initialized");
                }
            });

            app.manage(commands::AppState {
                llm_client: std::sync::RwLock::new(None),
                llm_config: std::sync::RwLock::new(None),
                llm_router: tokio::sync::RwLock::new(ttrpg_assistant::core::llm::router::LLMRouter::with_defaults()),
                campaign_manager: cm,
                session_manager: sm,
                npc_store: ns,
                credentials: creds,
                voice_manager: vm,
                sidecar_manager,
                search_client,
                personality_store,
                personality_manager,
                ingestion_pipeline: pipeline,
                database,
                version_manager,
                world_state_manager,
                relationship_manager,
                location_manager,
            });

            // Auto-configure Ollama if no providers are present (User Request)
            let handle_clone = handle.clone();
            tauri::async_runtime::spawn(async move {
                // Wait briefly for startup
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;

                if let Some(app_state) = handle_clone.try_state::<commands::AppState>() {
                    let has_providers = !app_state.llm_router.read().await.provider_ids().is_empty();

                    if !has_providers {
                        log::info!("No LLM providers configured. Attempting to auto-discover Ollama...");
                        let client = reqwest::Client::new();
                        // Try localhost default port
                        if let Ok(resp) = client.get("http://localhost:11434/api/tags").send().await {
                             if let Ok(json) = resp.json::<serde_json::Value>().await {
                                let mut configured_model = None;

                                if let Some(models) = json.get("models").and_then(|m| m.as_array()) {
                                    if let Some(first) = models.first() {
                                        if let Some(name) = first.get("name").and_then(|n| n.as_str()) {
                                            configured_model = Some(name.to_string());
                                        }
                                    }
                                }

                                // Fallback if no models found but server is running
                                let model_to_use = configured_model.unwrap_or_else(|| "llama3:latest".to_string());

                                log::info!("Auto-configuring Ollama with model: {}", model_to_use);
                                let provider = std::sync::Arc::new(
                                    ttrpg_assistant::core::llm::providers::OllamaProvider::localhost(model_to_use)
                                );
                                app_state.llm_router.write().await.add_provider(provider).await;
                             }
                        } else {
                            log::warn!("Could not connect to Ollama at localhost:11434");
                        }
                    }
                }
            });
            // TASK-022, TASK-023, TASK-024: Initialize analytics state wrappers
            app.manage(commands::UsageTrackerState::default());
            app.manage(commands::SearchAnalyticsState::default());
            app.manage(commands::AuditLoggerState::default());

            // TASK-025: Initialize synthesis queue state
            app.manage(commands::SynthesisQueueState::default());

            Ok(())
        })
        // Native features (DragDrop, Dialogs)
        .manage(NativeFeaturesState::new())
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
            commands::get_router_stats,
            commands::list_ollama_models,
            commands::list_claude_models,
            commands::list_openai_models,
            commands::list_gemini_models,
            commands::list_openrouter_models,
            commands::list_provider_models,

            // Campaign Commands
            commands::list_campaigns,
            commands::create_campaign,
            commands::get_campaign,
            commands::update_campaign,
            commands::delete_campaign,
            commands::get_campaign_theme,
            commands::set_campaign_theme,
            commands::get_theme_preset,

            // Campaign Snapshots
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
            commands::create_planned_session,
            commands::start_planned_session,
            commands::end_session,

            // TASK-014: Timeline Commands
            commands::add_timeline_event,
            commands::get_session_timeline,
            commands::get_timeline_summary,
            commands::get_timeline_events_by_type,

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

            // Advanced Condition Commands (TASK-015)
            commands::add_condition_advanced,
            commands::remove_condition_by_id,
            commands::get_combatant_conditions,
            commands::tick_conditions_end_of_turn,
            commands::tick_conditions_start_of_turn,
            commands::list_condition_templates,

            // Character Generation Commands (TASK-018)
            commands::generate_character,
            commands::generate_character_advanced,
            commands::get_supported_systems,
            commands::list_system_info,

            // Backstory Generation Commands (TASK-019)
            backstory_commands::generate_backstory,
            backstory_commands::edit_backstory,

            // Location Generation Commands (TASK-020)
            commands::generate_location_quick,
            commands::generate_location,
            commands::list_location_types,

            // Personality Application Commands (TASK-021)
            commands::set_active_personality,
            commands::get_active_personality,
            commands::get_personality_prompt,
            commands::apply_personality_to_text,
            commands::get_personality_context,
            commands::get_session_personality_context,
            commands::set_personality_context,
            commands::set_narrator_personality,
            commands::assign_npc_personality,
            commands::unassign_npc_personality,
            commands::set_scene_mood,
            commands::set_personality_settings,
            commands::set_personality_active,
            commands::preview_personality,
            commands::preview_personality_extended,
            commands::generate_personality_preview,
            commands::test_personality,
            commands::get_session_system_prompt,
            commands::style_npc_dialogue,
            commands::build_npc_system_prompt,
            commands::build_narration_prompt,
            commands::list_personalities,
            commands::clear_session_personality_context,

            // NPC Commands
            commands::generate_npc,
            commands::get_npc,
            commands::list_npcs,
            commands::update_npc,
            commands::delete_npc,
            commands::search_npcs,

            // NPC Conversation Commands
            commands::list_npc_conversations,
            commands::get_npc_conversation,
            commands::add_npc_message,
            commands::mark_npc_read,
            commands::list_npc_summaries,
            commands::reply_as_npc,

            // Document Ingestion & Search (Meilisearch)
            commands::ingest_document,
            commands::ingest_document_with_progress,
            commands::ingest_pdf,
            commands::search,
            commands::check_meilisearch_health,
            commands::reindex_library,
            commands::get_vector_store_status,

            // Voice Commands
            commands::speak,
            commands::configure_voice,
            commands::get_voice_config,
            commands::detect_voice_providers,
            commands::list_openai_voices,
            commands::list_openai_tts_models,
            commands::list_elevenlabs_voices,
            commands::list_available_voices,
            commands::queue_voice,
            commands::get_voice_queue,
            commands::cancel_voice,

            // Audio Cache Commands (TASK-005)
            commands::get_audio_cache_stats,
            commands::get_audio_cache_size,
            commands::clear_audio_cache,
            commands::clear_audio_cache_by_tag,
            commands::prune_audio_cache,
            commands::list_audio_cache_entries,

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
            commands::reorder_session,
            commands::get_campaign_stats,
            commands::generate_campaign_cover,
            commands::transcribe_audio,

            // Campaign Versioning Commands (TASK-006)
            commands::create_campaign_version,
            commands::list_campaign_versions,
            commands::get_campaign_version,
            commands::compare_campaign_versions,
            commands::rollback_campaign,
            commands::delete_campaign_version,
            commands::add_version_tag,
            commands::mark_version_milestone,

            // World State Commands (TASK-007)
            commands::get_world_state,
            commands::update_world_state,
            commands::set_in_game_date,
            commands::advance_in_game_date,
            commands::get_in_game_date,
            commands::add_world_event,
            commands::list_world_events,
            commands::delete_world_event,
            commands::set_location_state,
            commands::get_location_state,
            commands::list_locations,
            commands::update_location_condition,
            commands::set_world_custom_field,
            commands::get_world_custom_field,
            commands::list_world_custom_fields,
            commands::set_calendar_config,
            commands::get_calendar_config,

            // Entity Relationship Commands (TASK-009)
            commands::create_entity_relationship,
            commands::get_entity_relationship,
            commands::update_entity_relationship,
            commands::delete_entity_relationship,
            commands::list_entity_relationships,
            commands::get_relationships_for_entity,
            commands::get_relationships_between_entities,
            commands::get_entity_graph,
            commands::get_ego_graph,

            // TASK-022: Usage Tracking Commands
            commands::get_usage_stats,
            commands::get_usage_by_period,
            commands::get_cost_breakdown,
            commands::get_budget_status,
            commands::set_budget_limit,
            commands::get_provider_usage,
            commands::reset_usage_session,

            // TASK-023: Search Analytics Commands
            commands::get_search_analytics,
            commands::get_popular_queries,
            commands::get_cache_stats,
            commands::get_trending_queries,
            commands::get_zero_result_queries,
            commands::get_click_distribution,
            commands::record_search_selection,

            // TASK-024: Security Audit Commands
            commands::get_audit_logs,
            commands::query_audit_logs,
            commands::export_audit_logs,
            commands::clear_old_logs,
            commands::get_audit_summary,
            commands::get_security_events,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
