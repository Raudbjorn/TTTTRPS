#![allow(non_snake_case)]
use dioxus::prelude::*;
use crate::bindings::{chat, ChatRequestPayload, check_llm_health};

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
                    class: "flex gap-4",
                    Link { to: crate::Route::Library {}, class: "text-gray-300 hover:text-white", "Library" }
                    Link { to: crate::Route::Settings {}, class: "text-gray-300 hover:text-white", "Settings" }
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
                    class: "flex gap-2",
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
