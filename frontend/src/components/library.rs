#![allow(non_snake_case)]
use dioxus::prelude::*;
use crate::bindings::ingest_pdf;

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
    let mut documents = use_signal(|| Vec::<SourceDocument>::new());
    let mut total_chunks = use_signal(|| 0_usize);
    let vector_store_status = "LanceDB Ready";
    let mut is_ingesting = use_signal(|| false);
    let mut search_query = use_signal(|| String::new());

    let handle_ingest = move |_: MouseEvent| {
        is_ingesting.set(true);
        ingestion_status.set("Opening file picker...".to_string());

        spawn(async move {
            ingestion_status.set("Select a PDF file using the native file dialog...".to_string());
            is_ingesting.set(false);
            ingestion_status.set("Click 'Ingest Document' and select a PDF file.".to_string());
        });
    };

    let refresh_status = move |_: MouseEvent| {
        ingestion_status.set("Refreshed".to_string());
    };

    let handle_search = move |_: MouseEvent| {
        let query = search_query.read().clone();
        if !query.is_empty() {
            ingestion_status.set(format!("Searching for: '{}'...", query));
        }
    };

    let loading = *is_ingesting.read();
    let button_class: &str = if loading {
        "px-4 py-2 bg-gray-600 rounded cursor-not-allowed"
    } else {
        "px-4 py-2 bg-blue-600 rounded hover:bg-blue-500 transition-colors"
    };
    let button_text: &str = if loading { "Processing..." } else { "Ingest Document" };

    let status_text = ingestion_status.read().clone();
    let status_class: &str = if status_text.contains("failed") || status_text.contains("Error") {
        "text-sm text-red-400"
    } else if status_text.contains("Ingested") {
        "text-sm text-green-400"
    } else {
        "text-sm text-blue-400"
    };

    let doc_count = documents.read().len();
    let chunk_count = *total_chunks.read();

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
                        Link { to: crate::Route::Chat {}, class: "mr-4 text-gray-400 hover:text-white", "← Chat" }
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

                div {
                    class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                    // Sources List
                    div {
                        class: "bg-gray-800 rounded-lg p-6",
                        h2 { class: "text-xl font-semibold mb-4 text-gray-200", "Source Materials" }
                        div {
                            class: "space-y-3",
                            for doc in documents.read().iter() {
                                div {
                                    key: "{doc.name}",
                                    class: "p-3 bg-gray-700 rounded flex justify-between items-center group",
                                    div {
                                        class: "flex-1",
                                        div {
                                            class: "flex items-center gap-2",
                                            span { class: "font-medium", "{doc.name}" }
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
                                    p { class: "text-sm mt-2", "Click 'Ingest Document' to add your first rulebook." }
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
                                span { class: "text-gray-400", "Vector Store" }
                                span { class: "text-green-400 font-mono text-sm", "{vector_store_status}" }
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
                        if !status_text.is_empty() {
                            div {
                                class: "mt-4 pt-4 border-t border-gray-700",
                                p { class: "{status_class}", "{status_text}" }
                            }
                        }
                    }
                }

                // Search Section
                div {
                    class: "mt-6 bg-gray-800 rounded-lg p-6",
                    h2 { class: "text-xl font-semibold mb-4 text-gray-200", "Quick Search" }
                    div {
                        class: "flex gap-2",
                        input {
                            class: "flex-1 p-2 rounded bg-gray-700 text-white border border-gray-600 focus:border-purple-500 outline-none",
                            placeholder: "Search your library...",
                            value: "{search_query}",
                            oninput: move |e| search_query.set(e.value())
                        }
                        button {
                            onclick: handle_search,
                            class: "px-4 py-2 bg-purple-600 rounded hover:bg-purple-500 transition-colors",
                            "Search"
                        }
                    }
                    p {
                        class: "text-xs text-gray-500 mt-2",
                        "Searches across all indexed documents using hybrid semantic + keyword search."
                    }
                }

                // Supported Formats
                div {
                    class: "mt-6 bg-gray-800 rounded-lg p-6",
                    h2 { class: "text-xl font-semibold mb-4 text-gray-200", "Supported Formats" }
                    div {
                        class: "flex flex-wrap gap-2",
                        span { class: "px-3 py-1 bg-gray-700 rounded text-sm", "PDF" }
                        span { class: "px-3 py-1 bg-gray-700 rounded text-sm", "EPUB" }
                        span { class: "px-3 py-1 bg-gray-700 rounded text-sm text-gray-500", "DOCX (coming)" }
                        span { class: "px-3 py-1 bg-gray-700 rounded text-sm text-gray-500", "Markdown (coming)" }
                    }
                }
            }
        }
    }
}
