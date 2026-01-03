//! Claude Bridge - Leptos Frontend
//!
//! A Tauri + Leptos application for bridging local processes to Claude Desktop via CDP.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

mod components;
mod tauri;

use components::{Header, InputArea, Messages};

/// Application state
#[derive(Clone, Debug, Default)]
pub struct AppState {
    pub connected: bool,
    pub connecting: bool,
    pub messages: Vec<ChatMessage>,
    pub error: Option<String>,
    pub config_open: bool,
    pub port: u16,
}

/// A chat message
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }

    pub fn is_user(&self) -> bool {
        self.role == "user"
    }
}

/// Main application component
#[component]
pub fn App() -> impl IntoView {
    // Reactive state
    let connected = RwSignal::new(false);
    let connecting = RwSignal::new(false);
    let messages = RwSignal::new(Vec::<ChatMessage>::new());
    let error = RwSignal::new(Option::<String>::None);
    let sending = RwSignal::new(false);
    let config_open = RwSignal::new(false);
    let port = RwSignal::new(9222u16);

    // Connection handler
    let connect = move |_| {
        connecting.set(true);
        error.set(None);

        spawn_local(async move {
            match tauri::connect().await {
                Ok(status) => {
                    connected.set(status.connected);
                    if !status.connected {
                        error.set(Some("Failed to connect. Is Claude Desktop running with --remote-debugging-port?".to_string()));
                    }
                }
                Err(e) => {
                    error.set(Some(format!("Connection error: {}", e)));
                }
            }
            connecting.set(false);
        });
    };

    // Disconnect handler
    let disconnect = move |_| {
        spawn_local(async move {
            let _ = tauri::disconnect().await;
            connected.set(false);
            messages.set(Vec::new());
        });
    };

    // Send message handler
    let send_message = move |content: String| {
        if content.trim().is_empty() || !connected.get() {
            return;
        }

        // Add user message immediately
        messages.update(|msgs| {
            msgs.push(ChatMessage::user(content.clone()));
        });

        sending.set(true);
        error.set(None);

        spawn_local(async move {
            match tauri::send_message(&content).await {
                Ok(response) => {
                    messages.update(|msgs| {
                        msgs.push(ChatMessage::assistant(response));
                    });
                }
                Err(e) => {
                    error.set(Some(format!("Failed to send: {}", e)));
                }
            }
            sending.set(false);
        });
    };

    // New conversation handler
    let new_conversation = move |_| {
        spawn_local(async move {
            if tauri::new_conversation().await.is_ok() {
                messages.set(Vec::new());
            }
        });
    };

    // Toggle config panel
    let toggle_config = move |_| {
        config_open.update(|open| *open = !*open);
    };

    view! {
        <div class="app">
            <Header
                connected=connected
                connecting=connecting
                on_connect=connect
                on_disconnect=disconnect
                on_new_chat=new_conversation
                on_config=toggle_config
            />

            <Messages
                messages=messages
                sending=sending
            />

            <InputArea
                on_send=send_message
                disabled=Signal::derive(move || !connected.get() || sending.get())
            />

            // Error toast
            <Show when=move || error.get().is_some()>
                <div class="toast toast--error visible">
                    {move || error.get().unwrap_or_default()}
                </div>
            </Show>

            // Config panel overlay
            <div
                class="overlay"
                class:visible=move || config_open.get()
                on:click=move |_| config_open.set(false)
            />

            // Config panel
            <div class="config-panel" class:open=move || config_open.get()>
                <div class="config-panel__header">
                    <span class="config-panel__title">"Configuration"</span>
                    <button class="btn btn--ghost" on:click=move |_| config_open.set(false)>
                        "×"
                    </button>
                </div>

                <div class="config-group">
                    <label class="config-group__label">"CDP Port"</label>
                    <input
                        type="number"
                        class="config-input"
                        prop:value=move || port.get()
                        on:change=move |ev| {
                            if let Ok(p) = event_target_value(&ev).parse::<u16>() {
                                port.set(p);
                            }
                        }
                    />
                </div>

                <div class="config-group">
                    <label class="config-group__label">"Launch Command"</label>
                    <code class="config-input" style="font-size: 0.75rem; word-break: break-all;">
                        {move || format!("claude-desktop --remote-debugging-port={}", port.get())}
                    </code>
                </div>
            </div>
        </div>
    }
}

/// Entry point
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
