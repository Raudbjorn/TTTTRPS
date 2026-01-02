//! World State Editor Component
//!
//! Provides UI for managing world state including in-game date,
//! world events, location states, and custom fields.

use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::bindings::{
    get_world_state, set_in_game_date, advance_in_game_date,
    delete_world_event, set_world_custom_field,
    WorldState, WorldEvent, LocationState, InGameDate,
    CalendarConfig,
};

/// World state editor tab
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum WorldStateTab {
    Date,
    Events,
    Locations,
    Custom,
}

impl WorldStateTab {
    fn label(&self) -> &'static str {
        match self {
            Self::Date => "Date & Time",
            Self::Events => "Events",
            Self::Locations => "Locations",
            Self::Custom => "Custom Fields",
        }
    }
}

/// Tab button for world state editor
#[component]
fn WorldStateTabButton(
    tab: WorldStateTab,
    active_tab: WorldStateTab,
    on_click: Callback<WorldStateTab>,
) -> impl IntoView {
    let is_active = tab == active_tab;
    let class = if is_active {
        "px-4 py-2 text-sm font-medium bg-zinc-800 text-white rounded-lg"
    } else {
        "px-4 py-2 text-sm font-medium text-zinc-400 hover:text-white hover:bg-zinc-800/50 rounded-lg transition-colors"
    };

    view! {
        <button class=class on:click=move |_| on_click.run(tab)>
            {tab.label()}
        </button>
    }
}

/// Date picker component for in-game dates
#[component]
fn InGameDatePicker(
    date: InGameDate,
    calendar: Option<CalendarConfig>,
    on_change: Callback<InGameDate>,
) -> impl IntoView {
    let year = RwSignal::new(date.year);
    let month = RwSignal::new(date.month);
    let day = RwSignal::new(date.day);
    let era = RwSignal::new(date.era.clone().unwrap_or_default());

    let month_names = calendar.as_ref()
        .map(|c| c.month_names.clone())
        .unwrap_or_else(|| (1..=12).map(|i| format!("Month {}", i)).collect());

    let on_change_clone = on_change.clone();
    let emit_change = move || {
        let new_date = InGameDate {
            year: year.get(),
            month: month.get(),
            day: day.get(),
            era: if era.get().is_empty() { None } else { Some(era.get()) },
            calendar: date.calendar.clone(),
            time: date.time.clone(),
        };
        on_change_clone.run(new_date);
    };

    let handle_year_change = {
        let emit = emit_change.clone();
        move |evt: ev::Event| {
            let target = event_target::<web_sys::HtmlInputElement>(&evt);
            if let Ok(val) = target.value().parse::<i32>() {
                year.set(val);
                emit();
            }
        }
    };

    let handle_month_change = {
        let emit = emit_change.clone();
        move |evt: ev::Event| {
            let target = event_target::<web_sys::HtmlSelectElement>(&evt);
            if let Ok(val) = target.value().parse::<u8>() {
                month.set(val);
                emit();
            }
        }
    };

    let handle_day_change = {
        let emit = emit_change.clone();
        move |evt: ev::Event| {
            let target = event_target::<web_sys::HtmlInputElement>(&evt);
            if let Ok(val) = target.value().parse::<u8>() {
                day.set(val);
                emit();
            }
        }
    };

    view! {
        <div class="grid grid-cols-3 gap-4">
            // Year
            <div>
                <label class="block text-xs font-medium text-zinc-500 mb-1">"Year"</label>
                <input
                    type="number"
                    class="w-full px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-white focus:border-purple-500 focus:outline-none"
                    prop:value=move || year.get()
                    on:change=handle_year_change
                />
            </div>

            // Month
            <div>
                <label class="block text-xs font-medium text-zinc-500 mb-1">"Month"</label>
                <select
                    class="w-full px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-white focus:border-purple-500 focus:outline-none"
                    on:change=handle_month_change
                >
                    {month_names.iter().enumerate().map(|(i, name)| {
                        let val = (i + 1) as u8;
                        view! {
                            <option value=val.to_string() selected=move || month.get() == val>
                                {name.clone()}
                            </option>
                        }
                    }).collect_view()}
                </select>
            </div>

            // Day
            <div>
                <label class="block text-xs font-medium text-zinc-500 mb-1">"Day"</label>
                <input
                    type="number"
                    min="1"
                    max="31"
                    class="w-full px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-white focus:border-purple-500 focus:outline-none"
                    prop:value=move || day.get()
                    on:change=handle_day_change
                />
            </div>
        </div>
    }
}

/// Date & Time tab content
#[component]
fn DateTimeContent(
    campaign_id: String,
    world_state: WorldState,
    #[prop(optional)]
    _on_update: Option<Callback<WorldState>>,
) -> impl IntoView {
    let current_date = RwSignal::new(world_state.current_date.clone());
    let is_saving = RwSignal::new(false);
    let calendar_config = world_state.calendar_config.clone();
    let calendar_config_display = calendar_config.clone();

    let campaign_id_advance = campaign_id.clone();
    let handle_advance_day = move |_: ev::MouseEvent| {
        let cid = campaign_id_advance.clone();
        spawn_local(async move {
            if let Ok(new_date) = advance_in_game_date(cid, 1).await {
                current_date.set(new_date);
            }
        });
    };

    let campaign_id_save = campaign_id.clone();
    let handle_save_date = move |_: ev::MouseEvent| {
        is_saving.set(true);
        let cid = campaign_id_save.clone();
        let date = current_date.get();
        spawn_local(async move {
            let _ = set_in_game_date(cid, date).await;
            is_saving.set(false);
        });
    };

    let handle_date_change = Callback::new(move |date: InGameDate| {
        current_date.set(date);
    });

    view! {
        <div class="space-y-6">
            // Current Date Display
            <div class="bg-zinc-900 border border-zinc-800 rounded-lg p-6">
                <h3 class="text-lg font-bold text-white mb-4">"Current In-Game Date"</h3>

                <div class="text-4xl font-bold text-purple-400 mb-6">
                    {move || {
                        let d = current_date.get();
                        let month_name = calendar_config_display.month_names.get(d.month as usize - 1)
                            .cloned()
                            .unwrap_or_else(|| format!("Month {}", d.month));
                        format!("{} {}, Year {}", month_name, d.day, d.year)
                    }}
                </div>

                {move || current_date.get().time.map(|t| {
                    view! {
                        <div class="text-2xl text-zinc-400 mb-4">
                            {format!("{:02}:{:02}", t.hour, t.minute)}
                            {t.period.map(|p| format!(" {}", p)).unwrap_or_default()}
                        </div>
                    }
                })}

                // Quick Actions
                <div class="flex gap-2">
                    <button
                        class="px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white rounded-lg transition-colors"
                        on:click=handle_advance_day
                    >
                        "Advance 1 Day"
                    </button>
                    <button
                        class="px-4 py-2 bg-zinc-700 hover:bg-zinc-600 text-white rounded-lg transition-colors"
                        disabled=move || is_saving.get()
                        on:click=handle_save_date
                    >
                        {move || if is_saving.get() { "Saving..." } else { "Save Date" }}
                    </button>
                </div>
            </div>

            // Date Editor
            <div class="bg-zinc-900 border border-zinc-800 rounded-lg p-6">
                <h3 class="text-lg font-bold text-white mb-4">"Edit Date"</h3>
                <InGameDatePicker
                    date=current_date.get()
                    calendar=Some(calendar_config.clone())
                    on_change=handle_date_change
                />
            </div>

            // Calendar Info
            <div class="bg-zinc-900 border border-zinc-800 rounded-lg p-6">
                <h3 class="text-lg font-bold text-white mb-4">"Calendar: " {calendar_config.name.clone()}</h3>
                <div class="grid grid-cols-2 gap-4 text-sm">
                    <div>
                        <span class="text-zinc-500">"Months per Year: "</span>
                        <span class="text-white">{calendar_config.months_per_year}</span>
                    </div>
                    <div>
                        <span class="text-zinc-500">"Days per Week: "</span>
                        <span class="text-white">{calendar_config.week_days.len()}</span>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Event type badge
#[component]
fn EventTypeBadge(
    #[prop(into)]
    event_type: String,
) -> impl IntoView {
    let (bg, text) = match event_type.as_str() {
        "combat" | "battle" => ("bg-red-900/50", "text-red-300"),
        "political" => ("bg-blue-900/50", "text-blue-300"),
        "natural" => ("bg-green-900/50", "text-green-300"),
        "magical" => ("bg-purple-900/50", "text-purple-300"),
        "economic" => ("bg-yellow-900/50", "text-yellow-300"),
        "social" => ("bg-pink-900/50", "text-pink-300"),
        "discovery" => ("bg-cyan-900/50", "text-cyan-300"),
        "milestone" => ("bg-orange-900/50", "text-orange-300"),
        _ => ("bg-zinc-700", "text-zinc-300"),
    };

    view! {
        <span class=format!("px-2 py-0.5 text-xs rounded-full {} {}", bg, text)>
            {event_type}
        </span>
    }
}

/// World event card
#[component]
fn WorldEventCard(
    event: WorldEvent,
    #[prop(optional)]
    on_delete: Option<Callback<String>>,
) -> impl IntoView {
    let event_id = event.id.clone();

    let handle_delete = move |_: ev::MouseEvent| {
        if let Some(ref cb) = on_delete {
            cb.run(event_id.clone());
        }
    };

    view! {
        <div class="bg-zinc-800 border border-zinc-700 rounded-lg p-4 hover:border-zinc-600 transition-colors">
            <div class="flex justify-between items-start">
                <div class="flex-1">
                    <div class="flex items-center gap-2 mb-2">
                        <EventTypeBadge event_type=event.event_type.clone() />
                        <span class="text-xs text-zinc-500">
                            {format!("{}/{}/{}", event.in_game_date.month, event.in_game_date.day, event.in_game_date.year)}
                        </span>
                    </div>
                    <h4 class="font-medium text-white mb-1">{event.title}</h4>
                    <p class="text-sm text-zinc-400">{event.description}</p>

                    {(!event.consequences.is_empty()).then(|| view! {
                        <div class="mt-2 pt-2 border-t border-zinc-700">
                            <span class="text-xs text-zinc-500 uppercase">"Consequences:"</span>
                            <ul class="text-xs text-zinc-400 mt-1">
                                {event.consequences.iter().map(|c| view! {
                                    <li class="flex items-center gap-1">
                                        <span class="text-zinc-600">"*"</span>
                                        {c.clone()}
                                    </li>
                                }).collect_view()}
                            </ul>
                        </div>
                    })}
                </div>

                {on_delete.as_ref().map(|_| view! {
                    <button
                        class="p-1 text-zinc-500 hover:text-red-400 transition-colors"
                        on:click=handle_delete.clone()
                    >
                        "X"
                    </button>
                })}
            </div>
        </div>
    }
}

/// Events tab content
#[component]
fn EventsContent(
    campaign_id: String,
    events: Vec<WorldEvent>,
    on_refresh: Callback<()>,
) -> impl IntoView {
    let show_create = RwSignal::new(false);
    let events_signal = RwSignal::new(events);
    let filter_type = RwSignal::new(String::new());

    let campaign_id_delete = campaign_id.clone();
    let on_refresh_clone = on_refresh.clone();
    let handle_delete = Callback::new(move |event_id: String| {
        let cid = campaign_id_delete.clone();
        let refresh = on_refresh_clone.clone();
        spawn_local(async move {
            if delete_world_event(cid, event_id).await.is_ok() {
                refresh.run(());
            }
        });
    });

    view! {
        <div class="space-y-4">
            // Header
            <div class="flex justify-between items-center">
                <div class="flex gap-2">
                    <select
                        class="px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-white text-sm"
                        on:change=move |evt| {
                            let target = event_target::<web_sys::HtmlSelectElement>(&evt);
                            filter_type.set(target.value());
                        }
                    >
                        <option value="">"All Types"</option>
                        <option value="combat">"Combat"</option>
                        <option value="political">"Political"</option>
                        <option value="natural">"Natural"</option>
                        <option value="magical">"Magical"</option>
                        <option value="economic">"Economic"</option>
                        <option value="social">"Social"</option>
                        <option value="discovery">"Discovery"</option>
                        <option value="milestone">"Milestone"</option>
                    </select>
                </div>

                <button
                    class="px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white rounded-lg transition-colors"
                    on:click=move |_| show_create.set(true)
                >
                    "+ Add Event"
                </button>
            </div>

            // Events List
            <div class="space-y-3">
                {move || {
                    let filter = filter_type.get();
                    let delete_cb = handle_delete.clone();
                    events_signal.get()
                        .into_iter()
                        .filter(|e| filter.is_empty() || e.event_type == filter)
                        .map(|event| {
                            let cb = delete_cb.clone();
                            view! {
                                <WorldEventCard event=event on_delete=cb />
                            }
                        })
                        .collect_view()
                }}

                {move || events_signal.get().is_empty().then(|| view! {
                    <div class="text-center py-12 text-zinc-500">
                        "No world events recorded yet"
                    </div>
                })}
            </div>
        </div>
    }
}

/// Location state card
#[component]
fn LocationStateCard(
    location: LocationState,
    #[prop(optional)]
    on_edit: Option<Callback<String>>,
) -> impl IntoView {
    let location_id = location.location_id.clone();

    let condition_color = match location.condition.as_str() {
        "pristine" | "blessed" => "text-emerald-400",
        "normal" | "stable" => "text-zinc-400",
        "damaged" | "occupied" | "contested" => "text-yellow-400",
        "ruined" | "destroyed" | "cursed" => "text-red-400",
        _ => "text-zinc-400",
    };

    let handle_edit = move |_: ev::MouseEvent| {
        if let Some(ref cb) = on_edit {
            cb.run(location_id.clone());
        }
    };

    view! {
        <div class="bg-zinc-800 border border-zinc-700 rounded-lg p-4 hover:border-zinc-600 transition-colors">
            <div class="flex justify-between items-start">
                <div class="flex-1">
                    <h4 class="font-medium text-white">{location.name}</h4>
                    <div class=format!("text-sm {}", condition_color)>
                        {location.condition}
                    </div>

                    <div class="grid grid-cols-2 gap-2 mt-3 text-xs">
                        {location.ruler.map(|r| view! {
                            <div>
                                <span class="text-zinc-500">"Ruler: "</span>
                                <span class="text-white">{r}</span>
                            </div>
                        })}
                        {location.controlling_faction.map(|f| view! {
                            <div>
                                <span class="text-zinc-500">"Faction: "</span>
                                <span class="text-white">{f}</span>
                            </div>
                        })}
                        {location.population.map(|p| view! {
                            <div>
                                <span class="text-zinc-500">"Population: "</span>
                                <span class="text-white">{p.to_string()}</span>
                            </div>
                        })}
                    </div>

                    {(!location.active_effects.is_empty()).then(|| view! {
                        <div class="flex flex-wrap gap-1 mt-2">
                            {location.active_effects.iter().map(|effect| view! {
                                <span class="px-1.5 py-0.5 text-xs bg-purple-900/50 text-purple-300 rounded">
                                    {effect.clone()}
                                </span>
                            }).collect_view()}
                        </div>
                    })}
                </div>

                {on_edit.as_ref().map(|_| view! {
                    <button
                        class="p-2 text-zinc-500 hover:text-white transition-colors"
                        on:click=handle_edit.clone()
                    >
                        "Edit"
                    </button>
                })}
            </div>
        </div>
    }
}

/// Locations tab content
#[component]
fn LocationsContent(
    #[prop(into)]
    _campaign_id: String,
    locations: Vec<LocationState>,
) -> impl IntoView {
    let locations_signal = RwSignal::new(locations);
    let search = RwSignal::new(String::new());

    let handle_search = move |evt: ev::Event| {
        let target = event_target::<web_sys::HtmlInputElement>(&evt);
        search.set(target.value());
    };

    view! {
        <div class="space-y-4">
            // Search
            <div>
                <input
                    type="text"
                    placeholder="Search locations..."
                    class="w-full px-4 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-white placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                    on:input=handle_search
                />
            </div>

            // Locations Grid
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {move || {
                    let query = search.get().to_lowercase();
                    locations_signal.get()
                        .into_iter()
                        .filter(|l| query.is_empty() || l.name.to_lowercase().contains(&query))
                        .map(|location| view! {
                            <LocationStateCard location=location />
                        })
                        .collect_view()
                }}
            </div>

            {move || locations_signal.get().is_empty().then(|| view! {
                <div class="text-center py-12">
                    <div class="text-zinc-500 mb-4">"No locations tracked yet"</div>
                    <button class="px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white rounded-lg transition-colors">
                        "+ Add Location"
                    </button>
                </div>
            })}
        </div>
    }
}

/// Custom field editor
#[component]
fn CustomFieldItem(
    key: String,
    value: serde_json::Value,
    on_delete: Callback<String>,
) -> impl IntoView {
    let key_clone = key.clone();

    let display_value = match &value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Array(a) => format!("[{} items]", a.len()),
        serde_json::Value::Object(o) => format!("{{{} fields}}", o.len()),
        serde_json::Value::Null => "null".to_string(),
    };

    let handle_delete = move |_: ev::MouseEvent| {
        on_delete.run(key_clone.clone());
    };

    view! {
        <div class="flex items-center justify-between p-3 bg-zinc-800 rounded-lg">
            <div class="flex-1">
                <span class="text-sm font-medium text-purple-400">{key}</span>
                <span class="text-zinc-500">" = "</span>
                <span class="text-white">{display_value}</span>
            </div>
            <button
                class="p-1 text-zinc-500 hover:text-red-400 transition-colors"
                on:click=handle_delete
            >
                "X"
            </button>
        </div>
    }
}

/// Custom fields tab content
#[component]
fn CustomFieldsContent(
    campaign_id: String,
    custom_fields: std::collections::HashMap<String, serde_json::Value>,
) -> impl IntoView {
    let fields = RwSignal::new(custom_fields);
    let new_key = RwSignal::new(String::new());
    let new_value = RwSignal::new(String::new());

    let campaign_id_add = campaign_id.clone();
    let handle_add = move |_: ev::MouseEvent| {
        let key = new_key.get();
        let val = new_value.get();
        if !key.is_empty() && !val.is_empty() {
            let cid = campaign_id_add.clone();
            spawn_local(async move {
                let json_value = serde_json::Value::String(val);
                if set_world_custom_field(cid, key.clone(), json_value.clone()).await.is_ok() {
                    fields.update(|f| {
                        f.insert(key, json_value);
                    });
                    new_key.set(String::new());
                    new_value.set(String::new());
                }
            });
        }
    };

    let handle_delete = Callback::new(move |key: String| {
        fields.update(|f| {
            f.remove(&key);
        });
    });

    view! {
        <div class="space-y-4">
            // Add new field
            <div class="bg-zinc-900 border border-zinc-800 rounded-lg p-4">
                <h3 class="text-sm font-bold text-zinc-400 uppercase mb-3">"Add Custom Field"</h3>
                <div class="flex gap-2">
                    <input
                        type="text"
                        placeholder="Field name..."
                        class="flex-1 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                        prop:value=move || new_key.get()
                        on:input=move |evt| {
                            let target = event_target::<web_sys::HtmlInputElement>(&evt);
                            new_key.set(target.value());
                        }
                    />
                    <input
                        type="text"
                        placeholder="Value..."
                        class="flex-1 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                        prop:value=move || new_value.get()
                        on:input=move |evt| {
                            let target = event_target::<web_sys::HtmlInputElement>(&evt);
                            new_value.set(target.value());
                        }
                    />
                    <button
                        class="px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white rounded-lg transition-colors"
                        on:click=handle_add
                    >
                        "Add"
                    </button>
                </div>
            </div>

            // Fields list
            <div class="space-y-2">
                {move || fields.get().into_iter().map(|(key, value)| {
                    view! {
                        <CustomFieldItem key=key value=value on_delete=handle_delete />
                    }
                }).collect_view()}

                {move || fields.get().is_empty().then(|| view! {
                    <div class="text-center py-8 text-zinc-500">
                        "No custom fields defined"
                    </div>
                })}
            </div>
        </div>
    }
}

/// Main world state editor component
#[component]
pub fn WorldStateEditor(
    /// Campaign ID
    campaign_id: String,
) -> impl IntoView {
    let active_tab = RwSignal::new(WorldStateTab::Date);
    let world_state = RwSignal::new(Option::<WorldState>::None);
    let is_loading = RwSignal::new(true);
    let error = RwSignal::new(Option::<String>::None);

    // Load world state
    let campaign_id_load = campaign_id.clone();
    Effect::new(move |_| {
        let cid = campaign_id_load.clone();
        spawn_local(async move {
            is_loading.set(true);
            error.set(None);

            match get_world_state(cid).await {
                Ok(ws) => world_state.set(Some(ws)),
                Err(e) => error.set(Some(e)),
            }

            is_loading.set(false);
        });
    });

    let handle_tab_change = Callback::new(move |tab: WorldStateTab| {
        active_tab.set(tab);
    });

    let campaign_id_refresh = campaign_id.clone();
    let handle_refresh = Callback::new(move |_: ()| {
        let cid = campaign_id_refresh.clone();
        spawn_local(async move {
            if let Ok(ws) = get_world_state(cid).await {
                world_state.set(Some(ws));
            }
        });
    });

    let campaign_id_content = campaign_id.clone();

    view! {
        <div class="space-y-4">
            // Tab Navigation
            <div class="flex gap-2 bg-zinc-900 p-2 rounded-lg">
                <WorldStateTabButton tab=WorldStateTab::Date active_tab=active_tab.get() on_click=handle_tab_change />
                <WorldStateTabButton tab=WorldStateTab::Events active_tab=active_tab.get() on_click=handle_tab_change />
                <WorldStateTabButton tab=WorldStateTab::Locations active_tab=active_tab.get() on_click=handle_tab_change />
                <WorldStateTabButton tab=WorldStateTab::Custom active_tab=active_tab.get() on_click=handle_tab_change />
            </div>

            // Content
            {move || {
                if is_loading.get() {
                    view! {
                        <div class="text-center py-12 text-zinc-500">"Loading world state..."</div>
                    }.into_any()
                } else if let Some(err) = error.get() {
                    view! {
                        <div class="text-center py-12 text-red-400">{err}</div>
                    }.into_any()
                } else if let Some(ws) = world_state.get() {
                    let cid = campaign_id_content.clone();
                    match active_tab.get() {
                        WorldStateTab::Date => view! {
                            <DateTimeContent
                                campaign_id=cid.clone()
                                world_state=ws.clone()
                            />
                        }.into_any(),
                        WorldStateTab::Events => view! {
                            <EventsContent
                                campaign_id=cid
                                events=ws.events
                                on_refresh=handle_refresh
                            />
                        }.into_any(),
                        WorldStateTab::Locations => view! {
                            <LocationsContent
                                _campaign_id=cid
                                locations=ws.locations.into_values().collect()
                            />
                        }.into_any(),
                        WorldStateTab::Custom => view! {
                            <CustomFieldsContent
                                campaign_id=cid
                                custom_fields=ws.custom_fields
                            />
                        }.into_any(),
                    }
                } else {
                    view! {
                        <div class="text-center py-12 text-zinc-500">"No world state data"</div>
                    }.into_any()
                }
            }}
        </div>
    }
}
