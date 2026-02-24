//! System view — security vault, budget tracking, and audit alerts.
//!
//! Three tabs accessible via Tab key:
//! - **Vault**: API key provider status with masked key previews
//! - **Budget**: Spend tracking with per-provider cost breakdown
//! - **Alerts**: Recent audit events with severity filtering
//!
//! This view has no Focus variant — it operates as a standalone module
//! reachable from the command palette or embeddable as a sub-view.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tokio::sync::mpsc;

use super::super::theme;
use crate::core::audit::{AuditLogger, AuditSeverity};
use crate::core::credentials::mask_api_key;
use crate::core::llm::providers::PROVIDERS;
use crate::tui::services::Services;

// ── Tab enum ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SystemTab {
    Vault,
    Budget,
    Alerts,
}

impl SystemTab {
    fn label(self) -> &'static str {
        match self {
            Self::Vault => "Vault/Credentials",
            Self::Budget => "Budget",
            Self::Alerts => "Alerts",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Vault => Self::Budget,
            Self::Budget => Self::Alerts,
            Self::Alerts => Self::Vault,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Vault => Self::Alerts,
            Self::Budget => Self::Vault,
            Self::Alerts => Self::Budget,
        }
    }

    const ALL: [Self; 3] = [Self::Vault, Self::Budget, Self::Alerts];
}

// ── Vault data ──────────────────────────────────────────────────────────────

/// Credential status for a single provider.
struct CredentialRow {
    provider_id: String,
    display_name: String,
    status: CredentialStatus,
    masked_key: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CredentialStatus {
    Configured,
    Missing,
}

// ── Budget data ─────────────────────────────────────────────────────────────

struct BudgetData {
    total_cost: f64,
    monthly_cost: f64,
    daily_cost: f64,
    monthly_budget: Option<f64>,
    within_budget: bool,
    providers: Vec<ProviderCostRow>,
}

struct ProviderCostRow {
    name: String,
    cost: f64,
    requests: u64,
    input_tokens: u64,
    output_tokens: u64,
}

// ── State ───────────────────────────────────────────────────────────────────

pub struct SystemViewState {
    tab: SystemTab,
    scroll: usize,
    selected: usize,

    // Vault
    credentials: Vec<CredentialRow>,

    // Budget (async loaded)
    budget_data: Option<BudgetData>,
    budget_loading: bool,
    budget_rx: mpsc::UnboundedReceiver<BudgetData>,
    budget_tx: mpsc::UnboundedSender<BudgetData>,

    // Alerts
    audit_logger: AuditLogger,
    severity_filter: Option<AuditSeverity>,
    alert_count: usize,
}

impl SystemViewState {
    pub fn new() -> Self {
        let (budget_tx, budget_rx) = mpsc::unbounded_channel();
        Self {
            tab: SystemTab::Vault,
            scroll: 0,
            selected: 0,

            credentials: Vec::new(),

            budget_data: None,
            budget_loading: false,
            budget_rx,
            budget_tx,

            audit_logger: AuditLogger::new(),
            severity_filter: None,
            alert_count: 0,
        }
    }

    pub fn load(&mut self, services: &Services) {
        self.load_credentials(services);
        self.load_budget(services);
        self.refresh_alert_count();
    }

    pub fn poll(&mut self) {
        if let Ok(data) = self.budget_rx.try_recv() {
            self.budget_data = Some(data);
            self.budget_loading = false;
        }
    }

    // ── Credential loading ──────────────────────────────────────────────

    fn load_credentials(&mut self, services: &Services) {
        // LLM providers from the canonical PROVIDERS table
        let mut rows: Vec<CredentialRow> = PROVIDERS
            .iter()
            .map(|meta| {
                let secret = services.credentials.get_provider_secret(meta.id);
                let (status, masked) = match secret {
                    Ok(ref key) if !key.is_empty() => {
                        (CredentialStatus::Configured, Some(mask_api_key(key)))
                    }
                    _ => (CredentialStatus::Missing, None),
                };
                CredentialRow {
                    provider_id: meta.id.to_string(),
                    display_name: meta.display_name.to_string(),
                    status,
                    masked_key: masked,
                }
            })
            .collect();

        // Voice provider: ElevenLabs
        let elevenlabs_status = match services.credentials.get_voice_credential("elevenlabs") {
            Ok(cred) if !cred.api_key.is_empty() => {
                let masked = mask_api_key(&cred.api_key);
                (CredentialStatus::Configured, Some(masked))
            }
            _ => (CredentialStatus::Missing, None),
        };
        rows.push(CredentialRow {
            provider_id: "elevenlabs".to_string(),
            display_name: "ElevenLabs (Voice)".to_string(),
            status: elevenlabs_status.0,
            masked_key: elevenlabs_status.1,
        });

        self.credentials = rows;
    }

    // ── Budget loading (async) ──────────────────────────────────────────

    fn load_budget(&mut self, services: &Services) {
        self.budget_loading = true;
        let tx = self.budget_tx.clone();
        let llm = services.llm.clone();

        tokio::spawn(async move {
            let cost_summary = llm.get_cost_summary().await;
            let all_stats = llm.get_all_stats().await;

            let mut providers: Vec<ProviderCostRow> = all_stats
                .iter()
                .map(|(name, stats)| ProviderCostRow {
                    name: name.clone(),
                    cost: stats.total_cost_usd,
                    requests: stats.total_requests,
                    input_tokens: stats.total_input_tokens,
                    output_tokens: stats.total_output_tokens,
                })
                .collect();
            providers.sort_by(|a, b| {
                b.cost
                    .partial_cmp(&a.cost)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let data = BudgetData {
                total_cost: cost_summary.total_cost_usd,
                monthly_cost: cost_summary.monthly_cost,
                daily_cost: cost_summary.daily_cost,
                monthly_budget: cost_summary.monthly_budget,
                within_budget: cost_summary.is_within_budget,
                providers,
            };

            let _ = tx.send(data);
        });
    }

    // ── Alert helpers ───────────────────────────────────────────────────

    fn refresh_alert_count(&mut self) {
        self.alert_count = self.audit_logger.count();
    }

    // ── Input handling ──────────────────────────────────────────────────

    pub fn handle_input(&mut self, event: &Event, services: &Services) -> bool {
        let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            modifiers,
            ..
        }) = event
        else {
            return false;
        };

        match (*modifiers, *code) {
            // Tab switching
            (KeyModifiers::NONE, KeyCode::Tab) => {
                self.tab = self.tab.next();
                self.scroll = 0;
                self.selected = 0;
                true
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                self.tab = self.tab.prev();
                self.scroll = 0;
                self.selected = 0;
                true
            }

            // Navigation
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                let max = self.max_items();
                if max > 0 {
                    self.selected = (self.selected + 1).min(max.saturating_sub(1));
                    self.ensure_visible();
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                self.selected = self.selected.saturating_sub(1);
                self.ensure_visible();
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('g')) => {
                self.selected = 0;
                self.scroll = 0;
                true
            }
            (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
                let max = self.max_items();
                if max > 0 {
                    self.selected = max - 1;
                    self.ensure_visible();
                }
                true
            }

            // Refresh
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.load(services);
                true
            }

            // Alerts: severity filter cycling
            (KeyModifiers::NONE, KeyCode::Char('f')) if self.tab == SystemTab::Alerts => {
                self.severity_filter = match self.severity_filter {
                    None => Some(AuditSeverity::Info),
                    Some(AuditSeverity::Info) => Some(AuditSeverity::Warning),
                    Some(AuditSeverity::Warning) => Some(AuditSeverity::Security),
                    Some(AuditSeverity::Security) => Some(AuditSeverity::Critical),
                    Some(AuditSeverity::Critical) => None,
                };
                self.selected = 0;
                self.scroll = 0;
                self.refresh_alert_count();
                true
            }

            _ => false,
        }
    }

    fn max_items(&self) -> usize {
        match self.tab {
            SystemTab::Vault => self.credentials.len(),
            SystemTab::Budget => {
                self.budget_data
                    .as_ref()
                    .map(|d| d.providers.len())
                    .unwrap_or(0)
            }
            SystemTab::Alerts => self.alert_count,
        }
    }

    fn ensure_visible(&mut self) {
        if self.selected < self.scroll {
            self.scroll = self.selected;
        }
    }

    // ── Rendering ───────────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        self.render_tab_bar(frame, chunks[0]);

        match self.tab {
            SystemTab::Vault => self.render_vault(frame, chunks[1]),
            SystemTab::Budget => self.render_budget(frame, chunks[1]),
            SystemTab::Alerts => self.render_alerts(frame, chunks[1]),
        }
    }

    fn render_tab_bar(&self, frame: &mut Frame, area: Rect) {
        let spans: Vec<Span> = SystemTab::ALL
            .iter()
            .flat_map(|t| {
                let style = if *t == self.tab {
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT_MUTED)
                };
                vec![
                    Span::styled(format!(" {} ", t.label()), style),
                    Span::raw("\u{2502}"),
                ]
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_DIM));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(Paragraph::new(Line::from(spans)), inner);
    }

    // ── Vault tab ───────────────────────────────────────────────────────

    fn render_vault(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_focused("Vault / Credentials");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.credentials.is_empty() {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " Press 'r' to load credentials",
                    Style::default().fg(theme::TEXT_MUTED),
                ))),
                inner,
            );
            return;
        }

        let mut lines: Vec<Line<'static>> = Vec::new();

        // Header
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{:<24} {:>12} {}", "Provider", "Status", "  Key Preview"),
                Style::default()
                    .fg(theme::TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            format!("  {}", "\u{2500}".repeat(60)),
            Style::default().fg(theme::TEXT_DIM),
        )));

        for (i, row) in self.credentials.iter().enumerate() {
            let is_selected = i == self.selected;
            let marker = if is_selected { "\u{25b8} " } else { "  " };

            let (status_label, status_color) = match row.status {
                CredentialStatus::Configured => ("configured", theme::SUCCESS),
                CredentialStatus::Missing => ("missing", theme::WARNING),
            };

            let key_preview = row
                .masked_key
                .as_deref()
                .unwrap_or("\u{2014}")
                .to_string();

            let row_style = if is_selected {
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };

            lines.push(Line::from(vec![
                Span::styled(marker, row_style),
                Span::styled(format!("{:<24}", row.display_name), row_style),
                Span::styled(
                    format!("{:>12}", status_label),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(key_preview, Style::default().fg(theme::TEXT_DIM)),
            ]));
        }

        let configured = self
            .credentials
            .iter()
            .filter(|c| c.status == CredentialStatus::Configured)
            .count();
        let total = self.credentials.len();

        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{configured}/{total} providers configured"),
                Style::default().fg(theme::TEXT_MUTED),
            ),
        ]));

        // Hints
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  [Tab] switch tab  [r] refresh  [j/k] navigate",
            Style::default().fg(theme::TEXT_DIM),
        )));

        let visible = inner.height as usize;
        let max_scroll = lines.len().saturating_sub(visible);
        let scroll = self.scroll.min(max_scroll);

        frame.render_widget(Paragraph::new(lines).scroll((scroll as u16, 0)), inner);
    }

    // ── Budget tab ──────────────────────────────────────────────────────

    fn render_budget(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_focused("Budget & Costs");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.budget_loading && self.budget_data.is_none() {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " Loading...",
                    Style::default().fg(theme::TEXT_MUTED),
                ))),
                inner,
            );
            return;
        }

        let Some(ref data) = self.budget_data else {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " Press 'r' to load budget data",
                    Style::default().fg(theme::TEXT_MUTED),
                ))),
                inner,
            );
            return;
        };

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));

        // Cost overview
        lines.push(Line::from(Span::styled(
            "  SPEND OVERVIEW",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::raw(""));

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Total Spend: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("${:.4}", data.total_cost),
                Style::default()
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Monthly:     ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("${:.4}", data.monthly_cost),
                Style::default().fg(theme::TEXT),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Daily:       ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("${:.4}", data.daily_cost),
                Style::default().fg(theme::TEXT),
            ),
        ]));

        lines.push(Line::raw(""));

        // Budget bar
        if let Some(budget) = data.monthly_budget {
            let remaining = (budget - data.monthly_cost).max(0.0);
            let pct = if budget > 0.0 {
                (data.monthly_cost / budget * 100.0).min(999.9)
            } else {
                0.0
            };
            let color = if pct >= 95.0 {
                theme::ERROR
            } else if pct >= 80.0 {
                theme::WARNING
            } else {
                theme::SUCCESS
            };

            lines.push(Line::from(Span::styled(
                "  BUDGET STATUS",
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::raw(""));

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Limit:     ", Style::default().fg(theme::TEXT_MUTED)),
                Span::styled(
                    format!("${:.2}", budget),
                    Style::default().fg(theme::TEXT),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Remaining: ", Style::default().fg(theme::TEXT_MUTED)),
                Span::styled(
                    format!("${:.2}", remaining),
                    Style::default().fg(color),
                ),
            ]));

            // Progress bar
            let bar_width = 30usize;
            let filled = ((pct / 100.0) * bar_width as f64).min(bar_width as f64) as usize;
            let empty = bar_width.saturating_sub(filled);
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{:.1}% ", pct),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled("\u{2588}".repeat(filled), Style::default().fg(color)),
                Span::styled(
                    "\u{2591}".repeat(empty),
                    Style::default().fg(theme::TEXT_DIM),
                ),
            ]));

            let status_color = if data.within_budget {
                theme::SUCCESS
            } else {
                theme::ERROR
            };
            let status_text = if data.within_budget {
                "Within budget"
            } else {
                "Over budget!"
            };
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Status:    ", Style::default().fg(theme::TEXT_MUTED)),
                Span::styled(
                    status_text,
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "No monthly budget configured",
                    Style::default().fg(theme::TEXT_DIM),
                ),
            ]));
        }

        lines.push(Line::raw(""));

        // Per-provider breakdown
        lines.push(Line::from(Span::styled(
            "  PER-PROVIDER COSTS",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::raw(""));

        if data.providers.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No provider cost data yet",
                Style::default().fg(theme::TEXT_DIM),
            )));
        } else {
            // Table header
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!(
                        "{:<18} {:>10} {:>8} {:>10} {:>10}",
                        "Provider", "Cost", "Reqs", "In Tok", "Out Tok"
                    ),
                    Style::default()
                        .fg(theme::TEXT_MUTED)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(Span::styled(
                format!("  {}", "\u{2500}".repeat(58)),
                Style::default().fg(theme::TEXT_DIM),
            )));

            for (i, p) in data.providers.iter().enumerate() {
                let is_selected = i == self.selected;
                let marker = if is_selected { "\u{25b8} " } else { "  " };
                let style = if is_selected {
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT)
                };

                lines.push(Line::from(vec![
                    Span::styled(marker, style),
                    Span::styled(format!("{:<18}", p.name), style),
                    Span::styled(format!(" ${:>8.4}", p.cost), style),
                    Span::styled(format!(" {:>7}", p.requests), style),
                    Span::styled(
                        format!(" {:>9}", format_tokens(p.input_tokens)),
                        Style::default().fg(theme::TEXT_MUTED),
                    ),
                    Span::styled(
                        format!(" {:>9}", format_tokens(p.output_tokens)),
                        Style::default().fg(theme::TEXT_MUTED),
                    ),
                ]));
            }
        }

        // Hints
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  [Tab] switch tab  [r] refresh  [j/k] select provider",
            Style::default().fg(theme::TEXT_DIM),
        )));

        let visible = inner.height as usize;
        let max_scroll = lines.len().saturating_sub(visible);
        let scroll = self.scroll.min(max_scroll);

        frame.render_widget(Paragraph::new(lines).scroll((scroll as u16, 0)), inner);
    }

    // ── Alerts tab ──────────────────────────────────────────────────────

    fn render_alerts(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Filter bar
        self.render_alert_filter_bar(frame, chunks[0]);
        self.render_alert_list(frame, chunks[1]);
    }

    fn render_alert_filter_bar(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_DIM));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let filter_label = match self.severity_filter {
            None => "All",
            Some(AuditSeverity::Info) => "Info+",
            Some(AuditSeverity::Warning) => "Warning+",
            Some(AuditSeverity::Security) => "Security+",
            Some(AuditSeverity::Critical) => "Critical",
        };

        let filter_color = match self.severity_filter {
            None => theme::TEXT,
            Some(AuditSeverity::Info) => theme::INFO,
            Some(AuditSeverity::Warning) => theme::WARNING,
            Some(AuditSeverity::Security) => theme::ACCENT,
            Some(AuditSeverity::Critical) => theme::ERROR,
        };

        let line = Line::from(vec![
            Span::styled(" Filter: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                filter_label,
                Style::default()
                    .fg(filter_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{} events", self.alert_count),
                Style::default().fg(theme::TEXT_MUTED),
            ),
            Span::raw("  "),
            Span::styled(
                "[f] cycle filter  [j/k] navigate  [g/G] top/bottom",
                Style::default().fg(theme::TEXT_DIM),
            ),
        ]);

        frame.render_widget(Paragraph::new(line), inner);
    }

    fn render_alert_list(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_focused("System Alerts");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let events = if let Some(severity) = self.severity_filter {
            self.audit_logger.get_by_severity(severity)
        } else {
            self.audit_logger.get_recent(200)
        };

        if events.is_empty() {
            let msg = if self.severity_filter.is_some() {
                "No alerts match the current filter"
            } else {
                "No system alerts recorded yet"
            };
            frame.render_widget(
                Paragraph::new(vec![
                    Line::raw(""),
                    Line::from(Span::styled(
                        format!("  {msg}"),
                        Style::default().fg(theme::TEXT_MUTED),
                    )),
                    Line::raw(""),
                    Line::from(Span::styled(
                        "  Alerts will appear here as system events occur.",
                        Style::default().fg(theme::TEXT_DIM),
                    )),
                ]),
                inner,
            );
            return;
        }

        let visible_height = inner.height as usize;
        let scroll = self.scroll.min(events.len().saturating_sub(visible_height));

        let mut lines: Vec<Line<'static>> = Vec::new();
        for (i, event) in events.iter().enumerate().skip(scroll).take(visible_height) {
            let is_selected = i == self.selected;
            let marker = if is_selected { "\u{25b8}" } else { " " };

            let severity_color = match event.severity {
                AuditSeverity::Info => theme::INFO,
                AuditSeverity::Warning => theme::WARNING,
                AuditSeverity::Security => theme::ACCENT,
                AuditSeverity::Critical => theme::ERROR,
            };

            let severity_label = match event.severity {
                AuditSeverity::Info => "INFO",
                AuditSeverity::Warning => "WARN",
                AuditSeverity::Security => "SEC ",
                AuditSeverity::Critical => "CRIT",
            };

            let time = event.timestamp.format("%H:%M:%S").to_string();
            let desc = format!("{:?}", event.event_type);
            let desc_truncated = if desc.len() > 60 {
                format!("{}...", &desc[..57])
            } else {
                desc
            };

            let row_style = if is_selected {
                Style::default()
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };

            lines.push(Line::from(vec![
                Span::styled(format!("{marker} "), row_style),
                Span::styled(
                    format!("{severity_label} "),
                    Style::default()
                        .fg(severity_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("{time} "), Style::default().fg(theme::TEXT_DIM)),
                Span::styled(desc_truncated, row_style),
            ]));
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        format!("{tokens}")
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state_defaults() {
        let state = SystemViewState::new();
        assert_eq!(state.tab, SystemTab::Vault);
        assert_eq!(state.scroll, 0);
        assert_eq!(state.selected, 0);
        assert!(state.credentials.is_empty());
        assert!(state.budget_data.is_none());
        assert!(!state.budget_loading);
        assert!(state.severity_filter.is_none());
        assert_eq!(state.alert_count, 0);
    }

    #[test]
    fn test_tab_cycling_forward() {
        assert_eq!(SystemTab::Vault.next(), SystemTab::Budget);
        assert_eq!(SystemTab::Budget.next(), SystemTab::Alerts);
        assert_eq!(SystemTab::Alerts.next(), SystemTab::Vault);
    }

    #[test]
    fn test_tab_cycling_backward() {
        assert_eq!(SystemTab::Vault.prev(), SystemTab::Alerts);
        assert_eq!(SystemTab::Budget.prev(), SystemTab::Vault);
        assert_eq!(SystemTab::Alerts.prev(), SystemTab::Budget);
    }

    #[test]
    fn test_tab_labels() {
        assert_eq!(SystemTab::Vault.label(), "Vault/Credentials");
        assert_eq!(SystemTab::Budget.label(), "Budget");
        assert_eq!(SystemTab::Alerts.label(), "Alerts");
    }

    #[test]
    fn test_format_tokens_helper() {
        assert_eq!(format_tokens(500), "500");
        assert_eq!(format_tokens(1_500), "1.5K");
        assert_eq!(format_tokens(2_500_000), "2.5M");
        assert_eq!(format_tokens(0), "0");
    }
}
