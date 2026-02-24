use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::tui::services::Services;
use crate::tui::theme;
use crate::tui::app::truncate;
use super::shared::PersonalityDisplay;
use tokio::sync::mpsc;

pub enum ListAction {
    None,
    Edit(String),
    Delete(String),
    Preview(String),
    CreateNew,
}

pub struct PersonalityListState {
    pub profiles: Vec<PersonalityDisplay>,
    pub selected: usize,
    pub list_scroll: usize,
    pub detail_scroll: usize,
    pub show_detail: bool,
    pub loading: bool,
    data_rx: mpsc::UnboundedReceiver<Vec<PersonalityDisplay>>,
    data_tx: mpsc::UnboundedSender<Vec<PersonalityDisplay>>,
}

impl PersonalityListState {
    pub fn new() -> Self {
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        Self {
            profiles: Vec::new(),
            selected: 0,
            list_scroll: 0,
            detail_scroll: 0,
            show_detail: false,
            loading: false,
            data_rx,
            data_tx,
        }
    }

    pub fn load(&mut self, services: &Services) {
        if self.loading { return; }
        self.loading = true;

        let personality = services.personality.clone();
        let tx = self.data_tx.clone();

        tokio::spawn(async move {
            let store = personality.store();
            let profiles: Vec<PersonalityDisplay> = store
                .list()
                .iter()
                .map(PersonalityDisplay::from_profile)
                .collect();
            let _ = tx.send(profiles);
        });
    }

    pub fn poll(&mut self) {
        if let Ok(profiles) = self.data_rx.try_recv() {
            self.profiles = profiles;
            self.loading = false;
            if !self.profiles.is_empty() {
                self.selected = self.selected.min(self.profiles.len() - 1);
            } else {
                self.selected = 0;
            }
        }
    }

    pub fn handle_input(&mut self, event: &Event, _services: &Services) -> ListAction {
        let Event::Key(key) = event else { return ListAction::None; };
        if key.kind != KeyEventKind::Press { return ListAction::None; }

        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                self.select_next();
                ListAction::None
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                self.select_prev();
                ListAction::None
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                if !self.profiles.is_empty() {
                    self.show_detail = !self.show_detail;
                    self.detail_scroll = 0;
                }
                ListAction::None
            }
            (KeyModifiers::NONE, KeyCode::Char('a')) => ListAction::CreateNew,
            (KeyModifiers::NONE, KeyCode::Char('e')) => {
                if let Some(p) = self.profiles.get(self.selected) {
                    ListAction::Edit(p.id.clone())
                } else {
                    ListAction::None
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('d')) => {
                if let Some(p) = self.profiles.get(self.selected) {
                    ListAction::Delete(p.id.clone())
                } else {
                    ListAction::None
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('p')) => {
                if let Some(p) = self.profiles.get(self.selected) {
                    ListAction::Preview(p.id.clone())
                } else {
                    ListAction::None
                }
            }
            _ => ListAction::None,
        }
    }

    fn select_next(&mut self) {
        if !self.profiles.is_empty() {
            self.selected = (self.selected + 1).min(self.profiles.len() - 1);
            self.detail_scroll = 0;
        }
    }

    fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
        self.detail_scroll = 0;
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if self.show_detail && !self.profiles.is_empty() {
            let chunks = Layout::horizontal([
                Constraint::Percentage(40),
                Constraint::Percentage(60),
            ]).split(area);
            self.render_list(frame, chunks[0]);
            self.render_detail(frame, chunks[1]);
        } else {
            self.render_list(frame, area);
        }
    }

    fn render_list(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Personality Library ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_MUTED));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.loading && self.profiles.is_empty() {
            let loading = Paragraph::new(vec![
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Loading personalities...", Style::default().fg(theme::TEXT_MUTED)),
                ]),
            ]);
            frame.render_widget(loading, inner);
            return;
        }

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));

        if self.profiles.is_empty() {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("No personalities yet.", Style::default().fg(theme::TEXT_MUTED)),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  Press "),
                Span::styled("a", Style::default().fg(theme::PRIMARY_LIGHT).bold()),
                Span::raw(" to add a personality profile."),
            ]));
        } else {
            // Header
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("  {:<20} {:>6}  {}", "Name", "Form.", "Traits"),
                    Style::default().fg(theme::TEXT_MUTED).add_modifier(Modifier::BOLD),
                ),
            ]));

            for (i, p) in self.profiles.iter().enumerate() {
                let is_selected = i == self.selected;
                let cursor = if is_selected { "▸ " } else { "  " };
                let source_tag = format!("[{}]", truncate(&p.source, 8));
                let trait_display = if p.trait_summary.is_empty() { "—".to_string() } else { truncate(&p.trait_summary, 30) };

                let row_style = if is_selected { Style::default().add_modifier(Modifier::BOLD) } else { Style::default() };
                let accent = if is_selected { theme::ACCENT } else { theme::TEXT_MUTED }; // Fix styling hack

                lines.push(Line::from(vec![
                    Span::styled(cursor, Style::default().fg(accent)),
                    Span::styled(format!("{:<20}", truncate(&p.name, 20)), row_style),
                    Span::styled(format!("{:>3}/10", p.formality), Style::default().fg(theme::PRIMARY_LIGHT)),
                    Span::raw("  "),
                    Span::styled(trait_display, Style::default().fg(theme::TEXT_MUTED)),
                    Span::raw("  "),
                    Span::styled(source_tag, Style::default().fg(theme::TEXT_MUTED)),
                ]));
            }
        }

        // Auto-scroll
        let visible_height = inner.height as usize;
        let selected_line = 2 + self.selected;
        let scroll = if visible_height > 0 && selected_line >= self.list_scroll + visible_height {
            selected_line.saturating_sub(visible_height - 1)
        } else if selected_line < self.list_scroll {
            selected_line
        } else {
            self.list_scroll
        }; // list_scroll would be mutable on self normally, ratatui scroll requires u16 tuple

        let content = Paragraph::new(lines).scroll((scroll as u16, 0));
        frame.render_widget(content, inner);
    }

    fn render_detail(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Detail ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_MUTED));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(display) = self.profiles.get(self.selected) else { return; };

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Name: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(display.name.clone(), Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Source: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(display.source.clone()),
        ]));

        let content = Paragraph::new(lines);
        frame.render_widget(content, inner);
    }
}
