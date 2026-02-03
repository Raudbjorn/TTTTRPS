//! NPC list sidebar component (InfoPanel / Slack-style contact list)
//!
//! Displays the list of NPCs for direct messaging in a campaign session
//! using a Slack-style contact list metaphor.
//!
//! Design metaphor: Slack
//! - NPCs as contacts with presence indicators
//! - Unread message badges
//! - Role/relationship badges
//! - Grouped by status (online, away, offline)

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::bindings::{list_npc_summaries, NpcSummary};
use crate::components::design_system::Input;

/// NPC presence status
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NpcPresence {
    /// NPC is present in the current scene
    InScene,
    /// NPC is available but not in scene
    Available,
    /// NPC is away or busy
    Away,
    /// NPC is offline/unreachable
    Offline,
}

impl NpcPresence {
    pub fn from_str(status: &str) -> Self {
        match status.to_lowercase().as_str() {
            "online" | "in_scene" | "present" => NpcPresence::InScene,
            "available" | "active" => NpcPresence::Available,
            "away" | "busy" | "dnd" => NpcPresence::Away,
            _ => NpcPresence::Offline,
        }
    }

    pub fn color_class(&self) -> &'static str {
        match self {
            NpcPresence::InScene => "bg-green-500",
            NpcPresence::Available => "bg-blue-500",
            NpcPresence::Away => "bg-yellow-500",
            NpcPresence::Offline => "bg-zinc-500",
        }
    }

    /// Returns a human-readable label for the presence status
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            NpcPresence::InScene => "In Scene",
            NpcPresence::Available => "Available",
            NpcPresence::Away => "Away",
            NpcPresence::Offline => "Offline",
        }
    }

    pub fn sort_order(&self) -> u8 {
        match self {
            NpcPresence::InScene => 0,
            NpcPresence::Available => 1,
            NpcPresence::Away => 2,
            NpcPresence::Offline => 3,
        }
    }
}

/// Get role styling for NPC role badges
fn get_role_style(role: &str) -> (&'static str, &'static str) {
    let r = role.to_lowercase();
    if r.contains("ally") || r.contains("friend") || r.contains("companion") {
        ("bg-green-900/50", "text-green-400")
    } else if r.contains("enemy") || r.contains("villain") || r.contains("hostile") {
        ("bg-red-900/50", "text-red-400")
    } else if r.contains("merchant") || r.contains("shopkeeper") || r.contains("vendor") {
        ("bg-amber-900/50", "text-amber-400")
    } else if r.contains("quest") || r.contains("important") {
        ("bg-purple-900/50", "text-purple-400")
    } else if r.contains("neutral") {
        ("bg-zinc-800", "text-zinc-400")
    } else {
        ("bg-zinc-800", "text-zinc-400")
    }
}

/// Format ISO timestamp to short time (HH:MM) or relative time
fn format_time_short(iso: &str) -> String {
    if iso.is_empty() {
        return String::new();
    }

    if let Some(time_part) = iso.split('T').nth(1) {
        if let Some(hm) = time_part.get(0..5) {
            return hm.to_string();
        }
    }
    String::new()
}

/// Get initials from a name
fn get_initials(name: &str) -> String {
    name.split_whitespace()
        .take(2)
        .filter_map(|word| word.chars().next())
        .collect::<String>()
        .to_uppercase()
}

/// NPC selection data passed to callback
#[derive(Clone, Debug)]
pub struct NpcSelection {
    pub id: String,
    pub name: String,
}

/// Info Panel component (Slack-style NPC contact list)
#[component]
pub fn NpcList(
    /// Campaign ID to fetch NPCs for
    campaign_id: Signal<String>,
    /// Currently selected NPC ID (if any)
    selected_npc_id: Signal<Option<String>>,
    /// Callback when an NPC is selected (receives both ID and name)
    on_select_npc: Callback<NpcSelection>,
) -> impl IntoView {
    // NPC list state
    let npcs = RwSignal::new(Vec::<NpcSummary>::new());
    let is_loading = RwSignal::new(true);
    let search_query = RwSignal::new(String::new());

    // Collapsed section state
    let in_scene_collapsed = RwSignal::new(false);
    let available_collapsed = RwSignal::new(false);
    let away_collapsed = RwSignal::new(true);
    let offline_collapsed = RwSignal::new(true);

    // Load NPCs when campaign_id changes
    Effect::new(move |_| {
        let cid = campaign_id.get();
        if cid.is_empty() {
            is_loading.set(false);
            return;
        }

        is_loading.set(true);
        spawn_local(async move {
            match list_npc_summaries(cid).await {
                Ok(list) => npcs.set(list),
                Err(_) => npcs.set(vec![]),
            }
            is_loading.set(false);
        });
    });

    // Derive grouped and filtered NPCs
    let grouped_npcs = Memo::new(move |_| {
        let query = search_query.get().to_lowercase();
        let all_npcs = npcs.get();

        let mut filtered: Vec<NpcSummary> = if query.is_empty() {
            all_npcs
        } else {
            all_npcs
                .into_iter()
                .filter(|npc| {
                    npc.name.to_lowercase().contains(&query)
                        || npc.role.to_lowercase().contains(&query)
                })
                .collect()
        };

        // Sort by presence, then by unread count, then by name
        filtered.sort_by(|a, b| {
            let presence_a = NpcPresence::from_str(&a.status);
            let presence_b = NpcPresence::from_str(&b.status);

            match presence_a.sort_order().cmp(&presence_b.sort_order()) {
                std::cmp::Ordering::Equal => match b.unread_count.cmp(&a.unread_count) {
                    std::cmp::Ordering::Equal => a.name.cmp(&b.name),
                    other => other,
                },
                other => other,
            }
        });

        // Group by presence
        let mut in_scene = Vec::new();
        let mut available = Vec::new();
        let mut away = Vec::new();
        let mut offline = Vec::new();

        for npc in filtered {
            match NpcPresence::from_str(&npc.status) {
                NpcPresence::InScene => in_scene.push(npc),
                NpcPresence::Available => available.push(npc),
                NpcPresence::Away => away.push(npc),
                NpcPresence::Offline => offline.push(npc),
            }
        }

        (in_scene, available, away, offline)
    });

    // Total unread count
    let total_unread = Memo::new(move |_| npcs.get().iter().map(|n| n.unread_count).sum::<u32>());

    view! {
        <div class="flex flex-col h-full bg-zinc-950 border-l border-zinc-900 w-72 flex-shrink-0">
            // Header
            <div class="p-4 border-b border-zinc-900">
                <div class="flex justify-between items-center mb-3">
                    <div class="flex items-center gap-2">
                        <ContactsIcon />
                        <h2 class="text-zinc-300 text-sm font-bold">"Characters"</h2>
                        {move || {
                            let unread = total_unread.get();
                            if unread > 0 {
                                Some(view! {
                                    <span class="px-1.5 py-0.5 min-w-[1.25rem] bg-indigo-600 rounded-full text-[10px] font-bold text-white text-center">
                                        {unread}
                                    </span>
                                })
                            } else {
                                None
                            }
                        }}
                    </div>
                    <button
                        class="w-7 h-7 rounded-md hover:bg-zinc-800 flex items-center justify-center text-zinc-500 hover:text-white transition-colors"
                        aria-label="Add NPC"
                    >
                        <AddPersonIcon />
                    </button>
                </div>

                // Search
                <div class="relative">
                    <div class="absolute left-3 top-1/2 -translate-y-1/2 text-zinc-600">
                        <SearchIcon />
                    </div>
                    <Input
                        value=search_query
                        placeholder="Find or start a conversation"
                        class="w-full bg-zinc-900 border border-zinc-800 rounded-md pl-9 pr-3 py-2 text-sm placeholder:text-zinc-600"
                    />
                </div>
            </div>

            // Contact List
            <div class="flex-1 overflow-y-auto">
                {move || {
                    if is_loading.get() {
                        view! {
                            <div class="p-8 flex flex-col items-center justify-center text-zinc-600">
                                <LoadingSpinner />
                                <span class="text-xs mt-2">"Loading contacts..."</span>
                            </div>
                        }.into_any()
                    } else {
                        let (in_scene, available, away, offline) = grouped_npcs.get();
                        let has_any = !in_scene.is_empty() || !available.is_empty()
                            || !away.is_empty() || !offline.is_empty();

                        if !has_any {
                            view! {
                                <EmptyState search_active=Signal::derive(move || !search_query.get().is_empty()) />
                            }.into_any()
                        } else {
                            view! {
                                <div class="py-2">
                                    // In Scene Section
                                    <NpcSection
                                        title="In Scene"
                                        presence=NpcPresence::InScene
                                        npcs=in_scene
                                        collapsed=in_scene_collapsed
                                        selected_npc_id=selected_npc_id
                                        on_select_npc=on_select_npc
                                    />

                                    // Available Section
                                    <NpcSection
                                        title="Available"
                                        presence=NpcPresence::Available
                                        npcs=available
                                        collapsed=available_collapsed
                                        selected_npc_id=selected_npc_id
                                        on_select_npc=on_select_npc
                                    />

                                    // Away Section
                                    <NpcSection
                                        title="Away"
                                        presence=NpcPresence::Away
                                        npcs=away
                                        collapsed=away_collapsed
                                        selected_npc_id=selected_npc_id
                                        on_select_npc=on_select_npc
                                    />

                                    // Offline Section
                                    <NpcSection
                                        title="Offline"
                                        presence=NpcPresence::Offline
                                        npcs=offline
                                        collapsed=offline_collapsed
                                        selected_npc_id=selected_npc_id
                                        on_select_npc=on_select_npc
                                    />
                                </div>
                            }.into_any()
                        }
                    }
                }}
            </div>

            // Footer
            <div class="p-3 border-t border-zinc-900 bg-zinc-950/50">
                <div class="flex items-center justify-between text-[10px] text-zinc-600">
                    <div class="flex items-center gap-1.5">
                        <div class="w-2 h-2 rounded-full bg-green-500"></div>
                        <span>{move || {
                            let (in_scene, available, _, _) = grouped_npcs.get();
                            format!("{} active", in_scene.len() + available.len())
                        }}</span>
                    </div>
                    <span>"Neural Link Connected"</span>
                </div>
            </div>
        </div>
    }
}

/// Collapsible NPC section
#[allow(clippy::needless_pass_by_value)]
#[component]
fn NpcSection(
    title: &'static str,
    presence: NpcPresence,
    npcs: Vec<NpcSummary>,
    collapsed: RwSignal<bool>,
    selected_npc_id: Signal<Option<String>>,
    on_select_npc: Callback<NpcSelection>,
) -> impl IntoView {
    let count = npcs.len();

    if count == 0 {
        return view! { <div></div> }.into_any();
    }

    let unread_count: u32 = npcs.iter().map(|n| n.unread_count).sum();
    let npcs_store = StoredValue::new(npcs);

    view! {
        <div class="mb-1">
            // Section Header
            <button
                class="w-full flex items-center gap-2 px-4 py-1.5 text-left hover:bg-zinc-900/50 transition-colors"
                on:click=move |_| collapsed.update(|c| *c = !*c)
            >
                <span class=move || {
                    if collapsed.get() {
                        "text-zinc-600 transition-transform"
                    } else {
                        "text-zinc-600 transition-transform rotate-90"
                    }
                }>
                    <ChevronIcon />
                </span>
                <span class=format!("w-2 h-2 rounded-full {}", presence.color_class())></span>
                <span class="text-[11px] font-semibold text-zinc-500 uppercase tracking-wider flex-1">
                    {title}
                </span>
                <span class="text-[10px] text-zinc-600">{count}</span>
                {if unread_count > 0 {
                    Some(view! {
                        <span class="px-1.5 py-0.5 min-w-[1.25rem] bg-indigo-600 rounded-full text-[9px] font-bold text-white text-center">
                            {unread_count}
                        </span>
                    })
                } else {
                    None
                }}
            </button>

            // NPC List
            <Show when=move || !collapsed.get()>
                <div class="space-y-0.5 px-2">
                    {npcs_store.get_value().into_iter().map(|npc| {
                        let npc_id = npc.id.clone();
                        view! {
                            <NpcContactItem
                                npc=npc
                                is_selected=Signal::derive(move || selected_npc_id.get() == Some(npc_id.clone()))
                                on_click=on_select_npc
                            />
                        }
                    }).collect_view()}
                </div>
            </Show>
        </div>
    }.into_any()
}

/// Individual NPC contact item (Slack-style)
#[component]
fn NpcContactItem(
    npc: NpcSummary,
    is_selected: Signal<bool>,
    on_click: Callback<NpcSelection>,
) -> impl IntoView {
    let npc_selection = StoredValue::new(NpcSelection {
        id: npc.id.clone(),
        name: npc.name.clone(),
    });
    let npc_name = npc.name.clone();
    let npc_role = npc.role.clone();
    let avatar = npc.avatar_url.clone();
    let status = npc.status.clone();
    let last_message = npc.last_message.clone();
    let unread_count = npc.unread_count;
    let has_unread = unread_count > 0;
    let formatted_time = format_time_short(&npc.last_active);

    let presence = NpcPresence::from_str(&status);
    let (role_bg, role_text) = get_role_style(&npc_role);

    // Get initials for avatar fallback
    let initials = get_initials(&npc_name);
    let has_avatar = !avatar.is_empty() && avatar.len() > 2;

    view! {
        <button
            class=move || {
                let mut classes = vec![
                    "flex items-center gap-3 p-2 rounded-lg w-full text-left relative transition-all group",
                ];

                if is_selected.get() {
                    classes.push("bg-indigo-900/30 border border-indigo-500/30");
                } else {
                    classes.push("hover:bg-zinc-900/80 border border-transparent");
                }

                classes.join(" ")
            }
            on:click=move |_| on_click.run(npc_selection.get_value())
        >
            // Selection indicator
            {move || {
                if is_selected.get() {
                    Some(view! {
                        <div class="absolute left-0 top-1/2 -translate-y-1/2 w-0.5 h-6 bg-indigo-500 rounded-r"></div>
                    })
                } else {
                    None
                }
            }}

            // Avatar with Presence Indicator
            <div class="relative flex-shrink-0">
                <div class="w-9 h-9 rounded-lg bg-zinc-800 flex items-center justify-center text-sm font-bold text-zinc-400 border border-zinc-700 overflow-hidden">
                    {if has_avatar {
                        view! {
                            <img src=avatar.clone() alt="" class="w-full h-full object-cover" />
                        }.into_any()
                    } else {
                        view! {
                            <span class="text-xs">{initials}</span>
                        }.into_any()
                    }}
                </div>
                // Presence dot
                <div class=format!(
                    "absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full border-2 border-zinc-950 {}",
                    presence.color_class()
                )></div>
            </div>

            // Info
            <div class="flex-1 min-w-0">
                // Name row
                <div class="flex items-center gap-2">
                    <span class=move || {
                        if has_unread {
                            "text-sm font-bold text-white truncate"
                        } else if is_selected.get() {
                            "text-sm font-medium text-indigo-300 truncate"
                        } else {
                            "text-sm font-medium text-zinc-300 group-hover:text-white truncate transition-colors"
                        }
                    }>
                        {npc_name.clone()}
                    </span>

                    // Role badge
                    {if !npc_role.is_empty() && npc_role != "npc" {
                        Some(view! {
                            <span class=format!(
                                "px-1.5 py-0.5 rounded text-[9px] font-medium {} {} hidden group-hover:inline-flex",
                                role_bg, role_text
                            )>
                                {npc_role.clone()}
                            </span>
                        })
                    } else {
                        None
                    }}
                </div>

                // Last message preview
                <div class="flex items-center gap-2 mt-0.5">
                    <p class=move || {
                        if has_unread {
                            "text-xs text-zinc-300 truncate font-medium flex-1"
                        } else {
                            "text-xs text-zinc-500 truncate flex-1"
                        }
                    }>
                        {if !last_message.is_empty() {
                            last_message.clone()
                        } else {
                            "No messages yet".to_string()
                        }}
                    </p>
                </div>
            </div>

            // Right side: Time and unread badge
            <div class="flex flex-col items-end gap-1 flex-shrink-0">
                {if !formatted_time.is_empty() {
                    Some(view! {
                        <span class="text-[10px] text-zinc-600 font-mono">
                            {formatted_time.clone()}
                        </span>
                    })
                } else {
                    None
                }}

                {if has_unread {
                    Some(view! {
                        <div class="px-1.5 py-0.5 min-w-[1.25rem] bg-indigo-600 rounded-full text-[10px] font-bold text-white text-center">
                            {unread_count.to_string()}
                        </div>
                    })
                } else {
                    None
                }}
            </div>

            // Hover action
            <div class="opacity-0 group-hover:opacity-100 transition-opacity absolute right-2 top-1/2 -translate-y-1/2">
                <MessageIcon />
            </div>
        </button>
    }
}

/// Empty state component
#[component]
fn EmptyState(search_active: Signal<bool>) -> impl IntoView {
    view! {
        <div class="p-8 flex flex-col items-center justify-center text-center">
            <div class="w-12 h-12 rounded-full bg-zinc-900 flex items-center justify-center mb-4 text-zinc-600">
                <ContactsIcon />
            </div>
            {move || {
                if search_active.get() {
                    view! {
                        <>
                            <p class="text-sm text-zinc-400 mb-1">"No matches found"</p>
                            <p class="text-xs text-zinc-600">"Try a different search term"</p>
                        </>
                    }.into_any()
                } else {
                    view! {
                        <>
                            <p class="text-sm text-zinc-400 mb-1">"No characters yet"</p>
                            <p class="text-xs text-zinc-600">"Add NPCs to start conversations"</p>
                        </>
                    }.into_any()
                }
            }}
        </div>
    }
}

// Icon Components

#[component]
fn ContactsIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-indigo-400">
            <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path>
            <circle cx="9" cy="7" r="4"></circle>
            <path d="M23 21v-2a4 4 0 0 0-3-3.87"></path>
            <path d="M16 3.13a4 4 0 0 1 0 7.75"></path>
        </svg>
    }
}

#[component]
fn AddPersonIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M16 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path>
            <circle cx="8.5" cy="7" r="4"></circle>
            <line x1="20" y1="8" x2="20" y2="14"></line>
            <line x1="23" y1="11" x2="17" y2="11"></line>
        </svg>
    }
}

#[component]
fn SearchIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"></circle>
            <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
        </svg>
    }
}

#[component]
fn ChevronIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="9,18 15,12 9,6"></polyline>
        </svg>
    }
}

#[component]
fn MessageIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-zinc-600 hover:text-indigo-400">
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
        </svg>
    }
}

#[component]
fn LoadingSpinner() -> impl IntoView {
    view! {
        <svg class="animate-spin h-5 w-5 text-zinc-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
    }
}

/// Also export as InfoPanel for the new naming convention
#[allow(unused_imports)]
pub use NpcList as InfoPanel;
