//! Messages display component.

use crate::ChatMessage;
use leptos::prelude::*;

#[component]
pub fn Messages(messages: RwSignal<Vec<ChatMessage>>, sending: RwSignal<bool>) -> impl IntoView {
    // Auto-scroll to bottom when messages change
    let messages_ref = NodeRef::<leptos::html::Div>::new();

    Effect::new(move |_| {
        let _ = messages.get();
        if let Some(el) = messages_ref.get() {
            // Scroll to bottom
            let scroll_options = web_sys::ScrollIntoViewOptions::new();
            scroll_options.set_behavior(web_sys::ScrollBehavior::Smooth);
            el.scroll_into_view_with_scroll_into_view_options(&scroll_options);
        }
    });

    view! {
        <div class="messages">
            <Show
                when=move || !messages.get().is_empty()
                fallback=|| {
                    view! {
                        <div class="empty-state">
                            <div class="empty-state__icon">"⊢"</div>
                            <h2 class="empty-state__title">"No messages yet"</h2>
                            <p class="empty-state__hint">
                                "Connect to Claude Desktop and start chatting. "
                                "Make sure Claude Desktop is running with "
                                <code>"--remote-debugging-port=9222"</code>
                            </p>
                        </div>
                    }
                }
            >
                <For
                    each=move || messages.get()
                    key=|msg| format!("{}-{}", msg.role, msg.content.len())
                    children=move |msg| {
                        let class = if msg.is_user() {
                            "message message--user"
                        } else {
                            "message message--assistant"
                        };
                        let label = if msg.is_user() { "You" } else { "Claude" };

                        view! {
                            <div class=class>
                                <span class="message__label">{label}</span>
                                <div class="message__content">{msg.content.clone()}</div>
                            </div>
                        }
                    }
                />

                // Loading indicator when waiting for response
                <Show when=move || sending.get()>
                    <div class="message message--assistant">
                        <span class="message__label">"Claude"</span>
                        <div class="loading">
                            <span class="loading__dots">
                                <span class="loading__dot"></span>
                                <span class="loading__dot"></span>
                                <span class="loading__dot"></span>
                            </span>
                            <span>"Thinking..."</span>
                        </div>
                    </div>
                </Show>

                // Scroll anchor
                <div node_ref=messages_ref style="height: 1px;"></div>
            </Show>
        </div>
    }
}
