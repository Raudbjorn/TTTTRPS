#![allow(non_snake_case)]
use dioxus::prelude::*;
use crate::bindings::{chat, ChatRequestPayload, check_llm_health, get_session_usage, SessionUsage};

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
            class: "flex flex-col h-screen bg-gray-900 text-white font-sans",
            // Header
            div {
                class: "p-4 bg-gray-800 border-b border-gray-700 flex justify-between items-center",
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
                    // Usage indicator
                    button {
                        class: "flex items-center gap-2 text-xs px-2 py-1 bg-gray-700 rounded hover:bg-gray-600",
                        onclick: move |_| {
                            let current = *show_usage_panel.read();
                            show_usage_panel.set(!current);
                        },
                        span { class: "text-gray-400", "{total_tokens} tokens" }
                        span { class: "text-green-400", "{cost_display}" }
                    }
                    Link { to: crate::Route::Campaigns {}, class: "text-gray-300 hover:text-white", "Campaigns" }
                    Link { to: crate::Route::CharacterCreator {}, class: "text-gray-300 hover:text-white", "Characters" }
                    Link { to: crate::Route::Library {}, class: "text-gray-300 hover:text-white", "Library" }
                    Link { to: crate::Route::Settings {}, class: "text-gray-300 hover:text-white", "Settings" }
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
                            _ => "bg-gray-800 p-3 rounded-lg max-w-3xl",
                        },
                        div {
                            class: "whitespace-pre-wrap",
                            "{msg.content}"
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
                        class: "bg-gray-800 p-3 rounded-lg max-w-3xl animate-pulse",
                        div { class: "flex items-center gap-2",
                            div { class: "w-2 h-2 bg-blue-500 rounded-full animate-bounce" }
                            div { class: "w-2 h-2 bg-blue-500 rounded-full animate-bounce", style: "animation-delay: 0.1s" }
                            div { class: "w-2 h-2 bg-blue-500 rounded-full animate-bounce", style: "animation-delay: 0.2s" }
                            span { class: "text-gray-400 ml-2", "Thinking..." }
                        }
                    }
                }
            }
            // Input Area
            div {
                class: "p-4 bg-gray-800 border-t border-gray-700",
                div {
                    class: "flex gap-2 max-w-4xl mx-auto",
                    input {
                        class: "flex-1 p-2 rounded bg-gray-700 text-white border border-gray-600 focus:border-blue-500 outline-none",
                        placeholder: "Ask the DM...",
                        value: "{message_input}",
                        disabled: *is_loading.read(),
                        oninput: move |e| message_input.set(e.value()),
                        onkeydown: handle_keydown
                    }
                    button {
                        class: if *is_loading.read() { "px-4 py-2 bg-gray-600 rounded cursor-not-allowed" } else { "px-4 py-2 bg-blue-600 rounded hover:bg-blue-500 transition-colors" },
                        onclick: send_message,
                        disabled: *is_loading.read(),
                        if *is_loading.read() { "..." } else { "Send" }
                    }
                }
            }
        }
    }
}
