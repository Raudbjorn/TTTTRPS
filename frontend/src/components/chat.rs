#![allow(non_snake_case)]
use dioxus::prelude::*;
use crate::bindings::{chat, ChatRequestPayload, check_llm_health, get_session_usage, SessionUsage, speak};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use crate::components::design_system::{Button, ButtonVariant, Input, LoadingSpinner, Badge, BadgeVariant, Markdown, TypingIndicator};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

// SpeakRequest removed as we use bindings::speak now

#[derive(Clone, PartialEq)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub tokens: Option<(u32, u32)>, // (input, output)
}

#[component]
pub fn Chat() -> Element {
    let mut message_input = use_signal(|| String::new());
    let mut messages = use_signal(|| vec![
        Message {
            role: "assistant".to_string(),
            content: "Welcome to Sidecar DM! I'm your AI-powered TTRPG assistant. Configure an LLM provider in Settings to get started.".to_string(),
            tokens: None,
        }
    ]);
    let mut is_loading = use_signal(|| false);
    let mut llm_status = use_signal(|| "Checking...".to_string());
    let mut session_usage = use_signal(|| SessionUsage {
        session_input_tokens: 0,
        session_output_tokens: 0,
        session_requests: 0,
        session_cost_usd: 0.0,
    });
    let mut show_usage_panel = use_signal(|| false);

    // Check LLM health on mount
    use_effect(move || {
        spawn(async move {
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


    let play_message = move |text: String| {
        spawn(async move {
            match speak(text).await {
                Ok(_) => {}, // Success, audio playing
                Err(e) => {
                     // Show error in chat stream as system message or alert
                     // For now, easy way: push error message to chat
                     messages.write().push(Message {
                         role: "error".to_string(),
                         content: format!("Voice Error: {}", e),
                         tokens: None,
                     });
                }
            }
        });
    };

    let send_message = move |_: MouseEvent| {
        let msg = message_input.read().clone();
        if !msg.trim().is_empty() && !*is_loading.read() {
            // Add user message
            messages.write().push(Message {
                role: "user".to_string(),
                content: msg.clone(),
                tokens: None,
            });
            message_input.set(String::new());
            is_loading.set(true);

            message_input.set(String::new());
            is_loading.set(true);

            spawn(async move {
                let request = ChatRequestPayload {
                    message: msg,
                    system_prompt: None, // Use default GM prompt
                    context: None,
                };

                match chat(request).await {
                    Ok(response) => {
                        messages.write().push(Message {
                            role: "assistant".to_string(),
                            content: response.content,
                            tokens: match (response.input_tokens, response.output_tokens) {
                                (Some(i), Some(o)) => Some((i, o)),
                                _ => None,
                            },
                        });
                        // Update session usage
                        if let Ok(usage) = get_session_usage().await {
                            session_usage.set(usage);
                        }
                    }
                    Err(e) => {
                        messages.write().push(Message {
                            role: "error".to_string(),
                            content: format!("Error: {}", e),
                            tokens: None,
                        });
                    }
                }
                is_loading.set(false);
            });
        }
    };

    let handle_keydown = move |e: KeyboardEvent| {
        if e.key() == Key::Enter && !e.modifiers().shift() {
            let msg = message_input.read().clone();
            if !msg.trim().is_empty() && !*is_loading.read() {
                messages.write().push(Message {
                    role: "user".to_string(),
                    content: msg.clone(),
                    tokens: None,
                });
                message_input.set(String::new());
                is_loading.set(true);

                spawn(async move {
                    let request = ChatRequestPayload {
                        message: msg,
                        system_prompt: None,
                        context: None,
                    };

                    match chat(request).await {
                        Ok(response) => {
                            messages.write().push(Message {
                                role: "assistant".to_string(),
                                content: response.content,
                                tokens: match (response.input_tokens, response.output_tokens) {
                                    (Some(i), Some(o)) => Some((i, o)),
                                    _ => None,
                                },
                            });
                            // Update session usage
                            if let Ok(usage) = get_session_usage().await {
                                session_usage.set(usage);
                            }
                        }
                        Err(e) => {
                            messages.write().push(Message {
                                role: "error".to_string(),
                                content: format!("Error: {}", e),
                                tokens: None,
                            });
                        }
                    }
                    is_loading.set(false);
                });
            }
        }
    };

    let usage = session_usage.read();
    let total_tokens = usage.session_input_tokens + usage.session_output_tokens;
    let cost_display = if usage.session_cost_usd < 0.01 {
        format!("<$0.01")
    } else {
        format!("${:.2}", usage.session_cost_usd)
    };

    rsx! {
        div {
            class: "flex flex-col h-screen bg-theme-primary text-theme-primary font-sans transition-colors duration-300",
            // Header
            div {
                class: "p-4 bg-theme-secondary border-b border-theme flex justify-between items-center",
                div {
                    class: "flex items-center gap-4",
                    h1 { class: "text-xl font-bold", "Sidecar DM" }
                    span {
                        class: if llm_status.read().contains("connected") { "text-xs px-2 py-1 bg-green-900 text-green-300 rounded" } else { "text-xs px-2 py-1 bg-yellow-900 text-yellow-300 rounded" },
                        "{llm_status}"
                    }
                }
                div {
                    class: "flex items-center gap-4",
                    // Usage indicator (only show if tokens used with a paid model)
                    if total_tokens >= 1 && usage.session_cost_usd > 0.0 {
                        Button {
                            variant: ButtonVariant::Secondary,
                            class: "text-xs py-1",
                            onclick: move |_| {
                                let current = *show_usage_panel.read();
                                show_usage_panel.set(!current);
                            },
                            span { class: "text-gray-400", "{total_tokens} tokens" }
                            span { class: "text-green-400", "{cost_display}" }
                        }
                    }
                    Link { to: crate::Route::Campaigns {}, class: "text-theme-secondary hover:text-theme-primary", "Campaigns" }
                    Link { to: crate::Route::CharacterCreator {}, class: "text-theme-secondary hover:text-theme-primary", "Characters" }
                    Link { to: crate::Route::Library {}, class: "text-theme-secondary hover:text-theme-primary", "Library" }
                    Link { to: crate::Route::Settings {}, class: "text-theme-secondary hover:text-theme-primary", "Settings" }
                }
            }

            // Usage Panel (collapsible)
            if *show_usage_panel.read() {
                div {
                    class: "p-3 bg-gray-800 border-b border-gray-700",
                    div {
                        class: "max-w-4xl mx-auto",
                        div {
                            class: "flex justify-between items-center text-sm",
                            div {
                                class: "flex gap-6",
                                div {
                                    span { class: "text-gray-400", "Input: " }
                                    span { class: "text-white font-mono", "{usage.session_input_tokens}" }
                                }
                                div {
                                    span { class: "text-gray-400", "Output: " }
                                    span { class: "text-white font-mono", "{usage.session_output_tokens}" }
                                }
                                div {
                                    span { class: "text-gray-400", "Requests: " }
                                    span { class: "text-white font-mono", "{usage.session_requests}" }
                                }
                                div {
                                    span { class: "text-gray-400", "Est. Cost: " }
                                    span { class: "text-green-400 font-mono", "${usage.session_cost_usd:.4}" }
                                }
                            }
                            button {
                                class: "text-gray-500 hover:text-white text-xs",
                                onclick: move |_| show_usage_panel.set(false),
                                "Close"
                            }
                        }
                    }
                }
            }

            // Message Area
            div {
                class: "flex-1 p-4 overflow-y-auto space-y-4",
                for msg in messages.read().iter() {
                    div {
                        class: match msg.role.as_str() {
                            "user" => "bg-blue-800 p-3 rounded-lg max-w-3xl ml-auto",
                            "error" => "bg-red-900 p-3 rounded-lg max-w-3xl border border-red-700",
                            _ => "bg-theme-secondary p-3 rounded-lg max-w-3xl group relative border border-theme", // Assistant with theme
                        },
                        if msg.role == "assistant" {
                             div {
                                class: "absolute -left-10 top-1 opacity-0 group-hover:opacity-100 transition-opacity",
                                button {
                                    class: "p-2 bg-gray-700 rounded-full hover:bg-gray-600 text-gray-300 hover:text-white",
                                    title: "Read Aloud",
                                    onclick: {
                                        let c = msg.content.clone();
                                        move |_| play_message(c.clone())
                                    },
                                    // Simple Play Icon SVG
                                    svg {
                                        class: "w-4 h-4",
                                        view_box: "0 0 24 24",
                                        fill: "none",
                                        stroke: "currentColor",
                                        "stroke-width": "2",
                                        path {
                                            d: "M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"
                                        }
                                        path {
                                            d: "M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                                        }
                                    }
                                }
                                button {
                                    class: "p-2 bg-gray-700 rounded-full hover:bg-gray-600 text-gray-300 hover:text-white ml-2",
                                    title: "Copy",
                                    onclick: {
                                        let c = msg.content.clone();
                                        move |_| {
                                            let c = c.clone();
                                            spawn(async move {
                                                if let Some(window) = web_sys::window() {
                                                    let navigator = window.navigator();
                                                    let clipboard = navigator.clipboard();
                                                    let _ = wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&c)).await;
                                                }
                                            });
                                        }
                                    },
                                    svg {
                                        class: "w-4 h-4",
                                        view_box: "0 0 24 24",
                                        fill: "none",
                                        stroke: "currentColor",
                                        "stroke-width": "2",
                                        path { d: "M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" }
                                    }
                                }
                            }
                        }
                        div {
                            class: "min-w-0 break-words", // Ensure container handles overflow
                            if msg.role == "assistant" {
                                Markdown { content: msg.content.clone() }
                            } else {
                                div { class: "whitespace-pre-wrap text-white", "{msg.content}" }
                            }
                        }
                        if let Some((input, output)) = msg.tokens {
                            div {
                                class: "text-xs text-gray-500 mt-2",
                                "Tokens: {input} in / {output} out"
                            }
                        }
                    }
                }
                if *is_loading.read() {
                    div {
                        class: "bg-gray-800 p-3 rounded-lg max-w-3xl",
                        div { class: "flex items-center gap-2",
                            TypingIndicator {}
                            span { class: "text-xs text-gray-500", "Thinking..." }
                        }
                    }
                }
            }
            // Input Area
            div {
                class: "p-4 bg-theme-secondary border-t border-theme",
                div {
                    class: "flex gap-2 max-w-4xl mx-auto",
                    div { class: "flex-1",
                        Input {
                            value: "{message_input}",
                            placeholder: "Ask the DM...",
                            disabled: *is_loading.read(),
                            oninput: move |val| message_input.set(val),
                            onkeydown: handle_keydown
                        }
                    }
                    Button {
                        loading: *is_loading.read(),
                        onclick: send_message,
                        "Send"
                    }
                }
            }
        }
    }
}
