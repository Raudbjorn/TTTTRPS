//! Card Tray Component
//!
//! Phase 9 of the Campaign Generation Overhaul.
//!
//! Displays pinned quick reference cards in a horizontal row/grid.
//! Features:
//! - Maximum 6 card slots with visual indicators
//! - Drag-and-drop reordering (future enhancement)
//! - Inline expansion for quick access
//! - Integration with session control panel
//!
//! Design principles:
//! - Compact, always-visible reference area
//! - Quick access to session-critical information
//! - Non-intrusive when collapsed

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use super::entity_card::{EntityCardCompact, EntityCardCompactProps, PinnedCard};

// ============================================================================
// Constants
// ============================================================================

/// Maximum number of pinned cards allowed
pub const MAX_PINNED_CARDS: usize = 6;

// ============================================================================
// Types
// ============================================================================

/// Card tray state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardTray {
    pub session_id: String,
    pub cards: Vec<PinnedCard>,
    pub max_cards: usize,
}

impl CardTray {
    /// Compute remaining slots on demand to avoid stale state
    pub fn slots_remaining(&self) -> usize {
        self.max_cards.saturating_sub(self.cards.len())
    }
}

impl Default for CardTray {
    fn default() -> Self {
        Self {
            session_id: String::new(),
            cards: Vec::new(),
            max_cards: MAX_PINNED_CARDS,
        }
    }
}

// ============================================================================
// Card Tray Component
// ============================================================================

/// Empty slot placeholder
#[component]
fn EmptySlot(slot_number: usize) -> impl IntoView {
    view! {
        <div
            class="w-36 h-24 bg-zinc-900/50 border border-dashed border-zinc-700
                   rounded-lg flex items-center justify-center"
            title=format!("Slot {}", slot_number + 1)
        >
            <div class="text-center">
                <svg class="w-6 h-6 mx-auto text-zinc-700" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
                </svg>
                <span class="text-xs text-zinc-600">"Pin a card"</span>
            </div>
        </div>
    }
}

/// Slot counter showing used/total
#[component]
fn SlotCounter(used: usize, total: usize) -> impl IntoView {
    let is_full = used >= total;

    view! {
        <div class=format!(
            "px-2 py-1 rounded text-xs font-medium {}",
            if is_full { "bg-amber-900/50 text-amber-400" } else { "bg-zinc-800 text-zinc-400" }
        )>
            {used}"/"{ total }
            <span class="ml-1 text-zinc-500">"pinned"</span>
        </div>
    }
}

/// Main card tray component
///
/// Displays a horizontal row of pinned quick reference cards.
#[component]
pub fn CardTrayPanel(
    /// Session ID for the tray (reserved for future use)
    #[prop(into)]
    _session_id: String,
    /// Card tray state (externally managed)
    tray: RwSignal<CardTray>,
    /// Callback when a card is clicked (to expand details)
    #[prop(optional)]
    on_card_click: Option<Callback<PinnedCard>>,
    /// Callback when a card is unpinned
    #[prop(optional)]
    on_unpin: Option<Callback<String>>,
    /// Callback when cards are reordered (reserved for future use)
    #[prop(optional)]
    _on_reorder: Option<Callback<Vec<String>>>,
    /// Whether the tray is collapsed
    #[prop(default = false)]
    collapsed: bool,
) -> impl IntoView {
    let is_collapsed = RwSignal::new(collapsed);

    // Toggle collapse handler
    let toggle_collapse = move |_| {
        is_collapsed.update(|c| *c = !*c);
    };

    view! {
        <div class="bg-zinc-900/80 backdrop-blur-sm border-t border-zinc-800">
            // Header bar (always visible)
            <div
                class="flex items-center justify-between px-4 py-2 cursor-pointer hover:bg-zinc-800/50"
                on:click=toggle_collapse
            >
                <div class="flex items-center gap-3">
                    // Expand/collapse icon
                    <svg
                        class=move || format!(
                            "w-4 h-4 text-zinc-400 transition-transform {}",
                            if is_collapsed.get() { "" } else { "rotate-180" }
                        )
                        fill="none" stroke="currentColor" viewBox="0 0 24 24"
                    >
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                            d="M5 15l7-7 7 7" />
                    </svg>

                    <h3 class="text-sm font-semibold text-zinc-300">"Quick Reference"</h3>
                </div>

                // Slot counter
                <SlotCounter
                    used=tray.get().cards.len()
                    total=MAX_PINNED_CARDS
                />
            </div>

            // Card grid (collapsible)
            <Show when=move || !is_collapsed.get()>
                <div class="px-4 pb-4">
                    <div class="flex gap-2 overflow-x-auto pb-2">
                        // Render pinned cards
                        {move || tray.get().cards.iter().map(|card| {
                            view! {
                                <div class="flex-shrink-0 relative group">
                                    {EntityCardCompact(EntityCardCompactProps {
                                        card: card.clone(),
                                        on_click: on_card_click,
                                        on_unpin: on_unpin,
                                    })}
                                </div>
                            }
                        }).collect_view()}

                        // Render empty slots
                        {move || {
                            let used = tray.get().cards.len();
                            (used..MAX_PINNED_CARDS).map(|i| view! {
                                <div class="flex-shrink-0">
                                    <EmptySlot slot_number=i />
                                </div>
                            }).collect_view()
                        }}
                    </div>

                    // Tray footer with actions
                    <div class="flex justify-end gap-2 mt-2">
                        // Clear all button
                        <Show when=move || !tray.get().cards.is_empty()>
                            <button
                                type="button"
                                class="px-2 py-1 text-xs text-zinc-500 hover:text-red-400 transition-colors"
                                on:click=move |_| {
                                    if let Some(cb) = &on_unpin {
                                        // Unpin all cards
                                        for card in tray.get().cards.iter() {
                                            cb.run(card.pin_id.clone());
                                        }
                                    }
                                }
                            >
                                "Clear All"
                            </button>
                        </Show>
                    </div>
                </div>
            </Show>
        </div>
    }
}

/// Floating card tray for overlay mode
#[component]
pub fn FloatingCardTray(
    /// Session ID (reserved for future use)
    #[prop(into)]
    _session_id: String,
    /// Card tray state
    tray: RwSignal<CardTray>,
    /// Position (bottom-left, bottom-right, etc.)
    #[prop(default = "bottom-right")]
    position: &'static str,
    /// Callback when a card is clicked
    #[prop(optional)]
    on_card_click: Option<Callback<PinnedCard>>,
    /// Callback when a card is unpinned
    #[prop(optional)]
    on_unpin: Option<Callback<String>>,
) -> impl IntoView {
    let is_expanded = RwSignal::new(false);

    let position_class = match position {
        "bottom-left" => "bottom-4 left-4",
        "bottom-right" => "bottom-4 right-4",
        "top-left" => "top-4 left-4",
        "top-right" => "top-4 right-4",
        _ => "bottom-4 right-4",
    };

    view! {
        <div class=format!(
            "fixed {} z-50 transition-all",
            position_class
        )>
            // Collapsed state: just show count badge
            <Show when=move || !is_expanded.get()>
                <button
                    type="button"
                    class="p-3 bg-zinc-900 border border-zinc-700 rounded-full shadow-lg
                           hover:bg-zinc-800 transition-colors"
                    on:click=move |_| is_expanded.set(true)
                    title="Quick Reference Cards"
                >
                    <div class="relative">
                        <svg class="w-6 h-6 text-zinc-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z" />
                        </svg>
                        // Count badge
                        {move || {
                            let count = tray.get().cards.len();
                            (count > 0).then(|| view! {
                                <span class="absolute -top-1 -right-1 w-4 h-4 bg-purple-600
                                             text-white text-xs rounded-full flex items-center justify-center">
                                    {count}
                                </span>
                            })
                        }}
                    </div>
                </button>
            </Show>

            // Expanded state: show card grid
            <Show when=move || is_expanded.get()>
                <div class="bg-zinc-900 border border-zinc-700 rounded-lg shadow-xl p-4 w-auto max-w-lg">
                    // Header
                    <div class="flex items-center justify-between mb-3">
                        <h3 class="text-sm font-semibold text-zinc-300">"Quick Reference"</h3>
                        <button
                            type="button"
                            class="p-1 text-zinc-500 hover:text-white"
                            on:click=move |_| is_expanded.set(false)
                        >
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                    d="M6 18L18 6M6 6l12 12" />
                            </svg>
                        </button>
                    </div>

                    // Cards grid
                    <div class="grid grid-cols-3 gap-2">
                        {move || tray.get().cards.iter().map(|card| {
                            EntityCardCompact(EntityCardCompactProps {
                                card: card.clone(),
                                on_click: on_card_click,
                                on_unpin: on_unpin,
                            })
                        }).collect_view()}

                        // Empty slots
                        {move || {
                            let used = tray.get().cards.len();
                            (used..MAX_PINNED_CARDS).map(|i| view! {
                                <EmptySlot slot_number=i />
                            }).collect_view()
                        }}
                    </div>

                    // Slot counter
                    <div class="flex justify-end mt-3">
                        <SlotCounter
                            used=tray.get().cards.len()
                            total=MAX_PINNED_CARDS
                        />
                    </div>
                </div>
            </Show>
        </div>
    }
}

/// Mini card tray for sidebar integration
#[component]
pub fn MiniCardTray(
    /// Card tray state
    tray: RwSignal<CardTray>,
    /// Callback when a card is clicked
    #[prop(optional)]
    on_card_click: Option<Callback<PinnedCard>>,
) -> impl IntoView {
    view! {
        <div class="space-y-2">
            <div class="flex items-center justify-between">
                <h4 class="text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                    "Pinned"
                </h4>
                <span class="text-xs text-zinc-500">
                    {move || tray.get().cards.len()}"/"{ MAX_PINNED_CARDS }
                </span>
            </div>

            // Compact card list
            <div class="space-y-1">
                {move || tray.get().cards.iter().map(|card| {
                    let card_for_click = card.clone();
                    let entity_type = card.entity_type;

                    view! {
                        <div
                            class=format!(
                                "flex items-center gap-2 p-2 rounded cursor-pointer
                                 hover:bg-zinc-800 transition-colors border-l-2 {}",
                                entity_type.color()
                            )
                            on:click=move |_| {
                                if let Some(cb) = &on_card_click {
                                    cb.run(card_for_click.clone());
                                }
                            }
                        >
                            <svg class=format!("w-3 h-3 flex-shrink-0 {}", entity_type.color().split_whitespace().next().unwrap_or(""))
                                fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                    d={entity_type.icon()} />
                            </svg>
                            <span class="text-sm text-zinc-300 truncate">
                                {card.rendered.title.clone()}
                            </span>
                        </div>
                    }
                }).collect_view()}

                // Empty state
                <Show when=move || tray.get().cards.is_empty()>
                    <p class="text-xs text-zinc-600 text-center py-2">
                        "No pinned cards"
                    </p>
                </Show>
            </div>
        </div>
    }
}
