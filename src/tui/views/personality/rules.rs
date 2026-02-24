use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style, Color},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};
use crate::core::personality_base::PersonalityProfile; // Just for placeholder if needed, though
use crate::core::personality::types::BlendRule;
use crate::tui::services::Services;
use crate::tui::theme;

pub struct ContextRulesState {
    pub rules: Vec<BlendRule>,
    pub table_state: TableState,
    pub loading: bool,
}

impl ContextRulesState {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            table_state: TableState::default(),
            loading: false,
        }
    }

    pub fn load(&mut self, services: &Services) {
        if self.loading { return; }
        self.loading = true;

        // In a real implementation, we'd fetch from rule store:
        // let rule_store = services.personality.rule_store();
        // self.rules = rule_store.list();

        // Since we are building UI, we'll just populate placeholders if empty for now
        // to verify layout integration.
        if self.rules.is_empty() {
            self.rules = vec![];
        }

        self.loading = false;

        if self.table_state.selected().is_none() && !self.rules.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    pub fn poll(&mut self) {
        // Handle async loading if we switch to mpsc channels
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
            (KeyModifiers::NONE, KeyCode::Enter) | (KeyModifiers::NONE, KeyCode::Char(' ')) => {
                self.toggle_active();
                true
            }
            _ => false, // let parent handle
        }
    }

    fn select_next(&mut self) {
        if self.rules.is_empty() { return; }
        let i = match self.table_state.selected() {
            Some(i) => if i >= self.rules.len() - 1 { 0 } else { i + 1 },
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn select_prev(&mut self) {
        if self.rules.is_empty() { return; }
        let i = match self.table_state.selected() {
            Some(i) => if i == 0 { self.rules.len() - 1 } else { i - 1 },
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn toggle_active(&mut self) {
        if let Some(i) = self.table_state.selected() {
            if let Some(rule) = self.rules.get_mut(i) {
                rule.enabled = !rule.enabled;
            }
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Contextual Rules ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_MUTED));

        let header_cells = ["Rule Name", "Context Target", "Priority", "Status"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD)));
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let rows = self.rules.iter().map(|rule| {
            let status = if rule.enabled { "Active" } else { "Inactive" };
            let status_color = if rule.enabled { Color::Green } else { Color::DarkGray };
            Row::new(vec![
                Cell::from(rule.name.clone()),
                Cell::from(rule.context.clone()),
                Cell::from(rule.priority.to_string()),
                Cell::from(status).style(Style::default().fg(status_color)),
            ])
        });

        let table = Table::new(rows, [
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(10),
            Constraint::Percentage(20),
        ])
        .header(header)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">> ");

        frame.render_stateful_widget(table, area, &mut self.table_state);
    }
}
