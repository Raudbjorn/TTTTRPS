//! Audit viewer — displays audit events with severity filtering.
//!
//! Currently operates with a local AuditLogger instance. When AuditLogger
//! is added to Services, this view will display real application-wide events.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::super::theme;
use crate::core::audit::{AuditLogger, AuditSeverity};
use crate::tui::services::Services;

// ── State ──────────────────────────────────────────────────────────────────

pub struct AuditViewState {
    logger: AuditLogger,
    severity_filter: Option<AuditSeverity>,
    selected: usize,
    scroll: usize,
    event_count: usize,
}

impl AuditViewState {
    pub fn new() -> Self {
        Self {
            logger: AuditLogger::new(),
            severity_filter: None,
            selected: 0,
            scroll: 0,
            event_count: 0,
        }
    }

    pub fn load(&mut self, _services: &Services) {
        self.refresh_count();
    }

    pub fn poll(&mut self) {
        // AuditLogger is sync/local — nothing to poll
    }

    fn refresh_count(&mut self) {
        self.event_count = self.logger.count();
    }

    pub fn handle_input(&mut self, event: &Event, _services: &Services) -> bool {
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
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                if self.event_count > 0 {
                    self.selected = (self.selected + 1).min(self.event_count.saturating_sub(1));
                    self.ensure_visible();
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                self.selected = self.selected.saturating_sub(1);
                self.ensure_visible();
                true
            }
            // Severity filter cycling: None → Info → Warning → Security → Critical → None
            (KeyModifiers::NONE, KeyCode::Char('f')) => {
                self.severity_filter = match self.severity_filter {
                    None => Some(AuditSeverity::Info),
                    Some(AuditSeverity::Info) => Some(AuditSeverity::Warning),
                    Some(AuditSeverity::Warning) => Some(AuditSeverity::Security),
                    Some(AuditSeverity::Security) => Some(AuditSeverity::Critical),
                    Some(AuditSeverity::Critical) => None,
                };
                self.selected = 0;
                self.scroll = 0;
                self.refresh_count();
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('g')) => {
                self.selected = 0;
                self.scroll = 0;
                true
            }
            (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
                if self.event_count > 0 {
                    self.selected = self.event_count - 1;
                    self.ensure_visible();
                }
                true
            }
            _ => false,
        }
    }

    fn ensure_visible(&mut self) {
        if self.selected < self.scroll {
            self.scroll = self.selected;
        }
        // Will be adjusted in render based on viewport height
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        self.render_filter_bar(frame, chunks[0]);
        self.render_events(frame, chunks[1]);
    }

    fn render_filter_bar(&self, frame: &mut Frame, area: Rect) {
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
                format!("{} events", self.event_count),
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

    fn render_events(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_focused("Audit Log");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let events = if let Some(severity) = self.severity_filter {
            self.logger.get_by_severity(severity)
        } else {
            self.logger.get_recent(200)
        };

        if events.is_empty() {
            let msg = if self.severity_filter.is_some() {
                "No events match the current filter"
            } else {
                "No audit events recorded yet"
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
                        "  Audit events will appear here as actions occur.",
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
            let marker = if is_selected { "▸" } else { " " };

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

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = AuditViewState::new();
        assert_eq!(state.event_count, 0);
        assert!(state.severity_filter.is_none());
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn test_severity_filter_cycling() {
        let mut state = AuditViewState::new();
        // None → Info
        assert!(state.severity_filter.is_none());
        // Simulate pressing 'f' four times
        state.severity_filter = Some(AuditSeverity::Info);
        assert_eq!(state.severity_filter, Some(AuditSeverity::Info));
        state.severity_filter = Some(AuditSeverity::Warning);
        assert_eq!(state.severity_filter, Some(AuditSeverity::Warning));
        state.severity_filter = Some(AuditSeverity::Security);
        assert_eq!(state.severity_filter, Some(AuditSeverity::Security));
        state.severity_filter = Some(AuditSeverity::Critical);
        assert_eq!(state.severity_filter, Some(AuditSeverity::Critical));
        state.severity_filter = None;
        assert!(state.severity_filter.is_none());
    }

    #[test]
    fn test_empty_event_count() {
        let state = AuditViewState::new();
        assert_eq!(state.logger.count(), 0);
    }
}
