//! Entity Card Component
//!
//! Phase 9 of the Campaign Generation Overhaul.
//!
//! Provides quick reference card display for campaign entities:
//! - NPCs, Locations, Items, Plot Points, Scenes, Characters
//! - Support for click-to-expand to full detail
//! - Hover preview for tooltips
//! - Compact, scannable display styling
//!
//! Design principles:
//! - Progressive disclosure (minimal/summary/complete)
//! - Visual hierarchy for at-a-glance information
//! - Consistent styling across entity types

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Types
// ============================================================================

/// Entity type for cards
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardEntityType {
    Npc,
    Location,
    Item,
    PlotPoint,
    Scene,
    Character,
}

impl CardEntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CardEntityType::Npc => "npc",
            CardEntityType::Location => "location",
            CardEntityType::Item => "item",
            CardEntityType::PlotPoint => "plot_point",
            CardEntityType::Scene => "scene",
            CardEntityType::Character => "character",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            CardEntityType::Npc => "M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z",
            CardEntityType::Location => "M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z M15 11a3 3 0 11-6 0 3 3 0 016 0z",
            CardEntityType::Item => "M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4",
            CardEntityType::PlotPoint => "M13 10V3L4 14h7v7l9-11h-7z",
            CardEntityType::Scene => "M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z",
            CardEntityType::Character => "M5.121 17.804A13.937 13.937 0 0112 16c2.5 0 4.847.655 6.879 1.804M15 10a3 3 0 11-6 0 3 3 0 016 0zm6 2a9 9 0 11-18 0 9 9 0 0118 0z",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            CardEntityType::Npc => "text-blue-400 border-blue-500/50",
            CardEntityType::Location => "text-green-400 border-green-500/50",
            CardEntityType::Item => "text-amber-400 border-amber-500/50",
            CardEntityType::PlotPoint => "text-purple-400 border-purple-500/50",
            CardEntityType::Scene => "text-pink-400 border-pink-500/50",
            CardEntityType::Character => "text-cyan-400 border-cyan-500/50",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            CardEntityType::Npc => "NPC",
            CardEntityType::Location => "Location",
            CardEntityType::Item => "Item",
            CardEntityType::PlotPoint => "Plot",
            CardEntityType::Scene => "Scene",
            CardEntityType::Character => "Character",
        }
    }
}

/// Disclosure level for progressive detail display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisclosureLevel {
    Minimal,
    Summary,
    Complete,
}

impl DisclosureLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            DisclosureLevel::Minimal => "minimal",
            DisclosureLevel::Summary => "summary",
            DisclosureLevel::Complete => "complete",
        }
    }
}

impl Default for DisclosureLevel {
    fn default() -> Self {
        DisclosureLevel::Summary
    }
}

/// Quick stat for card header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickStat {
    pub label: String,
    pub value: String,
    pub icon: Option<String>,
}

/// Rendered card data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedCard {
    pub entity_type: CardEntityType,
    pub entity_id: String,
    pub disclosure_level: DisclosureLevel,
    pub title: String,
    pub subtitle: Option<String>,
    pub html_content: String,
    pub text_content: String,
    pub is_pinned: bool,
    pub pin_id: Option<String>,
    pub quick_stats: Vec<QuickStat>,
    pub tags: Vec<String>,
}

/// Hover preview data (minimal card for tooltips)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverPreview {
    pub entity_type: CardEntityType,
    pub entity_id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub summary: String,
    pub quick_stats: Vec<QuickStat>,
}

/// Pinned card with rendered content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedCard {
    pub pin_id: String,
    pub entity_type: CardEntityType,
    pub entity_id: String,
    pub display_order: i32,
    pub disclosure_level: DisclosureLevel,
    pub rendered: RenderedCard,
    pub pinned_at: String,
}

// ============================================================================
// Card Components
// ============================================================================

/// Entity card header with type icon and title
#[component]
fn CardHeader(
    entity_type: CardEntityType,
    title: String,
    subtitle: Option<String>,
    quick_stats: Vec<QuickStat>,
    is_pinned: bool,
) -> impl IntoView {
    let color_class = entity_type.color();
    let colon = ":";
    let has_quick_stats = !quick_stats.is_empty();

    view! {
        <div class="flex items-start justify-between mb-2">
            <div class="flex items-start gap-2">
                // Type icon
                <div class=format!(
                    "w-8 h-8 rounded-lg flex items-center justify-center bg-zinc-800 border {}",
                    color_class
                )>
                    <svg class=format!("w-4 h-4 {}", color_class.split_whitespace().next().unwrap_or(""))
                        fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                            d={entity_type.icon()} />
                    </svg>
                </div>

                // Title and subtitle
                <div>
                    <h3 class="font-semibold text-white leading-tight">{title}</h3>
                    {subtitle.map(|s| view! {
                        <p class="text-xs text-zinc-400">{s}</p>
                    })}
                </div>
            </div>

            // Pin indicator
            {is_pinned.then(|| view! {
                <div class="p-1 text-amber-400" title="Pinned">
                    <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                        <path d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z" />
                    </svg>
                </div>
            })}
        </div>

        // Quick stats row
        <Show when=move || has_quick_stats>
            <div class="flex flex-wrap gap-2 mb-2">
                {quick_stats.iter().map(|stat| view! {
                    <div class="px-2 py-0.5 bg-zinc-800 rounded-full text-xs flex items-center gap-1">
                        {stat.icon.as_ref().map(|_| view! {
                            <span class="text-zinc-400">{stat.label.clone()}{colon}</span>
                        })}
                        <span class="text-zinc-300">{stat.value.clone()}</span>
                    </div>
                }).collect_view()}
            </div>
        </Show>
    }
}

/// Entity card body content
#[component]
fn CardBody(html_content: String, disclosure_level: DisclosureLevel) -> impl IntoView {
    let max_height = match disclosure_level {
        DisclosureLevel::Minimal => "max-h-12",
        DisclosureLevel::Summary => "max-h-32",
        DisclosureLevel::Complete => "max-h-64",
    };

    view! {
        <div
            class=format!("{} overflow-hidden text-sm text-zinc-300", max_height)
            inner_html=html_content
        />
    }
}

/// Tag display for card footer
#[component]
fn CardTags(tags: Vec<String>) -> impl IntoView {
    let has_tags = !tags.is_empty();

    view! {
        <Show when=move || has_tags>
            <div class="flex flex-wrap gap-1 mt-2 pt-2 border-t border-zinc-800">
                {tags.iter().map(|tag| view! {
                    <span class="px-1.5 py-0.5 bg-zinc-800 rounded text-xs text-zinc-500">
                        {"#"}{tag.clone()}
                    </span>
                }).collect_view()}
            </div>
        </Show>
    }
}

/// Complete entity card component
///
/// Displays an entity card with progressive disclosure levels.
#[component]
pub fn EntityCard(
    /// Rendered card data
    card: RenderedCard,
    /// Whether the card is expanded to show full details
    #[prop(default = false)]
    expanded: bool,
    /// Callback when card is clicked
    #[prop(optional)]
    on_click: Option<Callback<RenderedCard>>,
    /// Callback for pin toggle
    #[prop(optional)]
    on_pin_toggle: Option<Callback<(CardEntityType, String)>>,
    /// Show in compact mode (for tray)
    #[prop(default = false)]
    compact: bool,
) -> impl IntoView {
    let card_ref = card.clone();
    let entity_type = card.entity_type;
    let entity_id = card.entity_id.clone();
    let is_pinned = card.is_pinned;
    let tags = card.tags.clone();

    let base_class = if compact { "w-40 p-2" } else { "w-full p-3" };

    let disclosure = if expanded {
        DisclosureLevel::Complete
    } else {
        card.disclosure_level
    };

    view! {
        <div
            class=format!(
                "{} bg-zinc-900 border rounded-lg transition-all hover:border-zinc-600 cursor-pointer {}",
                base_class,
                entity_type.color()
            )
            on:click=move |_| {
                if let Some(cb) = &on_click {
                    cb.run(card_ref.clone());
                }
            }
        >
            <CardHeader
                entity_type=entity_type
                title=card.title.clone()
                subtitle=card.subtitle.clone()
                quick_stats=card.quick_stats.clone()
                is_pinned=is_pinned
            />

            <CardBody
                html_content=card.html_content.clone()
                disclosure_level=disclosure
            />

            <Show when=move || !compact>
                <CardTags tags=tags.clone() />

                // Action buttons
                <div class="flex justify-end gap-2 mt-2 pt-2 border-t border-zinc-800">
                    // Pin/Unpin button
                    {on_pin_toggle.map(|cb| {
                        let entity_id_for_cb = entity_id.clone();
                        view! {
                            <button
                                type="button"
                                class=format!(
                                    "p-1.5 rounded hover:bg-zinc-800 transition-colors {}",
                                    if is_pinned { "text-amber-400" } else { "text-zinc-500" }
                                )
                                title={if is_pinned { "Unpin" } else { "Pin to tray" }}
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    cb.run((entity_type, entity_id_for_cb.clone()));
                                }
                            >
                                <svg class="w-4 h-4" fill=if is_pinned { "currentColor" } else { "none" }
                                    stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                        d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z" />
                                </svg>
                            </button>
                        }
                    })}
                </div>
            </Show>
        </div>
    }
}

/// Compact entity card for tray display
#[component]
pub fn EntityCardCompact(
    /// Pinned card data
    card: PinnedCard,
    /// Callback when card is clicked
    #[prop(optional)]
    on_click: Option<Callback<PinnedCard>>,
    /// Callback for unpin
    #[prop(optional)]
    on_unpin: Option<Callback<String>>,
) -> impl IntoView {
    let card_click = card.clone();
    let pin_id = card.pin_id.clone();
    let entity_type = card.entity_type;

    view! {
        <div
            class=format!(
                "w-36 p-2 bg-zinc-900 border rounded-lg transition-all hover:border-zinc-600
                 cursor-pointer flex flex-col relative group {}",
                entity_type.color()
            )
            on:click=move |_| {
                if let Some(cb) = &on_click {
                    cb.run(card_click.clone());
                }
            }
        >
            // Header
            <div class="flex items-center gap-1.5 mb-1">
                <svg class=format!("w-3 h-3 {}", entity_type.color().split_whitespace().next().unwrap_or(""))
                    fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d={entity_type.icon()} />
                </svg>
                <span class="text-xs text-zinc-500">{entity_type.label()}</span>
            </div>

            // Title
            <h4 class="font-medium text-white text-sm truncate " title={card.rendered.title.clone()}>
                {card.rendered.title.clone()}
            </h4>

            // Subtitle
            {card.rendered.subtitle.clone().map(|s| view! {
                <p class="text-xs text-zinc-400 truncate ">{s}</p>
            })}

            // Unpin button
            {on_unpin.map(|cb| view! {
                <button
                    type="button"
                    class="absolute top-1 right-1 p-0.5 text-zinc-500 hover:text-white
                           opacity-0 group-hover:opacity-100 transition-opacity "
                    title="Unpin"
                    on:click=move |ev| {
                        ev.stop_propagation();
                        cb.run(pin_id.clone());
                    }
                >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                            d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>
            })}
        </div>
    }
}

/// Hover preview tooltip component
#[component]
pub fn EntityHoverPreview(
    /// Preview data
    preview: HoverPreview,
) -> impl IntoView {
    let entity_type = preview.entity_type;
    let stat_badge_class = "px-1.5 py-0.5 bg-zinc-800 rounded text-xs text-zinc-400";
    let colon_sep = ": ";
    let has_quick_stats = !preview.quick_stats.is_empty();
    let quick_stats = preview.quick_stats.clone();

    view! {
        <div class=format!(
            "p-3 bg-zinc-900 border rounded-lg shadow-xl max-w-xs {}",
            entity_type.color()
        )>
            // Header
            <div class="flex items-center gap-2 mb-2">
                <svg class=format!("w-4 h-4 {}", entity_type.color().split_whitespace().next().unwrap_or(""))
                    fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d={entity_type.icon()} />
                </svg>
                <div>
                    <h4 class="font-semibold text-white text-sm ">{preview.title}</h4>
                    {preview.subtitle.map(|s| view! {
                        <p class="text-xs text-zinc-400">{s}</p>
                    })}
                </div>
            </div>

            // Summary
            <p class="text-xs text-zinc-300 mb-2">{preview.summary}</p>

            // Quick stats
            <Show when=move || has_quick_stats>
                <div class="flex flex-wrap gap-1">
                    {quick_stats.iter().map(|stat| {
                        let label = stat.label.clone();
                        let value = stat.value.clone();
                        view! {
                            <span class=stat_badge_class>
                                {label}{colon_sep}{value}
                            </span>
                        }
                    }).collect_view()}
                </div>
            </Show>
        </div>
    }
}

// ============================================================================
// Entity Type Specific Variants
// ============================================================================

/// NPC card variant with role-based styling
#[component]
pub fn NpcCard(
    /// NPC card data
    card: RenderedCard,
    /// Callback when clicked
    #[prop(optional)]
    on_click: Option<Callback<RenderedCard>>,
    /// Callback for pin toggle
    #[prop(optional)]
    on_pin_toggle: Option<Callback<(CardEntityType, String)>>,
) -> impl IntoView {
    // Simply render with EntityCard - callbacks are passed through
    EntityCard(EntityCardProps {
        card,
        expanded: false,
        on_click,
        on_pin_toggle,
        compact: false,
    })
}

/// Location card variant with map styling
#[component]
pub fn LocationCard(
    /// Location card data
    card: RenderedCard,
    /// Callback when clicked
    #[prop(optional)]
    on_click: Option<Callback<RenderedCard>>,
    /// Callback for pin toggle
    #[prop(optional)]
    on_pin_toggle: Option<Callback<(CardEntityType, String)>>,
) -> impl IntoView {
    EntityCard(EntityCardProps {
        card,
        expanded: false,
        on_click,
        on_pin_toggle,
        compact: false,
    })
}

/// Item card variant with rarity styling
#[component]
pub fn ItemCard(
    /// Item card data
    card: RenderedCard,
    /// Callback when clicked
    #[prop(optional)]
    on_click: Option<Callback<RenderedCard>>,
    /// Callback for pin toggle
    #[prop(optional)]
    on_pin_toggle: Option<Callback<(CardEntityType, String)>>,
) -> impl IntoView {
    EntityCard(EntityCardProps {
        card,
        expanded: false,
        on_click,
        on_pin_toggle,
        compact: false,
    })
}

/// Plot point card variant with status indicators
#[component]
pub fn PlotCard(
    /// Plot card data
    card: RenderedCard,
    /// Callback when clicked
    #[prop(optional)]
    on_click: Option<Callback<RenderedCard>>,
    /// Callback for pin toggle
    #[prop(optional)]
    on_pin_toggle: Option<Callback<(CardEntityType, String)>>,
) -> impl IntoView {
    EntityCard(EntityCardProps {
        card,
        expanded: false,
        on_click,
        on_pin_toggle,
        compact: false,
    })
}
