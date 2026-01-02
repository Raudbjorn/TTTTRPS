pub mod chat_message;

pub use chat_message::ChatMessage;

use leptos::ev;
use leptos::prelude::*;
use leptos_router::components::A;
use wasm_bindgen_futures::spawn_local;

use crate::bindings::{
    chat, check_llm_health, get_session_usage, ChatRequestPayload, SessionUsage, speak,
};
use crate::components::design_system::{Button, ButtonVariant, Input, TypingIndicator};

/// Message in the chat history
#[derive(Clone, PartialEq)]
pub struct Message {
    pub id: usize,
    pub role: String,
    pub content: String,
    pub tokens: Option<(u32, u32)>,
}

/// Main Chat component - the primary DM interface
#[component]
pub fn Chat() -> impl IntoView {
    // State signals
    let message_input = RwSignal::new(String::new());
    let messages = RwSignal::new(vec![Message {
        id: 0,
        role: "assistant".to_string(),
        content: "Welcome to Sidecar DM! I'm your AI-powered TTRPG assistant. Configure an LLM provider in Settings to get started.".to_string(),
        tokens: None,
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

    // Check LLM health on mount
    Effect::new(move |_| {
        spawn_local(async move {
            match check_llm_health().await {
                Ok(status) => {
                    if status.healthy {
                        llm_status.set(format!("{} connected", status.provider));
                    } else {
                        llm_status.set(format!("{}: {}", status.provider, status.message));
                    }
                }
                Err(e) => {
                    llm_status.set(format!("Error: {}", e));
                }
            }
        });
    });

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
                        });
                    });
                }
            }
        });
    };

    // Send message handler
    let send_message = move || {
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
                            content: format!("Error: {}", e),
                            tokens: None,
                        });
                    });
                }
            }
            is_loading.set(false);
        });
    };

    // Click handler for send button (plain closure for Button component)
    let on_send_click = move |_: ev::MouseEvent| {
        send_message();
    };

    // Keydown handler for Enter key
    let on_keydown = Callback::new(move |e: ev::KeyboardEvent| {
        if e.key() == "Enter" && !e.shift_key() {
            e.prevent_default();
            send_message();
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
                <div class="flex items-center gap-4">
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
                    <A href="/campaigns" attr:class="text-theme-secondary hover:text-theme-primary">
                        "Campaigns"
                    </A>
                    <A href="/character" attr:class="text-theme-secondary hover:text-theme-primary">
                        "Characters"
                    </A>
                    <A href="/library" attr:class="text-theme-secondary hover:text-theme-primary">
                        "Library"
                    </A>
                    <A href="/settings" attr:class="text-theme-secondary hover:text-theme-primary">
                        "Settings"
                    </A>
                </div>
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
                        let on_play_handler = if role == "assistant" {
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
                                on_play=on_play_handler
                            />
                        }
                    }
                />
                // Loading indicator
                {move || {
                    if is_loading.get() {
                        Some(
                            view! {
                                <div class="bg-zinc-800/50 p-3 rounded-lg max-w-3xl border border-zinc-700/50">
                                    <div class="flex items-center gap-2">
                                        <TypingIndicator />
                                        <span class="text-xs text-zinc-500">"Thinking..."</span>
                                    </div>
                                </div>
                            },
                        )
                    } else {
                        None
                    }
                }}
            </div>

            // Input Area
            <div class="p-4 bg-theme-secondary border-t border-theme">
                <div class="flex gap-2 max-w-4xl mx-auto">
                    <div class="flex-1">
                        <Input
                            value=message_input
                            placeholder="Ask the DM..."
                            disabled=is_loading.get()
                            on_keydown=on_keydown
                        />
                    </div>
                    <Button loading=is_loading.get() on_click=on_send_click>
                        "Send"
                    </Button>
                </div>
            </div>
        </div>
    }
}
