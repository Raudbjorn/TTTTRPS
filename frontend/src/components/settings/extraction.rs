//! Extraction provider settings component.
//!
//! This module provides UI for configuring text extraction providers,
//! including Kreuzberg (default) and Claude Gate.

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::bindings::{
    claude_gate_get_status, claude_gate_start_oauth, claude_gate_complete_oauth,
    claude_gate_logout, claude_gate_set_storage_backend,
    ClaudeGateStatus, ClaudeGateStorageBackend,
    get_extraction_settings, save_extraction_settings, ExtractionSettings, TextExtractionProvider,
    open_url_in_browser,
};
use crate::components::design_system::{Card, Badge, BadgeVariant, Select, SelectOption};
use crate::services::notification_service::{show_error, show_success};

/// Settings view for text extraction providers.
#[component]
pub fn ExtractionSettingsView() -> impl IntoView {
    // State
    let selected_provider = RwSignal::new(TextExtractionProvider::Kreuzberg);
    let extraction_settings = RwSignal::new(ExtractionSettings::default());
    let claude_gate_status = RwSignal::new(ClaudeGateStatus::default());
    let is_loading = RwSignal::new(false);
    let auth_code = RwSignal::new(String::new());
    let awaiting_code = RwSignal::new(false);
    let oauth_url = RwSignal::new(Option::<String>::None);
    let oauth_csrf_state = RwSignal::new(Option::<String>::None);
    let selected_storage_backend = RwSignal::new(String::from("auto"));

    // Refresh Claude Gate status
    let refresh_status = move || {
        is_loading.set(true);
        spawn_local(async move {
            match claude_gate_get_status().await {
                Ok(status) => claude_gate_status.set(status),
                Err(e) => show_error("Claude Gate Status", Some(&e), None),
            }
            is_loading.set(false);
        });
    };

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

    // Initial load - get extraction settings and Claude Gate status
    Effect::new(move |_| {
        spawn_local(async move {
            // Load extraction settings
            if let Ok(settings) = get_extraction_settings().await {
                selected_provider.set(settings.text_extraction_provider);
                extraction_settings.set(settings);
            }
        });
        refresh_status();
    });

    view! {
        <div class="space-y-8 animate-fade-in pb-20">
            <div class="space-y-2">
                <h3 class="text-xl font-bold text-[var(--text-primary)]">"Text Extraction"</h3>
                <p class="text-[var(--text-muted)]">"Configure how documents are extracted and processed."</p>
            </div>

            // Provider Selection
            <Card class="p-6 space-y-6">
                <h4 class="font-semibold text-[var(--text-secondary)]">"Extraction Provider"</h4>

                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    // Kreuzberg Option
                    <button
                        class=move || format!(
                            "relative p-4 rounded-xl border-2 text-left transition-all duration-300 hover:scale-[1.02] group {}",
                            if selected_provider.get() == TextExtractionProvider::Kreuzberg {
                                "border-[var(--accent-primary)] bg-[var(--bg-elevated)] ring-2 ring-[var(--accent-primary)]/20 shadow-lg"
                            } else {
                                "border-[var(--border-subtle)] hover:border-[var(--border-strong)] bg-[var(--bg-surface)] hover:bg-[var(--bg-elevated)]"
                            }
                        )
                        on:click=move |_| {
                            selected_provider.set(TextExtractionProvider::Kreuzberg);
                            save_provider(TextExtractionProvider::Kreuzberg);
                        }
                    >
                        <div class="flex items-center justify-between mb-2">
                            <span class="font-medium text-[var(--text-primary)] group-hover:text-[var(--accent-primary)] transition-colors">
                                "Kreuzberg"
                            </span>
                            <Badge variant=BadgeVariant::Info>"Default"</Badge>
                        </div>
                        <p class="text-sm text-[var(--text-muted)]">
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
                                <div class="absolute top-3 right-3 text-[var(--accent-primary)] animate-fade-in">
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

                    // Claude Gate Option
                    <button
                        class=move || format!(
                            "relative p-4 rounded-xl border-2 text-left transition-all duration-300 hover:scale-[1.02] group {}",
                            if selected_provider.get() == TextExtractionProvider::ClaudeGate {
                                "border-orange-400 bg-[var(--bg-elevated)] ring-2 ring-orange-400/20 shadow-lg"
                            } else {
                                "border-[var(--border-subtle)] hover:border-[var(--border-strong)] bg-[var(--bg-surface)] hover:bg-[var(--bg-elevated)]"
                            }
                        )
                        on:click=move |_| {
                            selected_provider.set(TextExtractionProvider::ClaudeGate);
                            save_provider(TextExtractionProvider::ClaudeGate);
                        }
                    >
                        <div class="flex items-center justify-between mb-2">
                            <span class="font-medium text-[var(--text-primary)] group-hover:text-orange-400 transition-colors">
                                "Claude Gate"
                            </span>
                            {move || if claude_gate_status.get().authenticated {
                                view! { <Badge variant=BadgeVariant::Success>"Authenticated"</Badge> }.into_any()
                            } else {
                                view! { <Badge variant=BadgeVariant::Warning>"Not Authenticated"</Badge> }.into_any()
                            }}
                        </div>
                        <p class="text-sm text-[var(--text-muted)]">
                            "Uses Claude API for intelligent extraction. Better for complex layouts."
                        </p>
                        <div class="mt-3 flex flex-wrap gap-2">
                            <span class="text-xs px-2 py-1 bg-orange-500/20 text-orange-400 rounded-full">"PDF"</span>
                            <span class="text-xs px-2 py-1 bg-orange-500/20 text-orange-400 rounded-full">"Images"</span>
                            <span class="text-xs px-2 py-1 bg-gray-500/20 text-gray-400 rounded-full">"API Costs"</span>
                        </div>

                        // Active indicator
                        {move || if selected_provider.get() == TextExtractionProvider::ClaudeGate {
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

            // Claude Gate Authentication Section (shown when Claude Gate is selected)
            {move || if selected_provider.get() == TextExtractionProvider::ClaudeGate {
                let status = claude_gate_status.get();
                view! {
                    <Card class="p-6 space-y-4 border-orange-400/30">
                        <div class="flex items-center justify-between">
                            <h4 class="font-semibold text-orange-400">"Claude Gate Authentication"</h4>
                            {if status.authenticated {
                                view! {
                                    <Badge variant=BadgeVariant::Success>
                                        {status.expiration_display.clone().unwrap_or_else(|| "Authenticated".to_string())}
                                    </Badge>
                                }.into_any()
                            } else {
                                view! {
                                    <Badge variant=BadgeVariant::Warning>"Login Required"</Badge>
                                }.into_any()
                            }}
                        </div>

                        <p class="text-sm text-[var(--text-muted)]">
                            "Claude Gate requires OAuth authentication with your Anthropic account."
                        </p>

                        // Storage backend selector
                        <div class="space-y-2">
                            <label class="text-xs text-[var(--text-muted)]">"Token Storage Backend"</label>
                            <div class="flex flex-col gap-2">
                                <Select
                                    value=Signal::derive(move || claude_gate_status.get().storage_backend)
                                    on_change=Callback::new(move |value: String| {
                                        selected_storage_backend.set(value.clone());
                                        spawn_local(async move {
                                            is_loading.set(true);
                                            let backend = match value.as_str() {
                                                "keyring" => ClaudeGateStorageBackend::Keyring,
                                                "file" => ClaudeGateStorageBackend::File,
                                                _ => ClaudeGateStorageBackend::Auto,
                                            };
                                            match claude_gate_set_storage_backend(backend).await {
                                                Ok(_) => {
                                                    show_success("Storage Changed", Some("You may need to re-authenticate"));
                                                    refresh_status();
                                                }
                                                Err(e) => show_error("Storage Change Failed", Some(&e), None),
                                            }
                                            is_loading.set(false);
                                        });
                                    })
                                    class="w-auto"
                                >
                                    {move || {
                                        let keyring_available = claude_gate_status.get().keyring_available;
                                        view! {
                                            <SelectOption
                                                value="auto"
                                                label=if keyring_available { "Auto (prefer keyring)" } else { "Auto (file only)" }
                                            />
                                            <SelectOption
                                                value="keyring"
                                                label=if keyring_available { "Keyring (secure)" } else { "Keyring (unavailable)" }
                                            />
                                            <SelectOption value="file" label="File" />
                                        }
                                    }}
                                </Select>
                                {move || if !claude_gate_status.get().keyring_available {
                                    view! {
                                        <p class="text-xs text-yellow-400/80">
                                            "Secret service not available. Install gnome-keyring or similar."
                                        </p>
                                    }.into_any()
                                } else {
                                    view! { <span /> }.into_any()
                                }}
                            </div>
                        </div>

                        // Status info
                        <div class="flex flex-wrap gap-2">
                            {status.error.map(|e| view! {
                                <div class="flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium bg-red-500/20 text-red-400">
                                    {format!("Error: {}", e)}
                                </div>
                            })}
                        </div>

                        // Action buttons and auth code input
                        <div class="flex flex-col gap-3 pt-2">
                            // Auth code input (shown when awaiting code)
                            {move || {
                                if awaiting_code.get() {
                                    view! {
                                        <div class="flex flex-col gap-2 p-3 rounded-lg bg-[var(--bg-elevated)] border border-[var(--border-subtle)]">
                                            // Show OAuth URL if available (for manual copy when popup blocked)
                                            {move || {
                                                if let Some(url) = oauth_url.get() {
                                                    view! {
                                                        <div class="flex flex-col gap-1">
                                                            <p class="text-xs text-[var(--text-secondary)]">
                                                                "If the browser didn't open, copy this URL:"
                                                            </p>
                                                            <div class="flex gap-2 items-center">
                                                                <input
                                                                    type="text"
                                                                    readonly=true
                                                                    class="flex-1 px-2 py-1 text-xs rounded bg-[var(--bg-deep)] border border-[var(--border-subtle)] text-[var(--text-muted)] font-mono truncate"
                                                                    prop:value=url.clone()
                                                                />
                                                                <button
                                                                    class="px-2 py-1 text-xs rounded bg-orange-500/20 text-orange-400 hover:bg-orange-500/30"
                                                                    on:click={
                                                                        let url_copy = url.clone();
                                                                        move |_| {
                                                                            if let Some(window) = web_sys::window() {
                                                                                let clipboard = window.navigator().clipboard();
                                                                                let url_to_copy = url_copy.clone();
                                                                                spawn_local(async move {
                                                                                    let _ = wasm_bindgen_futures::JsFuture::from(
                                                                                        clipboard.write_text(&url_to_copy)
                                                                                    ).await;
                                                                                    show_success("Copied", Some("URL copied to clipboard"));
                                                                                });
                                                                            }
                                                                        }
                                                                    }
                                                                >
                                                                    "Copy"
                                                                </button>
                                                            </div>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! { <div></div> }.into_any()
                                                }
                                            }}
                                            <p class="text-xs text-[var(--text-secondary)]">
                                                "After authorizing in your browser, paste the authorization code here:"
                                            </p>
                                            <div class="flex gap-2">
                                                <input
                                                    type="text"
                                                    placeholder="Paste authorization code..."
                                                    class="flex-1 px-3 py-1.5 text-sm rounded-lg bg-[var(--bg-deep)] border border-[var(--border-subtle)] text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none focus:border-orange-400"
                                                    prop:value=move || auth_code.get()
                                                    on:input=move |ev| {
                                                        auth_code.set(event_target_value(&ev));
                                                    }
                                                />
                                                <button
                                                    class="px-3 py-1.5 text-xs font-medium rounded-lg bg-green-500/20 text-green-400 hover:bg-green-500/30 transition-colors disabled:opacity-50"
                                                    disabled=move || is_loading.get() || auth_code.get().is_empty()
                                                    on:click=move |_| {
                                                        let code = auth_code.get();
                                                        let csrf_state = oauth_csrf_state.get();
                                                        spawn_local(async move {
                                                            is_loading.set(true);
                                                            match claude_gate_complete_oauth(code, csrf_state).await {
                                                                Ok(result) => {
                                                                    if result.success {
                                                                        show_success("Login Complete", Some("Successfully authenticated with Claude"));
                                                                        awaiting_code.set(false);
                                                                        auth_code.set(String::new());
                                                                        oauth_url.set(None);
                                                                        oauth_csrf_state.set(None);
                                                                        refresh_status();
                                                                    } else {
                                                                        show_error("OAuth Failed", result.error.as_deref(), None);
                                                                    }
                                                                }
                                                                Err(e) => show_error("OAuth Failed", Some(&e), None),
                                                            }
                                                            is_loading.set(false);
                                                        });
                                                    }
                                                >
                                                    "Complete Login"
                                                </button>
                                                <button
                                                    class="px-3 py-1.5 text-xs font-medium rounded-lg bg-[var(--bg-surface)] text-[var(--text-muted)] hover:bg-[var(--bg-elevated)] transition-colors"
                                                    on:click=move |_| {
                                                        awaiting_code.set(false);
                                                        auth_code.set(String::new());
                                                        oauth_url.set(None);
                                                        oauth_csrf_state.set(None);
                                                    }
                                                >
                                                    "Cancel"
                                                </button>
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <span /> }.into_any()
                                }
                            }}

                            // Main action buttons
                            <div class="flex gap-3">
                                {move || {
                                    let loading = is_loading.get();
                                    let authenticated = claude_gate_status.get().authenticated;
                                    let is_awaiting = awaiting_code.get();
                                    if !authenticated && !is_awaiting {
                                        view! {
                                            <button
                                                class="px-4 py-2 text-sm font-medium rounded-lg bg-orange-500 text-white hover:bg-orange-600 transition-colors disabled:opacity-50"
                                                disabled=loading
                                                on:click=move |_| {
                                                    spawn_local(async move {
                                                        is_loading.set(true);
                                                        match claude_gate_start_oauth().await {
                                                            Ok(response) => {
                                                                // Store URL for display if browser fails to open
                                                                oauth_url.set(Some(response.auth_url.clone()));
                                                                // Store CSRF state for verification
                                                                oauth_csrf_state.set(Some(response.state));
                                                                // Open URL using Tauri's shell plugin
                                                                match open_url_in_browser(response.auth_url).await {
                                                                    Ok(_) => {
                                                                        show_success("Login Started", Some("Complete authentication in your browser, then paste the code below"));
                                                                        awaiting_code.set(true);
                                                                    }
                                                                    Err(e) => {
                                                                        show_error("Browser Open Failed", Some(&format!("{}. Copy the URL shown below.", e)), None);
                                                                        awaiting_code.set(true);
                                                                    }
                                                                }
                                                            }
                                                            Err(e) => show_error("OAuth Failed", Some(&e), None),
                                                        }
                                                        is_loading.set(false);
                                                    });
                                                }
                                            >
                                                "Login with Claude"
                                            </button>
                                        }.into_any()
                                    } else if authenticated {
                                        view! {
                                            <button
                                                class="px-4 py-2 text-sm font-medium rounded-lg bg-red-500/20 text-red-400 hover:bg-red-500/30 transition-colors disabled:opacity-50"
                                                disabled=loading
                                                on:click=move |_| {
                                                    spawn_local(async move {
                                                        is_loading.set(true);
                                                        match claude_gate_logout().await {
                                                            Ok(_) => {
                                                                show_success("Logged Out", None);
                                                                refresh_status();
                                                            }
                                                            Err(e) => show_error("Logout Failed", Some(&e), None),
                                                        }
                                                        is_loading.set(false);
                                                    });
                                                }
                                            >
                                                "Logout"
                                            </button>
                                        }.into_any()
                                    } else {
                                        view! { <span /> }.into_any()
                                    }
                                }}

                                <button
                                    class="px-4 py-2 text-sm font-medium rounded-lg bg-[var(--bg-elevated)] text-[var(--text-secondary)] hover:bg-[var(--bg-surface)] transition-colors disabled:opacity-50"
                                    disabled=move || is_loading.get()
                                    on:click=move |_| refresh_status()
                                >
                                    {move || if is_loading.get() { "Checking..." } else { "Refresh Status" }}
                                </button>
                            </div>
                        </div>
                    </Card>
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
                        <h4 class="font-semibold text-[var(--text-secondary)]">"OCR Settings"</h4>
                        <p class="text-sm text-[var(--text-muted)]">
                            "For scanned documents and images, OCR settings can be configured in the Data & Library section. "
                            "Both providers support OCR fallback for documents without extractable text."
                        </p>
                    </div>
                </div>
            </Card>
        </div>
    }
}
