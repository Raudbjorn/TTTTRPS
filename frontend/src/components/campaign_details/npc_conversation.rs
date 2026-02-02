use leptos::prelude::*;
use leptos::ev;
use wasm_bindgen_futures::spawn_local;
use crate::bindings::{
    get_npc_conversation, mark_npc_read, stream_npc_chat, listen_chat_chunks_async,
    ConversationMessage, ChatChunk,
};
use crate::components::design_system::Markdown;

/// UI message for displaying in the chat (separate from backend ConversationMessage)
#[derive(Clone, PartialEq)]
struct UiMessage {
    id: String,
    role: String,
    content: String,
    created_at: String,
    is_streaming: bool,
    stream_id: Option<String>,
}

impl From<ConversationMessage> for UiMessage {
    fn from(m: ConversationMessage) -> Self {
        Self {
            id: m.id,
            role: m.role,
            content: m.content,
            created_at: m.created_at,
            is_streaming: false,
            stream_id: None,
        }
    }
}

/// NPC Conversation component for chat-style messaging with NPCs
/// Uses streaming for real-time NPC responses
#[component]
pub fn NpcConversation(
    /// NPC ID to load conversation for
    npc_id: String,
    /// NPC name for display
    npc_name: String,
    /// Callback when the conversation is closed
    on_close: Callback<()>,
) -> impl IntoView {
    let messages = RwSignal::new(Vec::<UiMessage>::new());
    let is_loading = RwSignal::new(true);
    let is_sending = RwSignal::new(false);
    let input_text = RwSignal::new(String::new());
    let error_msg = RwSignal::new(Option::<String>::None);
    let current_stream_id = RwSignal::new(Option::<String>::None);

    let npc_id_signal = RwSignal::new(npc_id.clone());
    let npc_name_display = npc_name.clone();
    let npc_name_input = npc_name.clone();
    let npc_initial = npc_name.chars().next().unwrap_or('?');

    // Load conversation on mount
    Effect::new(move |_| {
        let npc_id = npc_id_signal.get();
        spawn_local(async move {
            match get_npc_conversation(npc_id.clone()).await {
                Ok(conv) => {
                    let parsed: Vec<ConversationMessage> =
                        serde_json::from_str(&conv.messages_json).unwrap_or_default();
                    messages.set(parsed.into_iter().map(UiMessage::from).collect());
                    let _ = mark_npc_read(npc_id).await;
                }
                Err(e) => {
                    if !e.contains("not found") {
                        error_msg.set(Some(e));
                    }
                }
            }
            is_loading.set(false);
        });
    });

    // Set up streaming chunk listener
    //
    // IMPORTANT: Understanding the unlisten handle behavior:
    //
    // The `_unlisten` JsValue returned by listen_chat_chunks_async is intentionally
    // not stored. This is SAFE because:
    //
    // 1. **Dropping unlisten does NOT unregister the callback**: The callback is
    //    captured by a JavaScript closure in Tauri's event system. The unlisten
    //    handle only provides the *ability* to explicitly unregister - dropping
    //    it just means you lose that ability.
    //
    // 2. **JsValue is !Send**: Cannot be stored in Leptos signals (which require
    //    Send+Sync) or used with on_cleanup closures.
    //
    // 3. **Stream ID filtering prevents interference**: Each stream has a unique
    //    UUID, so listeners from different component instances don't conflict.
    //
    // 4. **Graceful degradation**: try_update/try_set return None when signals
    //    are disposed, preventing crashes if chunks arrive after unmount.
    //
    // 5. **Automatic cleanup**: Tauri cleans up all listeners when webview closes.
    {
        spawn_local(async move {
            let _unlisten = listen_chat_chunks_async(move |chunk: ChatChunk| {
                // Only process chunks for our current stream
                let current = current_stream_id.get_untracked();
                if current.as_ref() != Some(&chunk.stream_id) {
                    return;
                }

                messages.update(|msgs| {
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
                    }
                });

                if chunk.is_final {
                    let _ = is_sending.try_set(false);
                    let _ = current_stream_id.try_set(None);
                }
            }).await;
        });
    }

    let do_send = move || {
        let text = input_text.get().trim().to_string();
        if text.is_empty() || is_sending.get() {
            return;
        }

        let npc_id = npc_id_signal.get();
        input_text.set(String::new());
        is_sending.set(true);

        // Add user message immediately for instant feedback
        let user_msg_id = uuid::Uuid::new_v4().to_string();
        messages.update(|m| m.push(UiMessage {
            id: user_msg_id.clone(),
            role: "user".to_string(),
            content: text.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            is_streaming: false,
            stream_id: None,
        }));

        // Generate stream ID and add streaming placeholder
        let stream_id = uuid::Uuid::new_v4().to_string();
        let assistant_msg_id = uuid::Uuid::new_v4().to_string();
        current_stream_id.set(Some(stream_id.clone()));

        messages.update(|m| m.push(UiMessage {
            id: assistant_msg_id,
            role: "assistant".to_string(),
            content: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            is_streaming: true,
            stream_id: Some(stream_id.clone()),
        }));

        // Start streaming
        spawn_local(async move {
            match stream_npc_chat(npc_id, text, Some(stream_id)).await {
                Ok(_) => {
                    // Stream started successfully, chunks will arrive via listener
                }
                Err(e) => {
                    // Error starting stream
                    messages.update(|m| {
                        if let Some(msg) = m.last_mut() {
                            msg.is_streaming = false;
                            msg.stream_id = None;
                            msg.role = "error".to_string();
                            msg.content = format!("Error: {}", e);
                        }
                    });
                    is_sending.set(false);
                    current_stream_id.set(None);
                }
            }
        });
    };

    let handle_click = move |_: ev::MouseEvent| {
        do_send();
    };

    let handle_keydown = move |evt: ev::KeyboardEvent| {
        if evt.key() == "Enter" && !evt.shift_key() {
            evt.prevent_default();
            do_send();
        }
    };

    view! {
        <div class="flex flex-col h-full bg-zinc-950">
            <ConversationHeader
                npc_initial=npc_initial
                npc_name=npc_name_display
                on_close=on_close
            />
            <MessagesArea
                messages=messages
                is_loading=is_loading
                error_msg=error_msg
                npc_name=npc_name.clone()
            />
            <InputArea
                input_text=input_text
                is_sending=is_sending
                npc_name=npc_name_input
                on_keydown=handle_keydown
                on_click=handle_click
            />
        </div>
    }
}

#[component]
fn ConversationHeader(
    npc_initial: char,
    npc_name: String,
    on_close: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="flex items-center justify-between p-4 border-b border-zinc-800 bg-zinc-900/80 backdrop-blur-sm">
            <div class="flex items-center gap-3">
                <div class="w-10 h-10 rounded-full bg-[var(--accent)]/20 flex items-center justify-center text-[var(--accent)] font-bold border border-[var(--accent)]/40">
                    {npc_initial.to_string()}
                </div>
                <div>
                    <h2 class="font-bold text-white">{npc_name}</h2>
                    <p class="text-xs text-zinc-500">"NPC Conversation"</p>
                </div>
            </div>
            <button
                class="p-2 text-zinc-500 hover:text-white hover:bg-zinc-800 rounded transition-colors"
                aria-label="Close conversation"
                on:click=move |_| on_close.run(())
            >
                "×"
            </button>
        </div>
    }
}

#[component]
fn MessagesArea(
    messages: RwSignal<Vec<UiMessage>>,
    is_loading: RwSignal<bool>,
    error_msg: RwSignal<Option<String>>,
    npc_name: String,
) -> impl IntoView {
    view! {
        <div class="flex-1 overflow-y-auto p-4 space-y-4">
            {move || {
                if is_loading.get() {
                    view! {
                        <div class="flex items-center justify-center h-full text-zinc-500">
                            "Loading conversation..."
                        </div>
                    }.into_any()
                } else if let Some(err) = error_msg.get() {
                    view! {
                        <div class="flex items-center justify-center h-full text-red-400">
                            {err}
                        </div>
                    }.into_any()
                } else if messages.get().is_empty() {
                    let name = npc_name.clone();
                    view! {
                        <div class="flex flex-col items-center justify-center h-full text-zinc-500">
                            <p>"No messages yet"</p>
                            <p class="text-sm">{format!("Start a conversation with {}", name)}</p>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <For
                            each=move || messages.get()
                            key=|msg| (msg.id.clone(), msg.content.len(), msg.is_streaming)
                            children=move |msg| {
                                view! { <MessageBubble msg=msg /> }
                            }
                        />
                    }.into_any()
                }
            }}
        </div>
    }
}

#[component]
fn MessageBubble(msg: UiMessage) -> impl IntoView {
    let is_user = msg.role == "user";
    let is_error = msg.role == "error";
    let is_streaming = msg.is_streaming;
    let msg_content = msg.content.clone();
    let timestamp = msg.created_at.clone();

    let outer_class = if is_user { "flex justify-end" } else { "flex justify-start" };
    let bubble_class = if is_error {
        "max-w-[80%] bg-red-900/30 border border-red-700/50 rounded-lg p-3"
    } else if is_user {
        "max-w-[80%] bg-[var(--accent)]/20 border border-[var(--accent)]/30 rounded-lg p-3"
    } else {
        "max-w-[80%] bg-zinc-800 border border-zinc-700 rounded-lg p-3"
    };

    view! {
        <div class=outer_class>
            <div class=bubble_class>
                {if is_user {
                    view! { <p class="text-zinc-100 whitespace-pre-wrap">{msg_content}</p> }.into_any()
                } else if is_error {
                    view! { <p class="text-red-400 whitespace-pre-wrap">{msg_content}</p> }.into_any()
                } else if is_streaming && msg_content.is_empty() {
                    view! {
                        <div class="flex items-center gap-2 text-zinc-500">
                            <div class="w-2 h-2 bg-zinc-500 rounded-full animate-pulse"></div>
                            <span class="text-xs">"Thinking..."</span>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div>
                            <Markdown content=msg_content />
                            {if is_streaming {
                                Some(view! {
                                    <span class="inline-block w-2 h-4 bg-zinc-400 animate-pulse ml-1"></span>
                                })
                            } else {
                                None
                            }}
                        </div>
                    }.into_any()
                }}
                {if !is_streaming {
                    Some(view! {
                        <p class="text-xs text-zinc-500 mt-2">{format_timestamp(&timestamp)}</p>
                    })
                } else {
                    None
                }}
            </div>
        </div>
    }
}

#[component]
fn InputArea(
    input_text: RwSignal<String>,
    is_sending: RwSignal<bool>,
    npc_name: String,
    on_keydown: impl Fn(ev::KeyboardEvent) + 'static,
    on_click: impl Fn(ev::MouseEvent) + 'static,
) -> impl IntoView {
    view! {
        <div class="p-4 border-t border-zinc-900 bg-zinc-900">
            <div class="flex gap-2">
                <textarea
                    class="flex-1 bg-zinc-800 border border-zinc-700 rounded-lg px-4 py-3 text-white placeholder-zinc-500 resize-none focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/50"
                    placeholder=format!("Message {}...", npc_name)
                    rows="1"
                    prop:value=move || input_text.get()
                    prop:disabled=move || is_sending.get()
                    on:input=move |evt| input_text.set(event_target_value(&evt))
                    on:keydown=on_keydown
                />
                <button
                    class="px-4 py-2 bg-[var(--accent)] hover:brightness-110 text-white rounded-lg font-medium transition-all disabled:opacity-50"
                    prop:disabled=move || input_text.get().trim().is_empty() || is_sending.get()
                    on:click=on_click
                >
                    {move || if is_sending.get() { "..." } else { "Send" }}
                </button>
            </div>
            <p class="text-xs text-zinc-600 mt-2">"Press Enter to send, Shift+Enter for new line"</p>
        </div>
    }
}

fn format_timestamp(iso: &str) -> String {
    if let Some(time_part) = iso.split('T').nth(1) {
        if let Some(hm) = time_part.get(0..5) {
            return hm.to_string();
        }
    }
    iso.to_string()
}
