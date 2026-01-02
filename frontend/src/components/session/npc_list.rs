//! NPC list sidebar component
//!
//! Displays the list of NPCs for direct messaging in a campaign session

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::bindings::{list_npc_summaries, NpcSummary};
use crate::components::design_system::Input;

/// NPC list sidebar component
#[component]
pub fn NpcList(
    /// Campaign ID to fetch NPCs for
    campaign_id: Signal<String>,
    /// Currently selected NPC ID (if any)
    selected_npc_id: Signal<Option<String>>,
    /// Callback when an NPC is selected
    on_select_npc: Callback<String>,
) -> impl IntoView {
    // NPC list state
    let npcs = RwSignal::new(Vec::<NpcSummary>::new());
    let is_loading = RwSignal::new(true);
    let search_query = RwSignal::new(String::new());

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

    // Derive filtered NPCs (recomputes when npcs or search_query change)
    let get_filtered_npcs = move || {
        let query = search_query.get().to_lowercase();
        let all_npcs = npcs.get();

        if query.is_empty() {
            all_npcs
        } else {
            all_npcs
                .into_iter()
                .filter(|npc| npc.name.to_lowercase().contains(&query))
                .collect()
        }
    };

    view! {
        <div class="flex flex-col h-full bg-zinc-950 border-l border-zinc-900 w-72 flex-shrink-0">
            // Header
            <div class="p-4 border-b border-zinc-900">
                <div class="flex justify-between items-center mb-2">
                    <h2 class="text-zinc-400 text-xs font-bold uppercase tracking-wider">"Direct Messages"</h2>
                    <button
                        class="w-6 h-6 rounded hover:bg-zinc-800 flex items-center justify-center text-zinc-500 hover:text-white transition-colors"
                        aria-label="New Message"
                    >
                        "+"
                    </button>
                </div>

                // Search
                <div class="relative">
                    <Input
                        value=search_query
                        placeholder="Find a character..."
                        class="w-full bg-zinc-900 border border-zinc-800 rounded px-3 py-1.5 text-xs"
                    />
                </div>
            </div>

            // List
            <div class="flex-1 overflow-y-auto p-2 space-y-1">
                {move || {
                    if is_loading.get() {
                        view! {
                            <div class="p-4 text-center text-zinc-600 text-xs">
                                "Loading..."
                            </div>
                        }.into_any()
                    } else {
                        let list = get_filtered_npcs();
                        if list.is_empty() {
                            view! {
                                <div class="p-4 text-center text-zinc-600 text-xs italic">
                                    "No NPCs found."
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <For
                                    each=move || get_filtered_npcs()
                                    key=|npc| npc.id.clone()
                                    children=move |npc| {
                                        let npc_id = npc.id.clone();
                                        view! {
                                            <NpcListItem
                                                npc=npc
                                                is_selected=Signal::derive(move || selected_npc_id.get() == Some(npc_id.clone()))
                                                on_click=on_select_npc
                                            />
                                        }
                                    }
                                />
                            }.into_any()
                        }
                    }
                }}
            </div>

            // Footer
            <div class="p-2 border-t border-zinc-900 text-[10px] text-center text-zinc-700">
                "Synced with Neural Link"
            </div>
        </div>
    }
}

/// Individual NPC list item
#[component]
fn NpcListItem(
    npc: NpcSummary,
    is_selected: Signal<bool>,
    on_click: Callback<String>,
) -> impl IntoView {
    let npc_id = StoredValue::new(npc.id.clone());
    let npc_name = npc.name.clone();
    let avatar = npc.avatar_url.clone();
    let status = npc.status.clone();
    let last_message = npc.last_message.clone();
    let unread_count = npc.unread_count;
    let has_last_active = !npc.last_active.is_empty();
    let has_unread = unread_count > 0;
    let formatted_time = format_time_short(&npc.last_active);

    let status_color = match status.as_str() {
        "online" => "bg-green-500",
        "away" => "bg-yellow-500",
        _ => "bg-zinc-500",
    };

    view! {
        <button
            class=move || {
                if is_selected.get() {
                    "flex items-center gap-3 p-2 rounded bg-blue-900/20 w-full text-left relative overflow-hidden"
                } else {
                    "flex items-center gap-3 p-2 rounded hover:bg-zinc-900 transition-colors w-full text-left relative group"
                }
            }
            on:click=move |_| on_click.run(npc_id.get_value())
        >
            // Selection indicator
            {move || {
                if is_selected.get() {
                    Some(view! {
                        <div class="absolute left-0 top-1/2 -translate-y-1/2 w-0.5 h-6 bg-blue-500 rounded-r"></div>
                    })
                } else {
                    None
                }
            }}

            // Avatar with Status Bubble
            <div class="relative">
                <div class="w-9 h-9 rounded-md bg-zinc-800 flex items-center justify-center text-sm font-bold text-zinc-400 border border-zinc-700">
                    {avatar.clone()}
                </div>
                <div class=format!(
                    "absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full border-2 border-zinc-950 {}",
                    status_color
                )></div>
            </div>

            // Info
            <div class="flex-1 min-w-0">
                <div class="flex justify-between items-baseline">
                    <div class=move || {
                        if has_unread {
                            "text-sm font-bold text-white truncate"
                        } else {
                            "text-sm font-medium text-zinc-300 truncate"
                        }
                    }>
                        {npc_name.clone()}
                    </div>
                    {if has_last_active {
                        Some(view! {
                            <span class="text-[10px] text-zinc-500 font-mono ml-2">
                                {formatted_time.clone()}
                            </span>
                        })
                    } else {
                        None
                    }}
                </div>
                <div class="flex justify-between items-center">
                    <p class=move || {
                        if has_unread {
                            "text-xs text-zinc-300 truncate font-medium"
                        } else {
                            "text-xs text-zinc-500 truncate"
                        }
                    }>
                        {last_message.clone()}
                    </p>
                    {if has_unread {
                        Some(view! {
                            <div class="px-1.5 py-0.5 min-w-[1.25rem] bg-indigo-600 rounded-full text-[10px] font-bold text-white text-center ml-2">
                                {unread_count.to_string()}
                            </div>
                        })
                    } else {
                        None
                    }}
                </div>
            </div>
        </button>
    }
}

/// Format ISO timestamp to short time (HH:MM)
fn format_time_short(iso: &str) -> String {
    if let Some(time_part) = iso.split('T').nth(1) {
        if let Some(hm) = time_part.get(0..5) {
            return hm.to_string();
        }
    }
    String::new()
}
