//! Generation view manager.
//!
//! Wraps Character Generation and Campaign Generation Wizard, providing a single
//! entry point tab in the TUI that lets the user choose which tool to launch.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::services::Services;
use super::super::theme;

use crate::tui::views::character_gen::CharacterGenState;
use crate::tui::views::campaign_wizard::CampaignWizardState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationMode {
    Menu,
    Character,
    Campaign,
}

pub struct GenerationState {
    pub mode: GenerationMode,
    pub char_gen: CharacterGenState,
    pub camp_gen: CampaignWizardState,
    selected_option: usize,
}

impl GenerationState {
    pub fn new() -> Self {
        Self {
            mode: GenerationMode::Menu,
            char_gen: CharacterGenState::new(),
            camp_gen: CampaignWizardState::new(),
            selected_option: 0,
        }
    }

    pub fn load(&mut self, services: &Services) {
        self.char_gen.load(services);
        self.camp_gen.load(services);
    }

    pub fn poll(&mut self) {
        self.char_gen.poll();
        self.camp_gen.poll();
    }

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

        if self.mode == GenerationMode::Menu {
            return self.handle_menu_input(*code, *modifiers);
        }

        // Global escape hatch to get back to menu if in an early phase.
        // If the inner state handles Esc, it might return true, but we want a way out
        // if they bubble it up (i.e. they return false). Actually, character gen
        // returns true and handles its own Esc.
        // We will just let them process it.
        match self.mode {
            GenerationMode::Character => {
                let handled = self.char_gen.handle_input(event, services);
                if !handled && *code == KeyCode::Esc && *modifiers == KeyModifiers::NONE {
                    self.mode = GenerationMode::Menu;
                    return true;
                }
                handled
            }
            GenerationMode::Campaign => {
                let handled = self.camp_gen.handle_input(event, services);
                if !handled && *code == KeyCode::Esc && *modifiers == KeyModifiers::NONE {
                    self.mode = GenerationMode::Menu;
                    return true;
                }
                handled
            }
            _ => false,
        }
    }

    fn handle_menu_input(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        match (modifiers, code) {
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                self.selected_option = (self.selected_option + 1).min(1);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                self.selected_option = self.selected_option.saturating_sub(1);
                true
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                if self.selected_option == 0 {
                    self.mode = GenerationMode::Character;
                } else {
                    self.mode = GenerationMode::Campaign;
                }
                true
            }
            _ => false,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        match self.mode {
            GenerationMode::Menu => self.render_menu(frame, area),
            GenerationMode::Character => self.char_gen.render(frame, area),
            GenerationMode::Campaign => self.camp_gen.render(frame, area),
        }
    }

    fn render_menu(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Generation Hub ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_MUTED));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines = Vec::new();
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  Select a generator to launch:",
            Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::raw(""));

        let options = [
            ("👤 Character Generator", "Create PCs or NPCs with LLM-assisted backstories"),
            ("🗺 Campaign Wizard", "Design full campaign structure, arcs, and initial hooks"),
        ];

        for (i, (title, desc)) in options.iter().enumerate() {
            let is_selected = i == self.selected_option;
            let cursor = if is_selected { "▸ " } else { "  " };

            lines.push(Line::from(vec![
                Span::styled(
                    cursor,
                    if is_selected {
                        Style::default().fg(theme::ACCENT)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(
                    format!("{:<26}", title),
                    if is_selected {
                        Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(desc.to_string(), Style::default().fg(theme::TEXT_MUTED)),
            ]));
        }

        // Footer
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(inner.width.saturating_sub(4) as usize)),
            Style::default().fg(theme::TEXT_MUTED),
        )));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("j/k", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":navigate "),
            Span::styled("Enter", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":select "),
        ]));

        frame.render_widget(Paragraph::new(lines), inner);
    }
}
