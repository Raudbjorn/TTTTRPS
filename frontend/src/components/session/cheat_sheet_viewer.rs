//! Cheat Sheet Viewer Component
//!
//! Phase 9 of the Campaign Generation Overhaul.
//!
//! Displays session cheat sheets with:
//! - Collapsible section headers
//! - Truncation warnings
//! - Print-friendly mode toggle
//! - Floating panel mode for overlay display
//!
//! Design principles:
//! - Organized, scannable layout
//! - Quick access to session-critical information
//! - Optimized for both screen and print

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use super::entity_card::CardEntityType;

// ============================================================================
// Types
// ============================================================================

/// Section type for cheat sheet organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionType {
    KeyNpcs,
    Locations,
    PlotPoints,
    Objectives,
    Encounters,
    Scenes,
    Rules,
    PartyReminders,
    Custom,
}

impl SectionType {
    pub fn display_name(&self) -> &'static str {
        match self {
            SectionType::KeyNpcs => "Key NPCs",
            SectionType::Locations => "Locations",
            SectionType::PlotPoints => "Plot Points",
            SectionType::Objectives => "Objectives",
            SectionType::Encounters => "Encounters",
            SectionType::Scenes => "Scenes",
            SectionType::Rules => "Rules Reference",
            SectionType::PartyReminders => "Party Reminders",
            SectionType::Custom => "Notes",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SectionType::KeyNpcs => "M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z",
            SectionType::Locations => "M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z M15 11a3 3 0 11-6 0 3 3 0 016 0z",
            SectionType::PlotPoints => "M13 10V3L4 14h7v7l9-11h-7z",
            SectionType::Objectives => "M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z",
            SectionType::Encounters => "M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z",
            SectionType::Scenes => "M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z",
            SectionType::Rules => "M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253",
            SectionType::PartyReminders => "M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z",
            SectionType::Custom => "M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            SectionType::KeyNpcs => "text-blue-400",
            SectionType::Locations => "text-green-400",
            SectionType::PlotPoints => "text-purple-400",
            SectionType::Objectives => "text-amber-400",
            SectionType::Encounters => "text-red-400",
            SectionType::Scenes => "text-pink-400",
            SectionType::Rules => "text-cyan-400",
            SectionType::PartyReminders => "text-emerald-400",
            SectionType::Custom => "text-zinc-400",
        }
    }
}

/// A single item in a cheat sheet section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatSheetItem {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub entity_type: Option<CardEntityType>,
    pub entity_id: Option<String>,
    pub priority: i32,
    pub was_truncated: bool,
    pub original_chars: usize,
}

/// A section of the cheat sheet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatSheetSection {
    pub section_type: SectionType,
    pub title: String,
    pub items: Vec<CheatSheetItem>,
    pub priority: i32,
    pub was_truncated: bool,
    pub hidden_items: usize,
    pub collapsed: bool,
}

/// Truncation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruncationWarning {
    pub section: SectionType,
    pub chars_removed: usize,
    pub items_hidden: usize,
    pub reason: String,
}

/// Complete cheat sheet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatSheet {
    pub campaign_id: String,
    pub session_id: Option<String>,
    pub title: String,
    pub sections: Vec<CheatSheetSection>,
    pub total_chars: usize,
    pub max_chars: usize,
    pub warnings: Vec<TruncationWarning>,
    pub generated_at: String,
}

// ============================================================================
// Section Components
// ============================================================================

/// Truncation warning banner
#[component]
fn TruncationBanner(
    warnings: Vec<TruncationWarning>,
) -> impl IntoView {
    let has_warnings = !warnings.is_empty();
    let colon_space = ": ";
    let items_hidden_text = " items hidden, ";
    let chars_removed_text = " chars removed";

    view! {
        <Show when=move || has_warnings>
            <div class="mb-4 p-3 bg-amber-900/30 border border-amber-700/50 rounded-lg">
                <div class="flex items-start gap-2">
                    <svg class="w-5 h-5 text-amber-400 flex-shrink-0 mt-0.5"
                        fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                            d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                    </svg>
                    <div>
                        <h4 class="text-sm font-semibold text-amber-300">
                            "Some content was truncated"
                        </h4>
                        <ul class="mt-1 text-xs text-amber-200/70 space-y-0.5">
                            {warnings.iter().map(|w| view! {
                                <li>
                                    {w.section.display_name()}{colon_space}
                                    {w.items_hidden}{items_hidden_text}
                                    {w.chars_removed}{chars_removed_text}
                                </li>
                            }).collect_view()}
                        </ul>
                    </div>
                </div>
            </div>
        </Show>
    }
}

/// Section header with collapse toggle
#[component]
fn SectionHeader(
    section_type: SectionType,
    title: String,
    item_count: usize,
    hidden_count: usize,
    is_collapsed: Signal<bool>,
    on_toggle: Callback<()>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class="w-full flex items-center justify-between py-2 px-3 bg-zinc-800 rounded-t-lg
                   hover:bg-zinc-700 transition-colors group"
            on:click=move |_| on_toggle.run(())
        >
            <div class="flex items-center gap-2">
                <svg class=format!("w-4 h-4 {}", section_type.color())
                    fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d={section_type.icon()} />
                </svg>
                <span class="font-semibold text-white">{title}</span>
                <span class="text-xs text-zinc-500">
                    "("{ item_count }
                    {(hidden_count > 0).then(|| view! {
                        <span class="text-amber-500">" +"{hidden_count}" hidden"</span>
                    })}
                    ")"
                </span>
            </div>

            <svg
                class=format!(
                    "w-4 h-4 text-zinc-400 transition-transform {}",
                    if is_collapsed.get() { "" } else { "rotate-180" }
                )
                fill="none" stroke="currentColor" viewBox="0 0 24 24"
            >
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                    d="M19 9l-7 7-7-7" />
            </svg>
        </button>
    }
}

/// Section item display
#[component]
fn SectionItem(
    item: CheatSheetItem,
    #[prop(optional)]
    on_click: Option<Callback<CheatSheetItem>>,
) -> impl IntoView {
    let item_for_click = item.clone();

    view! {
        <div
            class=format!(
                "p-2 border-l-2 {} hover:bg-zinc-800/50 transition-colors {}",
                item.entity_type.map(|t| CardEntityType::color(&t)).unwrap_or("border-zinc-700"),
                if on_click.is_some() { "cursor-pointer" } else { "" }
            )
            on:click=move |_| {
                if let Some(cb) = &on_click {
                    cb.run(item_for_click.clone());
                }
            }
        >
            <div class="flex items-start justify-between">
                <h4 class=format!(
                    "font-medium text-white {}",
                    if item.was_truncated { "text-zinc-400" } else { "" }
                )>
                    {item.title.clone()}
                </h4>
                {item.was_truncated.then(|| view! {
                    <span class="text-xs text-amber-500" title="Content truncated">
                        "..."
                    </span>
                })}
            </div>
            {(!item.summary.is_empty()).then(|| view! {
                <p class="text-xs text-zinc-400 mt-0.5">{item.summary.clone()}</p>
            })}
            <p class="text-sm text-zinc-300 mt-1">{item.content.clone()}</p>
        </div>
    }
}

/// Complete section display
#[component]
fn CheatSheetSectionDisplay(
    section: CheatSheetSection,
    #[prop(optional)]
    on_item_click: Option<Callback<CheatSheetItem>>,
) -> impl IntoView {
    let is_collapsed = RwSignal::new(section.collapsed);

    let toggle = Callback::new(move |_: ()| {
        is_collapsed.update(|c| *c = !*c);
    });

    view! {
        <div class="border border-zinc-800 rounded-lg overflow-hidden">
            <SectionHeader
                section_type=section.section_type
                title=section.title.clone()
                item_count=section.items.len()
                hidden_count=section.hidden_items
                is_collapsed=Signal::derive(move || is_collapsed.get())
                on_toggle=toggle
            />

            <Show when=move || !is_collapsed.get()>
                <div class="divide-y divide-zinc-800/50">
                    {section.items.iter().map(|item| {
                        let item_clone = item.clone();
                        match on_item_click {
                            Some(cb) => view! {
                                <SectionItem
                                    item=item_clone
                                    on_click=cb
                                />
                            }.into_any(),
                            None => view! {
                                <SectionItem item=item_clone />
                            }.into_any(),
                        }
                    }).collect_view()}
                </div>
            </Show>
        </div>
    }
}

// ============================================================================
// Main Cheat Sheet Viewer
// ============================================================================

/// Main cheat sheet viewer component
#[component]
pub fn CheatSheetViewer(
    /// Cheat sheet data
    cheat_sheet: RwSignal<Option<CheatSheet>>,
    /// Whether in print-friendly mode
    #[prop(default = false)]
    print_mode: bool,
    /// Callback when an item is clicked
    #[prop(optional)]
    on_item_click: Option<Callback<CheatSheetItem>>,
    /// Callback to export as HTML
    #[prop(optional)]
    on_export: Option<Callback<()>>,
) -> impl IntoView {
    let is_print_mode = RwSignal::new(print_mode);

    view! {
        <div class=format!(
            "h-full flex flex-col {}",
            if is_print_mode.get() { "bg-white text-black" } else { "" }
        )>
            // Header
            <Show when=move || cheat_sheet.get().is_some()>
                {move || cheat_sheet.get().map(|sheet| view! {
                    <div class="flex items-center justify-between p-4 border-b border-zinc-800">
                        <div>
                            <h2 class="text-lg font-bold text-white">{sheet.title.clone()}</h2>
                            <p class="text-xs text-zinc-500">
                                "Generated: "{sheet.generated_at.clone()}" | "
                                {sheet.total_chars}" chars"
                            </p>
                        </div>

                        <div class="flex items-center gap-2">
                            // Print mode toggle
                            <button
                                type="button"
                                class=format!(
                                    "p-2 rounded transition-colors {}",
                                    if is_print_mode.get() { "bg-purple-600 text-white" } else { "bg-zinc-800 text-zinc-400 hover:text-white" }
                                )
                                title="Print Mode"
                                on:click=move |_| is_print_mode.update(|m| *m = !*m)
                            >
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                        d="M17 17h2a2 2 0 002-2v-4a2 2 0 00-2-2H5a2 2 0 00-2 2v4a2 2 0 002 2h2m2 4h6a2 2 0 002-2v-4a2 2 0 00-2-2H9a2 2 0 00-2 2v4a2 2 0 002 2zm8-12V5a2 2 0 00-2-2H9a2 2 0 00-2 2v4h10z" />
                                </svg>
                            </button>

                            // Export button
                            {on_export.map(|cb| view! {
                                <button
                                    type="button"
                                    class="p-2 bg-zinc-800 text-zinc-400 hover:text-white rounded transition-colors"
                                    title="Export as HTML"
                                    on:click=move |_| cb.run(())
                                >
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                            d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                                    </svg>
                                </button>
                            })}
                        </div>
                    </div>
                })}
            </Show>

            // Content
            <div class="flex-1 overflow-y-auto p-4">
                <Show
                    when=move || cheat_sheet.get().is_some()
                    fallback=|| view! {
                        <div class="flex items-center justify-center h-full text-zinc-500">
                            <div class="text-center">
                                <svg class="w-12 h-12 mx-auto text-zinc-600 mb-3"
                                    fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                        d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                                </svg>
                                <p class="text-sm">"No cheat sheet loaded"</p>
                                <p class="text-xs text-zinc-600 mt-1">
                                    "Generate a cheat sheet for your session"
                                </p>
                            </div>
                        </div>
                    }
                >
                    {move || cheat_sheet.get().map(|sheet| view! {
                        <div class="space-y-4">
                            // Truncation warnings
                            <TruncationBanner warnings=sheet.warnings.clone() />

                            // Sections
                            {sheet.sections.iter().map(|section| {
                                let section_clone = section.clone();
                                match on_item_click {
                                    Some(cb) => view! {
                                        <CheatSheetSectionDisplay
                                            section=section_clone
                                            on_item_click=cb
                                        />
                                    }.into_any(),
                                    None => view! {
                                        <CheatSheetSectionDisplay section=section_clone />
                                    }.into_any(),
                                }
                            }).collect_view()}

                            // Empty state
                            <Show when=move || sheet.sections.is_empty()>
                                <div class="text-center py-8 text-zinc-500">
                                    <p>"No content in this cheat sheet"</p>
                                </div>
                            </Show>
                        </div>
                    })}
                </Show>
            </div>
        </div>
    }
}

/// Floating cheat sheet panel
#[component]
pub fn FloatingCheatSheet(
    /// Cheat sheet data
    cheat_sheet: RwSignal<Option<CheatSheet>>,
    /// Whether the panel is visible
    is_visible: RwSignal<bool>,
    /// Position
    #[prop(default = "right")]
    position: &'static str,
    /// Callback when an item is clicked
    #[prop(optional)]
    on_item_click: Option<Callback<CheatSheetItem>>,
) -> impl IntoView {
    let position_class = match position {
        "left" => "left-0 top-0 bottom-0",
        "right" => "right-0 top-0 bottom-0",
        _ => "right-0 top-0 bottom-0",
    };

    view! {
        <Show when=move || is_visible.get()>
            // Backdrop
            <div
                class="fixed inset-0 bg-black/50 z-40"
                on:click=move |_| is_visible.set(false)
            />

            // Panel
            <div class=format!(
                "fixed {} w-96 bg-zinc-900 border-l border-zinc-800 shadow-2xl z-50
                 transform transition-transform",
                position_class
            )>
                // Close button
                <button
                    type="button"
                    class="absolute top-4 right-4 p-2 text-zinc-500 hover:text-white z-10"
                    on:click=move |_| is_visible.set(false)
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                            d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>

                // Viewer
                {match on_item_click {
                    Some(cb) => view! {
                        <CheatSheetViewer
                            cheat_sheet=cheat_sheet
                            on_item_click=cb
                        />
                    }.into_any(),
                    None => view! {
                        <CheatSheetViewer cheat_sheet=cheat_sheet />
                    }.into_any(),
                }}
            </div>
        </Show>
    }
}
