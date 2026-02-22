//! Campaign view — chat session management and campaign overview.
//!
//! Displays chat sessions from the database with status, dates, and linked
//! campaign info. Supports selecting a session and switching the chat view
//! to it. Data loaded asynchronously from SQLite. Scrollable with j/k,
//! selectable with Enter.

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
struct SessionDisplay {
    id: String,
    status: String,
    created_at: String,
    updated_at: String,
    linked_campaign: String,
    is_active: bool,
}

#[derive(Clone, Debug)]
struct CampaignData {
    sessions: Vec<SessionDisplay>,
    total_count: usize,
    active_count: usize,
    archived_count: usize,
}

// ── State ────────────────────────────────────────────────────────────────────

pub enum CampaignResult {
    /// Input consumed, view stays as-is.
    Consumed,
    /// User selected a session to switch to.
    SwitchSession(String),
}

pub struct CampaignState {
    data: Option<CampaignData>,
    lines_cache: Vec<Line<'static>>,
    scroll: usize,
    selected: usize,
    loading: bool,
    data_rx: mpsc::UnboundedReceiver<CampaignData>,
    data_tx: mpsc::UnboundedSender<CampaignData>,
}

impl CampaignState {
    pub fn new() -> Self {
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        Self {
            data: None,
            lines_cache: Vec::new(),
            scroll: 0,
            selected: 0,
            loading: false,
            data_rx,
            data_tx,
        }
    }

    /// Trigger async data load from SQLite database.
    pub fn load(&mut self, services: &Services) {
        if self.loading {
            return;
        }
        self.loading = true;

        let db = services.database.clone();
        let tx = self.data_tx.clone();

        tokio::spawn(async move {
            use crate::database::ChatOps;

            let sessions_result = db.list_chat_sessions(100).await;

            let sessions = match sessions_result {
                Ok(sessions) => sessions,
                Err(e) => {
                    log::warn!("Failed to load chat sessions: {e}");
                    Vec::new()
                }
            };

            let total_count = sessions.len();
            let active_count = sessions.iter().filter(|s| s.is_active()).count();
            let archived_count = total_count - active_count;

            let display_sessions: Vec<SessionDisplay> = sessions
                .into_iter()
                .map(|s| {
                    let is_active = s.is_active();
                    SessionDisplay {
                        id: s.id,
                        status: s.status,
                        created_at: format_datetime(&s.created_at),
                        updated_at: format_datetime(&s.updated_at),
                        linked_campaign: s
                            .linked_campaign_id
                            .unwrap_or_else(|| "—".to_string()),
                        is_active,
                    }
                })
                .collect();

            let data = CampaignData {
                sessions: display_sessions,
                total_count,
                active_count,
                archived_count,
            };

            let _ = tx.send(data);
        });
    }

    /// Poll for async data completion. Call from on_tick.
    pub fn poll(&mut self) {
        if let Ok(data) = self.data_rx.try_recv() {
            self.lines_cache = build_lines(&data, self.selected);
            self.data = Some(data);
            self.loading = false;
        }
    }

    /// Session count (for selection bounds).
    fn session_count(&self) -> usize {
        self.data.as_ref().map(|d| d.sessions.len()).unwrap_or(0)
    }

    // ── Input ────────────────────────────────────────────────────────────

    pub fn handle_input(&mut self, event: &Event, services: &Services) -> Option<CampaignResult> {
        let Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            ..
        }) = event
        else {
            return None;
        };

        match (*modifiers, *code) {
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                self.select_next();
                Some(CampaignResult::Consumed)
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                self.select_prev();
                Some(CampaignResult::Consumed)
            }
            (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
                let count = self.session_count();
                if count > 0 {
                    self.selected = count - 1;
                    self.rebuild_lines();
                }
                Some(CampaignResult::Consumed)
            }
            (KeyModifiers::NONE, KeyCode::Char('g')) => {
                self.selected = 0;
                self.rebuild_lines();
                Some(CampaignResult::Consumed)
            }
            (KeyModifiers::NONE, KeyCode::PageDown) => {
                for _ in 0..10 {
                    self.select_next();
                }
                Some(CampaignResult::Consumed)
            }
            (KeyModifiers::NONE, KeyCode::PageUp) => {
                for _ in 0..10 {
                    self.select_prev();
                }
                Some(CampaignResult::Consumed)
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                if let Some(ref data) = self.data {
                    if let Some(session) = data.sessions.get(self.selected) {
                        return Some(CampaignResult::SwitchSession(session.id.clone()));
                    }
                }
                Some(CampaignResult::Consumed)
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.load(services);
                Some(CampaignResult::Consumed)
            }
            _ => None,
        }
    }

    fn select_next(&mut self) {
        let count = self.session_count();
        if count > 0 {
            self.selected = (self.selected + 1).min(count - 1);
            self.rebuild_lines();
        }
    }

    fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
        self.rebuild_lines();
    }

    fn rebuild_lines(&mut self) {
        if let Some(ref data) = self.data {
            self.lines_cache = build_lines(data, self.selected);
        }
    }

    // ── Rendering ────────────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Campaign ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.loading && self.data.is_none() {
            let loading = Paragraph::new(vec![
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Loading sessions...",
                        Style::default().fg(Color::DarkGray),
                    ),
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

        // Auto-scroll to keep selected item visible
        let visible_height = inner.height as usize;
        let scroll = if visible_height > 0 {
            // Selected item line is at: header_lines (4) + selected
            let selected_line = 4 + self.selected;
            if selected_line >= self.scroll + visible_height {
                selected_line.saturating_sub(visible_height - 1)
            } else if selected_line < self.scroll {
                selected_line
            } else {
                self.scroll
            }
        } else {
            self.scroll
        };

        let content = Paragraph::new(self.lines_cache.clone()).scroll((scroll as u16, 0));
        frame.render_widget(content, inner);
    }
}

// ── Line builders ────────────────────────────────────────────────────────────

fn build_lines(data: &CampaignData, selected: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(data.sessions.len() + 15);

    // Header
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "  Chat Sessions",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        format!("  {}", "─".repeat(70)),
        Style::default().fg(Color::DarkGray),
    )));

    if data.sessions.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "No chat sessions. Start chatting to create one.",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    } else {
        // Table header
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!(
                    "  {:<10} {:>18} {:>18}  {}",
                    "Status", "Created", "Updated", "Campaign"
                ),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        for (i, session) in data.sessions.iter().enumerate() {
            let is_selected = i == selected;
            let cursor = if is_selected { "▸ " } else { "  " };

            let status_color = if session.is_active {
                Color::Green
            } else {
                Color::DarkGray
            };

            let campaign_display = if session.linked_campaign.len() > 16 {
                format!("{}...", &session.linked_campaign[..13])
            } else {
                session.linked_campaign.clone()
            };

            let row_style = if is_selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            lines.push(Line::from(vec![
                Span::styled(
                    cursor.to_string(),
                    if is_selected {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(
                    format!("{:<10}", session.status),
                    Style::default().fg(status_color),
                ),
                Span::styled(
                    format!(" {:>18} {:>18}", session.created_at, session.updated_at),
                    row_style,
                ),
                Span::raw(format!("  {}", campaign_display)),
            ]));
        }
    }

    // Summary
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        format!("  {}", "─".repeat(70)),
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Total: ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!("{} sessions", data.total_count)),
        Span::styled(" (", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} active", data.active_count),
            Style::default().fg(Color::Green),
        ),
        Span::raw(", "),
        Span::styled(
            format!("{} archived", data.archived_count),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(")", Style::default().fg(Color::DarkGray)),
    ]));

    // Footer
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("j/k", Style::default().fg(Color::DarkGray)),
        Span::raw(":select "),
        Span::styled("Enter", Style::default().fg(Color::DarkGray)),
        Span::raw(":switch "),
        Span::styled("G/g", Style::default().fg(Color::DarkGray)),
        Span::raw(":bottom/top "),
        Span::styled("r", Style::default().fg(Color::DarkGray)),
        Span::raw(":refresh"),
    ]));
    lines.push(Line::raw(""));

    lines
}

/// Format an RFC3339 datetime string into a shorter display format.
fn format_datetime(rfc3339: &str) -> String {
    // Try to parse RFC3339, fall back to truncated display
    chrono::DateTime::parse_from_rfc3339(rfc3339)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|_| {
            if rfc3339.len() > 16 {
                rfc3339[..16].to_string()
            } else {
                rfc3339.to_string()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_campaign_state_new() {
        let state = CampaignState::new();
        assert!(state.data.is_none());
        assert!(state.lines_cache.is_empty());
        assert_eq!(state.scroll, 0);
        assert_eq!(state.selected, 0);
        assert!(!state.loading);
    }

    #[test]
    fn test_build_lines_empty() {
        let data = CampaignData {
            sessions: vec![],
            total_count: 0,
            active_count: 0,
            archived_count: 0,
        };
        let lines = build_lines(&data, 0);
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
        assert!(text.contains("Chat Sessions"));
        assert!(text.contains("No chat sessions"));
        assert!(text.contains("0 sessions"));
    }

    #[test]
    fn test_build_lines_with_sessions() {
        let data = CampaignData {
            sessions: vec![
                SessionDisplay {
                    id: "sess-1".to_string(),
                    status: "active".to_string(),
                    created_at: "2026-02-22 14:00".to_string(),
                    updated_at: "2026-02-22 15:30".to_string(),
                    linked_campaign: "—".to_string(),
                    is_active: true,
                },
                SessionDisplay {
                    id: "sess-2".to_string(),
                    status: "archived".to_string(),
                    created_at: "2026-02-21 10:00".to_string(),
                    updated_at: "2026-02-21 12:00".to_string(),
                    linked_campaign: "D&D 5e".to_string(),
                    is_active: false,
                },
            ],
            total_count: 2,
            active_count: 1,
            archived_count: 1,
        };
        let lines = build_lines(&data, 0);
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
        assert!(text.contains("active"));
        assert!(text.contains("archived"));
        assert!(text.contains("2 sessions"));
        assert!(text.contains("1 active"));
        assert!(text.contains("1 archived"));
        assert!(text.contains("D&D 5e"));
    }

    #[test]
    fn test_build_lines_selection_marker() {
        let data = CampaignData {
            sessions: vec![
                SessionDisplay {
                    id: "a".to_string(),
                    status: "active".to_string(),
                    created_at: "2026-02-22".to_string(),
                    updated_at: "2026-02-22".to_string(),
                    linked_campaign: "—".to_string(),
                    is_active: true,
                },
                SessionDisplay {
                    id: "b".to_string(),
                    status: "archived".to_string(),
                    created_at: "2026-02-21".to_string(),
                    updated_at: "2026-02-21".to_string(),
                    linked_campaign: "—".to_string(),
                    is_active: false,
                },
            ],
            total_count: 2,
            active_count: 1,
            archived_count: 1,
        };

        // Selected = 0: first row should have ▸
        let lines_sel0 = build_lines(&data, 0);
        let text0: String = lines_sel0
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text0.contains("▸"));

        // Selected = 1: second row has marker
        let lines_sel1 = build_lines(&data, 1);
        let all_text: Vec<String> = lines_sel1
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.to_string())
                    .collect::<String>()
            })
            .collect();
        // The ▸ marker should be on a different line now
        let marker_lines: Vec<&String> =
            all_text.iter().filter(|line| line.contains('▸')).collect();
        assert_eq!(marker_lines.len(), 1);
    }

    #[test]
    fn test_format_datetime() {
        let rfc3339 = "2026-02-22T14:30:00+00:00";
        let result = format_datetime(rfc3339);
        assert_eq!(result, "2026-02-22 14:30");

        // Non-RFC3339 fallback
        let bad = "not-a-date";
        let result = format_datetime(bad);
        assert_eq!(result, "not-a-date");
    }

    #[test]
    fn test_selection_bounds() {
        let mut state = CampaignState::new();
        // No data — select_next/prev should not panic
        state.select_next();
        assert_eq!(state.selected, 0);
        state.select_prev();
        assert_eq!(state.selected, 0);
    }
}
