use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::core::personality_base::{create_preset_personality, PersonalityProfile};
use crate::tui::services::Services;
use crate::tui::theme;

/// All known preset IDs — matched against `create_preset_personality()`.
const PRESET_IDS: &[&str] = &[
    "tavern_keeper",
    "grumpy_merchant",
    "village_elder",
    "corrupt_guard",
    "mystic_seer",
    "eberron_artificer",
];

struct PresetEntry {
    id: &'static str,
    profile: Option<PersonalityProfile>,
}

pub struct PresetBrowserState {
    pub list_state: ListState,
    entries: Vec<PresetEntry>,
}

impl PresetBrowserState {
    pub fn new() -> Self {
        let entries: Vec<PresetEntry> = PRESET_IDS
            .iter()
            .map(|id| PresetEntry {
                id,
                profile: create_preset_personality(id),
            })
            .collect();

        let mut list_state = ListState::default();
        if !entries.is_empty() {
            list_state.select(Some(0));
        }

        Self {
            list_state,
            entries,
        }
    }

    fn current_entry(&self) -> Option<&PresetEntry> {
        self.list_state.selected().and_then(|i| self.entries.get(i))
    }

    pub fn handle_input(&mut self, event: &Event, _services: &Services) -> bool {
        let Event::Key(key) = event else {
            return false;
        };
        if key.kind != KeyEventKind::Press {
            return false;
        }

        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                self.select_next();
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                self.select_prev();
                true
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                // Future: clone preset into editor or save to active profile
                true
            }
            _ => false,
        }
    }

    fn select_next(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.entries.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn select_prev(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.entries.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(area);

        self.render_list(frame, chunks[0]);
        self.render_detail(frame, chunks[1]);
    }

    fn render_list(&mut self, frame: &mut Frame, area: Rect) {
        let block = theme::block_focused("Personality Presets");

        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| {
                let name = entry
                    .profile
                    .as_ref()
                    .map(|p| p.name.as_str())
                    .unwrap_or(entry.id);
                let available = if entry.profile.is_some() {
                    ""
                } else {
                    " (template)"
                };
                ListItem::new(format!("{name}{available}"))
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("\u{25b8} ");

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn render_detail(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_default("Detail");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(entry) = self.current_entry() else {
            return;
        };

        let Some(ref profile) = entry.profile else {
            frame.render_widget(
                Paragraph::new(vec![
                    Line::raw(""),
                    Line::from(Span::styled(
                        format!("  {} — preset template (not yet implemented)", entry.id),
                        Style::default().fg(theme::TEXT_MUTED),
                    )),
                ]),
                inner,
            );
            return;
        };

        let mut lines: Vec<Line<'static>> = Vec::new();

        // Header
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", profile.name),
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            format!(
                "  Formality: {}/10  |  Tags: {}",
                profile.speech_patterns.formality,
                profile.tags.join(", ")
            ),
            Style::default().fg(theme::TEXT_MUTED),
        )));

        // Traits
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  PERSONALITY TRAITS",
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )));
        for t in &profile.traits {
            let bar = format!(
                "{}{}",
                "\u{2588}".repeat(t.intensity as usize),
                "\u{2591}".repeat(10_usize.saturating_sub(t.intensity as usize))
            );
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{:<16}", t.trait_name),
                    Style::default().fg(theme::TEXT),
                ),
                Span::styled(bar, Style::default().fg(theme::PRIMARY_LIGHT)),
                Span::styled(
                    format!(" {}/10", t.intensity),
                    Style::default().fg(theme::TEXT_DIM),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    t.manifestation.clone(),
                    Style::default().fg(theme::TEXT_DIM),
                ),
            ]));
        }

        // Speech patterns
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  SPEECH PATTERNS",
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(vec![
            Span::raw("  Style: "),
            Span::styled(
                profile.speech_patterns.vocabulary_style.clone(),
                Style::default().fg(theme::TEXT),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  Pacing: "),
            Span::styled(
                profile.speech_patterns.pacing.clone(),
                Style::default().fg(theme::TEXT),
            ),
        ]));
        if let Some(ref dialect) = profile.speech_patterns.dialect_notes {
            lines.push(Line::from(vec![
                Span::raw("  Dialect: "),
                Span::styled(dialect.clone(), Style::default().fg(theme::TEXT)),
            ]));
        }

        // Common phrases
        if !profile.speech_patterns.common_phrases.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "  COMMON PHRASES",
                Style::default()
                    .fg(theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )));
            for phrase in &profile.speech_patterns.common_phrases {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("\u{201c}{phrase}\u{201d}"),
                        Style::default().fg(theme::TEXT),
                    ),
                ]));
            }
        }

        // Knowledge areas
        if !profile.knowledge_areas.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "  KNOWLEDGE AREAS",
                Style::default()
                    .fg(theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    profile.knowledge_areas.join(", "),
                    Style::default().fg(theme::TEXT),
                ),
            ]));
        }

        // Behavior
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  BEHAVIORAL TENDENCIES",
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )));
        let bt = &profile.behavioral_tendencies;
        for (label, value) in [
            ("Conflict", &bt.conflict_response),
            ("Strangers", &bt.stranger_response),
            ("Authority", &bt.authority_response),
            ("Help", &bt.help_response),
            ("Attitude", &bt.general_attitude),
        ] {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:<12}", label),
                    Style::default().fg(theme::TEXT_MUTED),
                ),
                Span::styled(value.clone(), Style::default().fg(theme::TEXT)),
            ]));
        }

        // Keybindings
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  [j/k] navigate  [Enter] select preset",
            Style::default().fg(theme::TEXT_DIM),
        )));

        frame.render_widget(Paragraph::new(lines), inner);
    }
}
