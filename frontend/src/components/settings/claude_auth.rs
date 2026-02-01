//! Reusable Claude OAuth authentication component.
//!
//! This component provides a complete OAuth authentication flow UI for Claude,
//! including status display, storage backend selection, login/logout buttons, and
//! auth code input. It can be used in any settings panel that needs Claude auth.

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::bindings::{
    claude_get_status, claude_start_oauth, claude_complete_oauth,
    claude_logout, claude_set_storage_backend, open_url_in_browser,
    ClaudeStatus, ClaudeStorageBackend,
};
use crate::components::design_system::{Badge, BadgeVariant, Select, SelectOption};
use crate::services::notification_service::{show_error, show_success};

/// Reusable Claude OAuth authentication component.
///
/// Provides complete OAuth flow UI including:
/// - Authentication status display
/// - Storage backend selection (Auto/Keyring/File)
/// - Login with OAuth flow
/// - Auth code input for completing OAuth
/// - Logout functionality
/// - Refresh status button
#[component]
pub fn ClaudeAuth(
    /// Optional callback when authentication status changes
    #[prop(optional)]
    on_status_change: Option<Callback<ClaudeStatus>>,
    /// Whether to show the card wrapper (default: true)
    #[prop(default = true)]
    show_card: bool,
    /// Compact mode for inline display
    #[prop(default = false)]
    compact: bool,
) -> impl IntoView {
    // Internal state
    let status = RwSignal::new(ClaudeStatus::default());
    let is_loading = RwSignal::new(false);
    let auth_code = RwSignal::new(String::new());
    let awaiting_code = RwSignal::new(false);
    let oauth_url = RwSignal::new(Option::<String>::None);
    let oauth_csrf_state = RwSignal::new(Option::<String>::None);

    // Refresh status from backend
    let refresh_status = move || {
        is_loading.set(true);
        spawn_local(async move {
            match claude_get_status().await {
                Ok(new_status) => {
                    status.set(new_status.clone());
                    if let Some(callback) = on_status_change {
                        callback.run(new_status);
                    }
                }
                Err(e) => show_error("Claude Status", Some(&e), None),
            }
            is_loading.set(false);
        });
    };

    // Initial load
    Effect::new(move |_| {
        refresh_status();
    });

    // Start OAuth flow
    let start_oauth = move || {
        spawn_local(async move {
            is_loading.set(true);
            match claude_start_oauth().await {
                Ok(response) => {
                    oauth_url.set(Some(response.auth_url.clone()));
                    oauth_csrf_state.set(Some(response.state));
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
    };

    // Complete OAuth with auth code
    let complete_oauth = move || {
        let code = auth_code.get();
        let csrf_state = oauth_csrf_state.get();
        spawn_local(async move {
            is_loading.set(true);
            match claude_complete_oauth(code, csrf_state).await {
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
    };

    // Logout
    let logout = move || {
        spawn_local(async move {
            is_loading.set(true);
            match claude_logout().await {
                Ok(_) => {
                    show_success("Logged Out", None);
                    refresh_status();
                }
                Err(e) => show_error("Logout Failed", Some(&e), None),
            }
            is_loading.set(false);
        });
    };

    // Cancel auth code input
    let cancel_auth = move || {
        awaiting_code.set(false);
        auth_code.set(String::new());
        oauth_url.set(None);
        oauth_csrf_state.set(None);
    };

    // Change storage backend
    let change_storage = move |backend: ClaudeStorageBackend| {
        spawn_local(async move {
            is_loading.set(true);
            match claude_set_storage_backend(backend).await {
                Ok(_) => {
                    show_success("Storage Changed", Some("You may need to re-authenticate"));
                    refresh_status();
                }
                Err(e) => show_error("Storage Change Failed", Some(&e), None),
            }
            is_loading.set(false);
        });
    };

    let content = view! {
        <div class=move || format!("space-y-4 {}", if compact { "text-sm" } else { "" })>
            // Header with status
            <div class="flex items-center justify-between">
                <h4 class="font-semibold text-orange-400">"Claude Authentication"</h4>
                {move || {
                    let s = status.get();
                    if s.authenticated {
                        view! {
                            <Badge variant=BadgeVariant::Success>
                                {s.expiration_display.clone().unwrap_or_else(|| "Authenticated".to_string())}
                            </Badge>
                        }.into_any()
                    } else {
                        view! {
                            <Badge variant=BadgeVariant::Warning>"Login Required"</Badge>
                        }.into_any()
                    }
                }}
            </div>

            <p class="text-sm text-[var(--text-muted)]">
                "Claude integration requires OAuth authentication with your Anthropic account."
            </p>

            // Storage backend selector
            <div class="space-y-2">
                <label class="text-xs text-[var(--text-muted)]">"Token Storage Backend"</label>
                <div class="flex flex-col gap-2">
                    <Select
                        value=Signal::derive(move || status.get().storage_backend)
                        on_change=Callback::new(move |value: String| {
                            let backend = match value.as_str() {
                                "keyring" => ClaudeStorageBackend::Keyring,
                                "file" => ClaudeStorageBackend::File,
                                _ => ClaudeStorageBackend::Auto,
                            };
                            change_storage(backend);
                        })
                        class="w-auto"
                    >
                        {move || {
                            let keyring_available = status.get().keyring_available;
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
                    {move || if !status.get().keyring_available {
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

            // Error message if any
            {move || status.get().error.map(|e| view! {
                <div class="flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium bg-red-500/20 text-red-400">
                    {format!("Error: {}", e)}
                </div>
            })}

            // Auth code input section (shown when awaiting code)
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
                                    on:click=move |_| complete_oauth()
                                >
                                    "Complete Login"
                                </button>
                                <button
                                    class="px-3 py-1.5 text-xs font-medium rounded-lg bg-[var(--bg-surface)] text-[var(--text-muted)] hover:bg-[var(--bg-elevated)] transition-colors"
                                    on:click=move |_| cancel_auth()
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
                    let authenticated = status.get().authenticated;
                    let is_awaiting = awaiting_code.get();
                    if !authenticated && !is_awaiting {
                        view! {
                            <button
                                class="px-4 py-2 text-sm font-medium rounded-lg bg-orange-500 text-white hover:bg-orange-600 transition-colors disabled:opacity-50"
                                disabled=loading
                                on:click=move |_| start_oauth()
                            >
                                "Login with Claude"
                            </button>
                        }.into_any()
                    } else if authenticated {
                        view! {
                            <button
                                class="px-4 py-2 text-sm font-medium rounded-lg bg-red-500/20 text-red-400 hover:bg-red-500/30 transition-colors disabled:opacity-50"
                                disabled=loading
                                on:click=move |_| logout()
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
    };

    if show_card {
        view! {
            <div class="p-6 rounded-xl bg-[var(--bg-surface)] border border-orange-400/30 space-y-4">
                {content}
            </div>
        }.into_any()
    } else {
        view! { <div>{content}</div> }.into_any()
    }
}

/// Compact status indicator for Claude authentication.
/// Shows just the authentication status badge.
#[component]
pub fn ClaudeStatusBadge() -> impl IntoView {
    let status = RwSignal::new(ClaudeStatus::default());

    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(s) = claude_get_status().await {
                status.set(s);
            }
        });
    });

    view! {
        {move || if status.get().authenticated {
            view! { <Badge variant=BadgeVariant::Success>"Authenticated"</Badge> }.into_any()
        } else {
            view! { <Badge variant=BadgeVariant::Warning>"Not Authenticated"</Badge> }.into_any()
        }}
    }
}
