//! Document Detail Component
//!
//! Shows detailed information about a selected search result or document:
//! - Full content text with highlight support
//! - Source metadata (page, source type, scores)
//! - Related search results
//! - Action buttons (copy, cite, bookmark)
//! - Content navigation for long results

use leptos::prelude::*;
use leptos::ev;
use leptos::task::spawn_local;

use crate::bindings::{hybrid_search, HybridSearchOptions, copy_to_clipboard};
use crate::components::design_system::{Badge, BadgeVariant, Button, ButtonVariant, Card, CardHeader, CardBody};
use super::{use_library_state, SearchResult, SourceType};

/// Document detail panel showing selected search result
#[component]
pub fn DocumentDetail() -> impl IntoView {
    let state = use_library_state();

    // Local state for related results
    let related_results = RwSignal::new(Vec::<SearchResult>::new());
    let is_loading_related = RwSignal::new(false);
    let show_full_content = RwSignal::new(false);
    let copied_status = RwSignal::new(false);

    // Close detail view
    let close_detail = {
        let selected = state.selected_document;
        move |_: ev::MouseEvent| {
            selected.set(None);
            related_results.set(Vec::new());
            show_full_content.set(false);
        }
    };

    // Load related results when document changes
    Effect::new({
        let selected_document = state.selected_document;
        move |_| {
            if let Some(doc) = selected_document.get() {
                is_loading_related.set(true);
                let source = doc.source.clone();
                spawn_local(async move {
                    let options = HybridSearchOptions {
                        limit: 5,
                        source_type: None,
                        campaign_id: None,
                        index: None,
                        semantic_weight: Some(0.8),
                        keyword_weight: Some(0.2),
                    };

                    match hybrid_search(source.clone(), Some(options)).await {
                        Ok(response) => {
                            let results: Vec<SearchResult> = response
                                .results
                                .into_iter()
                                .map(SearchResult::from)
                                .filter(|r| r.id != doc.id) // Exclude current doc
                                .take(4)
                                .collect();
                            related_results.set(results);
                        }
                        Err(_) => {
                            related_results.set(Vec::new());
                        }
                    }
                    is_loading_related.set(false);
                });
            }
        }
    });

    // Copy content to clipboard
    let copy_content = {
        let selected = state.selected_document;
        move |_: ev::MouseEvent| {
            if let Some(doc) = selected.get() {
                spawn_local(async move {
                    let _ = copy_to_clipboard(doc.content.clone()).await;
                    copied_status.set(true);
                    gloo_timers::future::TimeoutFuture::new(2000).await;
                    copied_status.set(false);
                });
            }
        }
    };

    // Generate citation
    let generate_citation = {
        let selected = state.selected_document;
        move |_: ev::MouseEvent| {
            if let Some(doc) = selected.get() {
                let citation = format!(
                    "{}{} ({})",
                    doc.source,
                    doc.page_number.map(|p| format!(", p.{}", p)).unwrap_or_default(),
                    doc.source_type.label()
                );
                spawn_local(async move {
                    let _ = copy_to_clipboard(citation).await;
                    copied_status.set(true);
                    gloo_timers::future::TimeoutFuture::new(2000).await;
                    copied_status.set(false);
                });
            }
        }
    };

    view! {
        <div class="flex-1 flex flex-col overflow-hidden">
            {move || {
                match state.selected_document.get() {
                    Some(doc) => {
                        view! {
                            <div class="flex-1 flex flex-col overflow-hidden">
                                // Header
                                <div class="flex-shrink-0 p-4 border-b border-[var(--border-subtle)]">
                                    <div class="flex items-start justify-between gap-3">
                                        <div class="flex-1 min-w-0">
                                            <h2 class="text-lg font-semibold text-[var(--text-primary)] truncate">
                                                {doc.title.clone()}
                                            </h2>
                                            <div class="flex items-center gap-2 mt-1">
                                                <span class="text-lg">{doc.source_type.icon()}</span>
                                                <Badge variant=BadgeVariant::Default>{doc.source_type.label()}</Badge>
                                                {doc.page_number.map(|p| view! {
                                                    <span class="text-sm text-[var(--text-muted)]">
                                                        {format!("Page {}", p)}
                                                    </span>
                                                })}
                                            </div>
                                        </div>
                                        <button
                                            class="p-2 rounded-lg hover:bg-[var(--bg-elevated)] text-[var(--text-muted)] transition-colors flex-shrink-0"
                                            on:click=close_detail.clone()
                                        >
                                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                            </svg>
                                        </button>
                                    </div>
                                </div>

                                // Content
                                <div class="flex-1 overflow-y-auto p-4 space-y-4">
                                    // Score badges
                                    <div class="flex flex-wrap gap-2">
                                        <div class="px-3 py-1.5 rounded-lg bg-[var(--accent)]/20 border border-[var(--accent)]/30">
                                            <span class="text-xs text-[var(--text-muted)]">"Score: "</span>
                                            <span class="text-sm font-mono text-[var(--accent)]">
                                                {format!("{:.3}", doc.score)}
                                            </span>
                                        </div>
                                        {doc.keyword_rank.map(|r| view! {
                                            <div class="px-3 py-1.5 rounded-lg bg-blue-900/30 border border-blue-500/30">
                                                <span class="text-xs text-[var(--text-muted)]">"Keyword: "</span>
                                                <span class="text-sm font-mono text-blue-400">
                                                    {format!("#{}", r + 1)}
                                                </span>
                                            </div>
                                        })}
                                        {doc.semantic_rank.map(|r| view! {
                                            <div class="px-3 py-1.5 rounded-lg bg-purple-900/30 border border-purple-500/30">
                                                <span class="text-xs text-[var(--text-muted)]">"Semantic: "</span>
                                                <span class="text-sm font-mono text-purple-400">
                                                    {format!("#{}", r + 1)}
                                                </span>
                                            </div>
                                        })}
                                    </div>

                                    // Content section
                                    <Card>
                                        <CardHeader>
                                            <div class="flex items-center justify-between">
                                                <h3 class="text-sm font-medium text-[var(--text-primary)]">"Content"</h3>
                                                <button
                                                    class="text-xs text-[var(--accent)] hover:underline"
                                                    on:click=move |_| show_full_content.update(|v| *v = !*v)
                                                >
                                                    {move || if show_full_content.get() { "Show less" } else { "Show more" }}
                                                </button>
                                            </div>
                                        </CardHeader>
                                        <CardBody>
                                            <div class=move || format!(
                                                "text-sm text-[var(--text-muted)] leading-relaxed {}",
                                                if show_full_content.get() { "" } else { "max-h-64 overflow-hidden relative" }
                                            )>
                                                <p class="whitespace-pre-wrap">
                                                    {if show_full_content.get() {
                                                        doc.content.clone()
                                                    } else {
                                                        doc.snippet.clone()
                                                    }}
                                                </p>
                                                {move || {
                                                    if !show_full_content.get() {
                                                        Some(view! {
                                                            <div class="absolute bottom-0 left-0 right-0 h-12 bg-gradient-to-t from-[var(--bg-elevated)] to-transparent"></div>
                                                        })
                                                    } else {
                                                        None
                                                    }
                                                }}
                                            </div>
                                        </CardBody>
                                    </Card>

                                    // Action buttons
                                    <div class="flex gap-2">
                                        <Button
                                            variant=ButtonVariant::Secondary
                                            on_click=copy_content.clone()
                                            class="flex-1"
                                        >
                                            {move || if copied_status.get() {
                                                view! {
                                                    <>
                                                        <svg class="w-4 h-4 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                                                        </svg>
                                                        "Copied!"
                                                    </>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <>
                                                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3" />
                                                        </svg>
                                                        "Copy"
                                                    </>
                                                }.into_any()
                                            }}
                                        </Button>
                                        <Button
                                            variant=ButtonVariant::Secondary
                                            on_click=generate_citation.clone()
                                            class="flex-1"
                                        >
                                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
                                            </svg>
                                            "Cite"
                                        </Button>
                                    </div>

                                    // Metadata
                                    <Card>
                                        <CardHeader>
                                            <h3 class="text-sm font-medium text-[var(--text-primary)]">"Metadata"</h3>
                                        </CardHeader>
                                        <CardBody class="space-y-2">
                                            <div class="flex justify-between items-center text-sm">
                                                <span class="text-[var(--text-muted)]">"Source"</span>
                                                <span class="text-[var(--text-primary)] truncate max-w-[200px]">{doc.source.clone()}</span>
                                            </div>
                                            <div class="flex justify-between items-center text-sm">
                                                <span class="text-[var(--text-muted)]">"Type"</span>
                                                <span class="text-[var(--text-primary)]">{doc.source_type.label()}</span>
                                            </div>
                                            {doc.page_number.map(|p| view! {
                                                <div class="flex justify-between items-center text-sm">
                                                    <span class="text-[var(--text-muted)]">"Page"</span>
                                                    <span class="text-[var(--text-primary)]">{p}</span>
                                                </div>
                                            })}
                                            <div class="flex justify-between items-center text-sm">
                                                <span class="text-[var(--text-muted)]">"Content Length"</span>
                                                <span class="text-[var(--text-primary)]">{format!("{} chars", doc.content.len())}</span>
                                            </div>
                                        </CardBody>
                                    </Card>

                                    // Related Results
                                    <Card>
                                        <CardHeader>
                                            <div class="flex items-center justify-between">
                                                <h3 class="text-sm font-medium text-[var(--text-primary)]">"Related Content"</h3>
                                                {move || {
                                                    if is_loading_related.get() {
                                                        Some(view! {
                                                            <div class="w-4 h-4 border-2 border-[var(--accent)] border-t-transparent rounded-full animate-spin"></div>
                                                        })
                                                    } else {
                                                        None
                                                    }
                                                }}
                                            </div>
                                        </CardHeader>
                                        <CardBody>
                                            {move || {
                                                let results = related_results.get();
                                                if results.is_empty() && !is_loading_related.get() {
                                                    view! {
                                                        <p class="text-sm text-[var(--text-muted)] text-center py-4">
                                                            "No related content found"
                                                        </p>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div class="space-y-2">
                                                            {results.into_iter().map(|result| {
                                                                let result_clone = result.clone();
                                                                view! {
                                                                    <button
                                                                        class="w-full p-2 rounded-lg bg-[var(--bg-surface)] hover:bg-[var(--bg-deep)] transition-colors text-left"
                                                                        on:click={
                                                                            let r = result_clone.clone();
                                                                            let selected = state.selected_document;
                                                                            move |_| selected.set(Some(r.clone()))
                                                                        }
                                                                    >
                                                                        <div class="flex items-start gap-2">
                                                                            <span class="text-sm">{result.source_type.icon()}</span>
                                                                            <div class="flex-1 min-w-0">
                                                                                <h4 class="text-sm font-medium text-[var(--text-primary)] truncate">
                                                                                    {result.title.clone()}
                                                                                </h4>
                                                                                <p class="text-xs text-[var(--text-muted)] line-clamp-1">
                                                                                    {result.snippet.clone()}
                                                                                </p>
                                                                            </div>
                                                                            <span class="text-xs font-mono text-[var(--accent)] flex-shrink-0">
                                                                                {format!("{:.2}", result.score)}
                                                                            </span>
                                                                        </div>
                                                                    </button>
                                                                }
                                                            }).collect_view()}
                                                        </div>
                                                    }.into_any()
                                                }
                                            }}
                                        </CardBody>
                                    </Card>

                                    // Search Tips Card
                                    <Card>
                                        <CardHeader>
                                            <h3 class="text-sm font-medium text-[var(--text-primary)]">"Related Searches"</h3>
                                        </CardHeader>
                                        <CardBody>
                                            <div class="flex flex-wrap gap-2">
                                                {vec![
                                                    format!("{} rules", doc.source_type.label().to_lowercase()),
                                                    "similar content".to_string(),
                                                    doc.source.clone(),
                                                ].into_iter().map(|suggestion| {
                                                    let suggestion_clone = suggestion.clone();
                                                    view! {
                                                        <button
                                                            class="px-2 py-1 text-xs rounded-full bg-[var(--bg-surface)] text-[var(--text-muted)] hover:bg-[var(--bg-deep)] hover:text-[var(--text-primary)] transition-colors"
                                                            on:click={
                                                                let s = suggestion_clone.clone();
                                                                let search_query = state.search_query;
                                                                move |_| search_query.set(s.clone())
                                                            }
                                                        >
                                                            {suggestion}
                                                        </button>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        </CardBody>
                                    </Card>
                                </div>
                            </div>
                        }.into_any()
                    }
                    None => {
                        view! {
                            <EmptyDetailState />
                        }.into_any()
                    }
                }
            }}
        </div>
    }
}

/// Empty state when no document is selected
#[component]
fn EmptyDetailState() -> impl IntoView {
    view! {
        <div class="flex-1 flex items-center justify-center p-8">
            <div class="text-center max-w-xs">
                <div class="w-16 h-16 mx-auto mb-4 rounded-2xl bg-[var(--bg-elevated)] flex items-center justify-center">
                    <svg class="w-8 h-8 text-[var(--text-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                    </svg>
                </div>
                <h3 class="text-lg font-medium text-[var(--text-primary)] mb-2">"Select a Document"</h3>
                <p class="text-sm text-[var(--text-muted)]">
                    "Click on a search result or document to view its details here."
                </p>
            </div>
        </div>
    }
}
