//! Conversation List Component
//!
//! Phase 10: Displays all conversation threads for a campaign.
//! Used in the right sidebar of the session workspace.

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::bindings::{
    ConversationThread, ConversationPurpose, ThreadListOptions,
    list_conversation_threads,
};

/// Conversation list for the session sidebar
#[component]
pub fn ConversationList(
    /// Campaign ID to filter conversations
    #[prop(into)]
    campaign_id: Signal<String>,
    /// Selected conversation ID
    #[prop(into)]
    selected_id: Signal<Option<String>>,
    /// Callback when a conversation is selected
    on_select: Callback<ConversationThread>,
) -> impl IntoView {
    let threads = RwSignal::new(Vec::<ConversationThread>::new());
    let is_loading = RwSignal::new(true);
    let search_query = RwSignal::new(String::new());

    // Load threads when campaign changes
    Effect::new(move |_| {
        let cid = campaign_id.get();
        if cid.is_empty() {
            threads.set(Vec::new());
            is_loading.set(false);
            return;
        }

        is_loading.set(true);

        spawn_local(async move {
            let options = ThreadListOptions {
                campaign_id: Some(cid),
                purpose: None,
                include_archived: false,
                limit: 50,
            };

            match list_conversation_threads(options).await {
                Ok(list) => threads.set(list),
                Err(e) => log::error!("Failed to load conversations: {}", e),
            }
            is_loading.set(false);
        });
    });

    // Filter threads by search query
    let filtered_threads = Memo::new(move |_| {
        let query = search_query.get().to_lowercase();
        let all_threads = threads.get();

        if query.is_empty() {
            all_threads
        } else {
            all_threads
                .into_iter()
                .filter(|t| {
                    t.display_title().to_lowercase().contains(&query)
                        || t.purpose.display_name().to_lowercase().contains(&query)
                })
                .collect()
        }
    });

    // Group threads by purpose
    let grouped_threads = Memo::new(move |_| {
        let filtered = filtered_threads.get();
        let mut groups: Vec<(ConversationPurpose, Vec<ConversationThread>)> = Vec::new();

        let purposes = [
            ConversationPurpose::SessionPlanning,
            ConversationPurpose::NpcGeneration,
            ConversationPurpose::WorldBuilding,
            ConversationPurpose::CharacterBackground,
            ConversationPurpose::CampaignCreation,
            ConversationPurpose::General,
        ];

        for purpose in purposes {
            let purpose_threads: Vec<ConversationThread> = filtered
                .iter()
                .filter(|t| t.purpose == purpose)
                .cloned()
                .collect();

            if !purpose_threads.is_empty() {
                groups.push((purpose, purpose_threads));
            }
        }

        groups
    });

    view! {
        <div class="flex flex-col h-full bg-zinc-900 border-l border-zinc-800">
            // Header
            <div class="p-3 border-b border-zinc-800">
                <h3 class="text-sm font-semibold text-zinc-300 mb-2">"Conversations"</h3>
                <input
                    type="text"
                    placeholder="Search threads..."
                    class="w-full px-3 py-1.5 text-sm bg-zinc-800 border border-zinc-700 rounded text-zinc-300 placeholder-zinc-500 focus:outline-none focus:border-purple-500"
                    prop:value=move || search_query.get()
                    on:input=move |ev| search_query.set(event_target_value(&ev))
                />
            </div>

            // Thread list
            <div class="flex-1 overflow-y-auto">
                <Show
                    when=move || !is_loading.get()
                    fallback=|| view! {
                        <div class="flex items-center justify-center py-8 text-zinc-500">
                            <div class="w-4 h-4 border-2 border-zinc-600 border-t-transparent rounded-full animate-spin mr-2"></div>
                            "Loading..."
                        </div>
                    }
                >
                    <Show
                        when=move || !filtered_threads.get().is_empty()
                        fallback=|| view! {
                            <div class="text-center py-8 text-zinc-500 text-sm">
                                "No conversations yet"
                            </div>
                        }
                    >
                        <For
                            each=move || grouped_threads.get()
                            key=|(purpose, _)| *purpose as u8
                            children=move |(purpose, purpose_threads)| {
                                view! {
                                    <div class="border-b border-zinc-800 last:border-b-0">
                                        // Purpose header
                                        <div class="px-3 py-2 text-xs font-medium text-zinc-500 uppercase tracking-wider bg-zinc-900/50 flex items-center gap-1.5">
                                            <span>{purpose.icon()}</span>
                                            <span>{purpose.display_name()}</span>
                                            <span class="ml-auto text-zinc-600">{purpose_threads.len()}</span>
                                        </div>

                                        // Threads in this purpose
                                        <For
                                            each=move || purpose_threads.clone()
                                            key=|thread| thread.id.clone()
                                            children=move |thread: ConversationThread| {
                                                let thread_for_click = thread.clone();
                                                let thread_id = thread.id.clone();
                                                view! {
                                                    <ConversationRow
                                                        thread=thread
                                                        is_selected=Signal::derive(move || {
                                                            selected_id.get() == Some(thread_id.clone())
                                                        })
                                                        on_click=Callback::new(move |_: ()| {
                                                            on_select.run(thread_for_click.clone());
                                                        })
                                                    />
                                                }
                                            }
                                        />
                                    </div>
                                }
                            }
                        />
                    </Show>
                </Show>
            </div>
        </div>
    }
}

/// Individual conversation row
#[component]
fn ConversationRow(
    thread: ConversationThread,
    #[prop(into)]
    is_selected: Signal<bool>,
    on_click: Callback<()>,
) -> impl IntoView {
    let title = thread.display_title();
    let message_count = thread.message_count;
    let updated_at = format_relative_time(&thread.updated_at);

    view! {
        <button
            type="button"
            class=move || format!(
                "w-full text-left px-3 py-2 hover:bg-zinc-800 transition-colors {}",
                if is_selected.get() { "bg-purple-900/20 border-l-2 border-purple-500" } else { "" }
            )
            on:click=move |_| on_click.run(())
        >
            <div class="flex items-center justify-between">
                <span class=move || format!(
                    "text-sm truncate {}",
                    if is_selected.get() { "text-purple-300 font-medium" } else { "text-zinc-300" }
                )>
                    {title}
                </span>
                <span class="text-xs text-zinc-600 ml-2 flex-shrink-0">
                    {message_count}
                </span>
            </div>
            <div class="text-xs text-zinc-500 mt-0.5">
                {updated_at}
            </div>
        </button>
    }
}

/// Format timestamp to relative time
fn format_relative_time(iso: &str) -> String {
    // Simple implementation - in production would use proper date parsing
    if iso.is_empty() {
        return String::new();
    }

    // Just show the date part for now
    if let Some(date) = iso.split('T').next() {
        date.to_string()
    } else {
        iso.to_string()
    }
}
