use dioxus::prelude::*;
use crate::bindings::{get_voice_queue, QueuedVoice, VoiceStatus};

#[component]
pub fn MediaBar() -> Element {
    let mut queue = use_signal(|| Vec::<QueuedVoice>::new());
    let mut is_playing = use_signal(|| false);
    let mut current_item = use_signal(|| Option::<QueuedVoice>::None);

    // Poll queue status
    use_future(move || async move {
        loop {
            if let Ok(q) = get_voice_queue().await {
                let playing = q.iter().find(|i| i.status == VoiceStatus::Playing);
                if let Some(item) = playing {
                    is_playing.set(true);
                    current_item.set(Some(item.clone()));
                } else {
                    is_playing.set(false);
                    current_item.set(None);
                }
                queue.set(q);
            }
            gloo_timers::future::TimeoutFuture::new(1000).await;
        }
    });

    rsx! {
        div {
            class: "h-full w-full flex items-center justify-between px-4 text-[var(--text-muted)]",

            // Left: Player Controls
            div { class: "flex items-center gap-4",
                button { class: "hover:text-[var(--text-primary)]", "⏮" }
                button {
                    class: format!("w-8 h-8 rounded-full bg-[var(--text-primary)] text-[var(--bg-deep)] flex items-center justify-center hover:scale-105 transition-transform {}", if is_playing() { "animate-pulse" } else { "" }),
                    if is_playing() { "⏸" } else { "▶" }
                }
                button { class: "hover:text-[var(--text-primary)]", "⏭" }
                if let Some(item) = current_item() {
                    span { class: "text-xs font-mono max-w-[200px] truncate", "{item.text}" }
                } else {
                    span { class: "text-xs font-mono", "0:00 / 0:00" }
                }
            }

            // Center: Scrubber (Visual only for now)
            div { class: "flex-1 mx-8 h-1 bg-[var(--bg-surface)] rounded-full overflow-hidden",
                div { class: "w-1/3 h-full bg-[var(--accent)]" }
            }

            // Right: Status
            div { class: "flex items-center gap-4",
                if !queue.read().is_empty() {
                    div { class: "flex items-center gap-2 px-3 py-1 bg-[var(--bg-surface)] rounded-full border border-[var(--border-subtle)]",
                         if is_playing() {
                            div { class: "w-2 h-2 rounded-full bg-green-500 animate-pulse" }
                            span { class: "text-xs font-bold text-[var(--text-primary)]", "SPEAKING" }
                         } else {
                            div { class: "w-2 h-2 rounded-full bg-yellow-500" }
                            span { class: "text-xs font-bold text-[var(--text-primary)]", "QUEUED ({queue.read().len()})" }
                         }
                    }
                }
                button { "🔊" }
            }
        }
    }
}
