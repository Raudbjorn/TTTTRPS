//! Campaign Dashboard Component
//!
//! Main dashboard view for campaign management with tabs for different features.

use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::bindings::{
    Campaign, get_campaign, get_campaign_stats, CampaignStats,
    list_npcs, list_locations, list_world_events, list_campaign_versions,
    VersionSummary, LocationState, WorldEvent, NPC,
};
use super::campaign_card::CampaignCard;
use super::version_history::VersionHistory;
use super::world_state_editor::WorldStateEditor;
use super::entity_browser::EntityBrowser;
use super::relationship_graph::RelationshipGraph;

/// Dashboard tab
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum DashboardTab {
    Overview,
    Entities,
    WorldState,
    Versions,
    Relationships,
}

impl Default for DashboardTab {
    fn default() -> Self {
        Self::Overview
    }
}

impl DashboardTab {
    fn label(&self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Entities => "Entities",
            Self::WorldState => "World State",
            Self::Versions => "Versions",
            Self::Relationships => "Relationships",
        }
    }
}

/// Tab button component
#[component]
fn TabButton(
    tab: DashboardTab,
    active_tab: DashboardTab,
    on_click: Callback<DashboardTab>,
) -> impl IntoView {
    let is_active = tab == active_tab;
    let base_class = "px-4 py-2 text-sm font-medium transition-colors rounded-t-lg";
    let active_class = if is_active {
        "bg-zinc-800 text-white border-b-2 border-purple-500"
    } else {
        "text-zinc-400 hover:text-white hover:bg-zinc-800/50"
    };

    view! {
        <button
            class=format!("{} {}", base_class, active_class)
            on:click=move |_| on_click.run(tab)
        >
            {tab.label()}
        </button>
    }
}

/// Stat card for overview
#[component]
fn StatCard(
    #[prop(into)]
    label: String,
    #[prop(into)]
    value: String,
    #[prop(optional)]
    icon: Option<String>,
) -> impl IntoView {
    view! {
        <div class="bg-zinc-900 border border-zinc-800 rounded-lg p-4">
            <div class="flex items-center gap-2">
                {icon.map(|i| view! { <span class="text-zinc-500">{i}</span> })}
                <div class="text-zinc-500 text-xs font-bold uppercase tracking-wider">
                    {label}
                </div>
            </div>
            <div class="text-2xl font-bold text-zinc-100 mt-1">
                {value}
            </div>
        </div>
    }
}

/// Main campaign dashboard component
#[component]
pub fn CampaignDashboard(
    /// Campaign ID to display
    campaign_id: String,
    /// Callback when user wants to go back
    #[prop(optional)]
    on_back: Option<Callback<()>>,
) -> impl IntoView {
    // State
    let active_tab = RwSignal::new(DashboardTab::Overview);
    let campaign = RwSignal::new(Option::<Campaign>::None);
    let stats = RwSignal::new(CampaignStats::default());
    let is_loading = RwSignal::new(true);
    let error = RwSignal::new(Option::<String>::None);

    // Load campaign data
    let campaign_id_clone = campaign_id.clone();
    Effect::new(move |_| {
        let cid = campaign_id_clone.clone();
        spawn_local(async move {
            is_loading.set(true);
            error.set(None);

            // Load campaign
            match get_campaign(cid.clone()).await {
                Ok(Some(c)) => campaign.set(Some(c)),
                Ok(None) => error.set(Some("Campaign not found".to_string())),
                Err(e) => error.set(Some(e)),
            }

            // Load stats
            if let Ok(s) = get_campaign_stats(cid).await {
                stats.set(s);
            }

            is_loading.set(false);
        });
    });

    let handle_tab_change = Callback::new(move |tab: DashboardTab| {
        active_tab.set(tab);
    });

    let handle_back = move |_: ev::MouseEvent| {
        if let Some(ref callback) = on_back {
            callback.run(());
        }
    };

    let campaign_id_for_content = campaign_id.clone();

    view! {
        <div class="h-full flex flex-col bg-zinc-950">
            // Header
            <div class="border-b border-zinc-800 px-6 py-4">
                <div class="flex items-center gap-4">
                    {on_back.as_ref().map(|_| view! {
                        <button
                            class="p-2 hover:bg-zinc-800 rounded-lg text-zinc-400 hover:text-white transition-colors"
                            on:click=handle_back.clone()
                        >
                            "< Back"
                        </button>
                    })}

                    <div class="flex-1">
                        {move || campaign.get().map(|c| view! {
                            <div>
                                <h1 class="text-2xl font-bold text-white">{c.name}</h1>
                                <p class="text-sm text-zinc-400">{c.system}</p>
                            </div>
                        })}
                    </div>

                    // Quick actions
                    <div class="flex gap-2">
                        <button class="px-3 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-white text-sm rounded-lg transition-colors">
                            "Save Version"
                        </button>
                    </div>
                </div>

                // Tabs
                <div class="flex gap-1 mt-4 -mb-px">
                    <TabButton tab=DashboardTab::Overview active_tab=active_tab.get() on_click=handle_tab_change />
                    <TabButton tab=DashboardTab::Entities active_tab=active_tab.get() on_click=handle_tab_change />
                    <TabButton tab=DashboardTab::WorldState active_tab=active_tab.get() on_click=handle_tab_change />
                    <TabButton tab=DashboardTab::Versions active_tab=active_tab.get() on_click=handle_tab_change />
                    <TabButton tab=DashboardTab::Relationships active_tab=active_tab.get() on_click=handle_tab_change />
                </div>
            </div>

            // Content Area
            <div class="flex-1 overflow-auto p-6">
                {move || {
                    if is_loading.get() {
                        view! {
                            <div class="flex items-center justify-center h-64">
                                <div class="text-zinc-500">"Loading..."</div>
                            </div>
                        }.into_any()
                    } else if let Some(err) = error.get() {
                        view! {
                            <div class="flex items-center justify-center h-64">
                                <div class="text-red-400">{err}</div>
                            </div>
                        }.into_any()
                    } else {
                        let cid = campaign_id_for_content.clone();
                        match active_tab.get() {
                            DashboardTab::Overview => view! {
                                <OverviewContent stats=stats.get() />
                            }.into_any(),
                            DashboardTab::Entities => view! {
                                <EntityBrowser campaign_id=cid.clone() />
                            }.into_any(),
                            DashboardTab::WorldState => view! {
                                <WorldStateEditor campaign_id=cid.clone() />
                            }.into_any(),
                            DashboardTab::Versions => view! {
                                <VersionHistory campaign_id=cid.clone() />
                            }.into_any(),
                            DashboardTab::Relationships => view! {
                                <RelationshipGraph campaign_id=cid.clone() />
                            }.into_any(),
                        }
                    }
                }}
            </div>
        </div>
    }
}

/// Overview content component
#[component]
fn OverviewContent(stats: CampaignStats) -> impl IntoView {
    view! {
        <div class="space-y-6">
            // Stats Grid
            <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                <StatCard label="Sessions" value=stats.session_count.to_string() />
                <StatCard label="NPCs" value=stats.npc_count.to_string() />
                <StatCard
                    label="Playtime"
                    value=format!("{}h {}m", stats.total_playtime_minutes / 60, stats.total_playtime_minutes % 60)
                />
                <StatCard
                    label="Last Played"
                    value=stats.last_played.clone().unwrap_or_else(|| "Never".to_string())
                />
            </div>

            // Quick Actions
            <div class="bg-zinc-900 border border-zinc-800 rounded-lg p-6">
                <h3 class="text-lg font-bold text-white mb-4">"Quick Actions"</h3>
                <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                    <button class="p-4 bg-zinc-800 hover:bg-zinc-700 rounded-lg text-left transition-colors">
                        <div class="text-white font-medium">"New Session"</div>
                        <div class="text-xs text-zinc-400">"Start playing"</div>
                    </button>
                    <button class="p-4 bg-zinc-800 hover:bg-zinc-700 rounded-lg text-left transition-colors">
                        <div class="text-white font-medium">"Add NPC"</div>
                        <div class="text-xs text-zinc-400">"Create character"</div>
                    </button>
                    <button class="p-4 bg-zinc-800 hover:bg-zinc-700 rounded-lg text-left transition-colors">
                        <div class="text-white font-medium">"World Event"</div>
                        <div class="text-xs text-zinc-400">"Record history"</div>
                    </button>
                    <button class="p-4 bg-zinc-800 hover:bg-zinc-700 rounded-lg text-left transition-colors">
                        <div class="text-white font-medium">"Export"</div>
                        <div class="text-xs text-zinc-400">"Backup data"</div>
                    </button>
                </div>
            </div>

            // Recent Activity Placeholder
            <div class="bg-zinc-900 border border-zinc-800 rounded-lg p-6">
                <h3 class="text-lg font-bold text-white mb-4">"Recent Activity"</h3>
                <div class="text-zinc-500 text-sm">"No recent activity to show."</div>
            </div>
        </div>
    }
}
