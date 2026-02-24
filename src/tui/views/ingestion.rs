//! Ingestion pipeline dashboard — visual multi-stage document ingestion tracker.
//!
//! Displays a 4-stage pipeline with stacked LineGauge progress bars:
//!   1. Document Parsing — PDF/EPUB extraction (kreuzberg)
//!   2. Semantic Chunking — raw text to semantic chunks
//!   3. Vector Generation — embedding creation
//!   4. Indexing — SurrealDB insertion
//!
//! Each stage shows its name, icon, LineGauge, item counts, and status.
//! Bottom section has a scrollable log of ingestion messages.
//!
//! Keybinds: `r` refresh, `j/k` scroll log, `c` clear log.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, LineGauge, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
    Frame,
};
use tokio::sync::mpsc;

use super::super::theme;
use crate::tui::services::Services;

// ── Stage icons ─────────────────────────────────────────────────────────────

const ICON_PARSE: &str = "\u{f15c}"; //
const ICON_CHUNK: &str = "\u{e235}"; //
const ICON_EMBED: &str = "\u{f1c0}"; //
const ICON_INDEX: &str = "\u{f1b2}"; //

// ── Stage count ─────────────────────────────────────────────────────────────

const STAGE_COUNT: usize = 4;

// ── Pipeline stage status ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum StageStatus {
    Idle,
    Processing,
    Complete,
    Error(String),
}

impl StageStatus {
    fn label(&self) -> &str {
        match self {
            Self::Idle => "Idle",
            Self::Processing => "Processing",
            Self::Complete => "Complete",
            Self::Error(_) => "Error",
        }
    }

    fn color(&self) -> ratatui::style::Color {
        match self {
            Self::Idle => theme::TEXT_DIM,
            Self::Processing => theme::PRIMARY_LIGHT,
            Self::Complete => theme::SUCCESS,
            Self::Error(_) => theme::ERROR,
        }
    }

    fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }
}

// ── Pipeline stage ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PipelineStage {
    name: &'static str,
    icon: &'static str,
    progress: f64,
    processed: usize,
    total: usize,
    status: StageStatus,
}

impl PipelineStage {
    fn new(name: &'static str, icon: &'static str) -> Self {
        Self {
            name,
            icon,
            progress: 0.0,
            processed: 0,
            total: 0,
            status: StageStatus::Idle,
        }
    }

    /// Clamp progress to [0.0, 1.0].
    fn set_progress(&mut self, ratio: f64) {
        self.progress = ratio.clamp(0.0, 1.0);
    }

    fn filled_color(&self) -> ratatui::style::Color {
        match self.status {
            StageStatus::Idle => theme::TEXT_DIM,
            StageStatus::Processing => theme::PRIMARY,
            StageStatus::Complete => theme::SUCCESS,
            StageStatus::Error(_) => theme::ERROR,
        }
    }

    fn unfilled_color(&self) -> ratatui::style::Color {
        match self.status {
            StageStatus::Idle => theme::BG_SURFACE,
            _ => theme::TEXT_DIM,
        }
    }
}

// ── Ingestion events (from background tasks) ────────────────────────────────

#[derive(Debug, Clone)]
pub enum IngestionEvent {
    /// A stage progressed.
    StageProgress {
        stage: usize,
        processed: usize,
        total: usize,
    },
    /// A stage completed.
    StageComplete { stage: usize },
    /// A stage encountered an error.
    StageError { stage: usize, message: String },
    /// Active file changed.
    ActiveFile(String),
    /// Log message emitted.
    Log(String),
    /// Pipeline reset to idle.
    Reset,
}

// ── State ───────────────────────────────────────────────────────────────────

pub struct IngestionState {
    stages: [PipelineStage; STAGE_COUNT],
    active_file: Option<String>,
    scroll: usize,
    log_messages: Vec<String>,
    data_rx: mpsc::UnboundedReceiver<IngestionEvent>,
    data_tx: mpsc::UnboundedSender<IngestionEvent>,
}

impl IngestionState {
    pub fn new() -> Self {
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        Self {
            stages: [
                PipelineStage::new("Document Parsing", ICON_PARSE),
                PipelineStage::new("Semantic Chunking", ICON_CHUNK),
                PipelineStage::new("Vector Generation", ICON_EMBED),
                PipelineStage::new("Indexing", ICON_INDEX),
            ],
            active_file: None,
            scroll: 0,
            log_messages: Vec::new(),
            data_rx,
            data_tx,
        }
    }

    /// Returns a clone of the event sender for background tasks.
    pub fn event_tx(&self) -> mpsc::UnboundedSender<IngestionEvent> {
        self.data_tx.clone()
    }

    /// Load initial data. Since the ingestion pipeline is not yet wired
    /// through Services, this initializes stages at Idle and logs a hint.
    pub fn load(&mut self, _services: &Services) {
        self.reset_stages();
        self.log_messages
            .push("Ingestion pipeline ready. Drop files or press 'r' to refresh.".into());
    }

    /// Drain pending events from the background channel.
    pub fn poll(&mut self) {
        while let Ok(event) = self.data_rx.try_recv() {
            self.apply_event(event);
        }
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
            // Refresh / reset
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.reset_stages();
                self.log_messages.push("Pipeline reset.".into());
                true
            }
            // Scroll log down
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                self.scroll_down();
                true
            }
            // Scroll log up
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                self.scroll = self.scroll.saturating_sub(1);
                true
            }
            // Clear log
            (KeyModifiers::NONE, KeyCode::Char('c')) => {
                self.log_messages.clear();
                self.scroll = 0;
                true
            }
            _ => false,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Height per gauge row: 3 lines (border + gauge + border)
        // 4 stages = 12 lines for gauges + 3 for header/file + rest for log
        let gauge_height = STAGE_COUNT as u16 * 3;
        let header_height: u16 = 3;

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(header_height),
                Constraint::Length(gauge_height),
                Constraint::Min(5),
            ])
            .split(area);

        self.render_header(frame, sections[0]);
        self.render_stages(frame, sections[1]);
        self.render_log(frame, sections[2]);
    }

    // ── Private helpers ─────────────────────────────────────────────────────

    fn apply_event(&mut self, event: IngestionEvent) {
        match event {
            IngestionEvent::StageProgress {
                stage,
                processed,
                total,
            } => {
                if let Some(s) = self.stages.get_mut(stage) {
                    s.processed = processed;
                    s.total = total;
                    s.status = StageStatus::Processing;
                    let ratio = if total > 0 {
                        processed as f64 / total as f64
                    } else {
                        0.0
                    };
                    s.set_progress(ratio);
                }
            }
            IngestionEvent::StageComplete { stage } => {
                if let Some(s) = self.stages.get_mut(stage) {
                    s.status = StageStatus::Complete;
                    s.set_progress(1.0);
                    s.processed = s.total;
                    self.log_messages
                        .push(format!("{} {} complete.", s.icon, s.name));
                }
            }
            IngestionEvent::StageError { stage, message } => {
                if let Some(s) = self.stages.get_mut(stage) {
                    s.status = StageStatus::Error(message.clone());
                    self.log_messages
                        .push(format!("{} {} error: {}", s.icon, s.name, message));
                }
            }
            IngestionEvent::ActiveFile(path) => {
                self.log_messages.push(format!("Processing: {}", path));
                self.active_file = Some(path);
            }
            IngestionEvent::Log(msg) => {
                self.log_messages.push(msg);
            }
            IngestionEvent::Reset => {
                self.reset_stages();
            }
        }
    }

    fn reset_stages(&mut self) {
        for stage in &mut self.stages {
            stage.progress = 0.0;
            stage.processed = 0;
            stage.total = 0;
            stage.status = StageStatus::Idle;
        }
        self.active_file = None;
    }

    fn scroll_down(&mut self) {
        let max = self.log_messages.len().saturating_sub(1);
        if self.scroll < max {
            self.scroll += 1;
        }
    }

    fn max_log_scroll(&self, visible_lines: usize) -> usize {
        self.log_messages.len().saturating_sub(visible_lines)
    }

    // ── Render sub-sections ─────────────────────────────────────────────────

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::PRIMARY))
            .title(Span::styled(
                " Ingestion Pipeline ",
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let file_text = match &self.active_file {
            Some(f) => format!("  Active: {f}"),
            None => "  No active file".into(),
        };
        let file_color = if self.active_file.is_some() {
            theme::PRIMARY_LIGHT
        } else {
            theme::TEXT_DIM
        };

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                file_text,
                Style::default().fg(file_color),
            ))),
            inner,
        );
    }

    fn render_stages(&self, frame: &mut Frame, area: Rect) {
        let constraints: Vec<Constraint> =
            (0..STAGE_COUNT).map(|_| Constraint::Length(3)).collect();

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        for (i, stage) in self.stages.iter().enumerate() {
            self.render_stage_row(frame, rows[i], stage);
        }
    }

    fn render_stage_row(&self, frame: &mut Frame, area: Rect, stage: &PipelineStage) {
        let status_label = stage.status.label();
        let count_label = format!("{}/{}", stage.processed, stage.total);
        let pct_label = format!("{:3.0}%", stage.progress * 100.0);

        // Build the title with icon + name
        let title = format!(" {} {} ", stage.icon, stage.name);

        // Build label shown inside the gauge: "  42%  12/30  Processing"
        let label = Line::from(vec![
            Span::styled(
                format!("  {pct_label}"),
                Style::default()
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(count_label, Style::default().fg(theme::TEXT_MUTED)),
            Span::raw("  "),
            Span::styled(
                status_label.to_string(),
                Style::default().fg(stage.status.color()),
            ),
        ]);

        let gauge = LineGauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if stage.status.is_idle() {
                        theme::TEXT_DIM
                    } else {
                        theme::PRIMARY
                    }))
                    .title(Span::styled(
                        title,
                        Style::default()
                            .fg(stage.status.color())
                            .add_modifier(Modifier::BOLD),
                    )),
            )
            .ratio(stage.progress)
            .label(label)
            .filled_style(Style::default().fg(stage.filled_color()))
            .unfilled_style(Style::default().fg(stage.unfilled_color()));

        frame.render_widget(gauge, area);
    }

    fn render_log(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_DIM))
            .title(Span::styled(
                " Log ",
                Style::default().fg(theme::TEXT_MUTED),
            ));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.log_messages.is_empty() {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "  No messages yet.",
                    Style::default().fg(theme::TEXT_DIM),
                ))),
                inner,
            );
            return;
        }

        let visible_height = inner.height as usize;
        let max_scroll = self.max_log_scroll(visible_height);
        let scroll = self.scroll.min(max_scroll);

        let lines: Vec<Line<'_>> = self
            .log_messages
            .iter()
            .enumerate()
            .map(|(i, msg)| {
                let idx_span = Span::styled(
                    format!("  {:>4} ", i + 1),
                    Style::default().fg(theme::TEXT_DIM),
                );
                let msg_span = Span::styled(msg.as_str(), Style::default().fg(theme::TEXT_MUTED));
                Line::from(vec![idx_span, msg_span])
            })
            .collect();

        frame.render_widget(Paragraph::new(lines).scroll((scroll as u16, 0)), inner);

        // Scrollbar for the log section
        if self.log_messages.len() > visible_height {
            let mut scrollbar_state = ScrollbarState::new(self.log_messages.len())
                .position(scroll)
                .viewport_content_length(visible_height);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .thumb_style(Style::default().fg(theme::PRIMARY))
                    .track_style(Style::default().fg(theme::TEXT_DIM)),
                inner,
                &mut scrollbar_state,
            );
        }

        // Key hints at bottom of log
        if inner.height > 1 {
            let hint_area = Rect {
                x: inner.x,
                y: inner.y + inner.height - 1,
                width: inner.width,
                height: 1,
            };
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "  [r] reset  [j/k] scroll  [c] clear log",
                    Style::default().fg(theme::TEXT_DIM),
                ))),
                hint_area,
            );
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state_all_stages_idle() {
        let state = IngestionState::new();
        assert_eq!(state.stages.len(), STAGE_COUNT);
        for stage in &state.stages {
            assert_eq!(stage.status, StageStatus::Idle);
            assert_eq!(stage.progress, 0.0);
            assert_eq!(stage.processed, 0);
            assert_eq!(stage.total, 0);
        }
        assert!(state.active_file.is_none());
        assert!(state.log_messages.is_empty());
        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn test_stage_names_and_icons() {
        let state = IngestionState::new();
        assert_eq!(state.stages[0].name, "Document Parsing");
        assert_eq!(state.stages[1].name, "Semantic Chunking");
        assert_eq!(state.stages[2].name, "Vector Generation");
        assert_eq!(state.stages[3].name, "Indexing");
        assert_eq!(state.stages[0].icon, ICON_PARSE);
        assert_eq!(state.stages[1].icon, ICON_CHUNK);
        assert_eq!(state.stages[2].icon, ICON_EMBED);
        assert_eq!(state.stages[3].icon, ICON_INDEX);
    }

    #[test]
    fn test_progress_event_updates_stage() {
        let mut state = IngestionState::new();
        state.apply_event(IngestionEvent::StageProgress {
            stage: 0,
            processed: 5,
            total: 10,
        });
        assert_eq!(state.stages[0].processed, 5);
        assert_eq!(state.stages[0].total, 10);
        assert!((state.stages[0].progress - 0.5).abs() < f64::EPSILON);
        assert_eq!(state.stages[0].status, StageStatus::Processing);
    }

    #[test]
    fn test_complete_event_sets_stage_done() {
        let mut state = IngestionState::new();
        state.stages[1].total = 20;
        state.apply_event(IngestionEvent::StageComplete { stage: 1 });
        assert_eq!(state.stages[1].status, StageStatus::Complete);
        assert!((state.stages[1].progress - 1.0).abs() < f64::EPSILON);
        assert_eq!(state.stages[1].processed, 20);
        assert!(!state.log_messages.is_empty());
    }

    #[test]
    fn test_error_event_sets_error_and_logs() {
        let mut state = IngestionState::new();
        state.apply_event(IngestionEvent::StageError {
            stage: 2,
            message: "embedding API timeout".into(),
        });
        assert!(
            matches!(state.stages[2].status, StageStatus::Error(ref m) if m.contains("timeout"))
        );
        assert!(state
            .log_messages
            .iter()
            .any(|m| m.contains("embedding API timeout")));
    }

    #[test]
    fn test_active_file_event() {
        let mut state = IngestionState::new();
        state.apply_event(IngestionEvent::ActiveFile("monsters.pdf".into()));
        assert_eq!(state.active_file.as_deref(), Some("monsters.pdf"));
        assert!(state
            .log_messages
            .iter()
            .any(|m| m.contains("monsters.pdf")));
    }

    #[test]
    fn test_reset_clears_all_stages() {
        let mut state = IngestionState::new();
        state.stages[0].progress = 0.75;
        state.stages[0].status = StageStatus::Processing;
        state.active_file = Some("test.pdf".into());
        state.apply_event(IngestionEvent::Reset);
        for stage in &state.stages {
            assert_eq!(stage.status, StageStatus::Idle);
            assert_eq!(stage.progress, 0.0);
        }
        assert!(state.active_file.is_none());
    }

    #[test]
    fn test_progress_clamping() {
        let mut stage = PipelineStage::new("Test", "T");
        stage.set_progress(1.5);
        assert!((stage.progress - 1.0).abs() < f64::EPSILON);
        stage.set_progress(-0.3);
        assert!(stage.progress.abs() < f64::EPSILON);
    }

    #[test]
    fn test_scroll_bounds() {
        let mut state = IngestionState::new();
        // Scroll up when already at 0 stays at 0
        state.scroll = 0;
        state.scroll = state.scroll.saturating_sub(1);
        assert_eq!(state.scroll, 0);

        // Scroll down with no messages stays at 0
        state.scroll_down();
        assert_eq!(state.scroll, 0);

        // Add messages and scroll
        for i in 0..20 {
            state.log_messages.push(format!("msg {i}"));
        }
        state.scroll_down();
        assert_eq!(state.scroll, 1);
    }

    #[test]
    fn test_max_log_scroll() {
        let mut state = IngestionState::new();
        for i in 0..30 {
            state.log_messages.push(format!("line {i}"));
        }
        // With 10 visible lines, max scroll = 30 - 10 = 20
        assert_eq!(state.max_log_scroll(10), 20);
        // With more visible lines than messages
        assert_eq!(state.max_log_scroll(50), 0);
    }

    #[test]
    fn test_log_event() {
        let mut state = IngestionState::new();
        state.apply_event(IngestionEvent::Log("custom log entry".into()));
        assert_eq!(state.log_messages.len(), 1);
        assert_eq!(state.log_messages[0], "custom log entry");
    }

    #[test]
    fn test_stage_status_labels_and_colors() {
        assert_eq!(StageStatus::Idle.label(), "Idle");
        assert_eq!(StageStatus::Processing.label(), "Processing");
        assert_eq!(StageStatus::Complete.label(), "Complete");
        assert_eq!(StageStatus::Error("x".into()).label(), "Error");

        assert_eq!(StageStatus::Idle.color(), theme::TEXT_DIM);
        assert_eq!(StageStatus::Processing.color(), theme::PRIMARY_LIGHT);
        assert_eq!(StageStatus::Complete.color(), theme::SUCCESS);
        assert_eq!(StageStatus::Error("x".into()).color(), theme::ERROR);
    }

    #[test]
    fn test_out_of_bounds_stage_event_is_safe() {
        let mut state = IngestionState::new();
        // Stage index 99 is out of bounds — should not panic
        state.apply_event(IngestionEvent::StageProgress {
            stage: 99,
            processed: 1,
            total: 1,
        });
        state.apply_event(IngestionEvent::StageComplete { stage: 99 });
        state.apply_event(IngestionEvent::StageError {
            stage: 99,
            message: "oops".into(),
        });
        // All stages remain idle
        for stage in &state.stages {
            assert_eq!(stage.status, StageStatus::Idle);
        }
    }

    #[test]
    fn test_poll_drains_channel() {
        let mut state = IngestionState::new();
        let tx = state.event_tx();
        tx.send(IngestionEvent::Log("one".into())).unwrap();
        tx.send(IngestionEvent::Log("two".into())).unwrap();
        tx.send(IngestionEvent::ActiveFile("f.pdf".into())).unwrap();
        state.poll();
        assert_eq!(state.log_messages.len(), 3); // "one", "two", and "Processing: f.pdf"
        assert_eq!(state.active_file.as_deref(), Some("f.pdf"));
    }

    #[test]
    fn test_zero_total_progress_is_zero() {
        let mut state = IngestionState::new();
        state.apply_event(IngestionEvent::StageProgress {
            stage: 0,
            processed: 0,
            total: 0,
        });
        assert!(state.stages[0].progress.abs() < f64::EPSILON);
    }
}
