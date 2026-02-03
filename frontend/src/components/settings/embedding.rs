//! Embedding provider settings component.
//!
//! This module provides UI for configuring embedding providers
//! and semantic analysis options.

use crate::components::design_system::{Badge, BadgeVariant, Card};
use leptos::prelude::*;

/// Embedding provider options
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EmbeddingProvider {
    /// Ollama - local embeddings
    #[default]
    Ollama,
    /// Voyage AI - cloud-based embeddings (placeholder)
    VoyageAI,
}

impl std::fmt::Display for EmbeddingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbeddingProvider::Ollama => write!(f, "Ollama"),
            EmbeddingProvider::VoyageAI => write!(f, "Voyage AI"),
        }
    }
}

/// Semantic analysis provider options
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SemanticAnalysisProvider {
    /// Disabled - no semantic analysis
    #[default]
    Disabled,
    /// Voyage AI - cloud-based semantic analysis (placeholder)
    VoyageAI,
}

impl std::fmt::Display for SemanticAnalysisProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SemanticAnalysisProvider::Disabled => write!(f, "Disabled"),
            SemanticAnalysisProvider::VoyageAI => write!(f, "Voyage AI"),
        }
    }
}

/// Settings view for embedding and semantic analysis providers.
#[component]
pub fn EmbeddingSettingsView() -> impl IntoView {
    // State
    let embedding_provider = RwSignal::new(EmbeddingProvider::Ollama);
    let _semantic_provider = RwSignal::new(SemanticAnalysisProvider::Disabled);

    view! {
        <div class="space-y-8 animate-fade-in pb-20">
            <div class="space-y-2">
                <h3 class="text-xl font-bold text-theme-primary">"Embeddings & Semantic Analysis"</h3>
                <p class="text-theme-muted">"Configure vector embeddings and semantic understanding."</p>
            </div>

            // Embedding Provider Selection
            <Card class="p-6 space-y-6">
                <h4 class="font-semibold text-theme-secondary">"Embedding Provider"</h4>
                <p class="text-sm text-theme-muted">
                    "Embeddings convert text into vectors for semantic search."
                </p>

                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    // Ollama Option
                    <button
                        class=move || format!(
                            "relative p-4 rounded-xl border-2 text-left transition-all duration-300 hover:scale-[1.02] group {}",
                            if embedding_provider.get() == EmbeddingProvider::Ollama {
                                "border-theme-accent bg-theme-elevated ring-2 ring-[var(--accent-primary)]/20 shadow-lg"
                            } else {
                                "border-theme-subtle hover:border-theme-strong bg-theme-surface hover:bg-theme-elevated"
                            }
                        )
                        on:click=move |_| embedding_provider.set(EmbeddingProvider::Ollama)
                    >
                        <div class="flex items-center justify-between mb-2">
                            <span class="font-medium text-theme-primary group-hover:text-theme-accent transition-colors">
                                "Ollama"
                            </span>
                            <Badge variant=BadgeVariant::Info>"Default"</Badge>
                        </div>
                        <p class="text-sm text-theme-muted">
                            "Local embeddings using Ollama. Fast, private, no API costs."
                        </p>
                        <div class="mt-3 flex flex-wrap gap-2">
                            <span class="text-xs px-2 py-1 bg-green-500/20 text-green-400 rounded-full">"nomic-embed-text"</span>
                            <span class="text-xs px-2 py-1 bg-green-500/20 text-green-400 rounded-full">"mxbai-embed-large"</span>
                        </div>

                        // Active indicator
                        {move || if embedding_provider.get() == EmbeddingProvider::Ollama {
                            view! {
                                <div class="absolute top-3 right-3 text-theme-accent animate-fade-in">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                        <path d="M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z"/>
                                        <path d="m9 12 2 2 4-4"/>
                                    </svg>
                                </div>
                            }.into_any()
                        } else {
                            view! { <span/> }.into_any()
                        }}
                    </button>

                    // Voyage AI Option (placeholder)
                    <div
                        class="relative p-4 rounded-xl border-2 text-left opacity-50 cursor-not-allowed border-theme-subtle bg-theme-surface"
                        title="Coming soon"
                    >
                        <div class="flex items-center justify-between mb-2">
                            <span class="font-medium text-theme-primary">
                                "Voyage AI"
                            </span>
                            <Badge variant=BadgeVariant::Default>"Coming Soon"</Badge>
                        </div>
                        <p class="text-sm text-theme-muted">
                            "Cloud-based embeddings optimized for retrieval."
                        </p>
                        <div class="mt-3 flex flex-wrap gap-2">
                            <span class="text-xs px-2 py-1 bg-violet-500/20 text-violet-400 rounded-full">"voyage-3"</span>
                            <span class="text-xs px-2 py-1 bg-violet-500/20 text-violet-400 rounded-full">"voyage-3-lite"</span>
                            <span class="text-xs px-2 py-1 bg-gray-500/20 text-gray-400 rounded-full">"API Costs"</span>
                        </div>
                    </div>
                </div>
            </Card>

            // Semantic Analysis Section (placeholder/disabled)
            <Card class="p-6 space-y-6 opacity-60">
                <div class="flex items-center justify-between">
                    <h4 class="font-semibold text-theme-secondary">"Semantic Analysis"</h4>
                    <Badge variant=BadgeVariant::Default>"Coming Soon"</Badge>
                </div>
                <p class="text-sm text-theme-muted">
                    "Advanced semantic analysis for document classification, entity extraction, and TTRPG content tagging."
                </p>

                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    // Disabled Option
                    <div
                        class="relative p-4 rounded-xl border-2 text-left border-theme-accent/50 bg-theme-elevated"
                    >
                        <div class="flex items-center justify-between mb-2">
                            <span class="font-medium text-theme-primary">
                                "Disabled"
                            </span>
                            <Badge variant=BadgeVariant::Info>"Current"</Badge>
                        </div>
                        <p class="text-sm text-theme-muted">
                            "Semantic analysis is currently disabled."
                        </p>
                    </div>

                    // Voyage AI Option (grayed out)
                    <div
                        class="relative p-4 rounded-xl border-2 text-left opacity-40 cursor-not-allowed border-theme-subtle bg-theme-surface"
                        title="Coming soon"
                    >
                        <div class="flex items-center justify-between mb-2">
                            <span class="font-medium text-theme-primary">
                                "Voyage AI"
                            </span>
                            <Badge variant=BadgeVariant::Default>"Planned"</Badge>
                        </div>
                        <p class="text-sm text-theme-muted">
                            "Document reranking and semantic classification."
                        </p>
                        <div class="mt-3 flex flex-wrap gap-2">
                            <span class="text-xs px-2 py-1 bg-violet-500/20 text-violet-400 rounded-full">"rerank-2"</span>
                            <span class="text-xs px-2 py-1 bg-gray-500/20 text-gray-400 rounded-full">"API Costs"</span>
                        </div>
                    </div>
                </div>
            </Card>

            // Info card about embedding configuration
            <Card class="p-6">
                <div class="flex items-start gap-4">
                    <div class="p-2 rounded-lg bg-blue-500/20">
                        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-blue-400">
                            <circle cx="12" cy="12" r="10"/>
                            <path d="M12 16v-4"/>
                            <path d="M12 8h.01"/>
                        </svg>
                    </div>
                    <div>
                        <h4 class="font-semibold text-theme-secondary">"Ollama Setup"</h4>
                        <p class="text-sm text-theme-muted">
                            "Make sure Ollama is running and you have an embedding model pulled. "
                            "Run "
                            <code class="px-1 py-0.5 bg-theme-deep rounded text-theme-accent">"ollama pull nomic-embed-text"</code>
                            " to get started."
                        </p>
                    </div>
                </div>
            </Card>
        </div>
    }
}
