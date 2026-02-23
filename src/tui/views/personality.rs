//! Personality view — CRUD for DM personality profiles with detail view and prompt preview.
//!
//! Press `a` to add (preset or manual), `e` to edit, `d` to delete, `p` to preview
//! the generated system prompt. Enter toggles a split detail panel. Press `r` to refresh.
//! On first load, presets are auto-seeded if the store is empty.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use tokio::sync::mpsc;

use crate::core::personality_base::{
    create_preset_personality, BehavioralTendencies, PersonalityProfile, PersonalityTrait,
    SpeechPatterns,
};
use crate::tui::app::centered_rect;
use crate::tui::services::Services;
use crate::tui::widgets::input_buffer::InputBuffer;

// ── Preset names ────────────────────────────────────────────────────────────

const PRESET_NAMES: &[(&str, &str)] = &[
    ("tavern_keeper", "Friendly Tavern Keeper"),
    ("grumpy_merchant", "Grumpy Merchant"),
];

// ── Display types ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct PersonalityDisplay {
    id: String,
    name: String,
    source: String,
    trait_summary: String,
    tag_summary: String,
    formality: u8,
}

impl PersonalityDisplay {
    fn from_profile(p: &PersonalityProfile) -> Self {
        let trait_summary = p
            .traits
            .iter()
            .take(3)
            .map(|t| t.trait_name.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        let tag_summary = if p.tags.is_empty() {
            "—".to_string()
        } else {
            p.tags.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
        };

        Self {
            id: p.id.clone(),
            name: p.name.clone(),
            source: p.source.clone().unwrap_or_else(|| "custom".to_string()),
            trait_summary,
            tag_summary,
            formality: p.speech_patterns.formality,
        }
    }
}

// ── Modal types ─────────────────────────────────────────────────────────────

enum PersonalityModal {
    Create(CreatePhase),
    Edit,
    Delete,
    Preview {
        scroll: usize,
        prompt_text: String,
    },
}

enum CreatePhase {
    PickMethod,
    Preset { selected: usize },
    Form,
}

// ── Form state ──────────────────────────────────────────────────────────────

const FORM_FIELD_COUNT: usize = 8;

#[derive(Clone, Debug)]
struct PersonalityForm {
    /// Field labels for rendering.
    labels: [&'static str; FORM_FIELD_COUNT],
    /// Current values for each field.
    values: [String; FORM_FIELD_COUNT],
    /// Which field is focused (0-based).
    focused: usize,
    /// Traits as a vec of (name, intensity, manifestation).
    traits: Vec<(String, u8, String)>,
    /// Whether the trait sub-editor is active.
    editing_trait: bool,
    /// Index within trait sub-fields (0=name, 1=intensity, 2=manifestation).
    trait_field: usize,
}

impl PersonalityForm {
    // Field indices
    const NAME: usize = 0;
    const FORMALITY: usize = 1;
    const VOCABULARY: usize = 2;
    const PACING: usize = 3;
    const DIALECT: usize = 4;
    const TAGS: usize = 5;
    const ATTITUDE: usize = 6;
    const TRAITS_FIELD: usize = 7;

    fn new() -> Self {
        Self {
            labels: [
                "Name",
                "Formality (1-10)",
                "Vocabulary Style",
                "Pacing",
                "Dialect Notes",
                "Tags (comma-sep)",
                "General Attitude",
                "Traits (Enter to add)",
            ],
            values: Default::default(),
            focused: 0,
            traits: Vec::new(),
            editing_trait: false,
            trait_field: 0,
        }
    }

    fn from_profile(p: &PersonalityProfile) -> Self {
        let mut form = Self::new();
        form.values[Self::NAME] = p.name.clone();
        form.values[Self::FORMALITY] = p.speech_patterns.formality.to_string();
        form.values[Self::VOCABULARY] = p.speech_patterns.vocabulary_style.clone();
        form.values[Self::PACING] = p.speech_patterns.pacing.clone();
        form.values[Self::DIALECT] = p
            .speech_patterns
            .dialect_notes
            .clone()
            .unwrap_or_default();
        form.values[Self::TAGS] = p.tags.join(", ");
        form.values[Self::ATTITUDE] = p.behavioral_tendencies.general_attitude.clone();
        form.traits = p
            .traits
            .iter()
            .map(|t| {
                (
                    t.trait_name.clone(),
                    t.intensity,
                    t.manifestation.clone(),
                )
            })
            .collect();
        form
    }

    fn to_profile(&self, existing_id: Option<&str>) -> PersonalityProfile {
        let formality: u8 = self.values[Self::FORMALITY]
            .parse()
            .unwrap_or(5)
            .clamp(1, 10);

        let tags: Vec<String> = self.values[Self::TAGS]
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let traits: Vec<PersonalityTrait> = self
            .traits
            .iter()
            .map(|(name, intensity, manifestation)| PersonalityTrait {
                trait_name: name.clone(),
                intensity: *intensity,
                manifestation: manifestation.clone(),
            })
            .collect();

        let now = chrono::Utc::now().to_rfc3339();

        PersonalityProfile {
            id: existing_id.unwrap_or("").to_string(),
            name: self.values[Self::NAME].clone(),
            source: Some("custom".to_string()),
            speech_patterns: SpeechPatterns {
                formality,
                common_phrases: Vec::new(),
                vocabulary_style: self.values[Self::VOCABULARY].clone(),
                dialect_notes: if self.values[Self::DIALECT].is_empty() {
                    None
                } else {
                    Some(self.values[Self::DIALECT].clone())
                },
                pacing: self.values[Self::PACING].clone(),
            },
            traits,
            knowledge_areas: Vec::new(),
            behavioral_tendencies: BehavioralTendencies {
                conflict_response: String::new(),
                stranger_response: String::new(),
                authority_response: String::new(),
                help_response: String::new(),
                general_attitude: self.values[Self::ATTITUDE].clone(),
            },
            example_phrases: Vec::new(),
            tags,
            metadata: std::collections::HashMap::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    fn is_valid(&self) -> bool {
        !self.values[Self::NAME].trim().is_empty()
    }
}

// ── State ───────────────────────────────────────────────────────────────────

pub struct PersonalityState {
    profiles: Vec<PersonalityDisplay>,
    selected: usize,
    scroll: usize,
    detail_scroll: usize,
    show_detail: bool,
    modal: Option<PersonalityModal>,
    loading: bool,
    seeded: bool,
    form: PersonalityForm,
    input: InputBuffer,
    data_rx: mpsc::UnboundedReceiver<Vec<PersonalityDisplay>>,
    data_tx: mpsc::UnboundedSender<Vec<PersonalityDisplay>>,
}

impl PersonalityState {
    pub fn new() -> Self {
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        Self {
            profiles: Vec::new(),
            selected: 0,
            scroll: 0,
            detail_scroll: 0,
            show_detail: false,
            modal: None,
            loading: false,
            seeded: false,
            form: PersonalityForm::new(),
            input: InputBuffer::new(),
            data_rx,
            data_tx,
        }
    }

    // ── Data loading ────────────────────────────────────────────────────

    pub fn load(&mut self, services: &Services) {
        if self.loading {
            return;
        }
        self.loading = true;

        let personality = services.personality.clone();
        let tx = self.data_tx.clone();
        let needs_seed = !self.seeded;

        tokio::spawn(async move {
            let store = personality.store();

            // Seed presets if store is empty on first load
            if needs_seed && store.list().is_empty() {
                for (preset_id, _) in PRESET_NAMES {
                    if let Some(profile) = create_preset_personality(preset_id) {
                        let _ = store.create(profile);
                    }
                }
            }

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
            self.seeded = true;
            // Clamp selection
            if !self.profiles.is_empty() {
                self.selected = self.selected.min(self.profiles.len() - 1);
            } else {
                self.selected = 0;
            }
        }
    }

    // ── Input handling ──────────────────────────────────────────────────

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

        if self.modal.is_some() {
            return self.handle_modal_input(*code, *modifiers, services);
        }

        match (*modifiers, *code) {
            // Navigation
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                self.select_next();
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                self.select_prev();
                true
            }
            (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
                if !self.profiles.is_empty() {
                    self.selected = self.profiles.len() - 1;
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('g')) => {
                self.selected = 0;
                self.detail_scroll = 0;
                true
            }
            (KeyModifiers::NONE, KeyCode::PageDown) => {
                for _ in 0..10 {
                    self.select_next();
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::PageUp) => {
                for _ in 0..10 {
                    self.select_prev();
                }
                true
            }

            // Detail toggle
            (KeyModifiers::NONE, KeyCode::Enter) => {
                if !self.profiles.is_empty() {
                    self.show_detail = !self.show_detail;
                    self.detail_scroll = 0;
                }
                true
            }

            // CRUD
            (KeyModifiers::NONE, KeyCode::Char('a')) => {
                self.modal = Some(PersonalityModal::Create(CreatePhase::PickMethod));
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('e')) => {
                if let Some(display) = self.profiles.get(self.selected) {
                    let store = services.personality.store();
                    if let Ok(profile) = store.get(&display.id) {
                        self.form = PersonalityForm::from_profile(&profile);
                        self.input.clear();
                        for c in self.form.values[0].chars() {
                            self.input.insert_char(c);
                        }
                        self.modal = Some(PersonalityModal::Edit);
                    }
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('d')) => {
                if !self.profiles.is_empty() {
                    self.modal = Some(PersonalityModal::Delete);
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('p')) => {
                if let Some(display) = self.profiles.get(self.selected) {
                    let store = services.personality.store();
                    let prompt_text = match store.get(&display.id) {
                        Ok(profile) => profile.to_system_prompt(),
                        Err(_) => format!("(Could not load profile {})", display.id),
                    };
                    self.modal = Some(PersonalityModal::Preview {
                        scroll: 0,
                        prompt_text,
                    });
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.load(services);
                true
            }

            _ => false,
        }
    }

    fn handle_modal_input(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        services: &Services,
    ) -> bool {
        let modal = self.modal.take().unwrap();

        match modal {
            PersonalityModal::Create(phase) => {
                self.handle_create_input(phase, code, modifiers, services);
            }
            PersonalityModal::Edit => {
                self.handle_form_input(code, modifiers, services, true);
            }
            PersonalityModal::Delete => match (modifiers, code) {
                (KeyModifiers::NONE, KeyCode::Char('y') | KeyCode::Char('Y')) => {
                    self.delete_selected(services);
                }
                (KeyModifiers::NONE, KeyCode::Char('n'))
                | (KeyModifiers::NONE, KeyCode::Esc) => {}
                _ => {
                    self.modal = Some(PersonalityModal::Delete);
                }
            },
            PersonalityModal::Preview { scroll, prompt_text } => match (modifiers, code) {
                (KeyModifiers::NONE, KeyCode::Esc) => {}
                (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                    self.modal = Some(PersonalityModal::Preview {
                        scroll: scroll + 1,
                        prompt_text,
                    });
                }
                (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                    self.modal = Some(PersonalityModal::Preview {
                        scroll: scroll.saturating_sub(1),
                        prompt_text,
                    });
                }
                (KeyModifiers::NONE, KeyCode::PageDown) => {
                    self.modal = Some(PersonalityModal::Preview {
                        scroll: scroll + 15,
                        prompt_text,
                    });
                }
                (KeyModifiers::NONE, KeyCode::PageUp) => {
                    self.modal = Some(PersonalityModal::Preview {
                        scroll: scroll.saturating_sub(15),
                        prompt_text,
                    });
                }
                _ => {
                    self.modal = Some(PersonalityModal::Preview { scroll, prompt_text });
                }
            },
        }

        true
    }

    fn handle_create_input(
        &mut self,
        phase: CreatePhase,
        code: KeyCode,
        modifiers: KeyModifiers,
        services: &Services,
    ) {
        match phase {
            CreatePhase::PickMethod => match (modifiers, code) {
                (KeyModifiers::NONE, KeyCode::Esc) => {}
                (KeyModifiers::NONE, KeyCode::Char('p') | KeyCode::Char('P')) => {
                    self.modal = Some(PersonalityModal::Create(CreatePhase::Preset {
                        selected: 0,
                    }));
                }
                (KeyModifiers::NONE, KeyCode::Char('m') | KeyCode::Char('M')) => {
                    self.form = PersonalityForm::new();
                    self.form.values[PersonalityForm::FORMALITY] = "5".to_string();
                    self.input.clear();
                    self.modal = Some(PersonalityModal::Create(CreatePhase::Form));
                }
                _ => {
                    self.modal = Some(PersonalityModal::Create(CreatePhase::PickMethod));
                }
            },
            CreatePhase::Preset { selected } => match (modifiers, code) {
                (KeyModifiers::NONE, KeyCode::Esc) => {
                    self.modal = Some(PersonalityModal::Create(CreatePhase::PickMethod));
                }
                (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                    let next = (selected + 1).min(PRESET_NAMES.len() - 1);
                    self.modal =
                        Some(PersonalityModal::Create(CreatePhase::Preset { selected: next }));
                }
                (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                    self.modal = Some(PersonalityModal::Create(CreatePhase::Preset {
                        selected: selected.saturating_sub(1),
                    }));
                }
                (KeyModifiers::NONE, KeyCode::Enter) => {
                    if let Some((preset_id, _)) = PRESET_NAMES.get(selected) {
                        self.create_from_preset(preset_id, services);
                    }
                }
                _ => {
                    self.modal =
                        Some(PersonalityModal::Create(CreatePhase::Preset { selected }));
                }
            },
            CreatePhase::Form => {
                self.handle_form_input(code, modifiers, services, false);
            }
        }
    }

    fn handle_form_input(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        services: &Services,
        is_edit: bool,
    ) {
        let focused = self.form.focused;

        // Trait sub-editor
        if self.form.editing_trait && focused == PersonalityForm::TRAITS_FIELD {
            match (modifiers, code) {
                (KeyModifiers::NONE, KeyCode::Esc) => {
                    self.form.editing_trait = false;
                    self.input.clear();
                    self.restore_modal(is_edit);
                }
                (KeyModifiers::NONE, KeyCode::Tab) => {
                    // Cycle through trait sub-fields
                    let val = self.input.take();
                    let idx = self.form.traits.len() - 1;
                    match self.form.trait_field {
                        0 => self.form.traits[idx].0 = val,
                        1 => {
                            self.form.traits[idx].1 =
                                val.parse().unwrap_or(5).clamp(1, 10);
                        }
                        2 => self.form.traits[idx].2 = val,
                        _ => {}
                    }
                    self.form.trait_field = (self.form.trait_field + 1) % 3;
                    // Load next sub-field
                    let next_val = match self.form.trait_field {
                        0 => self.form.traits[idx].0.clone(),
                        1 => self.form.traits[idx].1.to_string(),
                        2 => self.form.traits[idx].2.clone(),
                        _ => String::new(),
                    };
                    for c in next_val.chars() {
                        self.input.insert_char(c);
                    }
                    self.restore_modal(is_edit);
                }
                (KeyModifiers::NONE, KeyCode::Enter) => {
                    // Save trait and exit sub-editor
                    let val = self.input.take();
                    let idx = self.form.traits.len() - 1;
                    match self.form.trait_field {
                        0 => self.form.traits[idx].0 = val,
                        1 => {
                            self.form.traits[idx].1 =
                                val.parse().unwrap_or(5).clamp(1, 10);
                        }
                        2 => self.form.traits[idx].2 = val,
                        _ => {}
                    }
                    // Remove empty traits
                    self.form
                        .traits
                        .retain(|(name, _, _)| !name.trim().is_empty());
                    self.form.editing_trait = false;
                    self.restore_modal(is_edit);
                }
                _ => {
                    self.route_text_input(code, modifiers);
                    self.restore_modal(is_edit);
                }
            }
            return;
        }

        match (modifiers, code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                // Close modal
            }
            (KeyModifiers::NONE, KeyCode::Tab) | (KeyModifiers::NONE, KeyCode::Down) => {
                self.form.values[focused] = self.input.take();
                let next = (focused + 1) % FORM_FIELD_COUNT;
                self.form.focused = next;
                self.load_field_into_input(next);
                self.restore_modal(is_edit);
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) | (KeyModifiers::NONE, KeyCode::Up) => {
                self.form.values[focused] = self.input.take();
                let prev = if focused == 0 {
                    FORM_FIELD_COUNT - 1
                } else {
                    focused - 1
                };
                self.form.focused = prev;
                self.load_field_into_input(prev);
                self.restore_modal(is_edit);
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                if focused == PersonalityForm::TRAITS_FIELD {
                    // Start trait sub-editor
                    self.form.traits.push((String::new(), 5, String::new()));
                    self.form.editing_trait = true;
                    self.form.trait_field = 0;
                    self.input.clear();
                    self.restore_modal(is_edit);
                } else {
                    // Save
                    self.form.values[focused] = self.input.take();
                    if self.form.is_valid() {
                        if is_edit {
                            self.update_from_form(services);
                        } else {
                            self.create_from_form(services);
                        }
                    } else {
                        // Re-load input for current field
                        self.load_field_into_input(focused);
                        self.restore_modal(is_edit);
                    }
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                // Ctrl+S to save from any field
                self.form.values[focused] = self.input.take();
                if self.form.is_valid() {
                    if is_edit {
                        self.update_from_form(services);
                    } else {
                        self.create_from_form(services);
                    }
                } else {
                    self.load_field_into_input(focused);
                    self.restore_modal(is_edit);
                }
            }
            _ => {
                self.route_text_input(code, modifiers);
                self.restore_modal(is_edit);
            }
        }
    }

    fn route_text_input(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match (modifiers, code) {
            (KeyModifiers::NONE, KeyCode::Char(c)) | (KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                self.input.insert_char(c);
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => self.input.backspace(),
            (KeyModifiers::NONE, KeyCode::Delete) => self.input.delete(),
            (KeyModifiers::NONE, KeyCode::Left) => self.input.move_left(),
            (KeyModifiers::NONE, KeyCode::Right) => self.input.move_right(),
            (KeyModifiers::NONE, KeyCode::Home) => self.input.move_home(),
            (KeyModifiers::NONE, KeyCode::End) => self.input.move_end(),
            _ => {}
        }
    }

    fn restore_modal(&mut self, is_edit: bool) {
        if is_edit {
            self.modal = Some(PersonalityModal::Edit);
        } else {
            self.modal = Some(PersonalityModal::Create(CreatePhase::Form));
        }
    }

    fn load_field_into_input(&mut self, field_idx: usize) {
        self.input.clear();
        if field_idx < FORM_FIELD_COUNT {
            for c in self.form.values[field_idx].chars() {
                self.input.insert_char(c);
            }
        }
    }

    // ── CRUD operations ─────────────────────────────────────────────────

    fn create_from_preset(&mut self, preset_id: &str, services: &Services) {
        let store = services.personality.store();
        if let Some(profile) = create_preset_personality(preset_id) {
            let _ = store.create(profile);
        }
        self.load(services);
    }

    fn create_from_form(&mut self, services: &Services) {
        let profile = self.form.to_profile(None);
        let store = services.personality.store();
        let _ = store.create(profile);
        self.load(services);
    }

    fn update_from_form(&mut self, services: &Services) {
        if let Some(display) = self.profiles.get(self.selected) {
            let profile = self.form.to_profile(Some(&display.id));
            let store = services.personality.store();
            let _ = store.update(&display.id, profile);
        }
        self.load(services);
    }

    fn delete_selected(&mut self, services: &Services) {
        if let Some(display) = self.profiles.get(self.selected) {
            let store = services.personality.store();
            let _ = store.delete(&display.id);
        }
        self.load(services);
    }

    // ── Selection helpers ────────────────────────────────────────────────

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

    // ── Rendering ───────────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if self.show_detail && !self.profiles.is_empty() {
            let chunks = Layout::horizontal([
                Constraint::Percentage(40),
                Constraint::Percentage(60),
            ])
            .split(area);
            self.render_list(frame, chunks[0]);
            self.render_detail(frame, chunks[1]);
        } else {
            self.render_list(frame, area);
        }

        if let Some(ref modal) = self.modal {
            self.render_modal(frame, area, modal);
        }
    }

    fn render_list(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Personality ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.loading && self.profiles.is_empty() {
            let loading = Paragraph::new(vec![
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Loading personalities...",
                        Style::default().fg(Color::DarkGray),
                    ),
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
                Span::styled(
                    "No personalities yet.",
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  Press "),
                Span::styled("a", Style::default().fg(Color::Cyan).bold()),
                Span::raw(" to add a personality profile."),
            ]));
        } else {
            // Header
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("  {:<20} {:>6}  {}", "Name", "Form.", "Traits"),
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            for (i, p) in self.profiles.iter().enumerate() {
                let is_selected = i == self.selected;
                let cursor = if is_selected { "▸ " } else { "  " };

                let source_tag = format!("[{}]", truncate(&p.source, 8));

                let trait_display = if p.trait_summary.is_empty() {
                    "—".to_string()
                } else {
                    truncate(&p.trait_summary, 30)
                };

                let row_style = if is_selected {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                lines.push(Line::from(vec![
                    Span::styled(
                        cursor.to_string(),
                        if is_selected {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        },
                    ),
                    Span::styled(
                        format!("{:<20}", truncate(&p.name, 20)),
                        row_style,
                    ),
                    Span::styled(
                        format!("{:>3}/10", p.formality),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw("  "),
                    Span::styled(trait_display, Style::default().fg(Color::DarkGray)),
                    Span::raw("  "),
                    Span::styled(source_tag, Style::default().fg(Color::DarkGray)),
                ]));
            }
        }

        // Footer
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(inner.width.saturating_sub(4) as usize)),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{} profiles", self.profiles.len()),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("a", Style::default().fg(Color::DarkGray)),
            Span::raw(":add "),
            Span::styled("e", Style::default().fg(Color::DarkGray)),
            Span::raw(":edit "),
            Span::styled("d", Style::default().fg(Color::DarkGray)),
            Span::raw(":delete "),
            Span::styled("p", Style::default().fg(Color::DarkGray)),
            Span::raw(":preview "),
            Span::styled("Enter", Style::default().fg(Color::DarkGray)),
            Span::raw(":detail "),
            Span::styled("r", Style::default().fg(Color::DarkGray)),
            Span::raw(":refresh"),
        ]));

        // Auto-scroll to keep selected visible
        let visible_height = inner.height as usize;
        let selected_line = 2 + self.selected; // header lines + index
        let scroll = if visible_height > 0 && selected_line >= self.scroll + visible_height {
            selected_line.saturating_sub(visible_height - 1)
        } else if selected_line < self.scroll {
            selected_line
        } else {
            self.scroll
        };

        let content = Paragraph::new(lines).scroll((scroll as u16, 0));
        frame.render_widget(content, inner);
    }

    fn render_detail(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Detail ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(display) = self.profiles.get(self.selected) else {
            return;
        };

        // Fetch full profile for detail view
        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));

        // Basic info
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Name: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                display.name.clone(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Source: ", Style::default().fg(Color::DarkGray)),
            Span::raw(display.source.clone()),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Formality: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}/10", display.formality),
                Style::default().fg(Color::Cyan),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Tags: ", Style::default().fg(Color::DarkGray)),
            Span::raw(display.tag_summary.clone()),
        ]));

        // Render the full profile details inline (we have access to store but not async)
        // Use the display data we already have
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  TRAITS:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));

        if display.trait_summary.is_empty() {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled("(none)", Style::default().fg(Color::DarkGray)),
            ]));
        } else {
            for trait_name in display.trait_summary.split(", ") {
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled("• ", Style::default().fg(Color::Cyan)),
                    Span::raw(trait_name.to_string()),
                ]));
            }
        }

        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Press ", Style::default().fg(Color::DarkGray)),
            Span::styled("p", Style::default().fg(Color::Cyan).bold()),
            Span::styled(
                " to preview system prompt",
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        let content = Paragraph::new(lines).scroll((self.detail_scroll as u16, 0));
        frame.render_widget(content, inner);
    }

    fn render_modal(&self, frame: &mut Frame, area: Rect, modal: &PersonalityModal) {
        match modal {
            PersonalityModal::Create(phase) => match phase {
                CreatePhase::PickMethod => self.render_pick_method(frame, area),
                CreatePhase::Preset { selected } => self.render_preset_picker(frame, area, *selected),
                CreatePhase::Form => self.render_form_modal(frame, area, "Create Personality"),
            },
            PersonalityModal::Edit => self.render_form_modal(frame, area, "Edit Personality"),
            PersonalityModal::Delete => self.render_delete_confirm(frame, area),
            PersonalityModal::Preview { scroll, prompt_text } => {
                self.render_preview(frame, area, *scroll, prompt_text);
            }
        }
    }

    fn render_pick_method(&self, frame: &mut Frame, area: Rect) {
        let modal_area = centered_rect(40, 30, area);

        let lines = vec![
            Line::raw(""),
            Line::from(Span::styled(
                "  Create Personality",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("[P]", Style::default().fg(Color::Cyan).bold()),
                Span::raw("  From preset"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("[M]", Style::default().fg(Color::Cyan).bold()),
                Span::raw("  Manual entry"),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Esc", Style::default().fg(Color::DarkGray)),
                Span::raw(": cancel"),
            ]),
        ];

        let block = Block::default()
            .title(" New Personality ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        frame.render_widget(Clear, modal_area);
        frame.render_widget(Paragraph::new(lines).block(block), modal_area);
    }

    fn render_preset_picker(&self, frame: &mut Frame, area: Rect, selected: usize) {
        let modal_area = centered_rect(50, 40, area);

        let mut lines = vec![
            Line::raw(""),
            Line::from(Span::styled(
                "  Select Preset",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
        ];

        for (i, (_, display_name)) in PRESET_NAMES.iter().enumerate() {
            let is_sel = i == selected;
            let cursor = if is_sel { "▸ " } else { "  " };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {cursor}"),
                    if is_sel {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(
                    display_name.to_string(),
                    if is_sel {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
            ]));
        }

        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("j/k", Style::default().fg(Color::DarkGray)),
            Span::raw(":select "),
            Span::styled("Enter", Style::default().fg(Color::DarkGray)),
            Span::raw(":create "),
            Span::styled("Esc", Style::default().fg(Color::DarkGray)),
            Span::raw(":back"),
        ]));

        let block = Block::default()
            .title(" Presets ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        frame.render_widget(Clear, modal_area);
        frame.render_widget(Paragraph::new(lines).block(block), modal_area);
    }

    fn render_form_modal(&self, frame: &mut Frame, area: Rect, title: &str) {
        let modal_area = centered_rect(60, 75, area);

        let mut lines = vec![
            Line::raw(""),
            Line::from(Span::styled(
                format!("  {title}"),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
        ];

        for (i, label) in self.form.labels.iter().enumerate() {
            let is_focused = i == self.form.focused;
            let marker = if is_focused { "▸" } else { " " };
            let label_style = if is_focused {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default().fg(Color::DarkGray)
            };

            if i == PersonalityForm::TRAITS_FIELD {
                // Trait field — show list + add prompt
                lines.push(Line::from(vec![
                    Span::raw(format!("  {marker} ")),
                    Span::styled(format!("{label}:"), label_style),
                ]));

                for (j, (name, intensity, manifestation)) in self.form.traits.iter().enumerate() {
                    let editing_this =
                        is_focused && self.form.editing_trait && j == self.form.traits.len() - 1;

                    if editing_this {
                        // Show inline editor
                        let sub_labels = ["name", "intensity", "manifestation"];
                        let sub_vals = [
                            name.as_str(),
                            &intensity.to_string(),
                            manifestation.as_str(),
                        ];
                        for (si, (sl, sv)) in sub_labels.iter().zip(sub_vals.iter()).enumerate() {
                            let is_active = si == self.form.trait_field;
                            let display_val = if is_active {
                                format!("{}▎", self.input.text())
                            } else {
                                sv.to_string()
                            };
                            lines.push(Line::from(vec![
                                Span::raw("      "),
                                Span::styled(
                                    format!("{sl}: "),
                                    if is_active {
                                        Style::default().fg(Color::Yellow)
                                    } else {
                                        Style::default().fg(Color::DarkGray)
                                    },
                                ),
                                Span::styled(
                                    display_val,
                                    if is_active {
                                        Style::default().fg(Color::White)
                                    } else {
                                        Style::default()
                                    },
                                ),
                            ]));
                        }
                    } else {
                        lines.push(Line::from(vec![
                            Span::raw("      "),
                            Span::styled("• ", Style::default().fg(Color::Cyan)),
                            Span::raw(format!(
                                "{name} ({intensity}/10) — {manifestation}"
                            )),
                        ]));
                    }
                }

                if self.form.traits.is_empty() && !self.form.editing_trait {
                    lines.push(Line::from(vec![
                        Span::raw("      "),
                        Span::styled("(none — press Enter to add)", Style::default().fg(Color::DarkGray)),
                    ]));
                }
            } else {
                // Regular text field
                let display_val = if is_focused {
                    format!("{}▎", self.input.text())
                } else {
                    let val = &self.form.values[i];
                    if val.is_empty() {
                        "(empty)".to_string()
                    } else {
                        val.clone()
                    }
                };

                let val_style = if is_focused {
                    Style::default().fg(Color::White)
                } else if self.form.values[i].is_empty() {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                };

                lines.push(Line::from(vec![
                    Span::raw(format!("  {marker} ")),
                    Span::styled(format!("{:<20}", format!("{label}:")), label_style),
                    Span::styled(display_val, val_style),
                ]));
            }
        }

        // Validation hint
        if !self.form.is_valid() {
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("⚠ Name is required", Style::default().fg(Color::Red)),
            ]));
        }

        // Footer
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Tab/↓↑", Style::default().fg(Color::DarkGray)),
            Span::raw(":fields "),
            Span::styled("Enter", Style::default().fg(Color::DarkGray)),
            Span::raw(":save "),
            Span::styled("Ctrl+S", Style::default().fg(Color::DarkGray)),
            Span::raw(":save "),
            Span::styled("Esc", Style::default().fg(Color::DarkGray)),
            Span::raw(":cancel"),
        ]));

        let block = Block::default()
            .title(format!(" {title} "))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        frame.render_widget(Clear, modal_area);
        frame.render_widget(Paragraph::new(lines).block(block), modal_area);
    }

    fn render_delete_confirm(&self, frame: &mut Frame, area: Rect) {
        let modal_area = centered_rect(40, 25, area);

        let name = self
            .profiles
            .get(self.selected)
            .map(|p| p.name.as_str())
            .unwrap_or("?");

        let lines = vec![
            Line::raw(""),
            Line::from(Span::styled(
                "  Confirm Deletion",
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(vec![
                Span::raw("  Delete "),
                Span::styled(
                    name.to_string(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("?"),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("[Y]", Style::default().fg(Color::Red).bold()),
                Span::raw("es  "),
                Span::styled("[N]", Style::default().fg(Color::Green).bold()),
                Span::raw("o"),
            ]),
        ];

        let block = Block::default()
            .title(" Delete ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red));

        frame.render_widget(Clear, modal_area);
        frame.render_widget(Paragraph::new(lines).block(block), modal_area);
    }

    fn render_preview(&self, frame: &mut Frame, area: Rect, scroll: usize, prompt_text: &str) {
        let modal_area = centered_rect(70, 80, area);

        let name = self
            .profiles
            .get(self.selected)
            .map(|p| p.name.as_str())
            .unwrap_or("Preview");

        let lines: Vec<Line<'static>> = prompt_text
            .lines()
            .map(|line| Line::from(Span::raw(format!("  {}", line.to_string()))))
            .collect();

        let mut all_lines = vec![Line::raw("")];
        all_lines.extend(lines);
        all_lines.push(Line::raw(""));
        all_lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("j/k", Style::default().fg(Color::DarkGray)),
            Span::raw(":scroll "),
            Span::styled("PgUp/PgDn", Style::default().fg(Color::DarkGray)),
            Span::raw(":page "),
            Span::styled("Esc", Style::default().fg(Color::DarkGray)),
            Span::raw(":close"),
        ]));

        let block = Block::default()
            .title(format!(" Preview: {name} "))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        frame.render_widget(Clear, modal_area);
        frame.render_widget(
            Paragraph::new(all_lines)
                .block(block)
                .scroll((scroll as u16, 0)),
            modal_area,
        );
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max.saturating_sub(1)])
    } else {
        s.to_string()
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_personality_state_new() {
        let state = PersonalityState::new();
        assert!(state.profiles.is_empty());
        assert_eq!(state.selected, 0);
        assert!(!state.show_detail);
        assert!(state.modal.is_none());
        assert!(!state.loading);
    }

    #[test]
    fn test_personality_display_from_profile() {
        let profile = PersonalityProfile {
            id: "test-id".to_string(),
            name: "Test NPC".to_string(),
            source: Some("preset".to_string()),
            speech_patterns: SpeechPatterns {
                formality: 7,
                common_phrases: vec!["hello".to_string()],
                vocabulary_style: "formal".to_string(),
                dialect_notes: None,
                pacing: "measured".to_string(),
            },
            traits: vec![
                PersonalityTrait {
                    trait_name: "brave".to_string(),
                    intensity: 8,
                    manifestation: "stands firm".to_string(),
                },
                PersonalityTrait {
                    trait_name: "kind".to_string(),
                    intensity: 6,
                    manifestation: "helps others".to_string(),
                },
            ],
            knowledge_areas: Vec::new(),
            behavioral_tendencies: BehavioralTendencies {
                conflict_response: String::new(),
                stranger_response: String::new(),
                authority_response: String::new(),
                help_response: String::new(),
                general_attitude: "friendly".to_string(),
            },
            example_phrases: Vec::new(),
            tags: vec!["npc".to_string(), "tavern".to_string()],
            metadata: std::collections::HashMap::new(),
            created_at: "2026-02-23T00:00:00Z".to_string(),
            updated_at: "2026-02-23T00:00:00Z".to_string(),
        };

        let display = PersonalityDisplay::from_profile(&profile);
        assert_eq!(display.id, "test-id");
        assert_eq!(display.name, "Test NPC");
        assert_eq!(display.source, "preset");
        assert_eq!(display.formality, 7);
        assert!(display.trait_summary.contains("brave"));
        assert!(display.trait_summary.contains("kind"));
        assert!(display.tag_summary.contains("npc"));
    }

    #[test]
    fn test_personality_form_roundtrip() {
        let profile = PersonalityProfile {
            id: "original-id".to_string(),
            name: "Bartender".to_string(),
            source: Some("custom".to_string()),
            speech_patterns: SpeechPatterns {
                formality: 3,
                common_phrases: Vec::new(),
                vocabulary_style: "colloquial".to_string(),
                dialect_notes: Some("slight accent".to_string()),
                pacing: "rapid".to_string(),
            },
            traits: vec![PersonalityTrait {
                trait_name: "chatty".to_string(),
                intensity: 9,
                manifestation: "never shuts up".to_string(),
            }],
            knowledge_areas: Vec::new(),
            behavioral_tendencies: BehavioralTendencies {
                conflict_response: String::new(),
                stranger_response: String::new(),
                authority_response: String::new(),
                help_response: String::new(),
                general_attitude: "welcoming".to_string(),
            },
            example_phrases: Vec::new(),
            tags: vec!["barkeep".to_string(), "social".to_string()],
            metadata: std::collections::HashMap::new(),
            created_at: "2026-02-23T00:00:00Z".to_string(),
            updated_at: "2026-02-23T00:00:00Z".to_string(),
        };

        let form = PersonalityForm::from_profile(&profile);
        assert_eq!(form.values[PersonalityForm::NAME], "Bartender");
        assert_eq!(form.values[PersonalityForm::FORMALITY], "3");
        assert_eq!(form.values[PersonalityForm::VOCABULARY], "colloquial");
        assert_eq!(form.values[PersonalityForm::PACING], "rapid");
        assert_eq!(form.values[PersonalityForm::DIALECT], "slight accent");
        assert!(form.values[PersonalityForm::TAGS].contains("barkeep"));
        assert_eq!(form.values[PersonalityForm::ATTITUDE], "welcoming");
        assert_eq!(form.traits.len(), 1);
        assert_eq!(form.traits[0].0, "chatty");
        assert_eq!(form.traits[0].1, 9);

        // Round-trip back to profile
        let rebuilt = form.to_profile(Some("original-id"));
        assert_eq!(rebuilt.name, "Bartender");
        assert_eq!(rebuilt.speech_patterns.formality, 3);
        assert_eq!(rebuilt.speech_patterns.vocabulary_style, "colloquial");
        assert_eq!(rebuilt.traits.len(), 1);
        assert_eq!(rebuilt.traits[0].trait_name, "chatty");
        assert_eq!(rebuilt.tags, vec!["barkeep", "social"]);
    }

    #[test]
    fn test_form_validation() {
        let form = PersonalityForm::new();
        assert!(!form.is_valid()); // empty name

        let mut form = PersonalityForm::new();
        form.values[PersonalityForm::NAME] = "Test".to_string();
        assert!(form.is_valid());

        let mut form = PersonalityForm::new();
        form.values[PersonalityForm::NAME] = "   ".to_string();
        assert!(!form.is_valid()); // whitespace-only
    }

    #[test]
    fn test_formality_clamping() {
        let mut form = PersonalityForm::new();
        form.values[PersonalityForm::NAME] = "Test".to_string();
        form.values[PersonalityForm::FORMALITY] = "15".to_string();
        let profile = form.to_profile(None);
        assert_eq!(profile.speech_patterns.formality, 10);

        form.values[PersonalityForm::FORMALITY] = "0".to_string();
        let profile = form.to_profile(None);
        assert_eq!(profile.speech_patterns.formality, 1);

        form.values[PersonalityForm::FORMALITY] = "not a number".to_string();
        let profile = form.to_profile(None);
        assert_eq!(profile.speech_patterns.formality, 5); // default on parse failure
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hell…");
        assert_eq!(truncate("", 5), "");
    }

    #[test]
    fn test_select_bounds() {
        let mut state = PersonalityState::new();
        state.select_next();
        assert_eq!(state.selected, 0);
        state.select_prev();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn test_display_empty_tags() {
        let profile = PersonalityProfile {
            id: "id".to_string(),
            name: "Name".to_string(),
            source: None,
            speech_patterns: SpeechPatterns {
                formality: 5,
                common_phrases: Vec::new(),
                vocabulary_style: String::new(),
                dialect_notes: None,
                pacing: String::new(),
            },
            traits: Vec::new(),
            knowledge_areas: Vec::new(),
            behavioral_tendencies: BehavioralTendencies {
                conflict_response: String::new(),
                stranger_response: String::new(),
                authority_response: String::new(),
                help_response: String::new(),
                general_attitude: String::new(),
            },
            example_phrases: Vec::new(),
            tags: Vec::new(),
            metadata: std::collections::HashMap::new(),
            created_at: String::new(),
            updated_at: String::new(),
        };

        let display = PersonalityDisplay::from_profile(&profile);
        assert_eq!(display.source, "custom");
        assert_eq!(display.tag_summary, "—");
        assert!(display.trait_summary.is_empty());
    }
}
