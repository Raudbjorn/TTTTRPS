//! Campaign Card Component
//!
//! Spotify-style album cover campaign card with visual flair.
//! Design metaphor: Campaigns as "Albums" with cover art, genre (system),
//! player count, last played date, and "Now Playing" pulse animation.

use leptos::ev;
use leptos::prelude::*;
use crate::bindings::Campaign;
use phosphor_leptos::{Icon, TRASH, DISC, USERS, USER, PLAY_CIRCLE};

/// Genre/system category for styling
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CampaignGenre {
    Fantasy,
    Horror,
    Cyberpunk,
    SciFi,
    Modern,
    Historical,
    Unknown,
}

impl CampaignGenre {
    /// Detect genre from system string
    pub fn from_system(system: &str) -> Self {
        let s = system.to_lowercase();
        if s.contains("d&d") || s.contains("5e") || s.contains("pathfinder")
            || s.contains("fantasy") || s.contains("warhammer fantasy")
        {
            CampaignGenre::Fantasy
        } else if s.contains("cthulhu") || s.contains("horror") || s.contains("vampire")
            || s.contains("kult") || s.contains("vaesen") || s.contains("delta green")
        {
            CampaignGenre::Horror
        } else if s.contains("cyber") || s.contains("shadow") || s.contains("neon")
            || s.contains("sprawl")
        {
            CampaignGenre::Cyberpunk
        } else if s.contains("space") || s.contains("alien") || s.contains("scifi")
            || s.contains("traveller") || s.contains("mothership") || s.contains("stars without")
        {
            CampaignGenre::SciFi
        } else if s.contains("modern") || s.contains("fate") || s.contains("gurps") {
            CampaignGenre::Modern
        } else if s.contains("historical") || s.contains("pendragon") || s.contains("ars magica") {
            CampaignGenre::Historical
        } else {
            CampaignGenre::Unknown
        }
    }

    /// Get gradient and text styling for this genre
    pub fn style(&self) -> (&'static str, &'static str) {
        match self {
            CampaignGenre::Fantasy => (
                "bg-gradient-to-br from-amber-600 via-amber-700 to-amber-900",
                "text-amber-200",
            ),
            CampaignGenre::Horror => (
                "bg-gradient-to-br from-slate-800 via-red-950 to-black",
                "text-red-400",
            ),
            CampaignGenre::Cyberpunk => (
                "bg-gradient-to-br from-fuchsia-800 via-purple-900 to-indigo-950",
                "text-fuchsia-300",
            ),
            CampaignGenre::SciFi => (
                "bg-gradient-to-br from-cyan-700 via-blue-800 to-blue-950",
                "text-cyan-200",
            ),
            CampaignGenre::Modern => (
                "bg-gradient-to-br from-slate-600 via-slate-700 to-slate-900",
                "text-slate-200",
            ),
            CampaignGenre::Historical => (
                "bg-gradient-to-br from-stone-600 via-stone-700 to-stone-900",
                "text-stone-200",
            ),
            CampaignGenre::Unknown => (
                "bg-gradient-to-br from-zinc-600 via-zinc-700 to-zinc-900",
                "text-zinc-300",
            ),
        }
    }

    /// Get genre label for display
    pub fn label(&self) -> &'static str {
        match self {
            CampaignGenre::Fantasy => "Fantasy",
            CampaignGenre::Horror => "Horror",
            CampaignGenre::Cyberpunk => "Cyberpunk",
            CampaignGenre::SciFi => "Sci-Fi",
            CampaignGenre::Modern => "Modern",
            CampaignGenre::Historical => "Historical",
            CampaignGenre::Unknown => "RPG",
        }
    }
}

/// Helper function to get system-based styling
#[allow(dead_code)]
fn get_system_style(system: &str) -> (&'static str, &'static str) {
    CampaignGenre::from_system(system).style()
}

/// Format a timestamp to a human-readable "last played" string
fn format_last_played(timestamp: &str) -> String {
    // Parse ISO timestamp and calculate relative time
    // For now, return a simplified format
    if timestamp.is_empty() {
        return "Never played".to_string();
    }

    // Extract date part from ISO timestamp
    if let Some(date_part) = timestamp.split('T').next() {
        // Simple date display
        return format!("Last: {}", date_part);
    }

    "Recently".to_string()
}

/// System badge component with genre styling
#[allow(dead_code)]
#[component]
fn SystemBadge(
    #[prop(into)]
    system: String,
    #[prop(optional)]
    genre: Option<CampaignGenre>,
) -> impl IntoView {
    let genre = genre.unwrap_or_else(|| CampaignGenre::from_system(&system));
    let genre_label = genre.label();

    view! {
        <span class="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium bg-zinc-800/80 text-zinc-300 border border-zinc-700 backdrop-blur-sm">
            <span class="w-1.5 h-1.5 rounded-full bg-current opacity-60"></span>
            {genre_label}
        </span>
    }
}

/// Genre badge for the card corner
#[component]
fn GenreBadge(genre: CampaignGenre) -> impl IntoView {
    let (_, text_class) = genre.style();
    view! {
        <span class=format!(
            "inline-flex items-center px-2 py-0.5 rounded text-[10px] font-bold uppercase tracking-wider {} bg-black/30 backdrop-blur-sm",
            text_class
        )>
            {genre.label()}
        </span>
    }
}

/// Now Playing indicator with pulse animation
#[component]
fn NowPlayingIndicator() -> impl IntoView {
    view! {
        <div class="absolute top-3 left-3 flex items-center gap-2 px-2.5 py-1 rounded-full bg-green-500/20 border border-green-500/40 backdrop-blur-sm">
            <div class="relative flex items-center justify-center">
                // Animated pulse rings
                <div class="absolute w-3 h-3 rounded-full bg-green-500/30 animate-ping"></div>
                <div class="w-2 h-2 rounded-full bg-green-500"></div>
            </div>
            <span class="text-[10px] font-bold uppercase tracking-wider text-green-400">
                "Now Playing"
            </span>
        </div>
    }
}

/// Sound wave animation for active campaigns
#[component]
fn SoundWaveAnimation() -> impl IntoView {
    view! {
        <div class="flex items-end gap-0.5 h-3">
            <div class="w-0.5 bg-green-400 animate-soundbar-1 rounded-full"></div>
            <div class="w-0.5 bg-green-400 animate-soundbar-2 rounded-full"></div>
            <div class="w-0.5 bg-green-400 animate-soundbar-3 rounded-full"></div>
            <div class="w-0.5 bg-green-400 animate-soundbar-4 rounded-full"></div>
        </div>
    }
}

/// Album cover style campaign card (Spotify-inspired)
#[component]
pub fn CampaignCard(
    /// The campaign to display
    campaign: Campaign,
    /// Session count (optional, loaded separately)
    #[prop(optional, default = 0)]
    session_count: u32,
    /// Player/NPC count (optional)
    #[prop(optional, default = 0)]
    entity_count: usize,
    /// Player count (human players)
    #[prop(optional, default = 0)]
    player_count: usize,
    /// Last played timestamp (ISO format)
    #[prop(optional, into)]
    last_played: Option<String>,
    /// Cover image URL (optional)
    #[prop(optional, into)]
    cover_image: Option<String>,
    /// Whether this campaign is currently active ("Now Playing")
    #[prop(optional, default = false)]
    is_active: bool,
    /// Callback when card is clicked
    on_click: Callback<String>,
    /// Callback when delete is requested
    #[prop(optional)]
    on_delete: Option<Callback<(String, String)>>,
    /// Whether this card is selected
    #[prop(optional, default = false)]
    is_selected: bool,
) -> impl IntoView {
    let genre = CampaignGenre::from_system(&campaign.system);
    let (bg_class, text_class) = genre.style();

    // Get first two letters for better initials
    let initials: String = campaign.name
        .split_whitespace()
        .take(2)
        .filter_map(|word| word.chars().next())
        .collect();
    let initials = if initials.is_empty() {
        campaign.name.chars().next().unwrap_or('?').to_string()
    } else {
        initials
    };

    let campaign_name = campaign.name.clone();
    let campaign_system = campaign.system.clone();
    let campaign_desc = campaign.description.clone().unwrap_or_default();

    // Clone for closures
    let click_id = campaign.id.clone();
    let delete_id = campaign.id.clone();
    let delete_name = campaign.name.clone();

    let handle_click = move |_: ev::MouseEvent| {
        on_click.run(click_id.clone());
    };

    let handle_delete = move |evt: ev::MouseEvent| {
        evt.stop_propagation();
        if let Some(ref callback) = on_delete {
            callback.run((delete_id.clone(), delete_name.clone()));
        }
    };

    let selected_border = if is_selected {
        "ring-2 ring-purple-500 ring-offset-2 ring-offset-zinc-900"
    } else if is_active {
        "ring-2 ring-green-500/50"
    } else {
        "ring-1 ring-zinc-800 hover:ring-zinc-600"
    };

    let last_played_str = last_played.map(|lp| format_last_played(&lp));

    view! {
        <div
            class=format!(
                "group relative aspect-[3/4] bg-zinc-900 rounded-xl overflow-hidden shadow-2xl {} transition-all duration-300 hover:-translate-y-1 hover:shadow-3xl cursor-pointer",
                selected_border
            )
            on:click=handle_click
            role="button"
            tabindex="0"
            aria-label=format!("Campaign: {}", campaign_name.clone())
        >
            // Cover Art Background
            {match cover_image.clone() {
                Some(url) => view! {
                    <div
                        class="absolute inset-0 bg-cover bg-center"
                        style=format!("background-image: url('{}')", url)
                    >
                        <div class="absolute inset-0 bg-gradient-to-t from-zinc-900 via-zinc-900/60 to-transparent"></div>
                    </div>
                }.into_any(),
                None => view! {
                    <div class=format!("absolute inset-0 {} opacity-30 group-hover:opacity-40 transition-opacity duration-500", bg_class)>
                        // Decorative pattern overlay
                        <div class="absolute inset-0 opacity-10" style="background-image: radial-gradient(circle at 20% 80%, rgba(255,255,255,0.1) 0%, transparent 50%);"></div>
                    </div>
                }.into_any(),
            }}

            // Content Container
            <div class="relative h-full flex flex-col p-5">
                // Top Row: Genre Badge + Delete
                <div class="flex justify-between items-start">
                    <GenreBadge genre=genre />

                    // Delete button (visible on hover)
                    {move || on_delete.as_ref().map(|_| view! {
                        <button
                            class="opacity-0 group-hover:opacity-100 p-1.5 rounded-full bg-zinc-900/60 text-zinc-400 hover:text-red-400 hover:bg-red-900/40 transition-all backdrop-blur-sm"
                            on:click=handle_delete.clone()
                            aria-label="Delete campaign"
                        >
                            <Icon icon=TRASH size="14px" />
                        </button>
                    })}
                </div>

                // Now Playing Indicator
                {move || {
                    if is_active {
                        Some(view! { <NowPlayingIndicator /> })
                    } else {
                        None
                    }
                }}

                // Center: Album Art / Initials
                <div class="flex-1 flex items-center justify-center">
                    <div class="relative">
                        // Glow effect behind initials
                        <div class=format!("absolute inset-0 blur-3xl {} opacity-20 scale-150", bg_class)></div>
                        <span class=format!(
                            "relative text-7xl font-black {} opacity-40 select-none group-hover:scale-110 group-hover:opacity-60 transition-all duration-500",
                            text_class
                        )>
                            {initials}
                        </span>
                    </div>
                </div>

                // Bottom Info
                <div class="space-y-3 mt-auto">
                    // Title
                    <h3 class="text-xl font-bold text-white leading-tight group-hover:text-purple-300 transition-colors line-clamp-2">
                        {campaign_name.clone()}
                    </h3>

                    // System (as "artist" in Spotify metaphor)
                    <p class="text-sm text-zinc-400 group-hover:text-zinc-300 transition-colors">
                        {campaign_system}
                    </p>

                    // Description
                    {move || {
                        if !campaign_desc.is_empty() {
                            Some(view! {
                                <p class="text-xs text-zinc-500 line-clamp-2 italic">
                                    {campaign_desc.clone()}
                                </p>
                            })
                        } else {
                            None
                        }
                    }}

                    // Stats Row (track list metaphor)
                    <div class="pt-3 flex items-center justify-between text-xs font-medium text-zinc-500 border-t border-white/5">
                        <div class="flex items-center gap-4">
                            // Sessions as "tracks"
                            <div class="flex items-center gap-1.5">
                                <Icon icon=DISC size="12px" />
                                <span>{session_count}</span>
                                <span class="text-zinc-600">"tracks"</span>
                            </div>

                            // Players
                            {if player_count > 0 {
                                Some(view! {
                                    <div class="flex items-center gap-1.5">
                                        <Icon icon=USERS size="12px" />
                                        <span>{player_count}</span>
                                    </div>
                                }.into_any())
                            } else if entity_count > 0 {
                                Some(view! {
                                    <div class="flex items-center gap-1.5">
                                        <Icon icon=USER size="12px" />
                                        <span>{entity_count}</span>
                                    </div>
                                }.into_any())
                            } else {
                                None
                            }}
                        </div>

                        // Last played or sound wave animation
                        {move || {
                            if is_active {
                                view! { <SoundWaveAnimation /> }.into_any()
                            } else if let Some(ref lp) = last_played_str {
                                view! {
                                    <span class="text-[10px] text-zinc-600">
                                        {lp.clone()}
                                    </span>
                                }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }
                        }}
                    </div>
                </div>
            </div>

            // Bottom gradient bar for "Now Playing" effect
            {move || {
                if is_active {
                    Some(view! {
                        <div class="absolute bottom-0 left-0 right-0 h-1 bg-gradient-to-r from-green-500 via-green-400 to-green-500 animate-pulse"></div>
                    })
                } else if session_count > 0 {
                    Some(view! {
                        <div class="absolute bottom-0 left-0 right-0 h-0.5 bg-gradient-to-r from-transparent via-purple-500 to-transparent opacity-0 group-hover:opacity-100 transition-opacity"></div>
                    })
                } else {
                    None
                }
            }}

            // Selection indicator
            {move || {
                if is_selected && !is_active {
                    Some(view! {
                        <div class="absolute top-3 right-3 w-3 h-3 rounded-full bg-purple-500 animate-pulse"></div>
                    })
                } else {
                    None
                }
            }}
        </div>
    }
}

// Icon Components

#[component]
fn DeleteIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
        </svg>
    }
}

#[component]
fn TrackIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"></circle>
            <polygon points="10,8 16,12 10,16 10,8"></polygon>
        </svg>
    }
}

#[component]
fn PlayerIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path>
            <circle cx="9" cy="7" r="4"></circle>
            <path d="M23 21v-2a4 4 0 0 0-3-3.87"></path>
            <path d="M16 3.13a4 4 0 0 1 0 7.75"></path>
        </svg>
    }
}

#[component]
fn EntityIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="7" height="7"></rect>
            <rect x="14" y="3" width="7" height="7"></rect>
            <rect x="14" y="14" width="7" height="7"></rect>
            <rect x="3" y="14" width="7" height="7"></rect>
        </svg>
    }
}

/// Compact campaign card for lists (Spotify-style list item)
#[component]
pub fn CampaignCardCompact(
    /// The campaign to display
    campaign: Campaign,
    /// Callback when card is clicked
    on_click: Callback<String>,
    /// Whether this card is selected
    #[prop(optional, default = false)]
    is_selected: bool,
    /// Whether this campaign is currently active ("Now Playing")
    #[prop(optional, default = false)]
    is_active: bool,
    /// Session count for display
    #[prop(optional, default = 0)]
    session_count: u32,
) -> impl IntoView {
    let genre = CampaignGenre::from_system(&campaign.system);
    let (bg_class, text_class) = genre.style();

    // Get initials
    let initials: String = campaign.name
        .split_whitespace()
        .take(2)
        .filter_map(|word| word.chars().next())
        .collect();
    let initials = if initials.is_empty() {
        campaign.name.chars().next().unwrap_or('?').to_string()
    } else {
        initials
    };

    let campaign_id = campaign.id.clone();
    let campaign_name = campaign.name.clone();
    let campaign_system = campaign.system.clone();

    let handle_click = move |_: ev::MouseEvent| {
        on_click.run(campaign_id.clone());
    };

    let selected_class = if is_active {
        "bg-green-900/20 border-l-2 border-green-500"
    } else if is_selected {
        "bg-zinc-800 border-l-2 border-purple-500"
    } else {
        "hover:bg-zinc-800/50 border-l-2 border-transparent"
    };

    view! {
        <button
            class=format!("w-full flex items-center gap-3 p-3 rounded-r-lg transition-colors text-left group {}", selected_class)
            on:click=handle_click
        >
            // Avatar with genre color
            <div class=format!("relative w-10 h-10 rounded-lg {} flex items-center justify-center text-sm font-bold {} shadow-md", bg_class, text_class)>
                {initials.clone()}
                // Active indicator
                {if is_active {
                    Some(view! {
                        <div class="absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full bg-green-500 border-2 border-zinc-900"></div>
                    })
                } else {
                    None
                }}
            </div>

            // Info
            <div class="flex-1 min-w-0">
                <div class=move || {
                    if is_active {
                        "text-sm font-medium text-green-400 truncate"
                    } else {
                        "text-sm font-medium text-white group-hover:text-purple-300 truncate transition-colors"
                    }
                }>
                    {campaign_name}
                </div>
                <div class="flex items-center gap-2 text-xs text-zinc-500">
                    <span>{campaign_system}</span>
                    {if session_count > 0 {
                        Some(view! {
                            <>
                                <span class="text-zinc-700">"*"</span>
                                <span>{format!("{} sessions", session_count)}</span>
                            </>
                        })
                    } else {
                        None
                    }}
                </div>
            </div>

            // Sound wave or play icon
            {if is_active {
                view! {
                    <div class="flex items-end gap-0.5 h-3 mr-1">
                        <div class="w-0.5 h-1 bg-green-400 animate-pulse rounded-full"></div>
                        <div class="w-0.5 h-2 bg-green-400 animate-pulse rounded-full" style="animation-delay: 0.1s"></div>
                        <div class="w-0.5 h-1.5 bg-green-400 animate-pulse rounded-full" style="animation-delay: 0.2s"></div>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="opacity-0 group-hover:opacity-100 transition-opacity text-zinc-500">
                        <Icon icon=PLAY_CIRCLE size="16px" />
                    </div>
                }.into_any()
            }}
        </button>
    }
}

#[component]
fn PlayIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <polygon points="5,3 19,12 5,21 5,3"></polygon>
        </svg>
    }
}

/// Campaign card for grid view with minimal details
#[component]
pub fn CampaignCardMini(
    /// The campaign to display
    campaign: Campaign,
    /// Callback when card is clicked
    on_click: Callback<String>,
    /// Whether this campaign is currently active
    #[prop(optional, default = false)]
    is_active: bool,
) -> impl IntoView {
    let genre = CampaignGenre::from_system(&campaign.system);
    let (bg_class, text_class) = genre.style();

    let initials: String = campaign.name
        .split_whitespace()
        .take(2)
        .filter_map(|word| word.chars().next())
        .collect();
    let initials = if initials.is_empty() {
        campaign.name.chars().next().unwrap_or('?').to_string()
    } else {
        initials
    };

    let campaign_id = campaign.id.clone();
    let campaign_name = campaign.name.clone();

    let handle_click = move |_: ev::MouseEvent| {
        on_click.run(campaign_id.clone());
    };

    view! {
        <button
            class="group relative w-full aspect-square rounded-lg overflow-hidden transition-all hover:-translate-y-0.5 hover:shadow-xl"
            on:click=handle_click
        >
            // Background
            <div class=format!("absolute inset-0 {} transition-opacity", bg_class)></div>

            // Initials
            <div class="relative h-full flex items-center justify-center">
                <span class=format!("text-3xl font-black {} opacity-60 group-hover:opacity-80 transition-opacity", text_class)>
                    {initials}
                </span>
            </div>

            // Active indicator
            {if is_active {
                Some(view! {
                    <div class="absolute top-2 right-2 w-2 h-2 rounded-full bg-green-500 animate-pulse"></div>
                })
            } else {
                None
            }}

            // Title tooltip on hover
            <div class="absolute bottom-0 left-0 right-0 p-2 bg-gradient-to-t from-black/80 to-transparent opacity-0 group-hover:opacity-100 transition-opacity">
                <p class="text-xs font-medium text-white truncate">{campaign_name}</p>
            </div>
        </button>
    }
}
