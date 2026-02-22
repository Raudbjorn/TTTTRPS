//! Library view — displays ingested documents from SurrealDB storage.
//!
//! Shows library items with metadata (title, type, pages, chunks, status).
//! Data loaded asynchronously from SurrealDB. Scrollable with j/k.

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
struct ItemDisplay {
    title: String,
    file_type: String,
    page_count: Option<i32>,
    chunk_count: i64,
    status: String,
    game_system: String,
}

#[derive(Clone, Debug)]
struct LibraryData {
    items: Vec<ItemDisplay>,
    total_count: usize,
    ready_count: usize,
    pending_count: usize,
    error_count: usize,
}

// ── State ────────────────────────────────────────────────────────────────────

pub struct LibraryState {
    data: Option<LibraryData>,
    lines_cache: Vec<Line<'static>>,
    scroll: usize,
    loading: bool,
    data_rx: mpsc::UnboundedReceiver<LibraryData>,
    data_tx: mpsc::UnboundedSender<LibraryData>,
}

impl LibraryState {
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

    /// Trigger async data load from SurrealDB storage.
    pub fn load(&mut self, services: &Services) {
        if self.loading {
            return;
        }
        self.loading = true;

        let storage = services.storage.clone();
        let tx = self.data_tx.clone();

        tokio::spawn(async move {
            let db = storage.db();

            // Load all items (up to 200)
            let items_result =
                crate::core::storage::get_library_items(db, None, 200, 0).await;

            let items = match items_result {
                Ok(items) => items,
                Err(e) => {
                    log::warn!("Failed to load library items: {e}");
                    Vec::new()
                }
            };

            let total_count = items.len();
            let ready_count = items.iter().filter(|i| i.item.status == "ready").count();
            let pending_count = items
                .iter()
                .filter(|i| i.item.status == "pending" || i.item.status == "processing")
                .count();
            let error_count = items.iter().filter(|i| i.item.status == "error").count();

            let display_items: Vec<ItemDisplay> = items
                .into_iter()
                .map(|iwc| ItemDisplay {
                    title: iwc.item.title,
                    file_type: iwc
                        .item
                        .file_type
                        .unwrap_or_else(|| "—".to_string()),
                    page_count: iwc.item.page_count,
                    chunk_count: iwc.chunk_count,
                    status: iwc.item.status,
                    game_system: iwc
                        .item
                        .game_system
                        .unwrap_or_else(|| "—".to_string()),
                })
                .collect();

            let data = LibraryData {
                items: display_items,
                total_count,
                ready_count,
                pending_count,
                error_count,
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
            .title(" Library ")
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
                        "Loading library...",
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

        let content =
            Paragraph::new(self.lines_cache.clone()).scroll((self.scroll as u16, 0));
        frame.render_widget(content, inner);
    }
}

// ── Line builders ────────────────────────────────────────────────────────────

fn build_lines(data: &LibraryData) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(data.items.len() + 15);

    // Header
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "  Documents",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        format!("  {}", "─".repeat(70)),
        Style::default().fg(Color::DarkGray),
    )));

    if data.items.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "No documents in library. Ingest PDFs, EPUBs, or markdown to get started.",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    } else {
        // Table header
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!(
                    "{:<30} {:>5} {:>6} {:>7} {:>8}  {}",
                    "Title", "Type", "Pages", "Chunks", "Status", "System"
                ),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        for item in &data.items {
            let title_display = if item.title.len() > 28 {
                format!("{}...", &item.title[..25])
            } else {
                item.title.clone()
            };

            let pages = item
                .page_count
                .map(|p| p.to_string())
                .unwrap_or_else(|| "—".to_string());

            let status_color = match item.status.as_str() {
                "ready" => Color::Green,
                "processing" => Color::Yellow,
                "pending" => Color::DarkGray,
                "error" => Color::Red,
                _ => Color::DarkGray,
            };

            let system_display = if item.game_system.len() > 12 {
                format!("{}...", &item.game_system[..9])
            } else {
                item.game_system.clone()
            };

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{:<30}", title_display),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(format!(" {:>5} {:>6} {:>7} ", item.file_type, pages, item.chunk_count)),
                Span::styled(
                    format!("{:>8}", item.status),
                    Style::default().fg(status_color),
                ),
                Span::raw(format!("  {}", system_display)),
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
        Span::raw(format!("{} items", data.total_count)),
        Span::styled(" (", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} ready", data.ready_count),
            Style::default().fg(Color::Green),
        ),
        Span::raw(", "),
        Span::styled(
            format!("{} pending", data.pending_count),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(", "),
        Span::styled(
            format!("{} error", data.error_count),
            Style::default().fg(Color::Red),
        ),
        Span::styled(")", Style::default().fg(Color::DarkGray)),
    ]));

    // Footer
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_state_new() {
        let state = LibraryState::new();
        assert!(state.data.is_none());
        assert!(state.lines_cache.is_empty());
        assert_eq!(state.scroll, 0);
        assert!(!state.loading);
    }

    #[test]
    fn test_build_lines_empty() {
        let data = LibraryData {
            items: vec![],
            total_count: 0,
            ready_count: 0,
            pending_count: 0,
            error_count: 0,
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
        assert!(text.contains("Documents"));
        assert!(text.contains("No documents in library"));
        assert!(text.contains("0 items"));
    }

    #[test]
    fn test_build_lines_with_items() {
        let data = LibraryData {
            items: vec![
                ItemDisplay {
                    title: "Player's Handbook".to_string(),
                    file_type: "pdf".to_string(),
                    page_count: Some(300),
                    chunk_count: 1250,
                    status: "ready".to_string(),
                    game_system: "D&D 5e".to_string(),
                },
                ItemDisplay {
                    title: "Homebrew Notes".to_string(),
                    file_type: "md".to_string(),
                    page_count: None,
                    chunk_count: 45,
                    status: "pending".to_string(),
                    game_system: "—".to_string(),
                },
            ],
            total_count: 2,
            ready_count: 1,
            pending_count: 1,
            error_count: 0,
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
        assert!(text.contains("Player's Handbook"));
        assert!(text.contains("Homebrew Notes"));
        assert!(text.contains("2 items"));
        assert!(text.contains("1 ready"));
        assert!(text.contains("1 pending"));
    }

    #[test]
    fn test_scroll_bounds() {
        let mut state = LibraryState::new();
        state.scroll_down(10);
        assert_eq!(state.scroll, 0);

        state.lines_cache = vec![Line::raw(""); 20];
        state.scroll_down(5);
        assert_eq!(state.scroll, 5);
        state.scroll_down(100);
        assert_eq!(state.scroll, 19);
        state.scroll_up(100);
        assert_eq!(state.scroll, 0);
    }
}
