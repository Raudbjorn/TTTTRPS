//! Search Analytics Component
//!
//! Displays search query analytics, popular searches, and cache statistics.
//! Supports both in-memory (session) and database-backed (historical) data sources.

use leptos::prelude::*;
use leptos::ev;
use wasm_bindgen_futures::spawn_local;
use std::collections::HashMap;

use crate::bindings::{
    // In-memory (session) analytics
    get_search_analytics, get_popular_queries, get_cache_stats,
    get_trending_queries, get_zero_result_queries,
    // Database-backed (historical) analytics
    get_search_analytics_db, get_popular_queries_db, get_cache_stats_db,
    get_trending_queries_db, get_zero_result_queries_db,
    cleanup_search_analytics,
    SearchAnalyticsSummary, PopularQuery, CacheStats,
};
use crate::components::design_system::{Button, ButtonVariant, Card, CardBody, CardHeader, Select};

// ============================================================================
// Search Analytics Component
// ============================================================================

#[component]
pub fn SearchAnalyticsDashboard() -> impl IntoView {
    // State signals
    let analytics_summary = RwSignal::new(Option::<SearchAnalyticsSummary>::None);
    let popular_queries = RwSignal::new(Vec::<PopularQuery>::new());
    let cache_stats = RwSignal::new(Option::<CacheStats>::None);
    let trending_queries = RwSignal::new(Vec::<String>::new());
    let selected_period = RwSignal::new("24".to_string()); // hours
    let use_database = RwSignal::new(true); // Use database-backed analytics by default
    let is_loading = RwSignal::new(false);
    let error_message = RwSignal::new(Option::<String>::None);

    // Load data on mount and when period/source changes
    Effect::new(move |_| {
        let hours: i64 = selected_period.get().parse().unwrap_or(24);
        let use_db = use_database.get();

        spawn_local(async move {
            is_loading.set(true);
            error_message.set(None);

            // Fetch analytics summary (from database or in-memory)
            let summary_result = if use_db {
                get_search_analytics_db(hours).await
            } else {
                get_search_analytics(hours).await
            };

            match summary_result {
                Ok(summary) => analytics_summary.set(Some(summary)),
                Err(e) => error_message.set(Some(format!("Failed to load analytics: {}", e))),
            }

            // Fetch popular queries
            let queries_result = if use_db {
                get_popular_queries_db(20).await
            } else {
                get_popular_queries(20).await
            };

            match queries_result {
                Ok(queries) => popular_queries.set(queries),
                Err(_) => {} // Non-critical
            }

            // Fetch cache stats
            let cache_result = if use_db {
                get_cache_stats_db().await
            } else {
                get_cache_stats().await
            };

            match cache_result {
                Ok(stats) => cache_stats.set(Some(stats)),
                Err(_) => {} // Non-critical
            }

            // Fetch trending queries
            let trending_result = if use_db {
                get_trending_queries_db(10).await
            } else {
                get_trending_queries(10).await
            };

            match trending_result {
                Ok(queries) => trending_queries.set(queries),
                Err(_) => {} // Non-critical
            }

            is_loading.set(false);
        });
    });

    // Handle period change
    let on_period_change = move |val: String| {
        selected_period.set(val);
    };

    // Toggle data source
    let on_toggle_source = move |_: ev::MouseEvent| {
        use_database.update(|v| *v = !*v);
    };

    view! {
        <div class="space-y-6">
            // Header with period selector and source toggle
            <div class="flex flex-wrap items-center justify-between gap-4">
                <h2 class="text-xl font-bold text-theme-primary">"Search Analytics"</h2>
                <div class="flex items-center gap-4">
                    // Data source toggle
                    <div class="flex items-center gap-2">
                        <button
                            class=move || format!(
                                "px-3 py-1 text-sm rounded-l transition-colors {}",
                                if use_database.get() { "bg-purple-600 text-white" } else { "bg-gray-700 text-gray-400" }
                            )
                            on:click=on_toggle_source.clone()
                        >
                            "Historical"
                        </button>
                        <button
                            class=move || format!(
                                "px-3 py-1 text-sm rounded-r transition-colors {}",
                                if !use_database.get() { "bg-purple-600 text-white" } else { "bg-gray-700 text-gray-400" }
                            )
                            on:click=on_toggle_source
                        >
                            "Session"
                        </button>
                    </div>

                    // Period selector
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

            // Data source indicator
            <div class="text-sm text-theme-secondary">
                {move || if use_database.get() {
                    "Showing historical data from database (persisted across sessions)"
                } else {
                    "Showing current session data only (in-memory)"
                }}
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
                    "Loading search analytics..."
                </div>
            </Show>

            // Main content
            <Show when=move || !is_loading.get()>
                // Summary Cards
                <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
                    // Total Searches
                    <Card>
                        <CardBody>
                            <div class="text-center">
                                <div class="text-3xl font-bold text-theme-accent">
                                    {move || analytics_summary.get().map(|s| s.total_searches.to_string()).unwrap_or("0".to_string())}
                                </div>
                                <div class="text-sm text-theme-secondary">"Total Searches"</div>
                            </div>
                        </CardBody>
                    </Card>

                    // Click-Through Rate
                    <Card>
                        <CardBody>
                            <div class="text-center">
                                <div class="text-3xl font-bold text-green-400">
                                    {move || analytics_summary.get().map(|s| format!("{:.1}%", s.click_through_rate * 100.0)).unwrap_or("0%".to_string())}
                                </div>
                                <div class="text-sm text-theme-secondary">"Click-Through Rate"</div>
                            </div>
                        </CardBody>
                    </Card>

                    // Average Results
                    <Card>
                        <CardBody>
                            <div class="text-center">
                                <div class="text-3xl font-bold text-blue-400">
                                    {move || analytics_summary.get().map(|s| format!("{:.1}", s.avg_results_per_search)).unwrap_or("0".to_string())}
                                </div>
                                <div class="text-sm text-theme-secondary">"Avg Results/Search"</div>
                            </div>
                        </CardBody>
                    </Card>

                    // Zero Result Searches
                    <Card>
                        <CardBody>
                            <div class="text-center">
                                <div class="text-3xl font-bold text-yellow-400">
                                    {move || analytics_summary.get().map(|s| s.zero_result_searches.to_string()).unwrap_or("0".to_string())}
                                </div>
                                <div class="text-sm text-theme-secondary">"Zero Results"</div>
                            </div>
                        </CardBody>
                    </Card>
                </div>

                // Cache Statistics Card
                <Card>
                    <CardHeader>
                        <h3 class="text-lg font-semibold">"Cache Performance"</h3>
                    </CardHeader>
                    <CardBody>
                        {move || {
                            cache_stats.get().map(|stats| {
                                view! {
                                    <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
                                        // Hit Rate
                                        <div class="text-center">
                                            <div class="relative w-24 h-24 mx-auto">
                                                <svg class="w-24 h-24 transform -rotate-90">
                                                    <circle
                                                        cx="48"
                                                        cy="48"
                                                        r="40"
                                                        stroke="currentColor"
                                                        stroke-width="8"
                                                        fill="none"
                                                        class="text-gray-700"
                                                    />
                                                    <circle
                                                        cx="48"
                                                        cy="48"
                                                        r="40"
                                                        stroke="currentColor"
                                                        stroke-width="8"
                                                        fill="none"
                                                        stroke-dasharray=format!("{} 251.2", stats.hit_rate * 251.2)
                                                        class="text-green-500"
                                                    />
                                                </svg>
                                                <div class="absolute inset-0 flex items-center justify-center">
                                                    <span class="text-lg font-bold">{format!("{:.0}%", stats.hit_rate * 100.0)}</span>
                                                </div>
                                            </div>
                                            <div class="text-sm text-theme-secondary mt-2">"Cache Hit Rate"</div>
                                        </div>

                                        // Hits / Misses
                                        <div class="space-y-4">
                                            <div class="flex justify-between items-center">
                                                <span class="text-theme-secondary">"Hits"</span>
                                                <span class="font-bold text-green-400">{stats.hits.to_string()}</span>
                                            </div>
                                            <div class="flex justify-between items-center">
                                                <span class="text-theme-secondary">"Misses"</span>
                                                <span class="font-bold text-red-400">{stats.misses.to_string()}</span>
                                            </div>
                                            <div class="w-full bg-gray-700 rounded-full h-3">
                                                <div
                                                    class="bg-green-500 h-3 rounded-l-full"
                                                    style=format!("width: {}%", stats.hit_rate * 100.0)
                                                />
                                            </div>
                                        </div>

                                        // Time Saved
                                        <div class="text-center">
                                            <div class="text-3xl font-bold text-purple-400">
                                                {format!("{:.1}s", stats.total_time_saved_ms as f64 / 1000.0)}
                                            </div>
                                            <div class="text-sm text-theme-secondary">"Total Time Saved"</div>
                                            <div class="text-xs text-theme-secondary mt-1">
                                                {format!("Avg {:.0}ms/hit", stats.avg_time_saved_ms)}
                                            </div>
                                        </div>
                                    </div>
                                }
                            }).unwrap_or_else(|| {
                                view! {
                                    <div class="text-center py-4 text-theme-secondary">
                                        "No cache data available"
                                    </div>
                                }
                            })
                        }}
                    </CardBody>
                </Card>

                // Popular Queries
                <Card>
                    <CardHeader>
                        <h3 class="text-lg font-semibold">"Popular Searches"</h3>
                    </CardHeader>
                    <CardBody>
                        {move || {
                            let queries = popular_queries.get();
                            if queries.is_empty() {
                                view! {
                                    <div class="text-center py-4 text-theme-secondary">
                                        "No search data for this period"
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="space-y-2">
                                        {queries.iter().enumerate().take(10).map(|(i, query)| {
                                            let max_count = queries.first().map(|q| q.count).unwrap_or(1);
                                            let percentage = (query.count as f64 / max_count as f64) * 100.0;

                                            view! {
                                                <div class="flex items-center gap-3 group hover:bg-theme-secondary/50 p-2 rounded transition-colors">
                                                    <span class="text-sm font-mono text-theme-secondary w-6">{format!("#{}", i + 1)}</span>
                                                    <div class="flex-1 min-w-0">
                                                        <div class="flex items-center gap-2">
                                                            <span class="font-medium text-theme-primary truncate">{query.query.clone()}</span>
                                                            <span class="text-xs text-theme-secondary">{format!("({} searches)", query.count)}</span>
                                                        </div>
                                                        <div class="w-full bg-gray-700 rounded-full h-1 mt-1">
                                                            <div
                                                                class="bg-purple-500 h-1 rounded-full transition-all duration-300"
                                                                style=format!("width: {}%", percentage)
                                                            />
                                                        </div>
                                                    </div>
                                                    <div class="text-right">
                                                        <span class=format!("text-sm font-medium {}", if query.click_through_rate > 0.5 { "text-green-400" } else if query.click_through_rate > 0.2 { "text-yellow-400" } else { "text-red-400" })>
                                                            {format!("{:.0}% CTR", query.click_through_rate * 100.0)}
                                                        </span>
                                                    </div>
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                }.into_any()
                            }
                        }}
                    </CardBody>
                </Card>

                // Trending Queries
                <Card>
                    <CardHeader>
                        <h3 class="text-lg font-semibold">"Trending Searches"</h3>
                    </CardHeader>
                    <CardBody>
                        {move || {
                            let trends = trending_queries.get();
                            if trends.is_empty() {
                                view! {
                                    <div class="text-center py-4 text-theme-secondary">
                                        "Not enough data for trend analysis"
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="flex flex-wrap gap-2">
                                        {trends.iter().enumerate().map(|(i, query)| {
                                            let trend_color = if i < 3 { "text-orange-400" } else { "text-yellow-400" };
                                            view! {
                                                <div class=format!("px-3 py-2 bg-gray-800 rounded-lg flex items-center gap-2 {}", trend_color)>
                                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6" />
                                                    </svg>
                                                    <span class="text-sm font-medium">{query.clone()}</span>
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                }.into_any()
                            }
                        }}
                    </CardBody>
                </Card>

                // Failed Queries (Zero Results)
                <Card>
                    <CardHeader>
                        <h3 class="text-lg font-semibold">"Queries with No Results"</h3>
                    </CardHeader>
                    <CardBody>
                        {move || {
                            analytics_summary.get().map(|summary| {
                                if summary.failed_queries.is_empty() {
                                    view! {
                                        <div class="text-center py-4 text-green-400">
                                            "All searches returned results"
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="space-y-2">
                                            <p class="text-sm text-theme-secondary mb-3">
                                                "Consider adding content or improving indexing for these queries:"
                                            </p>
                                            <div class="flex flex-wrap gap-2">
                                                {summary.failed_queries.iter().map(|query| {
                                                    view! {
                                                        <span class="px-3 py-1 bg-red-900/30 border border-red-500/30 text-red-400 rounded-full text-sm">
                                                            {query.clone()}
                                                        </span>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        </div>
                                    }.into_any()
                                }
                            }).unwrap_or_else(|| {
                                view! {
                                    <div class="text-center py-4 text-theme-secondary">
                                        "No data available"
                                    </div>
                                }.into_any()
                            })
                        }}
                    </CardBody>
                </Card>

                // Search Type Breakdown
                <Card>
                    <CardHeader>
                        <h3 class="text-lg font-semibold">"Search Type Distribution"</h3>
                    </CardHeader>
                    <CardBody>
                        {move || {
                            analytics_summary.get().map(|summary| {
                                let total: u32 = summary.by_search_type.values().sum();
                                if total == 0 {
                                    view! {
                                        <div class="text-center py-4 text-theme-secondary">
                                            "No search type data"
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                                            {summary.by_search_type.iter().map(|(search_type, count)| {
                                                let percentage = (*count as f64 / total as f64) * 100.0;
                                                let color = match search_type.as_str() {
                                                    "hybrid" => "bg-purple-500",
                                                    "semantic" => "bg-blue-500",
                                                    "keyword" => "bg-green-500",
                                                    _ => "bg-gray-500",
                                                };

                                                view! {
                                                    <div class="p-4 bg-theme-secondary rounded">
                                                        <div class="flex items-center justify-between mb-2">
                                                            <span class="font-medium capitalize">{search_type.clone()}</span>
                                                            <span class="text-theme-secondary">{format!("{:.1}%", percentage)}</span>
                                                        </div>
                                                        <div class="w-full bg-gray-700 rounded-full h-2">
                                                            <div
                                                                class=format!("{} h-2 rounded-full", color)
                                                                style=format!("width: {}%", percentage)
                                                            />
                                                        </div>
                                                        <div class="text-sm text-theme-secondary mt-1">
                                                            {format!("{} searches", count)}
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
                                        "No search type data"
                                    </div>
                                }.into_any()
                            })
                        }}
                    </CardBody>
                </Card>

                // Performance Metrics
                <Card>
                    <CardHeader>
                        <h3 class="text-lg font-semibold">"Performance"</h3>
                    </CardHeader>
                    <CardBody>
                        {move || {
                            analytics_summary.get().map(|summary| {
                                view! {
                                    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                                        <div class="space-y-4">
                                            <div class="flex justify-between items-center p-3 bg-theme-secondary rounded">
                                                <span class="text-theme-secondary">"Average Response Time"</span>
                                                <span class="font-bold text-theme-primary">{format!("{:.0}ms", summary.avg_execution_time_ms)}</span>
                                            </div>
                                            <div class="flex justify-between items-center p-3 bg-theme-secondary rounded">
                                                <span class="text-theme-secondary">"Success Rate"</span>
                                                <span class="font-bold text-green-400">
                                                    {format!("{:.1}%", if summary.total_searches > 0 {
                                                        ((summary.total_searches - summary.zero_result_searches) as f64 / summary.total_searches as f64) * 100.0
                                                    } else {
                                                        0.0
                                                    })}
                                                </span>
                                            </div>
                                        </div>
                                        <div class="space-y-4">
                                            <div class="flex justify-between items-center p-3 bg-theme-secondary rounded">
                                                <span class="text-theme-secondary">"Searches with Clicks"</span>
                                                <span class="font-bold text-theme-primary">
                                                    {format!("{}", (summary.click_through_rate * summary.total_searches as f64) as u32)}
                                                </span>
                                            </div>
                                            <div class="flex justify-between items-center p-3 bg-theme-secondary rounded">
                                                <span class="text-theme-secondary">"Avg Results per Search"</span>
                                                <span class="font-bold text-theme-primary">{format!("{:.1}", summary.avg_results_per_search)}</span>
                                            </div>
                                        </div>
                                    </div>
                                }.into_any()
                            }).unwrap_or_else(|| {
                                view! {
                                    <div class="text-center py-4 text-theme-secondary">
                                        "No performance data available"
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

// ============================================================================
// Wrapper Page Component with Navigation
// ============================================================================

#[component]
pub fn SearchAnalyticsPage() -> impl IntoView {
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
                <SearchAnalyticsDashboard />
            </div>
        </div>
    }
}
