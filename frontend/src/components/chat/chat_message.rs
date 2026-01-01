use dioxus::prelude::*;
use crate::components::design_system::{Markdown, TypingIndicator};
use wasm_bindgen::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ChatMessageProps {
    pub role: String,
    pub content: String,
    pub tokens: Option<(u32, u32)>,
    pub on_play: Option<EventHandler<()>>,
}

#[component]
pub fn ChatMessage(props: ChatMessageProps) -> Element {
    let is_assistant = props.role == "assistant";
    let is_error = props.role == "error";
    let is_user = props.role == "user";

    let container_class = if is_user {
        "bg-blue-900/40 p-3 rounded-lg max-w-3xl ml-auto border border-blue-800"
    } else if is_error {
        "bg-red-900/40 p-3 rounded-lg max-w-3xl border border-red-800"
    } else {
        "bg-[var(--bg-surface)] p-3 rounded-lg max-w-3xl group relative border border-[var(--border-subtle)]"
    };

    let copy_to_clipboard = move |text: String| {
        spawn(async move {
            if let Some(window) = web_sys::window() {
                let navigator = window.navigator();
                let clipboard = navigator.clipboard();
                let _ = wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&text)).await;
            }
        });
    };

    rsx! {
        div {
            class: "{container_class}",
            if is_assistant {
                 div {
                    class: "absolute -left-12 top-1 opacity-0 group-hover:opacity-100 transition-opacity flex flex-col gap-1",
                    // Play Button
                    if let Some(handler) = props.on_play.clone() {
                        button {
                            class: "p-2 bg-zinc-800 rounded-full hover:bg-zinc-700 text-zinc-400 hover:text-white transition-colors shadow-sm",
                            title: "Read Aloud",
                            onclick: move |_| handler.call(()),
                            svg {
                                class: "w-4 h-4",
                                view_box: "0 0 24 24",
                                fill: "currentColor",
                                path { d: "M8 5v14l11-7z" }
                            }
                        }
                    }
                    // Copy Button
                    button {
                        class: "p-2 bg-zinc-800 rounded-full hover:bg-zinc-700 text-zinc-400 hover:text-white transition-colors shadow-sm",
                        title: "Copy",
                        onclick: {
                            let c = props.content.clone();
                            move |_| copy_to_clipboard(c.clone())
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
                class: "min-w-0 break-words prose prose-invert max-w-none text-sm leading-relaxed",
                if is_assistant {
                    Markdown { content: props.content.clone() }
                } else {
                    div { class: "whitespace-pre-wrap text-zinc-200", "{props.content}" }
                }
            }

            if let Some((input, output)) = props.tokens {
                div {
                    class: "text-[10px] text-zinc-500 mt-2 font-mono flex gap-2",
                    span { "IN: {input}" }
                    span { "OUT: {output}" }
                }
            }
        }
    }
}
