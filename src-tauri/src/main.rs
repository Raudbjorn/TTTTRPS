#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod native_features;

use ttrpg_assistant::commands;
use ttrpg_assistant::backstory_commands;
use ttrpg_assistant::ingestion;
use tauri::{Manager, RunEvent};
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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
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
            let (cm, sm, ns, creds, vm, sidecar_manager, search_client, personality_store, personality_manager, pipeline, _llm_router, version_manager, world_state_manager, relationship_manager, location_manager, llm_manager, claude_gate, gemini_gate, copilot_gate, setting_pack_loader,
                // Phase 4: Personality Extensions
                template_store, blend_rule_store, personality_blender, contextual_personality_manager) =
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

            // Start Meilisearch Sidecar (checks existing/PATH/downloads as needed)
            let sm_clone = sidecar_manager.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = sm_clone.start().await {
                    log::error!("Failed to start Meilisearch: {}", e);
                }
            });

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

            // Load persisted voice config or use default
            let voice_manager = if let Some(voice_config) = commands::load_voice_config_disk(app.handle()) {
                log::info!("Loading voice config from disk: provider={:?}", voice_config.provider);
                std::sync::Arc::new(tokio::sync::RwLock::new(
                    ttrpg_assistant::core::voice::VoiceManager::new(voice_config)
                ))
            } else {
                vm
            };

            app.manage(commands::AppState {
                llm_client: std::sync::RwLock::new(None),
                llm_config: std::sync::RwLock::new(commands::load_llm_config_disk(app.handle())),
                llm_router: tokio::sync::RwLock::new(ttrpg_assistant::core::llm::router::LLMRouter::with_defaults()),
                campaign_manager: cm,
                session_manager: sm,
                npc_store: ns,
                credentials: creds,
                voice_manager,
                sidecar_manager: sidecar_manager.clone(),
                search_client: search_client.clone(),
                personality_store,
                personality_manager,
                ingestion_pipeline: pipeline,
                database,
                version_manager,
                world_state_manager,
                relationship_manager,
                location_manager,
                llm_manager: llm_manager.clone(), // Clone for auto-configure block
                extraction_settings: tokio::sync::RwLock::new(
                    commands::load_extraction_config_disk(app.handle())
                        .unwrap_or_else(ingestion::ExtractionSettings::default)
                ),
                claude_gate,
                gemini_gate,
                copilot_gate,
                // Archetype Registry fields - initialized lazily after Meilisearch starts
                archetype_registry: tokio::sync::RwLock::new(None), // Initialized after Meilisearch is ready
                vocabulary_manager: tokio::sync::RwLock::new(None), // Initialized after Meilisearch is ready
                setting_pack_loader,
                // Phase 4: Personality Extensions
                template_store,
                blend_rule_store,
                personality_blender,
                contextual_personality_manager,
            });

            // Initialize Archetype Registry after Meilisearch starts
            let sc_for_archetype = search_client;
            let app_handle_for_archetype = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // Wait for Meilisearch to be ready
                if sc_for_archetype.wait_for_health(15).await {
                    // Get the Meilisearch config to create the client
                    let config = ttrpg_assistant::core::sidecar_manager::MeilisearchConfig::default();
                    let meili_client = match meilisearch_sdk::client::Client::new(
                        config.url(),
                        Some(&config.master_key),
                    ) {
                        Ok(client) => client,
                        Err(e) => {
                            log::error!("Failed to create Meilisearch client for archetypes: {}", e);
                            return;
                        }
                    };

                    // Initialize the archetype registry
                    match ttrpg_assistant::core::archetype::ArchetypeRegistry::new(meili_client.clone()).await {
                        Ok(registry) => {
                            log::info!("Archetype registry initialized");
                            // Update the AppState with the registry
                            if let Some(app_state) = app_handle_for_archetype.try_state::<commands::AppState>() {
                                *app_state.archetype_registry.write().await = Some(std::sync::Arc::new(registry));
                                log::info!("Archetype registry stored in AppState");
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to initialize archetype registry: {}", e);
                        }
                    }

                    // Initialize the vocabulary manager (not async)
                    let manager = ttrpg_assistant::core::archetype::VocabularyBankManager::with_meilisearch(meili_client);
                    let count = manager.count().await;
                    log::info!("Vocabulary manager initialized with {} banks", count);
                    // Update the AppState with the manager
                    if let Some(app_state) = app_handle_for_archetype.try_state::<commands::AppState>() {
                        *app_state.vocabulary_manager.write().await = Some(std::sync::Arc::new(manager));
                        log::info!("Vocabulary manager stored in AppState");
                    }
                } else {
                    log::warn!("Meilisearch not ready after 15 seconds - archetype registry not initialized");
                }
            });


            // Initialize Meilisearch Chat Client (fixes "Meilisearch chat client not configured" error)
            let sidecar_config = sidecar_manager.config().clone();
            let llm_manager_clone = llm_manager.clone();
            tauri::async_runtime::spawn(async move {
                // Wait for Meilisearch to start by polling health endpoint
                // Wait up to 30 seconds
                for _ in 0..30 {
                    if sidecar_manager.health_check().await {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }

                llm_manager_clone.write().await.set_chat_client(
                    &sidecar_config.url(),
                    Some(&sidecar_config.master_key),
                ).await;
            });

            // Auto-configure Ollama if no providers are present (User Request)
            let handle_clone = handle.clone();
            tauri::async_runtime::spawn(async move {
                // Wait briefly for startup
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;

                if let Some(app_state) = handle_clone.try_state::<commands::AppState>() {
                    // Check for persisted config
                    let config_opt = app_state.llm_config.read().unwrap().clone();

                    if let Some(config) = config_opt {
                        log::info!("Applying persisted LLM configuration...");
                        // Add to router (if not already present? Router is fresh here)
                        // Note: create_provider creates a new instance.
                        let provider = config.create_provider();
                        app_state.llm_router.write().await.add_provider(provider).await;

                        // Configure Meilisearch (retry loop to wait for Sidecar/Client init)
                        let manager = app_state.llm_manager.clone();
                        let config_clone = config.clone();
                        tokio::spawn(async move {
                            for i in 0..15 {
                                let res = manager.write().await.configure_for_chat(&config_clone, None).await;
                                if res.is_ok() {
                                    log::info!("Meilisearch chat configured from persistence.");
                                    break;
                                }
                                if i == 14 {
                                    log::warn!("Failed to configure Meilisearch from persistence after retries: {:?}", res.err());
                                }
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            }
                        });
                    } else {
                        // Fallback: Auto-configure Ollama if no config exists
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
            commands::stream_chat,
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

            // Campaign Wizard Commands (Phase 2 - Campaign Generation Overhaul)
            commands::start_campaign_wizard,
            commands::get_wizard_state,
            commands::list_incomplete_wizards,
            commands::delete_wizard,
            commands::advance_wizard_step,
            commands::wizard_go_back,
            commands::wizard_skip_step,
            commands::update_wizard_draft,
            commands::complete_wizard,
            commands::cancel_wizard,
            commands::auto_save_wizard,
            commands::link_wizard_conversation,

            // Conversation Commands (Phase 5 - Campaign Generation Overhaul)
            commands::create_conversation_thread,
            commands::get_conversation_thread,
            commands::list_conversation_threads,
            commands::archive_conversation_thread,
            commands::update_conversation_thread_title,
            commands::send_conversation_message,
            commands::get_conversation_messages,
            commands::add_conversation_message,
            commands::accept_suggestion,
            commands::reject_suggestion,
            commands::get_pending_suggestions,
            commands::branch_conversation,
            commands::generate_clarifying_questions,

            // Session Commands
            commands::start_session,
            commands::get_session,
            commands::get_active_session,
            commands::list_sessions,
            commands::create_planned_session,
            commands::start_planned_session,
            commands::end_session,

            // Global Chat Session Commands (Persistent LLM Chat History)
            commands::get_or_create_chat_session,
            commands::get_active_chat_session,
            commands::get_chat_messages,
            commands::add_chat_message,
            commands::update_chat_message,
            commands::link_chat_to_game_session,
            commands::end_chat_session_and_spawn_new,
            commands::clear_chat_messages,
            commands::list_chat_sessions,
            commands::get_chat_sessions_for_game,

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
            commands::ingest_document_two_phase,
            commands::import_layout_json,
            commands::list_library_documents,
            commands::delete_library_document,
            commands::update_library_document,
            commands::rebuild_library_metadata,
            commands::clear_and_reingest_document,
            commands::ingest_pdf,
            commands::search,
            commands::check_meilisearch_health,
            commands::reindex_library,
            commands::get_vector_store_status,
            commands::configure_meilisearch_embedder,
            commands::setup_ollama_embeddings,
            commands::setup_copilot_embeddings,
            commands::get_embedder_status,
            commands::list_ollama_embedding_models,
            commands::list_local_embedding_models,
            commands::setup_local_embeddings,

            // Voice Commands
            commands::speak,
            commands::configure_voice,
            commands::get_voice_config,
            commands::detect_voice_providers,
            commands::check_voice_provider_installations,
            commands::check_voice_provider_status,
            commands::install_voice_provider,
            commands::list_downloadable_piper_voices,
            commands::get_popular_piper_voices,
            commands::download_piper_voice,
            commands::list_openai_voices,
            commands::list_openai_tts_models,
            commands::list_elevenlabs_voices,
            commands::list_available_voices,
            commands::queue_voice,
            commands::get_voice_queue,
            commands::cancel_voice,
            commands::play_tts,
            commands::list_all_voices,

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

            // Meilisearch Chat Provider Commands
            commands::list_chat_providers,
            commands::configure_chat_workspace,
            commands::get_chat_workspace_settings,
            commands::configure_meilisearch_chat,

            // Model Selection Commands
            commands::get_model_selection,
            commands::get_model_selection_for_prompt,
            commands::set_model_override,

            // TTRPG Document Commands
            commands::list_ttrpg_documents_by_source,
            commands::list_ttrpg_documents_by_type,
            commands::list_ttrpg_documents_by_system,
            commands::search_ttrpg_documents_by_name,
            commands::list_ttrpg_documents_by_cr,
            commands::get_ttrpg_document,
            commands::get_ttrpg_document_attributes,
            commands::find_ttrpg_documents_by_attribute,
            commands::delete_ttrpg_document,
            commands::get_ttrpg_document_stats,
            commands::count_ttrpg_documents_by_type,
            commands::get_ttrpg_ingestion_job,
            commands::get_ttrpg_ingestion_job_by_document,
            commands::list_pending_ttrpg_ingestion_jobs,
            commands::list_active_ttrpg_ingestion_jobs,

            // Extraction Settings Commands
            commands::get_extraction_settings,
            commands::save_extraction_settings,
            commands::get_supported_formats,
            commands::get_extraction_presets,
            commands::check_ocr_availability,

            // Claude Gate OAuth Commands
            commands::oauth::claude::claude_gate_get_status,
            commands::oauth::claude::claude_gate_start_oauth,
            commands::oauth::claude::claude_gate_complete_oauth,
            commands::oauth::claude::claude_gate_logout,
            commands::oauth::claude::claude_gate_set_storage_backend,
            commands::oauth::claude::claude_gate_list_models,

            // Gemini OAuth Commands
            commands::oauth::gemini::gemini_gate_get_status,
            commands::oauth::gemini::gemini_gate_start_oauth,
            commands::oauth::gemini::gemini_gate_complete_oauth,
            commands::oauth::gemini::gemini_gate_logout,
            commands::oauth::gemini::gemini_gate_set_storage_backend,
            commands::oauth::gemini::gemini_gate_oauth_with_callback,
            commands::oauth::gemini::gemini_gate_list_models,

            // Copilot OAuth Commands (Device Code Flow)
            commands::oauth::copilot::start_copilot_auth,
            commands::oauth::copilot::poll_copilot_auth,
            commands::oauth::copilot::check_copilot_auth,
            commands::oauth::copilot::logout_copilot,
            commands::oauth::copilot::get_copilot_usage,
            commands::oauth::copilot::get_copilot_models,
            commands::oauth::copilot::copilot_gate_set_storage_backend,

            // Phase 4: Personality Extension Commands (TASK-PERS-014, TASK-PERS-015, TASK-PERS-016, TASK-PERS-017)
            // Template Commands
            commands::list_personality_templates,
            commands::filter_templates_by_game_system,
            commands::filter_templates_by_setting,
            commands::search_personality_templates,
            commands::get_template_preview,
            commands::apply_template_to_campaign,
            commands::create_template_from_personality,
            commands::export_personality_template,
            commands::import_personality_template,
            // Blend Rule Commands
            commands::set_blend_rule,
            commands::get_blend_rule,
            commands::list_blend_rules,
            commands::delete_blend_rule,
            // Context Detection Commands
            commands::detect_gameplay_context,
            commands::list_gameplay_contexts,
            // Contextual Personality Commands
            commands::get_contextual_personality,
            commands::get_current_context,
            commands::clear_context_history,
            commands::get_contextual_personality_config,
            commands::set_contextual_personality_config,
            commands::get_blender_cache_stats,
            commands::get_blend_rule_cache_stats,

            // Utility Commands
            commands::open_url_in_browser,

            // Archetype Registry Commands (TASK-ARCH-060)
            commands::archetype::crud::create_archetype,
            commands::archetype::crud::get_archetype,
            commands::archetype::crud::list_archetypes,
            commands::archetype::crud::update_archetype,
            commands::archetype::crud::delete_archetype,
            commands::archetype::crud::archetype_exists,
            commands::archetype::crud::count_archetypes,

            // Vocabulary Bank Commands (TASK-ARCH-061)
            commands::archetype::vocabulary::create_vocabulary_bank,
            commands::archetype::vocabulary::get_vocabulary_bank,
            commands::archetype::vocabulary::list_vocabulary_banks,
            commands::archetype::vocabulary::update_vocabulary_bank,
            commands::archetype::vocabulary::delete_vocabulary_bank,
            commands::archetype::vocabulary::get_phrases,

            // Setting Pack Commands (TASK-ARCH-062)
            commands::archetype::setting_packs::load_setting_pack,
            commands::archetype::setting_packs::list_setting_packs,
            commands::archetype::setting_packs::get_setting_pack,
            commands::archetype::setting_packs::activate_setting_pack,
            commands::archetype::setting_packs::deactivate_setting_pack,
            commands::archetype::setting_packs::get_active_setting_pack,
            commands::archetype::setting_packs::get_setting_pack_versions,

            // Archetype Resolution Commands (TASK-ARCH-063)
            commands::archetype::resolution::resolve_archetype,
            commands::archetype::resolution::resolve_for_npc,
            commands::archetype::resolution::get_archetype_cache_stats,
            commands::archetype::resolution::clear_archetype_cache,
            commands::archetype::resolution::is_archetype_registry_ready,

            // Quick Reference Card Commands (Phase 9 - Campaign Generation Overhaul)
            commands::get_entity_card,
            commands::get_hover_preview,
            commands::get_pinned_cards,
            commands::pin_card,
            commands::unpin_card,
            commands::reorder_pinned_cards,
            commands::update_card_disclosure,
            commands::get_max_pinned_cards,
            commands::build_cheat_sheet,
            commands::build_custom_cheat_sheet,
            commands::export_cheat_sheet_html,
            commands::save_cheat_sheet_preference,
            commands::get_cheat_sheet_preferences,
            commands::delete_cheat_sheet_preference,
            commands::invalidate_card_cache,
            commands::cleanup_card_cache,
            commands::list_card_entity_types,
            commands::list_disclosure_levels,
            commands::list_cheat_sheet_sections,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let RunEvent::ExitRequested { .. } = event {
                // Gracefully stop the LLM proxy service without blocking the event loop
                if let Some(app_state) = app_handle.try_state::<commands::AppState>() {
                    let llm_manager = app_state.llm_manager.clone();
                    tauri::async_runtime::spawn(async move {
                        log::info!("Shutting down LLM proxy service...");
                        llm_manager.write().await.stop_proxy().await;
                        log::info!("LLM proxy service stopped");
                    });
                }
            }
        });
}
