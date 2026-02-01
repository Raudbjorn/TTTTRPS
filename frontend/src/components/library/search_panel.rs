//! Search Panel Component
//!
//! Advanced search interface with:
//! - Hybrid search input with autocomplete
//! - Source type filter pills
//! - Advanced options (semantic/keyword weights)
//! - Search suggestions and query hints
//! - Search history

use leptos::prelude::*;
use leptos::ev;
use leptos::task::spawn_local;

use crate::bindings::{
    hybrid_search, HybridSearchOptions, get_search_suggestions,
};
use crate::components::design_system::{Button, ButtonVariant};
use super::{use_library_state, SourceType, SearchResult, SearchMeta};

/// Advanced search panel with filters and suggestions
#[component]
pub fn SearchPanel() -> impl IntoView {
    let state = use_library_state();

    // Local state for suggestions dropdown
    let suggestions = RwSignal::new(Vec::<String>::new());
    let show_suggestions = RwSignal::new(false);
    let suggestion_index = RwSignal::new(0_i32);

    // Debounced search suggestions
    let fetch_suggestions = move |query: String| {
        if query.len() < 2 {
            suggestions.set(Vec::new());
            show_suggestions.set(false);
            return;
        }

        spawn_local(async move {
            match get_search_suggestions(query).await {
                Ok(suggs) => {
                    suggestions.set(suggs);
                    show_suggestions.set(true);
                    suggestion_index.set(0);
                }
                Err(_) => {
                    suggestions.set(Vec::new());
                }
            }
        });
    };

    // Perform the actual search
    let perform_search = {
        let search_query = state.search_query;
        let is_searching = state.is_searching;
        let search_results = state.search_results;
        let search_meta = state.search_meta;
        let search_hints = state.search_hints;
        let ingestion_status = state.ingestion_status;
        let selected_source_type = state.selected_source_type;
        let semantic_weight = state.semantic_weight;
        let keyword_weight = state.keyword_weight;

        move || {
            let query = search_query.get();
            if query.is_empty() {
                return;
            }

            is_searching.set(true);
            show_suggestions.set(false);
            ingestion_status.set(format!("Searching: '{}'...", query));

            let source_type = selected_source_type.get();
            let sem_weight = semantic_weight.get();
            let key_weight = keyword_weight.get();

            spawn_local(async move {
                let options = HybridSearchOptions {
                    limit: Some(25),
                    source_type: if source_type == SourceType::All {
                        None
                    } else {
                        Some(source_type.as_str().to_string())
                    },
                    campaign_id: None,
                    index: None,
                    semantic_weight: Some(sem_weight),
                    keyword_weight: Some(key_weight),
                };

                match hybrid_search(query.clone(), Some(options)).await {
                    Ok(response) => {
                        let results: Vec<SearchResult> = response
                            .results
                            .into_iter()
                            .map(SearchResult::from)
                            .collect();

                        let count = results.len();
                        search_results.set(results);
                        search_meta.set(SearchMeta {
                            total_hits: response.total_hits,
                            processing_time_ms: response.processing_time_ms,
                            expanded_query: response.expanded_query,
                            corrected_query: response.corrected_query,
                            hints: response.hints.clone(),
                        });
                        search_hints.set(response.hints);
                        ingestion_status.set(format!(
                            "Found {} results in {}ms",
                            count, response.processing_time_ms
                        ));
                    }
                    Err(e) => {
                        ingestion_status.set(format!("Search failed: {}", e));
                        search_results.set(Vec::new());
                    }
                }
                is_searching.set(false);
            });
        }
    };

    // Event handlers
    let on_input = {
        let search_query = state.search_query;
        move |evt: ev::Event| {
            let value = event_target_value(&evt);
            search_query.set(value.clone());
            fetch_suggestions(value);
        }
    };

    let on_keydown = {
        let search_query = state.search_query;
        let perform_search = perform_search.clone();
        move |evt: ev::KeyboardEvent| {
            match evt.key().as_str() {
                "Enter" => {
                    evt.prevent_default();
                    if show_suggestions.get() && !suggestions.get().is_empty() {
                        let idx = suggestion_index.get() as usize;
                        if let Some(suggestion) = suggestions.get().get(idx) {
                            search_query.set(suggestion.clone());
                        }
                    }
                    show_suggestions.set(false);
                    perform_search();
                }
                "ArrowDown" => {
                    if show_suggestions.get() {
                        evt.prevent_default();
                        let max = suggestions.get().len().saturating_sub(1) as i32;
                        suggestion_index.update(|i| *i = (*i + 1).min(max));
                    }
                }
                "ArrowUp" => {
                    if show_suggestions.get() {
                        evt.prevent_default();
                        suggestion_index.update(|i| *i = (*i - 1).max(0));
                    }
                }
                "Escape" => {
                    show_suggestions.set(false);
                }
                _ => {}
            }
        }
    };

    let on_search_click = {
        let perform_search = perform_search.clone();
        move |_: ev::MouseEvent| {
            perform_search();
        }
    };

    let toggle_advanced = move |_: ev::MouseEvent| {
        state.show_advanced_search.update(|v| *v = !*v);
    };

    let clear_search = {
        let search_query = state.search_query;
        let search_results = state.search_results;
        let search_meta = state.search_meta;
        move |_: ev::MouseEvent| {
            search_query.set(String::new());
            search_results.set(Vec::new());
            search_meta.set(SearchMeta::default());
            show_suggestions.set(false);
        }
    };

    view! {
        <div class="flex-shrink-0 border-b border-[var(--border-subtle)] bg-[var(--bg-surface)]">
            <div class="p-4 space-y-4">
                // Search Input Row
                <div class="flex gap-3">
                    <div class="flex-1 relative">
                        // Search input with icon
                        <div class="relative">
                            <svg
                                class="absolute left-3 top-1/2 transform -translate-y-1/2 w-5 h-5 text-[var(--text-muted)]"
                                fill="none"
                                stroke="currentColor"
                                viewBox="0 0 24 24"
                            >
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                            </svg>
                            <input
                                type="text"
                                class="w-full pl-10 pr-10 py-3 bg-[var(--bg-elevated)] border border-[var(--border-subtle)] rounded-xl text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none focus:border-[var(--accent)] focus:ring-2 focus:ring-[var(--accent)]/20 transition-all"
                                placeholder="Search your library (supports TTRPG terms: HP, AC, spells, monsters...)"
                                prop:value=move || state.search_query.get()
                                on:input=on_input
                                on:keydown=on_keydown
                                on:blur=move |_| {
                                    // Delay hiding to allow click on suggestion
                                    spawn_local(async move {
                                        gloo_timers::future::TimeoutFuture::new(200).await;
                                        show_suggestions.set(false);
                                    });
                                }
                            />
                            // Clear button
                            {move || {
                                if !state.search_query.get().is_empty() {
                                    Some(view! {
                                        <button
                                            class="absolute right-3 top-1/2 transform -translate-y-1/2 text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors"
                                            on:click=clear_search.clone()
                                        >
                                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                            </svg>
                                        </button>
                                    })
                                } else {
                                    None
                                }
                            }}
                        </div>

                        // Suggestions dropdown
                        {move || {
                            let suggs = suggestions.get();
                            if show_suggestions.get() && !suggs.is_empty() {
                                Some(view! {
                                    <div class="absolute top-full left-0 right-0 mt-1 bg-[var(--bg-elevated)] border border-[var(--border-subtle)] rounded-lg shadow-lg z-50 overflow-hidden">
                                        {suggs.into_iter().enumerate().map(|(i, suggestion)| {
                                            let is_selected = move || suggestion_index.get() == i as i32;
                                            let suggestion_clone = suggestion.clone();
                                            view! {
                                                <button
                                                    class=move || format!(
                                                        "w-full px-4 py-2 text-left text-sm transition-colors {}",
                                                        if is_selected() { "bg-[var(--accent)]/20 text-[var(--text-primary)]" } else { "text-[var(--text-muted)] hover:bg-[var(--bg-surface)]" }
                                                    )
                                                    on:click={
                                                        let s = suggestion_clone.clone();
                                                        let search_query = state.search_query;
                                                        let perform_search = perform_search.clone();
                                                        move |_| {
                                                            search_query.set(s.clone());
                                                            show_suggestions.set(false);
                                                            perform_search();
                                                        }
                                                    }
                                                >
                                                    <div class="flex items-center gap-2">
                                                        <svg class="w-4 h-4 text-[var(--text-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                                                        </svg>
                                                        {suggestion}
                                                    </div>
                                                </button>
                                            }
                                        }).collect_view()}
                                    </div>
                                })
                            } else {
                                None
                            }
                        }}
                    </div>

                    <Button
                        variant=ButtonVariant::Primary
                        on_click=on_search_click
                        disabled=Signal::derive(move || state.is_searching.get())
                        loading=Signal::derive(move || state.is_searching.get())
                        class="px-6"
                    >
                        "Search"
                    </Button>

                    <button
                        class=move || format!(
                            "p-3 rounded-xl border transition-colors {}",
                            if state.show_advanced_search.get() {
                                "bg-[var(--accent)]/20 border-[var(--accent)] text-[var(--accent)]"
                            } else {
                                "bg-[var(--bg-elevated)] border-[var(--border-subtle)] text-[var(--text-muted)] hover:border-[var(--text-muted)]"
                            }
                        )
                        title="Advanced Options"
                        on:click=toggle_advanced
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4" />
                        </svg>
                    </button>
                </div>

                // Source Type Filter Pills
                <div class="flex flex-wrap gap-2">
                    {SourceType::all_types().iter().map(|st| {
                        let st = *st;
                        let is_active = move || state.selected_source_type.get() == st;
                        view! {
                            <button
                                class=move || format!(
                                    "px-4 py-1.5 text-sm rounded-full transition-all flex items-center gap-1.5 {}",
                                    if is_active() {
                                        "bg-[var(--accent)] text-white shadow-md"
                                    } else {
                                        "bg-[var(--bg-elevated)] text-[var(--text-muted)] hover:bg-[var(--bg-deep)] hover:text-[var(--text-primary)]"
                                    }
                                )
                                on:click=move |_| state.selected_source_type.set(st)
                            >
                                <span class="text-sm">{st.icon()}</span>
                                <span>{st.label()}</span>
                            </button>
                        }
                    }).collect_view()}
                </div>

                // Advanced Options Panel
                {move || {
                    if state.show_advanced_search.get() {
                        Some(view! {
                            <div class="p-4 bg-[var(--bg-elevated)] rounded-xl border border-[var(--border-subtle)] space-y-4">
                                <h4 class="text-sm font-medium text-[var(--text-primary)]">"Search Weights"</h4>
                                <div class="grid grid-cols-2 gap-6">
                                    <div class="space-y-2">
                                        <div class="flex justify-between items-center">
                                            <label class="text-sm text-[var(--text-muted)]">"Semantic"</label>
                                            <span class="text-sm font-mono text-[var(--accent)]">
                                                {move || format!("{:.0}%", state.semantic_weight.get() * 100.0)}
                                            </span>
                                        </div>
                                        <input
                                            type="range"
                                            min="0"
                                            max="1"
                                            step="0.05"
                                            class="w-full h-2 bg-[var(--bg-deep)] rounded-full appearance-none cursor-pointer accent-[var(--accent)]"
                                            prop:value=move || state.semantic_weight.get()
                                            on:input=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                                    state.semantic_weight.set(v);
                                                }
                                            }
                                        />
                                        <p class="text-xs text-[var(--text-muted)]">"Meaning-based matching"</p>
                                    </div>
                                    <div class="space-y-2">
                                        <div class="flex justify-between items-center">
                                            <label class="text-sm text-[var(--text-muted)]">"Keyword"</label>
                                            <span class="text-sm font-mono text-[var(--accent)]">
                                                {move || format!("{:.0}%", state.keyword_weight.get() * 100.0)}
                                            </span>
                                        </div>
                                        <input
                                            type="range"
                                            min="0"
                                            max="1"
                                            step="0.05"
                                            class="w-full h-2 bg-[var(--bg-deep)] rounded-full appearance-none cursor-pointer accent-[var(--accent)]"
                                            prop:value=move || state.keyword_weight.get()
                                            on:input=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                                    state.keyword_weight.set(v);
                                                }
                                            }
                                        />
                                        <p class="text-xs text-[var(--text-muted)]">"Exact word matching"</p>
                                    </div>
                                </div>
                                <div class="pt-2 border-t border-[var(--border-subtle)]">
                                    <p class="text-xs text-[var(--text-muted)]">
                                        "Semantic search finds conceptually similar content. Keyword search matches exact terms. Balance both for optimal results."
                                    </p>
                                </div>
                            </div>
                        })
                    } else {
                        None
                    }
                }}

                // Search Hints
                {move || {
                    let hints = state.search_hints.get();
                    if hints.is_empty() {
                        return None;
                    }
                    Some(view! {
                        <div class="flex flex-wrap gap-2">
                            <span class="text-xs text-[var(--text-muted)]">"Hints:"</span>
                            {hints.into_iter().map(|hint| {
                                view! {
                                    <span class="text-xs px-2 py-0.5 bg-blue-900/30 text-blue-400 rounded-full border border-blue-500/30">
                                        {hint}
                                    </span>
                                }
                            }).collect_view()}
                        </div>
                    })
                }}

                // Query Expansion/Correction Info
                {move || {
                    let meta = state.search_meta.get();
                    let has_expansion = meta.expanded_query.is_some();
                    let has_correction = meta.corrected_query.is_some();

                    if !has_expansion && !has_correction {
                        return None;
                    }

                    Some(view! {
                        <div class="flex flex-wrap gap-4 text-xs">
                            {meta.expanded_query.map(|expanded| view! {
                                <div class="flex items-center gap-1 text-[var(--text-muted)]">
                                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7l5 5m0 0l-5 5m5-5H6" />
                                    </svg>
                                    <span>"Expanded: "</span>
                                    <span class="text-[var(--text-primary)]">{expanded}</span>
                                </div>
                            })}
                            {meta.corrected_query.map(|corrected| {
                                let corrected_clone = corrected.clone();
                                view! {
                                    <button
                                        class="flex items-center gap-1 text-yellow-400 hover:text-yellow-300 transition-colors"
                                        on:click={
                                            let search_query = state.search_query;
                                            let perform_search = perform_search.clone();
                                            move |_| {
                                                search_query.set(corrected_clone.clone());
                                                perform_search();
                                            }
                                        }
                                    >
                                        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                                        </svg>
                                        <span>"Did you mean: "</span>
                                        <span class="underline">{corrected}</span>
                                        <span>"?"</span>
                                    </button>
                                }
                            })}
                        </div>
                    })
                }}
            </div>

            // Search Status Bar
            {move || {
                let status = state.ingestion_status.get();
                let meta = state.search_meta.get();

                if status.is_empty() && meta.total_hits == 0 {
                    return None;
                }

                let status_class = if status.contains("Error") || status.contains("failed") {
                    "bg-red-900/20 border-red-500/30 text-red-400"
                } else if status.contains("Found") {
                    "bg-green-900/20 border-green-500/30 text-green-400"
                } else {
                    "bg-blue-900/20 border-blue-500/30 text-blue-400"
                };

                Some(view! {
                    <div class=format!("px-4 py-2 text-sm border-t {}", status_class)>
                        <div class="flex items-center justify-between">
                            <span>{status}</span>
                            {move || {
                                let meta = state.search_meta.get();
                                if meta.total_hits > 0 {
                                    Some(view! {
                                        <span class="text-xs opacity-70">
                                            {format!("{} total • {}ms", meta.total_hits, meta.processing_time_ms)}
                                        </span>
                                    })
                                } else {
                                    None
                                }
                            }}
                        </div>
                    </div>
                })
            }}
        </div>
    }
}
