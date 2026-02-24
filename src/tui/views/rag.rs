//! RAG enrichment workflow inspector view.
//!
//! Two-tab view for inspecting and configuring the RAG pipeline:
//! - **Inspector tab**: Shows retrieved chunks with scores, source info, and content preview.
//!   Selected chunk displays full content in a detail pane.
//! - **Config tab**: Adjust semantic ratio, max chunks, max bytes, content type presets,
//!   and the include-sources toggle.
//!
//! Keybinds: Tab (switch tabs), j/k (navigate), Enter (apply preset),
//! +/- (adjust values), r (refresh).

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::super::theme;
use crate::tui::services::Services;

// ── Constants ────────────────────────────────────────────────────────────────

const SEMANTIC_RATIO_STEP: f32 = 0.05;
const CHUNK_STEP: usize = 1;
const BYTES_STEP: usize = 100;

const DEFAULT_SEMANTIC_RATIO_RULES: f32 = 0.7;
const DEFAULT_SEMANTIC_RATIO_FICTION: f32 = 0.6;
const DEFAULT_SEMANTIC_RATIO_NOTES: f32 = 0.5;

const DEFAULT_MAX_CHUNKS_RULES: usize = 10;
const DEFAULT_MAX_CHUNKS_FICTION: usize = 6;
const DEFAULT_MAX_CHUNKS_NOTES: usize = 12;

const DEFAULT_MAX_BYTES_RULES: usize = 600;
const DEFAULT_MAX_BYTES_FICTION: usize = 1000;
const DEFAULT_MAX_BYTES_NOTES: usize = 800;

const CONTENT_PREVIEW_LINES: usize = 2;
const CONTENT_PREVIEW_WIDTH: usize = 80;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RagTab {
    Inspector,
    Config,
}

impl RagTab {
    fn toggle(self) -> Self {
        match self {
            Self::Inspector => Self::Config,
            Self::Config => Self::Inspector,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Inspector => "Inspector",
            Self::Config => "Config",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConfigField {
    SemanticRatio,
    MaxChunks,
    MaxBytes,
    IncludeSources,
}

impl ConfigField {
    fn next(self) -> Self {
        match self {
            Self::SemanticRatio => Self::MaxChunks,
            Self::MaxChunks => Self::MaxBytes,
            Self::MaxBytes => Self::IncludeSources,
            Self::IncludeSources => Self::SemanticRatio,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::SemanticRatio => Self::IncludeSources,
            Self::MaxChunks => Self::SemanticRatio,
            Self::MaxBytes => Self::MaxChunks,
            Self::IncludeSources => Self::MaxBytes,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::SemanticRatio => "Semantic Ratio",
            Self::MaxChunks => "Max Context Chunks",
            Self::MaxBytes => "Max Context Bytes",
            Self::IncludeSources => "Include Sources",
        }
    }

    const ALL: [ConfigField; 4] = [
        Self::SemanticRatio,
        Self::MaxChunks,
        Self::MaxBytes,
        Self::IncludeSources,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ContentPreset {
    Rules,
    Fiction,
    Notes,
}

impl ContentPreset {
    fn label(self) -> &'static str {
        match self {
            Self::Rules => "Rules",
            Self::Fiction => "Fiction",
            Self::Notes => "Notes",
        }
    }

    fn semantic_ratio(self) -> f32 {
        match self {
            Self::Rules => DEFAULT_SEMANTIC_RATIO_RULES,
            Self::Fiction => DEFAULT_SEMANTIC_RATIO_FICTION,
            Self::Notes => DEFAULT_SEMANTIC_RATIO_NOTES,
        }
    }

    fn max_chunks(self) -> usize {
        match self {
            Self::Rules => DEFAULT_MAX_CHUNKS_RULES,
            Self::Fiction => DEFAULT_MAX_CHUNKS_FICTION,
            Self::Notes => DEFAULT_MAX_CHUNKS_NOTES,
        }
    }

    fn max_bytes(self) -> usize {
        match self {
            Self::Rules => DEFAULT_MAX_BYTES_RULES,
            Self::Fiction => DEFAULT_MAX_BYTES_FICTION,
            Self::Notes => DEFAULT_MAX_BYTES_NOTES,
        }
    }

    const ALL: [ContentPreset; 3] = [Self::Rules, Self::Fiction, Self::Notes];
}

#[derive(Clone, Debug)]
struct RagChunk {
    source: String,
    page: Option<u32>,
    score: f64,
    content: String,
}

// ── State ────────────────────────────────────────────────────────────────────

pub struct RagViewState {
    tab: RagTab,
    chunks: Vec<RagChunk>,
    selected_chunk: usize,
    config_field: ConfigField,
    semantic_ratio: f32,
    max_chunks: usize,
    max_bytes: usize,
    include_sources: bool,
    scroll: usize,
    selected_preset: usize,
}

impl RagViewState {
    pub fn new() -> Self {
        Self {
            tab: RagTab::Inspector,
            chunks: Vec::new(),
            selected_chunk: 0,
            config_field: ConfigField::SemanticRatio,
            semantic_ratio: DEFAULT_SEMANTIC_RATIO_RULES,
            max_chunks: 8,
            max_bytes: 4000,
            include_sources: true,
            scroll: 0,
            selected_preset: 0,
        }
    }

    pub fn load(&mut self, _services: &Services) {
        // Will be wired to actual RAG pipeline later.
        // For now, the inspector shows "No active query".
    }

    pub fn poll(&mut self) {
        // No async data to poll yet.
    }

    // ── Input ────────────────────────────────────────────────────────────

    pub fn handle_input(&mut self, event: &Event, _services: &Services) -> bool {
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
            // Tab switching
            (KeyModifiers::NONE, KeyCode::Tab) => {
                self.tab = self.tab.toggle();
                true
            }

            // Navigation
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                self.navigate_down();
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                self.navigate_up();
                true
            }

            // Value adjustment
            (KeyModifiers::NONE, KeyCode::Char('+') | KeyCode::Char('=')) => {
                self.adjust_value(true);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('-')) => {
                self.adjust_value(false);
                true
            }

            // Apply preset (config tab, Enter)
            (KeyModifiers::NONE, KeyCode::Enter) if self.tab == RagTab::Config => {
                self.apply_selected_preset();
                true
            }

            // Refresh
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                // Placeholder for future refresh logic
                true
            }

            _ => false,
        }
    }

    fn navigate_down(&mut self) {
        match self.tab {
            RagTab::Inspector => {
                if !self.chunks.is_empty() {
                    self.selected_chunk =
                        (self.selected_chunk + 1).min(self.chunks.len().saturating_sub(1));
                    self.scroll = 0;
                }
            }
            RagTab::Config => {
                self.config_field = self.config_field.next();
            }
        }
    }

    fn navigate_up(&mut self) {
        match self.tab {
            RagTab::Inspector => {
                self.selected_chunk = self.selected_chunk.saturating_sub(1);
                self.scroll = 0;
            }
            RagTab::Config => {
                self.config_field = self.config_field.prev();
            }
        }
    }

    fn adjust_value(&mut self, increase: bool) {
        if self.tab != RagTab::Config {
            return;
        }

        match self.config_field {
            ConfigField::SemanticRatio => {
                if increase {
                    self.semantic_ratio =
                        (self.semantic_ratio + SEMANTIC_RATIO_STEP).min(1.0);
                } else {
                    self.semantic_ratio =
                        (self.semantic_ratio - SEMANTIC_RATIO_STEP).max(0.0);
                }
                // Round to avoid floating point drift
                self.semantic_ratio = (self.semantic_ratio * 100.0).round() / 100.0;
            }
            ConfigField::MaxChunks => {
                if increase {
                    self.max_chunks = self.max_chunks.saturating_add(CHUNK_STEP).min(50);
                } else {
                    self.max_chunks = self.max_chunks.saturating_sub(CHUNK_STEP).max(1);
                }
            }
            ConfigField::MaxBytes => {
                if increase {
                    self.max_bytes = self.max_bytes.saturating_add(BYTES_STEP).min(20000);
                } else {
                    self.max_bytes = self.max_bytes.saturating_sub(BYTES_STEP).max(100);
                }
            }
            ConfigField::IncludeSources => {
                self.include_sources = !self.include_sources;
            }
        }
    }

    fn apply_selected_preset(&mut self) {
        // When on IncludeSources field, Enter toggles the boolean
        if self.config_field == ConfigField::IncludeSources {
            self.include_sources = !self.include_sources;
            return;
        }

        // Cycle through presets
        let preset = ContentPreset::ALL[self.selected_preset % ContentPreset::ALL.len()];
        self.semantic_ratio = preset.semantic_ratio();
        self.max_chunks = preset.max_chunks();
        self.max_bytes = preset.max_bytes();
        self.selected_preset = (self.selected_preset + 1) % ContentPreset::ALL.len();
    }

    // ── Rendering ────────────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" RAG Inspector ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_MUTED));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Tab bar + content layout
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1)])
            .split(inner);

        self.render_tab_bar(frame, layout[0]);

        match self.tab {
            RagTab::Inspector => self.render_inspector(frame, layout[1]),
            RagTab::Config => self.render_config(frame, layout[1]),
        }
    }

    fn render_tab_bar(&self, frame: &mut Frame, area: Rect) {
        let tabs = [RagTab::Inspector, RagTab::Config];
        let mut spans = vec![Span::raw("  ")];

        for (i, tab) in tabs.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" | ", Style::default().fg(theme::TEXT_DIM)));
            }

            let style = if *tab == self.tab {
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT_MUTED)
            };

            spans.push(Span::styled(tab.label(), style));
        }

        spans.push(Span::styled(
            "  (Tab to switch)",
            Style::default().fg(theme::TEXT_DIM),
        ));

        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn render_inspector(&self, frame: &mut Frame, area: Rect) {
        if self.chunks.is_empty() {
            let lines = vec![
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "No active query",
                        Style::default()
                            .fg(theme::TEXT_MUTED)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]),
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Send a chat message with RAG enabled to see retrieved chunks here.",
                        Style::default().fg(theme::TEXT_DIM),
                    ),
                ]),
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled("j/k", Style::default().fg(theme::TEXT_MUTED)),
                    Span::raw(":navigate "),
                    Span::styled("r", Style::default().fg(theme::TEXT_MUTED)),
                    Span::raw(":refresh "),
                    Span::styled("Tab", Style::default().fg(theme::TEXT_MUTED)),
                    Span::raw(":config"),
                ]),
            ];
            frame.render_widget(Paragraph::new(lines), area);
            return;
        }

        // Split into list (left) and detail (right)
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(area);

        self.render_chunk_list(frame, layout[0]);
        self.render_chunk_detail(frame, layout[1]);
    }

    fn render_chunk_list(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Chunks ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_DIM));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines = Vec::new();

        for (i, chunk) in self.chunks.iter().enumerate() {
            let is_selected = i == self.selected_chunk;

            // Indicator
            let indicator = if is_selected { ">" } else { " " };
            let indicator_style = if is_selected {
                Style::default().fg(theme::ACCENT)
            } else {
                Style::default()
            };

            // Score coloring
            let score_color = score_color(chunk.score);
            let score_str = format!("{:.2}", chunk.score);

            // Source with page
            let source_str = match chunk.page {
                Some(p) => format!("{} p.{}", chunk.source, p),
                None => chunk.source.clone(),
            };

            // Row 1: indicator + source + score
            let row_style = if is_selected {
                Style::default()
                    .fg(theme::PRIMARY_LIGHT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };

            lines.push(Line::from(vec![
                Span::styled(format!(" {indicator} "), indicator_style),
                Span::styled(truncate_str(&source_str, 24), row_style),
                Span::raw(" "),
                Span::styled(score_str, Style::default().fg(score_color)),
            ]));

            // Row 2: content preview (truncated to CONTENT_PREVIEW_LINES lines)
            let preview = content_preview(&chunk.content, CONTENT_PREVIEW_WIDTH);
            for preview_line in preview.iter().take(CONTENT_PREVIEW_LINES) {
                lines.push(Line::from(vec![
                    Span::raw("     "),
                    Span::styled(
                        preview_line.clone(),
                        Style::default().fg(theme::TEXT_DIM),
                    ),
                ]));
            }

            // Separator
            if i < self.chunks.len() - 1 {
                lines.push(Line::from(Span::styled(
                    format!("   {}", "─".repeat(inner.width.saturating_sub(4) as usize)),
                    Style::default().fg(theme::TEXT_DIM),
                )));
            }
        }

        let para = Paragraph::new(lines).scroll((0, 0));
        frame.render_widget(para, inner);
    }

    fn render_chunk_detail(&self, frame: &mut Frame, area: Rect) {
        let chunk = match self.chunks.get(self.selected_chunk) {
            Some(c) => c,
            None => return,
        };

        let block = Block::default()
            .title(" Detail ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::PRIMARY_DARK));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines = Vec::new();

        // Header
        let source_str = match chunk.page {
            Some(p) => format!("  Source: {} (p.{})", chunk.source, p),
            None => format!("  Source: {}", chunk.source),
        };
        lines.push(Line::from(Span::styled(
            source_str,
            Style::default()
                .fg(theme::PRIMARY_LIGHT)
                .add_modifier(Modifier::BOLD),
        )));

        let score_color = score_color(chunk.score);
        lines.push(Line::from(vec![
            Span::styled("  Score: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(format!("{:.4}", chunk.score), Style::default().fg(score_color)),
        ]));

        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(inner.width.saturating_sub(3) as usize)),
            Style::default().fg(theme::TEXT_DIM),
        )));

        // Full content
        for content_line in chunk.content.lines() {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(content_line.to_string(), Style::default().fg(theme::TEXT)),
            ]));
        }

        let para = Paragraph::new(lines)
            .scroll((self.scroll as u16, 0))
            .wrap(Wrap { trim: false });
        frame.render_widget(para, inner);
    }

    fn render_config(&self, frame: &mut Frame, area: Rect) {
        let mut lines = Vec::with_capacity(30);

        lines.push(Line::raw(""));

        // Section: Search Parameters
        lines.push(Line::from(Span::styled(
            "  Search Parameters",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(50)),
            Style::default().fg(theme::TEXT_MUTED),
        )));
        lines.push(Line::raw(""));

        // Semantic Ratio with gauge bar
        let ratio_focused = self.config_field == ConfigField::SemanticRatio;
        let ratio_indicator = if ratio_focused { ">" } else { " " };
        let ratio_label_style = field_label_style(ratio_focused);

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {ratio_indicator} "),
                Style::default().fg(theme::ACCENT),
            ),
            Span::styled(ConfigField::SemanticRatio.label(), ratio_label_style),
            Span::styled(
                format!("  {:.2}", self.semantic_ratio),
                Style::default().fg(theme::TEXT),
            ),
        ]));

        // Gauge bar
        let gauge_width = 40;
        let filled = ((self.semantic_ratio * gauge_width as f32) as usize).min(gauge_width);
        let empty = gauge_width - filled;

        let keyword_label = format!("KW {:.0}%", (1.0 - self.semantic_ratio) * 100.0);
        let semantic_label = format!("SEM {:.0}%", self.semantic_ratio * 100.0);

        lines.push(Line::from(vec![
            Span::raw("      "),
            Span::styled(
                format!("{keyword_label:<8}"),
                Style::default().fg(theme::INFO),
            ),
            Span::styled("█".repeat(filled), Style::default().fg(theme::PRIMARY)),
            Span::styled(
                "░".repeat(empty),
                Style::default().fg(theme::TEXT_DIM),
            ),
            Span::styled(
                format!(" {semantic_label}"),
                Style::default().fg(theme::PRIMARY_LIGHT),
            ),
        ]));
        lines.push(Line::raw(""));

        // Max Chunks
        let chunks_focused = self.config_field == ConfigField::MaxChunks;
        let chunks_indicator = if chunks_focused { ">" } else { " " };
        let chunks_label_style = field_label_style(chunks_focused);

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {chunks_indicator} "),
                Style::default().fg(theme::ACCENT),
            ),
            Span::styled(ConfigField::MaxChunks.label(), chunks_label_style),
            Span::styled(
                format!("  {}", self.max_chunks),
                Style::default().fg(theme::TEXT),
            ),
        ]));
        lines.push(Line::raw(""));

        // Max Bytes
        let bytes_focused = self.config_field == ConfigField::MaxBytes;
        let bytes_indicator = if bytes_focused { ">" } else { " " };
        let bytes_label_style = field_label_style(bytes_focused);

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {bytes_indicator} "),
                Style::default().fg(theme::ACCENT),
            ),
            Span::styled(ConfigField::MaxBytes.label(), bytes_label_style),
            Span::styled(
                format!("  {} bytes", self.max_bytes),
                Style::default().fg(theme::TEXT),
            ),
        ]));
        lines.push(Line::raw(""));

        // Include Sources toggle
        let sources_focused = self.config_field == ConfigField::IncludeSources;
        let sources_indicator = if sources_focused { ">" } else { " " };
        let sources_label_style = field_label_style(sources_focused);
        let toggle_str = if self.include_sources {
            "[x] On"
        } else {
            "[ ] Off"
        };
        let toggle_color = if self.include_sources {
            theme::SUCCESS
        } else {
            theme::TEXT_MUTED
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {sources_indicator} "),
                Style::default().fg(theme::ACCENT),
            ),
            Span::styled(ConfigField::IncludeSources.label(), sources_label_style),
            Span::raw("  "),
            Span::styled(toggle_str, Style::default().fg(toggle_color)),
        ]));

        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(50)),
            Style::default().fg(theme::TEXT_MUTED),
        )));
        lines.push(Line::raw(""));

        // Content Type Presets
        lines.push(Line::from(Span::styled(
            "  Content Type Presets",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::raw(""));

        let next_preset = self.selected_preset % ContentPreset::ALL.len();
        let mut preset_spans = vec![Span::raw("  ")];
        for (i, preset) in ContentPreset::ALL.iter().enumerate() {
            if i > 0 {
                preset_spans.push(Span::raw("  "));
            }

            let style = if i == next_preset {
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT_MUTED)
            };

            preset_spans.push(Span::styled(
                format!("[{}]", preset.label()),
                style,
            ));
        }
        lines.push(Line::from(preset_spans));

        // Show what the next preset would apply
        let next = ContentPreset::ALL[next_preset];
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!(
                    "Enter: apply {} (ratio={:.1}, chunks={}, bytes={})",
                    next.label(),
                    next.semantic_ratio(),
                    next.max_chunks(),
                    next.max_bytes(),
                ),
                Style::default().fg(theme::TEXT_DIM),
            ),
        ]));

        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(50)),
            Style::default().fg(theme::TEXT_MUTED),
        )));

        // Footer keybinds
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("j/k", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":navigate "),
            Span::styled("+/-", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":adjust "),
            Span::styled("Enter", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":apply preset "),
            Span::styled("Tab", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":inspector"),
        ]));

        let para = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::NONE),
            )
            .alignment(Alignment::Left);

        frame.render_widget(para, area);
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Color for a relevance score: green > 0.8, yellow > 0.5, red otherwise.
fn score_color(score: f64) -> ratatui::style::Color {
    if score > 0.8 {
        theme::SUCCESS
    } else if score > 0.5 {
        theme::WARNING
    } else {
        theme::ERROR
    }
}

/// Style for a config field label (bold + brighter when focused).
fn field_label_style(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(theme::PRIMARY_LIGHT)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::TEXT_MUTED)
    }
}

/// Truncate a string to fit within `max_width`, appending "..." if needed.
fn truncate_str(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        s.to_string()
    } else if max_width <= 3 {
        ".".repeat(max_width)
    } else {
        format!("{}...", &s[..max_width - 3])
    }
}

/// Produce a multi-line preview of content, each line capped to `max_width`.
fn content_preview(content: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        lines.push(truncate_str(trimmed, max_width));
        if lines.len() >= CONTENT_PREVIEW_LINES {
            break;
        }
    }
    if lines.is_empty() {
        lines.push("(empty)".to_string());
    }
    lines
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rag_view_state_new_defaults() {
        let state = RagViewState::new();

        assert_eq!(state.tab, RagTab::Inspector);
        assert!(state.chunks.is_empty());
        assert_eq!(state.selected_chunk, 0);
        assert_eq!(state.config_field, ConfigField::SemanticRatio);
        assert!((state.semantic_ratio - DEFAULT_SEMANTIC_RATIO_RULES).abs() < f32::EPSILON);
        assert_eq!(state.max_chunks, 8);
        assert_eq!(state.max_bytes, 4000);
        assert!(state.include_sources);
        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn test_tab_toggle() {
        assert_eq!(RagTab::Inspector.toggle(), RagTab::Config);
        assert_eq!(RagTab::Config.toggle(), RagTab::Inspector);
        assert_eq!(RagTab::Inspector.label(), "Inspector");
        assert_eq!(RagTab::Config.label(), "Config");
    }

    #[test]
    fn test_config_field_cycle() {
        // Forward cycle
        let mut field = ConfigField::SemanticRatio;
        field = field.next();
        assert_eq!(field, ConfigField::MaxChunks);
        field = field.next();
        assert_eq!(field, ConfigField::MaxBytes);
        field = field.next();
        assert_eq!(field, ConfigField::IncludeSources);
        field = field.next();
        assert_eq!(field, ConfigField::SemanticRatio);

        // Reverse cycle
        field = ConfigField::SemanticRatio;
        field = field.prev();
        assert_eq!(field, ConfigField::IncludeSources);
        field = field.prev();
        assert_eq!(field, ConfigField::MaxBytes);
        field = field.prev();
        assert_eq!(field, ConfigField::MaxChunks);
        field = field.prev();
        assert_eq!(field, ConfigField::SemanticRatio);
    }

    #[test]
    fn test_semantic_ratio_adjustment() {
        let mut state = RagViewState::new();
        state.tab = RagTab::Config;
        state.config_field = ConfigField::SemanticRatio;
        state.semantic_ratio = 0.50;

        // Increase by 0.05
        state.adjust_value(true);
        assert!((state.semantic_ratio - 0.55).abs() < 0.001);

        // Decrease by 0.05
        state.adjust_value(false);
        assert!((state.semantic_ratio - 0.50).abs() < 0.001);

        // Clamp at 1.0
        state.semantic_ratio = 0.98;
        state.adjust_value(true);
        assert!((state.semantic_ratio - 1.0).abs() < f32::EPSILON);

        // Clamp at 0.0
        state.semantic_ratio = 0.02;
        state.adjust_value(false);
        assert!((state.semantic_ratio - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_max_chunks_adjustment() {
        let mut state = RagViewState::new();
        state.tab = RagTab::Config;
        state.config_field = ConfigField::MaxChunks;
        state.max_chunks = 10;

        state.adjust_value(true);
        assert_eq!(state.max_chunks, 11);

        state.adjust_value(false);
        assert_eq!(state.max_chunks, 10);

        // Clamp at minimum 1
        state.max_chunks = 1;
        state.adjust_value(false);
        assert_eq!(state.max_chunks, 1);

        // Clamp at maximum 50
        state.max_chunks = 50;
        state.adjust_value(true);
        assert_eq!(state.max_chunks, 50);
    }

    #[test]
    fn test_max_bytes_adjustment() {
        let mut state = RagViewState::new();
        state.tab = RagTab::Config;
        state.config_field = ConfigField::MaxBytes;
        state.max_bytes = 4000;

        state.adjust_value(true);
        assert_eq!(state.max_bytes, 4100);

        state.adjust_value(false);
        assert_eq!(state.max_bytes, 4000);

        // Clamp at minimum 100
        state.max_bytes = 100;
        state.adjust_value(false);
        assert_eq!(state.max_bytes, 100);

        // Clamp at maximum 20000
        state.max_bytes = 20000;
        state.adjust_value(true);
        assert_eq!(state.max_bytes, 20000);
    }

    #[test]
    fn test_include_sources_toggle() {
        let mut state = RagViewState::new();
        state.tab = RagTab::Config;
        state.config_field = ConfigField::IncludeSources;

        assert!(state.include_sources);
        state.adjust_value(true); // Either direction toggles
        assert!(!state.include_sources);
        state.adjust_value(false);
        assert!(state.include_sources);
    }

    #[test]
    fn test_content_preset_values() {
        let rules = ContentPreset::Rules;
        assert!((rules.semantic_ratio() - 0.7).abs() < f32::EPSILON);
        assert_eq!(rules.max_chunks(), 10);
        assert_eq!(rules.max_bytes(), 600);
        assert_eq!(rules.label(), "Rules");

        let fiction = ContentPreset::Fiction;
        assert!((fiction.semantic_ratio() - 0.6).abs() < f32::EPSILON);
        assert_eq!(fiction.max_chunks(), 6);
        assert_eq!(fiction.max_bytes(), 1000);

        let notes = ContentPreset::Notes;
        assert!((notes.semantic_ratio() - 0.5).abs() < f32::EPSILON);
        assert_eq!(notes.max_chunks(), 12);
        assert_eq!(notes.max_bytes(), 800);
    }

    #[test]
    fn test_apply_preset_cycles() {
        let mut state = RagViewState::new();
        state.tab = RagTab::Config;
        state.config_field = ConfigField::SemanticRatio; // Not IncludeSources

        // Apply Rules (first preset)
        state.selected_preset = 0;
        state.apply_selected_preset();
        assert!((state.semantic_ratio - 0.7).abs() < f32::EPSILON);
        assert_eq!(state.max_chunks, 10);
        assert_eq!(state.max_bytes, 600);
        assert_eq!(state.selected_preset, 1);

        // Apply Fiction (second preset)
        state.apply_selected_preset();
        assert!((state.semantic_ratio - 0.6).abs() < f32::EPSILON);
        assert_eq!(state.max_chunks, 6);
        assert_eq!(state.max_bytes, 1000);
        assert_eq!(state.selected_preset, 2);

        // Apply Notes (third preset)
        state.apply_selected_preset();
        assert!((state.semantic_ratio - 0.5).abs() < f32::EPSILON);
        assert_eq!(state.max_chunks, 12);
        assert_eq!(state.max_bytes, 800);
        assert_eq!(state.selected_preset, 0); // Wraps around
    }

    #[test]
    fn test_score_color_thresholds() {
        assert_eq!(score_color(0.9), theme::SUCCESS);
        assert_eq!(score_color(0.81), theme::SUCCESS);
        assert_eq!(score_color(0.8), theme::WARNING); // 0.8 is not > 0.8
        assert_eq!(score_color(0.6), theme::WARNING);
        assert_eq!(score_color(0.51), theme::WARNING);
        assert_eq!(score_color(0.5), theme::ERROR); // 0.5 is not > 0.5
        assert_eq!(score_color(0.3), theme::ERROR);
        assert_eq!(score_color(0.0), theme::ERROR);
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world", 8), "hello...");
        assert_eq!(truncate_str("hi", 2), "hi");
        assert_eq!(truncate_str("hello", 3), "...");
        assert_eq!(truncate_str("hello", 4), "h...");
    }

    #[test]
    fn test_content_preview() {
        let content = "First line of content\nSecond line\nThird line\n";
        let preview = content_preview(content, 80);
        assert_eq!(preview.len(), 2);
        assert_eq!(preview[0], "First line of content");
        assert_eq!(preview[1], "Second line");

        // Empty content
        let preview = content_preview("", 80);
        assert_eq!(preview, vec!["(empty)"]);

        // Whitespace-only content
        let preview = content_preview("   \n   \n", 80);
        assert_eq!(preview, vec!["(empty)"]);

        // Long line gets truncated
        let long = "A".repeat(100);
        let preview = content_preview(&long, 20);
        assert_eq!(preview[0], format!("{}...", "A".repeat(17)));
    }

    #[test]
    fn test_navigate_inspector_empty() {
        let mut state = RagViewState::new();
        state.tab = RagTab::Inspector;

        // Navigation on empty chunk list should not panic
        state.navigate_down();
        assert_eq!(state.selected_chunk, 0);

        state.navigate_up();
        assert_eq!(state.selected_chunk, 0);
    }

    #[test]
    fn test_navigate_inspector_with_chunks() {
        let mut state = RagViewState::new();
        state.tab = RagTab::Inspector;
        state.chunks = vec![
            RagChunk {
                source: "phb-2024".to_string(),
                page: Some(100),
                score: 0.9,
                content: "Chunk 1".to_string(),
            },
            RagChunk {
                source: "phb-2024".to_string(),
                page: Some(200),
                score: 0.7,
                content: "Chunk 2".to_string(),
            },
            RagChunk {
                source: "dmg-2024".to_string(),
                page: None,
                score: 0.4,
                content: "Chunk 3".to_string(),
            },
        ];

        state.navigate_down();
        assert_eq!(state.selected_chunk, 1);
        state.navigate_down();
        assert_eq!(state.selected_chunk, 2);
        state.navigate_down(); // Already at end
        assert_eq!(state.selected_chunk, 2);

        state.navigate_up();
        assert_eq!(state.selected_chunk, 1);
        state.navigate_up();
        assert_eq!(state.selected_chunk, 0);
        state.navigate_up(); // Already at start
        assert_eq!(state.selected_chunk, 0);
    }

    #[test]
    fn test_adjust_value_ignored_on_inspector_tab() {
        let mut state = RagViewState::new();
        state.tab = RagTab::Inspector;
        let original_ratio = state.semantic_ratio;

        state.adjust_value(true);
        assert!((state.semantic_ratio - original_ratio).abs() < f32::EPSILON);
    }

    #[test]
    fn test_config_field_labels() {
        assert_eq!(ConfigField::SemanticRatio.label(), "Semantic Ratio");
        assert_eq!(ConfigField::MaxChunks.label(), "Max Context Chunks");
        assert_eq!(ConfigField::MaxBytes.label(), "Max Context Bytes");
        assert_eq!(ConfigField::IncludeSources.label(), "Include Sources");
    }
}
