//! Library component for document ingestion and search
//!
//! Provides:
//! - Document ingestion with file picker
//! - Progress events during ingestion
//! - Keyword and semantic search interface
//! - Results display with source info

use leptos::prelude::*;
use leptos::ev;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;

use crate::bindings::{
    check_meilisearch_health, ingest_document_with_progress, listen_event,
    pick_document_file, search, SearchOptions,
};
use crate::components::design_system::{Badge, BadgeVariant, Button, ButtonVariant, Card, CardBody, CardHeader, Input};

/// Represents a source document in the library
#[derive(Clone, PartialEq)]
pub struct SourceDocument {
    pub name: String,
    pub status: String,
    pub chunk_count: usize,
    pub page_count: usize,
}

/// Search result for display
#[derive(Clone, PartialEq)]
struct SearchResult {
    title: String,
    snippet: String,
    source: String,
    score: f32,
}

/// Library page component for document ingestion and search
#[component]
pub fn Library() -> impl IntoView {
    // State signals
    let ingestion_status = RwSignal::new(String::new());
    let ingestion_progress = RwSignal::new(0.0_f32);
    let documents = RwSignal::new(Vec::<SourceDocument>::new());
    let total_chunks = RwSignal::new(0_usize);
    let is_ingesting = RwSignal::new(false);
    let search_query = RwSignal::new(String::new());
    let is_drag_over = RwSignal::new(false);
    let search_results = RwSignal::new(Vec::<SearchResult>::new());
    let is_searching = RwSignal::new(false);
    let meilisearch_status = RwSignal::new("Checking...".to_string());

    // Fetch Meilisearch status on mount
    Effect::new(move |_| {
        spawn_local(async move {
            match check_meilisearch_health().await {
                Ok(status) => {
                    if status.healthy {
                        let doc_count = status
                            .document_counts
                            .as_ref()
                            .map(|c| c.values().sum::<u64>().to_string())
                            .unwrap_or_else(|| "0".to_string());
                        meilisearch_status.set(format!("Meilisearch: {} docs", doc_count));
                    } else {
                        meilisearch_status.set("Meilisearch: Offline".to_string());
                    }
                }
                Err(e) => {
                    meilisearch_status.set(format!("Error: {}", e));
                }
            }
        });
    });

    // Set up event listener for progress updates on mount
    Effect::new(move |_| {
        let _ = listen_event("ingest-progress", move |event: JsValue| {
            // Extract payload from event
            if let Ok(payload) = js_sys::Reflect::get(&event, &JsValue::from_str("payload")) {
                if let Ok(progress_val) =
                    js_sys::Reflect::get(&payload, &JsValue::from_str("progress"))
                {
                    if let Some(progress) = progress_val.as_f64() {
                        ingestion_progress.set(progress as f32);
                    }
                }
                if let Ok(message_val) =
                    js_sys::Reflect::get(&payload, &JsValue::from_str("message"))
                {
                    if let Some(message) = message_val.as_string() {
                        ingestion_status.set(message);
                    }
                }
            }
        });
    });

    // Handle document ingestion
    let handle_ingest = move |_: ev::MouseEvent| {
        spawn_local(async move {
            // Open file picker dialog
            if let Some(path) = pick_document_file().await {
                is_ingesting.set(true);
                ingestion_progress.set(0.0);
                let filename = path.split('/').last().unwrap_or(&path).to_string();
                ingestion_status.set(format!("Starting {}...", filename));

                // Use the progress-reporting ingestion
                match ingest_document_with_progress(path.clone(), Some("documents".to_string()))
                    .await
                {
                    Ok(result) => {
                        let doc = SourceDocument {
                            name: result.source_name.clone(),
                            status: "Indexed".to_string(),
                            chunk_count: result.character_count / 500,
                            page_count: result.page_count,
                        };
                        documents.update(|docs| docs.push(doc));
                        total_chunks.update(|c| *c += result.character_count / 500);
                        ingestion_status.set(format!(
                            "Indexed {} ({} pages, {} chars)",
                            result.source_name, result.page_count, result.character_count
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
    };

    // Handle refresh status
    let handle_refresh = move |_: ev::MouseEvent| {
        spawn_local(async move {
            match check_meilisearch_health().await {
                Ok(status) => {
                    if status.healthy {
                        let doc_count = status
                            .document_counts
                            .as_ref()
                            .map(|c| c.values().sum::<u64>().to_string())
                            .unwrap_or_else(|| "0".to_string());
                        meilisearch_status.set(format!("Meilisearch: {} docs", doc_count));
                        ingestion_status.set("Refreshed".to_string());
                    } else {
                        meilisearch_status.set("Meilisearch: Offline".to_string());
                    }
                }
                Err(e) => {
                    meilisearch_status.set(format!("Error: {}", e));
                }
            }
        });
    };

    // Handle search
    let handle_search = move |_: ev::MouseEvent| {
        let query = search_query.get();
        if query.is_empty() {
            return;
        }

        is_searching.set(true);
        ingestion_status.set(format!("Searching for: '{}'...", query));

        spawn_local(async move {
            let options = SearchOptions {
                limit: 10,
                source_type: None,
                campaign_id: None,
                index: None,
            };

            match search(query.clone(), Some(options)).await {
                Ok(results) => {
                    let mapped_results: Vec<SearchResult> = results
                        .into_iter()
                        .map(|r| {
                            let snippet = if r.content.len() > 200 {
                                format!("{}...", &r.content[0..200])
                            } else {
                                r.content.clone()
                            };

                            SearchResult {
                                title: format!(
                                    "{} (p.{})",
                                    r.source,
                                    r.page_number.unwrap_or(0)
                                ),
                                snippet,
                                source: format!("{} / {}", r.source_type, r.index),
                                score: r.score,
                            }
                        })
                        .collect();

                    let count = mapped_results.len();
                    search_results.set(mapped_results);
                    ingestion_status.set(format!("Found {} results for: '{}'", count, query));
                }
                Err(e) => {
                    ingestion_status.set(format!("Search failed: {}", e));
                    search_results.set(Vec::new());
                }
            }
            is_searching.set(false);
        });
    };

    // Handle search on Enter key
    let handle_search_keydown = move |evt: ev::KeyboardEvent| {
        if evt.key() == "Enter" && !search_query.get().is_empty() {
            // Trigger search directly
            let query = search_query.get();
            is_searching.set(true);
            ingestion_status.set(format!("Searching for: '{}'...", query));

            spawn_local(async move {
                let options = SearchOptions {
                    limit: 10,
                    source_type: None,
                    campaign_id: None,
                    index: None,
                };

                match search(query.clone(), Some(options)).await {
                    Ok(results) => {
                        let mapped_results: Vec<SearchResult> = results
                            .into_iter()
                            .map(|r| {
                                let snippet = if r.content.len() > 200 {
                                    format!("{}...", &r.content[0..200])
                                } else {
                                    r.content.clone()
                                };

                                SearchResult {
                                    title: format!(
                                        "{} (p.{})",
                                        r.source,
                                        r.page_number.unwrap_or(0)
                                    ),
                                    snippet,
                                    source: format!("{} / {}", r.source_type, r.index),
                                    score: r.score,
                                }
                            })
                            .collect();

                        let count = mapped_results.len();
                        search_results.set(mapped_results);
                        ingestion_status.set(format!("Found {} results for: '{}'", count, query));
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

    view! {
        <div class="p-6 bg-[var(--bg-deep)] text-[var(--text-primary)] min-h-full font-sans overflow-y-auto">
            <div class="max-w-6xl">
                // Header
                <div class="flex items-center justify-between mb-8">
                    <div class="flex items-center">
                        <a href="/" class="mr-4 text-gray-400 hover:text-white transition-colors">
                            "<- Chat"
                        </a>
                        <h1 class="text-2xl font-bold">"Library & Ingestion"</h1>
                    </div>
                    <div class="flex gap-2">
                        <Button
                            variant=ButtonVariant::Secondary
                            on_click=handle_refresh
                        >
                            "Refresh"
                        </Button>
                        <Button
                            variant=ButtonVariant::Primary
                            on_click=handle_ingest
                            disabled=is_ingesting.get()
                            loading=is_ingesting.get()
                        >
                            {move || if is_ingesting.get() { "Processing..." } else { "Ingest Document" }}
                        </Button>
                    </div>
                </div>

                // Drag and Drop Zone
                <div
                    class=move || {
                        if is_drag_over.get() {
                            "border-2 border-dashed border-purple-400 bg-purple-900/20 rounded-lg p-8 text-center transition-colors"
                        } else {
                            "border-2 border-dashed border-gray-600 hover:border-gray-500 rounded-lg p-8 text-center transition-colors cursor-pointer"
                        }
                    }
                    on:dragover=move |e: ev::DragEvent| {
                        e.prevent_default();
                        is_drag_over.set(true);
                    }
                    on:dragleave=move |_| {
                        is_drag_over.set(false);
                    }
                    on:drop=move |e: ev::DragEvent| {
                        e.prevent_default();
                        is_drag_over.set(false);
                        ingestion_status.set("Drop detected! Use Tauri file dialog for now.".to_string());
                    }
                >
                    <div class="space-y-2">
                        <div class="text-4xl">
                            {move || if is_drag_over.get() { "+" } else { "^" }}
                        </div>
                        <p class=move || if is_drag_over.get() {
                            "text-purple-300 font-semibold"
                        } else {
                            "text-gray-400"
                        }>
                            {move || if is_drag_over.get() {
                                "Drop files here!"
                            } else {
                                "Drag & Drop files here"
                            }}
                        </p>
                        <p class="text-gray-500 text-sm">
                            "Or use the 'Ingest Document' button to select files"
                        </p>
                    </div>
                </div>

                // Main Grid: Sources & Status
                <div class="mt-6 grid grid-cols-1 lg:grid-cols-2 gap-4">
                    // Sources List
                    <Card>
                        <CardHeader>
                            <h2 class="text-xl font-semibold text-gray-200">"Source Materials"</h2>
                        </CardHeader>
                        <CardBody>
                            <div class="space-y-3 max-h-64 overflow-y-auto">
                                {move || {
                                    let docs = documents.get();
                                    if docs.is_empty() {
                                        view! {
                                            <div class="text-center text-gray-500 py-8">
                                                <p>"No documents ingested yet."</p>
                                                <p class="text-sm mt-2">"Click 'Ingest Document' to add files."</p>
                                            </div>
                                        }.into_any()
                                    } else {
                                        docs.into_iter().map(|doc| {
                                            let status_class = if doc.status == "Indexed" {
                                                "text-green-400 text-xs"
                                            } else {
                                                "text-yellow-400 text-xs"
                                            };
                                            view! {
                                                <div class="p-3 bg-gray-700 rounded flex justify-between items-center group">
                                                    <div class="flex-1">
                                                        <div class="flex items-center gap-2">
                                                            <span class="font-medium truncate">{doc.name.clone()}</span>
                                                            <span class=status_class>{doc.status.clone()}</span>
                                                        </div>
                                                        <div class="text-xs text-gray-400 mt-1">
                                                            {format!("{} pages, ~{} chunks", doc.page_count, doc.chunk_count)}
                                                        </div>
                                                    </div>
                                                </div>
                                            }
                                        }).collect_view().into_any()
                                    }
                                }}
                            </div>
                        </CardBody>
                    </Card>

                    // Stats / Status
                    <Card>
                        <CardHeader>
                            <h2 class="text-xl font-semibold text-gray-200">"System Status"</h2>
                        </CardHeader>
                        <CardBody>
                            <div class="space-y-3">
                                <div class="flex justify-between items-center p-2 bg-gray-700 rounded">
                                    <span class="text-gray-400">"Search Engine"</span>
                                    <span class=move || {
                                        if meilisearch_status.get().contains("docs") {
                                            "text-green-400 font-mono text-sm"
                                        } else {
                                            "text-red-400 font-mono text-sm"
                                        }
                                    }>
                                        {move || meilisearch_status.get()}
                                    </span>
                                </div>
                                <div class="flex justify-between items-center p-2 bg-gray-700 rounded">
                                    <span class="text-gray-400">"Total Chunks"</span>
                                    <span class="text-white font-mono text-sm">
                                        {move || total_chunks.get()}
                                    </span>
                                </div>
                                <div class="flex justify-between items-center p-2 bg-gray-700 rounded">
                                    <span class="text-gray-400">"Documents"</span>
                                    <span class="text-white font-mono text-sm">
                                        {move || documents.get().len()}
                                    </span>
                                </div>
                            </div>

                            // Status message and progress bar
                            {move || {
                                let status = ingestion_status.get();
                                let loading = is_ingesting.get();

                                if status.is_empty() && !loading {
                                    return view! { <div /> }.into_any();
                                }

                                let status_class = if status.contains("Error") {
                                    "text-sm text-red-400"
                                } else if status.contains("Indexed") {
                                    "text-sm text-green-400"
                                } else {
                                    "text-sm text-blue-400"
                                };

                                let progress_percent = (ingestion_progress.get() * 100.0) as u32;

                                view! {
                                    <div class="mt-4 pt-4 border-t border-gray-700">
                                        <p class=status_class>{status.clone()}</p>
                                        {if loading {
                                            let progress_style = format!("width: {}%", progress_percent);
                                            let stage_text = if progress_percent < 40 {
                                                "Parsing..."
                                            } else if progress_percent < 60 {
                                                "Chunking..."
                                            } else if progress_percent < 100 {
                                                "Indexing..."
                                            } else {
                                                "Complete!"
                                            };
                                            Some(view! {
                                                <div class="mt-3">
                                                    <div class="flex justify-between text-xs text-gray-400 mb-1">
                                                        <span>{format!("{}%", progress_percent)}</span>
                                                        <span>{stage_text}</span>
                                                    </div>
                                                    <div class="w-full bg-gray-600 rounded-full h-2.5">
                                                        <div
                                                            class="bg-purple-500 h-2.5 rounded-full transition-all duration-300"
                                                            style=progress_style
                                                        />
                                                    </div>
                                                </div>
                                            })
                                        } else {
                                            None
                                        }}
                                    </div>
                                }.into_any()
                            }}
                        </CardBody>
                    </Card>
                </div>

                // Search Section
                <Card class="mt-6">
                    <CardHeader>
                        <h2 class="text-xl font-semibold text-gray-200">"Federated Search"</h2>
                    </CardHeader>
                    <CardBody>
                        <div class="flex gap-2">
                            <Input
                                value=search_query
                                placeholder="Search your library..."
                                on_keydown=Callback::new(handle_search_keydown)
                                class="flex-1"
                            />
                            <Button
                                variant=ButtonVariant::Primary
                                on_click=handle_search
                                disabled=is_searching.get()
                                loading=is_searching.get()
                                class="bg-purple-600 hover:bg-purple-500"
                            >
                                {move || if is_searching.get() { "Searching..." } else { "Search" }}
                            </Button>
                        </div>
                        <p class="text-xs text-gray-500 mt-2">
                            "Federated search across all indexes with typo tolerance and semantic matching."
                        </p>

                        // Search Results
                        {move || {
                            let results = search_results.get();
                            if results.is_empty() {
                                return view! { <div /> }.into_any();
                            }

                            view! {
                                <div class="mt-4 space-y-2">
                                    {results.into_iter().map(|result| {
                                        view! {
                                            <div class="p-3 bg-gray-700 rounded">
                                                <div class="flex justify-between items-start">
                                                    <h3 class="font-medium text-purple-300">{result.title.clone()}</h3>
                                                    <span class="text-xs text-gray-500">
                                                        {format!("Score: {:.2}", result.score)}
                                                    </span>
                                                </div>
                                                <p class="text-sm text-gray-300 mt-1">{result.snippet.clone()}</p>
                                                <span class="text-xs text-gray-500">{result.source.clone()}</span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            }.into_any()
                        }}
                    </CardBody>
                </Card>

                // Supported Formats
                <Card class="mt-6">
                    <CardHeader>
                        <h2 class="text-xl font-semibold text-gray-200">"Supported Formats"</h2>
                    </CardHeader>
                    <CardBody>
                        <div class="flex flex-wrap gap-2">
                            <Badge variant=BadgeVariant::Success>"PDF"</Badge>
                            <Badge variant=BadgeVariant::Success>"EPUB"</Badge>
                            <Badge variant=BadgeVariant::Success>"MOBI/AZW"</Badge>
                            <Badge variant=BadgeVariant::Success>"DOCX"</Badge>
                            <Badge variant=BadgeVariant::Success>"Markdown"</Badge>
                            <Badge variant=BadgeVariant::Success>"TXT"</Badge>
                        </div>
                    </CardBody>
                </Card>
            </div>
        </div>
    }
}
