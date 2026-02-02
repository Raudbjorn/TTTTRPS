//! Thread Tabs Component
//!
//! Phase 8: Conversation thread tabs for session workspace.
//! Displays tabs for different conversation purposes (session planning, NPC gen, etc.)
//! with a persistent "General" tab and ability to create new purpose-driven threads.

use leptos::prelude::*;
use leptos::ev;
use wasm_bindgen_futures::spawn_local;

use crate::bindings::{
    ConversationThread, ConversationPurpose, ThreadListOptions,
    list_conversation_threads, create_conversation_thread,
};
use crate::services::notification_service::show_error;

/// Thread tabs component for conversation management
#[component]
pub fn ThreadTabs(
    /// Campaign ID to filter threads (optional for global chat)
    #[prop(into)]
    campaign_id: Signal<Option<String>>,
    /// Currently selected thread ID
    selected_thread_id: RwSignal<Option<String>>,
    /// Callback when a thread is selected
    on_select: Callback<Option<ConversationThread>>,
) -> impl IntoView {
    // Threads list
    let threads = RwSignal::new(Vec::<ConversationThread>::new());
    let is_loading = RwSignal::new(true);
    let show_new_thread_menu = RwSignal::new(false);

    // Load threads when campaign changes
    Effect::new(move |_| {
        let cid = campaign_id.get();
        is_loading.set(true);

        spawn_local(async move {
            let options = ThreadListOptions {
                campaign_id: cid,
                purpose: None,
                include_archived: false,
                limit: 50,
            };

            match list_conversation_threads(options).await {
                Ok(thread_list) => {
                    threads.set(thread_list);
                }
                Err(e) => {
                    log::error!("Failed to load conversation threads: {}", e);
                }
            }
            is_loading.set(false);
        });
    });

    // Create new thread handler
    let create_thread = move |purpose: ConversationPurpose| {
        let cid = campaign_id.get();
        show_new_thread_menu.set(false);

        spawn_local(async move {
            match create_conversation_thread(
                cid,
                purpose.as_str().to_string(),
                None,
            ).await {
                Ok(new_thread) => {
                    // Add to list and select it
                    threads.update(|t| t.insert(0, new_thread.clone()));
                    selected_thread_id.set(Some(new_thread.id.clone()));
                    on_select.run(Some(new_thread));
                }
                Err(e) => {
                    log::error!("Failed to create thread: {}", e);
                    show_error("Failed to Create Thread", Some(&e), None);
                }
            }
        });
    };

    // Select thread handler
    let select_thread = move |thread: Option<ConversationThread>| {
        selected_thread_id.set(thread.as_ref().map(|t| t.id.clone()));
        on_select.run(thread);
    };

    view! {
        <div class="flex items-center gap-1 px-2 py-1.5 bg-zinc-900/50 border-b border-zinc-800 overflow-x-auto">
            // General (always-present) tab
            <TabButton
                label="General".to_string()
                icon="💬".to_string()
                is_selected=Signal::derive(move || selected_thread_id.get().is_none())
                on_click=Callback::new(move |_: ev::MouseEvent| {
                    select_thread(None);
                })
            />

            // Loading indicator
            <Show when=move || is_loading.get()>
                <div class="px-2 text-xs text-zinc-500 animate-pulse">
                    "..."
                </div>
            </Show>

            // Thread tabs
            <For
                each=move || threads.get()
                key=|thread| thread.id.clone()
                children=move |thread: ConversationThread| {
                    let thread_for_click = thread.clone();
                    let thread_id = thread.id.clone();
                    view! {
                        <TabButton
                            label=thread.display_title()
                            icon=thread.purpose.icon().to_string()
                            is_selected=Signal::derive(move || {
                                selected_thread_id.get() == Some(thread_id.clone())
                            })
                            on_click=Callback::new(move |_: ev::MouseEvent| {
                                select_thread(Some(thread_for_click.clone()));
                            })
                        />
                    }
                }
            />

            // New thread button
            <div class="relative ml-auto">
                <button
                    type="button"
                    class="flex items-center gap-1 px-2 py-1 text-xs text-zinc-400 hover:text-white hover:bg-zinc-800 rounded transition-colors"
                    on:click=move |_| show_new_thread_menu.update(|v| *v = !*v)
                >
                    <span class="text-sm">+</span>
                    <span class="hidden sm:inline">"New Thread"</span>
                </button>

                // Dropdown menu for thread purposes
                <Show when=move || show_new_thread_menu.get()>
                    <NewThreadMenu
                        on_select=Callback::new(move |purpose| create_thread(purpose))
                        on_close=Callback::new(move |_: ()| show_new_thread_menu.set(false))
                    />
                </Show>
            </div>
        </div>
    }
}

/// Individual tab button
#[component]
fn TabButton(
    /// Tab label text
    label: String,
    /// Tab icon (emoji)
    icon: String,
    /// Whether this tab is selected
    #[prop(into)]
    is_selected: Signal<bool>,
    /// Click handler
    on_click: Callback<ev::MouseEvent>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class=move || format!(
                "flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md transition-all {}",
                if is_selected.get() {
                    "bg-purple-600/20 text-purple-300 border border-purple-500/30"
                } else {
                    "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800"
                }
            )
            on:click=move |e| on_click.run(e)
        >
            <span>{icon}</span>
            <span class="max-w-[120px] truncate">{label}</span>
        </button>
    }
}

/// Dropdown menu for creating new threads
#[component]
fn NewThreadMenu(
    /// Callback when a purpose is selected
    on_select: Callback<ConversationPurpose>,
    /// Callback to close the menu
    on_close: Callback<()>,
) -> impl IntoView {
    let purposes = [
        ConversationPurpose::SessionPlanning,
        ConversationPurpose::NpcGeneration,
        ConversationPurpose::WorldBuilding,
        ConversationPurpose::CharacterBackground,
        ConversationPurpose::CampaignCreation,
    ];

    view! {
        // Backdrop to close menu on click outside
        <div
            class="fixed inset-0 z-40"
            on:click=move |_| on_close.run(())
        />

        <div class="absolute right-0 top-full mt-1 w-48 bg-zinc-800 border border-zinc-700 rounded-lg shadow-xl z-50 overflow-hidden">
            <div class="py-1">
                {purposes.into_iter().map(|purpose| {
                    let purpose_clone = purpose;
                    view! {
                        <button
                            type="button"
                            class="w-full flex items-center gap-2 px-3 py-2 text-sm text-zinc-300 hover:bg-zinc-700 hover:text-white transition-colors text-left"
                            on:click=move |_| on_select.run(purpose_clone)
                        >
                            <span>{purpose.icon()}</span>
                            <span>{purpose.display_name()}</span>
                        </button>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

/// Compact thread indicator for mobile/small screens
#[component]
pub fn ThreadIndicator(
    /// Currently selected thread
    #[prop(into)]
    thread: Signal<Option<ConversationThread>>,
    /// Click handler to open thread selector
    on_click: Callback<ev::MouseEvent>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class="flex items-center gap-1.5 px-2 py-1 text-xs bg-zinc-800 hover:bg-zinc-700 rounded transition-colors"
            on:click=move |e| on_click.run(e)
        >
            {move || {
                match thread.get() {
                    Some(t) => view! {
                        <span>{t.purpose.icon()}</span>
                        <span class="max-w-[80px] truncate">{t.display_title()}</span>
                    }.into_any(),
                    None => view! {
                        <span>"💬"</span>
                        <span>"General"</span>
                    }.into_any(),
                }
            }}
            <span class="text-zinc-500">"▼"</span>
        </button>
    }
}
