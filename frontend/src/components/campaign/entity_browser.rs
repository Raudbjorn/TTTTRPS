//! Entity Browser Component
//!
//! Browse and manage campaign entities (NPCs, locations, factions, etc.)

use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::bindings::{
    list_npcs, list_locations, NPC, LocationState,
};

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

    view! {
        <div class="space-y-4">
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
                                                    view! { <NpcEntityCard npc=npc /> }
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
        </div>
    }
}
