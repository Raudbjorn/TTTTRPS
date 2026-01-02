#![allow(non_snake_case)]
use dioxus::prelude::*;
use crate::bindings::{list_campaigns, create_campaign, delete_campaign, Campaign};
use crate::components::design_system::{Button, ButtonVariant, Input, Select, Badge, BadgeVariant, Modal, LoadingSpinner};

#[component]
pub fn Campaigns() -> Element {
    let mut campaigns = use_signal(|| Vec::<Campaign>::new());
    let mut status_message = use_signal(|| String::new());
    let mut show_create_modal = use_signal(|| false);
    let mut new_campaign_name = use_signal(|| String::new());
    let mut new_campaign_system = use_signal(|| "D&D 5e".to_string());
    let mut is_loading = use_signal(|| true);

    // Load campaigns on mount
    use_effect(move || {
        spawn(async move {
            match list_campaigns().await {
                Ok(list) => {
                    campaigns.set(list);
                }
                Err(e) => {
                    status_message.set(format!("Failed to load campaigns: {}", e));
                }
            }
            is_loading.set(false);
        });
    });

    let refresh_campaigns = move |_: MouseEvent| {
        is_loading.set(true);
        spawn(async move {
            match list_campaigns().await {
                Ok(list) => {
                    campaigns.set(list);
                    status_message.set("Refreshed".to_string());
                }
                Err(e) => {
                    status_message.set(format!("Error: {}", e));
                }
            }
            is_loading.set(false);
        });
    };

    let open_create_modal = move |_: MouseEvent| {
        show_create_modal.set(true);
        new_campaign_name.set(String::new());
        new_campaign_system.set("D&D 5e".to_string());
    };

    let handle_create = move |_: MouseEvent| {
        let name = new_campaign_name.read().clone();
        let system = new_campaign_system.read().clone();

        if name.trim().is_empty() {
            status_message.set("Campaign name is required".to_string());
            return;
        }

        spawn(async move {
            match create_campaign(name, system).await {
                Ok(campaign) => {
                    campaigns.write().push(campaign);
                    show_create_modal.set(false);
                    status_message.set("Campaign created!".to_string());
                }
                Err(e) => {
                    status_message.set(format!("Error: {}", e));
                }
            }
        });
    };

    let handle_delete = move |id: String, name: String| {
        move |evt: MouseEvent| {
            evt.stop_propagation(); // Prevent opening the campaign when clicking delete
            let id = id.clone();
            let name = name.clone();
            spawn(async move {
                match delete_campaign(id.clone()).await {
                    Ok(_) => {
                        campaigns.write().retain(|c| c.id != id);
                        status_message.set(format!("Deleted campaign: {}", name));
                    }
                    Err(e) => {
                        status_message.set(format!("Error deleting: {}", e));
                    }
                }
            });
        }
    };

    let mut view_mode = use_signal(|| "grid"); // "grid" or "list"

    let loading = *is_loading.read();
    let status = status_message.read().clone();
    let modal_open = *show_create_modal.read();

    let total_campaigns = campaigns.read().len();
    // Stats temporarily disabled or need fetching
    let total_sessions = 0;
    let total_players = 0;

    // Helper to get system color/initials
    let get_system_style = |system: &str| -> (&'static str, &'static str) {
        let s = system.to_lowercase();
        if s.contains("d&d") || s.contains("5e") || s.contains("pathfinder") {
            ("bg-gradient-to-br from-amber-700 to-amber-900", "text-amber-200")
        } else if s.contains("cthulhu") || s.contains("horror") || s.contains("vampire") {
             ("bg-gradient-to-br from-slate-800 to-black", "text-red-400")
        } else if s.contains("cyber") || s.contains("shadow") || s.contains("neon") {
             ("bg-gradient-to-br from-fuchsia-900 to-purple-900", "text-neon-pink")
        } else if s.contains("space") || s.contains("alien") || s.contains("scifi") {
             ("bg-gradient-to-br from-cyan-900 to-blue-900", "text-cyan-200")
        } else {
             ("bg-gradient-to-br from-zinc-700 to-zinc-900", "text-zinc-300")
        }
    };

    rsx! {
        div {
            class: "p-8 bg-zinc-950 text-zinc-100 min-h-screen font-sans selection:bg-purple-500/30",
            div {
                class: "max-w-7xl mx-auto space-y-8",

                // Header & Quick Actions
                div {
                    class: "flex flex-col md:flex-row md:items-end justify-between gap-4 pb-6 border-b border-zinc-900",
                    div {
                        class: "space-y-1",
                        Link { to: crate::Route::Chat {}, class: "text-zinc-500 hover:text-white transition-colors text-sm font-medium", "← Back to Hub" }
                        h1 { class: "text-4xl font-extrabold tracking-tight bg-clip-text text-transparent bg-gradient-to-r from-white to-zinc-500", "Campaigns" }
                        p { class: "text-zinc-400", "Manage your ongoing adventures and chronicles." }
                    }
                    div {
                        class: "flex gap-3",
                        // View Toggle
                        div { class: "flex bg-zinc-900 rounded-lg p-1 border border-zinc-800",
                            button {
                                class: format!("px-3 py-1 rounded transition-colors {}", if *view_mode.read() == "grid" { "bg-zinc-700 text-white" } else { "text-zinc-500 hover:text-zinc-300" }),
                                onclick: move |_| view_mode.set("grid"),
                                "Grid"
                            }
                            button {
                                class: format!("px-3 py-1 rounded transition-colors {}", if *view_mode.read() == "list" { "bg-zinc-700 text-white" } else { "text-zinc-500 hover:text-zinc-300" }),
                                onclick: move |_| view_mode.set("list"),
                                "List"
                            }
                        }
                         Button {
                            variant: ButtonVariant::Secondary,
                            onclick: refresh_campaigns,
                            "↻"
                        }
                        Button {
                            variant: ButtonVariant::Primary,
                            onclick: open_create_modal,
                            "+ New Adventure"
                        }
                    }
                }

                // Stats Row
                 div {
                    class: "grid grid-cols-2 md:grid-cols-4 gap-4",
                    StatCard { label: "Campaigns", value: total_campaigns.to_string() }
                    StatCard { label: "Total Sessions", value: total_sessions.to_string() }
                    StatCard { label: "Active Players", value: total_players.to_string() }
                }

                // Status Message
                if !status.is_empty() {
                     div { class: "bg-zinc-900 text-zinc-300 px-4 py-2 rounded border border-zinc-800 flex items-center gap-2",
                        div { class: "w-2 h-2 rounded-full bg-blue-500 animate-pulse" }
                        "{status}"
                    }
                }

                // Content Area
                if loading {
                    div {
                        class: "flex justify-center py-20",
                        LoadingSpinner { size: "lg" }
                    }
                } else if campaigns.read().is_empty() {
                     div {
                        class: "text-center py-24 bg-zinc-900/50 rounded-xl border border-dashed border-zinc-800 flex flex-col items-center justify-center",
                        div { class: "text-6xl mb-4 opacity-20", "📜" }
                        h3 { class: "text-2xl font-bold text-zinc-200 mb-2", "No campaigns found" }
                        p { class: "text-zinc-500 mb-8 max-w-md", "Your library is empty. Start your first journey into the unknown." }
                        Button {
                            onclick: open_create_modal,
                            "Create Campaign"
                        }
                    }
                } else if *view_mode.read() == "grid" {
                    // Album Grid View
                    div {
                        class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6",
                        for campaign in campaigns.read().iter() {
                            {
                                let (bg_class, text_class) = get_system_style(&campaign.system);
                                let initials = campaign.name.chars().next().unwrap_or('?');
                                let c_id = campaign.id.clone();
                                let c_name = campaign.name.clone();
                                let c_desc = campaign.description.clone().unwrap_or_default();
                                // let session_count = campaign.session_count; // REMOVED

                                rsx! {
                                    Link {
                                        to: crate::Route::Session { campaign_id: campaign.id.clone() },
                                        class: "group relative aspect-[3/4] bg-zinc-900 rounded-xl overflow-hidden shadow-2xl border border-zinc-800 hover:border-zinc-600 transition-all hover:-translate-y-1",

                                        // "Cover Art" Background
                                        div { class: "absolute inset-0 {bg_class} opacity-20 group-hover:opacity-30 transition-opacity" }

                                        // Content Container
                                        div { class: "relative h-full flex flex-col p-6",
                                            // Top Badge
                                            div { class: "flex justify-between items-start",
                                                Badge { variant: BadgeVariant::Outline, "{campaign.system}" }

                                                // Delete button (visible on hover)
                                                button {
                                                    class: "opacity-0 group-hover:opacity-100 p-2 text-zinc-400 hover:text-red-400 transition-all",
                                                    onclick: handle_delete(c_id.clone(), c_name.clone()),
                                                    "🗑"
                                                }
                                            }

                                            // Center Initials (Placeholder Art)
                                            div { class: "flex-1 flex items-center justify-center",
                                                 span { class: "text-8xl font-black {text_class} opacity-20 select-none group-hover:scale-110 transition-transform duration-500", "{initials}" }
                                            }

                                            // Bottom Info
                                            div { class: "space-y-2",
                                                h3 { class: "text-xl font-bold text-white leading-tight group-hover:text-purple-300 transition-colors", "{c_name}" }
                                                if !c_desc.is_empty() {
                                                    p { class: "text-xs text-zinc-400 line-clamp-2", "{c_desc}" }
                                                }

                                                // Stats Row (Disabled for now)
                                                /*
                                                div { class: "pt-4 flex items-center gap-4 text-xs font-medium text-zinc-500 border-t border-white/5",
                                                     div { class: "flex items-center gap-1",
                                                        span { "📜" }
                                                        "{session_count} Sessions"
                                                     }
                                                }
                                                */
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // List View
                    div {
                        class: "space-y-2",
                        for campaign in campaigns.read().iter() {
                            {
                                let (bg_class, text_class) = get_system_style(&campaign.system);
                                let initials = campaign.name.chars().next().unwrap_or('?');
                                let c_id = campaign.id.clone();
                                let c_name = campaign.name.clone();
                                let c_desc = campaign.description.clone().unwrap_or_default();

                                rsx! {
                                    Link {
                                        to: crate::Route::Session { campaign_id: campaign.id.clone() },
                                        class: "group flex items-center gap-4 p-4 bg-zinc-900 rounded-lg border border-zinc-800 hover:border-zinc-600 transition-all hover:bg-zinc-800/50",

                                        // Small Icon/Cover
                                        div {
                                            class: "w-12 h-12 rounded-lg {bg_class} flex items-center justify-center shrink-0",
                                            span { class: "text-xl font-bold {text_class}", "{initials}" }
                                        }

                                        // Content
                                        div { class: "flex-1 min-w-0",
                                            h3 { class: "text-lg font-bold text-white group-hover:text-purple-300 truncate", "{c_name}" }
                                            p { class: "text-sm text-zinc-500 truncate", "{c_desc}" }
                                        }

                                        // System Badge
                                        div { class: "shrink-0",
                                            Badge { variant: BadgeVariant::Outline, "{campaign.system}" }
                                        }

                                        // Delete Action
                                        button {
                                            class: "p-2 text-zinc-500 hover:text-red-400 transition-colors opacity-0 group-hover:opacity-100",
                                            onclick: handle_delete(c_id.clone(), c_name.clone()),
                                            "🗑"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Create Modal
            Modal {
                is_open: modal_open,
                onclose: move |_| show_create_modal.set(false),
                title: "Begin New Adventure",
                children: rsx! {
                    div {
                        class: "space-y-6 p-2",
                        div {
                            label { class: "block text-sm font-bold text-zinc-400 mb-2", "Campaign Name" }
                            Input {
                                placeholder: "e.g. The Tomb of Horrors",
                                value: "{new_campaign_name}",
                                oninput: move |e| new_campaign_name.set(e)
                            }
                        }
                        div {
                            label { class: "block text-sm font-bold text-zinc-400 mb-2", "Game System" }
                            Select {
                                value: "{new_campaign_system}",
                                onchange: move |e| new_campaign_system.set(e),
                                option { value: "D&D 5e", "D&D 5e" }
                                option { value: "Pathfinder 2e", "Pathfinder 2e" }
                                option { value: "Call of Cthulhu", "Call of Cthulhu" }
                                option { value: "Delta Green", "Delta Green" }
                                option { value: "Mothership", "Mothership" }
                                option { value: "Cyberpunk Red", "Cyberpunk Red" }
                                option { value: "Shadowrun", "Shadowrun" }
                                option { value: "Vampire: The Masquerade", "Vampire: The Masquerade" }
                                option { value: "Other", "Other" }
                            }
                        }
                        div {
                            class: "flex justify-end gap-3 mt-8 pt-4 border-t border-zinc-800",
                             Button {
                                variant: ButtonVariant::Secondary,
                                onclick: move |_| show_create_modal.set(false),
                                "Cancel"
                            }
                            Button {
                                onclick: handle_create,
                                "Launch Campaign"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatCard(label: String, value: String) -> Element {
    rsx! {
        div { class: "bg-zinc-900 border border-zinc-800 p-4 rounded-lg",
            div { class: "text-zinc-500 text-xs font-bold uppercase tracking-wider mb-1", "{label}" }
            div { class: "text-2xl font-bold text-zinc-100", "{value}" }
        }
    }
}
