use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use crate::tui::services::Services;
use crate::tui::theme;

const PRESET_NAMES: &[(&str, &str)] = &[
    ("tavern_keeper", "Friendly Tavern Keeper"),
    ("grumpy_merchant", "Grumpy Merchant"),
    ("village_elder", "Wise Village Elder"),
    ("corrupt_guard", "Corrupt City Guard"),
    ("mystic_seer", "Mysterious Seer"),
    ("eberron_artificer", "Eberron House Cannith Artificer"),
];

pub struct PresetBrowserState {
    pub list_state: ListState,
}

impl PresetBrowserState {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self { list_state }
    }

    pub fn handle_input(&mut self, event: &Event, _services: &Services) -> bool {
        let Event::Key(key) = event else { return false; };
        if key.kind != KeyEventKind::Press { return false; }

        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                self.select_next();
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                self.select_prev();
                true
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                // Here we would emit an action to clone this preset into Editor
                // or save it directly to the active profile DB list.
                true
            }
            _ => false,
        }
    }

    fn select_next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => if i >= PRESET_NAMES.len() - 1 { 0 } else { i + 1 },
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn select_prev(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => if i == 0 { PRESET_NAMES.len() - 1 } else { i - 1 },
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Built-in Personality Presets ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_MUTED));

        let items: Vec<ListItem> = PRESET_NAMES
            .iter()
            .map(|(id, name)| {
                let text = format!("{} ({})", name, id);
                ListItem::new(text)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }
}
