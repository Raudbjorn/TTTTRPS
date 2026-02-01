//! Audit Log Viewer Component
//!
//! Displays security audit logs with filtering, search, and export functionality.

use leptos::prelude::*;
use leptos::ev;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use std::collections::HashMap;

use crate::bindings::{
    query_audit_logs, export_audit_logs, clear_old_logs,
    get_audit_summary, get_security_events, SecurityAuditEvent,
};
use crate::components::design_system::{
    Badge, BadgeVariant, Button, ButtonVariant, Card, CardBody, CardHeader, Input, Select,
};

// ============================================================================
// Audit Log Viewer Component
// ============================================================================

#[component]
pub fn AuditLogViewer() -> impl IntoView {
    // State signals
    let audit_events = RwSignal::new(Vec::<SecurityAuditEvent>::new());
    let audit_summary = RwSignal::new(HashMap::<String, usize>::new());
    let security_events = RwSignal::new(Vec::<SecurityAuditEvent>::new());
    let is_loading = RwSignal::new(false);
    let error_message = RwSignal::new(Option::<String>::None);
    let success_message = RwSignal::new(Option::<String>::None);

    // Filter state
    let selected_severity = RwSignal::new("all".to_string());
    let selected_time_range = RwSignal::new("24".to_string()); // hours
    let search_text = RwSignal::new(String::new());
    let selected_event_type = RwSignal::new("all".to_string());

    // Export state
    let is_exporting = RwSignal::new(false);
    let export_format = RwSignal::new("json".to_string());

    // Cleanup state
    let cleanup_days = RwSignal::new("30".to_string());
    let is_cleaning = RwSignal::new(false);

    // View mode
    let show_security_only = RwSignal::new(false);

    // Load data function
    let load_data = move || {
        spawn_local(async move {
            is_loading.set(true);
            error_message.set(None);
            success_message.set(None);

            // Build query parameters
            let hours: Option<i64> = selected_time_range.get().parse().ok();
            let severity = if selected_severity.get() == "all" {
                None
            } else {
                Some(selected_severity.get())
            };
            let event_types = if selected_event_type.get() == "all" {
                None
            } else {
                Some(vec![selected_event_type.get()])
            };
            let search = if search_text.get().is_empty() {
                None
            } else {
                Some(search_text.get())
            };

            // Fetch audit logs
            match query_audit_logs(hours, severity, event_types, search, Some(500)).await {
                Ok(events) => audit_events.set(events),
                Err(e) => error_message.set(Some(format!("Failed to load audit logs: {}", e))),
            }

            // Fetch summary
            match get_audit_summary().await {
                Ok(summary) => audit_summary.set(summary),
                Err(_) => {} // Non-critical
            }

            // Fetch security events
            match get_security_events().await {
                Ok(events) => security_events.set(events),
                Err(_) => {} // Non-critical
            }

            is_loading.set(false);
        });
    };

    // Load data on mount and when filters change
    Effect::new(move |_| {
        let _ = selected_severity.get();
        let _ = selected_time_range.get();
        let _ = selected_event_type.get();
        load_data();
    });

    // Handle search callback - triggers data reload when search text changes
    let handle_search = Callback::new(move |_: String| {
        load_data();
    });

    // Handle export
    let handle_export = move |_| {
        is_exporting.set(true);
        let format = export_format.get();
        let hours: Option<i64> = selected_time_range.get().parse().ok();

        spawn_local(async move {
            match export_audit_logs(format.clone(), hours).await {
                Ok(content) => {
                    // Trigger download in browser
                    if let Some(window) = web_sys::window() {
                        if let Some(document) = window.document() {
                            let extension = match format.as_str() {
                                "csv" => "csv",
                                "jsonl" => "jsonl",
                                _ => "json",
                            };
                            let filename = format!("audit_logs_{}.{}",
                                chrono::Utc::now().format("%Y%m%d_%H%M%S"), extension);

                            // Create blob and download
                            let mime = match format.as_str() {
                                "csv" => "text/csv",
                                _ => "application/json",
                            };

                            let blob_parts = js_sys::Array::of1(&JsValue::from_str(&content));
                            let options = web_sys::BlobPropertyBag::new();
                            options.set_type(mime);
                            if let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(
                                &blob_parts,
                                &options,
                            ) {
                                if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                                    if let Ok(a) = document.create_element("a") {
                                        let _ = a.set_attribute("href", &url);
                                        let _ = a.set_attribute("download", &filename);
                                        a.set_text_content(Some(""));
                                        if let Some(body) = document.body() {
                                            let _ = body.append_child(&a);
                                            if let Some(html_a) = a.dyn_ref::<web_sys::HtmlElement>() {
                                                html_a.click();
                                            }
                                            let _ = body.remove_child(&a);
                                        }
                                        let _ = web_sys::Url::revoke_object_url(&url);
                                    }
                                }
                            }
                            success_message.set(Some(format!("Exported {} logs", audit_events.get().len())));
                        }
                    }
                }
                Err(e) => error_message.set(Some(format!("Export failed: {}", e))),
            }
            is_exporting.set(false);
        });
    };

    // Handle cleanup
    let handle_cleanup = move |_| {
        is_cleaning.set(true);
        let days: i64 = cleanup_days.get().parse().unwrap_or(30);

        spawn_local(async move {
            match clear_old_logs(days).await {
                Ok(count) => {
                    success_message.set(Some(format!("Cleaned up {} old log entries", count)));
                    load_data();
                }
                Err(e) => error_message.set(Some(format!("Cleanup failed: {}", e))),
            }
            is_cleaning.set(false);
        });
    };

    // Helper to get severity badge variant
    let get_severity_variant = |severity: &str| -> BadgeVariant {
        match severity.to_lowercase().as_str() {
            "critical" => BadgeVariant::Danger,
            "security" => BadgeVariant::Warning,
            "warning" => BadgeVariant::Warning,
            "info" => BadgeVariant::Info,
            "debug" => BadgeVariant::Default,
            _ => BadgeVariant::Default,
        }
    };

    // Helper to format event type for display
    let format_event_type = |event_type: &serde_json::Value| -> String {
        if let Some(obj) = event_type.as_object() {
            obj.keys().next().cloned().unwrap_or_else(|| "Unknown".to_string())
        } else if let Some(s) = event_type.as_str() {
            s.to_string()
        } else {
            format!("{:?}", event_type)
        }
    };

    // Helper to get event description
    let get_event_description = |event: &SecurityAuditEvent| -> String {
        if let Some(obj) = event.event_type.as_object() {
            // Build a human-readable description from the event data
            let event_name = obj.keys().next().cloned().unwrap_or_default();
            let details = obj.values().next();

            if let Some(detail_val) = details {
                if let Some(detail_obj) = detail_val.as_object() {
                    let parts: Vec<String> = detail_obj.iter()
                        .take(3)
                        .map(|(k, v)| {
                            let val_str = if let Some(s) = v.as_str() {
                                s.to_string()
                            } else {
                                format!("{}", v)
                            };
                            format!("{}: {}", k, val_str)
                        })
                        .collect();
                    return format!("{} - {}", event_name, parts.join(", "));
                }
            }
            event_name
        } else {
            format!("{:?}", event.event_type)
        }
    };

    view! {
        <div class="space-y-6">
            // Header
            <div class="flex items-center justify-between">
                <h2 class="text-xl font-bold text-theme-primary">"Security Audit Logs"</h2>
                <div class="flex items-center gap-2">
                    // Toggle security-only view
                    <Button
                        variant=if show_security_only.get() { ButtonVariant::Primary } else { ButtonVariant::Secondary }
                        on_click=move |_| show_security_only.update(|v| *v = !*v)
                    >
                        {if show_security_only.get() { "Show All" } else { "Security Only" }}
                    </Button>
                </div>
            </div>

            // Summary Cards
            <div class="grid grid-cols-2 md:grid-cols-5 gap-4">
                {move || {
                    let summary = audit_summary.get();
                    vec![
                        ("debug", "Debug", "text-gray-400"),
                        ("info", "Info", "text-blue-400"),
                        ("warning", "Warning", "text-yellow-400"),
                        ("security", "Security", "text-orange-400"),
                        ("critical", "Critical", "text-red-400"),
                    ].into_iter().map(|(key, label, color)| {
                        let count = summary.get(key).copied().unwrap_or(0);
                        view! {
                            <Card>
                                <CardBody>
                                    <div class="text-center">
                                        <div class=format!("text-2xl font-bold {}", color)>
                                            {count.to_string()}
                                        </div>
                                        <div class="text-xs text-theme-secondary">{label}</div>
                                    </div>
                                </CardBody>
                            </Card>
                        }
                    }).collect_view()
                }}
            </div>

            // Messages
            <Show when=move || error_message.get().is_some()>
                <div class="p-4 bg-red-900/20 border border-red-500/50 rounded text-red-400">
                    {move || error_message.get().unwrap_or_default()}
                </div>
            </Show>

            <Show when=move || success_message.get().is_some()>
                <div class="p-4 bg-green-900/20 border border-green-500/50 rounded text-green-400">
                    {move || success_message.get().unwrap_or_default()}
                </div>
            </Show>

            // Filters
            <Card>
                <CardBody>
                    <div class="grid grid-cols-1 md:grid-cols-5 gap-4">
                        // Time range
                        <div>
                            <label class="block text-xs text-theme-secondary mb-1">"Time Range"</label>
                            <Select
                                value=selected_time_range.get()
                                on_change=Callback::new(move |val: String| selected_time_range.set(val))
                            >
                                <option value="1">"Last Hour"</option>
                                <option value="24">"Last 24 Hours"</option>
                                <option value="168">"Last Week"</option>
                                <option value="720">"Last 30 Days"</option>
                                <option value="2160">"Last 90 Days"</option>
                            </Select>
                        </div>

                        // Severity
                        <div>
                            <label class="block text-xs text-theme-secondary mb-1">"Min Severity"</label>
                            <Select
                                value=selected_severity.get()
                                on_change=Callback::new(move |val: String| selected_severity.set(val))
                            >
                                <option value="all">"All"</option>
                                <option value="debug">"Debug+"</option>
                                <option value="info">"Info+"</option>
                                <option value="warning">"Warning+"</option>
                                <option value="security">"Security+"</option>
                                <option value="critical">"Critical Only"</option>
                            </Select>
                        </div>

                        // Event type
                        <div>
                            <label class="block text-xs text-theme-secondary mb-1">"Event Type"</label>
                            <Select
                                value=selected_event_type.get()
                                on_change=Callback::new(move |val: String| selected_event_type.set(val))
                            >
                                <option value="all">"All Events"</option>
                                <option value="ApiKey">"API Key"</option>
                                <option value="Document">"Document"</option>
                                <option value="Campaign">"Campaign"</option>
                                <option value="Session">"Session"</option>
                                <option value="Llm">"LLM"</option>
                                <option value="Setting">"Settings"</option>
                                <option value="Validation">"Validation"</option>
                                <option value="Application">"Application"</option>
                            </Select>
                        </div>

                        // Search
                        <div class="md:col-span-2">
                            <label class="block text-xs text-theme-secondary mb-1">"Search"</label>
                            <Input
                                value=search_text
                                placeholder="Search logs..."
                                on_input=handle_search
                            />
                        </div>
                    </div>
                </CardBody>
            </Card>

            // Actions Bar
            <div class="flex flex-wrap items-center gap-4">
                // Export
                <div class="flex items-center gap-2">
                    <Select
                        value=export_format.get()
                        on_change=Callback::new(move |val: String| export_format.set(val))
                    >
                        <option value="json">"JSON"</option>
                        <option value="csv">"CSV"</option>
                        <option value="jsonl">"JSON Lines"</option>
                    </Select>
                    <Button
                        variant=ButtonVariant::Secondary
                        loading=is_exporting
                        on_click=handle_export
                    >
                        "Export"
                    </Button>
                </div>

                // Cleanup
                <div class="flex items-center gap-2 ml-auto">
                    <span class="text-sm text-theme-secondary">"Clear logs older than"</span>
                    <select
                        class="px-2 py-1 rounded bg-gray-700 text-white border border-gray-600 text-sm"
                        prop:value=move || cleanup_days.get()
                        on:change=move |e| cleanup_days.set(event_target_value(&e))
                    >
                        <option value="7">"7 days"</option>
                        <option value="30">"30 days"</option>
                        <option value="90">"90 days"</option>
                    </select>
                    <Button
                        variant=ButtonVariant::Destructive
                        loading=is_cleaning
                        on_click=handle_cleanup
                    >
                        "Clear Old Logs"
                    </Button>
                </div>
            </div>

            // Loading state
            <Show when=move || is_loading.get()>
                <div class="text-center py-8 text-theme-secondary">
                    "Loading audit logs..."
                </div>
            </Show>

            // Security Events Section (24h)
            <Show when=move || show_security_only.get() && !security_events.get().is_empty()>
                <Card>
                    <CardHeader>
                        <div class="flex items-center gap-2">
                            <span class="text-lg font-semibold">"Security Events (Last 24h)"</span>
                            <Badge variant=BadgeVariant::Warning>
                                {move || security_events.get().len().to_string()}
                            </Badge>
                        </div>
                    </CardHeader>
                    <CardBody>
                        <div class="space-y-2 max-h-96 overflow-y-auto">
                            {move || {
                                security_events.get().iter().map(|event| {
                                    let severity = event.severity.clone();
                                    let timestamp = event.timestamp.clone();
                                    let description = get_event_description(event);
                                    let event_type = format_event_type(&event.event_type);
                                    let variant = get_severity_variant(&severity);

                                    view! {
                                        <div class="p-3 bg-theme-secondary rounded flex items-start gap-3">
                                            <Badge variant=variant>
                                                {severity.clone()}
                                            </Badge>
                                            <div class="flex-1 min-w-0">
                                                <div class="font-medium text-theme-primary truncate">
                                                    {event_type}
                                                </div>
                                                <div class="text-sm text-theme-secondary truncate">
                                                    {description}
                                                </div>
                                            </div>
                                            <div class="text-xs text-theme-secondary whitespace-nowrap">
                                                {format_timestamp(&timestamp)}
                                            </div>
                                        </div>
                                    }
                                }).collect_view()
                            }}
                        </div>
                    </CardBody>
                </Card>
            </Show>

            // Log Table
            <Show when=move || !is_loading.get()>
                <Card>
                    <CardHeader>
                        <div class="flex items-center justify-between w-full">
                            <h3 class="text-lg font-semibold">"Audit Log Entries"</h3>
                            <span class="text-sm text-theme-secondary">
                                {move || format!("{} entries", audit_events.get().len())}
                            </span>
                        </div>
                    </CardHeader>
                    <CardBody>
                        {move || {
                            let events = audit_events.get();
                            if events.is_empty() {
                                view! {
                                    <div class="text-center py-8 text-theme-secondary">
                                        "No audit log entries found"
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="overflow-x-auto">
                                        <table class="w-full text-sm">
                                            <thead>
                                                <tr class="border-b border-gray-700">
                                                    <th class="text-left py-2 px-2 text-theme-secondary">"Timestamp"</th>
                                                    <th class="text-left py-2 px-2 text-theme-secondary">"Severity"</th>
                                                    <th class="text-left py-2 px-2 text-theme-secondary">"Event Type"</th>
                                                    <th class="text-left py-2 px-2 text-theme-secondary">"Description"</th>
                                                    <th class="text-left py-2 px-2 text-theme-secondary">"Context"</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {events.iter().map(|event| {
                                                    let severity = event.severity.clone();
                                                    let timestamp = event.timestamp.clone();
                                                    let event_type = format_event_type(&event.event_type);
                                                    let description = get_event_description(event);
                                                    let description_title = description.clone();
                                                    let context = event.context.clone().unwrap_or_default();
                                                    let context_title = context.clone();
                                                    let variant = get_severity_variant(&severity);

                                                    view! {
                                                        <tr class="border-b border-gray-800 hover:bg-theme-secondary/50">
                                                            <td class="py-2 px-2 whitespace-nowrap text-xs">
                                                                {format_timestamp(&timestamp)}
                                                            </td>
                                                            <td class="py-2 px-2">
                                                                <Badge variant=variant>
                                                                    {severity.clone()}
                                                                </Badge>
                                                            </td>
                                                            <td class="py-2 px-2 font-mono text-xs">
                                                                {event_type}
                                                            </td>
                                                            <td class="py-2 px-2 max-w-md truncate" title=description_title>
                                                                {description}
                                                            </td>
                                                            <td class="py-2 px-2 text-xs text-theme-secondary max-w-xs truncate" title=context_title>
                                                                {context}
                                                            </td>
                                                        </tr>
                                                    }
                                                }).collect_view()}
                                            </tbody>
                                        </table>
                                    </div>
                                }.into_any()
                            }
                        }}
                    </CardBody>
                </Card>
            </Show>
        </div>
    }
}

// Helper function to format timestamp for display
fn format_timestamp(timestamp: &str) -> String {
    // Parse ISO 8601 timestamp and format for display
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        // Try parsing without timezone
        timestamp.chars().take(19).collect()
    }
}

// ============================================================================
// Wrapper Page Component with Navigation
// ============================================================================

#[component]
pub fn AuditLogsPage() -> impl IntoView {
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
                <AuditLogViewer />
            </div>
        </div>
    }
}
