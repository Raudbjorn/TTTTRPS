pub mod chat_message;
pub mod personality_selector;

pub use chat_message::ChatMessage;
pub use personality_selector::{PersonalitySelector, PersonalityIndicator};

use leptos::ev;
use leptos::prelude::*;
use leptos_router::components::A;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use std::sync::Arc;
use crate::services::notification_service::{show_error, ToastAction};
use crate::services::layout_service::use_layout_state;

use crate::bindings::{
    cancel_stream, chat, check_llm_health, get_session_usage, listen_chat_chunks_async, stream_chat,
    ChatChunk, ChatRequestPayload, SessionUsage, StreamingChatMessage, speak,
};
use crate::components::design_system::{Button, ButtonVariant, Input};

/// Message in the chat history
#[derive(Clone, PartialEq)]
pub struct Message {
    pub id: usize,
    pub role: String,
    pub content: String,
    pub tokens: Option<(u32, u32)>,
    /// Whether this message is currently being streamed
    pub is_streaming: bool,
    /// Stream ID for cancellation (only set for streaming messages)
    pub stream_id: Option<String>,
}

/// Main Chat component - the primary DM interface with streaming support
#[component]
pub fn Chat() -> impl IntoView {
    // State signals
    let message_input = RwSignal::new(String::new());
    let messages = RwSignal::new(vec![Message {
        id: 0,
        role: "assistant".to_string(),
        content: "Welcome to Sidecar DM! I'm your AI-powered TTRPG assistant. Configure an LLM provider in Settings to get started.".to_string(),
        tokens: None,
        is_streaming: false,
        stream_id: None,
    }]);
    let is_loading = RwSignal::new(false);
    let llm_status = RwSignal::new("Checking...".to_string());
    let session_usage = RwSignal::new(SessionUsage {
        session_input_tokens: 0,
        session_output_tokens: 0,
        session_requests: 0,
        session_cost_usd: 0.0,
    });
    let show_usage_panel = RwSignal::new(false);
    let next_message_id = RwSignal::new(1_usize);

    // Track the current streaming message ID and stream ID
    // Using RwSignal for UI reactivity
    let current_stream_id = RwSignal::new(Option::<String>::None);
    let streaming_message_id = RwSignal::new(Option::<usize>::None);

    // Shared health check logic
    // Trigger for health check retry
    let health_trigger = Trigger::new();

    // Shared health check logic (moved directly into effect for easier trigger usage)
    // or we can keep the closure if we prefer, but effect + trigger is cleaner for retry recursion
    let check_health = {
        let llm_status = llm_status;
        Arc::new(move || {
            let llm_status = llm_status;
            spawn_local(async move {
                llm_status.set("Checking...".to_string());
                match check_llm_health().await {
                    Ok(status) => {
                        if status.healthy {
                            llm_status.set(format!("{} connected", status.provider));
                        } else {
                            llm_status.set(format!("{}: {}", status.provider, status.message));
                            show_error("LLM Issue", Some(&status.message), None);
                        }
                    }
                    Err(e) => {
                        llm_status.set(format!("Error: {}", e));
                        let retry = Some(ToastAction {
                            label: "Retry".to_string(),
                            handler: Arc::new(move || health_trigger.notify()),
                        });
                        show_error(
                            "LLM Connection Error",
                            Some(&format!("Could not connect: {}", e)),
                            retry
                        );
                    }
                }
            });
        })
    };

    // Check LLM health on mount
    // Check LLM health on mount and refresh
    Effect::new({
        let check = check_health.clone();
        move |_| {
            health_trigger.track();
            (check)();
        }
    });

    // Set up streaming chunk listener on mount
    // Note: Tauri 2's listen() is async, so we spawn to await it
    // IMPORTANT: Find streaming message by stream_id in the messages list itself,
    // rather than relying on StoredValue which doesn't survive component remounts
    //
    // Cleanup note: JsValue is !Send, so we can't use on_cleanup with it directly.
    // This is safe because:
    // 1. try_update/try_set return None when signals are disposed (no crashes)
    // 2. Each listener matches by unique stream_id, so old listeners won't interfere
    // 3. Tauri cleans up event listeners when the webview closes
    {
        spawn_local(async move {
            let _unlisten = listen_chat_chunks_async(move |chunk: ChatChunk| {
                // Find the message that matches this stream_id directly in the messages list
                // This approach works even if component was remounted
                let result = messages.try_update(|msgs| {
                    // Debug: show what messages we have
                    #[cfg(debug_assertions)]
                    {
                        web_sys::console::log_1(&format!(
                            "[DEBUG] Searching {} messages for stream_id={}",
                            msgs.len(), &chunk.stream_id
                        ).into());
                        for (i, m) in msgs.iter().enumerate() {
                            web_sys::console::log_1(&format!(
                                "[DEBUG]   msg[{}]: id={}, stream_id={:?}, is_streaming={}",
                                i, m.id, m.stream_id, m.is_streaming
                            ).into());
                        }
                    }

                    // Find message by stream_id (set when message was created)
                    if let Some(msg) = msgs.iter_mut().find(|m| {
                        m.stream_id.as_ref() == Some(&chunk.stream_id) && m.is_streaming
                    }) {
                        #[cfg(debug_assertions)]
                        web_sys::console::log_1(&format!(
                            "[DEBUG] Found streaming message id={}, appending '{}' (len={})",
                            msg.id, &chunk.content, chunk.content.len()
                        ).into());

                        // Append content
                        if !chunk.content.is_empty() {
                            msg.content.push_str(&chunk.content);
                        }

                        // Handle stream completion
                        if chunk.is_final {
                            msg.is_streaming = false;
                            msg.stream_id = None;

                            // Mark as error if finish_reason is "error"
                            if chunk.finish_reason.as_deref() == Some("error") {
                                msg.role = "error".to_string();
                            }

                            // Set token usage if available
                            if let Some(usage) = &chunk.usage {
                                msg.tokens = Some((usage.input_tokens, usage.output_tokens));
                            }
                        }
                        true // Found and processed
                    } else {
                        false // Not found
                    }
                });

                match result {
                    Some(true) => {
                        // Successfully processed
                        if chunk.is_final {
                            // Clear loading state
                            let _ = is_loading.try_set(false);
                            let _ = current_stream_id.try_set(None);
                            let _ = streaming_message_id.try_set(None);

                            // Update session usage
                            spawn_local(async move {
                                if let Ok(usage) = get_session_usage().await {
                                    let _ = session_usage.try_set(usage);
                                }
                            });
                        }
                    }
                    Some(false) => {
                        // Message not found - might be for a different stream or old chunk
                        #[cfg(debug_assertions)]
                        web_sys::console::warn_1(&format!(
                            "[DEBUG] No streaming message found for stream_id={}",
                            &chunk.stream_id
                        ).into());
                    }
                    None => {
                        // Signal disposed - component unmounted, listener will be gc'd
                        #[cfg(debug_assertions)]
                        web_sys::console::warn_1(&"[DEBUG] messages signal disposed".into());
                    }
                }
            }).await;
        });
    }

    // Play message via TTS
    let play_message = move |text: String| {
        let messages = messages;
        let next_id = next_message_id;
        spawn_local(async move {
            // For now, use the default voice logic via play_tts with an empty ID
            // to trigger default behavior, or use the `speak` command if it does that.
            // The existing `speak` command likely uses the active provider's default.
            // My new `play_tts` expects a voice_id.
            // If I pass an empty string, does VoiceManager handle it?
            // VoiceManager::synthesize checks prefix. Empty string matches none.
            // Falls back to `get_active_provider_id`.
            // So `play_tts(text, "")` should equivalent to `speak(text)`.
            // Let's use `play_tts` to be consistent with new API.

            use crate::bindings::play_tts;

            match play_tts(text, "".to_string()).await {
                Ok(_) => {} // Success, audio playing
                Err(e) => {
                    let id = next_id.get();
                    next_id.set(id + 1);
                    messages.update(|msgs| {
                        msgs.push(Message {
                            id,
                            role: "error".to_string(),
                            content: format!("Voice Error: {}", e),
                            tokens: None,
                            is_streaming: false,
                            stream_id: None,
                        });
                    });
                }
            }
        });
    };

    // Cancel the current stream
    let cancel_current_stream = move || {
        if let Some(stream_id) = current_stream_id.get() {
            let stream_id_clone = stream_id.clone();
            spawn_local(async move {
                let _ = cancel_stream(stream_id_clone).await;
            });

            // Mark the message as cancelled
            if let Some(msg_id) = streaming_message_id.get() {
                messages.update(|msgs| {
                    if let Some(msg) = msgs.iter_mut().find(|m| m.id == msg_id) {
                        msg.is_streaming = false;
                        msg.stream_id = None;
                        if msg.content.is_empty() {
                            msg.content = "[Response cancelled]".to_string();
                        } else {
                            msg.content.push_str("\n\n[Stream cancelled]");
                        }
                    }
                });
            }

            // Clear streaming state
            is_loading.set(false);
            current_stream_id.set(None);
            streaming_message_id.set(None);
        }
    };

    // Send message handler with streaming support
    let send_message_streaming = move || {
        let msg = message_input.get();
        if msg.trim().is_empty() || is_loading.get() {
            return;
        }

        // Add user message
        let user_msg_id = next_message_id.get();
        next_message_id.set(user_msg_id + 1);
        messages.update(|msgs| {
            msgs.push(Message {
                id: user_msg_id,
                role: "user".to_string(),
                content: msg.clone(),
                tokens: None,
                is_streaming: false,
                stream_id: None,
            });
        });

        // Add placeholder assistant message for streaming
        let assistant_msg_id = next_message_id.get();
        next_message_id.set(assistant_msg_id + 1);
        messages.update(|msgs| {
            msgs.push(Message {
                id: assistant_msg_id,
                role: "assistant".to_string(),
                content: String::new(),
                tokens: None,
                is_streaming: true,
                stream_id: None,
            });
        });

        message_input.set(String::new());
        is_loading.set(true);
        // Generate stream ID on frontend to prevent race condition
        let stream_id = uuid::Uuid::new_v4().to_string();
        let stream_id_clone = stream_id.clone();

        // Set active stream BEFORE calling backend
        current_stream_id.set(Some(stream_id.clone()));
        streaming_message_id.set(Some(assistant_msg_id));

        // Update the placeholder message with the stream ID immediately
        messages.update(|msgs| {
            if let Some(msg) = msgs.iter_mut().find(|m| m.id == assistant_msg_id) {
                msg.stream_id = Some(stream_id.clone());
            }
        });

        // Build conversation history for context
        let history: Vec<StreamingChatMessage> = messages.get().iter()
            .filter(|m| m.role == "user" || m.role == "assistant")
            .filter(|m| m.id != assistant_msg_id) // Exclude the placeholder
            .map(|m| StreamingChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        spawn_local(async move {
            match stream_chat(history, None, None, None, Some(stream_id_clone)).await {
                Ok(_) => {
                    // Stream started successfully (ID already set)
                }
                Err(e) => {
                    // Replace streaming message with error
                    messages.update(|msgs| {
                        if let Some(msg) = msgs.iter_mut().find(|m| m.id == assistant_msg_id) {
                            msg.role = "error".to_string();
                            msg.content = format!("Streaming error: {}\n\nCourse of Action: Check your network connection or verify the LLM provider settings.", e);
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

    // Fallback to non-streaming chat (available as a backup if needed)
    #[allow(dead_code)]
    let _send_message_non_streaming = move || {
        let msg = message_input.get();
        if msg.trim().is_empty() || is_loading.get() {
            return;
        }

        // Add user message
        let id = next_message_id.get();
        next_message_id.set(id + 1);
        messages.update(|msgs| {
            msgs.push(Message {
                id,
                role: "user".to_string(),
                content: msg.clone(),
                tokens: None,
                is_streaming: false,
                stream_id: None,
            });
        });
        message_input.set(String::new());
        is_loading.set(true);

        spawn_local(async move {
            let request = ChatRequestPayload {
                message: msg,
                system_prompt: None,
                context: None,
                personality_id: None,
                use_rag: true,
            };

            match chat(request).await {
                Ok(response) => {
                    let id = next_message_id.get();
                    next_message_id.set(id + 1);
                    messages.update(|msgs| {
                        msgs.push(Message {
                            id,
                            role: "assistant".to_string(),
                            content: response.content,
                            tokens: match (response.input_tokens, response.output_tokens) {
                                (Some(i), Some(o)) => Some((i, o)),
                                _ => None,
                            },
                            is_streaming: false,
                            stream_id: None,
                        });
                    });
                    // Update session usage
                    if let Ok(usage) = get_session_usage().await {
                        session_usage.set(usage);
                    }
                }
                Err(e) => {
                    let id = next_message_id.get();
                    next_message_id.set(id + 1);
                    messages.update(|msgs| {
                        msgs.push(Message {
                            id,
                            role: "error".to_string(),
                            content: format!("Error: {}\n\nSuggestion: Ensure the model is downloaded and running.", e),
                            tokens: None,
                            is_streaming: false,
                            stream_id: None,
                        });
                    });
                    show_error("Request Failed", Some(&e), None);
                }
            }
            is_loading.set(false);
        });
    };

    // Use streaming by default
    let send_message = send_message_streaming;

    // Click handler for send button
    let on_send_click = move |_: ev::MouseEvent| {
        send_message();
    };

    // Click handler for cancel button
    let on_cancel_click = move |_: ev::MouseEvent| {
        cancel_current_stream();
    };

    // Keydown handler for Enter key
    let on_keydown = Callback::new(move |e: ev::KeyboardEvent| {
        if e.key() == "Enter" && !e.shift_key() {
            e.prevent_default();
            send_message();
        }
        // Escape key to cancel stream
        if e.key() == "Escape" && is_loading.get() {
            e.prevent_default();
            cancel_current_stream();
        }
    });

    view! {
        <div class="flex flex-col h-screen bg-theme-primary text-theme-primary font-sans transition-colors duration-300">
            // Header
            <div class="p-4 bg-theme-secondary border-b border-theme flex justify-between items-center">
                <div class="flex items-center gap-4">
                    <h1 class="text-xl font-bold">"Sidecar DM"</h1>
                    <span class=move || {
                        if llm_status.get().contains("connected") {
                            "text-xs px-2 py-1 bg-green-900 text-green-300 rounded"
                        } else {
                            "text-xs px-2 py-1 bg-yellow-900 text-yellow-300 rounded"
                        }
                    }>{move || llm_status.get()}</span>
                </div>
                <nav class="flex items-center gap-6">
                    // Usage indicator (only show if tokens used with a paid model)
                    {move || {
                        let usage = session_usage.get();
                        let total_tokens = usage.session_input_tokens + usage.session_output_tokens;
                        let cost_display = if usage.session_cost_usd < 0.01 {
                            "<$0.01".to_string()
                        } else {
                            format!("${:.2}", usage.session_cost_usd)
                        };
                        if total_tokens >= 1 && usage.session_cost_usd > 0.0 {
                            Some(
                                view! {
                                    <Button
                                        variant=ButtonVariant::Secondary
                                        class="text-xs py-1"
                                        on_click=move |_: ev::MouseEvent| {
                                            show_usage_panel.update(|v| *v = !*v);
                                        }
                                    >
                                        <span class="text-gray-400">
                                            {total_tokens} " tokens"
                                        </span>
                                        <span class="text-green-400">{cost_display}</span>
                                    </Button>
                                },
                            )
                        } else {
                            None
                        }
                    }}
                    <HeaderLink href="/campaigns" label="Campaigns">
                        <FolderIcon />
                    </HeaderLink>
                    <HeaderLink href="/character" label="Characters">
                        <UsersIcon />
                    </HeaderLink>
                    <HeaderLink href="/library" label="Library">
                        <BookIcon />
                    </HeaderLink>
                    <HeaderLink href="/settings" label="Settings">
                        <SettingsIcon />
                    </HeaderLink>
                </nav>
            </div>

            // Usage Panel (collapsible)
            {move || {
                if show_usage_panel.get() {
                    let usage = session_usage.get();
                    Some(
                        view! {
                            <div class="p-3 bg-gray-800 border-b border-gray-700">
                                <div class="max-w-4xl mx-auto">
                                    <div class="flex justify-between items-center text-sm">
                                        <div class="flex gap-6">
                                            <div>
                                                <span class="text-gray-400">"Input: "</span>
                                                <span class="text-white font-mono">
                                                    {usage.session_input_tokens}
                                                </span>
                                            </div>
                                            <div>
                                                <span class="text-gray-400">"Output: "</span>
                                                <span class="text-white font-mono">
                                                    {usage.session_output_tokens}
                                                </span>
                                            </div>
                                            <div>
                                                <span class="text-gray-400">"Requests: "</span>
                                                <span class="text-white font-mono">
                                                    {usage.session_requests}
                                                </span>
                                            </div>
                                            <div>
                                                <span class="text-gray-400">"Est. Cost: "</span>
                                                <span class="text-green-400 font-mono">
                                                    {format!("${:.4}", usage.session_cost_usd)}
                                                </span>
                                            </div>
                                        </div>
                                        <button
                                            class="text-gray-500 hover:text-white text-xs"
                                            on:click=move |_| show_usage_panel.set(false)
                                        >
                                            "Close"
                                        </button>
                                    </div>
                                </div>
                            </div>
                        },
                    )
                } else {
                    None
                }
            }}

            // Message Area
            <div class="flex-1 p-4 overflow-y-auto space-y-4">
                {
                    let layout = use_layout_state();
                    view! {
                        <For
                            each=move || messages.get()
                            key=|msg| (msg.id, msg.content.len(), msg.is_streaming)
                            children=move |msg| {
                                let role = msg.role.clone();
                                let content = msg.content.clone();
                                let tokens = msg.tokens;
                                let is_streaming = msg.is_streaming;
                                let show_tokens = layout.show_token_usage.get();
                                let on_play_handler = if role == "assistant" && !is_streaming {
                                    let content_for_play = content.clone();
                                    Some(Callback::new(move |_: ()| play_message(content_for_play.clone())))
                                } else {
                                    None
                                };
                                view! {
                                    <ChatMessage
                                        role=role
                                        content=content
                                        tokens=tokens
                                        is_streaming=is_streaming
                                        on_play=on_play_handler
                                        show_tokens=show_tokens
                                    />
                                }
                            }
                        />
                    }
                }
            </div>

            // Input Area
            <div class="p-4 bg-theme-secondary border-t border-theme">
                <div class="flex gap-2 max-w-4xl mx-auto">
                    <div class="flex-1">
                        <Input
                            value=message_input
                            placeholder="Ask the DM... (Escape to cancel)"
                            disabled=is_loading
                            on_keydown=on_keydown
                        />
                    </div>
                    {move || {
                        if is_loading.get() {
                            view! {
                                <Button
                                    variant=ButtonVariant::Secondary
                                    on_click=on_cancel_click
                                    class="bg-red-900 hover:bg-red-800 border-red-700"
                                >
                                    <svg class="w-4 h-4 mr-1" viewBox="0 0 24 24" fill="currentColor">
                                        <path d="M6 6h12v12H6z"/>
                                    </svg>
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
// SVG Icon Components for Header
#[component]
fn FolderIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
        </svg>
    }
}

#[component]
fn UsersIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path>
            <circle cx="9" cy="7" r="4"></circle>
            <path d="M23 21v-2a4 4 0 0 0-3-3.87"></path>
            <path d="M16 3.13a4 4 0 0 1 0 7.75"></path>
        </svg>
    }
}

#[component]
fn BookIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"></path>
            <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"></path>
        </svg>
    }
}

#[component]
fn SettingsIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="3"></circle>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
        </svg>
    }
}

// Navigation Link component with tooltip (dynamic text/icon mode)
#[component]
pub fn HeaderLink(
    href: &'static str,
    label: &'static str,
    children: Children,
) -> impl IntoView {
    let layout_state = crate::services::layout_service::use_layout_state();
    let text_mode = layout_state.text_navigation;
    let icon_children = children();

    view! {
        <A href=href attr:class="group relative text-theme-secondary hover:text-theme-primary transition-colors p-2 rounded hover:bg-white/5 flex items-center justify-center">
            // Icon (Hidden in text mode, but rendered once to avoid re-creation issues)
            <div class=move || if text_mode.get() { "hidden" } else { "" }>
                {icon_children}
            </div>

            // Text Label (Dynamic)
            {move || {
                if text_mode.get() {
                    view! { <span class="font-medium text-sm px-2 animate-fade-in">{label}</span> }.into_any()
                } else {
                    view! { <span class="hidden" /> }.into_any()
                }
            }}

            // Tooltip (Only in icon mode)
            <div
                class=move || format!(
                    "absolute top-full mt-2 left-1/2 -translate-x-1/2 bg-[var(--bg-elevated)] text-[var(--text-primary)] text-xs font-medium px-2 py-1 rounded shadow-lg opacity-0 group-hover:opacity-100 group-hover:translate-y-1 group-focus:opacity-100 transition-all duration-200 whitespace-nowrap border border-[var(--border-subtle)] z-[100] pointer-events-none backdrop-blur-md {}",
                    if text_mode.get() { "hidden" } else { "" }
                )
                role="tooltip"
            >
                {label}
            </div>
        </A>
    }
}
