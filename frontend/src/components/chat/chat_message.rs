use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::components::design_system::Markdown;

/// A single chat message with role-based styling
#[component]
pub fn ChatMessage(
    /// The role of the message sender: "user", "assistant", or "error"
    #[prop(into)]
    role: String,
    /// The message content
    #[prop(into)]
    content: String,
    /// Optional token usage (input, output) - passed directly as Option
    #[prop(default = None)]
    tokens: Option<(u32, u32)>,
    /// Optional callback to play the message via TTS
    #[prop(default = None)]
    on_play: Option<Callback<()>>,
) -> impl IntoView {
    let is_assistant = role == "assistant";
    let is_error = role == "error";
    let is_user = role == "user";

    let container_class = if is_user {
        "bg-blue-900/40 p-3 rounded-lg max-w-3xl ml-auto border border-blue-800"
    } else if is_error {
        "bg-red-900/40 p-3 rounded-lg max-w-3xl border border-red-800"
    } else {
        "bg-[var(--bg-surface)] p-3 rounded-lg max-w-3xl group relative border border-[var(--border-subtle)]"
    };

    // Clone content for various uses
    let content_for_clipboard = content.clone();
    let content_for_display = content.clone();
    let content_for_user = content.clone();

    // Copy to clipboard handler
    let copy_to_clipboard = {
        let text = content_for_clipboard.clone();
        Callback::new(move |_: ev::MouseEvent| {
            let text = text.clone();
            spawn_local(async move {
                if let Some(window) = web_sys::window() {
                    let navigator = window.navigator();
                    let clipboard = navigator.clipboard();
                    let _ = wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&text)).await;
                }
            });
        })
    };

    // Build action buttons for assistant messages
    let action_buttons = if is_assistant {
        let play_button = on_play.map(|handler| {
            view! {
                <button
                    class="p-2 bg-zinc-800 rounded-full hover:bg-zinc-700 text-zinc-400 hover:text-white transition-colors shadow-sm"
                    title="Read Aloud"
                    on:click=move |_| handler.run(())
                >
                    <svg class="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M8 5v14l11-7z" />
                    </svg>
                </button>
            }
        });

        let copy_handler = copy_to_clipboard.clone();
        Some(view! {
            <div class="absolute -left-12 top-1 opacity-0 group-hover:opacity-100 transition-opacity flex flex-col gap-1">
                {play_button}
                <button
                    class="p-2 bg-zinc-800 rounded-full hover:bg-zinc-700 text-zinc-400 hover:text-white transition-colors shadow-sm"
                    title="Copy"
                    on:click=move |e| copy_handler.run(e)
                >
                    <svg
                        class="w-4 h-4"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                    </svg>
                </button>
            </div>
        })
    } else {
        None
    };

    // Build message content based on role
    let message_content = if is_assistant {
        view! { <Markdown content=content_for_display /> }.into_any()
    } else {
        view! {
            <div class="whitespace-pre-wrap text-zinc-200">{content_for_user}</div>
        }
        .into_any()
    };

    // Build token display
    let token_display = tokens.map(|(input, output)| {
        view! {
            <div class="text-[10px] text-zinc-500 mt-2 font-mono flex gap-2">
                <span>"IN: " {input}</span>
                <span>"OUT: " {output}</span>
            </div>
        }
    });

    view! {
        <div class=container_class>
            {action_buttons}
            <div class="min-w-0 break-words prose prose-invert max-w-none text-sm leading-relaxed">
                {message_content}
            </div>
            {token_display}
        </div>
    }
}
