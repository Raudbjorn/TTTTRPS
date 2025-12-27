#![allow(non_snake_case)]
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct SourceDocument {
    pub name: String,
    pub status: DocumentStatus,
    pub chunk_count: usize,
}

#[derive(Clone, PartialEq)]
pub enum DocumentStatus {
    Indexed,
    Processing,
    Error(String),
    Pending,
}

impl std::fmt::Display for DocumentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentStatus::Indexed => write!(f, "Indexed"),
            DocumentStatus::Processing => write!(f, "Processing"),
            DocumentStatus::Error(msg) => write!(f, "Error: {}", msg),
            DocumentStatus::Pending => write!(f, "Pending"),
        }
    }
}

#[component]
pub fn Library() -> Element {
    let mut ingestion_status = use_signal(|| String::new());
    let mut documents = use_signal(|| vec![
        SourceDocument { name: "Core Rulebook.pdf".to_string(), status: DocumentStatus::Indexed, chunk_count: 842 },
        SourceDocument { name: "Campaign Setting.pdf".to_string(), status: DocumentStatus::Processing, chunk_count: 0 },
    ]);
    let mut total_chunks = use_signal(|| 1240_usize);
    let mut vector_store_status = use_signal(|| "LanceDB Connected".to_string());

    let handle_ingest = move |_: MouseEvent| {
        spawn(async move {
            ingestion_status.set("Opening file picker...".to_string());
            // TODO: Call Tauri command to open file dialog and ingest
            // let result = invoke("ingest_document", path).await;
            ingestion_status.set("File picker opened. (Placeholder - Tauri integration needed)".to_string());
        });
    };

    let refresh_status = move |_: MouseEvent| {
        spawn(async move {
            ingestion_status.set("Refreshing...".to_string());
            // TODO: Call Tauri command to get current status
            // let status = invoke("get_library_status").await;
            ingestion_status.set("Status refreshed.".to_string());
        });
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
                            class: "px-4 py-2 bg-blue-600 rounded hover:bg-blue-500 transition-colors",
                            "Ingest Document"
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
                                    class: "p-3 bg-gray-700 rounded flex justify-between items-center",
                                    div {
                                        span { class: "font-medium", "{doc.name}" }
                                        if doc.chunk_count > 0 {
                                            span { class: "text-xs text-gray-400 ml-2", "({doc.chunk_count} chunks)" }
                                        }
                                    }
                                    span {
                                        class: match doc.status {
                                            DocumentStatus::Indexed => "text-xs px-2 py-1 bg-green-900 text-green-300 rounded",
                                            DocumentStatus::Processing => "text-xs px-2 py-1 bg-yellow-900 text-yellow-300 rounded",
                                            DocumentStatus::Error(_) => "text-xs px-2 py-1 bg-red-900 text-red-300 rounded",
                                            DocumentStatus::Pending => "text-xs px-2 py-1 bg-gray-600 text-gray-300 rounded",
                                        },
                                        "{doc.status}"
                                    }
                                }
                            }
                            if documents.read().is_empty() {
                                div {
                                    class: "text-center text-gray-500 py-8",
                                    "No documents ingested yet. Click 'Ingest Document' to add your first rulebook."
                                }
                            }
                        }
                    }

                    // Stats / Status
                    div {
                        class: "bg-gray-800 rounded-lg p-6",
                        h2 { class: "text-xl font-semibold mb-4 text-gray-200", "System Status" }
                        div {
                            class: "space-y-2",
                            p {
                                class: "text-gray-400",
                                "Vector Store: "
                                span { class: "text-green-400 font-mono", "{vector_store_status}" }
                            }
                            p {
                                class: "text-gray-400",
                                "Total Chunks: "
                                span { class: "text-white font-mono", "{total_chunks}" }
                            }
                            p {
                                class: "text-gray-400",
                                "Documents: "
                                span { class: "text-white font-mono", "{documents.read().len()}" }
                            }
                        }
                        div {
                            class: "mt-4 pt-4 border-t border-gray-700",
                            if !ingestion_status.read().is_empty() {
                                p { class: "text-sm text-blue-400", "{ingestion_status}" }
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
                            class: "flex-1 p-2 rounded bg-gray-700 text-white border border-gray-600 focus:border-blue-500 outline-none",
                            placeholder: "Search your library..."
                        }
                        button {
                            class: "px-4 py-2 bg-purple-600 rounded hover:bg-purple-500 transition-colors",
                            "Search"
                        }
                    }
                }
            }
        }
    }
}
