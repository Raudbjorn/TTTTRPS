use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;
use crate::bindings::{get_voice_queue, QueuedVoice, VoiceStatus};

#[component]
pub fn MediaBar() -> impl IntoView {
    let queue = RwSignal::new(Vec::<QueuedVoice>::new());
    let is_playing = RwSignal::new(false);
    let current_item = RwSignal::new(Option::<QueuedVoice>::None);

    // Poll queue status
    Effect::new(move |_| {
        spawn_local(async move {
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
                // Wait 1 second before polling again
                gloo_timers::future::TimeoutFuture::new(1000).await;
            }
        });
    });

    view! {
        <div class="h-full w-full flex items-center justify-between px-4 text-[var(--text-muted)]">
            // Left: Player Controls
            <div class="flex items-center gap-4">
                <button class="hover:text-[var(--text-primary)]">"⏮"</button>
                <button
                    class=move || format!(
                        "w-8 h-8 rounded-full bg-[var(--text-primary)] text-[var(--bg-deep)] flex items-center justify-center hover:scale-105 transition-transform {}",
                        if is_playing.get() { "animate-pulse" } else { "" }
                    )
                >
                    {move || if is_playing.get() { "⏸" } else { "▶" }}
                </button>
                <button class="hover:text-[var(--text-primary)]">"⏭"</button>
                {move || {
                    if let Some(item) = current_item.get() {
                        view! {
                            <span class="text-xs font-mono max-w-[200px] truncate">{item.text}</span>
                        }.into_any()
                    } else {
                        view! {
                            <span class="text-xs font-mono">"0:00 / 0:00"</span>
                        }.into_any()
                    }
                }}
            </div>

            // Center: Scrubber (Visual only for now)
            <div class="flex-1 mx-8 h-1 bg-[var(--bg-surface)] rounded-full overflow-hidden">
                <div class="w-1/3 h-full bg-[var(--accent)]"></div>
            </div>

            // Right: Status
            <div class="flex items-center gap-4">
                {move || {
                    let q = queue.get();
                    if !q.is_empty() {
                        if is_playing.get() {
                            view! {
                                <div class="flex items-center gap-2 px-3 py-1 bg-[var(--bg-surface)] rounded-full border border-[var(--border-subtle)]">
                                    <div class="w-2 h-2 rounded-full bg-green-500 animate-pulse"></div>
                                    <span class="text-xs font-bold text-[var(--text-primary)]">"SPEAKING"</span>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="flex items-center gap-2 px-3 py-1 bg-[var(--bg-surface)] rounded-full border border-[var(--border-subtle)]">
                                    <div class="w-2 h-2 rounded-full bg-yellow-500"></div>
                                    <span class="text-xs font-bold text-[var(--text-primary)]">
                                        {format!("QUEUED ({})", q.len())}
                                    </span>
                                </div>
                            }.into_any()
                        }
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}
                <button>"🔊"</button>
            </div>
        </div>
    }
}
