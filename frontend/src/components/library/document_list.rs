//! Document List Component
//!
//! Displays search results and ingested documents with:
//! - Grid and list view modes
//! - Document cards with metadata
//! - Selection and hover states
//! - Empty state handling
//! - Drag-and-drop zone for ingestion

use leptos::prelude::*;
use leptos::ev;
use leptos::task::spawn_local;

use crate::bindings::{pick_document_file, ingest_document_two_phase};
use crate::components::design_system::{Badge, BadgeVariant, LoadingSpinner};
use super::{use_library_state, SourceType, SearchResult, ViewMode, SourceDocument, DocumentStatus};

/// Document list/grid component displaying search results or all documents
#[component]
pub fn DocumentList() -> impl IntoView {
    let state = use_library_state();

    // Drag-and-drop handlers
    let on_drag_enter = move |evt: ev::DragEvent| {
        evt.prevent_default();
        state.is_drag_over.set(true);
    };

    let on_drag_over = move |evt: ev::DragEvent| {
        evt.prevent_default();
        state.is_drag_over.set(true);
    };

    let on_drag_leave = move |evt: ev::DragEvent| {
        evt.prevent_default();
        state.is_drag_over.set(false);
    };

    let on_drop = {
        let is_ingesting = state.is_ingesting;
        let ingestion_progress = state.ingestion_progress;
        let ingestion_status = state.ingestion_status;
        let documents = state.documents;
        let total_chunks = state.total_chunks;
        let selected_source_type = state.selected_source_type;
        let is_drag_over = state.is_drag_over;

        move |evt: ev::DragEvent| {
            evt.prevent_default();
            is_drag_over.set(false);

            // Handle dropped files via Tauri file picker fallback
            // In a real implementation, we'd extract file paths from the DragEvent
            spawn_local(async move {
                if let Some(path) = pick_document_file().await {
                    is_ingesting.set(true);
                    ingestion_progress.set(0.0);
                    let filename = path.split('/').last().unwrap_or(&path).to_string();
                    ingestion_status.set(format!("Ingesting {}...", filename));

                    let source_type = selected_source_type.get();

                    // Use two-phase ingestion pipeline
                    match ingest_document_two_phase(path.clone(), None).await {
                        Ok(result) => {
                            let doc = SourceDocument {
                                id: result.slug.clone(),
                                name: result.source_name.clone(),
                                source_type,
                                status: DocumentStatus::Indexed,
                                chunk_count: result.chunk_count,
                                page_count: result.page_count,
                                file_size_bytes: result.total_chars,
                                ingested_at: Some("Just now".to_string()),
                                file_path: Some(path),
                                description: result.game_system.clone(),
                                tags: result.content_category.map(|c| vec![c]).unwrap_or_default(),
                            };
                            documents.update(|docs| docs.push(doc));
                            total_chunks.update(|c| *c += result.chunk_count);
                            ingestion_status.set(format!(
                                "Indexed '{}' → {} pages → {} chunks",
                                result.source_name, result.page_count, result.chunk_count
                            ));
                            ingestion_progress.set(1.0);
                        }
                        Err(e) => {
                            ingestion_status.set(format!("Error: {}", e));
                            ingestion_progress.set(0.0);
                        }
                    }
                    is_ingesting.set(false);
                }
            });
        }
    };

    view! {
        <div
            class="flex-1 overflow-y-auto relative"
            on:dragenter=on_drag_enter
            on:dragover=on_drag_over
            on:dragleave=on_drag_leave
            on:drop=on_drop
        >
            // Drag overlay
            {move || {
                if state.is_drag_over.get() {
                    Some(view! {
                        <div class="absolute inset-0 bg-[var(--accent)]/10 border-2 border-dashed border-[var(--accent)] rounded-lg z-50 flex items-center justify-center backdrop-blur-sm">
                            <div class="text-center">
                                <svg class="w-16 h-16 mx-auto text-[var(--accent)] mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                                </svg>
                                <p class="text-lg font-medium text-[var(--accent)]">"Drop files to ingest"</p>
                                <p class="text-sm text-[var(--text-muted)] mt-1">"PDF, EPUB, DOCX, MD, TXT"</p>
                            </div>
                        </div>
                    })
                } else {
                    None
                }
            }}

            // Content
            {move || {
                let results = state.search_results.get();
                let has_search = !state.search_query.get().is_empty();
                let view_mode = state.view_mode.get();

                if state.is_searching.get() {
                    // Loading state
                    view! {
                        <div class="flex items-center justify-center h-full">
                            <div class="text-center">
                                <LoadingSpinner size="lg" />
                                <p class="mt-4 text-[var(--text-muted)]">"Searching..."</p>
                            </div>
                        </div>
                    }.into_any()
                } else if has_search {
                    // Search results
                    if results.is_empty() {
                        view! {
                            <EmptySearchState query=state.search_query.get() />
                        }.into_any()
                    } else {
                        match view_mode {
                            ViewMode::Grid => view! {
                                <SearchResultsGrid results=results />
                            }.into_any(),
                            ViewMode::List => view! {
                                <SearchResultsList results=results />
                            }.into_any(),
                        }
                    }
                } else {
                    // No search - show ingested documents or empty state
                    let docs = state.documents.get();
                    if docs.is_empty() {
                        view! {
                            <EmptyLibraryState />
                        }.into_any()
                    } else {
                        match view_mode {
                            ViewMode::Grid => view! {
                                <DocumentsGrid documents=docs />
                            }.into_any(),
                            ViewMode::List => view! {
                                <DocumentsList documents=docs />
                            }.into_any(),
                        }
                    }
                }
            }}
        </div>
    }
}

/// Search results in grid layout
#[component]
fn SearchResultsGrid(results: Vec<SearchResult>) -> impl IntoView {
    let state = use_library_state();

    view! {
        <div class="p-4 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {results.into_iter().map(|result| {
                let result_clone = result.clone();
                let is_selected = {
                    let result_id = result.id.clone();
                    move || {
                        state.selected_document.get()
                            .map(|d| d.id == result_id)
                            .unwrap_or(false)
                    }
                };

                view! {
                    <div
                        class=move || format!(
                            "p-4 rounded-xl border cursor-pointer transition-all hover:shadow-lg {}",
                            if is_selected() {
                                "bg-[var(--accent)]/10 border-[var(--accent)] shadow-md"
                            } else {
                                "bg-[var(--bg-elevated)] border-[var(--border-subtle)] hover:border-[var(--text-muted)]"
                            }
                        )
                        on:click={
                            let r = result_clone.clone();
                            move |_| state.selected_document.set(Some(r.clone()))
                        }
                    >
                        <div class="flex items-start justify-between mb-2">
                            <div class="flex items-center gap-2">
                                <span class="text-lg">{result.source_type.icon()}</span>
                                <Badge variant=BadgeVariant::Default>{result.source_type.label()}</Badge>
                            </div>
                            <span class="text-xs font-mono text-[var(--accent)]">
                                {format!("{:.2}", result.score)}
                            </span>
                        </div>

                        <h3 class="font-medium text-[var(--text-primary)] mb-1 line-clamp-1">
                            {result.title.clone()}
                        </h3>

                        {result.page_number.map(|p| view! {
                            <p class="text-xs text-[var(--text-muted)] mb-2">
                                {format!("Page {}", p)}
                            </p>
                        })}

                        <p class="text-sm text-[var(--text-muted)] line-clamp-3">
                            {result.snippet.clone()}
                        </p>

                        <div class="flex gap-2 mt-3 text-xs text-[var(--text-muted)]">
                            {result.keyword_rank.map(|r| view! {
                                <span class="px-2 py-0.5 bg-blue-900/30 text-blue-400 rounded">
                                    {format!("Keyword #{}", r + 1)}
                                </span>
                            })}
                            {result.semantic_rank.map(|r| view! {
                                <span class="px-2 py-0.5 bg-purple-900/30 text-purple-400 rounded">
                                    {format!("Semantic #{}", r + 1)}
                                </span>
                            })}
                        </div>
                    </div>
                }
            }).collect_view()}
        </div>
    }
}

/// Search results in list layout
#[component]
fn SearchResultsList(results: Vec<SearchResult>) -> impl IntoView {
    let state = use_library_state();

    view! {
        <div class="divide-y divide-[var(--border-subtle)]">
            {results.into_iter().map(|result| {
                let result_clone = result.clone();
                let is_selected = {
                    let result_id = result.id.clone();
                    move || {
                        state.selected_document.get()
                            .map(|d| d.id == result_id)
                            .unwrap_or(false)
                    }
                };

                view! {
                    <div
                        class=move || format!(
                            "p-4 cursor-pointer transition-colors {}",
                            if is_selected() {
                                "bg-[var(--accent)]/10"
                            } else {
                                "hover:bg-[var(--bg-elevated)]"
                            }
                        )
                        on:click={
                            let r = result_clone.clone();
                            move |_| state.selected_document.set(Some(r.clone()))
                        }
                    >
                        <div class="flex items-start gap-4">
                            // Icon
                            <div class="flex-shrink-0 w-10 h-10 rounded-lg bg-[var(--bg-surface)] flex items-center justify-center text-xl">
                                {result.source_type.icon()}
                            </div>

                            // Content
                            <div class="flex-1 min-w-0">
                                <div class="flex items-center gap-2 mb-1">
                                    <h3 class="font-medium text-[var(--text-primary)] truncate">
                                        {result.title.clone()}
                                    </h3>
                                    {result.page_number.map(|p| view! {
                                        <span class="text-xs text-[var(--text-muted)]">
                                            {format!("p.{}", p)}
                                        </span>
                                    })}
                                </div>
                                <p class="text-sm text-[var(--text-muted)] line-clamp-2">
                                    {result.snippet.clone()}
                                </p>
                            </div>

                            // Score and badges
                            <div class="flex-shrink-0 text-right">
                                <span class="text-lg font-mono text-[var(--accent)]">
                                    {format!("{:.2}", result.score)}
                                </span>
                                <div class="flex gap-1 mt-1">
                                    <Badge variant=BadgeVariant::Default>{result.source_type.label()}</Badge>
                                </div>
                            </div>
                        </div>
                    </div>
                }
            }).collect_view()}
        </div>
    }
}

/// Documents in grid layout
#[component]
fn DocumentsGrid(documents: Vec<SourceDocument>) -> impl IntoView {
    let state = use_library_state();

    view! {
        <div class="p-4 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {documents.into_iter().map(|doc| {
                let doc_clone = doc.clone();
                let is_selected = {
                    let doc_id = doc.id.clone();
                    move || {
                        state.selected_source_doc.get()
                            .map(|d| d.id == doc_id)
                            .unwrap_or(false)
                    }
                };

                view! {
                    <div
                        class=move || format!(
                            "p-4 rounded-xl border cursor-pointer transition-all hover:shadow-lg {}",
                            if is_selected() {
                                "bg-[var(--accent)]/10 border-[var(--accent)] shadow-md"
                            } else {
                                "bg-[var(--bg-elevated)] border-[var(--border-subtle)] hover:border-[var(--text-muted)]"
                            }
                        )
                        on:click={
                            let d = doc_clone.clone();
                            move |_| state.selected_source_doc.set(Some(d.clone()))
                        }
                    >
                        <div class="flex items-center gap-3 mb-3">
                            <div class="w-12 h-12 rounded-lg bg-[var(--bg-surface)] flex items-center justify-center text-2xl">
                                {doc.source_type.icon()}
                            </div>
                            <div class="flex-1 min-w-0">
                                <h3 class="font-medium text-[var(--text-primary)] truncate">
                                    {doc.name.clone()}
                                </h3>
                                <Badge variant=doc.status.badge_variant()>{doc.status.as_str()}</Badge>
                            </div>
                        </div>

                        <div class="space-y-1 text-sm text-[var(--text-muted)]">
                            <div class="flex justify-between">
                                <span>"Pages"</span>
                                <span class="font-mono">{doc.page_count}</span>
                            </div>
                            <div class="flex justify-between">
                                <span>"Chunks"</span>
                                <span class="font-mono">{doc.chunk_count}</span>
                            </div>
                            <div class="flex justify-between">
                                <span>"Type"</span>
                                <span>{doc.source_type.label()}</span>
                            </div>
                        </div>

                        {doc.ingested_at.clone().map(|date| view! {
                            <div class="mt-3 pt-3 border-t border-[var(--border-subtle)] text-xs text-[var(--text-muted)]">
                                {format!("Ingested: {}", date)}
                            </div>
                        })}
                    </div>
                }
            }).collect_view()}
        </div>
    }
}

/// Documents in list layout
#[component]
fn DocumentsList(documents: Vec<SourceDocument>) -> impl IntoView {
    let state = use_library_state();

    view! {
        <div class="divide-y divide-[var(--border-subtle)]">
            {documents.into_iter().map(|doc| {
                let doc_clone = doc.clone();
                let is_selected = {
                    let doc_id = doc.id.clone();
                    move || {
                        state.selected_source_doc.get()
                            .map(|d| d.id == doc_id)
                            .unwrap_or(false)
                    }
                };

                view! {
                    <div
                        class=move || format!(
                            "p-4 cursor-pointer transition-colors {}",
                            if is_selected() {
                                "bg-[var(--accent)]/10"
                            } else {
                                "hover:bg-[var(--bg-elevated)]"
                            }
                        )
                        on:click={
                            let d = doc_clone.clone();
                            move |_| state.selected_source_doc.set(Some(d.clone()))
                        }
                    >
                        <div class="flex items-center gap-4">
                            // Icon
                            <div class="flex-shrink-0 w-12 h-12 rounded-lg bg-[var(--bg-surface)] flex items-center justify-center text-2xl">
                                {doc.source_type.icon()}
                            </div>

                            // Info
                            <div class="flex-1 min-w-0">
                                <h3 class="font-medium text-[var(--text-primary)] truncate">
                                    {doc.name.clone()}
                                </h3>
                                <div class="flex items-center gap-2 mt-1 text-sm text-[var(--text-muted)]">
                                    <span>{format!("{} pages", doc.page_count)}</span>
                                    <span>"•"</span>
                                    <span>{format!("{} chunks", doc.chunk_count)}</span>
                                    <span>"•"</span>
                                    <span>{doc.source_type.label()}</span>
                                </div>
                            </div>

                            // Status and date
                            <div class="flex-shrink-0 text-right">
                                <Badge variant=doc.status.badge_variant()>{doc.status.as_str()}</Badge>
                                {doc.ingested_at.clone().map(|date| view! {
                                    <p class="text-xs text-[var(--text-muted)] mt-1">{date}</p>
                                })}
                            </div>
                        </div>
                    </div>
                }
            }).collect_view()}
        </div>
    }
}

/// Empty state when library has no documents
#[component]
fn EmptyLibraryState() -> impl IntoView {
    view! {
        <div class="flex items-center justify-center h-full">
            <div class="text-center max-w-md mx-auto p-8">
                <div class="w-24 h-24 mx-auto mb-6 rounded-2xl bg-[var(--bg-elevated)] flex items-center justify-center">
                    <svg class="w-12 h-12 text-[var(--text-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
                    </svg>
                </div>

                <h2 class="text-xl font-semibold text-[var(--text-primary)] mb-2">
                    "Your library is empty"
                </h2>
                <p class="text-[var(--text-muted)] mb-6">
                    "Ingest your TTRPG rulebooks, adventures, and notes to enable powerful hybrid search across all your content."
                </p>

                <div class="space-y-3">
                    <div class="p-4 rounded-lg bg-[var(--bg-elevated)] border border-[var(--border-subtle)]">
                        <h3 class="text-sm font-medium text-[var(--text-primary)] mb-2">"Get Started"</h3>
                        <ul class="text-sm text-[var(--text-muted)] space-y-2 text-left">
                            <li class="flex items-start gap-2">
                                <span class="text-[var(--accent)]">"1."</span>
                                <span>"Click 'Ingest Document' or drag files here"</span>
                            </li>
                            <li class="flex items-start gap-2">
                                <span class="text-[var(--accent)]">"2."</span>
                                <span>"Select source type (Rulebook, Adventure, etc.)"</span>
                            </li>
                            <li class="flex items-start gap-2">
                                <span class="text-[var(--accent)]">"3."</span>
                                <span>"Search your content with hybrid search"</span>
                            </li>
                        </ul>
                    </div>

                    <div class="flex flex-wrap justify-center gap-2">
                        <Badge variant=BadgeVariant::Success>"PDF"</Badge>
                        <Badge variant=BadgeVariant::Success>"EPUB"</Badge>
                        <Badge variant=BadgeVariant::Success>"MOBI"</Badge>
                        <Badge variant=BadgeVariant::Success>"DOCX"</Badge>
                        <Badge variant=BadgeVariant::Success>"MD"</Badge>
                        <Badge variant=BadgeVariant::Success>"TXT"</Badge>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Empty state when search returns no results
#[component]
fn EmptySearchState(query: String) -> impl IntoView {
    view! {
        <div class="flex items-center justify-center h-full">
            <div class="text-center max-w-md mx-auto p-8">
                <div class="w-20 h-20 mx-auto mb-6 rounded-2xl bg-[var(--bg-elevated)] flex items-center justify-center">
                    <svg class="w-10 h-10 text-[var(--text-muted)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                    </svg>
                </div>

                <h2 class="text-xl font-semibold text-[var(--text-primary)] mb-2">
                    "No results found"
                </h2>
                <p class="text-[var(--text-muted)] mb-4">
                    {format!("No documents match '{}'. Try adjusting your search or filters.", query)}
                </p>

                <div class="p-4 rounded-lg bg-[var(--bg-elevated)] border border-[var(--border-subtle)] text-left">
                    <h3 class="text-sm font-medium text-[var(--text-primary)] mb-2">"Search Tips"</h3>
                    <ul class="text-sm text-[var(--text-muted)] space-y-1">
                        <li>"• Try different keywords or synonyms"</li>
                        <li>"• Use TTRPG abbreviations: HP, AC, DC, CR"</li>
                        <li>"• Remove source type filters to broaden search"</li>
                        <li>"• Adjust semantic/keyword weights in advanced options"</li>
                    </ul>
                </div>
            </div>
        </div>
    }
}
