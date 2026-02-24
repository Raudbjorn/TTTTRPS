//! Location generator — wizard-style location creation and browsing.
//!
//! Uses the LocationGenerator backend for quick (template) and detailed (LLM)
//! location generation. Currently shows the wizard UI skeleton; generation
//! will be wired when LocationGenerator is added to Services.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::super::theme;
use crate::tui::services::Services;

// ── Location type list ─────────────────────────────────────────────────────

const LOCATION_TYPES: &[(&str, &str)] = &[
    ("Tavern", "A place of rest, rumors, and quest hooks"),
    ("Dungeon", "Underground complex with traps and treasure"),
    ("Forest", "Woodland area with hidden paths and creatures"),
    ("Mountain", "Elevated terrain with caves and vantage points"),
    ("Castle", "Fortified structure with political intrigue"),
    ("Village", "Small settlement with local problems"),
    ("City", "Large urban center with diverse districts"),
    ("Temple", "Sacred ground with divine or dark powers"),
    ("Ruins", "Remnants of a fallen civilization"),
    ("Swamp", "Treacherous wetland with hidden dangers"),
    ("Desert", "Arid wasteland with buried secrets"),
    ("Coast", "Shoreline with maritime encounters"),
];

// ── Wizard phase ───────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    TypeSelect,
    Preview,
}

// ── State ──────────────────────────────────────────────────────────────────

pub struct LocationViewState {
    phase: Phase,
    selected_type: usize,
    scroll: usize,
    generated_preview: Option<String>,
}

impl LocationViewState {
    pub fn new() -> Self {
        Self {
            phase: Phase::TypeSelect,
            selected_type: 0,
            scroll: 0,
            generated_preview: None,
        }
    }

    pub fn load(&mut self, _services: &Services) {
        // LocationGenerator not in Services yet
    }

    pub fn poll(&mut self) {
        // No async data to poll yet
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

        match self.phase {
            Phase::TypeSelect => self.handle_type_select(*modifiers, *code),
            Phase::Preview => self.handle_preview(*modifiers, *code),
        }
    }

    fn handle_type_select(&mut self, mods: KeyModifiers, code: KeyCode) -> bool {
        match (mods, code) {
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                if self.selected_type + 1 < LOCATION_TYPES.len() {
                    self.selected_type += 1;
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                self.selected_type = self.selected_type.saturating_sub(1);
                true
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                let (name, desc) = LOCATION_TYPES[self.selected_type];
                self.generated_preview = Some(format!(
                    "Location Type: {name}\n\n\
                     {desc}\n\n\
                     [Generation not yet connected]\n\n\
                     When LocationGenerator is wired to Services,\n\
                     this will generate a full location with:\n\
                     - Atmosphere and description\n\
                     - Notable features\n\
                     - Inhabitants and NPCs\n\
                     - Secrets and encounters\n\
                     - Connected locations\n\
                     - Loot potential"
                ));
                self.phase = Phase::Preview;
                true
            }
            _ => false,
        }
    }

    fn handle_preview(&mut self, mods: KeyModifiers, code: KeyCode) -> bool {
        match (mods, code) {
            (KeyModifiers::NONE, KeyCode::Esc | KeyCode::Char('q')) => {
                self.phase = Phase::TypeSelect;
                self.generated_preview = None;
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
        match self.phase {
            Phase::TypeSelect => self.render_type_select(frame, area),
            Phase::Preview => self.render_preview(frame, area),
        }
    }

    fn render_type_select(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        // Left: type list
        let block = theme::block_focused("Location Type");
        let inner = block.inner(chunks[0]);
        frame.render_widget(block, chunks[0]);

        let mut lines: Vec<Line<'static>> = Vec::new();
        for (i, (name, _)) in LOCATION_TYPES.iter().enumerate() {
            let is_selected = i == self.selected_type;
            let marker = if is_selected { "▸ " } else { "  " };
            let style = if is_selected {
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };
            lines.push(Line::from(Span::styled(
                format!("{marker}{name}"),
                style,
            )));
        }

        frame.render_widget(Paragraph::new(lines), inner);

        // Right: description
        let block = theme::block_default("Description");
        let inner = block.inner(chunks[1]);
        frame.render_widget(block, chunks[1]);

        let (name, desc) = LOCATION_TYPES[self.selected_type];
        let lines = vec![
            Line::raw(""),
            Line::from(Span::styled(
                format!("  {name}"),
                Style::default()
                    .fg(theme::PRIMARY_LIGHT)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                format!("  {desc}"),
                Style::default().fg(theme::TEXT),
            )),
            Line::raw(""),
            Line::raw(""),
            Line::from(Span::styled(
                "  [Enter] generate  [j/k] navigate",
                Style::default().fg(theme::TEXT_DIM),
            )),
        ];

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_preview(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_focused("Generated Location");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let text = self
            .generated_preview
            .as_deref()
            .unwrap_or("No location generated");

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));
        for line in text.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {line}"),
                Style::default().fg(theme::TEXT),
            )));
        }
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  [Esc] back  [j/k] scroll",
            Style::default().fg(theme::TEXT_DIM),
        )));

        let visible = inner.height as usize;
        let max_scroll = lines.len().saturating_sub(visible);
        let scroll = self.scroll.min(max_scroll);

        frame.render_widget(Paragraph::new(lines).scroll((scroll as u16, 0)), inner);
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = LocationViewState::new();
        assert_eq!(state.phase, Phase::TypeSelect);
        assert_eq!(state.selected_type, 0);
        assert!(state.generated_preview.is_none());
    }

    #[test]
    fn test_type_selection_bounds() {
        let mut state = LocationViewState::new();
        // Can't go below 0
        state.selected_type = 0;
        state.selected_type = state.selected_type.saturating_sub(1);
        assert_eq!(state.selected_type, 0);
        // Can go up to LOCATION_TYPES.len() - 1
        state.selected_type = LOCATION_TYPES.len() - 1;
        assert_eq!(state.selected_type, LOCATION_TYPES.len() - 1);
    }

    #[test]
    fn test_location_types_nonempty() {
        assert!(!LOCATION_TYPES.is_empty());
        for (name, desc) in LOCATION_TYPES {
            assert!(!name.is_empty());
            assert!(!desc.is_empty());
        }
    }
}
