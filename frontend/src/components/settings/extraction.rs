//! Extraction provider settings component.
//!
//! This module provides UI for configuring text extraction providers,
//! including Kreuzberg (default) and Claude.
//!
//! Note: Claude is the new name for Claude Gate.

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use super::ClaudeAuth;
use crate::bindings::{
    claude_get_status, get_extraction_settings, save_extraction_settings, ClaudeStatus,
    TextExtractionProvider,
};
use crate::components::design_system::{Badge, BadgeVariant, Card};
use crate::services::notification_service::show_error;

/// Settings view for text extraction providers.
#[component]
pub fn ExtractionSettingsView() -> impl IntoView {
    // State
    let selected_provider = RwSignal::new(TextExtractionProvider::Kreuzberg);
    let claude_status = RwSignal::new(ClaudeStatus::default());

    // Save provider selection to backend
    let save_provider = move |provider: TextExtractionProvider| {
        spawn_local(async move {
            // Get current settings, update provider, and save
            match get_extraction_settings().await {
                Ok(mut settings) => {
                    settings.text_extraction_provider = provider;
                    if let Err(e) = save_extraction_settings(settings).await {
                        show_error("Save Failed", Some(&e), None);
                    }
                }
                Err(e) => show_error("Load Settings Failed", Some(&e), None),
            }
        });
    };

    // Initial load - get extraction settings and Claude status
    Effect::new(move |_| {
        spawn_local(async move {
            // Load extraction settings
            if let Ok(settings) = get_extraction_settings().await {
                selected_provider.set(settings.text_extraction_provider);
            }
            // Load Claude status for the badge
            if let Ok(status) = claude_get_status().await {
                claude_status.set(status);
            }
        });
    });

    view! {
        <div class="space-y-8 animate-fade-in pb-20">
            <div class="space-y-2">
                <h3 class="text-xl font-bold text-theme-primary">"Text Extraction"</h3>
                <p class="text-theme-muted">"Configure how documents are extracted and processed."</p>
            </div>

            // Provider Selection
            <Card class="p-6 space-y-6">
                <h4 class="font-semibold text-theme-secondary">"Extraction Provider"</h4>

                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    // Kreuzberg Option
                    <button
                        class=move || format!(
                            "relative p-4 rounded-xl border-2 text-left transition-all duration-300 hover:scale-[1.02] group {}",
                            if selected_provider.get() == TextExtractionProvider::Kreuzberg {
                                "border-theme-accent bg-theme-elevated ring-2 ring-[var(--accent-primary)]/20 shadow-lg"
                            } else {
                                "border-theme-subtle hover:border-theme-strong bg-theme-surface hover:bg-theme-elevated"
                            }
                        )
                        on:click=move |_| {
                            selected_provider.set(TextExtractionProvider::Kreuzberg);
                            save_provider(TextExtractionProvider::Kreuzberg);
                        }
                    >
                        <div class="flex items-center justify-between mb-2">
                            <span class="font-medium text-theme-primary group-hover:text-theme-accent transition-colors">
                                "Kreuzberg"
                            </span>
                            <Badge variant=BadgeVariant::Info>"Default"</Badge>
                        </div>
                        <p class="text-sm text-theme-muted">
                            "Local extraction using bundled pdfium. Fast, private, no API costs."
                        </p>
                        <div class="mt-3 flex flex-wrap gap-2">
                            <span class="text-xs px-2 py-1 bg-green-500/20 text-green-400 rounded-full">"PDF"</span>
                            <span class="text-xs px-2 py-1 bg-green-500/20 text-green-400 rounded-full">"EPUB"</span>
                            <span class="text-xs px-2 py-1 bg-green-500/20 text-green-400 rounded-full">"DOCX"</span>
                            <span class="text-xs px-2 py-1 bg-green-500/20 text-green-400 rounded-full">"Images"</span>
                        </div>

                        // Active indicator
                        {move || if selected_provider.get() == TextExtractionProvider::Kreuzberg {
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

                    // Claude Option
                    <button
                        class=move || format!(
                            "relative p-4 rounded-xl border-2 text-left transition-all duration-300 hover:scale-[1.02] group {}",
                            if selected_provider.get() == TextExtractionProvider::Claude {
                                "border-orange-400 bg-theme-elevated ring-2 ring-orange-400/20 shadow-lg"
                            } else {
                                "border-theme-subtle hover:border-theme-strong bg-theme-surface hover:bg-theme-elevated"
                            }
                        )
                        on:click=move |_| {
                            selected_provider.set(TextExtractionProvider::Claude);
                            save_provider(TextExtractionProvider::Claude);
                        }
                    >
                        <div class="flex items-center justify-between mb-2">
                            <span class="font-medium text-theme-primary group-hover:text-orange-400 transition-colors">
                                "Claude API"
                            </span>
                            {move || if claude_status.get().authenticated {
                                view! { <Badge variant=BadgeVariant::Success>"Authenticated"</Badge> }.into_any()
                            } else {
                                view! { <Badge variant=BadgeVariant::Warning>"Not Authenticated"</Badge> }.into_any()
                            }}
                        </div>
                        <p class="text-sm text-theme-muted">
                            "Uses Claude API for intelligent extraction. Better for complex layouts."
                        </p>
                        <div class="mt-3 flex flex-wrap gap-2">
                            <span class="text-xs px-2 py-1 bg-orange-500/20 text-orange-400 rounded-full">"PDF"</span>
                            <span class="text-xs px-2 py-1 bg-orange-500/20 text-orange-400 rounded-full">"Images"</span>
                            <span class="text-xs px-2 py-1 bg-gray-500/20 text-gray-400 rounded-full">"API Costs"</span>
                        </div>

                        // Active indicator
                        {move || if selected_provider.get() == TextExtractionProvider::Claude {
                            view! {
                                <div class="absolute top-3 right-3 text-orange-400 animate-fade-in">
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
                </div>
            </Card>

            // Claude Authentication Section (shown when Claude is selected)
            {move || if selected_provider.get() == TextExtractionProvider::Claude {
                view! {
                    <ClaudeAuth
                        on_status_change=Callback::new(move |status: ClaudeStatus| {
                            claude_status.set(status);
                        })
                    />
                }.into_any()
            } else {
                view! { <span/> }.into_any()
            }}

            // Additional extraction settings note
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
                        <h4 class="font-semibold text-theme-secondary">"OCR Settings"</h4>
                        <p class="text-sm text-theme-muted">
                            "For scanned documents and images, OCR settings can be configured in the Data & Library section. "
                            "Both providers support OCR fallback for documents without extractable text."
                        </p>
                    </div>
                </div>
            </Card>
        </div>
    }
}
