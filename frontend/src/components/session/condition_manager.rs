//! Condition Manager Component (TASK-015/TASK-016)
//!
//! Advanced condition management with duration tracking, stacking rules,
//! and full D&D 5e condition support.

use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::bindings::{
    add_condition, add_condition_advanced, get_combatant_conditions, remove_condition,
    remove_condition_by_id, AddConditionRequest, AdvancedCondition, ConditionDurationType,
};
use crate::components::design_system::{Button, ButtonVariant};

/// Common D&D 5e conditions with descriptions and colors
const COMMON_CONDITIONS: &[(&str, &str, &str)] = &[
    (
        "Blinded",
        "Can't see. Auto-fails sight checks. Disadvantage on attacks.",
        "#6b7280",
    ),
    (
        "Charmed",
        "Can't attack charmer. Charmer has advantage on social checks.",
        "#ec4899",
    ),
    (
        "Deafened",
        "Can't hear. Auto-fails hearing checks.",
        "#78716c",
    ),
    (
        "Frightened",
        "Disadvantage while source visible. Can't approach source.",
        "#eab308",
    ),
    ("Grappled", "Speed becomes 0.", "#78716c"),
    (
        "Incapacitated",
        "Can't take actions or reactions.",
        "#dc2626",
    ),
    (
        "Invisible",
        "Can't be seen. Advantage on attacks. Attacks against have disadvantage.",
        "#a855f7",
    ),
    (
        "Paralyzed",
        "Incapacitated. Auto-fails STR/DEX saves. Attacks have advantage.",
        "#7c3aed",
    ),
    (
        "Petrified",
        "Turned to stone. Weight x10. Resistant to all damage.",
        "#78716c",
    ),
    (
        "Poisoned",
        "Disadvantage on attacks and ability checks.",
        "#22c55e",
    ),
    (
        "Prone",
        "Can only crawl. Disadvantage on attacks.",
        "#78716c",
    ),
    (
        "Restrained",
        "Speed 0. Disadvantage on attacks and DEX saves.",
        "#ea580c",
    ),
    (
        "Stunned",
        "Incapacitated. Auto-fails STR/DEX saves.",
        "#fbbf24",
    ),
    (
        "Unconscious",
        "Incapacitated, prone, unaware. Attacks within 5ft are crits.",
        "#1e3a8a",
    ),
    (
        "Exhaustion",
        "Various effects based on level (1-6).",
        "#4b5563",
    ),
    (
        "Concentrating",
        "Maintaining a spell. CON save on damage.",
        "#3b82f6",
    ),
];

/// Saving throw types
const SAVE_TYPES: &[&str] = &["STR", "DEX", "CON", "INT", "WIS", "CHA"];

/// Advanced Condition Modal with full duration/stacking support
#[component]
pub fn AdvancedConditionModal(
    /// Target combatant ID
    combatant_id: RwSignal<Option<String>>,
    /// Session ID
    session_id: Signal<String>,
    /// Close callback
    on_close: Callback<()>,
    /// Condition added callback
    on_condition_added: Callback<()>,
) -> impl IntoView {
    // Form state
    let custom_condition = RwSignal::new(String::new());
    let duration_type = RwSignal::new(ConditionDurationType::UntilRemoved);
    let duration_value = RwSignal::new(1_u32);
    let save_type = RwSignal::new("CON".to_string());
    let save_dc = RwSignal::new(15_u32);
    let show_advanced = RwSignal::new(false);
    let selected_tab = RwSignal::new("common".to_string());

    // Add advanced condition handler
    let handle_add_advanced = move |condition_name: String| {
        let sid = session_id.get();
        let cid = combatant_id.get().unwrap_or_default();

        if cid.is_empty() || condition_name.is_empty() {
            return;
        }

        let dur_type = duration_type.get();
        let dur_val = duration_value.get();
        let s_type = save_type.get();
        let s_dc = save_dc.get();

        let request = AddConditionRequest {
            session_id: sid,
            combatant_id: cid,
            condition_name,
            duration_type: Some(dur_type.to_string_key().to_string()),
            duration_value: Some(dur_val),
            source_id: None,
            source_name: None,
            save_type: if dur_type == ConditionDurationType::UntilSave {
                Some(s_type)
            } else {
                None
            },
            save_dc: if dur_type == ConditionDurationType::UntilSave {
                Some(s_dc)
            } else {
                None
            },
        };

        spawn_local(async move {
            if add_condition_advanced(request).await.is_ok() {
                on_condition_added.run(());
            }
        });
    };

    // Quick add (simple mode - until removed)
    let handle_quick_add = move |condition: &str| {
        let sid = session_id.get();
        let cid = combatant_id.get().unwrap_or_default();

        if cid.is_empty() {
            return;
        }

        let condition_name = condition.to_string();
        spawn_local(async move {
            if add_condition(sid, cid, condition_name).await.is_ok() {
                on_condition_added.run(());
            }
        });
    };

    // Add custom condition with advanced options
    let handle_add_custom = move |_: ev::MouseEvent| {
        let condition = custom_condition.get();
        if !condition.is_empty() {
            if show_advanced.get() {
                handle_add_advanced(condition);
            } else {
                let sid = session_id.get();
                let cid = combatant_id.get().unwrap_or_default();
                let cond = condition.clone();
                spawn_local(async move {
                    if add_condition(sid, cid, cond).await.is_ok() {
                        on_condition_added.run(());
                    }
                });
            }
            custom_condition.set(String::new());
        }
    };

    view! {
        <div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50 p-4">
            <div class="w-full max-w-2xl bg-zinc-900 rounded-xl border border-zinc-700 shadow-2xl overflow-hidden">
                // Header
                <div class="flex items-center justify-between px-6 py-4 border-b border-zinc-700 bg-zinc-800/50">
                    <div class="flex items-center gap-3">
                        <h3 class="text-lg font-bold text-zinc-100">"Manage Conditions"</h3>
                        <span class="px-2 py-0.5 text-xs font-medium bg-purple-600/30 text-purple-300 rounded">
                            "TASK-015"
                        </span>
                    </div>
                    <button
                        class="p-1 text-zinc-400 hover:text-white transition-colors rounded-lg hover:bg-zinc-700"
                        on:click=move |_| on_close.run(())
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Tabs
                <div class="flex border-b border-zinc-700">
                    <button
                        class=move || format!(
                            "px-4 py-2 text-sm font-medium transition-colors {}",
                            if selected_tab.get() == "common" {
                                "text-purple-400 border-b-2 border-purple-400 bg-purple-900/20"
                            } else {
                                "text-zinc-400 hover:text-zinc-200"
                            }
                        )
                        on:click=move |_| selected_tab.set("common".to_string())
                    >
                        "Quick Add"
                    </button>
                    <button
                        class=move || format!(
                            "px-4 py-2 text-sm font-medium transition-colors {}",
                            if selected_tab.get() == "advanced" {
                                "text-purple-400 border-b-2 border-purple-400 bg-purple-900/20"
                            } else {
                                "text-zinc-400 hover:text-zinc-200"
                            }
                        )
                        on:click=move |_| selected_tab.set("advanced".to_string())
                    >
                        "Custom + Duration"
                    </button>
                </div>

                // Body
                <div class="p-6 space-y-6 max-h-[60vh] overflow-y-auto">
                    // Quick Add Tab
                    <Show when=move || selected_tab.get() == "common">
                        <div>
                            <h4 class="text-sm font-medium text-zinc-400 mb-3">"Standard Conditions"</h4>
                            <div class="grid grid-cols-2 md:grid-cols-3 gap-2">
                                {COMMON_CONDITIONS.iter().map(|(name, desc, color)| {
                                    let name_str = name.to_string();
                                    let name_clone = name_str.clone();
                                    view! {
                                        <button
                                            class="group p-3 text-left bg-zinc-800 hover:bg-zinc-700 rounded-lg border border-zinc-700 hover:border-zinc-600 transition-all"
                                            on:click=move |_| handle_quick_add(&name_clone)
                                        >
                                            <div class="flex items-center gap-2 mb-1">
                                                <div
                                                    class="w-2 h-2 rounded-full"
                                                    style:background-color=*color
                                                />
                                                <span class="font-medium text-zinc-200 text-sm group-hover:text-white">
                                                    {*name}
                                                </span>
                                            </div>
                                            <p class="text-xs text-zinc-500 line-clamp-2">
                                                {*desc}
                                            </p>
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                        </div>
                    </Show>

                    // Advanced Tab - Custom + Duration
                    <Show when=move || selected_tab.get() == "advanced">
                        <div class="space-y-4">
                            // Condition name
                            <div>
                                <label class="block text-sm font-medium text-zinc-400 mb-2">"Condition Name"</label>
                                <input
                                    type="text"
                                    placeholder="Enter condition name..."
                                    class="w-full px-4 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                                    prop:value=move || custom_condition.get()
                                    on:input=move |ev| custom_condition.set(event_target_value(&ev))
                                />
                            </div>

                            // Duration type
                            <div>
                                <label class="block text-sm font-medium text-zinc-400 mb-2">"Duration Type"</label>
                                <select
                                    class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm focus:border-purple-500 focus:outline-none"
                                    on:change=move |ev| {
                                        let val = event_target_value(&ev);
                                        let dt = match val.as_str() {
                                            "turns" => ConditionDurationType::Turns,
                                            "rounds" => ConditionDurationType::Rounds,
                                            "minutes" => ConditionDurationType::Minutes,
                                            "hours" => ConditionDurationType::Hours,
                                            "end_of_next_turn" => ConditionDurationType::EndOfNextTurn,
                                            "start_of_next_turn" => ConditionDurationType::StartOfNextTurn,
                                            "until_save" => ConditionDurationType::UntilSave,
                                            "permanent" => ConditionDurationType::Permanent,
                                            _ => ConditionDurationType::UntilRemoved,
                                        };
                                        duration_type.set(dt);
                                    }
                                >
                                    <option value="until_removed" selected>"Until Removed"</option>
                                    <option value="turns">"Turns"</option>
                                    <option value="rounds">"Rounds"</option>
                                    <option value="minutes">"Minutes"</option>
                                    <option value="end_of_next_turn">"End of Next Turn"</option>
                                    <option value="start_of_next_turn">"Start of Next Turn"</option>
                                    <option value="until_save">"Until Save"</option>
                                    <option value="permanent">"Permanent"</option>
                                </select>
                            </div>

                            // Duration value (for turns/rounds/minutes)
                            <Show when=move || matches!(duration_type.get(), ConditionDurationType::Turns | ConditionDurationType::Rounds | ConditionDurationType::Minutes | ConditionDurationType::Hours)>
                                <div>
                                    <label class="block text-sm font-medium text-zinc-400 mb-2">"Duration Value"</label>
                                    <input
                                        type="number"
                                        class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm focus:border-purple-500 focus:outline-none"
                                        prop:value=move || duration_value.get().to_string()
                                        on:input=move |ev| {
                                            if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                                                duration_value.set(v.max(1));
                                            }
                                        }
                                        min="1"
                                    />
                                </div>
                            </Show>

                            // Save options (for until_save)
                            <Show when=move || duration_type.get() == ConditionDurationType::UntilSave>
                                <div class="grid grid-cols-2 gap-4">
                                    <div>
                                        <label class="block text-sm font-medium text-zinc-400 mb-2">"Save Type"</label>
                                        <select
                                            class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm focus:border-purple-500 focus:outline-none"
                                            on:change=move |ev| save_type.set(event_target_value(&ev))
                                        >
                                            {SAVE_TYPES.iter().map(|st| {
                                                let st_val = st.to_string();
                                                view! {
                                                    <option value=st_val.clone()>{*st}</option>
                                                }
                                            }).collect_view()}
                                        </select>
                                    </div>
                                    <div>
                                        <label class="block text-sm font-medium text-zinc-400 mb-2">"Save DC"</label>
                                        <input
                                            type="number"
                                            class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm focus:border-purple-500 focus:outline-none"
                                            prop:value=move || save_dc.get().to_string()
                                            on:input=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                                                    save_dc.set(v.max(1));
                                                }
                                            }
                                            min="1"
                                        />
                                    </div>
                                </div>
                            </Show>

                            // Add button
                            <div class="pt-2">
                                <Button
                                    variant=ButtonVariant::Primary
                                    class="w-full py-2 bg-purple-600 hover:bg-purple-500 text-white font-medium"
                                    on_click=handle_add_custom
                                    disabled=(move || custom_condition.get().is_empty())()
                                >
                                    "Add Condition"
                                </Button>
                            </div>
                        </div>
                    </Show>
                </div>

                // Footer
                <div class="flex justify-end gap-2 px-6 py-4 border-t border-zinc-700 bg-zinc-800/30">
                    <Button
                        variant=ButtonVariant::Ghost
                        class="px-4 py-2 bg-zinc-700 text-zinc-300 text-sm"
                        on_click=move |_: ev::MouseEvent| on_close.run(())
                    >
                        "Close"
                    </Button>
                </div>
            </div>
        </div>
    }
}

/// Legacy Condition modal (backward compatible)
#[component]
pub fn ConditionModal(
    /// Target combatant ID
    combatant_id: RwSignal<Option<String>>,
    /// Session ID
    session_id: Signal<String>,
    /// Close callback
    on_close: Callback<()>,
    /// Condition added callback
    on_condition_added: Callback<()>,
) -> impl IntoView {
    // Delegate to advanced modal
    view! {
        <AdvancedConditionModal
            combatant_id=combatant_id
            session_id=session_id
            on_close=on_close
            on_condition_added=on_condition_added
        />
    }
}

// ============================================================================
// Condition Badge with Duration Display
// ============================================================================

/// A condition badge that shows details on hover with duration support
#[component]
pub fn ConditionBadge(
    /// Condition name
    name: String,
    /// Optional duration text
    #[prop(optional)]
    duration: Option<String>,
    /// Callback to remove condition
    #[prop(optional)]
    on_remove: Option<Callback<()>>,
) -> impl IntoView {
    let condition_info = COMMON_CONDITIONS
        .iter()
        .find(|(n, _, _)| n.to_lowercase() == name.to_lowercase())
        .map(|(_, desc, color)| (*desc, *color));

    let (description, color) = condition_info.unwrap_or(("Custom condition", "#94a3b8"));

    view! {
        <div class="group relative inline-block">
            <div
                class="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium"
                style:background-color=format!("{}20", color)
                style:color=color
                style:border=format!("1px solid {}40", color)
            >
                <span>{name.clone()}</span>
                {duration.map(|d| view! {
                    <span class="opacity-70 text-[10px]">{format!("({})", d)}</span>
                })}
                {on_remove.map(|callback| view! {
                    <button
                        class="ml-1 opacity-50 hover:opacity-100 transition-opacity"
                        on:click=move |_| callback.run(())
                    >
                        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                })}
            </div>

            // Tooltip
            <div class="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg shadow-xl opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-200 z-50 w-48 pointer-events-none">
                <p class="text-xs text-zinc-300">{description}</p>
                <div class="absolute top-full left-1/2 -translate-x-1/2 -mt-1">
                    <div class="border-4 border-transparent border-t-zinc-700"></div>
                </div>
            </div>
        </div>
    }
}

/// Advanced condition badge with full duration display
#[component]
pub fn AdvancedConditionBadge(
    /// The advanced condition
    condition: AdvancedCondition,
    /// Callback to remove condition (receives condition ID)
    #[prop(optional)]
    on_remove: Option<Callback<String>>,
) -> impl IntoView {
    let condition_id = condition.id.clone();
    let condition_name = condition.name.clone();
    let duration_display = condition.duration_display();

    let condition_info = COMMON_CONDITIONS
        .iter()
        .find(|(n, _, _)| n.to_lowercase() == condition_name.to_lowercase())
        .map(|(_, desc, color)| (*desc, *color));

    let (description, color) = condition_info
        .map(|(d, c)| (d.to_string(), c))
        .unwrap_or((condition.description.clone(), "#94a3b8"));

    view! {
        <div class="group relative inline-block">
            <div
                class="inline-flex items-center gap-1.5 px-2 py-1 rounded text-xs font-medium"
                style:background-color=format!("{}20", color)
                style:color=color
                style:border=format!("1px solid {}40", color)
            >
                <span class="font-semibold">{condition_name}</span>
                <span class="opacity-70 text-[10px] bg-black/20 px-1 rounded">{duration_display}</span>
                {on_remove.map(|callback| {
                    let cid = condition_id.clone();
                    view! {
                        <button
                            class="ml-1 opacity-50 hover:opacity-100 transition-opacity"
                            on:click=move |_| callback.run(cid.clone())
                        >
                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                            </svg>
                        </button>
                    }
                })}
            </div>

            // Tooltip with effects
            <div class="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg shadow-xl opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-200 z-50 w-56 pointer-events-none">
                <p class="text-xs text-zinc-300 mb-1">{description}</p>
                {condition.source_name.as_ref().map(|src| view! {
                    <p class="text-[10px] text-zinc-500">"Source: " {src.clone()}</p>
                })}
                <div class="absolute top-full left-1/2 -translate-x-1/2 -mt-1">
                    <div class="border-4 border-transparent border-t-zinc-700"></div>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Active Conditions List (Legacy)
// ============================================================================

/// List of active conditions for a combatant (legacy simple conditions)
#[component]
pub fn ActiveConditionsList(
    /// Session ID
    session_id: Signal<String>,
    /// Combatant ID
    combatant_id: String,
    /// List of condition names
    conditions: Vec<String>,
    /// Callback when condition is removed
    on_condition_removed: Callback<()>,
) -> impl IntoView {
    let cid = StoredValue::new(combatant_id);

    view! {
        <div class="flex flex-wrap gap-1">
            {conditions.into_iter().map(|condition| {
                let condition_name = condition.clone();
                let condition_for_remove = condition.clone();

                let handle_remove = move || {
                    let sid = session_id.get();
                    let combatant = cid.get_value();
                    let cond = condition_for_remove.clone();

                    spawn_local(async move {
                        if remove_condition(sid, combatant, cond).await.is_ok() {
                            on_condition_removed.run(());
                        }
                    });
                };

                view! {
                    <ConditionBadge
                        name=condition_name
                        on_remove=Callback::new(move |_| handle_remove())
                    />
                }
            }).collect_view()}
        </div>
    }
}

// ============================================================================
// Advanced Conditions List
// ============================================================================

/// List of advanced conditions for a combatant with full duration display
#[component]
pub fn AdvancedConditionsList(
    /// Session ID
    session_id: Signal<String>,
    /// Combatant ID
    combatant_id: String,
    /// Callback when condition is removed
    on_condition_removed: Callback<()>,
) -> impl IntoView {
    let cid = StoredValue::new(combatant_id.clone());
    let conditions = RwSignal::new(Vec::<AdvancedCondition>::new());

    // Fetch conditions on mount
    {
        let sid = session_id.get();
        let combatant = combatant_id.clone();
        spawn_local(async move {
            if let Ok(conds) = get_combatant_conditions(sid, combatant).await {
                conditions.set(conds);
            }
        });
    }

    let handle_remove = move |condition_id: String| {
        let sid = session_id.get();
        let combatant = cid.get_value();
        let cond_id = condition_id.clone();

        spawn_local(async move {
            if remove_condition_by_id(sid.clone(), combatant.clone(), cond_id)
                .await
                .is_ok()
            {
                // Refresh conditions
                if let Ok(conds) = get_combatant_conditions(sid, combatant).await {
                    conditions.set(conds);
                }
                on_condition_removed.run(());
            }
        });
    };

    view! {
        <div class="flex flex-wrap gap-1.5">
            <For
                each=move || conditions.get()
                key=|c| c.id.clone()
                children=move |condition| {
                    view! {
                        <AdvancedConditionBadge
                            condition=condition
                            on_remove=Callback::new(move |id: String| handle_remove(id))
                        />
                    }
                }
            />
        </div>
    }
}

// ============================================================================
// Condition Summary Panel
// ============================================================================

/// Panel showing condition summary for a combatant
#[component]
pub fn ConditionSummaryPanel(
    /// Session ID
    session_id: Signal<String>,
    /// Combatant ID
    combatant_id: String,
    /// Simple conditions list (legacy)
    simple_conditions: Vec<String>,
    /// Callback when conditions change
    on_conditions_changed: Callback<()>,
) -> impl IntoView {
    let cid = StoredValue::new(combatant_id.clone());
    let modal_open = RwSignal::new(false);
    let modal_combatant = RwSignal::new(Option::<String>::None);

    let open_modal = move || {
        modal_combatant.set(Some(cid.get_value()));
        modal_open.set(true);
    };

    view! {
        <div class="space-y-2">
            // Simple conditions (legacy)
            {
                let simple_conditions_check = simple_conditions.clone();
                view! {
                    <Show when=move || !simple_conditions_check.is_empty()>
                        <div class="flex flex-wrap gap-1">
                    {simple_conditions.iter().map(|condition| {
                        let condition_name = condition.clone();
                        let condition_for_remove = condition.clone();
                        let combatant = cid.get_value();

                        let handle_remove = move || {
                            let sid = session_id.get();
                            let comb = combatant.clone();
                            let cond = condition_for_remove.clone();

                            spawn_local(async move {
                                if remove_condition(sid, comb, cond).await.is_ok() {
                                    on_conditions_changed.run(());
                                }
                            });
                        };

                        view! {
                            <ConditionBadge
                                name=condition_name
                                on_remove=Callback::new(move |_| handle_remove())
                            />
                        }
                    }).collect_view()}
                </div>
            </Show>
        }
            }

            // Advanced conditions
            <AdvancedConditionsList
                session_id=session_id
                combatant_id=combatant_id.clone()
                on_condition_removed=on_conditions_changed
            />

            // Add condition button
            <button
                class="inline-flex items-center gap-1 px-2 py-1 text-xs text-zinc-400 hover:text-zinc-200 hover:bg-zinc-700/50 rounded transition-colors"
                on:click=move |_| open_modal()
            >
                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                </svg>
                "Add Condition"
            </button>

            // Modal
            <Show when=move || modal_open.get()>
                <AdvancedConditionModal
                    combatant_id=modal_combatant
                    session_id=session_id
                    on_close=Callback::new(move |_| modal_open.set(false))
                    on_condition_added=Callback::new(move |_| {
                        modal_open.set(false);
                        on_conditions_changed.run(());
                    })
                />
            </Show>
        </div>
    }
}
