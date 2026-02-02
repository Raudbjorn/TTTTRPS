//! Entity Browser Component
//!
//! Browse and manage campaign entities (NPCs, locations, factions, etc.)

use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::bindings::{
    list_npcs, list_locations, get_npc_conversation,
    NPC, LocationState, NpcConversation, ConversationMessage,
};
use crate::components::campaign_details::{NpcConversation as NpcConversationPanel, NpcChatSelection};

/// Entity type filter
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum EntityFilter {
    All,
    NPCs,
    Locations,
    Factions,
    Items,
}

impl EntityFilter {
    fn label(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::NPCs => "NPCs",
            Self::Locations => "Locations",
            Self::Factions => "Factions",
            Self::Items => "Items",
        }
    }
}

/// Filter chip component
#[component]
fn FilterChip(
    filter: EntityFilter,
    active_filter: EntityFilter,
    on_click: Callback<EntityFilter>,
) -> impl IntoView {
    let is_active = filter == active_filter;
    let class = if is_active {
        "px-3 py-1 text-sm bg-purple-600 text-white rounded-full"
    } else {
        "px-3 py-1 text-sm bg-zinc-800 text-zinc-400 hover:text-white rounded-full transition-colors"
    };

    view! {
        <button class=class on:click=move |_| on_click.run(filter)>
            {filter.label()}
        </button>
    }
}

/// Entity card for NPCs
#[component]
fn NpcEntityCard(
    npc: NPC,
    #[prop(optional)]
    on_select: Option<Callback<String>>,
) -> impl IntoView {
    let npc_id = npc.id.clone();
    let initials = npc.name.chars().next().unwrap_or('?');

    let handle_click = move |_: ev::MouseEvent| {
        if let Some(ref callback) = on_select {
            callback.run(npc_id.clone());
        }
    };

    view! {
        <div
            class="bg-zinc-900 border border-zinc-800 rounded-lg p-4 hover:border-zinc-600 transition-colors cursor-pointer"
            on:click=handle_click
        >
            <div class="flex items-start gap-3">
                // Avatar
                <div class="w-12 h-12 rounded-lg bg-purple-900/50 flex items-center justify-center text-purple-300 font-bold">
                    {initials.to_string()}
                </div>

                // Info
                <div class="flex-1 min-w-0">
                    <div class="font-medium text-white truncate">{npc.name.clone()}</div>
                    <div class="text-sm text-zinc-500">{npc.role.clone()}</div>
                    <div class="flex flex-wrap gap-1 mt-2">
                        {npc.tags.iter().take(3).map(|tag| {
                            view! {
                                <span class="px-1.5 py-0.5 text-xs bg-zinc-800 text-zinc-400 rounded">
                                    {tag.clone()}
                                </span>
                            }
                        }).collect_view()}
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Entity card for locations
#[component]
fn LocationEntityCard(
    location: LocationState,
    #[prop(optional)]
    on_select: Option<Callback<String>>,
) -> impl IntoView {
    let location_id = location.location_id.clone();

    let handle_click = move |_: ev::MouseEvent| {
        if let Some(ref callback) = on_select {
            callback.run(location_id.clone());
        }
    };

    let condition_color = match location.condition.as_str() {
        "pristine" | "blessed" => "text-emerald-400",
        "normal" => "text-zinc-400",
        "damaged" | "occupied" => "text-yellow-400",
        "ruined" | "destroyed" | "cursed" => "text-red-400",
        _ => "text-zinc-400",
    };

    view! {
        <div
            class="bg-zinc-900 border border-zinc-800 rounded-lg p-4 hover:border-zinc-600 transition-colors cursor-pointer"
            on:click=handle_click
        >
            <div class="flex items-start gap-3">
                // Icon
                <div class="w-12 h-12 rounded-lg bg-emerald-900/50 flex items-center justify-center text-emerald-300">
                    "M"
                </div>

                // Info
                <div class="flex-1 min-w-0">
                    <div class="font-medium text-white truncate">{location.name.clone()}</div>
                    <div class=format!("text-sm {}", condition_color)>{location.condition.clone()}</div>
                    {location.population.map(|pop| view! {
                        <div class="text-xs text-zinc-500 mt-1">{format!("Pop: {}", pop)}</div>
                    })}
                </div>
            </div>
        </div>
    }
}

/// Main entity browser component
#[component]
pub fn EntityBrowser(
    /// Campaign ID to browse entities for
    campaign_id: String,
) -> impl IntoView {
    // State
    let active_filter = RwSignal::new(EntityFilter::All);
    let search_query = RwSignal::new(String::new());
    let npcs = RwSignal::new(Vec::<NPC>::new());
    let locations = RwSignal::new(Vec::<LocationState>::new());
    let is_loading = RwSignal::new(true);

    // NPC Detail/Chat state
    let selected_npc = RwSignal::new(Option::<NPC>::None);
    let show_npc_chat = RwSignal::new(false);
    let chat_npc_id = RwSignal::new(Option::<String>::None);
    let chat_npc_name = RwSignal::new(Option::<String>::None);

    // Load data
    let campaign_id_clone = campaign_id.clone();
    Effect::new(move |_| {
        let cid = campaign_id_clone.clone();
        spawn_local(async move {
            is_loading.set(true);

            // Load NPCs
            if let Ok(npc_list) = list_npcs(Some(cid.clone())).await {
                npcs.set(npc_list);
            }

            // Load locations
            if let Ok(loc_list) = list_locations(cid).await {
                locations.set(loc_list);
            }

            is_loading.set(false);
        });
    });

    let handle_filter_change = Callback::new(move |filter: EntityFilter| {
        active_filter.set(filter);
    });

    let handle_search = move |evt: ev::Event| {
        let target = event_target::<web_sys::HtmlInputElement>(&evt);
        search_query.set(target.value());
    };

    // NPC selection handler - opens detail panel
    let handle_npc_select = {
        let npcs = npcs.clone();
        Callback::new(move |npc_id: String| {
            if let Some(npc) = npcs.get().into_iter().find(|n| n.id == npc_id) {
                selected_npc.set(Some(npc));
                show_npc_chat.set(false);
            }
        })
    };

    // Open NPC chat handler
    let handle_open_chat = Callback::new(move |selection: NpcChatSelection| {
        chat_npc_id.set(Some(selection.id));
        chat_npc_name.set(Some(selection.name));
        show_npc_chat.set(true);
    });

    // Close detail panel handler
    let handle_close_detail = Callback::new(move |_: ()| {
        selected_npc.set(None);
        show_npc_chat.set(false);
        chat_npc_id.set(None);
        chat_npc_name.set(None);
    });

    view! {
        <div class="flex gap-4">
            // Main content area
            <div class="flex-1 space-y-4">
            // Header
            <div class="flex flex-col md:flex-row md:items-center gap-4">
                // Search
                <div class="flex-1">
                    <input
                        type="text"
                        class="w-full px-4 py-2 bg-zinc-900 border border-zinc-800 rounded-lg text-white placeholder-zinc-500 focus:border-zinc-600 focus:outline-none"
                        placeholder="Search entities..."
                        prop:value=move || search_query.get()
                        on:input=handle_search
                    />
                </div>

                // Add button
                <button class="px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white rounded-lg transition-colors">
                    "+ Add Entity"
                </button>
            </div>

            // Filters
            <div class="flex gap-2 flex-wrap">
                <FilterChip filter=EntityFilter::All active_filter=active_filter.get() on_click=handle_filter_change />
                <FilterChip filter=EntityFilter::NPCs active_filter=active_filter.get() on_click=handle_filter_change />
                <FilterChip filter=EntityFilter::Locations active_filter=active_filter.get() on_click=handle_filter_change />
                <FilterChip filter=EntityFilter::Factions active_filter=active_filter.get() on_click=handle_filter_change />
                <FilterChip filter=EntityFilter::Items active_filter=active_filter.get() on_click=handle_filter_change />
            </div>

            // Content
            {move || {
                if is_loading.get() {
                    view! {
                        <div class="text-center py-12 text-zinc-500">"Loading entities..."</div>
                    }.into_any()
                } else {
                    let filter = active_filter.get();
                    let query = search_query.get().to_lowercase();

                    let show_npcs = filter == EntityFilter::All || filter == EntityFilter::NPCs;
                    let show_locations = filter == EntityFilter::All || filter == EntityFilter::Locations;

                    let filtered_npcs: Vec<_> = npcs.get().into_iter()
                        .filter(|n| query.is_empty() || n.name.to_lowercase().contains(&query))
                        .collect();

                    let filtered_locations: Vec<_> = locations.get().into_iter()
                        .filter(|l| query.is_empty() || l.name.to_lowercase().contains(&query))
                        .collect();

                    let total = (if show_npcs { filtered_npcs.len() } else { 0 })
                        + (if show_locations { filtered_locations.len() } else { 0 });

                    if total == 0 {
                        view! {
                            <div class="text-center py-12">
                                <div class="text-zinc-500 mb-4">"No entities found"</div>
                                <button class="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-white rounded-lg transition-colors">
                                    "Create First Entity"
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="space-y-6">
                                // NPCs Section
                                {if show_npcs && !filtered_npcs.is_empty() {
                                    Some(view! {
                                        <div>
                                            <h3 class="text-sm font-bold text-zinc-400 uppercase tracking-wider mb-3">
                                                {format!("NPCs ({})", filtered_npcs.len())}
                                            </h3>
                                            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                                {filtered_npcs.into_iter().map(|npc| {
                                                    view! { <NpcEntityCard npc=npc on_select=handle_npc_select /> }
                                                }).collect_view()}
                                            </div>
                                        </div>
                                    })
                                } else {
                                    None
                                }}

                                // Locations Section
                                {if show_locations && !filtered_locations.is_empty() {
                                    Some(view! {
                                        <div>
                                            <h3 class="text-sm font-bold text-zinc-400 uppercase tracking-wider mb-3">
                                                {format!("Locations ({})", filtered_locations.len())}
                                            </h3>
                                            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                                {filtered_locations.into_iter().map(|loc| {
                                                    view! { <LocationEntityCard location=loc /> }
                                                }).collect_view()}
                                            </div>
                                        </div>
                                    })
                                } else {
                                    None
                                }}
                            </div>
                        }.into_any()
                    }
                }
            }}
            </div> // Close main content area

            // NPC Detail/Chat Panel (right side)
            {move || {
                let npc_id = chat_npc_id.get();
                let npc_name = chat_npc_name.get();

                if show_npc_chat.get() && npc_id.is_some() && npc_name.is_some() {
                    // Full NPC Chat view
                    view! {
                        <div class="w-96 flex-shrink-0 border-l border-zinc-800 bg-zinc-950">
                            <NpcConversationPanel
                                npc_id=npc_id.unwrap()
                                npc_name=npc_name.unwrap()
                                on_close=handle_close_detail
                            />
                        </div>
                    }.into_any()
                } else if let Some(npc) = selected_npc.get() {
                    // NPC Detail Panel with conversation preview
                    view! {
                        <NpcDetailPanel
                            npc=npc
                            on_chat=handle_open_chat
                            on_close=handle_close_detail
                        />
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}
        </div>
    }
}

/// NPC Detail Panel showing NPC info and conversation preview
#[component]
fn NpcDetailPanel(
    npc: NPC,
    on_chat: Callback<NpcChatSelection>,
    on_close: Callback<()>,
) -> impl IntoView {
    let npc_id = npc.id.clone();
    let npc_name = npc.name.clone();
    let npc_id_for_chat = npc.id.clone();
    let npc_name_for_chat = npc.name.clone();

    // Fetch conversation for this NPC
    let conversation = RwSignal::new(Option::<NpcConversation>::None);
    let is_loading = RwSignal::new(true);

    Effect::new(move |_| {
        let id = npc_id.clone();
        spawn_local(async move {
            match get_npc_conversation(id).await {
                Ok(conv) => conversation.set(Some(conv)),
                Err(_) => conversation.set(None),
            }
            is_loading.set(false);
        });
    });

    // Parse messages from conversation
    let messages = Memo::new(move |_| {
        conversation.get()
            .map(|c| {
                match serde_json::from_str::<Vec<ConversationMessage>>(&c.messages_json) {
                    Ok(msgs) => msgs,
                    Err(e) => {
                        log::error!("Failed to parse NPC conversation messages: {}", e);
                        Vec::new()
                    }
                }
            })
            .unwrap_or_default()
    });

    let initials = npc_name.chars().next().unwrap_or('?');

    view! {
        <div class="w-80 flex-shrink-0 border-l border-zinc-800 bg-zinc-900 flex flex-col h-full">
            // Header
            <div class="p-4 border-b border-zinc-800 flex items-center justify-between">
                <div class="flex items-center gap-3">
                    <div class="w-10 h-10 rounded-lg bg-purple-900/50 flex items-center justify-center text-purple-300 font-bold">
                        {initials.to_string()}
                    </div>
                    <div>
                        <h3 class="font-bold text-white">{npc.name.clone()}</h3>
                        <p class="text-sm text-zinc-500">{npc.role.clone()}</p>
                    </div>
                </div>
                <button
                    class="p-2 text-zinc-500 hover:text-white hover:bg-zinc-800 rounded transition-colors"
                    aria-label="Close panel"
                    on:click=move |_| on_close.run(())
                >
                    "×"
                </button>
            </div>

            // Tags
            {(!npc.tags.is_empty()).then(|| view! {
                <div class="p-4 border-b border-zinc-800">
                    <div class="flex flex-wrap gap-1">
                        {npc.tags.iter().map(|tag| {
                            view! {
                                <span class="px-2 py-1 text-xs bg-zinc-800 text-zinc-400 rounded">
                                    {tag.clone()}
                                </span>
                            }
                        }).collect_view()}
                    </div>
                </div>
            })}

            // Conversation History
            <div class="flex-1 overflow-y-auto p-4">
                <div class="flex items-center justify-between mb-3">
                    <h4 class="text-sm font-bold text-zinc-400 uppercase tracking-wider">"Conversation History"</h4>
                    <span class="text-xs text-zinc-600">{move || format!("{} messages", messages.get().len())}</span>
                </div>

                {move || {
                    if is_loading.get() {
                        view! {
                            <div class="text-center py-8 text-zinc-500">"Loading..."</div>
                        }.into_any()
                    } else if messages.get().is_empty() {
                        view! {
                            <div class="text-center py-8 text-zinc-500">
                                <p class="mb-2">"No conversation yet"</p>
                                <p class="text-xs">"Start chatting to build a history"</p>
                            </div>
                        }.into_any()
                    } else {
                        let msgs = messages.get();
                        let recent_msgs: Vec<_> = msgs.into_iter().rev().take(10).collect();
                        view! {
                            <div class="space-y-3">
                                {recent_msgs.into_iter().map(|msg| {
                                    let is_user = msg.role == "user";
                                    let bubble_class = if is_user {
                                        "bg-purple-900/30 border-purple-700/30"
                                    } else {
                                        "bg-zinc-800 border-zinc-700"
                                    };
                                    let preview = {
                                        let chars: Vec<char> = msg.content.chars().take(101).collect();
                                        if chars.len() > 100 {
                                            format!("{}...", chars[..100].iter().collect::<String>())
                                        } else {
                                            msg.content.clone()
                                        }
                                    };
                                    view! {
                                        <div class=format!("p-2 rounded border {} text-sm", bubble_class)>
                                            <span class=if is_user { "text-purple-300" } else { "text-zinc-300" }>
                                                {preview}
                                            </span>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }
                }}
            </div>

            // Chat Action Button
            <div class="p-4 border-t border-zinc-800">
                <button
                    class="w-full px-4 py-3 bg-purple-600 hover:bg-purple-500 text-white rounded-lg font-medium transition-colors flex items-center justify-center gap-2"
                    on:click=move |_| {
                        on_chat.run(NpcChatSelection {
                            id: npc_id_for_chat.clone(),
                            name: npc_name_for_chat.clone(),
                        });
                    }
                >
                    <ChatIcon />
                    "Start Conversation"
                </button>
            </div>
        </div>
    }
}

/// Chat icon for the detail panel
#[component]
fn ChatIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
        </svg>
    }
}
