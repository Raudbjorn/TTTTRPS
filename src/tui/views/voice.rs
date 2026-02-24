//! Voice manager — queue status, provider config, voice profiles.
//!
//! Displays synthesis queue stats and voice provider configuration
//! from Services (SynthesisQueue + VoiceManager).

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

struct VoiceData {
    // Queue stats
    queue_pending: usize,
    queue_processing: usize,
    queue_completed: usize,
    queue_failed: usize,
    // Config
    provider: String,
    cache_enabled: bool,
    max_concurrent: usize,
}

// ── Tab ────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
enum VoiceTab {
    Queue,
    Config,
}

impl VoiceTab {
    fn label(self) -> &'static str {
        match self {
            Self::Queue => "Queue",
            Self::Config => "Config",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Queue => Self::Config,
            Self::Config => Self::Queue,
        }
    }
}

// ── State ──────────────────────────────────────────────────────────────────

pub struct VoiceViewState {
    data: Option<VoiceData>,
    loading: bool,
    tab: VoiceTab,
    scroll: usize,
    data_rx: mpsc::UnboundedReceiver<VoiceData>,
    data_tx: mpsc::UnboundedSender<VoiceData>,
}

impl VoiceViewState {
    pub fn new() -> Self {
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        Self {
            data: None,
            loading: false,
            tab: VoiceTab::Queue,
            scroll: 0,
            data_rx,
            data_tx,
        }
    }

    pub fn load(&mut self, services: &Services) {
        self.loading = true;
        let tx = self.data_tx.clone();
        let voice_queue = services.voice.clone();
        let voice_mgr = services.voice_manager.clone();

        tokio::spawn(async move {
            let stats = voice_queue.stats().await;
            let mgr = voice_mgr.read().await;
            let config = mgr.get_config();

            let provider = format!("{:?}", config.provider);
            let cache_enabled = config.cache_dir.is_some();

            let data = VoiceData {
                queue_pending: stats.pending_count,
                queue_processing: stats.processing_count,
                queue_completed: stats.completed_count as usize,
                queue_failed: stats.failed_count as usize,
                provider,
                cache_enabled,
                max_concurrent: stats.utilization as usize,
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
                self.tab = self.tab.next(); // Only 2 tabs, next == prev
                self.scroll = 0;
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.load(services);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                self.scroll = self.scroll.saturating_add(1);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                self.scroll = self.scroll.saturating_sub(1);
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
            VoiceTab::Queue => self.render_queue(frame, chunks[1]),
            VoiceTab::Config => self.render_config(frame, chunks[1]),
        }
    }

    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let tabs = [VoiceTab::Queue, VoiceTab::Config];
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

    fn render_queue(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_focused("Synthesis Queue");
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
                    " Press 'r' to load queue status",
                    Style::default().fg(theme::TEXT_MUTED),
                ))),
                inner,
            );
            return;
        };

        let total = data.queue_pending + data.queue_processing + data.queue_completed + data.queue_failed;

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  QUEUE STATUS",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::raw(""));

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Pending:    ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", data.queue_pending),
                Style::default().fg(theme::WARNING),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Processing: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", data.queue_processing),
                Style::default().fg(theme::INFO),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Completed:  ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", data.queue_completed),
                Style::default().fg(theme::SUCCESS),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Failed:     ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", data.queue_failed),
                if data.queue_failed > 0 {
                    Style::default().fg(theme::ERROR)
                } else {
                    Style::default().fg(theme::TEXT_DIM)
                },
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Total:      ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("{total}"),
                Style::default()
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Concurrent: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", data.max_concurrent),
                Style::default().fg(theme::TEXT),
            ),
        ]));

        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  [Tab] switch tab  [r] refresh",
            Style::default().fg(theme::TEXT_DIM),
        )));

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_config(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_focused("Voice Configuration");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(ref data) = self.data else {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " Press 'r' to load configuration",
                    Style::default().fg(theme::TEXT_MUTED),
                ))),
                inner,
            );
            return;
        };

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  PROVIDER",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::raw(""));

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Default Provider: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                data.provider.clone(),
                Style::default().fg(theme::PRIMARY_LIGHT),
            ),
        ]));

        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  CACHE",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::raw(""));

        let cache_color = if data.cache_enabled {
            theme::SUCCESS
        } else {
            theme::TEXT_DIM
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Cache: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                if data.cache_enabled {
                    "Enabled"
                } else {
                    "Disabled"
                },
                Style::default()
                    .fg(cache_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  [Tab] switch tab  [r] refresh",
            Style::default().fg(theme::TEXT_DIM),
        )));

        frame.render_widget(Paragraph::new(lines), inner);
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = VoiceViewState::new();
        assert!(state.data.is_none());
        assert!(!state.loading);
        assert_eq!(state.tab, VoiceTab::Queue);
    }

    #[test]
    fn test_tab_cycling() {
        assert_eq!(VoiceTab::Queue.next(), VoiceTab::Config);
        assert_eq!(VoiceTab::Config.next(), VoiceTab::Queue);
    }

    #[test]
    fn test_tab_labels() {
        assert_eq!(VoiceTab::Queue.label(), "Queue");
        assert_eq!(VoiceTab::Config.label(), "Config");
    }
}
