use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Gauge},
    Frame,
};
use crate::tui::services::Services;
use crate::tui::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BlendPhase {
    Base,
    Setting,
    Situational,
    Override,
}

impl BlendPhase {
    fn next(&self) -> Self {
        match self {
            Self::Base => Self::Setting,
            Self::Setting => Self::Situational,
            Self::Situational => Self::Override,
            Self::Override => Self::Base,
        }
    }

    fn prev(&self) -> Self {
        match self {
            Self::Base => Self::Override,
            Self::Setting => Self::Base,
            Self::Situational => Self::Setting,
            Self::Override => Self::Situational,
        }
    }
}

pub struct PersonalityBlenderState {
    pub focused_phase: BlendPhase,
    pub base_weight: u16,
    pub setting_weight: u16,
    pub situational_weight: u16,
    pub override_weight: u16,
}

impl PersonalityBlenderState {
    pub fn new() -> Self {
        Self {
            focused_phase: BlendPhase::Base,
            base_weight: 40,
            setting_weight: 30,
            situational_weight: 20,
            override_weight: 10,
        }
    }

    pub fn handle_input(&mut self, event: &Event, _services: &Services) -> bool {
        let Event::Key(key) = event else { return false; };
        if key.kind != KeyEventKind::Press { return false; }

        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Down) | (KeyModifiers::NONE, KeyCode::Tab) => {
                self.focused_phase = self.focused_phase.next();
                true
            }
            (KeyModifiers::NONE, KeyCode::Up) | (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                self.focused_phase = self.focused_phase.prev();
                true
            }
            (KeyModifiers::NONE, KeyCode::Left) => {
                self.adjust_weight(-5);
                true
            }
            (KeyModifiers::NONE, KeyCode::Right) => {
                self.adjust_weight(5);
                true
            }
            _ => false,
        }
    }

    fn adjust_weight(&mut self, delta: i32) {
        let apply = |val: &mut u16| {
            let new_val = (*val as i32 + delta).clamp(0, 100) as u16;
            *val = new_val;
        };

        match self.focused_phase {
            BlendPhase::Base => apply(&mut self.base_weight),
            BlendPhase::Setting => apply(&mut self.setting_weight),
            BlendPhase::Situational => apply(&mut self.situational_weight),
            BlendPhase::Override => apply(&mut self.override_weight),
        }

        // Normalize? Or let the user set what they want and normalize on submit?
        // Usually, manual sliders are easier arrayed without strict auto-normalization.
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" 4-Phase Personality Blend Settings ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_MUTED));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Base
                Constraint::Length(3), // Setting
                Constraint::Length(3), // Situational
                Constraint::Length(3), // Override
                Constraint::Min(0),    // Padding
            ])
            .split(inner);

        self.render_gauge(frame, chunks[0], "1. Base Personality", self.base_weight, self.focused_phase == BlendPhase::Base, Color::Blue);
        self.render_gauge(frame, chunks[1], "2. Setting Modifications", self.setting_weight, self.focused_phase == BlendPhase::Setting, Color::Green);
        self.render_gauge(frame, chunks[2], "3. Situational State", self.situational_weight, self.focused_phase == BlendPhase::Situational, Color::Yellow);
        self.render_gauge(frame, chunks[3], "4. Active Override", self.override_weight, self.focused_phase == BlendPhase::Override, Color::Red);
    }

    fn render_gauge(&self, frame: &mut Frame, area: Rect, label: &str, value: u16, is_focused: bool, color: Color) {
        let border_style = if is_focused {
            Style::default().fg(theme::PRIMARY)
        } else {
            Style::default().fg(theme::TEXT_MUTED)
        };

        let gauge = Gauge::default()
            .block(Block::default().title(label).borders(Borders::ALL).border_style(border_style))
            .gauge_style(Style::default().fg(color).bg(Color::DarkGray))
            .percent(value);

        frame.render_widget(gauge, area);
    }
}
