use leptos::prelude::*;
use leptos::ev;
use wasm_bindgen_futures::spawn_local;
use crate::bindings::{
    get_npc_conversation, add_npc_message, mark_npc_read, reply_as_npc,
    ConversationMessage,
};
use crate::components::design_system::Markdown;

/// NPC Conversation component for chat-style messaging with NPCs
#[component]
pub fn NpcConversation(
    /// NPC ID to load conversation for
    npc_id: String,
    /// NPC name for display
    npc_name: String,
    /// Callback when the conversation is closed
    on_close: Callback<()>,
) -> impl IntoView {
    let messages = RwSignal::new(Vec::<ConversationMessage>::new());
    let is_loading = RwSignal::new(true);
    let is_sending = RwSignal::new(false);
    let is_typing = RwSignal::new(false);
    let input_text = RwSignal::new(String::new());
    let error_msg = RwSignal::new(Option::<String>::None);

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
                    messages.set(parsed);
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

    let do_send = move || {
        let text = input_text.get().trim().to_string();
        if text.is_empty() || is_sending.get() {
            return;
        }

        input_text.set(String::new());
        is_sending.set(true);
        let npc_id = npc_id_signal.get();

        spawn_local(async move {
            match add_npc_message(npc_id.clone(), text.clone(), "user".to_string(), None).await {
                Ok(msg) => {
                    messages.update(|m| m.push(msg));
                    is_typing.set(true);
                    match reply_as_npc(npc_id.clone()).await {
                        Ok(ai_msg) => {
                            messages.update(|m| m.push(ai_msg));
                        }
                        Err(e) => {
                            web_sys::console::log_1(&format!("NPC failed to reply: {}", e).into());
                        }
                    }
                    is_typing.set(false);
                }
                Err(e) => {
                    error_msg.set(Some(format!("Failed to send: {}", e)));
                }
            }
            is_sending.set(false);
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
                is_typing=is_typing
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
    messages: RwSignal<Vec<ConversationMessage>>,
    is_loading: RwSignal<bool>,
    is_typing: RwSignal<bool>,
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
                            key=|msg| msg.id.clone()
                            children=move |msg| {
                                view! { <MessageBubble msg=msg /> }
                            }
                        />
                    }.into_any()
                }
            }}
            <Show when=move || is_typing.get() fallback=|| ()>
                <div class="flex justify-start">
                    <div class="bg-zinc-800 border border-zinc-700 rounded-lg p-3">
                        <span class="text-xs text-zinc-500">"Typing..."</span>
                    </div>
                </div>
            </Show>
        </div>
    }
}

#[component]
fn MessageBubble(msg: ConversationMessage) -> impl IntoView {
    let is_user = msg.role == "user";
    let msg_content = msg.content.clone();
    let timestamp = msg.created_at.clone();

    let outer_class = if is_user { "flex justify-end" } else { "flex justify-start" };
    let bubble_class = if is_user {
        "max-w-[80%] bg-[var(--accent)]/20 border border-[var(--accent)]/30 rounded-lg p-3"
    } else {
        "max-w-[80%] bg-zinc-800 border border-zinc-700 rounded-lg p-3"
    };

    view! {
        <div class=outer_class>
            <div class=bubble_class>
                {if is_user {
                    view! { <p class="text-zinc-100 whitespace-pre-wrap">{msg_content}</p> }.into_any()
                } else {
                    view! { <Markdown content=msg_content /> }.into_any()
                }}
                <p class="text-xs text-zinc-500 mt-2">{format_timestamp(&timestamp)}</p>
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
