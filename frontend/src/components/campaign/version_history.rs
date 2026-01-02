//! Version History Component
//!
//! View and manage campaign versions with diff comparison and rollback.

use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::bindings::{
    list_campaign_versions, create_campaign_version, compare_campaign_versions,
    rollback_campaign, delete_campaign_version, mark_version_milestone,
    VersionSummary, CampaignDiff, DiffStats,
};

/// Version list item component
#[component]
fn VersionListItem(
    version: VersionSummary,
    is_selected: bool,
    on_select: Callback<String>,
    #[prop(optional)]
    on_compare: Option<Callback<String>>,
) -> impl IntoView {
    let version_id = version.id.clone();
    let version_id_compare = version.id.clone();

    let handle_select = move |_: ev::MouseEvent| {
        on_select.run(version_id.clone());
    };

    let handle_compare = move |evt: ev::MouseEvent| {
        evt.stop_propagation();
        if let Some(ref cb) = on_compare {
            cb.run(version_id_compare.clone());
        }
    };

    let type_badge_class = match version.version_type.as_str() {
        "Manual" => "bg-blue-900/50 text-blue-300",
        "Auto" => "bg-zinc-800 text-zinc-400",
        "Milestone" => "bg-amber-900/50 text-amber-300",
        "PreRollback" => "bg-red-900/50 text-red-300",
        _ => "bg-zinc-800 text-zinc-400",
    };

    let selected_class = if is_selected {
        "bg-zinc-800 border-l-2 border-purple-500"
    } else {
        "hover:bg-zinc-800/50"
    };

    view! {
        <div
            class=format!("p-4 border-b border-zinc-800 cursor-pointer transition-colors {}", selected_class)
            on:click=handle_select
        >
            <div class="flex items-start justify-between gap-4">
                <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2">
                        <span class="font-medium text-white">{"v"}{version.version_number}</span>
                        <span class=format!("px-2 py-0.5 text-xs rounded {}", type_badge_class)>
                            {version.version_type.clone()}
                        </span>
                    </div>
                    <div class="text-sm text-zinc-400 mt-1 truncate">{version.description.clone()}</div>
                    <div class="text-xs text-zinc-500 mt-2">
                        {format_timestamp(&version.created_at)}
                        {version.created_by.as_ref().map(|by| format!(" by {}", by))}
                    </div>

                    // Tags
                    {if !version.tags.is_empty() {
                        Some(view! {
                            <div class="flex flex-wrap gap-1 mt-2">
                                {version.tags.iter().map(|tag| {
                                    view! {
                                        <span class="px-1.5 py-0.5 text-xs bg-zinc-700 text-zinc-300 rounded">
                                            {tag.clone()}
                                        </span>
                                    }
                                }).collect_view()}
                            </div>
                        })
                    } else {
                        None
                    }}
                </div>

                // Actions
                <div class="flex gap-2">
                    {on_compare.as_ref().map(|_| view! {
                        <button
                            class="p-1.5 text-zinc-500 hover:text-white hover:bg-zinc-700 rounded transition-colors"
                            title="Compare"
                            on:click=handle_compare.clone()
                        >
                            "~"
                        </button>
                    })}
                </div>
            </div>
        </div>
    }
}

/// Diff viewer component
#[component]
fn DiffViewer(diff: CampaignDiff) -> impl IntoView {
    view! {
        <div class="bg-zinc-900 border border-zinc-800 rounded-lg overflow-hidden">
            // Header
            <div class="p-4 border-b border-zinc-800 bg-zinc-800/50">
                <div class="flex items-center justify-between">
                    <div class="text-sm text-zinc-400">
                        {"Comparing v"}{diff.from_version_number}{" to v"}{diff.to_version_number}
                    </div>
                    <div class="flex gap-4 text-xs">
                        <span class="text-green-400">{"+"}{diff.stats.added_count}{" added"}</span>
                        <span class="text-red-400">{"-"}{diff.stats.removed_count}{" removed"}</span>
                        <span class="text-yellow-400">{"~"}{diff.stats.modified_count}{" modified"}</span>
                    </div>
                </div>
            </div>

            // Changes
            <div class="max-h-80 overflow-y-auto">
                {if diff.changes.is_empty() {
                    view! {
                        <div class="p-4 text-center text-zinc-500">"No changes"</div>
                    }.into_any()
                } else {
                    view! {
                        <div class="divide-y divide-zinc-800">
                            {diff.changes.iter().map(|change| {
                                let op_class = match change.operation.as_str() {
                                    "Added" => "text-green-400 bg-green-900/20",
                                    "Removed" => "text-red-400 bg-red-900/20",
                                    "Modified" => "text-yellow-400 bg-yellow-900/20",
                                    _ => "text-zinc-400",
                                };
                                let op_symbol = match change.operation.as_str() {
                                    "Added" => "+",
                                    "Removed" => "-",
                                    "Modified" => "~",
                                    _ => "?",
                                };

                                view! {
                                    <div class=format!("p-3 {}", op_class)>
                                        <div class="flex items-center gap-2">
                                            <span class="font-mono text-sm">{op_symbol}</span>
                                            <span class="text-sm font-medium">{change.path.clone()}</span>
                                        </div>
                                        {change.old_value.as_ref().map(|v| view! {
                                            <div class="text-xs text-red-300 mt-1 font-mono line-through">
                                                {format!("{}", v)}
                                            </div>
                                        })}
                                        {change.new_value.as_ref().map(|v| view! {
                                            <div class="text-xs text-green-300 mt-1 font-mono">
                                                {format!("{}", v)}
                                            </div>
                                        })}
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    }.into_any()
                }}
            </div>
        </div>
    }
}

/// Create version modal
#[component]
fn CreateVersionModal(
    is_open: RwSignal<bool>,
    on_create: Callback<(String, String)>,
) -> impl IntoView {
    let description = RwSignal::new(String::new());
    let version_type = RwSignal::new("manual".to_string());

    let handle_create = move |_: ev::MouseEvent| {
        let desc = description.get();
        let vtype = version_type.get();
        if !desc.trim().is_empty() {
            on_create.run((desc, vtype));
            description.set(String::new());
            is_open.set(false);
        }
    };

    let handle_close = move |_: ev::MouseEvent| {
        is_open.set(false);
    };

    view! {
        <Show when=move || is_open.get()>
            <div
                class="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center"
                on:click=handle_close.clone()
            >
                <div
                    class="bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl w-full max-w-md mx-4"
                    on:click=move |evt: ev::MouseEvent| evt.stop_propagation()
                >
                    // Header
                    <div class="px-6 py-4 border-b border-zinc-800">
                        <h2 class="text-xl font-bold text-white">"Create Version Snapshot"</h2>
                    </div>

                    // Body
                    <div class="p-6 space-y-4">
                        <div>
                            <label class="block text-sm font-medium text-zinc-400 mb-2">
                                "Description"
                            </label>
                            <input
                                type="text"
                                class="w-full px-4 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                                placeholder="What changed?"
                                prop:value=move || description.get()
                                on:input=move |evt| description.set(event_target_value(&evt))
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-zinc-400 mb-2">
                                "Version Type"
                            </label>
                            <select
                                class="w-full px-4 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white focus:border-purple-500 focus:outline-none"
                                prop:value=move || version_type.get()
                                on:change=move |evt| version_type.set(event_target_value(&evt))
                            >
                                <option value="manual">"Manual"</option>
                                <option value="milestone">"Milestone"</option>
                            </select>
                        </div>
                    </div>

                    // Footer
                    <div class="px-6 py-4 border-t border-zinc-800 flex justify-end gap-3">
                        <button
                            class="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-white rounded-lg transition-colors"
                            on:click=handle_close
                        >
                            "Cancel"
                        </button>
                        <button
                            class="px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white rounded-lg transition-colors"
                            on:click=handle_create
                        >
                            "Create Version"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}

/// Main version history component
#[component]
pub fn VersionHistory(
    /// Campaign ID
    campaign_id: String,
) -> impl IntoView {
    // State
    let versions = RwSignal::new(Vec::<VersionSummary>::new());
    let selected_version = RwSignal::new(Option::<String>::None);
    let compare_version = RwSignal::new(Option::<String>::None);
    let diff_result = RwSignal::new(Option::<CampaignDiff>::None);
    let is_loading = RwSignal::new(true);
    let show_create_modal = RwSignal::new(false);
    let status_message = RwSignal::new(Option::<String>::None);

    // Load versions
    let campaign_id_load = campaign_id.clone();
    Effect::new(move |_| {
        let cid = campaign_id_load.clone();
        spawn_local(async move {
            is_loading.set(true);
            if let Ok(list) = list_campaign_versions(cid).await {
                versions.set(list);
            }
            is_loading.set(false);
        });
    });

    let handle_select = Callback::new(move |id: String| {
        if selected_version.get() == Some(id.clone()) {
            selected_version.set(None);
        } else {
            selected_version.set(Some(id));
        }
        diff_result.set(None);
    });

    let campaign_id_compare = campaign_id.clone();
    let handle_compare = Callback::new(move |target_id: String| {
        if let Some(from_id) = selected_version.get() {
            let cid = campaign_id_compare.clone();
            let from = from_id.clone();
            let to = target_id.clone();
            spawn_local(async move {
                if let Ok(diff) = compare_campaign_versions(cid, from, to).await {
                    diff_result.set(Some(diff));
                }
            });
        } else {
            compare_version.set(Some(target_id));
        }
    });

    let campaign_id_create = campaign_id.clone();
    let handle_create = Callback::new(move |(desc, vtype): (String, String)| {
        let cid = campaign_id_create.clone();
        spawn_local(async move {
            match create_campaign_version(cid.clone(), desc, vtype).await {
                Ok(new_version) => {
                    versions.update(|v| v.insert(0, new_version));
                    status_message.set(Some("Version created".to_string()));
                }
                Err(e) => {
                    status_message.set(Some(format!("Error: {}", e)));
                }
            }
        });
    });

    view! {
        <div class="space-y-4">
            // Header
            <div class="flex items-center justify-between">
                <div>
                    <h3 class="text-lg font-bold text-white">"Version History"</h3>
                    <p class="text-sm text-zinc-500">"Track changes and restore previous states"</p>
                </div>
                <button
                    class="px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white rounded-lg transition-colors"
                    on:click=move |_| show_create_modal.set(true)
                >
                    "+ Create Version"
                </button>
            </div>

            // Status message
            {move || status_message.get().map(|msg| view! {
                <div class="px-4 py-2 bg-zinc-800 text-zinc-300 rounded-lg text-sm">
                    {msg}
                </div>
            })}

            // Content
            <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
                // Version list
                <div class="bg-zinc-900 border border-zinc-800 rounded-lg overflow-hidden">
                    {move || {
                        if is_loading.get() {
                            view! {
                                <div class="p-8 text-center text-zinc-500">"Loading..."</div>
                            }.into_any()
                        } else if versions.get().is_empty() {
                            view! {
                                <div class="p-8 text-center">
                                    <div class="text-zinc-500 mb-4">"No versions yet"</div>
                                    <button
                                        class="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-white rounded-lg transition-colors"
                                        on:click=move |_| show_create_modal.set(true)
                                    >
                                        "Create First Version"
                                    </button>
                                </div>
                            }.into_any()
                        } else {
                            let selected = selected_version.get();
                            view! {
                                <div class="max-h-96 overflow-y-auto">
                                    {versions.get().into_iter().map(|v| {
                                        let is_selected = selected.as_ref() == Some(&v.id);
                                        view! {
                                            <VersionListItem
                                                version=v
                                                is_selected=is_selected
                                                on_select=handle_select
                                                on_compare=Some(handle_compare)
                                            />
                                        }
                                    }).collect_view()}
                                </div>
                            }.into_any()
                        }
                    }}
                </div>

                // Diff viewer or details
                <div>
                    {move || {
                        if let Some(diff) = diff_result.get() {
                            view! { <DiffViewer diff=diff /> }.into_any()
                        } else if let Some(id) = selected_version.get() {
                            // Version details
                            let version = versions.get().into_iter().find(|v| v.id == id);
                            if let Some(v) = version {
                                view! {
                                    <div class="bg-zinc-900 border border-zinc-800 rounded-lg p-6">
                                        <h4 class="text-lg font-bold text-white mb-4">
                                            {"Version "}{v.version_number}
                                        </h4>
                                        <div class="space-y-3 text-sm">
                                            <div>
                                                <span class="text-zinc-500">"Description: "</span>
                                                <span class="text-white">{v.description.clone()}</span>
                                            </div>
                                            <div>
                                                <span class="text-zinc-500">"Type: "</span>
                                                <span class="text-white">{v.version_type.clone()}</span>
                                            </div>
                                            <div>
                                                <span class="text-zinc-500">"Created: "</span>
                                                <span class="text-white">{format_timestamp(&v.created_at)}</span>
                                            </div>
                                            <div>
                                                <span class="text-zinc-500">"Size: "</span>
                                                <span class="text-white">{format_bytes(v.size_bytes)}</span>
                                            </div>
                                        </div>
                                        <div class="mt-6 flex gap-2">
                                            <button class="px-4 py-2 bg-amber-600 hover:bg-amber-500 text-white rounded-lg transition-colors text-sm">
                                                "Rollback to This"
                                            </button>
                                            <button class="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-white rounded-lg transition-colors text-sm">
                                                "Export"
                                            </button>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="bg-zinc-900 border border-zinc-800 rounded-lg p-8 text-center text-zinc-500">
                                        "Version not found"
                                    </div>
                                }.into_any()
                            }
                        } else {
                            view! {
                                <div class="bg-zinc-900 border border-zinc-800 rounded-lg p-8 text-center text-zinc-500">
                                    "Select a version to view details or compare"
                                </div>
                            }.into_any()
                        }
                    }}
                </div>
            </div>

            // Create modal
            <CreateVersionModal
                is_open=show_create_modal
                on_create=handle_create
            />
        </div>
    }
}

/// Format timestamp to readable string
fn format_timestamp(iso: &str) -> String {
    // Simple formatting - just extract date part
    if let Some(date_part) = iso.split('T').next() {
        date_part.to_string()
    } else {
        iso.to_string()
    }
}

/// Format bytes to human readable
fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
