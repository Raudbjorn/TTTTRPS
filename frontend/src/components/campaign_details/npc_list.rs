//! NPC List / Info Panel Component
//!
//! A Slack-style direct messages contact list for NPCs.
//! Features:
//!   - Search/filter NPCs
//!   - Online/away/offline status indicators
//!   - Unread message counts
//!   - Last message preview
//!   - Drag-drop support for personality assignment
//!   - Keyboard accessible

use leptos::prelude::*;
use crate::bindings::{list_npc_summaries, NpcSummary};

/// Info panel displaying NPC contacts (Slack DM style)
#[component]
pub fn InfoPanel(
    /// Campaign ID to load NPCs for
    campaign_id: String,
    /// Currently selected NPC ID
    #[prop(optional)]
    selected_npc_id: Option<String>,
    /// Callback when an NPC is selected
    #[prop(optional, into)]
    on_select_npc: Option<Callback<String>>,
    /// Callback to create a new NPC
    #[prop(optional, into)]
    on_create_npc: Option<Callback<()>>,
) -> impl IntoView {
    let campaign_id_clone = campaign_id.clone();
    let search_query = RwSignal::new(String::new());

    // Fetch NPCs from backend
    let npcs_resource = LocalResource::new(move || {
        let cid = campaign_id_clone.clone();
        async move { list_npc_summaries(cid).await.unwrap_or_default() }
    });

    // Filtered NPCs based on search
    let filtered_npcs = move || {
        let query = search_query.get().to_lowercase();
        npcs_resource.get().map(|list| {
            let all: Vec<_> = list.to_vec();
            if query.is_empty() {
                all
            } else {
                all.into_iter()
                    .filter(|npc| npc.name.to_lowercase().contains(&query))
                    .collect()
            }
        })
    };

    view! {
        <aside
            class="flex flex-col h-full bg-[var(--bg-surface)] border-l border-[var(--border-subtle)]"
            role="complementary"
            aria-label="NPC contacts"
        >
            // Header
            <header class="p-4 border-b border-[var(--border-subtle)]">
                <div class="flex justify-between items-center mb-3">
                    <h2 class="text-xs font-bold uppercase tracking-wider text-[var(--text-muted)]">
                        "Direct Messages"
                    </h2>
                    {on_create_npc.map(|cb| view! {
                        <button
                            class="w-6 h-6 rounded flex items-center justify-center text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
                            aria-label="Add new NPC"
                            title="New NPC"
                            on:click=move |_| cb.run(())
                        >
                            <PlusIcon />
                        </button>
                    })}
                </div>

                // Search
                <div class="relative">
                    <SearchIcon />
                    <input
                        type="search"
                        class="w-full bg-[var(--bg-deep)] border border-[var(--border-subtle)] rounded-lg pl-9 pr-3 py-2 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none focus:border-[var(--accent)] focus:ring-1 focus:ring-[var(--accent)] transition-colors"
                        placeholder="Find a character..."
                        prop:value=move || search_query.get()
                        on:input=move |e| search_query.set(event_target_value(&e))
                    />
                </div>
            </header>

            // NPC List
            <nav class="flex-1 overflow-y-auto">
                <Suspense fallback=move || view! {
                    <div class="p-4 space-y-3">
                        <NpcSkeleton />
                        <NpcSkeleton />
                        <NpcSkeleton />
                    </div>
                }>
                    {move || {
                        filtered_npcs().map(|list: Vec<NpcSummary>| {
                            if list.is_empty() {
                                view! {
                                    <div class="p-8 text-center">
                                        <div class="w-12 h-12 mx-auto mb-3 rounded-full bg-[var(--bg-elevated)] flex items-center justify-center text-[var(--text-muted)]">
                                            <UserIcon />
                                        </div>
                                        <p class="text-sm text-[var(--text-muted)]">
                                            {if search_query.get().is_empty() {
                                                "No NPCs in this campaign yet"
                                            } else {
                                                "No characters match your search"
                                            }}
                                        </p>
                                        {on_create_npc.map(|cb| view! {
                                            <button
                                                class="mt-3 text-sm text-[var(--accent)] hover:underline"
                                                on:click=move |_| cb.run(())
                                            >
                                                "Create first NPC"
                                            </button>
                                        })}
                                    </div>
                                }.into_any()
                            } else {
                                let selected = selected_npc_id.clone();
                                let on_click = on_select_npc.clone();
                                view! {
                                    <ul class="p-2 space-y-0.5" role="listbox" aria-label="NPC list">
                                        {list.iter().map(|npc| {
                                            let is_selected = selected.as_ref() == Some(&npc.id);
                                            let callback = on_click.clone();
                                            view! {
                                                <NpcContactItem
                                                    npc=npc.clone()
                                                    is_selected=is_selected
                                                    on_click=callback
                                                />
                                            }
                                        }).collect_view()}
                                    </ul>
                                }.into_any()
                            }
                        })
                    }}
                </Suspense>
            </nav>

            // Footer with sync status
            <footer class="p-2 border-t border-[var(--border-subtle)] text-center">
                <span class="text-[10px] text-[var(--text-muted)] flex items-center justify-center gap-1">
                    <span class="w-1.5 h-1.5 rounded-full bg-green-500"></span>
                    "Synced with Neural Link"
                </span>
            </footer>
        </aside>
    }
}

/// Individual NPC contact item
#[component]
fn NpcContactItem(
    npc: NpcSummary,
    is_selected: bool,
    on_click: Option<Callback<String>>,
) -> impl IntoView {
    let id = npc.id.clone();
    let name = npc.name.clone();
    let avatar = npc.avatar_url.clone();
    let status = npc.status.clone();
    let last_active = npc.last_active.clone();
    let last_message = npc.last_message.clone();
    let unread = npc.unread_count;

    let status_color = match status.as_str() {
        "online" => "bg-green-500",
        "away" => "bg-yellow-500",
        "busy" => "bg-red-500",
        _ => "bg-zinc-500",
    };

    let base_class = if is_selected {
        "bg-[var(--accent)]/10 ring-1 ring-inset ring-[var(--accent)]"
    } else {
        "hover:bg-[var(--bg-elevated)]"
    };

    let name_class = if unread > 0 {
        "font-bold text-[var(--text-primary)]"
    } else {
        "font-medium text-[var(--text-primary)]"
    };

    let message_class = if unread > 0 {
        "font-medium text-[var(--text-muted)]"
    } else {
        "text-[var(--text-muted)]"
    };

    view! {
        <li role="option" aria-selected=is_selected.to_string()>
            <button
                class=format!(
                    "w-full flex items-center gap-3 p-2 rounded-lg transition-colors text-left focus:outline-none focus:ring-2 focus:ring-[var(--accent)] {}",
                    base_class
                )
                on:click=move |_| {
                    if let Some(ref cb) = on_click {
                        cb.run(id.clone());
                    }
                }
                on:dragover=move |e| e.prevent_default()
                on:drop=move |e| {
                    e.prevent_default();
                    // TODO: Handle personality drag-drop assignment
                }
            >
                // Selection indicator
                {is_selected.then(|| view! {
                    <div class="absolute left-0 top-1/2 -translate-y-1/2 w-0.5 h-6 bg-[var(--accent)] rounded-r"></div>
                })}

                // Avatar with status
                <div class="relative flex-shrink-0">
                    <div class="w-10 h-10 rounded-lg bg-[var(--bg-elevated)] border border-[var(--border-subtle)] flex items-center justify-center text-sm font-bold text-[var(--text-muted)] overflow-hidden">
                        {if avatar.is_empty() {
                            view! {
                                <span>{name.chars().next().unwrap_or('?')}</span>
                            }.into_any()
                        } else {
                            view! {
                                <img src=avatar.clone() alt="" class="w-full h-full object-cover" />
                            }.into_any()
                        }}
                    </div>
                    // Status indicator
                    <div class=format!(
                        "absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full border-2 border-[var(--bg-surface)] {}",
                        status_color
                    )></div>
                </div>

                // Content
                <div class="flex-1 min-w-0">
                    <div class="flex items-baseline justify-between gap-2">
                        <span class=format!("text-sm truncate {}", name_class)>
                            {name}
                        </span>
                        {(!last_active.is_empty()).then(|| view! {
                            <span class="text-[10px] text-[var(--text-muted)] font-mono flex-shrink-0">
                                {format_time_short(&last_active)}
                            </span>
                        })}
                    </div>
                    <div class="flex items-center justify-between gap-2">
                        <p class=format!("text-xs truncate {}", message_class)>
                            {if last_message.is_empty() {
                                "No messages yet".to_string()
                            } else {
                                last_message.clone()
                            }}
                        </p>
                        {(unread > 0).then(|| view! {
                            <span class="flex-shrink-0 px-1.5 py-0.5 min-w-[1.25rem] text-center text-[10px] font-bold text-white bg-[var(--accent)] rounded-full">
                                {if unread > 99 { "99+".to_string() } else { unread.to_string() }}
                            </span>
                        })}
                    </div>
                </div>
            </button>
        </li>
    }
}

/// Loading skeleton for NPC items
#[component]
fn NpcSkeleton() -> impl IntoView {
    view! {
        <div class="flex items-center gap-3 p-2 animate-pulse">
            <div class="w-10 h-10 rounded-lg bg-[var(--bg-elevated)]"></div>
            <div class="flex-1 space-y-2">
                <div class="h-3 bg-[var(--bg-elevated)] rounded w-24"></div>
                <div class="h-2 bg-[var(--bg-elevated)] rounded w-32"></div>
            </div>
        </div>
    }
}

/// Format ISO timestamp to short time (HH:MM or "Yesterday" etc.)
fn format_time_short(iso: &str) -> String {
    if let Some(time_part) = iso.split('T').nth(1) {
        if let Some(hm) = time_part.get(0..5) {
            return hm.to_string();
        }
    }
    String::new()
}

// Backwards compatibility alias
pub use InfoPanel as NPCList;

// SVG Icon Components

#[component]
fn PlusIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="12" y1="5" x2="12" y2="19"></line>
            <line x1="5" y1="12" x2="19" y2="12"></line>
        </svg>
    }
}

#[component]
fn SearchIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--text-muted)]">
            <circle cx="11" cy="11" r="8"></circle>
            <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
        </svg>
    }
}

#[component]
fn UserIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"></path>
            <circle cx="12" cy="7" r="4"></circle>
        </svg>
    }
}
