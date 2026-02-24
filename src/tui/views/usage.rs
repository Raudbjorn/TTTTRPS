//! Usage dashboard — token costs, budget status, provider breakdown.
//!
//! Displays cost summaries from the LLM router, per-provider stats,
//! and budget tracking. Data loaded asynchronously via mpsc channel.

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
use crate::tui::services::Services;

// ── Data types ─────────────────────────────────────────────────────────────

struct UsageData {
    total_cost: f64,
    monthly_cost: f64,
    daily_cost: f64,
    monthly_budget: Option<f64>,
    daily_budget: Option<f64>,
    within_budget: bool,
    providers: Vec<ProviderRow>,
    // Search analytics snapshot
    search_total: u64,
    search_zero_results: usize,
    search_popular: Vec<(String, u32)>,
}

struct ProviderRow {
    name: String,
    requests: u64,
    success_rate: f64,
    avg_latency_ms: u64,
    total_cost: f64,
    input_tokens: u64,
    output_tokens: u64,
}

// ── Tab ────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
enum UsageTab {
    Summary,
    Providers,
}

impl UsageTab {
    fn label(self) -> &'static str {
        match self {
            Self::Summary => "Summary",
            Self::Providers => "Providers",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Summary => Self::Providers,
            Self::Providers => Self::Summary,
        }
    }

    fn prev(self) -> Self {
        self.next()
    }
}

// ── State ──────────────────────────────────────────────────────────────────

pub struct UsageViewState {
    data: Option<UsageData>,
    loading: bool,
    tab: UsageTab,
    scroll: usize,
    selected_provider: usize,
    data_rx: mpsc::UnboundedReceiver<UsageData>,
    data_tx: mpsc::UnboundedSender<UsageData>,
}

impl UsageViewState {
    pub fn new() -> Self {
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        Self {
            data: None,
            loading: false,
            tab: UsageTab::Summary,
            scroll: 0,
            selected_provider: 0,
            data_rx,
            data_tx,
        }
    }

    pub fn load(&mut self, services: &Services) {
        self.loading = true;
        let tx = self.data_tx.clone();
        let llm = services.llm.clone();

        // Capture search analytics snapshot (sync, in-memory)
        let analytics_summary = services.search_analytics.get_summary(24);
        let search_total = analytics_summary.total_searches as u64;
        let search_zero_results = services.search_analytics.get_zero_result_queries(24).len();
        let search_popular = services.search_analytics.get_popular_queries(5);

        tokio::spawn(async move {
            let cost_summary = llm.get_cost_summary().await;
            let all_stats = llm.get_all_stats().await;

            let mut providers: Vec<ProviderRow> = all_stats
                .iter()
                .map(|(name, stats)| ProviderRow {
                    name: name.clone(),
                    requests: stats.total_requests,
                    success_rate: stats.success_rate(),
                    avg_latency_ms: stats.avg_latency_ms(),
                    total_cost: stats.total_cost_usd,
                    input_tokens: stats.total_input_tokens,
                    output_tokens: stats.total_output_tokens,
                })
                .collect();
            providers.sort_by(|a, b| b.total_cost.partial_cmp(&a.total_cost).unwrap_or(std::cmp::Ordering::Equal));

            let data = UsageData {
                total_cost: cost_summary.total_cost_usd,
                monthly_cost: cost_summary.monthly_cost,
                daily_cost: cost_summary.daily_cost,
                monthly_budget: cost_summary.monthly_budget,
                daily_budget: cost_summary.daily_budget,
                within_budget: cost_summary.is_within_budget,
                providers,
                search_total,
                search_zero_results,
                search_popular,
            };

            let _ = tx.send(data);
        });
    }

    pub fn poll(&mut self) {
        if let Ok(data) = self.data_rx.try_recv() {
            self.data = Some(data);
            self.loading = false;
        }
    }

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
            (KeyModifiers::NONE, KeyCode::Tab) => {
                self.tab = self.tab.next();
                self.scroll = 0;
                true
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                self.tab = self.tab.prev();
                self.scroll = 0;
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.load(services);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                if self.tab == UsageTab::Providers {
                    if let Some(ref data) = self.data {
                        if self.selected_provider + 1 < data.providers.len() {
                            self.selected_provider += 1;
                        }
                    }
                } else {
                    self.scroll = self.scroll.saturating_add(1);
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                if self.tab == UsageTab::Providers {
                    self.selected_provider = self.selected_provider.saturating_sub(1);
                } else {
                    self.scroll = self.scroll.saturating_sub(1);
                }
                true
            }
            _ => false,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        self.render_tabs(frame, chunks[0]);

        match self.tab {
            UsageTab::Summary => self.render_summary(frame, chunks[1]),
            UsageTab::Providers => self.render_providers(frame, chunks[1]),
        }
    }

    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let tabs = [UsageTab::Summary, UsageTab::Providers];
        let spans: Vec<Span> = tabs
            .iter()
            .flat_map(|t| {
                let style = if *t == self.tab {
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT_MUTED)
                };
                vec![Span::styled(format!(" {} ", t.label()), style), Span::raw("│")]
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_DIM));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(Paragraph::new(Line::from(spans)), inner);
    }

    fn render_summary(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_focused("Usage Summary");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.loading && self.data.is_none() {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " Loading...",
                    Style::default().fg(theme::TEXT_MUTED),
                ))),
                inner,
            );
            return;
        }

        let Some(ref data) = self.data else {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " Press 'r' to load usage data",
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
            "  COST OVERVIEW",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::raw(""));

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Total:   ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("${:.4}", data.total_cost),
                Style::default()
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Monthly: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("${:.4}", data.monthly_cost),
                Style::default().fg(theme::TEXT),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Daily:   ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("${:.4}", data.daily_cost),
                Style::default().fg(theme::TEXT),
            ),
        ]));

        lines.push(Line::raw(""));

        // Budget status
        lines.push(Line::from(Span::styled(
            "  BUDGET",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::raw(""));

        if let Some(monthly) = data.monthly_budget {
            let pct = if monthly > 0.0 {
                (data.monthly_cost / monthly * 100.0).min(999.9)
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

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Monthly: ", Style::default().fg(theme::TEXT_MUTED)),
                Span::styled(
                    format!("${:.2} / ${:.2} ({:.1}%)", data.monthly_cost, monthly, pct),
                    Style::default().fg(color),
                ),
            ]));

            // Budget bar
            let bar_width = 30usize;
            let filled = ((pct / 100.0) * bar_width as f64).min(bar_width as f64) as usize;
            let empty = bar_width.saturating_sub(filled);
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("█".repeat(filled), Style::default().fg(color)),
                Span::styled("░".repeat(empty), Style::default().fg(theme::TEXT_DIM)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Monthly: ", Style::default().fg(theme::TEXT_MUTED)),
                Span::styled("No budget set", Style::default().fg(theme::TEXT_DIM)),
            ]));
        }

        if let Some(daily) = data.daily_budget {
            let pct = if daily > 0.0 {
                (data.daily_cost / daily * 100.0).min(999.9)
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
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Daily:   ", Style::default().fg(theme::TEXT_MUTED)),
                Span::styled(
                    format!("${:.2} / ${:.2} ({:.1}%)", data.daily_cost, daily, pct),
                    Style::default().fg(color),
                ),
            ]));
        }

        lines.push(Line::raw(""));

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
            Span::styled("Status: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                status_text,
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::raw(""));

        // Provider summary
        lines.push(Line::from(Span::styled(
            "  PROVIDERS",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::raw(""));

        for p in &data.providers {
            let tokens = p.input_tokens + p.output_tokens;
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{:<16}", p.name),
                    Style::default().fg(theme::PRIMARY_LIGHT),
                ),
                Span::styled(
                    format!("${:.4}", p.total_cost),
                    Style::default().fg(theme::TEXT),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("{}req", p.requests),
                    Style::default().fg(theme::TEXT_MUTED),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("{}tok", format_tokens(tokens)),
                    Style::default().fg(theme::TEXT_DIM),
                ),
            ]));
        }

        if data.providers.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No providers configured",
                Style::default().fg(theme::TEXT_DIM),
            )));
        }

        // Search analytics
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  SEARCH (24h)",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Queries: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", data.search_total),
                Style::default().fg(theme::TEXT),
            ),
            Span::raw("  "),
            Span::styled("Zero-result: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", data.search_zero_results),
                Style::default().fg(if data.search_zero_results > 0 { theme::WARNING } else { theme::TEXT }),
            ),
        ]));
        if !data.search_popular.is_empty() {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Popular: ", Style::default().fg(theme::TEXT_MUTED)),
                Span::styled(
                    data.search_popular
                        .iter()
                        .map(|(q, c)| format!("{q} ({c})"))
                        .collect::<Vec<_>>()
                        .join(", "),
                    Style::default().fg(theme::TEXT_DIM),
                ),
            ]));
        }

        // Hints
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  [Tab] switch tab  [r] refresh  [j/k] scroll",
            Style::default().fg(theme::TEXT_DIM),
        )));

        let visible = inner.height as usize;
        let max_scroll = lines.len().saturating_sub(visible);
        let scroll = self.scroll.min(max_scroll);

        frame.render_widget(Paragraph::new(lines).scroll((scroll as u16, 0)), inner);
    }

    fn render_providers(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_focused("Provider Details");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(ref data) = self.data else {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " Press 'r' to load usage data",
                    Style::default().fg(theme::TEXT_MUTED),
                ))),
                inner,
            );
            return;
        };

        if data.providers.is_empty() {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " No providers configured",
                    Style::default().fg(theme::TEXT_DIM),
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
                format!(
                    "{:<16} {:>10} {:>8} {:>8} {:>10} {:>10}",
                    "Provider", "Cost", "Reqs", "Rate", "In Tok", "Out Tok"
                ),
                Style::default()
                    .fg(theme::TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(66)),
            Style::default().fg(theme::TEXT_DIM),
        )));

        for (i, p) in data.providers.iter().enumerate() {
            let is_selected = i == self.selected_provider;
            let marker = if is_selected { "▸ " } else { "  " };
            let style = if is_selected {
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };

            let rate_color = if p.success_rate >= 0.95 {
                theme::SUCCESS
            } else if p.success_rate >= 0.80 {
                theme::WARNING
            } else {
                theme::ERROR
            };

            lines.push(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(format!("{:<16}", p.name), style),
                Span::styled(format!(" ${:>8.4}", p.total_cost), style),
                Span::styled(format!(" {:>7}", p.requests), style),
                Span::styled(
                    format!(" {:>6.1}%", p.success_rate * 100.0),
                    Style::default().fg(rate_color),
                ),
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

        // Selected provider detail
        if let Some(p) = data.providers.get(self.selected_provider) {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                format!("  {}", "─".repeat(40)),
                Style::default().fg(theme::TEXT_DIM),
            )));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    p.name.clone(),
                    Style::default()
                        .fg(theme::PRIMARY_LIGHT)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Avg Latency: ", Style::default().fg(theme::TEXT_MUTED)),
                Span::styled(
                    format!("{}ms", p.avg_latency_ms),
                    Style::default().fg(theme::TEXT),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Cost/Request: ", Style::default().fg(theme::TEXT_MUTED)),
                Span::styled(
                    if p.requests > 0 {
                        format!("${:.6}", p.total_cost / p.requests as f64)
                    } else {
                        "N/A".to_string()
                    },
                    Style::default().fg(theme::TEXT),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Tok/Request:  ", Style::default().fg(theme::TEXT_MUTED)),
                Span::styled(
                    if p.requests > 0 {
                        format!(
                            "{}",
                            (p.input_tokens + p.output_tokens) / p.requests
                        )
                    } else {
                        "N/A".to_string()
                    },
                    Style::default().fg(theme::TEXT),
                ),
            ]));
        }

        // Hints
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  [Tab] switch tab  [r] refresh  [j/k] select",
            Style::default().fg(theme::TEXT_DIM),
        )));

        frame.render_widget(Paragraph::new(lines), inner);
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        format!("{tokens}")
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = UsageViewState::new();
        assert!(state.data.is_none());
        assert!(!state.loading);
        assert_eq!(state.tab, UsageTab::Summary);
    }

    #[test]
    fn test_tab_cycling() {
        assert_eq!(UsageTab::Summary.next(), UsageTab::Providers);
        assert_eq!(UsageTab::Providers.next(), UsageTab::Summary);
    }

    #[test]
    fn test_format_tokens() {
        assert_eq!(format_tokens(500), "500");
        assert_eq!(format_tokens(1500), "1.5K");
        assert_eq!(format_tokens(2_500_000), "2.5M");
    }

    #[test]
    fn test_tab_labels() {
        assert_eq!(UsageTab::Summary.label(), "Summary");
        assert_eq!(UsageTab::Providers.label(), "Providers");
    }
}
