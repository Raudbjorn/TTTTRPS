use leptos::prelude::*;
use crate::bindings::{list_npc_summaries, NpcSummary};

/// NPC List component showing NPCs as a "Direct Messages" style list
#[component]
pub fn NPCList(
    /// Campaign ID to load NPCs for
    campaign_id: String,
    /// Currently selected NPC ID
    #[prop(optional)]
    selected_npc_id: Option<String>,
    /// Callback when an NPC is selected
    #[prop(optional)]
    on_select_npc: Option<Callback<String>>,
) -> impl IntoView {
    let campaign_id_clone = campaign_id.clone();

    // Fetch NPCs from backend using LocalResource
    let npcs_resource = LocalResource::new(move || {
        let cid = campaign_id_clone.clone();
        async move { list_npc_summaries(cid).await.unwrap_or_default() }
    });

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

                // Search (Visual only for now)
                <div class="relative">
                    <input
                        class="w-full bg-zinc-900 border border-zinc-800 rounded px-3 py-1.5 text-xs text-white placeholder-zinc-600 focus:outline-none focus:border-zinc-700"
                        placeholder="Find a character..."
                    />
                </div>
            </div>

            // List
            <div class="flex-1 overflow-y-auto p-2 space-y-1">
                <Suspense fallback=move || view! {
                    <div class="p-4 text-center text-zinc-600 text-xs">"Loading..."</div>
                }>
                    {move || {
                        npcs_resource.get().map(|list| {
                            let list_vec: Vec<_> = (*list).clone();
                            if list_vec.is_empty() {
                                view! {
                                    <div class="p-4 text-center text-zinc-600 text-xs italic">
                                        "No NPCs found."
                                    </div>
                                }.into_any()
                            } else {
                                let selected = selected_npc_id.clone();
                                let on_click = on_select_npc.clone();
                                view! {
                                    <For
                                        each=move || list_vec.clone()
                                        key=|npc| npc.id.clone()
                                        children=move |npc| {
                                            let is_selected = selected.as_ref() == Some(&npc.id);
                                            let callback = on_click.clone();
                                            view! {
                                                <NpcListItem
                                                    npc=npc
                                                    is_selected=is_selected
                                                    on_click=callback
                                                />
                                            }
                                        }
                                    />
                                }.into_any()
                            }
                        })
                    }}
                </Suspense>
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
    /// NPC summary data
    npc: NpcSummary,
    /// Whether this NPC is currently selected
    is_selected: bool,
    /// Click handler
    on_click: Option<Callback<String>>,
) -> impl IntoView {
    let base_class = if is_selected {
        "flex items-center gap-3 p-2 rounded bg-blue-900/20 w-full text-left relative overflow-hidden"
    } else {
        "flex items-center gap-3 p-2 rounded hover:bg-zinc-900 transition-colors w-full text-left relative group"
    };

    let status_color = match npc.status.as_str() {
        "online" => "bg-green-500",
        "away" => "bg-yellow-500",
        _ => "bg-zinc-500",
    };

    let id_click = npc.id.clone();
    let avatar_initial = npc.avatar_url.clone();
    let name = npc.name.clone();
    let last_active = npc.last_active.clone();
    let last_message = npc.last_message.clone();
    let unread_count = npc.unread_count;

    let name_class = if unread_count > 0 {
        "text-sm font-bold text-white truncate"
    } else {
        "text-sm font-medium text-zinc-300 truncate"
    };

    let message_class = if unread_count > 0 {
        "text-xs text-zinc-300 truncate font-medium"
    } else {
        "text-xs text-zinc-500 truncate"
    };

    view! {
        <button
            class=base_class
            on:click=move |_| {
                if let Some(ref callback) = on_click {
                    callback.run(id_click.clone());
                }
            }
        >
            // Selection indicator
            {if is_selected {
                Some(view! {
                    <div class="absolute left-0 top-1/2 -translate-y-1/2 w-0.5 h-6 bg-blue-500 rounded-r"></div>
                })
            } else {
                None
            }}

            // Avatar with Status Bubble
            <div class="relative">
                <div class="w-9 h-9 rounded-md bg-zinc-800 flex items-center justify-center text-sm font-bold text-zinc-400 border border-zinc-700">
                    {avatar_initial}
                </div>
                <div class=format!("absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full border-2 border-zinc-950 {}", status_color)></div>
            </div>

            // Info
            <div class="flex-1 min-w-0">
                <div class="flex justify-between items-baseline">
                    <div class=name_class>{name}</div>
                    {if !last_active.is_empty() {
                        Some(view! {
                            <span class="text-[10px] text-zinc-500 font-mono ml-2">
                                {format_time_short(&last_active)}
                            </span>
                        })
                    } else {
                        None
                    }}
                </div>
                <div class="flex justify-between items-center">
                    <p class=message_class>{last_message}</p>
                    {if unread_count > 0 {
                        Some(view! {
                            <div class="px-1.5 py-0.5 min-w-[1.25rem] bg-indigo-600 rounded-full text-[10px] font-bold text-white text-center ml-2">
                                {unread_count}
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

/// Format ISO timestamp to a short time format (HH:MM)
fn format_time_short(iso: &str) -> String {
    if let Some(time_part) = iso.split('T').nth(1) {
        if let Some(hm) = time_part.get(0..5) {
            return hm.to_string();
        }
    }
    String::new()
}
