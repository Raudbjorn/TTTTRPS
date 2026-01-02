//! Campaign Card Component
//!
//! An album cover-style visual design for campaign display.
//! Features:
//!   - 3:4 aspect ratio like vinyl records
//!   - Dynamic gradient background based on game system
//!   - Large initial letter as cover art placeholder
//!   - Hover state with "Now Playing" indicator
//!   - Session count and player statistics
//!   - Quick-action delete button

use leptos::prelude::*;
use leptos::ev;
use crate::bindings::Campaign;

/// Get system-based styling for campaign card
fn get_system_style(system: &str) -> (&'static str, &'static str) {
    let s = system.to_lowercase();
    if s.contains("d&d") || s.contains("5e") || s.contains("pathfinder") {
        (
            "from-amber-700 via-amber-800 to-purple-900",
            "text-amber-200",
        )
    } else if s.contains("cthulhu") || s.contains("horror") || s.contains("vampire") {
        (
            "from-slate-800 via-emerald-950 to-black",
            "text-emerald-400",
        )
    } else if s.contains("cyber") || s.contains("shadow") || s.contains("neon") {
        (
            "from-fuchsia-900 via-violet-900 to-purple-950",
            "text-fuchsia-300",
        )
    } else if s.contains("space") || s.contains("alien") || s.contains("scifi") || s.contains("mothership") {
        (
            "from-cyan-900 via-blue-950 to-slate-900",
            "text-cyan-200",
        )
    } else if s.contains("delta") || s.contains("spy") || s.contains("noir") {
        (
            "from-stone-800 via-amber-950 to-stone-900",
            "text-amber-100",
        )
    } else {
        (
            "from-zinc-700 via-zinc-800 to-zinc-900",
            "text-zinc-300",
        )
    }
}

/// Album cover-style campaign card
#[component]
pub fn CampaignCard(
    /// The campaign data
    campaign: Campaign,
    /// Session count for this campaign
    #[prop(default = 0)]
    session_count: u32,
    /// Player/NPC count
    #[prop(default = 0)]
    player_count: usize,
    /// Callback when card is clicked
    #[prop(into)]
    on_click: Callback<String>,
    /// Callback for delete action
    #[prop(optional, into)]
    on_delete: Option<Callback<(String, String)>>,
) -> impl IntoView {
    let (bg_gradient, text_color) = get_system_style(&campaign.system);
    let initials = campaign.name.chars().next().unwrap_or('?').to_uppercase().to_string();
    let is_hovered = RwSignal::new(false);

    let campaign_id = campaign.id.clone();
    let campaign_name = campaign.name.clone();
    let campaign_system = campaign.system.clone();
    let campaign_desc = campaign.description.clone().unwrap_or_default();

    // Click handlers
    let click_id = campaign_id.clone();
    let handle_click = move |_: ev::MouseEvent| {
        on_click.run(click_id.clone());
    };

    let delete_id = campaign_id.clone();
    let delete_name = campaign_name.clone();
    let handle_delete = move |evt: ev::MouseEvent| {
        evt.stop_propagation();
        if let Some(ref cb) = on_delete {
            cb.run((delete_id.clone(), delete_name.clone()));
        }
    };

    view! {
        <article
            class="group relative aspect-[3/4] bg-[var(--bg-deep)] rounded-xl overflow-hidden shadow-2xl border border-[var(--border-subtle)] hover:border-[var(--border-strong)] transition-all duration-300 hover:-translate-y-2 cursor-pointer"
            on:click=handle_click
            on:mouseenter=move |_| is_hovered.set(true)
            on:mouseleave=move |_| is_hovered.set(false)
            role="button"
            tabindex="0"
            aria-label=format!("Open campaign: {}", campaign_name.clone())
        >
            // Cover Art Background
            <div class=format!(
                "absolute inset-0 bg-gradient-to-br {} opacity-20 group-hover:opacity-40 transition-opacity duration-500",
                bg_gradient
            )></div>

            // Subtle texture overlay
            <div class="absolute inset-0 bg-[url('data:image/svg+xml,%3Csvg viewBox=\"0 0 200 200\" xmlns=\"http://www.w3.org/2000/svg\"%3E%3Cfilter id=\"noise\"%3E%3CfeTurbulence type=\"fractalNoise\" baseFrequency=\"0.9\" numOctaves=\"4\" stitchTiles=\"stitch\"/%3E%3C/filter%3E%3Crect width=\"100%25\" height=\"100%25\" filter=\"url(%23noise)\" opacity=\"0.05\"/%3E%3C/svg%3E')] opacity-30"></div>

            // Content Container
            <div class="relative h-full flex flex-col p-5">
                // Top Row: Badge and Actions
                <header class="flex justify-between items-start">
                    // System Badge
                    <span class="inline-flex items-center px-2.5 py-1 rounded-md text-[10px] font-bold uppercase tracking-wider bg-black/40 backdrop-blur-sm text-[var(--text-muted)] border border-white/10">
                        {campaign_system}
                    </span>

                    // Delete Button (visible on hover)
                    {on_delete.is_some().then(|| view! {
                        <button
                            class="opacity-0 group-hover:opacity-100 p-2 rounded-lg bg-black/40 backdrop-blur-sm text-[var(--text-muted)] hover:text-red-400 hover:bg-red-500/20 transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-red-500"
                            aria-label="Delete campaign"
                            on:click=handle_delete
                        >
                            <DeleteIcon />
                        </button>
                    })}
                </header>

                // Center: Cover Art Initial
                <div class="flex-1 flex items-center justify-center relative">
                    // Glow effect on hover
                    <div class=move || format!(
                        "absolute inset-0 bg-gradient-radial from-white/5 to-transparent transition-opacity duration-500 {}",
                        if is_hovered.get() { "opacity-100" } else { "opacity-0" }
                    )></div>

                    <span class=format!(
                        "text-[7rem] font-black {} opacity-25 select-none group-hover:scale-110 group-hover:opacity-40 transition-all duration-500",
                        text_color
                    )>
                        {initials}
                    </span>
                </div>

                // Bottom: Info Section
                <footer class="space-y-3">
                    // Title
                    <h3 class="text-lg font-bold text-[var(--text-primary)] leading-tight group-hover:text-[var(--accent)] transition-colors line-clamp-2">
                        {campaign_name.clone()}
                    </h3>

                    // Description (if available)
                    {(!campaign_desc.is_empty()).then(|| view! {
                        <p class="text-xs text-[var(--text-muted)] line-clamp-2 leading-relaxed">
                            {campaign_desc}
                        </p>
                    })}

                    // Stats Row
                    <div class="pt-3 flex items-center gap-4 text-[11px] font-medium text-[var(--text-muted)] border-t border-white/5">
                        <div class="flex items-center gap-1.5">
                            <SessionIcon />
                            <span>{session_count}</span>
                            <span class="opacity-60">"sessions"</span>
                        </div>
                        <div class="flex items-center gap-1.5">
                            <UserIcon />
                            <span>{player_count}</span>
                            <span class="opacity-60">"characters"</span>
                        </div>
                    </div>
                </footer>
            </div>

            // "Now Playing" Indicator (bottom edge glow on hover)
            <div class=move || format!(
                "absolute bottom-0 left-0 right-0 h-1 bg-gradient-to-r from-transparent via-[var(--accent)] to-transparent transition-opacity duration-300 {}",
                if is_hovered.get() { "opacity-100" } else { "opacity-0" }
            )></div>

            // Corner accent (active state indicator)
            {(session_count > 0).then(|| view! {
                <div class="absolute top-3 right-3 w-2 h-2 rounded-full bg-green-500 animate-pulse shadow-lg shadow-green-500/50"></div>
            })}
        </article>
    }
}

/// Compact campaign card variant for list views
#[component]
pub fn CampaignCardCompact(
    campaign: Campaign,
    session_count: u32,
    #[prop(into)]
    on_click: Callback<String>,
) -> impl IntoView {
    let (bg_gradient, text_color) = get_system_style(&campaign.system);
    let campaign_id = campaign.id.clone();
    let campaign_name = campaign.name.clone();
    let campaign_system = campaign.system.clone();

    view! {
        <button
            class=format!(
                "w-full flex items-center gap-4 p-3 rounded-lg bg-[var(--bg-surface)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] transition-all text-left group focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            )
            on:click=move |_| on_click.run(campaign_id.clone())
        >
            // Mini cover art
            <div class=format!(
                "w-12 h-16 rounded-md bg-gradient-to-br {} flex items-center justify-center flex-shrink-0",
                bg_gradient
            )>
                <span class=format!("text-2xl font-black {} opacity-40", text_color)>
                    {campaign_name.chars().next().unwrap_or('?')}
                </span>
            </div>

            // Info
            <div class="flex-1 min-w-0">
                <h4 class="text-sm font-semibold text-[var(--text-primary)] group-hover:text-[var(--accent)] transition-colors truncate">
                    {campaign_name}
                </h4>
                <div class="flex items-center gap-2 text-[10px] text-[var(--text-muted)]">
                    <span>{campaign_system}</span>
                    <span class="opacity-50">"/"</span>
                    <span>{format!("{} sessions", session_count)}</span>
                </div>
            </div>

            // Arrow
            <ChevronRightIcon />
        </button>
    }
}

// SVG Icon Components

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
fn SessionIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect>
            <line x1="16" y1="2" x2="16" y2="6"></line>
            <line x1="8" y1="2" x2="8" y2="6"></line>
            <line x1="3" y1="10" x2="21" y2="10"></line>
        </svg>
    }
}

#[component]
fn UserIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"></path>
            <circle cx="12" cy="7" r="4"></circle>
        </svg>
    }
}

#[component]
fn ChevronRightIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-[var(--text-muted)] group-hover:text-[var(--accent)] transition-colors">
            <polyline points="9 18 15 12 9 6"></polyline>
        </svg>
    }
}
