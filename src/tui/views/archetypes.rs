//! Archetype browser — read-only tree view of archetype registry.
//!
//! Displays archetype categories, individual archetypes, and their
//! personality affinities, NPC role mappings, and naming cultures.
//! Currently shows the category structure; full archetype data will
//! load when ArchetypeRegistry is wired through Services.

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

// ── Category definitions ───────────────────────────────────────────────────

struct ArchetypeCategory {
    name: &'static str,
    icon: &'static str,
    archetypes: &'static [&'static str],
}

const CATEGORIES: &[ArchetypeCategory] = &[
    ArchetypeCategory {
        name: "Warrior",
        icon: "⚔",
        archetypes: &["Knight", "Barbarian", "Ranger", "Paladin", "Fighter"],
    },
    ArchetypeCategory {
        name: "Magic",
        icon: "✦",
        archetypes: &["Wizard", "Sorcerer", "Warlock", "Druid", "Cleric"],
    },
    ArchetypeCategory {
        name: "Rogue",
        icon: "◆",
        archetypes: &["Thief", "Assassin", "Bard", "Scout", "Spy"],
    },
    ArchetypeCategory {
        name: "Social",
        icon: "♦",
        archetypes: &["Noble", "Merchant", "Diplomat", "Scholar", "Healer"],
    },
    ArchetypeCategory {
        name: "Creature",
        icon: "◎",
        archetypes: &["Beast", "Undead", "Fiend", "Fey", "Construct"],
    },
];

// ── State ──────────────────────────────────────────────────────────────────

pub struct ArchetypeViewState {
    selected_category: usize,
    selected_archetype: usize,
    focus_panel: Panel,
    #[allow(dead_code)]
    scroll: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Panel {
    Categories,
    Archetypes,
}

impl ArchetypeViewState {
    pub fn new() -> Self {
        Self {
            selected_category: 0,
            selected_archetype: 0,
            focus_panel: Panel::Categories,
            scroll: 0,
        }
    }

    pub fn load(&mut self, _services: &Services) {
        // ArchetypeRegistry not in Services yet
    }

    pub fn poll(&mut self) {
        // No async data to poll
    }

    fn current_category(&self) -> &ArchetypeCategory {
        &CATEGORIES[self.selected_category]
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
            (KeyModifiers::NONE, KeyCode::Tab) => {
                self.focus_panel = match self.focus_panel {
                    Panel::Categories => Panel::Archetypes,
                    Panel::Archetypes => Panel::Categories,
                };
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('l') | KeyCode::Enter) => {
                if self.focus_panel == Panel::Categories {
                    self.focus_panel = Panel::Archetypes;
                    self.selected_archetype = 0;
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('h') | KeyCode::Esc) => {
                if self.focus_panel == Panel::Archetypes {
                    self.focus_panel = Panel::Categories;
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                match self.focus_panel {
                    Panel::Categories => {
                        if self.selected_category + 1 < CATEGORIES.len() {
                            self.selected_category += 1;
                            self.selected_archetype = 0;
                        }
                    }
                    Panel::Archetypes => {
                        let cat = self.current_category();
                        if self.selected_archetype + 1 < cat.archetypes.len() {
                            self.selected_archetype += 1;
                        }
                    }
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                match self.focus_panel {
                    Panel::Categories => {
                        self.selected_category = self.selected_category.saturating_sub(1);
                        self.selected_archetype = 0;
                    }
                    Panel::Archetypes => {
                        self.selected_archetype = self.selected_archetype.saturating_sub(1);
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(30),
                Constraint::Percentage(45),
            ])
            .split(area);

        self.render_categories(frame, chunks[0]);
        self.render_archetypes(frame, chunks[1]);
        self.render_detail(frame, chunks[2]);
    }

    fn render_categories(&self, frame: &mut Frame, area: Rect) {
        let is_focused = self.focus_panel == Panel::Categories;
        let block = if is_focused {
            theme::block_focused("Categories")
        } else {
            theme::block_default("Categories")
        };
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line<'static>> = Vec::new();
        for (i, cat) in CATEGORIES.iter().enumerate() {
            let is_selected = i == self.selected_category;
            let marker = if is_selected && is_focused {
                "▸ "
            } else if is_selected {
                "  "
            } else {
                "  "
            };

            let style = if is_selected {
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };

            lines.push(Line::from(vec![
                Span::styled(marker.to_string(), style),
                Span::styled(format!("{} ", cat.icon), style),
                Span::styled(cat.name.to_string(), style),
                Span::styled(
                    format!(" ({})", cat.archetypes.len()),
                    Style::default().fg(theme::TEXT_DIM),
                ),
            ]));
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_archetypes(&self, frame: &mut Frame, area: Rect) {
        let is_focused = self.focus_panel == Panel::Archetypes;
        let cat = self.current_category();
        let block = if is_focused {
            theme::block_focused(cat.name)
        } else {
            theme::block_default(cat.name)
        };
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line<'static>> = Vec::new();
        for (i, name) in cat.archetypes.iter().enumerate() {
            let is_selected = i == self.selected_archetype;
            let marker = if is_selected && is_focused {
                "▸ "
            } else {
                "  "
            };

            let style = if is_selected {
                Style::default()
                    .fg(theme::PRIMARY_LIGHT)
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
    }

    fn render_detail(&self, frame: &mut Frame, area: Rect) {
        let cat = self.current_category();
        let archetype_name = cat
            .archetypes
            .get(self.selected_archetype)
            .unwrap_or(&"—");

        let block = theme::block_default("Detail");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = vec![
            Line::raw(""),
            Line::from(Span::styled(
                format!("  {} {}", cat.icon, archetype_name),
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("  Category: {}", cat.name),
                Style::default().fg(theme::TEXT_MUTED),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "  PERSONALITY AFFINITIES",
                Style::default()
                    .fg(theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "  (Not connected — awaiting ArchetypeRegistry)",
                Style::default().fg(theme::TEXT_DIM),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "  NPC ROLE MAPPINGS",
                Style::default()
                    .fg(theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "  (Not connected — awaiting ArchetypeRegistry)",
                Style::default().fg(theme::TEXT_DIM),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "  NAMING CULTURES",
                Style::default()
                    .fg(theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "  (Not connected — awaiting ArchetypeRegistry)",
                Style::default().fg(theme::TEXT_DIM),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "  [Tab] panel  [h/l] navigate  [j/k] select",
                Style::default().fg(theme::TEXT_DIM),
            )),
        ];

        frame.render_widget(Paragraph::new(lines), inner);
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = ArchetypeViewState::new();
        assert_eq!(state.selected_category, 0);
        assert_eq!(state.selected_archetype, 0);
        assert_eq!(state.focus_panel, Panel::Categories);
    }

    #[test]
    fn test_categories_nonempty() {
        assert!(!CATEGORIES.is_empty());
        for cat in CATEGORIES {
            assert!(!cat.archetypes.is_empty());
        }
    }

    #[test]
    fn test_panel_toggle() {
        let mut state = ArchetypeViewState::new();
        assert_eq!(state.focus_panel, Panel::Categories);
        state.focus_panel = Panel::Archetypes;
        assert_eq!(state.focus_panel, Panel::Archetypes);
    }

    #[test]
    fn test_category_selection_bounds() {
        let mut state = ArchetypeViewState::new();
        state.selected_category = CATEGORIES.len() - 1;
        // Should not go beyond
        let next = state.selected_category + 1;
        assert!(next >= CATEGORIES.len());
    }
}
