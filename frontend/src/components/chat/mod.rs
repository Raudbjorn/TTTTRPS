pub mod chat_message;
pub mod personality_selector;

pub use chat_message::ChatMessage;
pub use personality_selector::{PersonalitySelector, PersonalityIndicator};

use leptos::ev;
use leptos::prelude::*;
use leptos_router::components::A;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use std::sync::Arc;
use crate::services::notification_service::{show_error, ToastAction};

use crate::bindings::{
    cancel_stream, chat, check_llm_health, get_session_usage, listen_chat_chunks, stream_chat,
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
    let current_stream_id = RwSignal::new(Option::<String>::None);
    let streaming_message_id = RwSignal::new(Option::<usize>::None);

    // Store the unlisten handle for cleanup
    let unlisten_handle: Rc<RefCell<Option<JsValue>>> = Rc::new(RefCell::new(None));

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
    {
        let unlisten_handle = unlisten_handle.clone();
        Effect::new(move |_| {
            let handle = listen_chat_chunks(move |chunk: ChatChunk| {
                // Only process chunks for our active stream
                if let Some(active_stream) = current_stream_id.get() {
                    if chunk.stream_id != active_stream {
                        return;
                    }
                }

                if let Some(msg_id) = streaming_message_id.get() {
                    // Append content to the streaming message
                    if !chunk.content.is_empty() {
                        messages.update(|msgs| {
                            if let Some(msg) = msgs.iter_mut().find(|m| m.id == msg_id) {
                                msg.content.push_str(&chunk.content);
                            }
                        });
                    }

                    // Handle stream completion
                    if chunk.is_final {
                        messages.update(|msgs| {
                            if let Some(msg) = msgs.iter_mut().find(|m| m.id == msg_id) {
                                msg.is_streaming = false;
                                msg.stream_id = None;
                                // Set token usage if available
                                if let Some(usage) = &chunk.usage {
                                    msg.tokens = Some((usage.input_tokens, usage.output_tokens));
                                }
                            }
                        });

                        // Update session usage
                        spawn_local(async move {
                            if let Ok(usage) = get_session_usage().await {
                                session_usage.set(usage);
                            }
                        });

                        // Clear streaming state
                        is_loading.set(false);
                        current_stream_id.set(None);
                        streaming_message_id.set(None);
                    }
                }
            });

            *unlisten_handle.borrow_mut() = Some(handle);
        });
    }

    // Play message via TTS
    let play_message = move |text: String| {
        let messages = messages;
        let next_id = next_message_id;
        spawn_local(async move {
            match speak(text).await {
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
        streaming_message_id.set(Some(assistant_msg_id));

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
            match stream_chat(history, None, None, None).await {
                Ok(stream_id) => {
                    // Update the placeholder message with the stream ID
                    messages.update(|msgs| {
                        if let Some(msg) = msgs.iter_mut().find(|m| m.id == assistant_msg_id) {
                            msg.stream_id = Some(stream_id.clone());
                        }
                    });
                    current_stream_id.set(Some(stream_id));
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
                <nav class="flex items-center gap-4">
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
                    <A href="/campaigns" attr:class="text-theme-secondary hover:text-theme-primary px-2">
                        "Campaigns"
                    </A>
                    <A href="/character" attr:class="text-theme-secondary hover:text-theme-primary px-2">
                        "Characters"
                    </A>
                    <A href="/library" attr:class="text-theme-secondary hover:text-theme-primary px-2">
                        "Library"
                    </A>
                    <A href="/settings" attr:class="text-theme-secondary hover:text-theme-primary px-2">
                        "Settings"
                    </A>
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
                <For
                    each=move || messages.get()
                    key=|msg| msg.id
                    children=move |msg| {
                        let role = msg.role.clone();
                        let content = msg.content.clone();
                        let tokens = msg.tokens;
                        let is_streaming = msg.is_streaming;
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
                            />
                        }
                    }
                />
            </div>

            // Input Area
            <div class="p-4 bg-theme-secondary border-t border-theme">
                <div class="flex gap-2 max-w-4xl mx-auto">
                    <div class="flex-1">
                        <Input
                            value=message_input
                            placeholder="Ask the DM... (Escape to cancel)"
                            disabled=is_loading.get()
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
