#![allow(non_snake_case)]
use dioxus::prelude::*;
use crate::bindings::{search, SearchOptions, check_meilisearch_health, pick_document_file, ingest_document_with_progress, IngestProgress};

#[derive(Clone, PartialEq)]
pub struct SourceDocument {
    pub name: String,
    pub status: String,
    pub status_class: String,
    pub chunk_count: usize,
    pub page_count: usize,
}

#[component]
pub fn Library() -> Element {
    let mut ingestion_status = use_signal(|| String::new());
    let mut ingestion_progress = use_signal(|| 0.0_f32);
    let mut documents = use_signal(|| Vec::<SourceDocument>::new());
    let mut total_chunks = use_signal(|| 0_usize);

    // Fetch Meilisearch status
    let status_resource = use_resource(move || async move {
        match check_meilisearch_health().await {
            Ok(status) => {
                if status.healthy {
                    format!("Meilisearch: {} docs",
                        status.document_counts.as_ref()
                            .map(|c| c.values().sum::<u64>().to_string())
                            .unwrap_or_else(|| "0".to_string()))
                } else {
                    "Meilisearch: Offline".to_string()
                }
            }
            Err(e) => format!("Error: {}", e)
        }
    });

    let current_status = status_resource.read().clone().unwrap_or_else(|| "Checking...".to_string());

    let mut is_ingesting = use_signal(|| false);
    let mut search_query = use_signal(|| String::new());
    let mut is_drag_over = use_signal(|| false);
    let mut search_results = use_signal(|| Vec::<SearchResult>::new());
    let mut is_searching = use_signal(|| false);

    // Set up event listener for progress updates
    use_effect(move || {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;
        use std::cell::RefCell;
        use std::rc::Rc;

        // Wrap signals in Rc<RefCell> for interior mutability
        let progress_signal = Rc::new(RefCell::new(ingestion_progress));
        let status_signal = Rc::new(RefCell::new(ingestion_status));

        let progress_clone = progress_signal.clone();
        let status_clone = status_signal.clone();

        let callback = Closure::wrap(Box::new(move |event: JsValue| {
            // Extract payload from event
            if let Ok(payload) = js_sys::Reflect::get(&event, &JsValue::from_str("payload")) {
                if let Ok(progress_val) = js_sys::Reflect::get(&payload, &JsValue::from_str("progress")) {
                    if let Some(progress) = progress_val.as_f64() {
                        progress_clone.borrow_mut().set(progress as f32);
                    }
                }
                if let Ok(message_val) = js_sys::Reflect::get(&payload, &JsValue::from_str("message")) {
                    if let Some(message) = message_val.as_string() {
                        status_clone.borrow_mut().set(message);
                    }
                }
            }
        }) as Box<dyn Fn(JsValue)>);

        // Get the listen function from Tauri
        if let Some(window) = web_sys::window() {
            if let Ok(tauri) = js_sys::Reflect::get(&window, &JsValue::from_str("__TAURI__")) {
                if let Ok(event_module) = js_sys::Reflect::get(&tauri, &JsValue::from_str("event")) {
                    if let Ok(listen_fn) = js_sys::Reflect::get(&event_module, &JsValue::from_str("listen")) {
                        if let Some(listen) = listen_fn.dyn_ref::<js_sys::Function>() {
                            let _ = listen.call2(
                                &JsValue::NULL,
                                &JsValue::from_str("ingest-progress"),
                                callback.as_ref().unchecked_ref(),
                            );
                        }
                    }
                }
            }
        }

        callback.forget();
    });

    let handle_ingest = move |_: MouseEvent| {
        spawn(async move {
            // Open file picker dialog
            if let Some(path) = pick_document_file().await {
                is_ingesting.set(true);
                ingestion_progress.set(0.0);
                let filename = path.split('/').last().unwrap_or(&path).to_string();
                ingestion_status.set(format!("Starting {}...", filename));

                // Use the progress-reporting ingestion
                match ingest_document_with_progress(path.clone(), Some("documents".to_string())).await {
                    Ok(result) => {
                        let doc = SourceDocument {
                            name: result.source_name.clone(),
                            status: "Indexed".to_string(),
                            status_class: "text-green-400 text-xs".to_string(),
                            chunk_count: result.character_count / 500,
                            page_count: result.page_count,
                        };
                        documents.write().push(doc);
                        *total_chunks.write() += result.character_count / 500;
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

    let refresh_status = move |_: MouseEvent| {
        ingestion_status.set("Refreshed".to_string());
    };

    let handle_search = move |_: MouseEvent| {
        let query = search_query.read().clone();
        if query.is_empty() {
            return;
        }

        is_searching.set(true);
        ingestion_status.set(format!("Searching for: '{}'...", query));

        spawn(async move {
            let options = SearchOptions {
                limit: 10,
                source_type: None,
                campaign_id: None,
                index: None,
            };

            match search(query.clone(), Some(options)).await {
                Ok(results) => {
                    let mapped_results: Vec<SearchResult> = results.into_iter().map(|r| {
                        let snippet = if r.content.len() > 200 {
                            format!("{}...", &r.content[0..200])
                        } else {
                            r.content.clone()
                        };

                        SearchResult {
                            title: format!("{} (p.{})", r.source, r.page_number.unwrap_or(0)),
                            snippet,
                            source: format!("{} / {}", r.source_type, r.index),
                            score: r.score,
                        }
                    }).collect();

                    let count = mapped_results.len();
                    search_results.set(mapped_results);
                    ingestion_status.set(format!("Found {} results for: '{}'", count, query));
                },
                Err(e) => {
                    ingestion_status.set(format!("Search failed: {}", e));
                    search_results.set(Vec::new());
                }
            }
            is_searching.set(false);
        });
    };

    let loading = *is_ingesting.read();
    let drag_active = *is_drag_over.read();

    let button_class: &str = if loading {
        "px-4 py-2 bg-gray-600 rounded cursor-not-allowed"
    } else {
        "px-4 py-2 bg-blue-600 rounded hover:bg-blue-500 transition-colors"
    };
    let button_text: &str = if loading { "Processing..." } else { "Ingest Document" };

    let status_text = ingestion_status.read().clone();
    let status_class: &str = if status_text.contains("Error") {
        "text-sm text-red-400"
    } else if status_text.contains("Ingested") || status_text.contains("Indexed") {
        "text-sm text-green-400"
    } else {
        "text-sm text-blue-400"
    };

    let doc_count = documents.read().len();
    let chunk_count = *total_chunks.read();
    let progress_percent = (*ingestion_progress.read() * 100.0) as u32;

    let drop_zone_class = if drag_active {
        "border-2 border-dashed border-purple-400 bg-purple-900/20 rounded-lg p-8 text-center transition-colors"
    } else {
        "border-2 border-dashed border-gray-600 hover:border-gray-500 rounded-lg p-8 text-center transition-colors cursor-pointer"
    };

    rsx! {
        div {
            class: "p-8 bg-gray-900 text-white min-h-screen font-sans",
            div {
                class: "max-w-4xl mx-auto",
                // Header
                div {
                    class: "flex items-center justify-between mb-8",
                    div {
                        class: "flex items-center",
                        Link { to: crate::Route::Chat {}, class: "mr-4 text-gray-400 hover:text-white", "<- Chat" }
                        h1 { class: "text-2xl font-bold", "Library & Ingestion" }
                    }
                    div {
                        class: "flex gap-2",
                        button {
                            onclick: refresh_status,
                            class: "px-4 py-2 bg-gray-600 rounded hover:bg-gray-500 transition-colors",
                            "Refresh"
                        }
                        button {
                            onclick: handle_ingest,
                            class: "{button_class}",
                            "{button_text}"
                        }
                    }
                }

                // Drag and Drop Zone
                div {
                    class: "{drop_zone_class}",
                    ondragover: move |e| {
                        e.prevent_default();
                        is_drag_over.set(true);
                    },
                    ondragleave: move |_| {
                        is_drag_over.set(false);
                    },
                    ondrop: move |e| {
                        e.prevent_default();
                        is_drag_over.set(false);
                        // Note: In Tauri with WASM, file drops are handled via Tauri events
                        // The browser drag/drop API doesn't give us file paths for security
                        ingestion_status.set("Drop detected! Use Tauri file dialog for now.".to_string());
                    },
                    div {
                        class: "space-y-2",
                        div {
                            class: "text-4xl",
                            if drag_active { "+" } else { "^" }
                        }
                        p {
                            class: if drag_active { "text-purple-300 font-semibold" } else { "text-gray-400" },
                            if drag_active {
                                "Drop files here!"
                            } else {
                                "Drag & Drop files here"
                            }
                        }
                        p {
                            class: "text-gray-500 text-sm",
                            "Or use the 'Ingest Document' button to select files"
                        }
                    }
                }

                div {
                    class: "mt-6 grid grid-cols-1 md:grid-cols-2 gap-6",
                    // Sources List
                    div {
                        class: "bg-gray-800 rounded-lg p-6",
                        h2 { class: "text-xl font-semibold mb-4 text-gray-200", "Source Materials" }
                        div {
                            class: "space-y-3 max-h-64 overflow-y-auto",
                            for doc in documents.read().iter() {
                                div {
                                    key: "{doc.name}",
                                    class: "p-3 bg-gray-700 rounded flex justify-between items-center group",
                                    div {
                                        class: "flex-1",
                                        div {
                                            class: "flex items-center gap-2",
                                            span { class: "font-medium truncate", "{doc.name}" }
                                            span { class: "{doc.status_class}", "{doc.status}" }
                                        }
                                        div {
                                            class: "text-xs text-gray-400 mt-1",
                                            "{doc.page_count} pages, ~{doc.chunk_count} chunks"
                                        }
                                    }
                                }
                            }
                            if documents.read().is_empty() {
                                div {
                                    class: "text-center text-gray-500 py-8",
                                    p { "No documents ingested yet." }
                                    p { class: "text-sm mt-2", "Click 'Ingest Document' to add files." }
                                }
                            }
                        }
                    }

                    // Stats / Status
                    div {
                        class: "bg-gray-800 rounded-lg p-6",
                        h2 { class: "text-xl font-semibold mb-4 text-gray-200", "System Status" }
                        div {
                            class: "space-y-3",
                            div {
                                class: "flex justify-between items-center p-2 bg-gray-700 rounded",
                                span { class: "text-gray-400", "Search Engine" }
                                span {
                                    class: if current_status.contains("docs") { "text-green-400 font-mono text-sm" } else { "text-red-400 font-mono text-sm" },
                                    "{current_status}"
                                }
                            }
                            div {
                                class: "flex justify-between items-center p-2 bg-gray-700 rounded",
                                span { class: "text-gray-400", "Total Chunks" }
                                span { class: "text-white font-mono text-sm", "{chunk_count}" }
                            }
                            div {
                                class: "flex justify-between items-center p-2 bg-gray-700 rounded",
                                span { class: "text-gray-400", "Documents" }
                                span { class: "text-white font-mono text-sm", "{doc_count}" }
                            }
                        }
                        if !status_text.is_empty() || loading {
                            div {
                                class: "mt-4 pt-4 border-t border-gray-700",
                                p { class: "{status_class}", "{status_text}" }

                                // Progress bar during ingestion
                                if loading {
                                    div {
                                        class: "mt-3",
                                        div {
                                            class: "flex justify-between text-xs text-gray-400 mb-1",
                                            span { "{progress_percent}%" }
                                            span {
                                                if progress_percent < 40 { "Parsing..." }
                                                else if progress_percent < 60 { "Chunking..." }
                                                else if progress_percent < 100 { "Indexing..." }
                                                else { "Complete!" }
                                            }
                                        }
                                        div {
                                            class: "w-full bg-gray-600 rounded-full h-2.5",
                                            div {
                                                class: "bg-purple-500 h-2.5 rounded-full transition-all duration-300",
                                                style: "width: {progress_percent}%",
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Search Section
                div {
                    class: "mt-6 bg-gray-800 rounded-lg p-6",
                    h2 { class: "text-xl font-semibold mb-4 text-gray-200", "Federated Search" }
                    div {
                        class: "flex gap-2",
                        input {
                            class: "flex-1 p-2 rounded bg-gray-700 text-white border border-gray-600 focus:border-purple-500 outline-none",
                            placeholder: "Search your library...",
                            value: "{search_query}",
                            oninput: move |e| search_query.set(e.value()),
                            onkeypress: move |e| {
                                if e.key() == Key::Enter {
                                    // Trigger search on Enter
                                    let query = search_query.read().clone();
                                    if !query.is_empty() {
                                        is_searching.set(true);
                                        ingestion_status.set(format!("Searching..."));
                                    }
                                }
                            }
                        }
                        button {
                            onclick: handle_search,
                            disabled: *is_searching.read(),
                            class: if *is_searching.read() {
                                "px-4 py-2 bg-gray-600 rounded cursor-not-allowed"
                            } else {
                                "px-4 py-2 bg-purple-600 rounded hover:bg-purple-500 transition-colors"
                            },
                            if *is_searching.read() { "Searching..." } else { "Search" }
                        }
                    }
                    p {
                        class: "text-xs text-gray-500 mt-2",
                        "Federated search across all indexes with typo tolerance and semantic matching."
                    }

                    // Search Results
                    if !search_results.read().is_empty() {
                        div {
                            class: "mt-4 space-y-2",
                            for result in search_results.read().iter() {
                                div {
                                    key: "{result.title}",
                                    class: "p-3 bg-gray-700 rounded",
                                    div {
                                        class: "flex justify-between items-start",
                                        h3 { class: "font-medium text-purple-300", "{result.title}" }
                                        span { class: "text-xs text-gray-500", "Score: {result.score:.2}" }
                                    }
                                    p { class: "text-sm text-gray-300 mt-1", "{result.snippet}" }
                                    span { class: "text-xs text-gray-500", "{result.source}" }
                                }
                            }
                        }
                    }
                }

                // Supported Formats
                div {
                    class: "mt-6 bg-gray-800 rounded-lg p-6",
                    h2 { class: "text-xl font-semibold mb-4 text-gray-200", "Supported Formats" }
                    div {
                        class: "flex flex-wrap",
                        span { class: "px-3 py-1 mr-2 mb-2 bg-green-900 text-green-300 rounded text-sm", "PDF" }
                        span { class: "px-3 py-1 mr-2 mb-2 bg-green-900 text-green-300 rounded text-sm", "EPUB" }
                        span { class: "px-3 py-1 mr-2 mb-2 bg-green-900 text-green-300 rounded text-sm", "MOBI/AZW" }
                        span { class: "px-3 py-1 mr-2 mb-2 bg-green-900 text-green-300 rounded text-sm", "DOCX" }
                        span { class: "px-3 py-1 mr-2 mb-2 bg-green-900 text-green-300 rounded text-sm", "Markdown" }
                        span { class: "px-3 py-1 mr-2 mb-2 bg-green-900 text-green-300 rounded text-sm", "TXT" }
                    }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
struct SearchResult {
    title: String,
    snippet: String,
    source: String,
    score: f32,
}
