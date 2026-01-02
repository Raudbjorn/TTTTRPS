//! MediaBar Component
//!
//! A persistent media control bar anchored to the bottom of the layout.
//! Features:
//!   - Play/pause/skip controls
//!   - Interactive progress scrubber
//!   - Volume control with mute toggle
//!   - Current speaker/voice status indicator
//!   - Voice queue display

use leptos::prelude::*;
use wasm_bindgen::JsCast; // Import JsCast for dyn_ref
use std::ops::Deref; // Import Deref for deref
use wasm_bindgen_futures::spawn_local;
use crate::bindings::{get_voice_queue, QueuedVoice, VoiceStatus};

/// Volume level for the media bar
#[derive(Clone, Copy, PartialEq)]
pub enum VolumeLevel {
    Muted,
    Low,
    Medium,
    High,
}

impl VolumeLevel {
    fn from_value(value: f32) -> Self {
        if value == 0.0 {
            VolumeLevel::Muted
        } else if value < 0.33 {
            VolumeLevel::Low
        } else if value < 0.66 {
            VolumeLevel::Medium
        } else {
            VolumeLevel::High
        }
    }
}

#[component]
pub fn MediaBar() -> impl IntoView {
    let queue = RwSignal::new(Vec::<QueuedVoice>::new());
    let is_playing = RwSignal::new(false);
    let current_item = RwSignal::new(Option::<QueuedVoice>::None);
    let progress = RwSignal::new(0.0_f32); // 0-1 progress
    let volume = RwSignal::new(0.75_f32); // 0-1 volume
    let prev_volume = RwSignal::new(0.75_f32); // For mute toggle
    let is_scrubbing = RwSignal::new(false);
    let show_volume_slider = RwSignal::new(false);

    // Poll queue status
    Effect::new(move |_| {
        spawn_local(async move {
            loop {
                if let Ok(q) = get_voice_queue().await {
                    let playing = q.iter().find(|i| i.status == VoiceStatus::Playing);
                    if let Some(item) = playing {
                        is_playing.set(true);
                        current_item.set(Some(item.clone()));
                        // Simulate progress (would come from actual playback in production)
                        if !is_scrubbing.get() {
                            progress.update(|p| {
                                *p = (*p + 0.01).min(1.0);
                                if *p >= 1.0 {
                                    *p = 0.0;
                                }
                            });
                        }
                    } else {
                        is_playing.set(false);
                        current_item.set(None);
                        progress.set(0.0);
                    }
                    queue.set(q);
                }
                gloo_timers::future::TimeoutFuture::new(1000).await;
            }
        });
    });

    // Volume toggle (mute/unmute)
    let toggle_mute = move |_: web_sys::MouseEvent| {
        let current = volume.get();
        if current > 0.0 {
            prev_volume.set(current);
            volume.set(0.0);
        } else {
            volume.set(prev_volume.get());
        }
    };

    // Volume change handler
    let on_volume_change = move |e: web_sys::Event| {
        if let Some(target) = e.target() {
            if let Some(input) = target.dyn_ref::<web_sys::HtmlInputElement>() {
                if let Ok(val) = input.value().parse::<f32>() {
                    volume.set(val / 100.0);
                }
            }
        }
    };

    // Progress scrubber handlers
    let on_scrub_start = move |_: web_sys::MouseEvent| {
        is_scrubbing.set(true);
    };

    let on_scrub_change = move |e: web_sys::Event| {
        if let Some(target) = e.target() {
            if let Some(input) = target.dyn_ref::<web_sys::HtmlInputElement>() {
                if let Ok(val) = input.value().parse::<f32>() {
                    progress.set(val / 100.0);
                }
            }
        }
    };

    let on_scrub_end = move |_: web_sys::MouseEvent| {
        is_scrubbing.set(false);
        // Would trigger seek in production
    };

    // Volume level for icon
    let volume_level = Signal::derive(move || VolumeLevel::from_value(volume.get()));

    view! {
        <div
            class="h-full w-full flex items-center justify-between px-4 bg-[var(--bg-elevated)] border-t border-[var(--border-subtle)]"
            role="region"
            aria-label="Media controls"
        >
            // Left: Player Controls & Current Track Info
            <div class="flex items-center gap-4 min-w-0 flex-shrink-0">
                // Previous Button
                <button
                    class="p-1.5 text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--accent)] rounded"
                    aria-label="Previous track"
                    title="Previous"
                >
                    <SkipBackIcon />
                </button>

                // Play/Pause Button
                <button
                    class=move || format!(
                        "w-9 h-9 rounded-full flex items-center justify-center transition-all focus:outline-none focus:ring-2 focus:ring-[var(--accent)] {}",
                        if is_playing.get() {
                            "bg-[var(--accent)] text-[var(--bg-deep)] hover:bg-[var(--accent-hover)]"
                        } else {
                            "bg-[var(--text-primary)] text-[var(--bg-deep)] hover:scale-105"
                        }
                    )
                    aria-label=move || if is_playing.get() { "Pause" } else { "Play" }
                >
                    {move || if is_playing.get() {
                        view! { <PauseIcon /> }.into_any()
                    } else {
                        view! { <PlayIcon /> }.into_any()
                    }}
                </button>

                // Next Button
                <button
                    class="p-1.5 text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--accent)] rounded"
                    aria-label="Next track"
                    title="Next"
                >
                    <SkipForwardIcon />
                </button>

                // Current Track Info
                <div class="ml-2 min-w-0">
                    {move || {
                        if let Some(item) = current_item.get() {
                            view! {
                                <div class="flex flex-col min-w-0">
                                    <span class="text-xs font-medium text-[var(--text-primary)] truncate max-w-[180px]">
                                        {item.text.chars().take(40).collect::<String>()}
                                        {if item.text.len() > 40 { "..." } else { "" }}
                                    </span>
                                    <span class="text-[10px] text-[var(--text-muted)]">
                                        "Speaking..."
                                    </span>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <span class="text-xs text-[var(--text-muted)] font-mono">
                                    "No audio playing"
                                </span>
                            }.into_any()
                        }
                    }}
                </div>
            </div>

            // Center: Progress Scrubber
            <div class="flex-1 mx-8 flex items-center gap-3">
                // Current time
                <span class="text-[10px] font-mono text-[var(--text-muted)] w-10 text-right">
                    {move || format_time(progress.get() * 60.0)} // Assuming 60s max for demo
                </span>

                // Scrubber
                <div class="flex-1 relative group">
                    <input
                        type="range"
                        min="0"
                        max="100"
                        step="1"
                        prop:value=move || (progress.get() * 100.0) as i32
                        class="w-full h-1 bg-[var(--bg-surface)] rounded-full appearance-none cursor-pointer
                               [&::-webkit-slider-thumb]:appearance-none
                               [&::-webkit-slider-thumb]:w-3
                               [&::-webkit-slider-thumb]:h-3
                               [&::-webkit-slider-thumb]:rounded-full
                               [&::-webkit-slider-thumb]:bg-[var(--accent)]
                               [&::-webkit-slider-thumb]:opacity-0
                               [&::-webkit-slider-thumb]:group-hover:opacity-100
                               [&::-webkit-slider-thumb]:transition-opacity
                               [&::-webkit-slider-thumb]:cursor-pointer
                               [&::-moz-range-thumb]:w-3
                               [&::-moz-range-thumb]:h-3
                               [&::-moz-range-thumb]:rounded-full
                               [&::-moz-range-thumb]:bg-[var(--accent)]
                               [&::-moz-range-thumb]:border-0
                               [&::-moz-range-thumb]:cursor-pointer
                               focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
                        aria-label="Playback progress"
                        on:mousedown=on_scrub_start
                        on:mouseup=on_scrub_end
                        on:input=on_scrub_change
                    />
                    // Progress fill overlay
                    <div
                        class="absolute top-0 left-0 h-1 bg-[var(--accent)] rounded-full pointer-events-none"
                        style:width=move || format!("{}%", progress.get() * 100.0)
                    ></div>
                </div>

                // Total time
                <span class="text-[10px] font-mono text-[var(--text-muted)] w-10">
                    "1:00"
                </span>
            </div>

            // Right: Volume & Status
            <div class="flex items-center gap-4 flex-shrink-0">
                // Queue Status
                {move || {
                    let q = queue.get();
                    if !q.is_empty() {
                        if is_playing.get() {
                            view! {
                                <div class="flex items-center gap-2 px-3 py-1 bg-[var(--bg-surface)] rounded-full border border-[var(--border-subtle)]">
                                    <div class="w-2 h-2 rounded-full bg-green-500 animate-pulse"></div>
                                    <span class="text-[10px] font-bold text-[var(--text-primary)] uppercase tracking-wide">
                                        "Speaking"
                                    </span>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="flex items-center gap-2 px-3 py-1 bg-[var(--bg-surface)] rounded-full border border-[var(--border-subtle)]">
                                    <div class="w-2 h-2 rounded-full bg-yellow-500"></div>
                                    <span class="text-[10px] font-bold text-[var(--text-primary)] uppercase tracking-wide">
                                        {format!("Queued ({})", q.len())}
                                    </span>
                                </div>
                            }.into_any()
                        }
                    } else {
                        view! { <div class="w-24"></div> }.into_any()
                    }
                }}

                // Volume Control Group
                <div
                    class="relative flex items-center gap-2"
                    on:mouseenter=move |_| show_volume_slider.set(true)
                    on:mouseleave=move |_| show_volume_slider.set(false)
                >
                    // Volume Button
                    <button
                        class="p-1.5 text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--accent)] rounded"
                        aria-label=move || {
                            match volume_level.get() {
                                VolumeLevel::Muted => "Unmute",
                                _ => "Mute",
                            }
                        }
                        on:click=toggle_mute
                    >
                        {move || match volume_level.get() {
                            VolumeLevel::Muted => view! { <VolumeMutedIcon /> }.into_any(),
                            VolumeLevel::Low => view! { <VolumeLowIcon /> }.into_any(),
                            VolumeLevel::Medium => view! { <VolumeMediumIcon /> }.into_any(),
                            VolumeLevel::High => view! { <VolumeHighIcon /> }.into_any(),
                        }}
                    </button>

                    // Volume Slider (shown on hover)
                    <div
                        class=move || format!(
                            "absolute bottom-full right-0 mb-2 p-2 bg-[var(--bg-elevated)] rounded-lg border border-[var(--border-subtle)] shadow-lg transition-opacity {}",
                            if show_volume_slider.get() { "opacity-100" } else { "opacity-0 pointer-events-none" }
                        )
                    >
                        <div class="flex flex-col items-center h-24">
                            <input
                                type="range"
                                min="0"
                                max="100"
                                step="1"
                                prop:value=move || (volume.get() * 100.0) as i32
                                class="w-1 h-20 bg-[var(--bg-surface)] rounded-full appearance-none cursor-pointer
                                       [writing-mode:vertical-lr]
                                       [direction:rtl]
                                       [&::-webkit-slider-thumb]:appearance-none
                                       [&::-webkit-slider-thumb]:w-3
                                       [&::-webkit-slider-thumb]:h-3
                                       [&::-webkit-slider-thumb]:rounded-full
                                       [&::-webkit-slider-thumb]:bg-[var(--accent)]
                                       [&::-webkit-slider-thumb]:cursor-pointer
                                       [&::-moz-range-thumb]:w-3
                                       [&::-moz-range-thumb]:h-3
                                       [&::-moz-range-thumb]:rounded-full
                                       [&::-moz-range-thumb]:bg-[var(--accent)]
                                       [&::-moz-range-thumb]:border-0
                                       focus:outline-none"
                                aria-label="Volume"
                                on:input=on_volume_change
                            />
                            <span class="text-[10px] text-[var(--text-muted)] mt-1">
                                {move || format!("{}%", (volume.get() * 100.0) as i32)}
                            </span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Format seconds to MM:SS
fn format_time(seconds: f32) -> String {
    let mins = (seconds / 60.0).floor() as u32;
    let secs = (seconds % 60.0).floor() as u32;
    format!("{}:{:02}", mins, secs)
}

// SVG Icon Components

#[component]
fn PlayIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <polygon points="5 3 19 12 5 21 5 3"></polygon>
        </svg>
    }
}

#[component]
fn PauseIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <rect x="6" y="4" width="4" height="16"></rect>
            <rect x="14" y="4" width="4" height="16"></rect>
        </svg>
    }
}

#[component]
fn SkipBackIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polygon points="19 20 9 12 19 4 19 20"></polygon>
            <line x1="5" y1="19" x2="5" y2="5"></line>
        </svg>
    }
}

#[component]
fn SkipForwardIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polygon points="5 4 15 12 5 20 5 4"></polygon>
            <line x1="19" y1="5" x2="19" y2="19"></line>
        </svg>
    }
}

#[component]
fn VolumeMutedIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon>
            <line x1="23" y1="9" x2="17" y2="15"></line>
            <line x1="17" y1="9" x2="23" y2="15"></line>
        </svg>
    }
}

#[component]
fn VolumeLowIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon>
            <path d="M15.54 8.46a5 5 0 0 1 0 7.07"></path>
        </svg>
    }
}

#[component]
fn VolumeMediumIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon>
            <path d="M15.54 8.46a5 5 0 0 1 0 7.07"></path>
            <path d="M19.07 4.93a10 10 0 0 1 0 14.14"></path>
        </svg>
    }
}

#[component]
fn VolumeHighIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon>
            <path d="M19.07 4.93a10 10 0 0 1 0 14.14M15.54 8.46a5 5 0 0 1 0 7.07"></path>
        </svg>
    }
}
