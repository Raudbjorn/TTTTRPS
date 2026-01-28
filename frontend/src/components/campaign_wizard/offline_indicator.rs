//! Offline Fallback and LLM Availability Indicator
//!
//! Detects LLM provider availability and provides UI for offline scenarios.

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

use crate::bindings::invoke;

// ============================================================================
// LLM Availability Types
// ============================================================================

/// LLM availability status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LlmAvailability {
    /// Status unknown (checking)
    #[default]
    Unknown,
    /// LLM is available and ready
    Available,
    /// LLM is unavailable (no API key, network error, etc.)
    Unavailable,
    /// Currently checking availability
    Checking,
}

impl LlmAvailability {
    pub fn is_available(&self) -> bool {
        matches!(self, LlmAvailability::Available)
    }

    pub fn is_known(&self) -> bool {
        !matches!(self, LlmAvailability::Unknown | LlmAvailability::Checking)
    }
}

/// LLM provider info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderStatus {
    pub provider: String,
    pub available: bool,
    pub error: Option<String>,
}

/// Health status returned from check_llm_health command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub provider: String,
    pub healthy: bool,
    pub message: String,
}

// ============================================================================
// LLM Availability Hook
// ============================================================================

/// Hook to check and track LLM availability
pub fn use_llm_availability() -> (
    Signal<LlmAvailability>,
    Signal<Option<String>>,
    Callback<()>,
) {
    let status = RwSignal::new(LlmAvailability::Unknown);
    let error_message: RwSignal<Option<String>> = RwSignal::new(None);
    let check_trigger = Trigger::new();

    // Check availability on mount and when triggered
    Effect::new(move |_| {
        check_trigger.track();
        status.set(LlmAvailability::Checking);

        spawn_local(async move {
            match check_llm_availability().await {
                Ok(true) => {
                    status.set(LlmAvailability::Available);
                    error_message.set(None);
                }
                Ok(false) => {
                    status.set(LlmAvailability::Unavailable);
                    error_message.set(Some("No LLM provider configured".to_string()));
                }
                Err(e) => {
                    status.set(LlmAvailability::Unavailable);
                    error_message.set(Some(e));
                }
            }
        });
    });

    let recheck = Callback::new(move |_: ()| {
        check_trigger.notify();
    });

    (
        Signal::derive(move || status.get()),
        Signal::derive(move || error_message.get()),
        recheck,
    )
}

/// Check if any LLM provider is available using structured health status
async fn check_llm_availability() -> Result<bool, String> {
    #[derive(Serialize)]
    struct Args {}

    // Use check_llm_health which returns structured HealthStatus
    let result: Result<HealthStatus, String> = invoke("check_llm_health", &Args {}).await;

    match result {
        Ok(status) => {
            // HealthStatus.healthy directly tells us if the provider is available
            Ok(status.healthy)
        }
        Err(e) => {
            // If the command fails entirely (e.g., no provider configured at all),
            // treat as unavailable rather than error
            log::debug!("LLM health check failed: {}", e);
            Ok(false)
        }
    }
}

// ============================================================================
// UI Components
// ============================================================================

/// Banner showing AI unavailable status
#[component]
pub fn AiUnavailableBanner(
    /// Error message to display
    error: Signal<Option<String>>,
    /// Callback to retry checking
    on_retry: Callback<()>,
    /// Whether to show in compact mode
    #[prop(default = false)]
    compact: bool,
) -> impl IntoView {
    if compact {
        view! {
            <div class="flex items-center gap-2 px-3 py-1.5 bg-amber-900/30 border border-amber-700/50 rounded-lg text-sm">
                <svg class="w-4 h-4 text-amber-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M18.364 5.636a9 9 0 010 12.728m0 0l-2.829-2.829m2.829 2.829L21 21M15.536 8.464a5 5 0 010 7.072m0 0l-2.829-2.829m-4.243 2.829a4.978 4.978 0 01-1.414-2.83m-1.414 5.658a9 9 0 01-2.167-9.238m7.824 2.167a1 1 0 111.414 1.414m-1.414-1.414L3 3m8.293 8.293l1.414 1.414" />
                </svg>
                <span class="text-amber-300">"AI Unavailable"</span>
                <button
                    type="button"
                    class="text-amber-400 hover:text-amber-300 underline"
                    on:click=move |_| on_retry.run(())
                >
                    "Retry"
                </button>
            </div>
        }.into_any()
    } else {
        view! {
            <div class="p-4 bg-amber-900/20 border border-amber-700/50 rounded-lg">
                <div class="flex items-start gap-3">
                    <div class="p-2 bg-amber-900/50 rounded-lg shrink-0">
                        <svg class="w-5 h-5 text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                d="M18.364 5.636a9 9 0 010 12.728m0 0l-2.829-2.829m2.829 2.829L21 21M15.536 8.464a5 5 0 010 7.072m0 0l-2.829-2.829m-4.243 2.829a4.978 4.978 0 01-1.414-2.83m-1.414 5.658a9 9 0 01-2.167-9.238m7.824 2.167a1 1 0 111.414 1.414m-1.414-1.414L3 3m8.293 8.293l1.414 1.414" />
                        </svg>
                    </div>
                    <div class="flex-1">
                        <h4 class="font-medium text-amber-300">"AI Assistant Unavailable"</h4>
                        <p class="text-sm text-amber-400/80 mt-1">
                            {move || error.get().unwrap_or_else(|| "Unable to connect to AI provider.".to_string())}
                        </p>
                        <p class="text-sm text-zinc-400 mt-2">
                            "You can still complete the wizard manually. AI features will be available once connection is restored."
                        </p>
                        <div class="flex items-center gap-3 mt-3">
                            <button
                                type="button"
                                class="px-3 py-1.5 bg-amber-700 hover:bg-amber-600 text-white text-sm rounded transition-colors"
                                on:click=move |_| on_retry.run(())
                            >
                                "Check Again"
                            </button>
                            <a
                                href="/settings"
                                class="text-sm text-amber-400 hover:text-amber-300 underline"
                            >
                                "Configure API Keys"
                            </a>
                        </div>
                    </div>
                </div>
            </div>
        }.into_any()
    }
}

/// Small indicator dot for AI status
#[component]
pub fn AiStatusDot(
    status: Signal<LlmAvailability>,
    #[prop(default = false)]
    show_label: bool,
) -> impl IntoView {
    view! {
        <div class="flex items-center gap-1.5" title={move || match status.get() {
            LlmAvailability::Available => "AI Available",
            LlmAvailability::Unavailable => "AI Unavailable",
            LlmAvailability::Checking => "Checking AI...",
            LlmAvailability::Unknown => "AI Status Unknown",
        }}>
            <div class=move || format!(
                "w-2 h-2 rounded-full {}",
                match status.get() {
                    LlmAvailability::Available => "bg-green-400",
                    LlmAvailability::Unavailable => "bg-amber-400",
                    LlmAvailability::Checking => "bg-zinc-400 animate-pulse",
                    LlmAvailability::Unknown => "bg-zinc-600",
                }
            ) />
            {show_label.then(|| view! {
                <span class=move || format!(
                    "text-xs {}",
                    match status.get() {
                        LlmAvailability::Available => "text-green-400",
                        LlmAvailability::Unavailable => "text-amber-400",
                        _ => "text-zinc-500",
                    }
                )>
                    {move || match status.get() {
                        LlmAvailability::Available => "AI Ready",
                        LlmAvailability::Unavailable => "AI Offline",
                        LlmAvailability::Checking => "Checking...",
                        LlmAvailability::Unknown => "Unknown",
                    }}
                </span>
            })}
        </div>
    }
}

/// Inline notice when AI features are disabled
#[component]
pub fn AiFeatureDisabledNotice(
    /// Feature name that requires AI
    feature: &'static str,
) -> impl IntoView {
    view! {
        <div class="flex items-center gap-2 px-3 py-2 bg-zinc-800/50 border border-zinc-700 rounded-lg text-sm text-zinc-400">
            <svg class="w-4 h-4 text-zinc-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                    d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <span>{format!("{} requires AI assistance", feature)}</span>
        </div>
    }
}

// ============================================================================
// Offline Queue (for future implementation)
// ============================================================================

/// Placeholder for queued AI requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedAiRequest {
    pub id: String,
    pub request_type: String,
    pub payload: String,
    pub created_at: String,
}

/// Queue state for offline AI requests
#[derive(Clone)]
pub struct OfflineQueue {
    pub requests: RwSignal<Vec<QueuedAiRequest>>,
    pub is_processing: RwSignal<bool>,
}

impl OfflineQueue {
    pub fn new() -> Self {
        Self {
            requests: RwSignal::new(Vec::new()),
            is_processing: RwSignal::new(false),
        }
    }

    /// Add a request to the queue
    pub fn enqueue(&self, request_type: &str, payload: &str) {
        let request = QueuedAiRequest {
            id: uuid::Uuid::new_v4().to_string(),
            request_type: request_type.to_string(),
            payload: payload.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.requests.update(|r| r.push(request));
    }

    /// Get queue length
    pub fn len(&self) -> usize {
        self.requests.get().len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.requests.get().is_empty()
    }

    /// Clear the queue
    pub fn clear(&self) {
        self.requests.set(Vec::new());
    }
}

impl Default for OfflineQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Indicator showing queued requests
#[component]
pub fn QueuedRequestsIndicator(queue: OfflineQueue) -> impl IntoView {
    let count = Signal::derive(move || queue.requests.get().len());

    view! {
        <Show when=move || { count.get() > 0 }>
            <div class="flex items-center gap-2 px-3 py-1.5 bg-blue-900/30 border border-blue-700/50 rounded-lg text-sm">
                <svg class="w-4 h-4 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <span class="text-blue-300">
                    {move || format!("{} request{} queued", count.get(), if count.get() == 1 { "" } else { "s" })}
                </span>
            </div>
        </Show>
    }
}
