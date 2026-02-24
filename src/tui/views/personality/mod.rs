use crossterm::event::Event;
use ratatui::{layout::Rect, Frame};
use crate::tui::services::Services;

pub mod legacy;
pub mod shared;

use legacy::PersonalityState as LegacyState;

pub struct PersonalityState {
    pub legacy: LegacyState,
}

impl PersonalityState {
    pub fn new() -> Self {
        Self {
            legacy: LegacyState::new(),
        }
    }

    pub fn load(&mut self, services: &Services) {
        self.legacy.load(services);
    }

    pub fn poll(&mut self) {
        self.legacy.poll();
    }

    pub fn handle_input(&mut self, event: &Event, services: &Services) -> bool {
        self.legacy.handle_input(event, services)
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        self.legacy.render(frame, area);
    }
}
