//! Library Module
//!
//! Provides document ingestion, hybrid search, and library browsing UI.
//!
//! # Components
//! - `Library` - Main library browser with search and ingestion
//! - `SearchPanel` - Advanced search with filters and suggestions
//! - `DocumentList` - Document listing with source type filtering
//! - `DocumentDetail` - Detailed document view with metadata
//! - `SourceManager` - Source management and ingestion

mod search_panel;
mod document_list;
mod document_detail;
mod source_manager;

pub use search_panel::SearchPanel;
pub use document_list::DocumentList;
pub use document_detail::DocumentDetail;
pub use source_manager::SourceManager;

use leptos::prelude::*;
use leptos::ev;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;
use crate::services::notification_service::{show_error, show_success, ToastAction};
use std::sync::Arc;

use crate::bindings::{
    check_meilisearch_health, ingest_document_two_phase, listen_event,
    pick_document_file, hybrid_search, HybridSearchOptions,
    HybridSearchResultPayload, list_library_documents, LibraryDocument,
    rebuild_library_metadata,
};
use crate::components::design_system::{Badge, BadgeVariant, Button, ButtonVariant, Card, CardHeader, CardBody, Input, Modal, LoadingSpinner};

// ============================================================================
// Types
// ============================================================================

/// Source type for filtering
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum SourceType {
    All,
    Rulebook,
    Adventure,
    Homebrew,
    Notes,
    Characters,
    Sessions,
    Custom,
}

impl SourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceType::All => "all",
            SourceType::Rulebook => "rulebook",
            SourceType::Adventure => "adventure",
            SourceType::Homebrew => "homebrew",
            SourceType::Notes => "notes",
            SourceType::Characters => "characters",
            SourceType::Sessions => "sessions",
            SourceType::Custom => "custom",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SourceType::All => "All Sources",
            SourceType::Rulebook => "Rulebooks",
            SourceType::Adventure => "Adventures",
            SourceType::Homebrew => "Homebrew",
            SourceType::Notes => "Notes",
            SourceType::Characters => "Characters",
            SourceType::Sessions => "Sessions",
            SourceType::Custom => "Custom",
        }
    }

    pub fn all_types() -> &'static [SourceType] {
        &[
            SourceType::All,
            SourceType::Rulebook,
            SourceType::Adventure,
            SourceType::Homebrew,
            SourceType::Notes,
            SourceType::Characters,
            SourceType::Sessions,
            SourceType::Custom,
        ]
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "rulebook" | "rules" => SourceType::Rulebook,
            "adventure" => SourceType::Adventure,
            "homebrew" => SourceType::Homebrew,
            "notes" => SourceType::Notes,
            "characters" => SourceType::Characters,
            "sessions" => SourceType::Sessions,
            "custom" => SourceType::Custom,
            _ => SourceType::All,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SourceType::All => "📚",
            SourceType::Rulebook => "📖",
            SourceType::Adventure => "🗺️",
            SourceType::Homebrew => "🍺",
            SourceType::Notes => "📝",
            SourceType::Characters => "👤",
            SourceType::Sessions => "🎭",
            SourceType::Custom => "📁",
        }
    }
}

/// Represents a source document in the library
#[derive(Clone, PartialEq, Debug)]
pub struct SourceDocument {
    pub id: String,
    pub name: String,
    pub source_type: SourceType,
    pub status: DocumentStatus,
    pub chunk_count: usize,
    pub page_count: usize,
    pub file_size_bytes: usize,
    pub ingested_at: Option<String>,
    pub file_path: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

impl Default for SourceDocument {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            source_type: SourceType::Custom,
            status: DocumentStatus::Pending,
            chunk_count: 0,
            page_count: 0,
            file_size_bytes: 0,
            ingested_at: None,
            file_path: None,
            description: None,
            tags: Vec::new(),
        }
    }
}

/// Document indexing status
#[derive(Clone, PartialEq, Debug, Copy)]
pub enum DocumentStatus {
    Pending,
    Indexing,
    Indexed,
    Failed,
}

impl DocumentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentStatus::Pending => "Pending",
            DocumentStatus::Indexing => "Indexing",
            DocumentStatus::Indexed => "Indexed",
            DocumentStatus::Failed => "Failed",
        }
    }

    pub fn badge_variant(&self) -> BadgeVariant {
        match self {
            DocumentStatus::Pending => BadgeVariant::Default,
            DocumentStatus::Indexing => BadgeVariant::Info,
            DocumentStatus::Indexed => BadgeVariant::Success,
            DocumentStatus::Failed => BadgeVariant::Danger,
        }
    }
}

/// Enhanced search result for display
#[derive(Clone, PartialEq, Debug)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub content: String,
    pub snippet: String,
    pub source: String,
    pub source_type: SourceType,
    pub page_number: Option<u32>,
    pub score: f32,
    pub keyword_rank: Option<usize>,
    pub semantic_rank: Option<usize>,
    pub highlights: Vec<String>,
}

impl From<HybridSearchResultPayload> for SearchResult {
    fn from(r: HybridSearchResultPayload) -> Self {
        let snippet = if r.content.len() > 300 {
            format!("{}...", &r.content[0..300])
        } else {
            r.content.clone()
        };

        Self {
            id: format!("{}-{:?}", r.source, r.page_number),
            title: format!("{}", r.source),
            content: r.content.clone(),
            snippet,
            source: r.source,
            source_type: SourceType::from_str(&r.source_type),
            page_number: r.page_number,
            score: r.score,
            keyword_rank: r.keyword_rank,
            semantic_rank: r.semantic_rank,
            highlights: Vec::new(),
        }
    }
}

/// Search metadata
#[derive(Clone, PartialEq, Debug, Default)]
pub struct SearchMeta {
    pub total_hits: usize,
    pub processing_time_ms: u64,
    pub expanded_query: Option<String>,
    pub corrected_query: Option<String>,
    pub hints: Vec<String>,
}

/// View mode for the library
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ViewMode {
    Grid,
    List,
}

/// Sort options for documents
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SortOption {
    NameAsc,
    NameDesc,
    DateNewest,
    DateOldest,
    SizeAsc,
    SizeDesc,
}

impl SortOption {
    pub fn label(&self) -> &'static str {
        match self {
            SortOption::NameAsc => "Name (A-Z)",
            SortOption::NameDesc => "Name (Z-A)",
            SortOption::DateNewest => "Date (Newest)",
            SortOption::DateOldest => "Date (Oldest)",
            SortOption::SizeAsc => "Size (Smallest)",
            SortOption::SizeDesc => "Size (Largest)",
        }
    }

    pub fn all() -> &'static [SortOption] {
        &[
            SortOption::NameAsc,
            SortOption::NameDesc,
            SortOption::DateNewest,
            SortOption::DateOldest,
            SortOption::SizeAsc,
            SortOption::SizeDesc,
        ]
    }
}

// ============================================================================
// Library State Context
// ============================================================================

// Flag to prevent concurrent auto-repairs (primitive, not a signal)
thread_local! {
    static AUTO_REPAIR_DONE: std::cell::Cell<bool> = std::cell::Cell::new(false);
}

fn should_auto_repair() -> bool {
    AUTO_REPAIR_DONE.with(|cell| !cell.get())
}

fn mark_auto_repair_done() {
    AUTO_REPAIR_DONE.with(|cell| cell.set(true));
}

/// Shared library state that can be provided to child components
#[derive(Clone)]
pub struct LibraryState {
    pub documents: RwSignal<Vec<SourceDocument>>,
    pub search_results: RwSignal<Vec<SearchResult>>,
    pub search_meta: RwSignal<SearchMeta>,
    pub search_query: RwSignal<String>,
    pub selected_source_type: RwSignal<SourceType>,
    pub selected_document: RwSignal<Option<SearchResult>>,
    pub selected_source_doc: RwSignal<Option<SourceDocument>>,
    pub is_searching: RwSignal<bool>,
    pub is_ingesting: RwSignal<bool>,
    pub ingestion_progress: RwSignal<f32>,
    pub ingestion_status: RwSignal<String>,
    pub meilisearch_status: RwSignal<String>,
    pub view_mode: RwSignal<ViewMode>,
    pub sort_option: RwSignal<SortOption>,
    pub show_advanced_search: RwSignal<bool>,
    pub semantic_weight: RwSignal<f32>,
    pub keyword_weight: RwSignal<f32>,
    pub search_hints: RwSignal<Vec<String>>,
    pub is_drag_over: RwSignal<bool>,
    pub show_source_manager: RwSignal<bool>,
    pub editing_document: RwSignal<Option<SourceDocument>>,
    pub total_chunks: RwSignal<usize>,
}

impl LibraryState {
    pub fn new() -> Self {
        Self {
            documents: RwSignal::new(Vec::new()),
            search_results: RwSignal::new(Vec::new()),
            search_meta: RwSignal::new(SearchMeta::default()),
            search_query: RwSignal::new(String::new()),
            selected_source_type: RwSignal::new(SourceType::All),
            selected_document: RwSignal::new(None),
            selected_source_doc: RwSignal::new(None),
            is_searching: RwSignal::new(false),
            is_ingesting: RwSignal::new(false),
            ingestion_progress: RwSignal::new(0.0),
            ingestion_status: RwSignal::new(String::new()),
            meilisearch_status: RwSignal::new("Checking...".to_string()),
            view_mode: RwSignal::new(ViewMode::List),
            sort_option: RwSignal::new(SortOption::DateNewest),
            show_advanced_search: RwSignal::new(false),
            semantic_weight: RwSignal::new(0.5),
            keyword_weight: RwSignal::new(0.5),
            search_hints: RwSignal::new(Vec::new()),
            is_drag_over: RwSignal::new(false),
            show_source_manager: RwSignal::new(false),
            editing_document: RwSignal::new(None),
            total_chunks: RwSignal::new(0),
        }
    }
}

/// Provide library state context
pub fn provide_library_state() {
    provide_context(LibraryState::new());
}

/// Get library state from context
pub fn use_library_state() -> LibraryState {
    expect_context::<LibraryState>()
}

// ============================================================================
// Main Library Component
// ============================================================================

/// Enhanced Library page component with hybrid search and source management
#[component]
pub fn Library() -> impl IntoView {
    // Provide shared state
    let state = LibraryState::new();
    provide_context(state.clone());

    // Initialize Meilisearch status on mount
    Effect::new({
        let meilisearch_status = state.meilisearch_status;
        move |_| {
            spawn_local(async move {
                match check_meilisearch_health().await {
                    Ok(status) => {
                        if status.healthy {
                            let doc_count = status
                                .document_counts
                                .as_ref()
                                .map(|c| c.values().sum::<u64>().to_string())
                                .unwrap_or_else(|| "0".to_string());
                            meilisearch_status.set(format!("Healthy: {} docs indexed", doc_count));
                        } else {
                            meilisearch_status.set("Offline".to_string());
                        }
                    }
                    Err(e) => {
                        meilisearch_status.set(format!("Error: {}", e));
                    }
                }
            });
        }
    });

    // Load persisted documents from Meilisearch on mount
    // Auto-repair: if library is empty but content exists, rebuild metadata
    Effect::new({
        let documents = state.documents;
        let total_chunks = state.total_chunks;
        let ingestion_status = state.ingestion_status;
        move |_| {
            spawn_local(async move {
                // Helper to convert LibraryDocument to SourceDocument
                let convert_docs = |docs: Vec<LibraryDocument>| -> Vec<SourceDocument> {
                    docs.into_iter()
                        .map(|d| SourceDocument {
                            id: d.id,
                            name: d.name,
                            source_type: SourceType::from_str(&d.source_type),
                            status: match d.status.as_str() {
                                "ready" | "indexed" => DocumentStatus::Indexed,
                                "pending" => DocumentStatus::Pending,
                                "processing" => DocumentStatus::Indexing,
                                _ => DocumentStatus::Failed,
                            },
                            chunk_count: d.chunk_count as usize,
                            page_count: d.page_count as usize,
                            file_size_bytes: 0,
                            ingested_at: Some(d.ingested_at),
                            file_path: d.file_path,
                            description: None,
                            tags: Vec::new(),
                        })
                        .collect()
                };

                match list_library_documents().await {
                    Ok(docs) => {
                        if docs.is_empty() && should_auto_repair() {
                            // Library metadata is empty - check if we have indexed content
                            // and auto-repair if so (only once per session)
                            mark_auto_repair_done(); // Prevent re-running on subsequent mounts

                            if let Ok(health) = check_meilisearch_health().await {
                                let total_indexed: u64 = health.document_counts
                                    .as_ref()
                                    .map(|c| c.values().sum())
                                    .unwrap_or(0);

                                if total_indexed > 0 {
                                    log::info!("Library empty but {} docs indexed, auto-repairing...", total_indexed);
                                    ingestion_status.set("Recovering library metadata...".to_string());

                                    // Auto-repair
                                    if let Ok(count) = rebuild_library_metadata().await {
                                        if count > 0 {
                                            log::info!("Auto-repaired {} documents", count);
                                            // Reload the list
                                            if let Ok(repaired_docs) = list_library_documents().await {
                                                let source_docs = convert_docs(repaired_docs);
                                                let chunks: usize = source_docs.iter().map(|d| d.chunk_count).sum();
                                                documents.set(source_docs);
                                                total_chunks.set(chunks);
                                                ingestion_status.set(format!("Recovered {} documents", count));
                                            }
                                        } else {
                                            ingestion_status.set(String::new());
                                        }
                                    }
                                }
                            }
                        } else if !docs.is_empty() {
                            let source_docs = convert_docs(docs);
                            let chunks: usize = source_docs.iter().map(|d| d.chunk_count).sum();
                            documents.set(source_docs);
                            total_chunks.set(chunks);
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to load library documents: {}", e);
                    }
                }
            });
        }
    });

    // Set up event listener for progress updates
    // This automatically shows/hides the progress bar based on backend events
    Effect::new({
        let ingestion_progress = state.ingestion_progress;
        let ingestion_status = state.ingestion_status;
        let is_ingesting = state.is_ingesting;
        move |_| {
            let _ = listen_event("ingest-progress", move |event: JsValue| {
                if let Ok(payload) = js_sys::Reflect::get(&event, &JsValue::from_str("payload")) {
                    // Extract progress value
                    let progress = js_sys::Reflect::get(&payload, &JsValue::from_str("progress"))
                        .ok()
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);

                    // Extract stage
                    let stage = js_sys::Reflect::get(&payload, &JsValue::from_str("stage"))
                        .ok()
                        .and_then(|v| v.as_string())
                        .unwrap_or_default();

                    // Extract message
                    let message = js_sys::Reflect::get(&payload, &JsValue::from_str("message"))
                        .ok()
                        .and_then(|v| v.as_string());

                    // Update progress and status
                    ingestion_progress.set(progress as f32);
                    if let Some(msg) = message {
                        ingestion_status.set(msg);
                    }

                    // Auto-manage is_ingesting based on stage/progress
                    if stage == "complete" || progress >= 1.0 {
                        is_ingesting.set(false);
                    } else if progress > 0.0 {
                        is_ingesting.set(true);
                    }
                }
            });
        }
    });

    // Handlers
    let handle_refresh = {
        let meilisearch_status = state.meilisearch_status;
        let ingestion_status = state.ingestion_status;
        move |_: ev::MouseEvent| {
            spawn_local(async move {
                match check_meilisearch_health().await {
                    Ok(status) => {
                        if status.healthy {
                            let doc_count = status
                                .document_counts
                                .as_ref()
                                .map(|c| c.values().sum::<u64>().to_string())
                                .unwrap_or_else(|| "0".to_string());
                            meilisearch_status.set(format!("Healthy: {} docs indexed", doc_count));
                            ingestion_status.set("Status refreshed".to_string());
                        } else {
                            meilisearch_status.set("Offline".to_string());
                        }
                    }
                    Err(e) => {
                        meilisearch_status.set(format!("Error: {}", e));
                    }
                }
            });
        }
    };

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
                    ingestion_status.set(format!("Starting {}...", filename));

                    let source_type = selected_source_type.get_untracked();

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
                                ingested_at: Some(chrono_now()),
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
                            ingestion_status.set(format!("Failed: {}", e));
                            ingestion_progress.set(0.0);
                            show_error(
                                "Ingestion Failed",
                                Some(&format!("Could not process {}.\nReason: {}\n\nCheck file permissions or format.", filename, e)),
                                None
                            );
                        }
                    }
                    is_ingesting.set(false);
                }
            });
        }
    };

    let toggle_source_manager = {
        let show = state.show_source_manager;
        move |_: ev::MouseEvent| {
            show.update(|v| *v = !*v);
        }
    };

    // View mode toggle
    let toggle_view_mode = {
        let view_mode = state.view_mode;
        move |_: ev::MouseEvent| {
            view_mode.update(|v| {
                *v = match v {
                    ViewMode::Grid => ViewMode::List,
                    ViewMode::List => ViewMode::Grid,
                };
            });
        }
    };

    view! {
        <div class="flex flex-col h-full bg-[var(--bg-deep)] text-[var(--text-primary)] overflow-hidden">
            // Header
            <header class="flex-shrink-0 px-6 py-4 border-b border-[var(--border-subtle)] bg-[var(--bg-surface)]">
                <div class="flex items-center justify-between">
                    <div class="flex items-center gap-4">
                        <a
                            href="/"
                            class="text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors text-sm flex items-center gap-1"
                        >
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
                            </svg>
                            "Back"
                        </a>
                        <div class="h-6 w-px bg-[var(--border-subtle)]"></div>
                        <h1 class="text-2xl font-bold text-[var(--text-primary)]">"Library"</h1>
                        <StatusBadge status=state.meilisearch_status />
                    </div>

                    <div class="flex items-center gap-3">
                        // View mode toggle
                        <button
                            class="p-2 rounded-lg bg-[var(--bg-elevated)] hover:bg-[var(--bg-surface)] text-[var(--text-muted)] transition-colors"
                            title=move || if state.view_mode.get() == ViewMode::Grid { "Switch to List View" } else { "Switch to Grid View" }
                            on:click=toggle_view_mode
                        >
                            {move || if state.view_mode.get() == ViewMode::Grid {
                                view! {
                                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16" />
                                    </svg>
                                }.into_any()
                            } else {
                                view! {
                                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
                                    </svg>
                                }.into_any()
                            }}
                        </button>

                        <Button
                            variant=ButtonVariant::Ghost
                            on_click=handle_refresh
                            class="p-2"
                            title="Refresh search index status"
                        >
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                            </svg>
                            "Refresh"
                        </Button>

                        <Button
                            variant=ButtonVariant::Secondary
                            on_click=toggle_source_manager
                        >
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                            </svg>
                            "Manage Sources"
                        </Button>

                        <Button
                            variant=ButtonVariant::Primary
                            on_click=handle_ingest
                            disabled=Signal::derive(move || state.is_ingesting.get())
                            loading=Signal::derive(move || state.is_ingesting.get())
                            class="flex items-center gap-2"
                            title="Import a new document into the library"
                        >
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
                            </svg>
                            {move || if state.is_ingesting.get() { "Ingesting..." } else { "Ingest Document" }}
                        </Button>
                    </div>
                </div>
            </header>

            // Main Content
            <div class="flex-1 flex overflow-hidden">
                // Left Panel: Search & Results
                <div class="flex-1 flex flex-col overflow-hidden">
                    // Search Panel
                    <SearchPanel />

                    // Results/Documents List
                    <DocumentList />
                </div>

                // Right Panel: Document Detail / Sources
                <aside class="w-[420px] flex-shrink-0 border-l border-[var(--border-subtle)] flex flex-col overflow-hidden bg-[var(--bg-surface)]">
                    {move || {
                        if state.show_source_manager.get() {
                            view! { <SourceManager /> }.into_any()
                        } else if state.selected_document.get().is_some() {
                            view! { <DocumentDetail /> }.into_any()
                        } else {
                            view! { <LibrarySidebar /> }.into_any()
                        }
                    }}
                </aside>
            </div>
        </div>
    }
}

// ============================================================================
// Helper Components
// ============================================================================

/// Status badge for Meilisearch connection
#[component]
fn StatusBadge(status: RwSignal<String>) -> impl IntoView {
    view! {
        <span class=move || {
            let status_text = status.get();
            let base = "text-xs px-3 py-1 rounded-full font-medium";
            if status_text.contains("Healthy") {
                format!("{} bg-green-900/50 text-green-400 border border-green-500/30", base)
            } else if status_text.contains("Checking") {
                format!("{} bg-yellow-900/50 text-yellow-400 border border-yellow-500/30", base)
            } else {
                format!("{} bg-red-900/50 text-red-400 border border-red-500/30", base)
            }
        }>
            {move || status.get()}
        </span>
    }
}

/// Default sidebar when no document is selected
#[component]
fn LibrarySidebar() -> impl IntoView {
    let state = use_library_state();

    view! {
        <div class="flex-1 flex flex-col overflow-hidden">
            <div class="flex-shrink-0 p-4 border-b border-[var(--border-subtle)]">
                <h2 class="text-lg font-semibold text-[var(--text-primary)]">"Library Overview"</h2>
            </div>

            <div class="flex-1 overflow-y-auto p-4 space-y-6">
                // Ingestion Progress - always rendered when ingesting, reactively updated
                <Show when=move || state.is_ingesting.get()>
                    <Card>
                        <CardBody>
                            <div class="space-y-2">
                                <div class="flex justify-between text-sm">
                                    <span class="text-[var(--text-muted)]">
                                        {move || {
                                            let progress = (state.ingestion_progress.get() * 100.0) as u32;
                                            if progress < 30 {
                                                "Parsing document..."
                                            } else if progress < 50 {
                                                "Extracting text..."
                                            } else if progress < 70 {
                                                "Chunking content..."
                                            } else if progress < 90 {
                                                "Generating embeddings..."
                                            } else if progress < 100 {
                                                "Indexing..."
                                            } else {
                                                "Complete!"
                                            }
                                        }}
                                    </span>
                                    <span class="font-mono text-[var(--accent)]">
                                        {move || format!("{}%", (state.ingestion_progress.get() * 100.0) as u32)}
                                    </span>
                                </div>
                                <div class="w-full bg-[var(--bg-deep)] rounded-full h-2 overflow-hidden">
                                    <div
                                        class="bg-[var(--accent)] h-2 rounded-full transition-all duration-300"
                                        style:width=move || format!("{}%", (state.ingestion_progress.get() * 100.0) as u32)
                                    />
                                </div>
                                // Show status message from backend
                                <Show when=move || !state.ingestion_status.get().is_empty()>
                                    <p class="text-xs text-[var(--text-muted)] mt-1 truncate">
                                        {move || state.ingestion_status.get()}
                                    </p>
                                </Show>
                            </div>
                        </CardBody>
                    </Card>
                </Show>

                // Statistics
                <Card>
                    <CardHeader>
                        <h3 class="font-medium text-[var(--text-primary)]">"Statistics"</h3>
                    </CardHeader>
                    <CardBody class="space-y-3">
                        <div class="flex justify-between items-center">
                            <span class="text-[var(--text-muted)] text-sm">"Documents"</span>
                            <span class="font-mono text-[var(--text-primary)]">{move || state.documents.get().len()}</span>
                        </div>
                        <div class="flex justify-between items-center">
                            <span class="text-[var(--text-muted)] text-sm">"Total Chunks"</span>
                            <span class="font-mono text-[var(--text-primary)]">{move || state.total_chunks.get()}</span>
                        </div>
                        <div class="flex justify-between items-center">
                            <span class="text-[var(--text-muted)] text-sm">"Search Results"</span>
                            <span class="font-mono text-[var(--text-primary)]">{move || state.search_results.get().len()}</span>
                        </div>
                    </CardBody>
                </Card>

                // Recent Documents
                <Card>
                    <CardHeader>
                        <h3 class="font-medium text-[var(--text-primary)]">"Recent Documents"</h3>
                    </CardHeader>
                    <CardBody>
                        {move || {
                            let docs = state.documents.get();
                            if docs.is_empty() {
                                view! {
                                    <div class="text-center py-8">
                                        <svg class="w-12 h-12 mx-auto text-[var(--text-muted)] opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                                        </svg>
                                        <p class="mt-2 text-sm text-[var(--text-muted)]">"No documents yet"</p>
                                        <p class="text-xs text-[var(--text-muted)] opacity-70">"Click 'Ingest Document' to add files"</p>
                                    </div>
                                }.into_any()
                            } else {
                                let recent: Vec<_> = docs.into_iter().rev().take(5).collect();
                                view! {
                                    <div class="space-y-2">
                                        {recent.into_iter().map(|doc| {
                                            view! {
                                                <div class="p-2 rounded-lg bg-[var(--bg-elevated)] hover:bg-[var(--bg-deep)] transition-colors cursor-pointer">
                                                    <div class="flex items-center gap-2">
                                                        <span class="text-lg">{doc.source_type.icon()}</span>
                                                        <span class="text-sm font-medium text-[var(--text-primary)] truncate flex-1">{doc.name.clone()}</span>
                                                    </div>
                                                    <div class="flex items-center gap-2 mt-1 text-xs text-[var(--text-muted)]">
                                                        <span>{format!("{} pages", doc.page_count)}</span>
                                                        <span>"•"</span>
                                                        <Badge variant=doc.status.badge_variant()>{doc.status.as_str()}</Badge>
                                                    </div>
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                }.into_any()
                            }
                        }}
                    </CardBody>
                </Card>

                // Supported Formats
                <Card>
                    <CardHeader>
                        <h3 class="font-medium text-[var(--text-primary)]">"Supported Formats"</h3>
                    </CardHeader>
                    <CardBody>
                        <div class="grid grid-cols-3 gap-2">
                            <div class="text-center"><Badge variant=BadgeVariant::Success class="w-full justify-center">"PDF"</Badge></div>
                            <div class="text-center"><Badge variant=BadgeVariant::Success class="w-full justify-center">"EPUB"</Badge></div>
                            <div class="text-center"><Badge variant=BadgeVariant::Success class="w-full justify-center">"MOBI"</Badge></div>
                            <div class="text-center"><Badge variant=BadgeVariant::Success class="w-full justify-center">"AZW3"</Badge></div>
                            <div class="text-center"><Badge variant=BadgeVariant::Success class="w-full justify-center">"DOCX"</Badge></div>
                            <div class="text-center"><Badge variant=BadgeVariant::Success class="w-full justify-center">"MD"</Badge></div>
                            <div class="text-center col-span-3"><Badge variant=BadgeVariant::Success class="w-full justify-center">"TXT"</Badge></div>
                        </div>
                    </CardBody>
                </Card>

                // Quick Tips
                <Card>
                    <CardHeader>
                        <h3 class="font-medium text-[var(--text-primary)]">"Search Tips"</h3>
                    </CardHeader>
                    <CardBody class="space-y-2 text-sm text-[var(--text-muted)]">
                        <p>"• Use abbreviations: HP, AC, DC, CR"</p>
                        <p>"• Search spells, monsters, items by name"</p>
                        <p>"• Filter by source type for focused results"</p>
                        <p>"• Adjust semantic/keyword weights in advanced mode"</p>
                    </CardBody>
                </Card>
            </div>
        </div>
    }
}

/// Simple timestamp generator (without chrono dependency)
fn chrono_now() -> String {
    // In production, this would use chrono or js_sys::Date
    "2024-01-15T12:00:00Z".to_string()
}
