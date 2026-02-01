//! Session Recap Viewer Component
//!
//! Phase 8 of the Campaign Generation Overhaul.
//!
//! Displays session recaps with:
//! - Read-aloud prose with copy button
//! - Bullet summary with checkboxes
//! - Cliffhanger highlight
//! - Editing support
//! - PC knowledge filtering

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Types (mirrored from backend)
// ============================================================================

/// Entity reference for NPCs/Locations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityReference {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub role: Option<String>,
}

/// Recap status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecapStatus {
    Pending,
    Generating,
    Complete,
    Failed,
    Edited,
}

/// Session recap data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecap {
    pub id: String,
    pub session_id: String,
    pub campaign_id: String,
    pub prose: Option<String>,
    pub bullets: Vec<String>,
    pub cliffhanger: Option<String>,
    pub key_npcs: Vec<EntityReference>,
    pub key_locations: Vec<EntityReference>,
    pub key_events: Vec<String>,
    pub status: RecapStatus,
    pub generated_at: Option<String>,
    pub edited_at: Option<String>,
}

/// PC filter options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PCFilter {
    pub character_id: String,
    pub character_name: String,
}

// ============================================================================
// Recap Viewer Component
// ============================================================================

/// Main recap viewer component
#[component]
pub fn RecapViewer(
    /// The recap to display
    recap: SessionRecap,
    /// Whether editing is enabled
    #[prop(default = false)]
    editable: bool,
    /// Available PC filters
    #[prop(optional)]
    pc_filters: Option<Vec<PCFilter>>,
    /// Callback when recap is edited
    #[prop(optional)]
    on_save: Option<Callback<SessionRecap>>,
) -> impl IntoView {
    // Clone fields for use in closures
    let initial_prose = recap.prose.clone();
    let initial_bullets = recap.bullets.clone();
    let initial_cliffhanger = recap.cliffhanger.clone();

    // State for editing
    let is_editing = RwSignal::new(false);
    let edited_prose = RwSignal::new(initial_prose.clone().unwrap_or_default());
    let edited_bullets = RwSignal::new(initial_bullets.clone());
    let edited_cliffhanger = RwSignal::new(initial_cliffhanger.clone().unwrap_or_default());

    // State for active view tab
    let active_tab = RwSignal::new(RecapTab::Prose);

    // State for selected PC filter
    let selected_pc = RwSignal::<Option<String>>::new(None);

    // Copy to clipboard handler (reserved for future toolbar)
    let _handle_copy = {
        let prose = recap.prose.clone();
        move |_: leptos::ev::MouseEvent| {
            if let Some(text) = &prose {
                copy_to_clipboard(text);
            }
        }
    };

    // Save handler
    let handle_save = {
        let recap_id = recap.id.clone();
        let session_id = recap.session_id.clone();
        let campaign_id = recap.campaign_id.clone();
        let key_npcs = recap.key_npcs.clone();
        let key_locations = recap.key_locations.clone();
        let key_events = recap.key_events.clone();
        let on_save = on_save.clone();

        Callback::new(move |_| {
            let updated = SessionRecap {
                id: recap_id.clone(),
                session_id: session_id.clone(),
                campaign_id: campaign_id.clone(),
                prose: Some(edited_prose.get()),
                bullets: edited_bullets.get(),
                cliffhanger: Some(edited_cliffhanger.get()),
                key_npcs: key_npcs.clone(),
                key_locations: key_locations.clone(),
                key_events: key_events.clone(),
                status: RecapStatus::Edited,
                generated_at: None,
                edited_at: Some(chrono::Utc::now().to_rfc3339()),
            };

            if let Some(callback) = &on_save {
                callback.run(updated);
            }

            is_editing.set(false);
        })
    };

    view! {
        <div class="recap-viewer bg-zinc-900 rounded-xl border border-zinc-800 overflow-hidden">
            // Header with tabs and actions
            <div class="flex items-center justify-between px-4 py-3 bg-zinc-800/50 border-b border-zinc-700">
                // Tab buttons
                <div class="flex gap-1">
                    <TabButton
                        label="Read-Aloud"
                        active=Signal::derive(move || active_tab.get() == RecapTab::Prose)
                        on_click=move |_| active_tab.set(RecapTab::Prose)
                    />
                    <TabButton
                        label="Bullet Points"
                        active=Signal::derive(move || active_tab.get() == RecapTab::Bullets)
                        on_click=move |_| active_tab.set(RecapTab::Bullets)
                    />
                    <TabButton
                        label="Key Info"
                        active=Signal::derive(move || active_tab.get() == RecapTab::KeyInfo)
                        on_click=move |_| active_tab.set(RecapTab::KeyInfo)
                    />
                </div>

                // Actions
                <div class="flex items-center gap-2">
                    // PC filter dropdown
                    {pc_filters.clone().map(|filters| view! {
                        <select
                            class="px-3 py-1.5 bg-zinc-800 border border-zinc-700 rounded-lg text-sm text-zinc-300 focus:outline-none focus:border-purple-500"
                            on:change=move |ev| {
                                let value = event_target_value(&ev);
                                selected_pc.set(if value.is_empty() { None } else { Some(value) });
                            }
                        >
                            <option value="">"All Players"</option>
                            {filters.into_iter().map(|pc| view! {
                                <option value={pc.character_id.clone()}>
                                    {pc.character_name}
                                </option>
                            }).collect::<Vec<_>>()}
                        </select>
                    })}

                    // Edit/Save buttons
                    <Show when=move || editable>
                        <Show
                            when=move || is_editing.get()
                            fallback=move || view! {
                                <button
                                    class="px-3 py-1.5 bg-zinc-700 hover:bg-zinc-600 text-white rounded-lg text-sm transition-colors"
                                    on:click=move |_| is_editing.set(true)
                                >
                                    "Edit"
                                </button>
                            }
                        >
                            <button
                                class="px-3 py-1.5 bg-zinc-700 hover:bg-zinc-600 text-white rounded-lg text-sm transition-colors"
                                on:click=move |_| is_editing.set(false)
                            >
                                "Cancel"
                            </button>
                            <button
                                class="px-3 py-1.5 bg-green-600 hover:bg-green-500 text-white rounded-lg text-sm transition-colors"
                                on:click=move |ev| handle_save.run(ev)
                            >
                                "Save"
                            </button>
                        </Show>
                    </Show>
                </div>
            </div>

            // Status indicator
            <StatusBadge status=recap.status />

            // Content area
            <div class="p-4">
                // Prose tab
                {
                    let prose_for_display = initial_prose.clone();
                    let prose_signal = RwSignal::new(prose_for_display.clone().unwrap_or_default());
                    let handle_copy_cb = Callback::new(move |_: leptos::ev::MouseEvent| {
                        if let Some(text) = &prose_for_display {
                            copy_to_clipboard(text);
                        }
                    });
                    view! {
                        <Show when=move || active_tab.get() == RecapTab::Prose>
                            <ProseView
                                prose=Signal::derive(move || {
                                    if is_editing.get() {
                                        edited_prose.get()
                                    } else {
                                        prose_signal.get()
                                    }
                                })
                                is_editing=is_editing
                                on_change=Callback::new(move |text: String| edited_prose.set(text))
                                on_copy=handle_copy_cb
                            />
                        </Show>
                    }
                }

                // Bullets tab
                {
                    let bullets_for_display = initial_bullets.clone();
                    let bullets_signal = RwSignal::new(bullets_for_display);
                    view! {
                        <Show when=move || active_tab.get() == RecapTab::Bullets>
                            <BulletsView
                                bullets=Signal::derive(move || {
                                    if is_editing.get() {
                                        edited_bullets.get()
                                    } else {
                                        bullets_signal.get()
                                    }
                                })
                                is_editing=is_editing
                                on_change=Callback::new(move |bullets: Vec<String>| edited_bullets.set(bullets))
                            />
                        </Show>
                    }
                }

                // Key info tab
                <Show when=move || active_tab.get() == RecapTab::KeyInfo>
                    <KeyInfoView
                        npcs=recap.key_npcs.clone()
                        locations=recap.key_locations.clone()
                        events=recap.key_events.clone()
                    />
                </Show>
            </div>

            // Cliffhanger section
            {initial_cliffhanger.clone().map(|cliffhanger| {
                let cliffhanger_signal = RwSignal::new(cliffhanger);
                view! {
                    <CliffhangerSection
                        cliffhanger=Signal::derive(move || {
                            if is_editing.get() {
                                edited_cliffhanger.get()
                            } else {
                                cliffhanger_signal.get()
                            }
                        })
                        is_editing=is_editing
                        on_change=Callback::new(move |text: String| edited_cliffhanger.set(text))
                    />
                }
            })}
        </div>
    }
}

// ============================================================================
// Sub-Components
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecapTab {
    Prose,
    Bullets,
    KeyInfo,
}

/// Tab button component
#[component]
fn TabButton(
    label: &'static str,
    active: Signal<bool>,
    on_click: impl Fn(leptos::ev::MouseEvent) + 'static,
) -> impl IntoView {
    view! {
        <button
            class=move || format!(
                "px-3 py-1.5 rounded-lg text-sm font-medium transition-colors {}",
                if active.get() {
                    "bg-purple-600 text-white"
                } else {
                    "text-zinc-400 hover:text-white hover:bg-zinc-700"
                }
            )
            on:click=on_click
        >
            {label}
        </button>
    }
}

/// Status badge component
#[component]
fn StatusBadge(status: RecapStatus) -> impl IntoView {
    let (bg_class, text_class, label) = match status {
        RecapStatus::Pending => ("bg-yellow-900/30", "text-yellow-400", "Pending"),
        RecapStatus::Generating => ("bg-blue-900/30", "text-blue-400", "Generating..."),
        RecapStatus::Complete => ("bg-green-900/30", "text-green-400", "Complete"),
        RecapStatus::Failed => ("bg-red-900/30", "text-red-400", "Failed"),
        RecapStatus::Edited => ("bg-purple-900/30", "text-purple-400", "Edited"),
    };

    view! {
        <div class=format!("px-4 py-2 border-b border-zinc-800 {}", bg_class)>
            <span class=format!("text-sm {}", text_class)>
                {label}
            </span>
        </div>
    }
}

/// Prose view component
#[component]
fn ProseView(
    prose: Signal<String>,
    is_editing: RwSignal<bool>,
    on_change: Callback<String>,
    on_copy: Callback<leptos::ev::MouseEvent>,
) -> impl IntoView {
    view! {
        <div class="prose-view">
            <div class="flex items-center justify-between mb-3">
                <h4 class="text-sm font-medium text-zinc-400">"Read-Aloud Version"</h4>
                <button
                    class="flex items-center gap-1 px-2 py-1 text-xs text-zinc-400 hover:text-white hover:bg-zinc-800 rounded transition-colors"
                    on:click=move |ev| on_copy.run(ev)
                    title="Copy to clipboard"
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                    </svg>
                    "Copy"
                </button>
            </div>

            <Show
                when=move || is_editing.get()
                fallback=move || view! {
                    <div class="px-4 py-3 bg-zinc-800/50 rounded-lg">
                        <p class="text-zinc-200 leading-relaxed whitespace-pre-wrap">
                            {prose}
                        </p>
                    </div>
                }
            >
                <textarea
                    class="w-full h-64 px-4 py-3 bg-zinc-800 border border-zinc-700 rounded-lg text-white placeholder-zinc-500 focus:outline-none focus:border-purple-500 resize-none"
                    prop:value=prose
                    on:input=move |ev| on_change.run(event_target_value(&ev))
                />
            </Show>
        </div>
    }
}

/// Bullets view component
#[component]
fn BulletsView(
    bullets: Signal<Vec<String>>,
    is_editing: RwSignal<bool>,
    on_change: Callback<Vec<String>>,
) -> impl IntoView {
    // Track checked items for reading
    let checked_items = RwSignal::<Vec<usize>>::new(Vec::new());

    view! {
        <div class="bullets-view">
            <h4 class="text-sm font-medium text-zinc-400 mb-3">"Key Points"</h4>

            <Show
                when=move || is_editing.get()
                fallback=move || {
                    let current_bullets = bullets.get();
                    view! {
                        <ul class="space-y-2">
                            {current_bullets.into_iter().enumerate().map(|(i, bullet)| {
                                let is_checked = Signal::derive(move || checked_items.get().contains(&i));
                                view! {
                                    <li class="flex items-start gap-3">
                                        <input
                                            type="checkbox"
                                            class="mt-1.5 w-4 h-4 rounded border-zinc-600 bg-zinc-800 text-purple-600 focus:ring-purple-500"
                                            prop:checked=is_checked
                                            on:change=move |_| {
                                                checked_items.update(|items| {
                                                    if items.contains(&i) {
                                                        items.retain(|&x| x != i);
                                                    } else {
                                                        items.push(i);
                                                    }
                                                });
                                            }
                                        />
                                        <span class=move || format!(
                                            "text-zinc-200 {}",
                                            if is_checked.get() { "line-through opacity-50 " } else { "" }
                                        )>
                                            {bullet}
                                        </span>
                                    </li>
                                }
                            }).collect::<Vec<_>>()}
                        </ul>
                    }
                }
            >
                {
                    let current_bullets = bullets.get();
                    view! {
                        <div class="space-y-2">
                            {current_bullets.into_iter().enumerate().map(|(i, bullet)| {
                                view! {
                                    <div class="flex gap-2">
                                        <input
                                            type="text"
                                            class="flex-1 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white focus:outline-none focus:border-purple-500"
                                            prop:value=bullet
                                            on:input=move |ev| {
                                                let mut updated_bullets = bullets.get();
                                                if i < updated_bullets.len() {
                                                    updated_bullets[i] = event_target_value(&ev);
                                                    on_change.run(updated_bullets);
                                                }
                                            }
                                        />
                                        <button
                                            class="px-2 py-2 text-red-400 hover:text-red-300 hover:bg-red-900/20 rounded transition-colors"
                                            on:click=move |_| {
                                                let mut updated_bullets = bullets.get();
                                                updated_bullets.remove(i);
                                                on_change.run(updated_bullets);
                                            }
                                        >
                                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                            </svg>
                                        </button>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}

                            <button
                                class="w-full py-2 text-zinc-400 hover:text-white hover:bg-zinc-800 rounded-lg border border-dashed border-zinc-700 transition-colors"
                                on:click=move |_| {
                                    let mut updated_bullets = bullets.get();
                                    updated_bullets.push(String::new());
                                    on_change.run(updated_bullets);
                                }
                            >
                                "+ Add Point"
                            </button>
                        </div>
                    }
                }
            </Show>
        </div>
    }
}

/// Key info view component
#[component]
fn KeyInfoView(
    npcs: Vec<EntityReference>,
    locations: Vec<EntityReference>,
    events: Vec<String>,
) -> impl IntoView {
    view! {
        <div class="key-info-view space-y-6">
            // NPCs
            {(!npcs.is_empty()).then(|| view! {
                <div>
                    <h4 class="text-sm font-medium text-zinc-400 mb-2">"Key NPCs"</h4>
                    <div class="flex flex-wrap gap-2">
                        {npcs.into_iter().map(|npc| view! {
                            <span class="px-3 py-1.5 bg-purple-900/30 text-purple-300 rounded-lg text-sm">
                                {npc.name}
                                {npc.role.map(|role| format!(" ({})", role))}
                            </span>
                        }).collect::<Vec<_>>()}
                    </div>
                </div>
            })}

            // Locations
            {(!locations.is_empty()).then(|| view! {
                <div>
                    <h4 class="text-sm font-medium text-zinc-400 mb-2">"Key Locations"</h4>
                    <div class="flex flex-wrap gap-2">
                        {locations.into_iter().map(|loc| view! {
                            <span class="px-3 py-1.5 bg-blue-900/30 text-blue-300 rounded-lg text-sm">
                                {loc.name}
                            </span>
                        }).collect::<Vec<_>>()}
                    </div>
                </div>
            })}

            // Events
            {(!events.is_empty()).then(|| view! {
                <div>
                    <h4 class="text-sm font-medium text-zinc-400 mb-2">"Key Events"</h4>
                    <ul class="space-y-1">
                        {events.into_iter().map(|event| view! {
                            <li class="flex items-start gap-2 text-zinc-200 text-sm">
                                <span class="text-zinc-500">"•"</span>
                                {event}
                            </li>
                        }).collect::<Vec<_>>()}
                    </ul>
                </div>
            })}
        </div>
    }
}

/// Cliffhanger section component
#[component]
fn CliffhangerSection(
    cliffhanger: Signal<String>,
    is_editing: RwSignal<bool>,
    on_change: Callback<String>,
) -> impl IntoView {
    view! {
        <div class="border-t border-zinc-800 bg-gradient-to-r from-orange-900/20 to-red-900/20">
            <div class="px-4 py-3">
                <h4 class="text-sm font-medium text-orange-400 mb-2 flex items-center gap-2">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                    </svg>
                    "Cliffhanger"
                </h4>

                <Show
                    when=move || is_editing.get()
                    fallback=move || view! {
                        <p class="text-zinc-200 italic">
                            {cliffhanger}
                        </p>
                    }
                >
                    <textarea
                        class="w-full h-20 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white placeholder-zinc-500 focus:outline-none focus:border-orange-500 resize-none"
                        prop:value=cliffhanger
                        on:input=move |ev| on_change.run(event_target_value(&ev))
                    />
                </Show>
            </div>
        </div>
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get event target value helper
fn event_target_value(ev: &leptos::ev::Event) -> String {
    use wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|input| input.value())
        .or_else(|| {
            ev.target()
                .and_then(|t| t.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
                .map(|textarea| textarea.value())
        })
        .or_else(|| {
            ev.target()
                .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
                .map(|select| select.value())
        })
        .unwrap_or_default()
}

/// Copy text to clipboard
#[allow(dead_code, unused_variables)]
fn copy_to_clipboard(text: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen_futures::spawn_local;
        let text = text.to_string();
        spawn_local(async move {
            if let Some(window) = web_sys::window() {
                let clipboard = window.navigator().clipboard();
                let _ = wasm_bindgen_futures::JsFuture::from(
                    clipboard.write_text(&text)
                ).await;
            }
        });
    }
}
