use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

use crate::core::personality::types::{BlendRule, BlendRuleId};
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

    pub fn load(&mut self, _services: &Services) {
        if self.loading {
            return;
        }
        self.loading = true;

        // Load built-in blend rules for common gameplay contexts.
        // BlendRuleStore requires Meilisearch — use sensible defaults until
        // Phase 7 provides an in-memory adapter.
        if self.rules.is_empty() {
            let now = chrono::Utc::now().to_rfc3339();
            self.rules = vec![
                BlendRule {
                    id: BlendRuleId::new("builtin_combat"),
                    name: "Combat Encounter".into(),
                    description: Some(
                        "Shifts DM personality toward tactical and intense during combat"
                            .into(),
                    ),
                    context: "combat".into(),
                    priority: 100,
                    enabled: true,
                    is_builtin: true,
                    campaign_id: None,
                    blend_weights: Default::default(),
                    tags: vec!["combat".into(), "tactical".into()],
                    created_at: now.clone(),
                },
                BlendRule {
                    id: BlendRuleId::new("builtin_social"),
                    name: "Social Interaction".into(),
                    description: Some(
                        "Enhances conversational and empathetic traits for roleplay"
                            .into(),
                    ),
                    context: "social".into(),
                    priority: 90,
                    enabled: true,
                    is_builtin: true,
                    campaign_id: None,
                    blend_weights: Default::default(),
                    tags: vec!["social".into(), "roleplay".into()],
                    created_at: now.clone(),
                },
                BlendRule {
                    id: BlendRuleId::new("builtin_exploration"),
                    name: "Exploration & Discovery".into(),
                    description: Some(
                        "Emphasizes descriptive and mysterious qualities for exploration"
                            .into(),
                    ),
                    context: "exploration".into(),
                    priority: 80,
                    enabled: true,
                    is_builtin: true,
                    campaign_id: None,
                    blend_weights: Default::default(),
                    tags: vec!["exploration".into(), "descriptive".into()],
                    created_at: now.clone(),
                },
                BlendRule {
                    id: BlendRuleId::new("builtin_puzzle"),
                    name: "Puzzle & Mystery".into(),
                    description: Some(
                        "Adds cryptic and hint-giving tendencies for puzzle encounters"
                            .into(),
                    ),
                    context: "puzzle".into(),
                    priority: 70,
                    enabled: false,
                    is_builtin: true,
                    campaign_id: None,
                    blend_weights: Default::default(),
                    tags: vec!["puzzle".into(), "mystery".into()],
                    created_at: now,
                },
            ];
        }

        self.loading = false;

        if self.table_state.selected().is_none() && !self.rules.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    pub fn poll(&mut self) {
        // Reserved for future async loading via mpsc channel
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
            (KeyModifiers::NONE, KeyCode::Enter | KeyCode::Char(' ')) => {
                self.toggle_active();
                true
            }
            _ => false,
        }
    }

    fn select_next(&mut self) {
        if self.rules.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.rules.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn select_prev(&mut self) {
        if self.rules.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.rules.len() - 1
                } else {
                    i - 1
                }
            }
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
            .title(" Contextual Blend Rules ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_MUTED));

        let header_cells = ["Rule Name", "Context", "Priority", "Status"]
            .iter()
            .map(|h| {
                Cell::from(*h).style(
                    Style::default()
                        .fg(theme::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                )
            });
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let rows = self.rules.iter().map(|rule| {
            let status = if rule.enabled { "Active" } else { "Inactive" };
            let status_color = if rule.enabled {
                Color::Green
            } else {
                Color::DarkGray
            };
            Row::new(vec![
                Cell::from(rule.name.clone()),
                Cell::from(rule.context.clone()),
                Cell::from(rule.priority.to_string()),
                Cell::from(status).style(Style::default().fg(status_color)),
            ])
        });

        let table = Table::new(
            rows,
            [
                ratatui::layout::Constraint::Percentage(40),
                ratatui::layout::Constraint::Percentage(25),
                ratatui::layout::Constraint::Percentage(10),
                ratatui::layout::Constraint::Percentage(25),
            ],
        )
        .header(header)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">> ");

        frame.render_stateful_widget(table, area, &mut self.table_state);
    }
}
