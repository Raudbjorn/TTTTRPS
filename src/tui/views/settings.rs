//! Settings view — displays LLM provider status, router config, costs, and app config.
//!
//! Read-only scrollable view. Data is loaded asynchronously from backend services
//! and cached for rendering. Press `r` to refresh.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tokio::sync::mpsc;

use crate::tui::services::Services;

// ── Display types ────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct ProviderDisplayInfo {
    name: String,
    model: String,
    is_healthy: bool,
    circuit_state: String,
    uptime: f64,
    total_requests: u64,
    success_rate: f64,
    avg_latency_ms: u64,
    total_cost: f64,
}

#[derive(Clone, Debug)]
struct SettingsData {
    providers: Vec<ProviderDisplayInfo>,
    routing_strategy: String,
    request_timeout_secs: u64,
    health_check_interval_secs: u64,
    max_retries: u32,
    enable_fallback: bool,
    stream_chunk_timeout_secs: u64,
    // Cost
    total_cost: f64,
    monthly_cost: f64,
    daily_cost: f64,
    monthly_budget: Option<f64>,
    daily_budget: Option<f64>,
    within_budget: bool,
    // Health overview
    total_providers: usize,
    healthy_count: usize,
    unhealthy_count: usize,
    avg_uptime: f64,
    // App config
    data_dir: String,
    tick_rate_ms: u64,
    mouse_enabled: bool,
    theme: String,
}

// ── State ────────────────────────────────────────────────────────────────────

pub struct SettingsState {
    data: Option<SettingsData>,
    lines_cache: Vec<Line<'static>>,
    scroll: usize,
    loading: bool,
    data_rx: mpsc::UnboundedReceiver<SettingsData>,
    data_tx: mpsc::UnboundedSender<SettingsData>,
}

impl SettingsState {
    pub fn new() -> Self {
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        Self {
            data: None,
            lines_cache: Vec::new(),
            scroll: 0,
            loading: false,
            data_rx,
            data_tx,
        }
    }

    /// Trigger async data load from backend services.
    pub fn load(&mut self, services: &Services) {
        if self.loading {
            return;
        }
        self.loading = true;

        let llm = services.llm.clone();
        let tx = self.data_tx.clone();

        // Sync data captured before spawn
        let provider_ids = llm.provider_ids();
        let strategy = format!("{:?}", llm.routing_strategy());
        let config = llm.config().clone();

        // App config (sync — re-reads from disk, cheap)
        let app_config = crate::config::AppConfig::load();
        let data_dir = app_config.data_dir().display().to_string();
        let tick_rate_ms = app_config.tui.tick_rate_ms;
        let mouse_enabled = app_config.tui.mouse_enabled;
        let theme = app_config.tui.theme.clone();

        tokio::spawn(async move {
            let all_stats = llm.get_all_stats().await;
            let all_health = llm.get_all_health().await;
            let cost_summary = llm.get_cost_summary().await;
            let health_summary = llm.get_health_summary().await;

            let mut providers = Vec::with_capacity(provider_ids.len());
            for id in &provider_ids {
                let provider = llm.get_provider(id);
                let stats = all_stats.get(id);
                let health = all_health.get(id);

                providers.push(ProviderDisplayInfo {
                    name: provider
                        .as_ref()
                        .map(|p| p.name().to_string())
                        .unwrap_or_else(|| id.clone()),
                    model: provider
                        .as_ref()
                        .map(|p| p.model().to_string())
                        .unwrap_or_default(),
                    is_healthy: health.map(|h| h.is_healthy).unwrap_or(false),
                    circuit_state: health
                        .map(|h| format!("{:?}", h.circuit_state))
                        .unwrap_or_else(|| "Unknown".to_string()),
                    uptime: health.map(|h| h.uptime_percentage).unwrap_or(0.0),
                    total_requests: stats.map(|s| s.total_requests).unwrap_or(0),
                    success_rate: stats.map(|s| s.success_rate()).unwrap_or(0.0),
                    avg_latency_ms: stats.map(|s| s.avg_latency_ms()).unwrap_or(0),
                    total_cost: stats.map(|s| s.total_cost_usd).unwrap_or(0.0),
                });
            }

            let data = SettingsData {
                providers,
                routing_strategy: strategy,
                request_timeout_secs: config.request_timeout.as_secs(),
                health_check_interval_secs: config.health_check_interval.as_secs(),
                max_retries: config.max_retries,
                enable_fallback: config.enable_fallback,
                stream_chunk_timeout_secs: config.stream_chunk_timeout.as_secs(),
                total_cost: cost_summary.total_cost_usd,
                monthly_cost: cost_summary.monthly_cost,
                daily_cost: cost_summary.daily_cost,
                monthly_budget: cost_summary.monthly_budget,
                daily_budget: cost_summary.daily_budget,
                within_budget: cost_summary.is_within_budget,
                total_providers: health_summary.total_providers,
                healthy_count: health_summary.healthy_providers,
                unhealthy_count: health_summary.unhealthy_providers,
                avg_uptime: health_summary.average_uptime_percentage,
                data_dir,
                tick_rate_ms,
                mouse_enabled,
                theme,
            };

            let _ = tx.send(data);
        });
    }

    /// Poll for async data completion. Call from on_tick.
    pub fn poll(&mut self) {
        if let Ok(data) = self.data_rx.try_recv() {
            self.lines_cache = build_lines(&data);
            self.data = Some(data);
            self.loading = false;
        }
    }

    // ── Input ────────────────────────────────────────────────────────────

    pub fn handle_input(&mut self, event: &Event, services: &Services) -> bool {
        let Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            ..
        }) = event
        else {
            return false;
        };

        match (*modifiers, *code) {
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                self.scroll_down(1);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                self.scroll_up(1);
                true
            }
            (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
                self.scroll = self.lines_cache.len().saturating_sub(1);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('g')) => {
                self.scroll = 0;
                true
            }
            (KeyModifiers::NONE, KeyCode::PageDown) => {
                self.scroll_down(15);
                true
            }
            (KeyModifiers::NONE, KeyCode::PageUp) => {
                self.scroll_up(15);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.load(services);
                true
            }
            _ => false,
        }
    }

    fn scroll_down(&mut self, n: usize) {
        self.scroll = self
            .scroll
            .saturating_add(n)
            .min(self.lines_cache.len().saturating_sub(1));
    }

    fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }

    // ── Rendering ────────────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Settings ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.loading && self.data.is_none() {
            let loading = Paragraph::new(vec![
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Loading settings...", Style::default().fg(Color::DarkGray)),
                ]),
            ]);
            frame.render_widget(loading, inner);
            return;
        }

        if self.lines_cache.is_empty() {
            let empty = Paragraph::new(vec![
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "No data loaded. Press r to refresh.",
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
            ]);
            frame.render_widget(empty, inner);
            return;
        }

        let content = Paragraph::new(self.lines_cache.clone())
            .scroll((self.scroll as u16, 0));
        frame.render_widget(content, inner);
    }
}

// ── Line builders ────────────────────────────────────────────────────────────

fn build_lines(data: &SettingsData) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(60);

    // ── LLM Providers ──
    lines.extend(section_header("LLM Providers"));

    if data.providers.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "No providers configured",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    } else {
        // Table header
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!(
                    "{:<14} {:<20} {:>6} {:>9} {:>6} {:>5} {:>7} {:>8}",
                    "Name", "Model", "Health", "Circuit", "Reqs", "OK%", "Latency", "Cost"
                ),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        for p in &data.providers {
            let health_icon = if p.is_healthy { "✓" } else { "✗" };
            let health_color = if p.is_healthy {
                Color::Green
            } else {
                Color::Red
            };

            let name_display = if p.name.len() > 12 {
                format!("{}...", &p.name[..9])
            } else {
                p.name.clone()
            };

            let model_display = if p.model.len() > 18 {
                format!("{}...", &p.model[..15])
            } else {
                p.model.clone()
            };

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{:<14}", name_display),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(format!("{:<20} ", model_display)),
                Span::styled(
                    format!("{:>4}", health_icon),
                    Style::default().fg(health_color),
                ),
                Span::raw(format!(
                    " {:>9} {:>6} {:>4.0}% {:>6}ms ${:>7.4}",
                    p.circuit_state,
                    p.total_requests,
                    p.success_rate * 100.0,
                    p.avg_latency_ms,
                    p.total_cost,
                )),
            ]));

            // Per-provider uptime on second line
            if p.uptime > 0.0 {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{:<14}", ""),
                        Style::default(),
                    ),
                    Span::styled(
                        format!("uptime: {:.1}%", p.uptime),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
        }
    }

    // ── Health Overview ──
    lines.extend(section_header("Health Overview"));
    lines.push(kv_row(
        "Total Providers",
        &data.total_providers.to_string(),
    ));
    lines.push(kv_row(
        "Healthy",
        &format!(
            "{}{}",
            data.healthy_count,
            if data.unhealthy_count > 0 {
                format!(" ({} unhealthy)", data.unhealthy_count)
            } else {
                String::new()
            }
        ),
    ));
    lines.push(kv_row(
        "Average Uptime",
        &format!("{:.1}%", data.avg_uptime),
    ));

    // ── Router Configuration ──
    lines.extend(section_header("Router Configuration"));
    lines.push(kv_row("Routing Strategy", &data.routing_strategy));
    lines.push(kv_row(
        "Request Timeout",
        &format!("{}s", data.request_timeout_secs),
    ));
    lines.push(kv_row(
        "Stream Timeout",
        &format!("{}s", data.stream_chunk_timeout_secs),
    ));
    lines.push(kv_row(
        "Health Check",
        &format!("every {}s", data.health_check_interval_secs),
    ));
    lines.push(kv_row("Max Retries", &data.max_retries.to_string()));
    lines.push(kv_row(
        "Fallback",
        if data.enable_fallback {
            "enabled"
        } else {
            "disabled"
        },
    ));

    // ── Cost Summary ──
    lines.extend(section_header("Cost Summary"));
    let budget_icon = if data.within_budget { "✓" } else { "✗" };
    let budget_color = if data.within_budget {
        Color::Green
    } else {
        Color::Red
    };
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{:<22}", "Budget Status"),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(
            format!(
                "{} {}",
                budget_icon,
                if data.within_budget {
                    "Within budget"
                } else {
                    "OVER BUDGET"
                }
            ),
            Style::default().fg(budget_color),
        ),
    ]));
    lines.push(kv_row(
        "Total Cost",
        &format!("${:.4}", data.total_cost),
    ));
    lines.push(kv_row(
        "Monthly Cost",
        &format!("${:.4}", data.monthly_cost),
    ));
    lines.push(kv_row(
        "Daily Cost",
        &format!("${:.4}", data.daily_cost),
    ));
    lines.push(kv_row(
        "Monthly Budget",
        &data
            .monthly_budget
            .map(|b| format!("${:.2}", b))
            .unwrap_or_else(|| "none".to_string()),
    ));
    lines.push(kv_row(
        "Daily Budget",
        &data
            .daily_budget
            .map(|b| format!("${:.2}", b))
            .unwrap_or_else(|| "none".to_string()),
    ));

    // ── Application ──
    lines.extend(section_header("Application"));
    lines.push(kv_row(
        "Config File",
        &dirs::config_dir()
            .map(|d| d.join("ttttrps/config.toml").display().to_string())
            .unwrap_or_else(|| "config.toml".to_string()),
    ));
    lines.push(kv_row("Data Directory", &data.data_dir));
    lines.push(kv_row(
        "Tick Rate",
        &format!("{}ms", data.tick_rate_ms),
    ));
    lines.push(kv_row(
        "Mouse",
        if data.mouse_enabled {
            "enabled"
        } else {
            "disabled"
        },
    ));
    lines.push(kv_row("Theme", &data.theme));

    // ── Footer ──
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("j/k", Style::default().fg(Color::DarkGray)),
        Span::raw(":scroll "),
        Span::styled("G/g", Style::default().fg(Color::DarkGray)),
        Span::raw(":bottom/top "),
        Span::styled("r", Style::default().fg(Color::DarkGray)),
        Span::raw(":refresh"),
    ]));
    lines.push(Line::raw(""));

    lines
}

fn section_header(title: &str) -> Vec<Line<'static>> {
    vec![
        Line::raw(""),
        Line::from(Span::styled(
            format!("  {title}"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("  {}", "─".repeat(50)),
            Style::default().fg(Color::DarkGray),
        )),
    ]
}

fn kv_row(key: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{:<22}", key),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(value.to_string()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_state_new() {
        let state = SettingsState::new();
        assert!(state.data.is_none());
        assert!(state.lines_cache.is_empty());
        assert_eq!(state.scroll, 0);
        assert!(!state.loading);
    }

    #[test]
    fn test_build_lines_no_providers() {
        let data = SettingsData {
            providers: vec![],
            routing_strategy: "Priority".to_string(),
            request_timeout_secs: 120,
            health_check_interval_secs: 60,
            max_retries: 1,
            enable_fallback: true,
            stream_chunk_timeout_secs: 30,
            total_cost: 0.0,
            monthly_cost: 0.0,
            daily_cost: 0.0,
            monthly_budget: None,
            daily_budget: None,
            within_budget: true,
            total_providers: 0,
            healthy_count: 0,
            unhealthy_count: 0,
            avg_uptime: 0.0,
            data_dir: "/tmp/test".to_string(),
            tick_rate_ms: 50,
            mouse_enabled: false,
            theme: "default".to_string(),
        };
        let lines = build_lines(&data);
        assert!(!lines.is_empty());
        // Should contain section headers
        let text: String = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("LLM Providers"));
        assert!(text.contains("No providers configured"));
        assert!(text.contains("Router Configuration"));
        assert!(text.contains("Cost Summary"));
        assert!(text.contains("Application"));
    }

    #[test]
    fn test_build_lines_with_providers() {
        let data = SettingsData {
            providers: vec![
                ProviderDisplayInfo {
                    name: "Claude".to_string(),
                    model: "claude-3-5-sonnet".to_string(),
                    is_healthy: true,
                    circuit_state: "Closed".to_string(),
                    uptime: 99.5,
                    total_requests: 42,
                    success_rate: 0.95,
                    avg_latency_ms: 1200,
                    total_cost: 0.0123,
                },
                ProviderDisplayInfo {
                    name: "Ollama".to_string(),
                    model: "llama3".to_string(),
                    is_healthy: false,
                    circuit_state: "Open".to_string(),
                    uptime: 50.0,
                    total_requests: 10,
                    success_rate: 0.5,
                    avg_latency_ms: 500,
                    total_cost: 0.0,
                },
            ],
            routing_strategy: "Priority".to_string(),
            request_timeout_secs: 120,
            health_check_interval_secs: 60,
            max_retries: 1,
            enable_fallback: true,
            stream_chunk_timeout_secs: 30,
            total_cost: 0.0123,
            monthly_cost: 0.0123,
            daily_cost: 0.005,
            monthly_budget: Some(10.0),
            daily_budget: None,
            within_budget: true,
            total_providers: 2,
            healthy_count: 1,
            unhealthy_count: 1,
            avg_uptime: 74.75,
            data_dir: "/tmp/test".to_string(),
            tick_rate_ms: 50,
            mouse_enabled: false,
            theme: "default".to_string(),
        };
        let lines = build_lines(&data);
        let text: String = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("Claude"));
        assert!(text.contains("Ollama"));
        assert!(text.contains("Priority"));
        assert!(text.contains("$10.00"));
    }

    #[test]
    fn test_scroll_bounds() {
        let mut state = SettingsState::new();
        // Empty lines — scroll should stay at 0
        state.scroll_down(10);
        assert_eq!(state.scroll, 0);
        state.scroll_up(10);
        assert_eq!(state.scroll, 0);

        // Simulate some cached lines
        state.lines_cache = vec![Line::raw(""); 30];
        state.scroll_down(5);
        assert_eq!(state.scroll, 5);
        state.scroll_down(100);
        assert_eq!(state.scroll, 29); // clamped to len-1
        state.scroll_up(100);
        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn test_section_header() {
        let lines = section_header("Test Section");
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_kv_row() {
        let line = kv_row("Key", "Value");
        let text: String = line
            .spans
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert!(text.contains("Key"));
        assert!(text.contains("Value"));
    }
}
