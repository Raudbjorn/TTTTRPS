#![allow(non_snake_case)]
use dioxus::prelude::*;

#[component]
pub fn Chat() -> Element {
    let mut message_input = use_signal(|| String::new());
    let mut messages = use_signal(|| vec![
        ("assistant".to_string(), "Welcome to the Rust-powered TTRPG Assistant! I am ready to help you run your campaign.".to_string())
    ]);

    let send_message = move |_: MouseEvent| {
        let msg = message_input.read().clone();
        if !msg.trim().is_empty() {
            messages.write().push(("user".to_string(), msg.clone()));
            message_input.set(String::new());

            // TODO: Call Tauri backend for LLM response
            spawn(async move {
                // Placeholder for Tauri invoke
                // let response = invoke("chat", ChatRequest { message: msg }).await;
                messages.write().push(("assistant".to_string(), "I received your message. LLM integration coming soon!".to_string()));
            });
        }
    };

    rsx! {
        div {
            class: "flex flex-col h-screen bg-gray-900 text-white font-sans",
            // Header
            div {
                class: "p-4 bg-gray-800 border-b border-gray-700 flex justify-between items-center",
                h1 { class: "text-xl font-bold", "Sidecar DM" }
                div {
                    class: "flex gap-4",
                    Link { to: crate::Route::Library {}, class: "text-gray-300 hover:text-white", "Library" }
                    Link { to: crate::Route::Settings {}, class: "text-gray-300 hover:text-white", "Settings" }
                }
            }
            // Message Area
            div {
                class: "flex-1 p-4 overflow-y-auto space-y-4",
                for (role, content) in messages.read().iter() {
                    div {
                        class: if role == "user" { "bg-blue-800 p-3 rounded-lg max-w-3xl ml-auto" } else { "bg-gray-800 p-3 rounded-lg max-w-3xl" },
                        "{content}"
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
                        oninput: move |e| message_input.set(e.value()),
                        onkeydown: move |e: KeyboardEvent| {
                            if e.key() == Key::Enter {
                                let msg = message_input.read().clone();
                                if !msg.trim().is_empty() {
                                    messages.write().push(("user".to_string(), msg.clone()));
                                    message_input.set(String::new());
                                    spawn(async move {
                                        messages.write().push(("assistant".to_string(), "I received your message. LLM integration coming soon!".to_string()));
                                    });
                                }
                            }
                        }
                    }
                    button {
                        class: "px-4 py-2 bg-blue-600 rounded hover:bg-blue-500 transition-colors",
                        onclick: send_message,
                        "Send"
                    }
                }
            }
        }
    }
}
