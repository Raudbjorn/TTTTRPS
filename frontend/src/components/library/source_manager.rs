//! Source Manager Component
//!
//! Manages document sources:
//! - Document ingestion with progress
//! - Source listing with stats
//! - Metadata editing (name, type, description, tags)
//! - Source deletion with confirmation
//! - Reindexing options

use leptos::prelude::*;
use leptos::ev;
use leptos::task::spawn_local;

use crate::bindings::{
    pick_document_file, ingest_document_with_progress, reindex_library,
};
use crate::components::design_system::{
    Badge, BadgeVariant, Button, ButtonVariant, Card, CardHeader, CardBody, Input, Modal,
};
use super::{
    use_library_state, SourceType, SourceDocument, DocumentStatus,
};

/// Source manager panel for document ingestion and management
#[component]
pub fn SourceManager() -> impl IntoView {
    let state = use_library_state();

    // Local state for editing
    let editing_name = RwSignal::new(String::new());
    let editing_description = RwSignal::new(String::new());
    let editing_type = RwSignal::new(SourceType::Custom);
    let editing_tags = RwSignal::new(String::new());
    let show_delete_confirm = RwSignal::new(false);
    let delete_target_id = RwSignal::new(String::new());
    let is_reindexing = RwSignal::new(false);

    // Close the source manager
    let close_manager = {
        let show = state.show_source_manager;
        let editing = state.editing_document;
        move |_: ev::MouseEvent| {
            show.set(false);
            editing.set(None);
        }
    };

    // Start editing a document
    let start_editing = {
        let editing = state.editing_document;
        move |doc: SourceDocument| {
            editing_name.set(doc.name.clone());
            editing_description.set(doc.description.clone().unwrap_or_default());
            editing_type.set(doc.source_type);
            editing_tags.set(doc.tags.join(", "));
            editing.set(Some(doc));
        }
    };

    // Save edits
    let save_edits = {
        let documents = state.documents;
        let editing = state.editing_document;
        move |_: ev::MouseEvent| {
            if let Some(doc) = editing.get() {
                documents.update(|docs| {
                    if let Some(d) = docs.iter_mut().find(|d| d.id == doc.id) {
                        d.name = editing_name.get();
                        d.description = Some(editing_description.get()).filter(|s| !s.is_empty());
                        d.source_type = editing_type.get();
                        d.tags = editing_tags.get()
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                });
                editing.set(None);
            }
        }
    };

    // Cancel editing
    let cancel_editing = {
        let editing = state.editing_document;
        move |_: ev::MouseEvent| {
            editing.set(None);
        }
    };

    // Confirm delete
    let confirm_delete = move |id: String| {
        delete_target_id.set(id);
        show_delete_confirm.set(true);
    };

    // Execute delete
    let execute_delete = {
        let documents = state.documents;
        let total_chunks = state.total_chunks;
        move |_: ev::MouseEvent| {
            let id = delete_target_id.get();
            documents.update(|docs| {
                if let Some(idx) = docs.iter().position(|d| d.id == id) {
                    let removed = docs.remove(idx);
                    total_chunks.update(|c| *c = c.saturating_sub(removed.chunk_count));
                }
            });
            show_delete_confirm.set(false);
            delete_target_id.set(String::new());
        }
    };

    // Cancel delete
    let cancel_delete = move |_: ev::MouseEvent| {
        show_delete_confirm.set(false);
        delete_target_id.set(String::new());
    };

    // Reindex all documents
    let handle_reindex = {
        let ingestion_status = state.ingestion_status;
        move |_: ev::MouseEvent| {
            is_reindexing.set(true);
            spawn_local(async move {
                match reindex_library(None).await {
                    Ok(msg) => {
                        ingestion_status.set(format!("Reindex complete: {}", msg));
                    }
                    Err(e) => {
                        ingestion_status.set(format!("Reindex failed: {}", e));
                    }
                }
                is_reindexing.set(false);
            });
        }
    };

    // Ingest new document
    let handle_ingest = {
        let is_ingesting = state.is_ingesting;
        let ingestion_progress = state.ingestion_progress;
        let ingestion_status = state.ingestion_status;
        let documents = state.documents;
        let total_chunks = state.total_chunks;
        let selected_source_type = state.selected_source_type;
        move |_: ev::MouseEvent| {
            spawn_local(async move {
                if let Some(path) = pick_document_file().await {
                    is_ingesting.set(true);
                    ingestion_progress.set(0.0);
                    let filename = path.split('/').last().unwrap_or(&path).to_string();
                    ingestion_status.set(format!("Ingesting {}...", filename));

                    let source_type = selected_source_type.get();
                    let source_type_str = if source_type == SourceType::All {
                        "documents".to_string()
                    } else {
                        source_type.as_str().to_string()
                    };

                    match ingest_document_with_progress(path.clone(), Some(source_type_str.clone())).await {
                        Ok(result) => {
                            let doc_id = format!("doc-{}", documents.get().len() + 1);
                            let doc = SourceDocument {
                                id: doc_id,
                                name: result.source_name.clone(),
                                source_type,
                                status: DocumentStatus::Indexed,
                                chunk_count: result.character_count / 500,
                                page_count: result.page_count,
                                file_size_bytes: result.character_count,
                                ingested_at: Some("Just now".to_string()),
                                file_path: Some(path),
                                description: None,
                                tags: Vec::new(),
                            };
                            documents.update(|docs| docs.push(doc));
                            total_chunks.update(|c| *c += result.character_count / 500);
                            ingestion_status.set(format!(
                                "Indexed {} ({} pages)",
                                result.source_name, result.page_count
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
        <div class="flex-1 flex flex-col overflow-hidden">
            // Header
            <div class="flex-shrink-0 p-4 border-b border-[var(--border-subtle)]">
                <div class="flex items-center justify-between">
                    <h2 class="text-lg font-semibold text-[var(--text-primary)]">"Source Manager"</h2>
                    <button
                        class="p-2 rounded-lg hover:bg-[var(--bg-elevated)] text-[var(--text-muted)] transition-colors"
                        on:click=close_manager
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                        </svg>
                    </button>
                </div>
            </div>

            // Content
            <div class="flex-1 overflow-y-auto p-4 space-y-4">
                // Actions
                <div class="flex gap-2">
                    <Button
                        variant=ButtonVariant::Primary
                        on_click=handle_ingest
                        disabled=state.is_ingesting.get()
                        loading=state.is_ingesting.get()
                        class="flex-1"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
                        </svg>
                        "Add Document"
                    </Button>
                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=handle_reindex
                        disabled=is_reindexing.get()
                        loading=is_reindexing.get()
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                        </svg>
                        "Reindex"
                    </Button>
                </div>

                // Ingestion Progress
                {move || {
                    if state.is_ingesting.get() {
                        let progress = (state.ingestion_progress.get() * 100.0) as u32;
                        Some(view! {
                            <Card>
                                <CardBody>
                                    <div class="space-y-2">
                                        <div class="flex justify-between text-sm">
                                            <span class="text-[var(--text-muted)]">{state.ingestion_status.get()}</span>
                                            <span class="font-mono text-[var(--accent)]">{format!("{}%", progress)}</span>
                                        </div>
                                        <div class="w-full bg-[var(--bg-deep)] rounded-full h-2 overflow-hidden">
                                            <div
                                                class="bg-[var(--accent)] h-2 rounded-full transition-all duration-300"
                                                style=format!("width: {}%", progress)
                                            />
                                        </div>
                                    </div>
                                </CardBody>
                            </Card>
                        })
                    } else {
                        None
                    }
                }}

                // Source Type Selector for new ingestions
                <Card>
                    <CardHeader>
                        <h3 class="text-sm font-medium text-[var(--text-primary)]">"Default Source Type"</h3>
                    </CardHeader>
                    <CardBody>
                        <div class="flex flex-wrap gap-2">
                            {SourceType::all_types().iter().filter(|st| **st != SourceType::All).map(|st| {
                                let st = *st;
                                let is_active = move || state.selected_source_type.get() == st;
                                view! {
                                    <button
                                        class=move || format!(
                                            "px-3 py-1.5 text-xs rounded-full transition-all flex items-center gap-1 {}",
                                            if is_active() {
                                                "bg-[var(--accent)] text-white"
                                            } else {
                                                "bg-[var(--bg-elevated)] text-[var(--text-muted)] hover:bg-[var(--bg-deep)]"
                                            }
                                        )
                                        on:click=move |_| state.selected_source_type.set(st)
                                    >
                                        <span>{st.icon()}</span>
                                        <span>{st.label()}</span>
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                    </CardBody>
                </Card>

                // Document List
                <div class="space-y-2">
                    <h3 class="text-sm font-medium text-[var(--text-primary)]">"Ingested Documents"</h3>
                    {move || {
                        let docs = state.documents.get();
                        let editing = state.editing_document.get();

                        if docs.is_empty() {
                            view! {
                                <div class="text-center py-8 text-[var(--text-muted)]">
                                    <p>"No documents in library"</p>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="space-y-2">
                                    {docs.into_iter().map(|doc| {
                                        let doc_id = doc.id.clone();
                                        let is_editing = editing.as_ref().map(|e| e.id == doc.id).unwrap_or(false);
                                        let doc_for_edit = doc.clone();
                                        let doc_for_delete = doc.id.clone();

                                        if is_editing {
                                            view! {
                                                <Card class="border-[var(--accent)]">
                                                    <CardBody class="space-y-3">
                                                        <div>
                                                            <label class="block text-xs text-[var(--text-muted)] mb-1">"Name"</label>
                                                            <Input
                                                                value=editing_name
                                                                placeholder="Document name"
                                                            />
                                                        </div>
                                                        <div>
                                                            <label class="block text-xs text-[var(--text-muted)] mb-1">"Description"</label>
                                                            <Input
                                                                value=editing_description
                                                                placeholder="Optional description"
                                                            />
                                                        </div>
                                                        <div>
                                                            <label class="block text-xs text-[var(--text-muted)] mb-1">"Tags (comma-separated)"</label>
                                                            <Input
                                                                value=editing_tags
                                                                placeholder="d&d, 5e, rulebook"
                                                            />
                                                        </div>
                                                        <div>
                                                            <label class="block text-xs text-[var(--text-muted)] mb-1">"Source Type"</label>
                                                            <div class="flex flex-wrap gap-1">
                                                                {SourceType::all_types().iter().filter(|st| **st != SourceType::All).map(|st| {
                                                                    let st = *st;
                                                                    let is_active = move || editing_type.get() == st;
                                                                    view! {
                                                                        <button
                                                                            class=move || format!(
                                                                                "px-2 py-1 text-xs rounded transition-colors {}",
                                                                                if is_active() {
                                                                                    "bg-[var(--accent)] text-white"
                                                                                } else {
                                                                                    "bg-[var(--bg-deep)] text-[var(--text-muted)] hover:bg-[var(--bg-surface)]"
                                                                                }
                                                                            )
                                                                            on:click=move |_| editing_type.set(st)
                                                                        >
                                                                            {st.label()}
                                                                        </button>
                                                                    }
                                                                }).collect_view()}
                                                            </div>
                                                        </div>
                                                        <div class="flex gap-2 pt-2">
                                                            <Button
                                                                variant=ButtonVariant::Primary
                                                                on_click=save_edits.clone()
                                                                class="flex-1"
                                                            >
                                                                "Save"
                                                            </Button>
                                                            <Button
                                                                variant=ButtonVariant::Secondary
                                                                on_click=cancel_editing.clone()
                                                            >
                                                                "Cancel"
                                                            </Button>
                                                        </div>
                                                    </CardBody>
                                                </Card>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <div class="p-3 rounded-lg bg-[var(--bg-elevated)] border border-[var(--border-subtle)] hover:border-[var(--text-muted)] transition-colors">
                                                    <div class="flex items-start gap-3">
                                                        <div class="w-10 h-10 rounded-lg bg-[var(--bg-surface)] flex items-center justify-center text-xl flex-shrink-0">
                                                            {doc.source_type.icon()}
                                                        </div>
                                                        <div class="flex-1 min-w-0">
                                                            <h4 class="font-medium text-[var(--text-primary)] truncate">
                                                                {doc.name.clone()}
                                                            </h4>
                                                            <div class="flex items-center gap-2 mt-1 text-xs text-[var(--text-muted)]">
                                                                <span>{format!("{} pages", doc.page_count)}</span>
                                                                <span>"•"</span>
                                                                <span>{format!("{} chunks", doc.chunk_count)}</span>
                                                            </div>
                                                            {doc.description.clone().map(|desc| view! {
                                                                <p class="text-xs text-[var(--text-muted)] mt-1 line-clamp-1">{desc}</p>
                                                            })}
                                                            {if !doc.tags.is_empty() {
                                                                Some(view! {
                                                                    <div class="flex flex-wrap gap-1 mt-2">
                                                                        {doc.tags.iter().map(|tag| view! {
                                                                            <span class="text-xs px-1.5 py-0.5 bg-[var(--bg-surface)] text-[var(--text-muted)] rounded">
                                                                                {tag.clone()}
                                                                            </span>
                                                                        }).collect_view()}
                                                                    </div>
                                                                })
                                                            } else {
                                                                None
                                                            }}
                                                        </div>
                                                        <div class="flex-shrink-0 flex gap-1">
                                                            <button
                                                                class="p-1.5 rounded hover:bg-[var(--bg-surface)] text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors"
                                                                title="Edit"
                                                                on:click={
                                                                    let d = doc_for_edit.clone();
                                                                    let start = start_editing.clone();
                                                                    move |_| start(d.clone())
                                                                }
                                                            >
                                                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                                                                </svg>
                                                            </button>
                                                            <button
                                                                class="p-1.5 rounded hover:bg-red-900/30 text-[var(--text-muted)] hover:text-red-400 transition-colors"
                                                                title="Delete"
                                                                on:click={
                                                                    let id = doc_for_delete.clone();
                                                                    move |_| confirm_delete(id.clone())
                                                                }
                                                            >
                                                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                                                                </svg>
                                                            </button>
                                                        </div>
                                                    </div>
                                                </div>
                                            }.into_any()
                                        }
                                    }).collect_view()}
                                </div>
                            }.into_any()
                        }
                    }}
                </div>

                // Statistics
                <Card>
                    <CardHeader>
                        <h3 class="text-sm font-medium text-[var(--text-primary)]">"Library Statistics"</h3>
                    </CardHeader>
                    <CardBody class="space-y-2">
                        <div class="flex justify-between items-center text-sm">
                            <span class="text-[var(--text-muted)]">"Total Documents"</span>
                            <span class="font-mono text-[var(--text-primary)]">{move || state.documents.get().len()}</span>
                        </div>
                        <div class="flex justify-between items-center text-sm">
                            <span class="text-[var(--text-muted)]">"Total Chunks"</span>
                            <span class="font-mono text-[var(--text-primary)]">{move || state.total_chunks.get()}</span>
                        </div>
                        <div class="flex justify-between items-center text-sm">
                            <span class="text-[var(--text-muted)]">"Index Status"</span>
                            <span class="font-mono">{move || state.meilisearch_status.get()}</span>
                        </div>
                        <div class="pt-2 border-t border-[var(--border-subtle)]">
                            <h4 class="text-xs text-[var(--text-muted)] mb-2">"By Source Type"</h4>
                            <div class="space-y-1">
                                {move || {
                                    let docs = state.documents.get();
                                    let mut counts: std::collections::HashMap<SourceType, usize> = std::collections::HashMap::new();
                                    for doc in docs.iter() {
                                        *counts.entry(doc.source_type).or_insert(0) += 1;
                                    }
                                    counts.into_iter().map(|(st, count)| {
                                        view! {
                                            <div class="flex justify-between items-center text-xs">
                                                <span class="flex items-center gap-1 text-[var(--text-muted)]">
                                                    <span>{st.icon()}</span>
                                                    <span>{st.label()}</span>
                                                </span>
                                                <span class="font-mono text-[var(--text-primary)]">{count}</span>
                                            </div>
                                        }
                                    }).collect_view()
                                }}
                            </div>
                        </div>
                    </CardBody>
                </Card>
            </div>

            // Delete Confirmation Modal
            <Modal
                is_open=show_delete_confirm
                title="Delete Document"
                class="max-w-md"
            >
                <div class="p-6">
                    <p class="text-[var(--text-muted)] mb-6">
                        "Are you sure you want to delete this document? This will remove it from the search index. This action cannot be undone."
                    </p>
                    <div class="flex gap-3 justify-end">
                        <Button
                            variant=ButtonVariant::Secondary
                            on_click=cancel_delete
                        >
                            "Cancel"
                        </Button>
                        <Button
                            variant=ButtonVariant::Danger
                            on_click=execute_delete
                        >
                            "Delete"
                        </Button>
                    </div>
                </div>
            </Modal>
        </div>
    }
}
