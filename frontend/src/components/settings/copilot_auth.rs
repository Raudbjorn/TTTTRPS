//! Reusable Copilot (GitHub) OAuth authentication component.
//!
//! This component provides a complete Device Code OAuth authentication flow UI for GitHub Copilot,
//! including status display, device code entry, login/logout buttons, and polling.

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;

use crate::bindings::{
    check_copilot_auth, start_copilot_auth, poll_copilot_auth, logout_copilot,
    get_copilot_usage, open_url_in_browser, copilot_set_storage_backend,
    CopilotAuthStatus, CopilotUsageInfo, CopilotStorageBackend,
};
use crate::components::design_system::{Select, SelectOption};
use crate::components::design_system::{Badge, BadgeVariant};
use crate::services::notification_service::{show_error, show_success};

/// Reusable Copilot OAuth authentication component.
///
/// Provides complete Device Code OAuth flow UI including:
/// - Authentication status display
/// - Device code flow with user code display
/// - Login/logout functionality
/// - Refresh status button
/// - Usage/quota display when authenticated
#[component]
pub fn CopilotAuth(
    /// Optional callback when authentication status changes
    #[prop(optional)]
    on_status_change: Option<Callback<CopilotAuthStatus>>,
    /// Whether to show the card wrapper (default: true)
    #[prop(default = true)]
    show_card: bool,
    /// Compact mode for inline display
    #[prop(default = false)]
    compact: bool,
) -> impl IntoView {
    // Internal state
    let status = RwSignal::new(CopilotAuthStatus::default());
    let usage = RwSignal::new(Option::<CopilotUsageInfo>::None);
    let is_loading = RwSignal::new(false);
    let awaiting_auth = RwSignal::new(false);

    // Device code flow state
    let user_code = RwSignal::new(String::new());
    let verification_uri = RwSignal::new(String::new());
    let device_code = RwSignal::new(String::new());
    let poll_interval_secs = RwSignal::new(5u64);

    // Cancellation flag for polling loop cleanup
    let polling_cancelled = RwSignal::new(false);

    // Refresh status from backend
    let refresh_status = move || {
        is_loading.set(true);
        spawn_local(async move {
            match check_copilot_auth().await {
                Ok(new_status) => {
                    status.set(new_status.clone());
                    if let Some(callback) = on_status_change {
                        callback.run(new_status.clone());
                    }
                    // Fetch usage if authenticated
                    if new_status.authenticated {
                        if let Ok(usage_info) = get_copilot_usage().await {
                            usage.set(Some(usage_info));
                        }
                    } else {
                        usage.set(None);
                    }
                }
                Err(e) => show_error("Copilot Status", Some(&e), None),
            }
            is_loading.set(false);
        });
    };

    // Initial load
    Effect::new(move |_| {
        refresh_status();
    });

    // Cleanup: cancel polling when component unmounts
    on_cleanup(move || {
        polling_cancelled.set(true);
        awaiting_auth.set(false);
    });

    // Polling effect - runs when awaiting_auth is true
    // Uses a loop with TimeoutFuture for cleaner async control flow
    Effect::new(move |_| {
        if !awaiting_auth.get() {
            return;
        }

        let code = device_code.get();
        if code.is_empty() {
            return;
        }

        // Reset cancellation flag when starting new polling
        polling_cancelled.set(false);

        spawn_local(async move {
            const MAX_CONSECUTIVE_ERRORS: u32 = 3;
            let mut consecutive_errors: u32 = 0;

            while awaiting_auth.get_untracked() && !polling_cancelled.get_untracked() {
                let interval_ms = (poll_interval_secs.get_untracked() * 1000).max(5000) as u32;
                TimeoutFuture::new(interval_ms).await;

                // Check cancellation after timeout
                if polling_cancelled.get_untracked() || !awaiting_auth.get_untracked() {
                    break;
                }

                let code = device_code.get_untracked();
                if code.is_empty() {
                    break;
                }

                match poll_copilot_auth(code).await {
                    Ok(result) => {
                        // Check cancellation before updating signals
                        if polling_cancelled.get_untracked() {
                            break;
                        }
                        consecutive_errors = 0; // Reset on success
                        match result.status.as_str() {
                            "success" => {
                                show_success("Login Complete", Some("Successfully authenticated with GitHub Copilot"));
                                awaiting_auth.set(false);
                                user_code.set(String::new());
                                verification_uri.set(String::new());
                                device_code.set(String::new());
                                refresh_status();
                                break;
                            }
                            "expired" | "denied" | "error" => {
                                let msg = result.error.unwrap_or_else(|| result.status.clone());
                                show_error("Authentication Failed", Some(&msg), None);
                                awaiting_auth.set(false);
                                user_code.set(String::new());
                                verification_uri.set(String::new());
                                device_code.set(String::new());
                                break;
                            }
                            "slow_down" => {
                                poll_interval_secs.update(|i| *i = (*i + 5).min(30));
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        // Check cancellation before updating signals
                        if polling_cancelled.get_untracked() {
                            break;
                        }
                        consecutive_errors += 1;
                        if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                            show_error("Poll Failed", Some(&format!("{} (giving up after {} attempts)", e, MAX_CONSECUTIVE_ERRORS)), None);
                            awaiting_auth.set(false);
                            user_code.set(String::new());
                            verification_uri.set(String::new());
                            device_code.set(String::new());
                            break;
                        }
                        // Show error but continue polling
                        show_error("Poll Failed", Some(&format!("{} (retrying...)", e)), None);
                    }
                }
            }
        });
    });

    // Start Device Code OAuth flow
    let start_auth = move || {
        web_sys::console::log_1(&"[CopilotAuth] start_auth called".into());
        spawn_local(async move {
            web_sys::console::log_1(&"[CopilotAuth] spawn_local started, calling start_copilot_auth".into());
            is_loading.set(true);
            match start_copilot_auth().await {
                Ok(response) => {
                    web_sys::console::log_1(&format!("[CopilotAuth] Got response: user_code={}, uri={}", response.user_code, response.verification_uri).into());
                    user_code.set(response.user_code.clone());
                    verification_uri.set(response.verification_uri.clone());
                    device_code.set(response.device_code.clone());
                    poll_interval_secs.set(response.interval);
                    awaiting_auth.set(true);

                    // Open the verification URL in browser
                    match open_url_in_browser(response.verification_uri.clone()).await {
                        Ok(_) => {
                            show_success(
                                "Login Started",
                                Some(&format!("Enter code {} at GitHub", response.user_code))
                            );
                        }
                        Err(e) => {
                            show_error("Browser Open Failed", Some(&format!("{}. Please open the URL manually.", e)), None);
                        }
                    }
                }
                Err(e) => {
                    web_sys::console::log_1(&format!("[CopilotAuth] Error: {}", e).into());
                    show_error("OAuth Failed", Some(&e), None);
                }
            }
            is_loading.set(false);
        });
    };

    // Logout
    let logout = move || {
        spawn_local(async move {
            is_loading.set(true);
            match logout_copilot().await {
                Ok(_) => {
                    show_success("Logged Out", None);
                    // refresh_status() sets is_loading to true then false when done
                    refresh_status();
                }
                Err(e) => {
                    show_error("Logout Failed", Some(&e), None);
                    is_loading.set(false);
                }
            }
        });
    };

    // Cancel auth
    let cancel_auth = move || {
        awaiting_auth.set(false);
        user_code.set(String::new());
        verification_uri.set(String::new());
        device_code.set(String::new());
    };

    let content = view! {
        <div class=move || format!("space-y-4 {}", if compact { "text-sm" } else { "" })>
            // Header with status
            <div class="flex items-center justify-between">
                <h4 class="font-semibold text-[#6e40c9]">"GitHub Copilot Authentication"</h4>
                {move || {
                    let s = status.get();
                    if s.authenticated {
                        view! {
                            <Badge variant=BadgeVariant::Success>
                                "Authenticated"
                            </Badge>
                        }.into_any()
                    } else {
                        view! {
                            <Badge variant=BadgeVariant::Warning>"Login Required"</Badge>
                        }.into_any()
                    }
                }}
            </div>

            <p class="text-sm text-theme-muted">
                "GitHub Copilot uses Device Code authentication with your GitHub account."
            </p>

            // Storage backend selector
            <div class="space-y-2">
                <label class="text-xs text-theme-muted">"Token Storage Backend"</label>
                <div class="flex flex-col gap-2">
                    <Select
                        value=Signal::derive(move || status.get().storage_backend)
                        on_change=Callback::new(move |value: String| {
                            let backend = match value.as_str() {
                                "keyring" => CopilotStorageBackend::Keyring,
                                "file" => CopilotStorageBackend::File,
                                _ => CopilotStorageBackend::Auto,
                            };
                            spawn_local(async move {
                                is_loading.set(true);
                                match copilot_set_storage_backend(backend).await {
                                    Ok(_) => {
                                        show_success("Storage Changed", Some("You may need to re-authenticate"));
                                        refresh_status();
                                    }
                                    Err(e) => {
                                        show_error("Storage Change Failed", Some(&e), None);
                                        is_loading.set(false);
                                    }
                                }
                            });
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

            // Usage info when authenticated
            {move || {
                if let Some(usage_info) = usage.get() {
                    view! {
                        <div class="p-3 rounded-lg bg-theme-elevated border border-theme-subtle space-y-2">
                            <div class="flex items-center justify-between text-xs">
                                <span class="text-theme-muted">"Plan"</span>
                                <span class="text-theme-primary font-medium">{usage_info.copilot_plan.clone()}</span>
                            </div>
                            <div class="flex items-center justify-between text-xs">
                                <span class="text-theme-muted">"Quota Reset"</span>
                                <span class="text-theme-secondary">{usage_info.quota_reset_date.clone()}</span>
                            </div>
                            {usage_info.premium_requests.map(|pr| {
                                if pr.unlimited {
                                    view! {
                                        <div class="flex items-center justify-between text-xs">
                                            <span class="text-theme-muted">"Premium Requests"</span>
                                            <span class="text-green-400">"Unlimited"</span>
                                        </div>
                                    }.into_any()
                                } else {
                                    let exhausted = pr.is_exhausted;
                                    view! {
                                        <div class="flex items-center justify-between text-xs">
                                            <span class="text-theme-muted">"Premium Requests"</span>
                                            <span class=if exhausted { "text-red-400" } else { "text-theme-secondary" }>
                                                {format!("{} / {}", pr.used, pr.limit)}
                                            </span>
                                        </div>
                                    }.into_any()
                                }
                            })}
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }
            }}

            // Device code display (shown when awaiting auth)
            {move || {
                if awaiting_auth.get() {
                    let code = user_code.get();
                    let uri = verification_uri.get();
                    view! {
                        <div class="flex flex-col gap-3 p-4 rounded-lg bg-theme-elevated border border-[#6e40c9]/30">
                            <div class="text-center space-y-2">
                                <p class="text-sm text-theme-secondary">
                                    "Enter this code at GitHub:"
                                </p>
                                <div class="flex justify-center">
                                    <code class="px-6 py-3 text-2xl font-mono font-bold tracking-widest bg-theme-deep rounded-lg text-[#6e40c9] select-all">
                                        {code.clone()}
                                    </code>
                                </div>
                                <button
                                    type="button"
                                    class="px-3 py-1.5 text-xs font-medium rounded-lg bg-[#6e40c9]/20 text-[#6e40c9] hover:bg-[#6e40c9]/30 transition-colors"
                                    on:click={
                                        let code_copy = code.clone();
                                        move |_| {
                                            let code_to_copy = code_copy.clone();
                                            if let Some(window) = web_sys::window() {
                                                let clipboard = window.navigator().clipboard();
                                                spawn_local(async move {
                                                    match wasm_bindgen_futures::JsFuture::from(
                                                        clipboard.write_text(&code_to_copy)
                                                    ).await {
                                                        Ok(_) => show_success("Copied", Some("Code copied to clipboard")),
                                                        Err(_) => show_error("Copy Failed", Some("Could not copy to clipboard"), None),
                                                    }
                                                });
                                            }
                                        }
                                    }
                                >
                                    "Copy Code"
                                </button>
                            </div>

                            <div class="flex flex-col gap-1 pt-2 border-t border-theme-subtle">
                                <p class="text-xs text-theme-muted">"Visit this URL if the browser didn't open:"</p>
                                <div class="flex gap-2 items-center">
                                    <input
                                        type="text"
                                        readonly
                                        class="flex-1 px-2 py-1 text-xs rounded bg-theme-deep border border-theme-subtle text-theme-muted font-mono truncate"
                                        prop:value=uri.clone()
                                    />
                                    <button
                                        type="button"
                                        class="px-2 py-1 text-xs rounded bg-[#6e40c9]/20 text-[#6e40c9] hover:bg-[#6e40c9]/30"
                                        on:click={
                                            let uri_copy = uri.clone();
                                            move |_| {
                                                let url = uri_copy.clone();
                                                spawn_local(async move {
                                                    let _ = open_url_in_browser(url).await;
                                                });
                                            }
                                        }
                                    >
                                        "Open"
                                    </button>
                                </div>
                            </div>

                            <div class="flex items-center justify-between text-xs text-theme-muted pt-2">
                                <span>"Waiting for authorization..."</span>
                                <button
                                    type="button"
                                    class="px-3 py-1.5 text-xs font-medium rounded-lg bg-theme-surface text-theme-muted hover:bg-theme-elevated transition-colors"
                                    on:click=move |_| cancel_auth()
                                >
                                    "Cancel"
                                </button>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }
            }}

            // Main action buttons
            <div class="flex gap-3">
                {move || {
                    let loading = is_loading.get();
                    let authenticated = status.get().authenticated;
                    let is_awaiting = awaiting_auth.get();
                    if !authenticated && !is_awaiting {
                        view! {
                            <button
                                type="button"
                                class="px-4 py-2 text-sm font-medium rounded-lg bg-[#6e40c9] text-white hover:bg-[#5a32a3] transition-colors disabled:opacity-50"
                                disabled=loading
                                on:click=move |_| start_auth()
                            >
                                "Login with GitHub"
                            </button>
                        }.into_any()
                    } else if authenticated {
                        view! {
                            <button
                                type="button"
                                class="px-4 py-2 text-sm font-medium rounded-lg bg-red-500/20 text-red-400 hover:bg-red-500/30 transition-colors disabled:opacity-50"
                                disabled=loading
                                on:click=move |_| logout()
                            >
                                "Logout"
                            </button>
                        }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }
                }}

                <button
                    type="button"
                    class="px-4 py-2 text-sm font-medium rounded-lg bg-theme-elevated text-theme-secondary hover:bg-theme-surface transition-colors disabled:opacity-50"
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
            <div class="p-6 rounded-xl bg-theme-surface border border-[#6e40c9]/30 space-y-4">
                {content}
            </div>
        }.into_any()
    } else {
        view! { <div>{content}</div> }.into_any()
    }
}

/// Compact status indicator for Copilot authentication.
/// Shows just the authentication status badge.
#[component]
pub fn CopilotStatusBadge() -> impl IntoView {
    let status = RwSignal::new(CopilotAuthStatus::default());

    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(s) = check_copilot_auth().await {
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
