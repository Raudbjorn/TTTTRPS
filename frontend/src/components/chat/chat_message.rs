use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::components::design_system::{Markdown, TypingIndicator};

/// A single chat message with role-based styling and streaming support
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
    /// Whether this message is currently being streamed
    #[prop(default = false)]
    is_streaming: bool,
    /// Optional callback to play the message via TTS
    #[prop(default = None)]
    on_play: Option<Callback<()>>,
    /// Whether to show token usage (as tooltip)
    #[prop(default = false)]
    show_tokens: bool,
) -> impl IntoView {
    let is_assistant = role == "assistant";
    let is_error = role == "error";
    let is_user = role == "user";

    let container_class = if is_user {
        "bg-blue-900/40 p-3 rounded-lg max-w-3xl ml-auto border border-blue-800"
    } else if is_error {
        "bg-red-900/40 p-3 rounded-lg max-w-3xl border border-red-800"
    } else if is_streaming {
        "bg-[var(--bg-surface)] p-3 rounded-lg max-w-3xl group relative border border-blue-500/50 animate-pulse"
    } else {
        "bg-[var(--bg-surface)] p-3 rounded-lg max-w-3xl group relative border border-[var(--border-subtle)]"
    };

    // Clone content for various uses
    let content_for_clipboard = content.clone();
    let content_for_display = content.clone();
    let content_for_user = content.clone();
    let content_for_streaming = content.clone();
    let content_is_empty = content.is_empty();

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

    // Build action buttons for assistant messages (only when not streaming)
    let action_buttons = if is_assistant && !is_streaming {
        let play_button = on_play.map(|handler| {
            view! {
                <button
                    class="p-1.5 rounded hover:bg-zinc-700 text-zinc-400 hover:text-green-400 transition-colors"
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
            <div class="flex items-center gap-1 mt-2 pt-2 border-t border-zinc-700/50">
                {play_button}
                <button
                    class="p-1.5 rounded hover:bg-zinc-700 text-zinc-400 hover:text-blue-400 transition-colors"
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

    // Build message content based on role and streaming state
    let message_content = if is_streaming && content_is_empty {
        // Show typing indicator when streaming with no content yet
        view! {
            <div class="flex items-center gap-2">
                <TypingIndicator />
                <span class="text-xs text-zinc-500">"Thinking..."</span>
            </div>
        }
        .into_any()
    } else if is_streaming {
        // Show content with a blinking cursor at the end
        view! {
            <div class="streaming-content">
                <Markdown content=content_for_streaming />
                <span class="inline-block w-2 h-4 bg-blue-400 animate-blink ml-0.5 align-text-bottom"></span>
            </div>
        }.into_any()
    } else if is_assistant {
        view! { <Markdown content=content_for_display /> }.into_any()
    } else {
        view! {
            <div class="whitespace-pre-wrap text-zinc-200">{content_for_user}</div>
        }
        .into_any()
    };

    // Build token tooltip (only when show_tokens is enabled and not streaming)
    let token_tooltip = if show_tokens && !is_streaming {
        tokens.map(|(input, output)| format!("Tokens: {} in / {} out", input, output))
    } else {
        None
    };

    // Streaming indicator
    let streaming_indicator = if is_streaming && !content_is_empty {
        Some(view! {
            <div class="text-[10px] text-blue-400 mt-2 flex items-center gap-1">
                <svg class="w-3 h-3 animate-spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="10" stroke-opacity="0.25" />
                    <path d="M12 2a10 10 0 0 1 10 10" stroke-linecap="round" />
                </svg>
                <span>"Streaming..."</span>
            </div>
        })
    } else {
        None
    };

    view! {
        <div class=container_class title=token_tooltip>
            <div class="min-w-0 break-words prose prose-invert max-w-none text-sm leading-relaxed">
                {message_content}
            </div>
            {streaming_indicator}
            {action_buttons}
        </div>
    }
}
