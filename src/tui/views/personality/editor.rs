use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders},
    Frame,
};
use ratatui_textarea::TextArea;
use crate::core::personality_base::{BehavioralTendencies, PersonalityProfile, PersonalityTrait, SpeechPatterns};
use crate::tui::services::Services;
use crate::tui::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EditorField {
    Name,
    Formality,
    Vocabulary,
    Dialect,
    Pacing,
    Tags,
    Attitude,
    Traits,
}

impl EditorField {
    fn next(&self) -> Self {
        match self {
            Self::Name => Self::Formality,
            Self::Formality => Self::Vocabulary,
            Self::Vocabulary => Self::Dialect,
            Self::Dialect => Self::Pacing,
            Self::Pacing => Self::Tags,
            Self::Tags => Self::Attitude,
            Self::Attitude => Self::Traits,
            Self::Traits => Self::Name,
        }
    }

    fn prev(&self) -> Self {
        match self {
            Self::Name => Self::Traits,
            Self::Formality => Self::Name,
            Self::Vocabulary => Self::Formality,
            Self::Dialect => Self::Vocabulary,
            Self::Pacing => Self::Dialect,
            Self::Tags => Self::Pacing,
            Self::Attitude => Self::Tags,
            Self::Traits => Self::Attitude,
        }
    }
}

pub struct PersonalityEditorState {
    pub active_profile_id: Option<String>,
    pub focused_field: EditorField,

    pub name: TextArea<'static>,
    pub formality: TextArea<'static>,
    pub vocabulary: TextArea<'static>,
    pub dialect: TextArea<'static>,
    pub pacing: TextArea<'static>,
    pub tags: TextArea<'static>,
    pub attitude: TextArea<'static>,
    pub traits_raw: TextArea<'static>,
}

impl PersonalityEditorState {
    pub fn new() -> Self {
        Self {
            active_profile_id: None,
            focused_field: EditorField::Name,
            name: Self::create_textarea("Name"),
            formality: Self::create_textarea("Formality (1-10)"),
            vocabulary: Self::create_textarea("Vocabulary Style"),
            dialect: Self::create_textarea("Dialect Notes"),
            pacing: Self::create_textarea("Pacing"),
            tags: Self::create_textarea("Tags (comma separated)"),
            attitude: Self::create_textarea("General Attitude"),
            traits_raw: Self::create_textarea("Traits (Format: Name | 1-10 | Manifestation)"),
        }
    }

    fn create_textarea(title: &'static str) -> TextArea<'static> {
        let mut ta = TextArea::default();
        let b = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_MUTED))
            .title(title);
        ta.set_block(b);
        ta.set_cursor_line_style(Style::default());
        ta
    }

    fn focus_active_textarea(&mut self) {
        // Reset styles first
        self.name.set_block(self.name.block().unwrap().clone().border_style(Style::default().fg(theme::TEXT_MUTED)));
        self.formality.set_block(self.formality.block().unwrap().clone().border_style(Style::default().fg(theme::TEXT_MUTED)));
        self.vocabulary.set_block(self.vocabulary.block().unwrap().clone().border_style(Style::default().fg(theme::TEXT_MUTED)));
        self.dialect.set_block(self.dialect.block().unwrap().clone().border_style(Style::default().fg(theme::TEXT_MUTED)));
        self.pacing.set_block(self.pacing.block().unwrap().clone().border_style(Style::default().fg(theme::TEXT_MUTED)));
        self.tags.set_block(self.tags.block().unwrap().clone().border_style(Style::default().fg(theme::TEXT_MUTED)));
        self.attitude.set_block(self.attitude.block().unwrap().clone().border_style(Style::default().fg(theme::TEXT_MUTED)));
        self.traits_raw.set_block(self.traits_raw.block().unwrap().clone().border_style(Style::default().fg(theme::TEXT_MUTED)));

        let active_style = Style::default().fg(theme::PRIMARY);

        match self.focused_field {
            EditorField::Name => self.name.set_block(self.name.block().unwrap().clone().border_style(active_style)),
            EditorField::Formality => self.formality.set_block(self.formality.block().unwrap().clone().border_style(active_style)),
            EditorField::Vocabulary => self.vocabulary.set_block(self.vocabulary.block().unwrap().clone().border_style(active_style)),
            EditorField::Dialect => self.dialect.set_block(self.dialect.block().unwrap().clone().border_style(active_style)),
            EditorField::Pacing => self.pacing.set_block(self.pacing.block().unwrap().clone().border_style(active_style)),
            EditorField::Tags => self.tags.set_block(self.tags.block().unwrap().clone().border_style(active_style)),
            EditorField::Attitude => self.attitude.set_block(self.attitude.block().unwrap().clone().border_style(active_style)),
            EditorField::Traits => self.traits_raw.set_block(self.traits_raw.block().unwrap().clone().border_style(active_style)),
        }
    }

    pub fn load_profile(&mut self, profile: Option<&PersonalityProfile>) {
        if let Some(p) = profile {
            self.active_profile_id = Some(p.id.clone());

            // Text substitution helper
            let set_lines = |ta: &mut TextArea, text: &str| {
                ta.delete_line_by_head();
                ta.delete_line_by_end();
                ta.insert_str(text);
            };

            set_lines(&mut self.name, &p.name);
            set_lines(&mut self.formality, &p.speech_patterns.formality.to_string());
            set_lines(&mut self.vocabulary, &p.speech_patterns.vocabulary_style);
            set_lines(&mut self.dialect, p.speech_patterns.dialect_notes.as_deref().unwrap_or(""));
            set_lines(&mut self.pacing, &p.speech_patterns.pacing);
            set_lines(&mut self.tags, &p.tags.join(", "));
            set_lines(&mut self.attitude, &p.behavioral_tendencies.general_attitude);

            let traits_str = p.traits.iter().map(|t| {
                format!("{} | {} | {}", t.trait_name, t.intensity, t.manifestation)
            }).collect::<Vec<_>>().join("\n");

            self.traits_raw = Self::create_textarea("Traits (Format: Name | 1-10 | Manifestation)");
            for line in traits_str.lines() {
                self.traits_raw.insert_str(line);
                self.traits_raw.insert_newline();
            }

        } else {
            self.active_profile_id = None;
            // Blank new form
            *self = Self::new();
        }
        self.focus_active_textarea();
    }

    // Export form logic back to Profile rules
    pub fn build_profile(&self) -> PersonalityProfile {
        let name = self.name.lines().join("").trim().to_string();
        let formality: u8 = self.formality.lines().join("").trim().parse().unwrap_or(5).clamp(1, 10);
        let tags: Vec<String> = self.tags.lines().join("").split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        let dialect_raw = self.dialect.lines().join("").trim().to_string();

        // Parse traits textarea
        let mut traits = Vec::new();
        for line in self.traits_raw.lines() {
            if line.trim().is_empty() { continue; }
            let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
            if parts.len() >= 3 {
                traits.push(PersonalityTrait {
                    trait_name: parts[0].to_string(),
                    intensity: parts[1].parse().unwrap_or(5).clamp(1, 10),
                    manifestation: parts[2].to_string(),
                });
            } else if parts.len() == 1 {
                traits.push(PersonalityTrait {
                    trait_name: parts[0].to_string(),
                    intensity: 5,
                    manifestation: "Typical focus".to_string(),
                });
            }
        }

        let now = chrono::Utc::now().to_rfc3339();

        PersonalityProfile {
            id: self.active_profile_id.clone().unwrap_or_else(|| format!("custom_{}", uuid::Uuid::new_v4())),
            name: if name.is_empty() { "Unnamed".to_string() } else { name },
            source: Some("custom".to_string()),
            speech_patterns: SpeechPatterns {
                formality,
                common_phrases: Vec::new(),
                vocabulary_style: self.vocabulary.lines().join("").trim().to_string(),
                dialect_notes: if dialect_raw.is_empty() { None } else { Some(dialect_raw) },
                pacing: self.pacing.lines().join("").trim().to_string(),
            },
            traits,
            knowledge_areas: Vec::new(),
            behavioral_tendencies: BehavioralTendencies {
                conflict_response: String::new(),
                stranger_response: String::new(),
                authority_response: String::new(),
                help_response: String::new(),
                general_attitude: self.attitude.lines().join("").trim().to_string(),
            },
            example_phrases: Vec::new(),
            tags,
            metadata: std::collections::HashMap::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn handle_input(&mut self, event: &Event, services: &Services) -> bool {
        let Event::Key(key) = event else { return false; };
        if key.kind != KeyEventKind::Press { return false; }

        match (key.modifiers, key.code) {
            // Save shortcut
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                let profile = self.build_profile();
                let store = services.personality.store();
                if self.active_profile_id.is_some() {
                    let _ = store.update(&profile.id.clone(), profile);
                } else {
                    let _ = store.create(profile);
                }
                // Could emit an action to switch back to List view
                return true;
            }
            // Navigate fields
            (KeyModifiers::NONE, KeyCode::Tab) => {
                self.focused_field = self.focused_field.next();
                self.focus_active_textarea();
                return true;
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                self.focused_field = self.focused_field.prev();
                self.focus_active_textarea();
                return true;
            }
            _ => {}
        }

        // Delegate to active textarea
        let active_ta = match self.focused_field {
            EditorField::Name => &mut self.name,
            EditorField::Formality => &mut self.formality,
            EditorField::Vocabulary => &mut self.vocabulary,
            EditorField::Dialect => &mut self.dialect,
            EditorField::Pacing => &mut self.pacing,
            EditorField::Tags => &mut self.tags,
            EditorField::Attitude => &mut self.attitude,
            EditorField::Traits => &mut self.traits_raw,
        };

        match key.code {
            KeyCode::Enter if self.focused_field != EditorField::Traits => {
                // Return false to stop Enter from typing newline in single-line fields
                // Maybe focus next field?
                self.focused_field = self.focused_field.next();
                self.focus_active_textarea();
                return true;
            }
            _ => {
                active_ta.input(*key);
            }
        }

        true
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Name
                Constraint::Length(3), // Formality + Tags row limits
                Constraint::Length(3), // Vocab + Dialect
                Constraint::Length(3), // Attitude + Pacing
                Constraint::Min(5),    // Traits (multiline)
                Constraint::Length(1), // Help footer
            ])
            .split(area);

        // Name
        frame.render_widget(&self.name, chunks[0]);

        let row2 = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(chunks[1]);
        frame.render_widget(&self.formality, row2[0]);
        frame.render_widget(&self.tags, row2[1]);

        let row3 = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(chunks[2]);
        frame.render_widget(&self.vocabulary, row3[0]);
        frame.render_widget(&self.dialect, row3[1]);

        let row4 = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(chunks[3]);
        frame.render_widget(&self.attitude, row4[0]);
        frame.render_widget(&self.pacing, row4[1]);

        // Traits
        frame.render_widget(&self.traits_raw, chunks[4]);

        // Footer
        use ratatui::text::{Line, Span};
        let help = Line::from(vec![
            Span::styled("Tab", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(": Next Field | "),
            Span::styled("Ctrl+S", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(": Save Profile | "),
            Span::styled("Esc", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(": Back to List"),
        ]);
        frame.render_widget(help, chunks[5]);
    }
}
