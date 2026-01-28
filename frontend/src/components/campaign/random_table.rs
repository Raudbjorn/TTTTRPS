//! Random Table Component
//!
//! Phase 8 of the Campaign Generation Overhaul.
//!
//! Interactive random table display with:
//! - Probability visualization
//! - Roll button with animated result
//! - Roll history sidebar
//! - Table editing support

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use js_sys;

// ============================================================================
// Types (mirrored from backend)
// ============================================================================

/// Table entry for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableEntry {
    pub id: String,
    pub range_start: i32,
    pub range_end: i32,
    pub result_text: String,
    pub weight: f64,
    pub nested_table_id: Option<String>,
}

/// Random table data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomTable {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub dice_notation: String,
    pub category: Option<String>,
    pub entries: Vec<TableEntry>,
}

/// Roll result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRollResult {
    pub table_id: String,
    pub table_name: String,
    pub roll_total: i32,
    pub entry: TableEntry,
    pub final_text: String,
}

/// Roll history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollHistoryEntry {
    pub id: String,
    pub dice_notation: String,
    pub raw_roll: i32,
    pub final_result: i32,
    pub result_text: Option<String>,
    pub rolled_at: String,
}

// ============================================================================
// Random Table Display Component
// ============================================================================

/// Main random table display component
#[component]
pub fn RandomTableDisplay(
    /// The table to display
    table: RandomTable,
    /// Whether editing is enabled (reserved for future use)
    #[prop(default = false)]
    _editable: bool,
    /// Callback when a roll is made
    #[prop(optional)]
    on_roll: Option<Callback<TableRollResult>>,
    /// Session ID for history tracking (reserved for future use)
    #[prop(optional)]
    _session_id: Option<String>,
) -> impl IntoView {
    // State for current roll result
    let roll_result = RwSignal::<Option<TableRollResult>>::new(None);
    let is_rolling = RwSignal::new(false);
    let roll_animation = RwSignal::new(false);

    // Calculate probability for each entry
    let entries_with_probability = {
        let table = table.clone();
        let dice_notation = table.dice_notation.clone();
        let max_roll = parse_max_roll(&dice_notation);

        table.entries.iter().map(|entry| {
            let range_size = (entry.range_end - entry.range_start + 1) as f64;
            let probability = range_size / max_roll as f64 * 100.0;
            (entry.clone(), probability)
        }).collect::<Vec<_>>()
    };

    // Handle roll button click
    let handle_roll = {
        let table = table.clone();
        let on_roll = on_roll.clone();
        move |_| {
            is_rolling.set(true);
            roll_animation.set(true);

            // Simulate roll (in production, this would call the backend)
            let result = simulate_roll(&table);
            roll_result.set(Some(result.clone()));

            if let Some(callback) = &on_roll {
                callback.run(result);
            }

            // Reset animation after delay
            set_timeout(
                move || {
                    is_rolling.set(false);
                    roll_animation.set(false);
                },
                std::time::Duration::from_millis(500),
            );
        }
    };

    view! {
        <div class="random-table bg-zinc-900 rounded-xl border border-zinc-800 overflow-hidden">
            // Header with table name and roll button
            <div class="flex items-center justify-between px-4 py-3 bg-zinc-800/50 border-b border-zinc-700">
                <div>
                    <h3 class="text-lg font-semibold text-white">{table.name.clone()}</h3>
                    <span class="text-sm text-zinc-400">
                        "Roll: " {table.dice_notation.clone()}
                    </span>
                </div>
                <button
                    class=move || format!(
                        "px-4 py-2 rounded-lg font-medium transition-all {}",
                        if roll_animation.get() {
                            "bg-purple-500 text-white animate-pulse scale-105"
                        } else {
                            "bg-purple-600 hover:bg-purple-500 text-white"
                        }
                    )
                    on:click=handle_roll
                    disabled=move || is_rolling.get()
                >
                    {move || if is_rolling.get() { "Rolling..." } else { "Roll" }}
                </button>
            </div>

            // Description if present
            {table.description.clone().map(|desc| view! {
                <div class="px-4 py-2 text-sm text-zinc-400 border-b border-zinc-800">
                    {desc}
                </div>
            })}

            // Roll result display
            <Show when=move || roll_result.get().is_some()>
                <div class="mx-4 my-3 p-4 bg-purple-900/30 border border-purple-700 rounded-lg">
                    <div class="flex items-center gap-3">
                        <div class="w-12 h-12 bg-purple-600 rounded-lg flex items-center justify-center">
                            <span class="text-2xl font-bold text-white">
                                {move || roll_result.get().map(|r| r.roll_total.to_string()).unwrap_or_default()}
                            </span>
                        </div>
                        <div class="flex-1">
                            <p class="text-white font-medium">
                                {move || roll_result.get().map(|r| r.final_text.clone()).unwrap_or_default()}
                            </p>
                        </div>
                    </div>
                </div>
            </Show>

            // Table entries with probability bars
            <div class="divide-y divide-zinc-800">
                {entries_with_probability.into_iter().map(|(entry, probability)| {
                    let is_rolled = Signal::derive({
                        let entry_id = entry.id.clone();
                        move || roll_result.get().map(|r| r.entry.id == entry_id).unwrap_or(false)
                    });

                    view! {
                        <div class=move || format!(
                            "relative px-4 py-3 transition-colors {}",
                            if is_rolled.get() {
                                "bg-purple-900/20"
                            } else {
                                "hover:bg-zinc-800/50"
                            }
                        )>
                            // Probability background bar
                            <div
                                class="absolute inset-y-0 left-0 bg-purple-600/10"
                                style=format!("width: {}%", probability)
                            />

                            // Content
                            <div class="relative flex items-center gap-3">
                                // Range badge
                                <div class="shrink-0 px-2 py-1 bg-zinc-800 rounded text-xs text-zinc-300 font-mono min-w-[3rem] text-center">
                                    {if entry.range_start == entry.range_end {
                                        entry.range_start.to_string()
                                    } else {
                                        format!("{}-{}", entry.range_start, entry.range_end)
                                    }}
                                </div>

                                // Result text
                                <p class="flex-1 text-zinc-200">
                                    {entry.result_text.clone()}
                                </p>

                                // Probability percentage
                                <span class="shrink-0 text-xs text-zinc-500 tabular-nums">
                                    {format!("{:.1}%", probability)}
                                </span>

                                // Nested indicator
                                {entry.nested_table_id.as_ref().map(|_| view! {
                                    <span class="shrink-0 px-1.5 py-0.5 bg-blue-900/50 text-blue-300 text-xs rounded">
                                        "nested"
                                    </span>
                                })}
                            </div>
                        </div>
                    }
                }).collect::<Vec<_>>()}
            </div>

            // Footer with category if present
            {table.category.clone().map(|cat| view! {
                <div class="px-4 py-2 text-xs text-zinc-500 border-t border-zinc-800">
                    "Category: " {cat}
                </div>
            })}
        </div>
    }
}

// ============================================================================
// Roll History Sidebar Component
// ============================================================================

/// Roll history sidebar component
#[component]
pub fn RollHistorySidebar(
    /// Roll history entries
    history: Vec<RollHistoryEntry>,
    /// Whether the sidebar is visible
    visible: RwSignal<bool>,
) -> impl IntoView {
    // If not visible, short-circuit rendering
    if !visible.get_untracked() {
        return view! {
            <Show when=move || visible.get()>
                <div></div>
            </Show>
        }.into_any();
    }

    let has_history = !history.is_empty();

    view! {
        <Show when=move || visible.get()>
            <div class="w-80 bg-zinc-900 border-l border-zinc-800 flex flex-col h-full">
                // Header
                <div class="flex items-center justify-between px-4 py-3 border-b border-zinc-800">
                    <h3 class="font-semibold text-white">"Roll History"</h3>
                    <button
                        class="p-1 text-zinc-400 hover:text-white transition-colors"
                        on:click=move |_| visible.set(false)
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                        </svg>
                    </button>
                </div>

                // History list
                <div class="flex-1 overflow-y-auto">
                    {if has_history {
                        view! {
                            <div class="divide-y divide-zinc-800">
                                {history.iter().map(|entry| view! {
                                    <div class="px-4 py-3">
                                        <div class="flex items-center justify-between mb-1">
                                            <span class="font-mono text-sm text-zinc-300">
                                                {entry.dice_notation.clone()}
                                            </span>
                                            <span class="text-lg font-bold text-purple-400">
                                                {entry.final_result}
                                            </span>
                                        </div>
                                        {entry.result_text.clone().map(|text| view! {
                                            <p class="text-sm text-zinc-400 line-clamp-2">
                                                {text}
                                            </p>
                                        })}
                                        <p class="mt-1 text-xs text-zinc-600">
                                            {format_timestamp(&entry.rolled_at)}
                                        </p>
                                    </div>
                                }).collect_view()}
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="px-4 py-8 text-center text-zinc-500">
                                "No rolls yet"
                            </div>
                        }.into_any()
                    }}
                </div>
            </div>
        </Show>
    }.into_any()
}

// ============================================================================
// Dice Roller Widget Component
// ============================================================================

/// Quick dice roller widget
#[component]
pub fn DiceRollerWidget(
    /// Initial dice notation
    #[prop(default = "d20".to_string())]
    initial_notation: String,
    /// Callback when a roll is made
    #[prop(optional)]
    on_roll: Option<Callback<(String, i32)>>,
) -> impl IntoView {
    let notation = RwSignal::new(initial_notation);
    let last_roll = RwSignal::<Option<i32>>::new(None);
    let is_rolling = RwSignal::new(false);

    // Quick dice buttons
    let quick_dice = vec!["d4", "d6", "d8", "d10", "d12", "d20", "d100"];

    let handle_roll = {
        let on_roll = on_roll.clone();
        move |_| {
            is_rolling.set(true);

            // Simulate roll
            let min = parse_min_roll(&notation.get());
            let max = parse_max_roll(&notation.get());
            let result = simulate_dice_roll(min, max);
            last_roll.set(Some(result));

            if let Some(callback) = &on_roll {
                callback.run((notation.get(), result));
            }

            set_timeout(move || is_rolling.set(false), std::time::Duration::from_millis(300));
        }
    };

    view! {
        <div class="bg-zinc-900 rounded-xl border border-zinc-800 p-4">
            <h3 class="text-sm font-medium text-zinc-400 mb-3">"Quick Dice Roller"</h3>

            // Quick dice buttons
            <div class="flex flex-wrap gap-2 mb-4">
                {quick_dice.into_iter().map(|die| {
                    let die_str = die.to_string();
                    let die_str_for_class = die_str.clone();
                    let die_str_for_click = die_str.clone();
                    view! {
                        <button
                            class=move || format!(
                                "px-3 py-1.5 rounded-lg text-sm font-medium transition-colors {}",
                                if notation.get() == die_str_for_class {
                                    "bg-purple-600 text-white"
                                } else {
                                    "bg-zinc-800 text-zinc-300 hover:bg-zinc-700"
                                }
                            )
                            on:click=move |_| notation.set(die_str_for_click.clone())
                        >
                            {die}
                        </button>
                    }
                }).collect::<Vec<_>>()}
            </div>

            // Custom notation input
            <div class="flex gap-2 mb-4">
                <input
                    type="text"
                    class="flex-1 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white placeholder-zinc-500 focus:outline-none focus:border-purple-500"
                    placeholder="e.g., 2d6+3"
                    prop:value=move || notation.get()
                    on:input=move |ev| {
                        let value = event_target_value(&ev);
                        notation.set(value);
                    }
                />
                <button
                    class=move || format!(
                        "px-6 py-2 rounded-lg font-medium transition-all {}",
                        if is_rolling.get() {
                            "bg-purple-500 text-white animate-pulse"
                        } else {
                            "bg-purple-600 hover:bg-purple-500 text-white"
                        }
                    )
                    on:click=handle_roll
                    disabled=move || is_rolling.get()
                >
                    "Roll"
                </button>
            </div>

            // Result display
            <Show when=move || last_roll.get().is_some()>
                <div class="text-center py-4 bg-zinc-800/50 rounded-lg">
                    <p class="text-sm text-zinc-400 mb-1">{move || notation.get()}</p>
                    <p class="text-4xl font-bold text-purple-400">
                        {move || last_roll.get().map(|r| r.to_string()).unwrap_or_default()}
                    </p>
                </div>
            </Show>
        </div>
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse max roll from dice notation (handles compound dice like "2d6+3")
fn parse_max_roll(notation: &str) -> i32 {
    let notation = notation.to_lowercase();

    // Handle special notations
    if notation.contains("d100") || notation.contains("d%") {
        return 100;
    } else if notation.contains("d66") {
        return 66;
    }

    // Parse standard notation: [count]d<sides>[+/-modifier]
    if let Some(pos) = notation.find('d') {
        // Parse dice count (default 1 if not specified)
        let count_str: String = notation[..pos].chars().filter(|c| c.is_ascii_digit()).collect();
        let count: i32 = count_str.parse().unwrap_or(1).max(1);

        // Parse sides
        let rest = &notation[pos + 1..];
        let sides_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        let sides: i32 = sides_str.parse().unwrap_or(20);

        // Parse modifier (+ or -)
        let modifier = if let Some(plus_pos) = rest.find('+') {
            let mod_str: String = rest[plus_pos + 1..].chars().take_while(|c| c.is_ascii_digit()).collect();
            mod_str.parse::<i32>().unwrap_or(0)
        } else if let Some(minus_pos) = rest.find('-') {
            let mod_str: String = rest[minus_pos + 1..].chars().take_while(|c| c.is_ascii_digit()).collect();
            -mod_str.parse::<i32>().unwrap_or(0)
        } else {
            0
        };

        // Max roll = count * sides + modifier
        count * sides + modifier
    } else {
        20
    }
}

/// Parse minimum roll from dice notation (handles compound dice like "2d6+3")
fn parse_min_roll(notation: &str) -> i32 {
    let notation = notation.to_lowercase();

    // Handle special notations
    if notation.contains("d100") || notation.contains("d%") {
        return 1;
    } else if notation.contains("d66") {
        return 11; // d66 minimum is 11
    }

    // Parse standard notation: [count]d<sides>[+/-modifier]
    if let Some(pos) = notation.find('d') {
        // Parse dice count (default 1 if not specified)
        let count_str: String = notation[..pos].chars().filter(|c| c.is_ascii_digit()).collect();
        let count: i32 = count_str.parse().unwrap_or(1).max(1);

        // Parse modifier (+ or -)
        let rest = &notation[pos + 1..];
        let modifier = if let Some(plus_pos) = rest.find('+') {
            let mod_str: String = rest[plus_pos + 1..].chars().take_while(|c| c.is_ascii_digit()).collect();
            mod_str.parse::<i32>().unwrap_or(0)
        } else if let Some(minus_pos) = rest.find('-') {
            let mod_str: String = rest[minus_pos + 1..].chars().take_while(|c| c.is_ascii_digit()).collect();
            -mod_str.parse::<i32>().unwrap_or(0)
        } else {
            0
        };

        // Min roll = count * 1 + modifier
        count + modifier
    } else {
        1
    }
}

/// Simulate a roll result (placeholder for backend call)
fn simulate_roll(table: &RandomTable) -> TableRollResult {
    let min = parse_min_roll(&table.dice_notation);
    let max = parse_max_roll(&table.dice_notation);
    let roll = simulate_dice_roll(min, max);

    let entry = table.entries.iter()
        .find(|e| roll >= e.range_start && roll <= e.range_end)
        .cloned()
        .unwrap_or_else(|| TableEntry {
            id: "unknown".to_string(),
            range_start: roll,
            range_end: roll,
            result_text: "Unknown result".to_string(),
            weight: 1.0,
            nested_table_id: None,
        });

    TableRollResult {
        table_id: table.id.clone(),
        table_name: table.name.clone(),
        roll_total: roll,
        entry: entry.clone(),
        final_text: entry.result_text,
    }
}

/// Simulate a dice roll in the range [min, max] inclusive
fn simulate_dice_roll(min: i32, max: i32) -> i32 {
    // Normalize range to prevent issues when min > max
    let (lo, hi) = if min <= max { (min, max) } else { (max, min) };
    let range = (hi - lo + 1) as f64;
    // Use js_sys::Math::random() for WASM-compatible random numbers
    let random = js_sys::Math::random();
    (random * range).floor() as i32 + lo
}

/// Format timestamp for display
fn format_timestamp(timestamp: &str) -> String {
    // Simplified timestamp formatting
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(dt);

        if duration.num_minutes() < 1 {
            "just now".to_string()
        } else if duration.num_minutes() < 60 {
            format!("{} min ago", duration.num_minutes())
        } else if duration.num_hours() < 24 {
            format!("{} hours ago", duration.num_hours())
        } else {
            format!("{} days ago", duration.num_days())
        }
    } else {
        timestamp.to_string()
    }
}

/// Get event target value helper
fn event_target_value(ev: &leptos::ev::Event) -> String {
    use wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default()
}

/// Set timeout helper
fn set_timeout<F>(callback: F, duration: std::time::Duration)
where
    F: FnOnce() + 'static,
{
    #[cfg(target_arch = "wasm32")]
    {
        use gloo_timers::callback::Timeout;
        let timeout = Timeout::new(duration.as_millis() as u32, callback);
        timeout.forget();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // For non-wasm targets, just execute immediately
        // (This is mainly for testing - in practice this is always wasm)
        let _ = duration;
        callback();
    }
}
