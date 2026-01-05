//! Usage Dashboard Component
//!
//! Displays token usage, costs, and budget status for LLM providers.

use leptos::prelude::*;
use leptos::ev;
use wasm_bindgen_futures::spawn_local;
use std::collections::HashMap;

use crate::bindings::{
    get_usage_stats, get_usage_by_period, get_cost_breakdown, get_budget_status,
    set_budget_limit, UsageStats, CostBreakdown, BudgetStatus, BudgetLimit,
};
use crate::components::design_system::{Button, ButtonVariant, Card, CardBody, CardHeader, Input, Select};

// ============================================================================
// Usage Dashboard Component
// ============================================================================

#[component]
pub fn UsageDashboard() -> impl IntoView {
    // State signals
    let usage_stats = RwSignal::new(Option::<UsageStats>::None);
    let cost_breakdown = RwSignal::new(Option::<CostBreakdown>::None);
    let budget_statuses = RwSignal::new(Vec::<BudgetStatus>::new());
    let selected_period = RwSignal::new("24".to_string()); // hours
    let is_loading = RwSignal::new(false);
    let error_message = RwSignal::new(Option::<String>::None);

    // Budget configuration
    let budget_limit = RwSignal::new("50.0".to_string());
    let budget_period = RwSignal::new("monthly".to_string());
    let is_saving_budget = RwSignal::new(false);

    // Load data on mount
    Effect::new(move |_| {
        let hours: i64 = selected_period.get().parse().unwrap_or(24);
        spawn_local(async move {
            is_loading.set(true);
            error_message.set(None);

            // Fetch usage stats
            match get_usage_by_period(hours).await {
                Ok(stats) => usage_stats.set(Some(stats)),
                Err(e) => error_message.set(Some(format!("Failed to load usage stats: {}", e))),
            }

            // Fetch cost breakdown
            match get_cost_breakdown(Some(hours)).await {
                Ok(breakdown) => cost_breakdown.set(Some(breakdown)),
                Err(e) => {
                    if error_message.get().is_none() {
                        error_message.set(Some(format!("Failed to load cost breakdown: {}", e)));
                    }
                }
            }

            // Fetch budget status
            match get_budget_status().await {
                Ok(statuses) => budget_statuses.set(statuses),
                Err(_) => {} // Budget might not be configured
            }

            is_loading.set(false);
        });
    });

    // Handle period change
    let on_period_change = move |val: String| {
        selected_period.set(val);
    };

    // Handle save budget
    let save_budget = move |_: ev::MouseEvent| {
        is_saving_budget.set(true);
        let limit: f64 = budget_limit.get().parse().unwrap_or(50.0);
        let period = budget_period.get();

        spawn_local(async move {
            let budget = BudgetLimit {
                limit_usd: limit,
                period: period.clone(),
                warning_threshold: 0.8,
                critical_threshold: 0.95,
                block_on_limit: false,
            };

            match set_budget_limit(budget).await {
                Ok(_) => {
                    // Refresh budget status
                    if let Ok(statuses) = get_budget_status().await {
                        budget_statuses.set(statuses);
                    }
                }
                Err(e) => {
                    error_message.set(Some(format!("Failed to save budget: {}", e)));
                }
            }
            is_saving_budget.set(false);
        });
    };

    view! {
        <div class="space-y-6">
            // Header with period selector
            <div class="flex items-center justify-between">
                <h2 class="text-xl font-bold text-theme-primary">"Usage & Costs"</h2>
                <div class="flex items-center gap-4">
                    <label class="text-sm text-theme-secondary">"Time Period:"</label>
                    <Select
                        value=selected_period.get()
                        on_change=Callback::new(on_period_change)
                    >
                        <option value="1">"Last Hour"</option>
                        <option value="24">"Last 24 Hours"</option>
                        <option value="168">"Last Week"</option>
                        <option value="720">"Last 30 Days"</option>
                    </Select>
                </div>
            </div>

            // Error message
            <Show when=move || error_message.get().is_some()>
                <div class="p-4 bg-red-900/20 border border-red-500/50 rounded text-red-400">
                    {move || error_message.get().unwrap_or_default()}
                </div>
            </Show>

            // Loading state
            <Show when=move || is_loading.get()>
                <div class="text-center py-8 text-theme-secondary">
                    "Loading usage data..."
                </div>
            </Show>

            // Main content
            <Show when=move || !is_loading.get()>
                // Summary Cards
                <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
                    // Total Requests
                    <Card>
                        <CardBody>
                            <div class="text-center">
                                <div class="text-3xl font-bold text-theme-accent">
                                    {move || usage_stats.get().map(|s| s.total_requests.to_string()).unwrap_or("0".to_string())}
                                </div>
                                <div class="text-sm text-theme-secondary">"Total Requests"</div>
                            </div>
                        </CardBody>
                    </Card>

                    // Input Tokens
                    <Card>
                        <CardBody>
                            <div class="text-center">
                                <div class="text-3xl font-bold text-blue-400">
                                    {move || usage_stats.get().map(|s| format_tokens(s.total_input_tokens)).unwrap_or("0".to_string())}
                                </div>
                                <div class="text-sm text-theme-secondary">"Input Tokens"</div>
                            </div>
                        </CardBody>
                    </Card>

                    // Output Tokens
                    <Card>
                        <CardBody>
                            <div class="text-center">
                                <div class="text-3xl font-bold text-green-400">
                                    {move || usage_stats.get().map(|s| format_tokens(s.total_output_tokens)).unwrap_or("0".to_string())}
                                </div>
                                <div class="text-sm text-theme-secondary">"Output Tokens"</div>
                            </div>
                        </CardBody>
                    </Card>

                    // Total Cost
                    <Card>
                        <CardBody>
                            <div class="text-center">
                                <div class="text-3xl font-bold text-yellow-400">
                                    {move || usage_stats.get().map(|s| format!("${:.4}", s.total_cost_usd)).unwrap_or("$0.00".to_string())}
                                </div>
                                <div class="text-sm text-theme-secondary">"Estimated Cost"</div>
                            </div>
                        </CardBody>
                    </Card>
                </div>

                // Cost Breakdown by Provider
                <Card>
                    <CardHeader>
                        <h3 class="text-lg font-semibold">"Cost by Provider"</h3>
                    </CardHeader>
                    <CardBody>
                        {move || {
                            cost_breakdown.get().map(|breakdown| {
                                if breakdown.by_provider.is_empty() {
                                    view! {
                                        <div class="text-center py-4 text-theme-secondary">
                                            "No usage data for this period"
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="space-y-3">
                                            {breakdown.by_provider.iter().map(|(name, details)| {
                                                let percentage = if breakdown.total_cost_usd > 0.0 {
                                                    (details.total_cost_usd / breakdown.total_cost_usd) * 100.0
                                                } else {
                                                    0.0
                                                };

                                                view! {
                                                    <div class="space-y-1">
                                                        <div class="flex justify-between text-sm">
                                                            <span class="font-medium text-theme-primary">{name.clone()}</span>
                                                            <span class="text-theme-secondary">
                                                                {format!("${:.4} ({:.1}%)", details.total_cost_usd, percentage)}
                                                            </span>
                                                        </div>
                                                        <div class="w-full bg-gray-700 rounded-full h-2">
                                                            <div
                                                                class="bg-gradient-to-r from-purple-500 to-blue-500 h-2 rounded-full transition-all duration-300"
                                                                style=format!("width: {}%", percentage.min(100.0))
                                                            />
                                                        </div>
                                                        <div class="flex justify-between text-xs text-theme-secondary">
                                                            <span>{format!("{} requests", details.requests)}</span>
                                                            <span>{format!("{}in / {}out tokens", format_tokens(details.input_tokens), format_tokens(details.output_tokens))}</span>
                                                        </div>
                                                    </div>
                                                }
                                            }).collect_view()}
                                        </div>
                                    }.into_any()
                                }
                            }).unwrap_or_else(|| {
                                view! {
                                    <div class="text-center py-4 text-theme-secondary">
                                        "No cost data available"
                                    </div>
                                }.into_any()
                            })
                        }}
                    </CardBody>
                </Card>

                // Budget Status
                <Card>
                    <CardHeader>
                        <h3 class="text-lg font-semibold">"Budget Status"</h3>
                    </CardHeader>
                    <CardBody class="space-y-4">
                        // Current budget statuses
                        {move || {
                            let statuses = budget_statuses.get();
                            if statuses.is_empty() {
                                view! {
                                    <div class="text-center py-2 text-theme-secondary">
                                        "No budget limits configured"
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="space-y-3">
                                        {statuses.iter().map(|status| {
                                            let (status_color, status_text) = match status.status.as_str() {
                                                "exceeded" => ("text-red-400", "Exceeded"),
                                                "critical" => ("text-red-400", "Critical"),
                                                "warning" => ("text-yellow-400", "Warning"),
                                                _ => ("text-green-400", "Normal"),
                                            };

                                            view! {
                                                <div class="p-3 bg-theme-secondary rounded space-y-2">
                                                    <div class="flex justify-between items-center">
                                                        <span class="font-medium capitalize">{status.period.clone()}</span>
                                                        <span class=status_color>{status_text}</span>
                                                    </div>
                                                    <div class="flex justify-between text-sm text-theme-secondary">
                                                        <span>{format!("${:.2} / ${:.2}", status.spent_usd, status.limit_usd)}</span>
                                                        <span>{format!("{:.1}% used", status.percentage_used * 100.0)}</span>
                                                    </div>
                                                    <div class="w-full bg-gray-600 rounded-full h-2">
                                                        <div
                                                            class=format!("h-2 rounded-full transition-all duration-300 {}", match status.status.as_str() {
                                                                "exceeded" | "critical" => "bg-red-500",
                                                                "warning" => "bg-yellow-500",
                                                                _ => "bg-green-500",
                                                            })
                                                            style=format!("width: {}%", (status.percentage_used * 100.0).min(100.0))
                                                        />
                                                    </div>
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                }.into_any()
                            }
                        }}

                        // Configure budget
                        <div class="pt-4 border-t border-gray-700">
                            <h4 class="text-sm font-medium text-theme-secondary mb-3">"Set Budget Limit"</h4>
                            <div class="flex gap-4 items-end">
                                <div class="flex-1">
                                    <label class="block text-xs text-theme-secondary mb-1">"Limit (USD)"</label>
                                    <Input
                                        value=budget_limit
                                        placeholder="50.00"
                                        r#type="number"
                                    />
                                </div>
                                <div class="flex-1">
                                    <label class="block text-xs text-theme-secondary mb-1">"Period"</label>
                                    <Select
                                        value=budget_period.get()
                                        on_change=Callback::new(move |val: String| budget_period.set(val))
                                    >
                                        <option value="hourly">"Hourly"</option>
                                        <option value="daily">"Daily"</option>
                                        <option value="weekly">"Weekly"</option>
                                        <option value="monthly">"Monthly"</option>
                                    </Select>
                                </div>
                                <Button
                                    variant=ButtonVariant::Primary
                                    loading=is_saving_budget.get()
                                    on_click=save_budget
                                >
                                    "Set Limit"
                                </Button>
                            </div>
                        </div>
                    </CardBody>
                </Card>

                // Model Usage Breakdown
                <Card>
                    <CardHeader>
                        <h3 class="text-lg font-semibold">"Usage by Model"</h3>
                    </CardHeader>
                    <CardBody>
                        {move || {
                            cost_breakdown.get().map(|breakdown| {
                                if breakdown.by_model.is_empty() {
                                    view! {
                                        <div class="text-center py-4 text-theme-secondary">
                                            "No model usage data"
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="overflow-x-auto">
                                            <table class="w-full text-sm">
                                                <thead>
                                                    <tr class="border-b border-gray-700">
                                                        <th class="text-left py-2 text-theme-secondary">"Model"</th>
                                                        <th class="text-right py-2 text-theme-secondary">"Requests"</th>
                                                        <th class="text-right py-2 text-theme-secondary">"Input"</th>
                                                        <th class="text-right py-2 text-theme-secondary">"Output"</th>
                                                        <th class="text-right py-2 text-theme-secondary">"Cost"</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    {breakdown.by_model.iter().map(|(_key, details)| {
                                                        view! {
                                                            <tr class="border-b border-gray-800">
                                                                <td class="py-2">
                                                                    <div class="font-medium text-theme-primary">{details.model.clone()}</div>
                                                                    <div class="text-xs text-theme-secondary">{details.provider.clone()}</div>
                                                                </td>
                                                                <td class="text-right py-2">{details.requests.to_string()}</td>
                                                                <td class="text-right py-2">{format_tokens(details.input_tokens)}</td>
                                                                <td class="text-right py-2">{format_tokens(details.output_tokens)}</td>
                                                                <td class="text-right py-2 text-yellow-400">{format!("${:.4}", details.total_cost_usd)}</td>
                                                            </tr>
                                                        }
                                                    }).collect_view()}
                                                </tbody>
                                            </table>
                                        </div>
                                    }.into_any()
                                }
                            }).unwrap_or_else(|| {
                                view! {
                                    <div class="text-center py-4 text-theme-secondary">
                                        "No model data available"
                                    </div>
                                }.into_any()
                            })
                        }}
                    </CardBody>
                </Card>
            </Show>
        </div>
    }
}

// Helper function to format large token numbers
fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

// ============================================================================
// Wrapper Page Component with Navigation
// ============================================================================

#[component]
pub fn UsageDashboardPage() -> impl IntoView {
    use leptos_router::hooks::use_navigate;

    let navigate = use_navigate();

    let handle_back = {
        let navigate = navigate.clone();
        move |_: ev::MouseEvent| {
            navigate("/settings", Default::default());
        }
    };

    view! {
        <div class="p-8 bg-theme-primary text-theme-primary min-h-screen font-sans transition-colors duration-300">
            <div class="max-w-6xl mx-auto">
                // Back button
                <div class="mb-6">
                    <button
                        class="text-gray-400 hover:text-white transition-colors"
                        on:click=handle_back
                    >
                        "< Back to Settings"
                    </button>
                </div>

                // Main content
                <UsageDashboard />
            </div>
        </div>
    }
}
