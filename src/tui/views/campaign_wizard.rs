use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, List, ListItem, ListState},
    Frame,
};
use ratatui_textarea::TextArea;

use crate::core::campaign::wizard::WizardState;
use crate::core::character_gen::{CharacterGenerator, SystemInfo};
use crate::tui::services::Services;
use super::super::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardPhase {
    Basics,
    Party,
    Templates,
    Confirm,
}

pub struct CampaignWizardState {
    pub phase: WizardPhase,

    // Systems available
    pub systems: Vec<SystemInfo>,

    // Basics Phase data
    pub name_input: TextArea<'static>,
    pub sys_list_state: ListState,
    pub desc_input: TextArea<'static>,
    pub focus_index: usize, // 0 = Name, 1 = System, 2 = Desc

    // Party Phase data
    pub party_size: u8,

    // Templates Phase data
    pub template_idx: usize,

    pub error: Option<String>,
}

impl CampaignWizardState {
    pub fn new() -> Self {
        let systems = CharacterGenerator::list_system_info();
        let mut sys_list_state = ListState::default();
        if !systems.is_empty() {
            sys_list_state.select(Some(0));
        }

        let mut name_input = TextArea::default();
        name_input.set_block(Block::default().borders(Borders::ALL).title(" Campaign Name "));
        name_input.set_style(theme::border_focused());

        let mut desc_input = TextArea::default();
        desc_input.set_block(Block::default().borders(Borders::ALL).title(" Setting Prompt (Description) "));
        desc_input.set_style(theme::border_default());

        Self {
            phase: WizardPhase::Basics,
            systems,
            name_input,
            sys_list_state,
            desc_input,
            focus_index: 0,
            party_size: 4,
            template_idx: 0,
            error: None,
        }
    }

    pub fn load(&mut self, _services: &Services) {
        // Init logic
    }

    pub fn poll(&mut self) {}

    pub fn handle_input(&mut self, event: &Event, services: &Services) -> bool {
        let key = match event {
            Event::Key(k) if k.kind == KeyEventKind::Press => k,
            _ => return false,
        };

        match self.phase {
            WizardPhase::Basics => self.handle_basics_input(*key, event),
            WizardPhase::Party => self.handle_party_input(*key),
            WizardPhase::Templates => self.handle_templates_input(*key),
            WizardPhase::Confirm => self.handle_confirm_input(*key, services),
        }
    }

    fn handle_basics_input(&mut self, key: KeyEvent, event: &Event) -> bool {
        // Tab switching between fields
        if key.code == KeyCode::Tab || key.code == KeyCode::Down {
            self.focus_index = (self.focus_index + 1) % 3;
            self.update_focus_styles();
            return true;
        }
        if key.code == KeyCode::BackTab || key.code == KeyCode::Up {
            self.focus_index = (self.focus_index + 2) % 3;
            self.update_focus_styles();
            return true;
        }

        // Ctrl+Enter advances to next phase
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Enter {
            self.phase = WizardPhase::Party;
            return true;
        }

        match self.focus_index {
            0 => {
                // Name Input
                if key.code == KeyCode::Enter {
                    self.focus_index = 1;
                    self.update_focus_styles();
                } else {
                    self.name_input.input(event.clone());
                }
                true
            }
            1 => {
                // System List
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        let i = match self.sys_list_state.selected() {
                            Some(i) => (i + 1).min(self.systems.len().saturating_sub(1)),
                            None => 0,
                        };
                        self.sys_list_state.select(Some(i));
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        let i = match self.sys_list_state.selected() {
                            Some(i) => i.saturating_sub(1),
                            None => 0,
                        };
                        self.sys_list_state.select(Some(i));
                    }
                    KeyCode::Enter => {
                        self.focus_index = 2;
                        self.update_focus_styles();
                    }
                    _ => {}
                }
                true
            }
            2 => {
                // Description Input
                self.desc_input.input(event.clone());
                true
            }
            _ => false,
        }
    }

    fn handle_party_input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down | KeyCode::Left => {
                self.party_size = self.party_size.saturating_sub(1).max(1);
                true
            }
            KeyCode::Char('k') | KeyCode::Up | KeyCode::Right => {
                self.party_size = self.party_size.saturating_add(1).min(10);
                true
            }
            KeyCode::Enter => {
                self.phase = WizardPhase::Templates;
                true
            }
            KeyCode::Esc => {
                self.phase = WizardPhase::Basics;
                true
            }
            _ => false,
        }
    }

    fn handle_templates_input(&mut self, key: KeyEvent) -> bool {
        let template_count = 7; // e.g. HerosJourney, ThreeAct etc.
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.template_idx = (self.template_idx + 1).min(template_count - 1);
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.template_idx = self.template_idx.saturating_sub(1);
                true
            }
            KeyCode::Enter => {
                self.phase = WizardPhase::Confirm;
                true
            }
            KeyCode::Esc => {
                self.phase = WizardPhase::Party;
                true
            }
            _ => false,
        }
    }

    fn handle_confirm_input(&mut self, key: KeyEvent, services: &Services) -> bool {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                self.submit_wizard(services);
                true
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.phase = WizardPhase::Templates;
                true
            }
            _ => false,
        }
    }

    fn update_focus_styles(&mut self) {
        self.name_input.set_style(if self.focus_index == 0 { theme::border_focused() } else { theme::border_default() });
        self.desc_input.set_style(if self.focus_index == 2 { theme::border_focused() } else { theme::border_default() });
    }

    fn submit_wizard(&mut self, services: &Services) {
        // Gather data
        let name = self.name_input.lines().join(" ");
        let desc = self.desc_input.lines().join("\n");
        let system = self.sys_list_state.selected().and_then(|i| self.systems.get(i)).map(|s| s.id.clone()).unwrap_or_else(|| "dnd5e".into());

        let uid = uuid::Uuid::new_v4().to_string();
        let mut db_wizard = WizardState::new(uid.clone(), false);
        db_wizard.campaign_draft.name = Some(name.clone());
        db_wizard.campaign_draft.description = Some(desc.clone());
        db_wizard.campaign_draft.system = Some(system.clone());
        db_wizard.campaign_draft.player_count = Some(self.party_size);

        // Persist via CampaignManager
        let mut campaign = services.campaign_manager.create_campaign(&name, &system);
        campaign.description = Some(desc.clone());

        // Update with description
        if let Err(e) = services.campaign_manager.update_campaign(campaign.clone(), false) {
            log::error!("Failed to update campaign description: {e}");
        }

        log::info!(
            "Campaign created: id={}, name={}, system={}, party_size={}",
            campaign.id, name, system, self.party_size
        );
        let _ = db_wizard; // WizardState for future expanded persistence

        // Reset wizard
        self.name_input = TextArea::default();
        self.name_input.set_block(Block::default().borders(Borders::ALL).title(" Campaign Name "));
        self.desc_input = TextArea::default();
        self.desc_input.set_block(Block::default().borders(Borders::ALL).title(" Setting Prompt (Description) "));
        self.phase = WizardPhase::Basics;
        self.focus_index = 0;
        self.update_focus_styles();
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(" Campaign Wizard - {:?} Phase ", self.phase))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_MUTED));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        match self.phase {
            WizardPhase::Basics => self.render_basics(frame, inner),
            WizardPhase::Party => self.render_party(frame, inner),
            WizardPhase::Templates => self.render_templates(frame, inner),
            WizardPhase::Confirm => self.render_confirm(frame, inner),
        }
    }

    fn render_basics(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(3),  // Name
            Constraint::Length(6),  // System List
            Constraint::Min(5),     // Desc
            Constraint::Length(1),  // Help
        ])
        .split(area);

        // Name
        frame.render_widget(&self.name_input, chunks[0]);

        // System
        let items: Vec<ListItem> = self.systems.iter().map(|s| ListItem::new(s.name.clone())).collect();
        let sys_block = Block::default().title(" System Selection ").borders(Borders::ALL).border_style(if self.focus_index == 1 { theme::border_focused() } else { theme::border_default() });
        let list = List::new(items)
            .block(sys_block)
            .highlight_style(Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD))
            .highlight_symbol("▸ ");

        let mut render_state = self.sys_list_state.clone();
        frame.render_stateful_widget(list, chunks[1], &mut render_state);

        // Desc
        frame.render_widget(&self.desc_input, chunks[2]);

        // Help
        let help = Paragraph::new(Line::from(vec![
            Span::styled("Tab", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":next field  "),
            Span::styled("Ctrl+Enter", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":next step"),
        ]));
        frame.render_widget(help, chunks[3]);
    }

    fn render_party(&self, frame: &mut Frame, area: Rect) {
        let p = Paragraph::new(vec![
            Line::raw(""),
            Line::from(Span::raw(format!("  Party Size: {}", self.party_size))),
            Line::raw(""),
            Line::from(vec![
                Span::styled("  Left/Right", Style::default().fg(theme::TEXT_MUTED)),
                Span::raw(" to adjust, "),
                Span::styled("Enter", Style::default().fg(theme::TEXT_MUTED)),
                Span::raw(" to continue, "),
                Span::styled("Esc", Style::default().fg(theme::TEXT_MUTED)),
                Span::raw(" to go back."),
            ])
        ]);
        frame.render_widget(p, area);
    }

    fn render_templates(&self, frame: &mut Frame, area: Rect) {
        let templates = [
            "Hero's Journey", "Three Act", "Five Act", "Mystery",
            "Political Intrigue", "Dungeon Delve", "Sandbox"
        ];

        let items: Vec<ListItem> = templates.iter().enumerate().map(|(i, &s)| {
            let cursor = if i == self.template_idx { "▸ " } else { "  " };
            let style = if i == self.template_idx { Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD) } else { Style::default() };
            ListItem::new(Line::from(vec![Span::styled(cursor, style), Span::styled(s, style)]))
        }).collect();

        let list = List::new(items)
            .block(Block::default().title(" Select Arc Template ").borders(Borders::ALL))
            .highlight_style(Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD));

        let mut state = ListState::default();
        state.select(Some(self.template_idx));

        let chunks = Layout::vertical([Constraint::Min(5), Constraint::Length(1)]).split(area);
        frame.render_stateful_widget(list, chunks[0], &mut state);

        frame.render_widget(Paragraph::new(Line::from(vec![
            Span::styled("  j/k", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(" to navigate, "),
            Span::styled("Enter", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(" to finish."),
        ])), chunks[1]);
    }

    fn render_confirm(&self, frame: &mut Frame, area: Rect) {
        let sys_name = self.sys_list_state.selected().and_then(|i| self.systems.get(i)).map(|s| s.name.clone()).unwrap_or_default();
        let name = self.name_input.lines().join(" ");
        let p = Paragraph::new(vec![
            Line::raw(""),
            Line::from(Span::styled("  Confirm Generation?", Style::default().fg(theme::ACCENT))),
            Line::raw(""),
            Line::from(format!("  Campaign Name: {}", name)),
            Line::from(format!("  System: {}", sys_name)),
            Line::from(format!("  Party Size: {}", self.party_size)),
            Line::raw(""),
            Line::from(vec![
                Span::styled("  y/Enter", Style::default().fg(theme::SUCCESS)),
                Span::raw(" to submit, "),
                Span::styled("n/Esc", Style::default().fg(theme::ERROR)),
                Span::raw(" to cancel."),
            ])
        ]);
        frame.render_widget(p, area);
    }
}
