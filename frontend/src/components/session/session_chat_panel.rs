//! Session Chat Panel Component
//!
//! Phase 8: Campaign-aware chat panel for session workspace.
//! Integrates thread tabs with streaming chat, campaign context, and purpose-driven prompts.

use leptos::prelude::*;
use leptos::ev;
use wasm_bindgen_futures::spawn_local;

use crate::bindings::{
    ConversationThread, ConversationPurpose,
    stream_chat, cancel_stream, listen_chat_chunks_async,
    ChatChunk, StreamingChatMessage,
    get_or_create_chat_session, get_chat_messages, add_chat_message, update_chat_message,
};
use crate::components::design_system::{Button, ButtonVariant, Input};
use crate::components::chat::ChatMessage;
use crate::components::session::thread_tabs::ThreadTabs;
use crate::services::notification_service::show_error;
use crate::services::chat_context::try_use_chat_context;

/// Message in the chat history (mirrors main Chat component)
#[derive(Clone, PartialEq)]
struct Message {
    id: usize,
    role: String,
    content: String,
    is_streaming: bool,
    stream_id: Option<String>,
    persistent_id: Option<String>,
}

/// Pending finalization data for race condition handling
#[derive(Clone)]
struct PendingFinalization {
    message_id: usize,
    final_content: String,
}

/// Session chat panel for campaign workspace
#[component]
pub fn SessionChatPanel(
    /// Campaign ID for this session
    #[prop(into)]
    campaign_id: Signal<Option<String>>,
) -> impl IntoView {
    // Thread state
    let selected_thread = RwSignal::new(Option::<ConversationThread>::None);
    let selected_thread_id = RwSignal::new(Option::<String>::None);

    // Chat state
    let message_input = RwSignal::new(String::new());
    let messages = RwSignal::new(Vec::<Message>::new());
    let is_loading = RwSignal::new(false);
    let is_loading_history = RwSignal::new(false);
    let next_message_id = RwSignal::new(1_usize);
    let chat_session_id = RwSignal::new(Option::<String>::None);

    // Streaming state
    let current_stream_id = RwSignal::new(Option::<String>::None);
    let streaming_message_id = RwSignal::new(Option::<usize>::None);
    let streaming_persistent_id = RwSignal::new(Option::<String>::None);

    // Fix #2: Pending finalization for race condition handling
    // When stream finishes before persistent_id is set, store content here
    let pending_finalization = RwSignal::new(Option::<PendingFinalization>::None);

    // Fix #4: Load/reload chat when thread changes - use thread-specific session
    Effect::new(move |_| {
        let thread = selected_thread.get();
        is_loading_history.set(true);
        messages.set(Vec::new());
        chat_session_id.set(None);

        // Cancel any in-progress stream when switching threads
        if let Some(stream_id) = current_stream_id.get_untracked() {
            spawn_local(async move {
                let _ = cancel_stream(stream_id).await;
            });
            current_stream_id.set(None);
            streaming_message_id.set(None);
            is_loading.set(false);
        }

        // Clear streaming state
        streaming_persistent_id.set(None);
        pending_finalization.set(None);

        spawn_local(async move {
            // Get or create a chat session (future: pass thread_id for thread-specific sessions)
            match get_or_create_chat_session().await {
                Ok(session) => {
                    chat_session_id.set(Some(session.id.clone()));

                    // Load messages for this session
                    // Note: In future, this should filter by thread_id
                    match get_chat_messages(session.id, Some(50)).await {
                        Ok(stored) => {
                            let ui_messages: Vec<Message> = stored
                                .iter()
                                .enumerate()
                                .map(|(idx, m)| Message {
                                    id: idx,
                                    role: m.role.clone(),
                                    content: m.content.clone(),
                                    is_streaming: m.is_streaming != 0,
                                    stream_id: None,
                                    persistent_id: Some(m.id.clone()),
                                })
                                .collect();
                            next_message_id.set(ui_messages.len());
                            messages.set(ui_messages);
                        }
                        Err(e) => {
                            log::error!("Failed to load messages: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to get chat session: {}", e);
                }
            }
            is_loading_history.set(false);

            // Log thread context for debugging
            if let Some(t) = thread {
                log::info!("Loaded chat for thread: {} ({:?})", t.display_title(), t.purpose);
            }
        });
    });

    // Fix #2: Effect to handle pending finalization when persistent_id arrives
    Effect::new(move |_| {
        let pid = streaming_persistent_id.get();
        let pending = pending_finalization.get();

        if let (Some(persistent_id), Some(finalization)) = (pid, pending) {
            // Update the message with persistent_id
            messages.update(|msgs| {
                if let Some(msg) = msgs.iter_mut().find(|m| m.id == finalization.message_id) {
                    msg.persistent_id = Some(persistent_id.clone());
                }
            });

            // Now persist the final content
            let content = finalization.final_content.clone();
            spawn_local(async move {
                if let Err(e) = update_chat_message(persistent_id, content, None, false).await {
                    log::error!("Failed to persist deferred message: {}", e);
                }
            });

            // Clear pending state
            pending_finalization.set(None);
            streaming_persistent_id.set(None);
        }
    });

    // Set up streaming chunk listener
    // Note: The unlisten handle is intentionally not stored for cleanup because:
    // 1. JsValue isn't Send+Sync, so can't be stored in Leptos signals
    // 2. on_cleanup requires Send+Sync closures
    // 3. The stream_id filtering ensures only relevant chunks are processed
    // 4. The listener is cleaned up when the window closes
    {
        spawn_local(async move {
            let _unlisten = listen_chat_chunks_async(move |chunk: ChatChunk| {
                let result = messages.try_update(|msgs| {
                    if let Some(msg) = msgs.iter_mut().find(|m| {
                        m.stream_id.as_ref() == Some(&chunk.stream_id) && m.is_streaming
                    }) {
                        if !chunk.content.is_empty() {
                            msg.content.push_str(&chunk.content);
                        }

                        if chunk.is_final {
                            msg.is_streaming = false;
                            msg.stream_id = None;

                            if chunk.finish_reason.as_deref() == Some("error") {
                                msg.role = "error".to_string();
                            }
                        }
                        true
                    } else {
                        false
                    }
                });

                if let Some(true) = result {
                    if chunk.is_final {
                        let _ = is_loading.try_set(false);
                        let _ = current_stream_id.try_set(None);

                        // Get the message id and content for finalization
                        let msg_id = streaming_message_id.get_untracked();
                        let _ = streaming_message_id.try_set(None);

                        // Fix #2: Check if persistent_id is available
                        if let Some(pid) = streaming_persistent_id.get_untracked() {
                            // Persistent ID is ready - persist immediately
                            let final_content = messages.try_with(|msgs| {
                                msgs.iter()
                                    .find(|m| m.persistent_id.as_ref() == Some(&pid))
                                    .map(|m| m.content.clone())
                            });

                            if let Some(Some(content)) = final_content {
                                let pid_clone = pid.clone();
                                spawn_local(async move {
                                    if let Err(e) = update_chat_message(pid_clone, content, None, false).await {
                                        log::error!("Failed to persist message: {}", e);
                                    }
                                });
                            }
                            let _ = streaming_persistent_id.try_set(None);
                        } else if let Some(mid) = msg_id {
                            // Fix #2: Race condition - persistent_id not ready yet
                            // Store content for deferred persistence
                            let final_content = messages.try_with(|msgs| {
                                msgs.iter()
                                    .find(|m| m.id == mid)
                                    .map(|m| m.content.clone())
                            });

                            if let Some(Some(content)) = final_content {
                                log::info!("Deferring finalization for message {} (persistent_id not ready)", mid);
                                let _ = pending_finalization.try_set(Some(PendingFinalization {
                                    message_id: mid,
                                    final_content: content,
                                }));
                            }
                        }
                    }
                }
            }).await;
        });
    }

    // Thread selection handler
    let on_thread_select = Callback::new(move |thread: Option<ConversationThread>| {
        selected_thread_id.set(thread.as_ref().map(|t| t.id.clone()));
        selected_thread.set(thread);
    });

    // Build system prompt based on thread purpose
    let build_system_prompt = move || {
        let thread = selected_thread.get();
        let purpose = thread.as_ref().map(|t| t.purpose).unwrap_or(ConversationPurpose::General);

        let base_prompt = match purpose {
            ConversationPurpose::SessionPlanning => {
                "You are a TTRPG session planning assistant. Help the GM plan engaging sessions \
                 with plot hooks, encounter ideas, NPC interactions, and pacing suggestions. \
                 Focus on practical advice that can be used at the table."
            }
            ConversationPurpose::NpcGeneration => {
                "You are an NPC creation specialist for TTRPGs. Help create memorable, \
                 believable NPCs with distinct personalities, motivations, secrets, and \
                 speaking styles. Include quirks and roleplaying hooks."
            }
            ConversationPurpose::WorldBuilding => {
                "You are a world-building consultant for TTRPGs. Help develop rich settings, \
                 locations, cultures, histories, and lore. Focus on details that enhance \
                 gameplay and create immersion."
            }
            ConversationPurpose::CharacterBackground => {
                "You are a character backstory specialist. Help players and GMs develop \
                 compelling character backgrounds, motivations, connections, and story hooks \
                 that integrate well with the campaign setting."
            }
            ConversationPurpose::CampaignCreation => {
                "You are a campaign architect for TTRPGs. Help design overarching campaign \
                 structures, major story arcs, recurring themes, and long-term plot threads \
                 that create satisfying narrative journeys."
            }
            ConversationPurpose::General => {
                "You are a TTRPG assistant helping a Game Master run engaging tabletop sessions. \
                 You have expertise in narrative design, encounter balancing, improvisation, \
                 and player engagement. Be helpful, creative, and supportive."
            }
        };

        // Add campaign context if available
        if let Some(chat_ctx) = try_use_chat_context() {
            if let Some(augmentation) = chat_ctx.build_prompt_augmentation() {
                Some(format!("{}{}", base_prompt, augmentation))
            } else {
                Some(base_prompt.to_string())
            }
        } else {
            Some(base_prompt.to_string())
        }
    };

    // Cancel stream handler
    let cancel_current_stream = move || {
        if let Some(stream_id) = current_stream_id.get() {
            spawn_local(async move {
                let _ = cancel_stream(stream_id).await;
            });

            if let Some(msg_id) = streaming_message_id.get() {
                messages.update(|msgs| {
                    if let Some(msg) = msgs.iter_mut().find(|m| m.id == msg_id) {
                        msg.is_streaming = false;
                        msg.stream_id = None;
                        if msg.content.is_empty() {
                            msg.content = "[Canceled]".to_string();
                        } else {
                            msg.content.push_str("\n\n[Canceled]");
                        }
                    }
                });
            }

            is_loading.set(false);
            current_stream_id.set(None);
            streaming_message_id.set(None);
        }
    };

    // Send message handler
    let send_message = move || {
        let msg = message_input.get();
        if msg.trim().is_empty() || is_loading.get() {
            return;
        }

        let session_id = match chat_session_id.get() {
            Some(id) => id,
            None => {
                show_error("Chat Not Ready", Some("Please wait for the session to load."), None);
                return;
            }
        };

        // Add user message
        let user_msg_id = next_message_id.get();
        next_message_id.set(user_msg_id + 1);
        messages.update(|msgs| {
            msgs.push(Message {
                id: user_msg_id,
                role: "user".to_string(),
                content: msg.clone(),
                is_streaming: false,
                stream_id: None,
                persistent_id: None,
            });
        });

        // Persist user message
        {
            let sid = session_id.clone();
            let content = msg.clone();
            spawn_local(async move {
                if let Err(e) = add_chat_message(sid, "user".to_string(), content, None).await {
                    log::error!("Failed to persist user message: {}", e);
                }
            });
        }

        // Add assistant placeholder
        let assistant_msg_id = next_message_id.get();
        next_message_id.set(assistant_msg_id + 1);
        messages.update(|msgs| {
            msgs.push(Message {
                id: assistant_msg_id,
                role: "assistant".to_string(),
                content: String::new(),
                is_streaming: true,
                stream_id: None,
                persistent_id: None,
            });
        });

        // Persist placeholder
        {
            let sid = session_id;
            spawn_local(async move {
                match add_chat_message(sid, "assistant".to_string(), String::new(), None).await {
                    Ok(record) => {
                        let pid = record.id.clone();
                        streaming_persistent_id.set(Some(record.id));
                        messages.update(|msgs| {
                            if let Some(msg) = msgs.iter_mut().find(|m| m.id == assistant_msg_id) {
                                msg.persistent_id = Some(pid);
                            }
                        });
                    }
                    Err(e) => log::error!("Failed to persist placeholder: {}", e),
                }
            });
        }

        message_input.set(String::new());
        is_loading.set(true);

        let stream_id = uuid::Uuid::new_v4().to_string();
        current_stream_id.set(Some(stream_id.clone()));
        streaming_message_id.set(Some(assistant_msg_id));

        messages.update(|msgs| {
            if let Some(msg) = msgs.iter_mut().find(|m| m.id == assistant_msg_id) {
                msg.stream_id = Some(stream_id.clone());
            }
        });

        // Build history
        let history: Vec<StreamingChatMessage> = messages.get().iter()
            .filter(|m| m.role == "user" || m.role == "assistant")
            .filter(|m| m.id != assistant_msg_id)
            .map(|m| StreamingChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let system_prompt = build_system_prompt();
        let stream_id_for_call = stream_id;

        spawn_local(async move {
            match stream_chat(history, system_prompt, None, None, Some(stream_id_for_call)).await {
                Ok(_) => {}
                Err(e) => {
                    messages.update(|msgs| {
                        if let Some(msg) = msgs.iter_mut().find(|m| m.id == assistant_msg_id) {
                            msg.role = "error".to_string();
                            msg.content = format!("Error: {}", e);
                            msg.is_streaming = false;
                        }
                    });
                    is_loading.set(false);
                    streaming_message_id.set(None);
                    current_stream_id.set(None);
                    show_error("Streaming Failed", Some(&e), None);
                }
            }
        });
    };

    let on_send_click = move |_: ev::MouseEvent| send_message();
    let on_cancel_click = move |_: ev::MouseEvent| cancel_current_stream();

    let on_keydown = Callback::new(move |e: ev::KeyboardEvent| {
        if e.key() == "Enter" && !e.shift_key() {
            e.prevent_default();
            send_message();
        }
        if e.key() == "Escape" && is_loading.get() {
            e.prevent_default();
            cancel_current_stream();
        }
    });

    view! {
        <div class="flex flex-col h-full bg-zinc-900 rounded-lg border border-zinc-800 overflow-hidden">
            // Thread Tabs
            <ThreadTabs
                campaign_id=campaign_id
                selected_thread_id=selected_thread_id
                on_select=on_thread_select
            />

            // Thread context indicator
            <Show when=move || selected_thread.get().is_some()>
                {move || {
                    if let Some(thread) = selected_thread.get() {
                        view! {
                            <div class="px-3 py-1.5 bg-zinc-800/50 border-b border-zinc-700 flex items-center gap-2 text-xs">
                                <span class="text-zinc-500">{thread.purpose.icon()}</span>
                                <span class="text-zinc-400">{thread.display_title()}</span>
                                <span class="text-zinc-600">{format!("{} messages", thread.message_count)}</span>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}
            </Show>

            // Messages area
            <div class="flex-1 overflow-y-auto p-3 space-y-3">
                <Show
                    when=move || !is_loading_history.get()
                    fallback=|| view! {
                        <div class="flex items-center justify-center h-24 text-zinc-500">
                            <div class="flex items-center gap-2">
                                <div class="w-3 h-3 border-2 border-zinc-600 border-t-transparent rounded-full animate-spin"></div>
                                <span class="text-sm">"Loading..."</span>
                            </div>
                        </div>
                    }
                >
                    <Show
                        when=move || !messages.get().is_empty()
                        fallback=move || {
                            let thread = selected_thread.get();
                            let hint = match thread.as_ref().map(|t| t.purpose) {
                                Some(ConversationPurpose::SessionPlanning) => "Ask for help planning your next session...",
                                Some(ConversationPurpose::NpcGeneration) => "Describe an NPC you need to create...",
                                Some(ConversationPurpose::WorldBuilding) => "Ask about locations, cultures, or lore...",
                                _ => "Ask the DM assistant anything...",
                            };
                            view! {
                                <div class="flex items-center justify-center h-24 text-zinc-500 text-sm">
                                    {hint}
                                </div>
                            }
                        }
                    >
                        <For
                            each=move || messages.get()
                            key=|msg| (msg.id, msg.content.len(), msg.is_streaming)
                            children=move |msg| {
                                view! {
                                    <ChatMessage
                                        role=msg.role.clone()
                                        content=msg.content.clone()
                                        tokens=None
                                        is_streaming=msg.is_streaming
                                        on_play=None
                                        show_tokens=false
                                    />
                                }
                            }
                        />
                    </Show>
                </Show>
            </div>

            // Input area
            <div class="p-3 border-t border-zinc-800 bg-zinc-900/80">
                <div class="flex gap-2">
                    <div class="flex-1">
                        <Input
                            value=message_input
                            placeholder="Type a message... (Esc to cancel)"
                            disabled=Signal::derive(move || is_loading.get() || is_loading_history.get())
                            on_keydown=on_keydown
                        />
                    </div>
                    {move || {
                        if is_loading_history.get() {
                            view! {
                                <Button
                                    variant=ButtonVariant::Secondary
                                    on_click=move |_: ev::MouseEvent| {}
                                    disabled=true
                                    class="opacity-50"
                                >
                                    "..."
                                </Button>
                            }.into_any()
                        } else if is_loading.get() {
                            view! {
                                <Button
                                    variant=ButtonVariant::Secondary
                                    on_click=on_cancel_click
                                    class="bg-red-900/50 hover:bg-red-800/50 border-red-700/50"
                                >
                                    "Stop"
                                </Button>
                            }.into_any()
                        } else {
                            view! {
                                <Button on_click=on_send_click>
                                    "Send"
                                </Button>
                            }.into_any()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
