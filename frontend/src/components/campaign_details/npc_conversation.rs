use dioxus::prelude::*;
use crate::bindings::{
    get_npc_conversation, add_npc_message, mark_npc_read,
    NpcConversation as NpcConversationData, ConversationMessage,
};
use crate::components::design_system::{TypingIndicator, Markdown};

#[derive(Props, Clone, PartialEq)]
pub struct NpcConversationProps {
    pub npc_id: String,
    pub npc_name: String,
    pub on_close: EventHandler<()>,
}

#[component]
pub fn NpcConversation(props: NpcConversationProps) -> Element {
    let mut messages = use_signal(|| Vec::<ConversationMessage>::new());
    let mut is_loading = use_signal(|| true);
    let mut is_sending = use_signal(|| false);
    let mut input_text = use_signal(|| String::new());
    let mut error_msg = use_signal(|| Option::<String>::None);

    let npc_id_sig = use_signal(|| props.npc_id.clone());
    let npc_name = props.npc_name.clone();

    // Load conversation on mount
    use_effect(move || {
        let npc_id = npc_id_sig.read().clone();
        spawn(async move {
            match get_npc_conversation(npc_id.clone()).await {
                Ok(conv) => {
                    // Parse messages from JSON
                    let parsed: Vec<ConversationMessage> = serde_json::from_str(&conv.messages_json)
                        .unwrap_or_default();
                    messages.set(parsed);
                    // Mark as read
                    let _ = mark_npc_read(npc_id).await;
                }
                Err(e) => {
                    // Conversation might not exist yet - that's OK
                    if !e.contains("not found") {
                        error_msg.set(Some(e));
                    }
                }
            }
            is_loading.set(false);
        });
    });

    let handle_send = move |_| {
        let text = input_text.read().trim().to_string();
        if text.is_empty() || is_sending.read().clone() {
            return;
        }

        input_text.set(String::new());
        is_sending.set(true);
        let npc_id = npc_id_sig.read().clone();

        spawn(async move {
            match add_npc_message(npc_id.clone(), text.clone(), "user".to_string()).await {
                Ok(msg) => {
                    // Add user message to list
                    messages.with_mut(|m| m.push(msg));

                    // TODO: In future, trigger NPC response via LLM here
                    // For now, the backend would need to handle NPC responses
                    // or we simulate a placeholder response
                }
                Err(e) => {
                    error_msg.set(Some(format!("Failed to send: {}", e)));
                }
            }
            is_sending.set(false);
        });
    };

    let handle_keydown = move |e: KeyboardEvent| {
        if e.key() == Key::Enter && !e.modifiers().shift() {
            e.prevent_default();
            handle_send(());
        }
    };

    rsx! {
        div {
            class: "flex flex-col h-full bg-zinc-900",

            // Header
            div { class: "flex items-center justify-between p-4 border-b border-zinc-800 bg-zinc-900/80 backdrop-blur-sm",
                div { class: "flex items-center gap-3",
                    // Avatar
                    div {
                        class: "w-10 h-10 rounded-full bg-[var(--accent)]/20 flex items-center justify-center text-[var(--accent)] font-bold border border-[var(--accent)]/40",
                        "{npc_name.chars().next().unwrap_or('?')}"
                    }
                    div {
                        h2 { class: "font-bold text-white", "{npc_name}" }
                        p { class: "text-xs text-zinc-500", "NPC Conversation" }
                    }
                }
                button {
                    class: "p-2 text-zinc-500 hover:text-white hover:bg-zinc-800 rounded transition-colors",
                    aria_label: "Close conversation",
                    onclick: move |_| props.on_close.call(()),
                    "×"
                }
            }

            // Messages Area
            div { class: "flex-1 overflow-y-auto p-4 space-y-4",
                if is_loading.read().clone() {
                    div { class: "flex items-center justify-center h-full text-zinc-500",
                        "Loading conversation..."
                    }
                } else if let Some(err) = error_msg.read().as_ref() {
                    div { class: "flex items-center justify-center h-full text-red-400",
                        "{err}"
                    }
                } else if messages.read().is_empty() {
                    div { class: "flex flex-col items-center justify-center h-full text-zinc-500",
                        div { class: "text-4xl mb-4 opacity-20", "💬" }
                        p { "No messages yet" }
                        p { class: "text-sm", "Start a conversation with {npc_name}" }
                    }
                } else {
                    for msg in messages.read().iter().cloned() {{
                        let is_user = msg.role == "user";
                        let msg_content = msg.content.clone();
                        let msg_id = msg.id.clone();
                        let timestamp = msg.created_at.clone();

                        rsx! {
                            div {
                                key: "{msg_id}",
                                class: if is_user {
                                    "flex justify-end"
                                } else {
                                    "flex justify-start"
                                },

                                div {
                                    class: if is_user {
                                        "max-w-[80%] bg-[var(--accent)]/20 border border-[var(--accent)]/30 rounded-lg p-3"
                                    } else {
                                        "max-w-[80%] bg-zinc-800 border border-zinc-700 rounded-lg p-3"
                                    },

                                    if is_user {
                                        p { class: "text-zinc-100 whitespace-pre-wrap", "{msg_content}" }
                                    } else {
                                        Markdown { content: msg_content }
                                    }

                                    p {
                                        class: "text-xs text-zinc-500 mt-2",
                                        "{format_timestamp(&timestamp)}"
                                    }
                                }
                            }
                        }
                    }}
                }

                // Typing indicator when sending
                if is_sending.read().clone() {
                    div { class: "flex justify-start",
                        div { class: "bg-zinc-800 border border-zinc-700 rounded-lg p-3 flex items-center gap-2",
                            TypingIndicator {}
                            span { class: "text-xs text-zinc-500", "..." }
                        }
                    }
                }
            }

            // Input Area
            div { class: "p-4 border-t border-zinc-800 bg-zinc-900/80",
                div { class: "flex gap-2",
                    textarea {
                        class: "flex-1 bg-zinc-800 border border-zinc-700 rounded-lg px-4 py-3 text-white placeholder-zinc-500 resize-none focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/50 focus:border-[var(--accent)]",
                        placeholder: "Message {npc_name}...",
                        rows: "1",
                        value: "{input_text}",
                        disabled: is_sending.read().clone(),
                        oninput: move |e| input_text.set(e.value()),
                        onkeydown: handle_keydown,
                    }
                    button {
                        class: "px-4 py-2 bg-[var(--accent)] hover:brightness-110 text-white rounded-lg font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed",
                        disabled: input_text.read().trim().is_empty() || is_sending.read().clone(),
                        onclick: handle_send,
                        if is_sending.read().clone() {
                            "..."
                        } else {
                            "Send"
                        }
                    }
                }
                p { class: "text-xs text-zinc-600 mt-2", "Press Enter to send, Shift+Enter for new line" }
            }
        }
    }
}

/// Format ISO timestamp to a readable format
fn format_timestamp(iso: &str) -> String {
    // Simple formatting - in production would use chrono
    if let Some(time_part) = iso.split('T').nth(1) {
        if let Some(hm) = time_part.get(0..5) {
            return hm.to_string();
        }
    }
    iso.to_string()
}
