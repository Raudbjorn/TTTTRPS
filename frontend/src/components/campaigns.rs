//! Campaigns Component - Leptos Migration
//!
//! Displays a list of campaigns with create/delete functionality.
//! Campaign management components with archive/restore support.

use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use std::sync::Arc;
use crate::services::notification_service::{show_error, show_success, ToastAction};

use crate::bindings::{list_campaigns, create_campaign, delete_campaign, archive_campaign, restore_campaign, list_archived_campaigns, Campaign};
use crate::components::design_system::{Button, ButtonVariant, LoadingSpinner};
use crate::components::campaign::CampaignCreateModal;
use crate::components::campaign_wizard::WizardShell;

/// Helper function to get system-based styling
fn get_system_style(system: &str) -> (&'static str, &'static str) {
    let s = system.to_lowercase();
    if s.contains("d&d") || s.contains("5e") || s.contains("pathfinder") {
        (
            "bg-gradient-to-br from-amber-700 to-amber-900",
            "text-amber-200",
        )
    } else if s.contains("cthulhu") || s.contains("horror") || s.contains("vampire") {
        (
            "bg-gradient-to-br from-slate-800 to-black",
            "text-red-400",
        )
    } else if s.contains("cyber") || s.contains("shadow") || s.contains("neon") {
        (
            "bg-gradient-to-br from-fuchsia-900 to-purple-900",
            "text-fuchsia-300",
        )
    } else if s.contains("space") || s.contains("alien") || s.contains("scifi") {
        (
            "bg-gradient-to-br from-cyan-900 to-blue-900",
            "text-cyan-200",
        )
    } else {
        (
            "bg-gradient-to-br from-zinc-700 to-zinc-900",
            "text-zinc-300",
        )
    }
}

/// Stat card component for displaying summary statistics
#[component]
fn StatCard(
    /// Label for the stat
    #[prop(into)]
    label: String,
    /// Value to display
    #[prop(into)]
    value: String,
) -> impl IntoView {
    view! {
        <div class="bg-zinc-900 border border-zinc-800 p-4 rounded-lg">
            <div class="text-zinc-500 text-xs font-bold uppercase tracking-wider mb-1">
                {label}
            </div>
            <div class="text-2xl font-bold text-zinc-100">
                {value}
            </div>
        </div>
    }
}

/// Badge component for system type display
#[component]
fn SystemBadge(
    #[prop(into)]
    system: String,
) -> impl IntoView {
    view! {
        <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-zinc-800 text-zinc-300 border border-zinc-700">
            {system}
        </span>
    }
}

/// Campaign card component
#[component]
fn CampaignCard(
    campaign: Campaign,
    on_delete: Callback<(String, String)>,
) -> impl IntoView {
    let navigate = use_navigate();
    let (bg_class, text_class) = get_system_style(&campaign.system);
    let initials = campaign.name.chars().next().unwrap_or('?');

    let campaign_id = campaign.id.clone();
    let campaign_name = campaign.name.clone();
    let campaign_system = campaign.system.clone();
    let campaign_desc = campaign.description.clone().unwrap_or_default();
    // Note: session_count and player_count now require separate API call via get_campaign_stats
    let session_count = 0_u32;
    let player_count = 0_usize;

    // Clone for closures
    let delete_id = campaign.id.clone();
    let delete_name = campaign.name.clone();
    let nav_id = campaign_id.clone();

    let handle_click = move |_: ev::MouseEvent| {
        let nav = navigate.clone();
        let id = nav_id.clone();
        nav(&format!("/session/{}", id), Default::default());
    };

    let handle_delete = move |evt: ev::MouseEvent| {
        evt.stop_propagation();
        on_delete.run((delete_id.clone(), delete_name.clone()));
    };

    view! {
        <div
            class="group relative aspect-[3/4] bg-zinc-900 rounded-xl overflow-hidden shadow-2xl border border-zinc-800 hover:border-zinc-600 transition-all hover:-translate-y-1 cursor-pointer"
            on:click=handle_click
        >
            // "Cover Art" Background
            <div class=format!("absolute inset-0 {} opacity-20 group-hover:opacity-30 transition-opacity", bg_class)></div>

            // Content Container
            <div class="relative h-full flex flex-col p-6">
                // Top Badge
                <div class="flex justify-between items-start">
                    <SystemBadge system=campaign_system />

                    // Delete button (visible on hover)
                    <button
                        class="opacity-0 group-hover:opacity-100 p-2 text-zinc-400 hover:text-red-400 transition-all"
                        on:click=handle_delete
                    >
                        "X"
                    </button>
                </div>

                // Center Initials (Placeholder Art)
                <div class="flex-1 flex items-center justify-center">
                    <span class=format!("text-8xl font-black {} opacity-20 select-none group-hover:scale-110 transition-transform duration-500", text_class)>
                        {initials.to_string()}
                    </span>
                </div>

                // Bottom Info
                <div class="space-y-2">
                    <h3 class="text-xl font-bold text-white leading-tight group-hover:text-purple-300 transition-colors">
                        {campaign_name}
                    </h3>
                    {move || {
                        if !campaign_desc.is_empty() {
                            Some(view! {
                                <p class="text-xs text-zinc-400 line-clamp-2">{campaign_desc.clone()}</p>
                            })
                        } else {
                            None
                        }
                    }}

                    // Stats Row
                    <div class="pt-4 flex items-center gap-4 text-xs font-medium text-zinc-500 border-t border-white/5">
                        <div class="flex items-center gap-1">
                            <span>{session_count}</span>
                            " Sessions"
                        </div>
                        <div class="flex items-center gap-1">
                            <span>{player_count}</span>
                            " Players"
                        </div>
                    </div>
                </div>
            </div>

            // "Now Playing" Pulse (if has sessions)
            {move || {
                if session_count > 0 {
                    Some(view! {
                        <div class="absolute bottom-0 left-0 w-full h-1 bg-gradient-to-r from-transparent via-purple-500 to-transparent opacity-0 group-hover:opacity-100 transition-opacity"></div>
                    })
                } else {
                    None
                }
            }}
        </div>
    }
}

/// Create Campaign Modal component (legacy, kept for reference)
#[allow(dead_code)]
#[component]
fn CreateCampaignModal(
    is_open: RwSignal<bool>,
    on_create: Callback<(String, String)>,
) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let system = RwSignal::new("D&D 5e".to_string());

    let handle_create = move |_: ev::MouseEvent| {
        let n = name.get();
        let s = system.get();
        if !n.trim().is_empty() {
            on_create.run((n, s));
            name.set(String::new());
            system.set("D&D 5e".to_string());
        }
    };

    let handle_close = move |_: ev::MouseEvent| {
        is_open.set(false);
    };

    let systems = vec![
        "D&D 5e",
        "Pathfinder 2e",
        "Call of Cthulhu",
        "Delta Green",
        "Mothership",
        "Cyberpunk Red",
        "Shadowrun",
        "Vampire: The Masquerade",
        "Other",
    ];

    view! {
        <Show when=move || is_open.get()>
            // Backdrop
            <div
                class="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center"
                on:click=handle_close
            >
                // Modal Content
                <div
                    class="bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl w-full max-w-md mx-4"
                    on:click=move |evt: ev::MouseEvent| evt.stop_propagation()
                >
                    // Header
                    <div class="px-6 py-4 border-b border-zinc-800">
                        <h2 class="text-xl font-bold text-white">"Begin New Adventure"</h2>
                    </div>

                    // Body
                    <div class="p-6 space-y-6">
                        <div>
                            <label class="block text-sm font-bold text-zinc-400 mb-2">
                                "Campaign Name"
                            </label>
                            <input
                                type="text"
                                class="w-full px-4 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white placeholder-zinc-500 focus:border-blue-500 focus:ring-1 focus:ring-blue-500 outline-none transition-colors"
                                placeholder="e.g. The Tomb of Horrors"
                                prop:value=move || name.get()
                                on:input=move |evt| {
                                    name.set(event_target_value(&evt));
                                }
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-bold text-zinc-400 mb-2">
                                "Game System"
                            </label>
                            <select
                                class="w-full px-4 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white focus:border-blue-500 focus:ring-1 focus:ring-blue-500 outline-none transition-colors"
                                prop:value=move || system.get()
                                on:change=move |evt| {
                                    system.set(event_target_value(&evt));
                                }
                            >
                                {systems.iter().map(|s| {
                                    let s_owned = s.to_string();
                                    view! {
                                        <option value=s_owned.clone()>{s_owned.clone()}</option>
                                    }
                                }).collect::<Vec<_>>()}
                            </select>
                        </div>
                    </div>

                    // Footer
                    <div class="px-6 py-4 border-t border-zinc-800 flex justify-end gap-3">
                        <Button
                            variant=ButtonVariant::Secondary
                            on_click=handle_close
                        >
                            "Cancel"
                        </Button>
                        <Button
                            variant=ButtonVariant::Primary
                            on_click=handle_create
                        >
                            "Launch Campaign"
                        </Button>
                    </div>
                </div>
            </div>
        </Show>
    }
}

/// View mode for campaign list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignViewMode {
    Grid,
    List,
}

/// Filter state for campaigns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignFilter {
    Active,
    Archived,
    All,
}

/// Campaign switcher component for header
#[component]
pub fn CampaignSwitcher(
    campaigns: Vec<Campaign>,
    selected_id: Option<String>,
    on_select: Callback<String>,
) -> impl IntoView {
    let handle_change = move |evt: ev::Event| {
        let target = event_target::<web_sys::HtmlSelectElement>(&evt);
        let value = target.value();
        if !value.is_empty() {
            on_select.run(value);
        }
    };

    view! {
        <div class="relative">
            <select
                class="appearance-none px-4 py-2 pr-8 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm focus:border-purple-500 focus:outline-none cursor-pointer"
                on:change=handle_change
            >
                <option value="">"Select Campaign"</option>
                {campaigns.iter().map(|c| {
                    let id = c.id.clone();
                    let name = c.name.clone();
                    let is_selected = selected_id.as_ref() == Some(&id);
                    view! {
                        <option value=id selected=is_selected>{name}</option>
                    }
                }).collect_view()}
            </select>
            <div class="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-zinc-400">
                "v"
            </div>
        </div>
    }
}

/// Main Campaigns page component
#[component]
pub fn Campaigns() -> impl IntoView {
    let navigate = use_navigate();

    // State signals
    let campaigns = RwSignal::new(Vec::<Campaign>::new());
    let archived_campaigns = RwSignal::new(Vec::<Campaign>::new());
    let status_message = RwSignal::new(String::new());
    let show_create_modal = RwSignal::new(false);
    let show_ai_wizard = RwSignal::new(false);
    let is_loading = RwSignal::new(true);
    let delete_confirm = RwSignal::new(Option::<(String, String)>::None);
    let view_mode = RwSignal::new(CampaignViewMode::Grid);
    let filter = RwSignal::new(CampaignFilter::Active);
    let archive_confirm = RwSignal::new(Option::<(String, String, bool)>::None); // (id, name, is_archived)

    // Trigger for retrying fetches
    let refresh_trigger = Trigger::new();

    // Shared fetch logic for retry actions
    let fetch_campaigns = {
        let campaigns = campaigns;
        let archived_campaigns = archived_campaigns;
        let status_message = status_message;
        let is_loading = is_loading;

        Arc::new(move || {
            let campaigns = campaigns;
            let archived_campaigns = archived_campaigns;
            let _status_message = status_message;
            let is_loading = is_loading;

            spawn_local(async move {
                is_loading.set(true);
                // Load active campaigns
                match list_campaigns().await {
                    Ok(list) => {
                        campaigns.set(list);
                    }
                    Err(e) => {
                        let retry = Some(ToastAction {
                            label: "Retry".to_string(),
                            handler: Arc::new(move || refresh_trigger.notify()),
                        });

                         show_error(
                            "Failed to load campaigns",
                            Some(&format!("Could not fetch campaign list: {}", e)),
                            retry
                        );
                    }
                }

                // Load archived campaigns
                match list_archived_campaigns().await {
                    Ok(list) => {
                        archived_campaigns.set(list);
                    }
                    Err(_) => {
                        // Silently fail for archived
                    }
                }

                is_loading.set(false);
            });
        })
    };

    // Load campaigns on mount and refresh
    let fetch_for_effect = fetch_campaigns.clone();
    Effect::new(move |_| {
        refresh_trigger.track();
        (fetch_for_effect)();
    });

    // Refresh campaigns handler
    let refresh_campaigns = {
        let fetch = fetch_campaigns.clone();
        move |_: ev::MouseEvent| {
            (fetch)();
            show_success("Refreshing...", None);
        }
    };

    // Stats
    let total_sessions = RwSignal::new(0_u32);
    let total_players = RwSignal::new(0_usize);

    Effect::new(move |_| {
        let list = campaigns.get();
        if list.is_empty() { return; }

        spawn_local(async move {
            let mut s = 0;
            let mut p = 0;
            for c in list {
                if let Ok(st) = crate::bindings::get_campaign_stats(c.id).await {
                    s += st.session_count;
                    p += st.npc_count;
                }
            }
            total_sessions.set(s as u32);
            total_players.set(p);
        });
    });

    // Dropdown state for create menu
    let show_create_menu = RwSignal::new(false);

    // Open create modal (quick mode)
    let open_create_modal = move |_: ev::MouseEvent| {
        show_create_menu.set(false);
        show_create_modal.set(true);
    };

    // Open AI wizard
    let open_ai_wizard = move |_: ev::MouseEvent| {
        show_create_menu.set(false);
        show_ai_wizard.set(true);
    };

    // Toggle create menu
    let toggle_create_menu = move |_: ev::MouseEvent| {
        show_create_menu.update(|v| *v = !*v);
    };

    // Handle campaign creation (from wizard modal)
    let handle_create_wizard = Callback::new(move |campaign: Campaign| {
        campaigns.update(|c| c.push(campaign.clone()));
        show_create_modal.set(false);
        show_success("Campaign created!", Some("Ready for adventure."));
    });

    // Handle campaign creation (from AI wizard)
    let handle_ai_wizard_create = Callback::new(move |campaign: Campaign| {
        campaigns.update(|c| c.push(campaign));
        show_ai_wizard.set(false);
        show_success("Campaign created with AI assistance!", Some("Your adventure awaits."));
    });

    // Handle legacy campaign creation (from simple modal)
    let _handle_create = Callback::new(move |(name, system): (String, String)| {
        spawn_local(async move {
            match create_campaign(name, system).await {
                Ok(campaign) => {
                    campaigns.update(|c| c.push(campaign));
                    show_create_modal.set(false);
                    show_success("Campaign created!", None);
                }
                Err(e) => {
                    show_error("Failed to create campaign", Some(&e), None);
                }
            }
        });
    });

    // Handle archive request
    let _handle_archive_request = Callback::new(move |(id, name): (String, String)| {
        archive_confirm.set(Some((id, name, false)));
    });

    // Handle restore request
    let _handle_restore_request = Callback::new(move |(id, name): (String, String)| {
        archive_confirm.set(Some((id, name, true)));
    });

    // Handle confirmed archive/restore
    let handle_confirm_archive = move |_: ev::MouseEvent| {
        if let Some((id, name, is_restore)) = archive_confirm.get() {
            spawn_local(async move {
                let result = if is_restore {
                    restore_campaign(id.clone()).await
                } else {
                    archive_campaign(id.clone()).await
                };

                match result {
                    Ok(_) => {
                        if is_restore {
                            // Move from archived to active
                            archived_campaigns.update(|c| c.retain(|campaign| campaign.id != id));
                            // Reload to get the campaign
                            if let Ok(list) = list_campaigns().await {
                                campaigns.set(list);
                            }
                            show_success("Campaign Restored", Some(&format!("{} is back in action.", name)));
                        } else {
                            // Move from active to archived
                            if let Some(campaign) = campaigns.get().into_iter().find(|c| c.id == id) {
                                archived_campaigns.update(|c| c.push(campaign));
                            }
                            campaigns.update(|c| c.retain(|campaign| campaign.id != id));
                            show_success("Campaign Archived", Some(&format!("{} has been archived.", name)));
                        }
                    }
                    Err(e) => {
                        show_error(
                            if is_restore { "Failed to restore" } else { "Failed to archive" },
                            Some(&e),
                            None
                        );
                    }
                }
                archive_confirm.set(None);
            });
        }
    };

    let handle_cancel_archive = move |_: ev::MouseEvent| {
        archive_confirm.set(None);
    };

    // Toggle view mode
    let toggle_view_mode = move |_: ev::MouseEvent| {
        view_mode.update(|v| {
            *v = match v {
                CampaignViewMode::Grid => CampaignViewMode::List,
                CampaignViewMode::List => CampaignViewMode::Grid,
            };
        });
    };

    // Set filter
    let set_filter_active = move |_: ev::MouseEvent| {
        filter.set(CampaignFilter::Active);
    };

    let set_filter_archived = move |_: ev::MouseEvent| {
        filter.set(CampaignFilter::Archived);
    };

    let set_filter_all = move |_: ev::MouseEvent| {
        filter.set(CampaignFilter::All);
    };

    // Handle delete request (show confirmation)
    let handle_delete_request = Callback::new(move |(id, name): (String, String)| {
        delete_confirm.set(Some((id, name)));
    });

    // Handle confirmed delete
    let handle_confirm_delete = move |_: ev::MouseEvent| {
        if let Some((id, name)) = delete_confirm.get() {
            spawn_local(async move {
                match delete_campaign(id.clone()).await {
                    Ok(_) => {
                        campaigns.update(|c| c.retain(|campaign| campaign.id != id));
                         show_success("Campaign Deleted", Some(&format!("{} is gone forever.", name)));
                         delete_confirm.set(None);
                    }
                    Err(e) => {
                        show_error("Failed to delete", Some(&e), None);
                        // Do not close modal so user can retry
                    }
                }
            });
        }
    };

    let handle_cancel_delete = move |_: ev::MouseEvent| {
        delete_confirm.set(None);
    };

    // Navigate back to hub
    let nav_hub_fn = navigate.clone();
    let nav_to_hub = move |_: ev::MouseEvent| {
        nav_hub_fn("/", Default::default());
    };

    // Close create menu when clicking outside
    let close_create_menu = move |_: ev::MouseEvent| {
        if show_create_menu.get() {
            show_create_menu.set(false);
        }
    };

    view! {
        <div
            class="p-8 bg-zinc-950 text-zinc-100 min-h-screen font-sans selection:bg-purple-500/30"
            on:click=close_create_menu
        >
            <div class="max-w-7xl mx-auto space-y-8">
                // Header & Quick Actions
                <div class="flex flex-col md:flex-row md:items-end justify-between gap-4 pb-6 border-b border-zinc-900">
                    <div class="space-y-1">
                        <button
                            class="text-zinc-500 hover:text-white transition-colors text-sm font-medium"
                            on:click=nav_to_hub
                        >
                            "< Back to Hub"
                        </button>
                        <h1 class="text-4xl font-extrabold tracking-tight bg-clip-text text-transparent bg-gradient-to-r from-white to-zinc-500">
                            "Campaigns"
                        </h1>
                        <p class="text-zinc-400">"Manage your ongoing adventures and chronicles."</p>
                    </div>
                    <div class="flex items-center gap-3">
                        // View mode toggle
                        <div class="flex border border-zinc-700 rounded-lg overflow-hidden">
                            <button
                                class=move || if view_mode.get() == CampaignViewMode::Grid {
                                    "px-3 py-2 bg-zinc-700 text-white"
                                } else {
                                    "px-3 py-2 bg-zinc-800 text-zinc-400 hover:text-white"
                                }
                                on:click=toggle_view_mode.clone()
                                title="Grid View"
                            >
                                "Grid"
                            </button>
                            <button
                                class=move || if view_mode.get() == CampaignViewMode::List {
                                    "px-3 py-2 bg-zinc-700 text-white"
                                } else {
                                    "px-3 py-2 bg-zinc-800 text-zinc-400 hover:text-white"
                                }
                                on:click=toggle_view_mode.clone()
                                title="List View"
                            >
                                "List"
                            </button>
                        </div>

                        // Filter tabs
                        <div class="flex border border-zinc-700 rounded-lg overflow-hidden">
                            <button
                                class=move || if filter.get() == CampaignFilter::Active {
                                    "px-3 py-2 bg-zinc-700 text-white text-sm"
                                } else {
                                    "px-3 py-2 bg-zinc-800 text-zinc-400 hover:text-white text-sm"
                                }
                                on:click=set_filter_active
                            >
                                "Active"
                            </button>
                            <button
                                class=move || if filter.get() == CampaignFilter::Archived {
                                    "px-3 py-2 bg-zinc-700 text-white text-sm"
                                } else {
                                    "px-3 py-2 bg-zinc-800 text-zinc-400 hover:text-white text-sm"
                                }
                                on:click=set_filter_archived
                            >
                                "Archived"
                            </button>
                            <button
                                class=move || if filter.get() == CampaignFilter::All {
                                    "px-3 py-2 bg-zinc-700 text-white text-sm"
                                } else {
                                    "px-3 py-2 bg-zinc-800 text-zinc-400 hover:text-white text-sm"
                                }
                                on:click=set_filter_all
                            >
                                "All"
                            </button>
                        </div>

                        <Button
                            variant=ButtonVariant::Secondary
                            on_click=refresh_campaigns
                        >
                            "Refresh"
                        </Button>

                        // Create Campaign Dropdown
                        <div
                            class="relative"
                            on:click=move |ev: ev::MouseEvent| ev.stop_propagation()
                        >
                            <Button
                                variant=ButtonVariant::Primary
                                on_click=toggle_create_menu
                            >
                                "+ New Adventure"
                                <svg class="w-4 h-4 ml-1 inline-block" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                                </svg>
                            </Button>

                            // Dropdown menu
                            <Show when=move || show_create_menu.get()>
                                <div class="absolute right-0 mt-2 w-64 bg-zinc-900 border border-zinc-700 rounded-lg shadow-xl z-50 overflow-hidden">
                                    <button
                                        class="w-full px-4 py-3 text-left hover:bg-zinc-800 transition-colors border-b border-zinc-800"
                                        on:click=open_create_modal
                                    >
                                        <div class="font-medium text-white">"Quick Create"</div>
                                        <div class="text-xs text-zinc-400 mt-0.5">"Simple wizard with basic options"</div>
                                    </button>
                                    <button
                                        class="w-full px-4 py-3 text-left hover:bg-zinc-800 transition-colors group"
                                        on:click=open_ai_wizard
                                    >
                                        <div class="flex items-center gap-2">
                                            <span class="font-medium text-white">"AI-Assisted"</span>
                                            <span class="px-1.5 py-0.5 bg-purple-900/50 text-purple-300 text-xs rounded">
                                                "Recommended"
                                            </span>
                                        </div>
                                        <div class="text-xs text-zinc-400 mt-0.5">"Comprehensive wizard with AI guidance"</div>
                                    </button>
                                </div>
                            </Show>
                        </div>
                    </div>
                </div>

                // Stats Row
                {move || {
                    let c = campaigns.get();
                    let count_c = c.len();
                    // Stats fetched via effect
                    let count_s = total_sessions.get();
                    let count_p = total_players.get();

                    view! {
                        <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                            <StatCard label="Campaigns" value=count_c.to_string() />
                            <StatCard label="Total Sessions" value=count_s.to_string() />
                            <StatCard label="Active Players" value=count_p.to_string() />
                        </div>
                    }
                }}

                // Status Message
                {move || {
                    let status = status_message.get();
                    if !status.is_empty() {
                        Some(view! {
                            <div class="bg-zinc-900 text-zinc-300 px-4 py-2 rounded border border-zinc-800 flex items-center gap-2">
                                <div class="w-2 h-2 rounded-full bg-blue-500 animate-pulse"></div>
                                {status}
                            </div>
                        })
                    } else {
                        None
                    }
                }}

                // Content Area
                {move || {
                    let loading = is_loading.get();
                    let current_filter = filter.get();
                    let current_view = view_mode.get();

                    // Get the appropriate list based on filter
                    let campaign_list: Vec<Campaign> = match current_filter {
                        CampaignFilter::Active => campaigns.get(),
                        CampaignFilter::Archived => archived_campaigns.get(),
                        CampaignFilter::All => {
                            let mut all = campaigns.get();
                            all.extend(archived_campaigns.get());
                            all
                        }
                    };

                    if loading {
                        view! {
                            <div class="flex justify-center py-20">
                                <LoadingSpinner size="lg" />
                            </div>
                        }.into_any()
                    } else if campaign_list.is_empty() {
                        let empty_message = match current_filter {
                            CampaignFilter::Active => "Your library is empty. Start your first journey into the unknown.",
                            CampaignFilter::Archived => "No archived campaigns. Campaigns you archive will appear here.",
                            CampaignFilter::All => "No campaigns found.",
                        };
                        view! {
                            <div class="text-center py-24 bg-zinc-900/50 rounded-xl border border-dashed border-zinc-800 flex flex-col items-center justify-center">
                                <div class="text-6xl mb-4 opacity-20">"..."</div>
                                <h3 class="text-2xl font-bold text-zinc-200 mb-2">"No campaigns found"</h3>
                                <p class="text-zinc-500 mb-8 max-w-md">{empty_message}</p>
                                {(current_filter != CampaignFilter::Archived).then(|| view! {
                                    <Button on_click=open_create_modal>
                                        "Create Campaign"
                                    </Button>
                                })}
                            </div>
                        }.into_any()
                    } else {
                        match current_view {
                            CampaignViewMode::Grid => view! {
                                <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
                                    {campaign_list.into_iter().map(|campaign| {
                                        view! {
                                            <CampaignCard
                                                campaign=campaign
                                                on_delete=handle_delete_request
                                            />
                                        }
                                    }).collect_view()}
                                </div>
                            }.into_any(),
                            CampaignViewMode::List => view! {
                                <div class="space-y-2">
                                    {campaign_list.into_iter().map(|campaign| {
                                        let campaign_id = campaign.id.clone();
                                        let campaign_name = campaign.name.clone();
                                        let campaign_system = campaign.system.clone();
                                        let delete_id = campaign.id.clone();
                                        let delete_name = campaign.name.clone();
                                        let nav = navigate.clone();

                                        let handle_click = move |_: ev::MouseEvent| {
                                            let nav = nav.clone();
                                            let id = campaign_id.clone();
                                            nav(&format!("/session/{}", id), Default::default());
                                        };

                                        let handle_delete = move |evt: ev::MouseEvent| {
                                            evt.stop_propagation();
                                            handle_delete_request.run((delete_id.clone(), delete_name.clone()));
                                        };

                                        view! {
                                            <div
                                                class="flex items-center justify-between p-4 bg-zinc-900 border border-zinc-800 rounded-lg hover:border-zinc-700 cursor-pointer transition-colors"
                                                on:click=handle_click
                                            >
                                                <div class="flex items-center gap-4">
                                                    <div class="w-10 h-10 rounded-lg bg-zinc-800 flex items-center justify-center text-lg font-bold text-zinc-400">
                                                        {campaign_name.chars().next().unwrap_or('?').to_string()}
                                                    </div>
                                                    <div>
                                                        <div class="font-medium text-white">{campaign_name.clone()}</div>
                                                        <div class="text-sm text-zinc-500">{campaign_system}</div>
                                                    </div>
                                                </div>
                                                <button
                                                    class="p-2 text-zinc-500 hover:text-red-400 transition-colors"
                                                    on:click=handle_delete
                                                >
                                                    "X"
                                                </button>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            }.into_any(),
                        }
                    }
                }}
            </div>

            // Quick Create Campaign Modal
            <CampaignCreateModal
                is_open=show_create_modal
                on_create=handle_create_wizard
            />

            // AI-Assisted Campaign Wizard
            <WizardShell
                is_open=show_ai_wizard
                on_create=handle_ai_wizard_create
                ai_assisted=true
            />

            // Delete Confirmation Modal
            <Show when=move || delete_confirm.get().is_some()>
                <div class="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center">
                    <div class="bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl w-full max-w-sm mx-4 p-6">
                        <h3 class="text-lg font-bold text-white mb-2">"Delete Campaign?"</h3>
                        <p class="text-zinc-400 mb-6">
                            "Are you sure you want to delete "
                            <span class="text-white font-medium">
                                {move || delete_confirm.get().map(|(_, name)| name).unwrap_or_default()}
                            </span>
                            "? This action cannot be undone."
                        </p>
                        <div class="flex justify-end gap-3">
                            <Button
                                variant=ButtonVariant::Secondary
                                on_click=handle_cancel_delete
                            >
                                "Cancel"
                            </Button>
                            <Button
                                variant=ButtonVariant::Destructive
                                on_click=handle_confirm_delete
                            >
                                "Delete"
                            </Button>
                        </div>
                    </div>
                </div>
            </Show>

            // Archive/Restore Confirmation Modal
            <Show when=move || archive_confirm.get().is_some()>
                <div class="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center">
                    <div class="bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl w-full max-w-sm mx-4 p-6">
                        <h3 class="text-lg font-bold text-white mb-2">
                            {move || {
                                if archive_confirm.get().map(|(_, _, is_restore)| is_restore).unwrap_or(false) {
                                    "Restore Campaign?"
                                } else {
                                    "Archive Campaign?"
                                }
                            }}
                        </h3>
                        <p class="text-zinc-400 mb-6">
                            {move || {
                                let (_, name, is_restore) = archive_confirm.get().unwrap_or_default();
                                if is_restore {
                                    format!("Restore \"{}\" to your active campaigns?", name)
                                } else {
                                    format!("Archive \"{}\"? You can restore it later from the archived view.", name)
                                }
                            }}
                        </p>
                        <div class="flex justify-end gap-3">
                            <Button
                                variant=ButtonVariant::Secondary
                                on_click=handle_cancel_archive
                            >
                                "Cancel"
                            </Button>
                            <Button
                                variant=ButtonVariant::Primary
                                on_click=handle_confirm_archive
                            >
                                {move || {
                                    if archive_confirm.get().map(|(_, _, is_restore)| is_restore).unwrap_or(false) {
                                        "Restore"
                                    } else {
                                        "Archive"
                                    }
                                }}
                            </Button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
