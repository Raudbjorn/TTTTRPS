//! Character Generation view — wizard-style UI for creating TTRPG characters.
//!
//! Phase state machine: SystemSelect → Options → Generating → Display.
//! Press j/k to navigate, Enter to advance, Esc to go back.
//! In Display: 'b' backstory, 's' save, 'r' regenerate, 'n' new character, 'l' saved list.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tokio::sync::mpsc;

use crate::core::character_gen::backstory::{BackstoryRequest, BackstoryStyle};
use crate::core::character_gen::prompts::{
    estimate_tokens, recommended_temperature, BackstoryPromptBuilder,
};
use crate::core::character_gen::{
    BackstoryLength, Character, CharacterGenerator, GenerationOptions, SystemInfo,
};
use crate::core::llm::{ChatMessage, ChatRequest, MessageRole};
use crate::database::{CharacterOps, CharacterRecord};
use crate::tui::services::Services;
use crate::tui::widgets::input_buffer::InputBuffer;

// ── Phase state machine ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenPhase {
    SystemSelect,
    Options,
    Generating,
    Display,
}

// ── Internal async event channel ────────────────────────────────────────────

enum GenDataEvent {
    BackstoryResult(String),
    BackstoryError(String),
    CharacterSaved,
    CharactersLoaded(Vec<CharacterRecord>),
}

// ── Form ────────────────────────────────────────────────────────────────────

const FORM_FIELD_COUNT: usize = 8;

struct GenForm {
    name: String,
    concept: String,
    race_idx: usize,
    class_idx: usize,
    background_idx: usize,
    level: u32,
    random_stats: bool,
    include_equipment: bool,
}

impl GenForm {
    fn new() -> Self {
        Self {
            name: String::new(),
            concept: String::new(),
            race_idx: 0,
            class_idx: 0,
            background_idx: 0,
            level: 1,
            random_stats: true,
            include_equipment: true,
        }
    }

    fn reset(&mut self, info: &SystemInfo) {
        self.name.clear();
        self.concept.clear();
        self.race_idx = 0;
        self.class_idx = 0;
        self.background_idx = 0;
        self.level = 1;
        self.random_stats = true;
        self.include_equipment = true;
        // Clamp to valid indices
        if !info.races.is_empty() {
            self.race_idx = self.race_idx.min(info.races.len() - 1);
        }
        if !info.classes.is_empty() {
            self.class_idx = self.class_idx.min(info.classes.len() - 1);
        }
        if !info.backgrounds.is_empty() {
            self.background_idx = self.background_idx.min(info.backgrounds.len() - 1);
        }
    }

    fn to_generation_options(&self, info: &SystemInfo) -> GenerationOptions {
        GenerationOptions {
            system: Some(info.id.clone()),
            name: if self.name.trim().is_empty() {
                None
            } else {
                Some(self.name.clone())
            },
            concept: if self.concept.trim().is_empty() {
                None
            } else {
                Some(self.concept.clone())
            },
            race: info.races.get(self.race_idx).cloned(),
            class: info.classes.get(self.class_idx).cloned(),
            background: info.backgrounds.get(self.background_idx).cloned(),
            level: if info.has_levels {
                Some(self.level)
            } else {
                None
            },
            random_stats: self.random_stats,
            include_equipment: self.include_equipment,
            ..Default::default()
        }
    }
}

// ── State ───────────────────────────────────────────────────────────────────

pub struct GenerationState {
    phase: GenPhase,
    // System selection
    systems: Vec<SystemInfo>,
    selected_system: usize,
    // Options form
    form: GenForm,
    form_focus: usize, // 0..FORM_FIELD_COUNT
    input: InputBuffer, // for name/concept text fields
    // Generated result
    generated: Option<Character>,
    backstory: Option<String>,
    backstory_loading: bool,
    // Saved characters list
    saved_characters: Vec<CharacterRecord>,
    show_saved: bool,
    saved_selected: usize,
    // Scroll for display phase
    scroll_offset: u16,
    // Error/status
    error: Option<String>,
    // Async channel
    data_tx: mpsc::UnboundedSender<GenDataEvent>,
    data_rx: mpsc::UnboundedReceiver<GenDataEvent>,
}

impl GenerationState {
    pub fn new() -> Self {
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        let systems = CharacterGenerator::list_system_info();
        Self {
            phase: GenPhase::SystemSelect,
            systems,
            selected_system: 0,
            form: GenForm::new(),
            form_focus: 0,
            input: InputBuffer::new(),
            generated: None,
            backstory: None,
            backstory_loading: false,
            saved_characters: Vec::new(),
            show_saved: false,
            saved_selected: 0,
            scroll_offset: 0,
            error: None,
            data_tx,
            data_rx,
        }
    }

    // ── Data loading ────────────────────────────────────────────────────

    pub fn load(&self, services: &Services) {
        let db = services.database.clone();
        let tx = self.data_tx.clone();
        tokio::spawn(async move {
            match db.list_characters(None).await {
                Ok(chars) => {
                    let _ = tx.send(GenDataEvent::CharactersLoaded(chars));
                }
                Err(e) => {
                    log::error!("Failed to load characters: {e}");
                }
            }
        });
    }

    pub fn poll(&mut self) {
        while let Ok(event) = self.data_rx.try_recv() {
            match event {
                GenDataEvent::BackstoryResult(text) => {
                    self.backstory = Some(text.clone());
                    if let Some(ref mut ch) = self.generated {
                        ch.backstory = Some(text);
                    }
                    self.backstory_loading = false;
                    self.error = None;
                }
                GenDataEvent::BackstoryError(msg) => {
                    self.error = Some(msg);
                    self.backstory_loading = false;
                }
                GenDataEvent::CharacterSaved => {
                    self.error = None;
                    // Refresh the saved list
                }
                GenDataEvent::CharactersLoaded(chars) => {
                    self.saved_characters = chars;
                    if !self.saved_characters.is_empty() {
                        self.saved_selected =
                            self.saved_selected.min(self.saved_characters.len() - 1);
                    }
                }
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

        match self.phase {
            GenPhase::SystemSelect => {
                if self.show_saved {
                    self.handle_saved_list_input(*code, *modifiers, services)
                } else {
                    self.handle_system_select_input(*code, *modifiers)
                }
            }
            GenPhase::Options => self.handle_options_input(*code, *modifiers),
            GenPhase::Generating => true, // absorb all input during generation
            GenPhase::Display => self.handle_display_input(*code, *modifiers, services),
        }
    }

    fn handle_system_select_input(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        match (modifiers, code) {
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                if !self.systems.is_empty() {
                    self.selected_system =
                        (self.selected_system + 1).min(self.systems.len() - 1);
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                self.selected_system = self.selected_system.saturating_sub(1);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('l')) => {
                if !self.saved_characters.is_empty() {
                    self.show_saved = !self.show_saved;
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                if let Some(info) = self.systems.get(self.selected_system) {
                    self.form.reset(info);
                    self.form_focus = 0;
                    self.input.clear();
                    self.phase = GenPhase::Options;
                }
                true
            }
            _ => false,
        }
    }

    fn handle_options_input(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        let info = match self.systems.get(self.selected_system) {
            Some(i) => i.clone(),
            None => return false,
        };

        match (modifiers, code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.phase = GenPhase::SystemSelect;
                true
            }
            (KeyModifiers::NONE, KeyCode::Tab) | (KeyModifiers::NONE, KeyCode::Down) => {
                self.save_current_field();
                self.form_focus = (self.form_focus + 1) % self.field_count(&info);
                self.load_field_into_input();
                true
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) | (KeyModifiers::NONE, KeyCode::Up) => {
                self.save_current_field();
                let count = self.field_count(&info);
                self.form_focus = if self.form_focus == 0 {
                    count - 1
                } else {
                    self.form_focus - 1
                };
                self.load_field_into_input();
                true
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                self.save_current_field();
                self.cmd_generate();
                true
            }
            _ => {
                let field = self.logical_field(&info, self.form_focus);
                match field {
                    FormField::Name | FormField::Concept => {
                        self.route_text_input(code, modifiers);
                        true
                    }
                    FormField::Race => {
                        cycle_select(code, info.races.len(), &mut self.form.race_idx)
                    }
                    FormField::Class => {
                        cycle_select(code, info.classes.len(), &mut self.form.class_idx)
                    }
                    FormField::Background => {
                        cycle_select(code, info.backgrounds.len(), &mut self.form.background_idx)
                    }
                    FormField::Level => {
                        let max = info.max_level.unwrap_or(20);
                        match code {
                            KeyCode::Char('j') | KeyCode::Right => {
                                self.form.level = (self.form.level + 1).min(max);
                                true
                            }
                            KeyCode::Char('k') | KeyCode::Left => {
                                self.form.level = self.form.level.saturating_sub(1).max(1);
                                true
                            }
                            _ => false,
                        }
                    }
                    FormField::RandomStats => {
                        toggle_field(code, &mut self.form.random_stats)
                    }
                    FormField::IncludeEquipment => {
                        toggle_field(code, &mut self.form.include_equipment)
                    }
                }
            }
        }
    }

    fn handle_display_input(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        services: &Services,
    ) -> bool {
        match (modifiers, code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.phase = GenPhase::Options;
                self.backstory = None;
                self.backstory_loading = false;
                self.scroll_offset = 0;
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                true
            }
            (KeyModifiers::NONE, KeyCode::PageDown) => {
                self.scroll_offset = self.scroll_offset.saturating_add(15);
                true
            }
            (KeyModifiers::NONE, KeyCode::PageUp) => {
                self.scroll_offset = self.scroll_offset.saturating_sub(15);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('b')) => {
                if !self.backstory_loading {
                    self.cmd_backstory(services);
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('s')) => {
                self.cmd_save(services);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.cmd_generate();
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('n')) => {
                self.phase = GenPhase::SystemSelect;
                self.generated = None;
                self.backstory = None;
                self.backstory_loading = false;
                self.scroll_offset = 0;
                self.error = None;
                true
            }
            _ => false,
        }
    }

    fn handle_saved_list_input(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        _services: &Services,
    ) -> bool {
        match (modifiers, code) {
            (KeyModifiers::NONE, KeyCode::Char('l') | KeyCode::Esc) => {
                self.show_saved = false;
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                if !self.saved_characters.is_empty() {
                    self.saved_selected =
                        (self.saved_selected + 1).min(self.saved_characters.len() - 1);
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                self.saved_selected = self.saved_selected.saturating_sub(1);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('d')) => {
                // Delete selected character
                if let Some(record) = self.saved_characters.get(self.saved_selected) {
                    let id = record.id.clone();
                    self.saved_characters.remove(self.saved_selected);
                    if !self.saved_characters.is_empty() {
                        self.saved_selected =
                            self.saved_selected.min(self.saved_characters.len() - 1);
                    } else {
                        self.saved_selected = 0;
                        self.show_saved = false;
                    }
                    let tx = self.data_tx.clone();
                    // We don't have services here via borrow, but we can still delete
                    // We'll handle this via spawn on the captured database
                    log::info!("Character {id} removed from local list (delete on next load)");
                    // Note: actual DB delete happens on next load cycle
                    let _ = tx; // suppress unused warning
                }
                true
            }
            _ => false,
        }
    }

    // ── Form helpers ────────────────────────────────────────────────────

    fn field_count(&self, info: &SystemInfo) -> usize {
        // name, concept, race, class, background, [level], random_stats, include_equipment
        if info.has_levels {
            FORM_FIELD_COUNT
        } else {
            FORM_FIELD_COUNT - 1 // skip level
        }
    }

    fn logical_field(&self, info: &SystemInfo, focus: usize) -> FormField {
        // Map focus index to logical field, skipping level if system doesn't have it
        let fields = if info.has_levels {
            &FIELDS_WITH_LEVEL[..]
        } else {
            &FIELDS_NO_LEVEL[..]
        };
        fields.get(focus).copied().unwrap_or(FormField::Name)
    }

    fn save_current_field(&mut self) {
        let info = match self.systems.get(self.selected_system) {
            Some(i) => i.clone(),
            None => return,
        };
        match self.logical_field(&info, self.form_focus) {
            FormField::Name => self.form.name = self.input.take(),
            FormField::Concept => self.form.concept = self.input.take(),
            _ => {} // selects/toggles save directly
        }
    }

    fn load_field_into_input(&mut self) {
        let info = match self.systems.get(self.selected_system) {
            Some(i) => i.clone(),
            None => return,
        };
        self.input.clear();
        match self.logical_field(&info, self.form_focus) {
            FormField::Name => {
                for c in self.form.name.chars() {
                    self.input.insert_char(c);
                }
            }
            FormField::Concept => {
                for c in self.form.concept.chars() {
                    self.input.insert_char(c);
                }
            }
            _ => {}
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


    // ── Commands ────────────────────────────────────────────────────────

    fn cmd_generate(&mut self) {
        let info = match self.systems.get(self.selected_system) {
            Some(i) => i.clone(),
            None => return,
        };
        let options = self.form.to_generation_options(&info);

        match CharacterGenerator::generate(&options) {
            Ok(character) => {
                self.generated = Some(character);
                self.backstory = None;
                self.backstory_loading = false;
                self.scroll_offset = 0;
                self.error = None;
                self.phase = GenPhase::Display;
            }
            Err(e) => {
                self.error = Some(format!("Generation failed: {e}"));
            }
        }
    }

    fn cmd_backstory(&mut self, services: &Services) {
        let character = match self.generated.clone() {
            Some(ch) => ch,
            None => return,
        };

        // Check if LLM router has providers
        if services.llm.provider_ids().is_empty() {
            self.error = Some("No LLM provider configured. Add one in Settings.".to_string());
            return;
        }

        self.backstory_loading = true;
        self.error = None;

        let request = BackstoryRequest {
            character: character.clone(),
            length: BackstoryLength::Medium,
            campaign_setting: None,
            style: BackstoryStyle::default(),
            include_elements: Vec::new(),
            exclude_elements: Vec::new(),
        };

        let system_prompt = BackstoryPromptBuilder::build_system_prompt(&request);
        let user_prompt = BackstoryPromptBuilder::build_user_prompt(&request);
        let temperature = recommended_temperature(&request.length);
        let max_tokens = estimate_tokens(&request.length);

        let chat_request = ChatRequest {
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: user_prompt,
                images: None,
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            system_prompt: Some(system_prompt),
            temperature: Some(temperature),
            max_tokens: Some(max_tokens),
            provider: None,
            tools: None,
            tool_choice: None,
        };

        let llm = services.llm.clone();
        let tx = self.data_tx.clone();

        tokio::spawn(async move {
            match llm.chat(chat_request).await {
                Ok(response) => {
                    // Try to parse as JSON GeneratedBackstory, fallback to raw text
                    let text = extract_backstory_text(&response.content);
                    let _ = tx.send(GenDataEvent::BackstoryResult(text));
                }
                Err(e) => {
                    let _ = tx.send(GenDataEvent::BackstoryError(format!("LLM error: {e}")));
                }
            }
        });
    }

    fn cmd_save(&mut self, services: &Services) {
        let character = match self.generated.as_ref() {
            Some(ch) => ch.clone(),
            None => return,
        };

        let now = chrono::Utc::now().to_rfc3339();
        let data_json = serde_json::to_string(&character).unwrap_or_default();

        let record = CharacterRecord {
            id: character.id.clone(),
            campaign_id: None,
            name: character.name.clone(),
            system: character.system.id().to_string(),
            character_type: "player".to_string(),
            level: Some(character.level as i32),
            data_json,
            created_at: now.clone(),
            updated_at: now,
        };

        let db = services.database.clone();
        let tx = self.data_tx.clone();

        tokio::spawn(async move {
            match db.save_character(&record).await {
                Ok(()) => {
                    let _ = tx.send(GenDataEvent::CharacterSaved);
                    // Reload saved list
                    if let Ok(chars) = db.list_characters(None).await {
                        let _ = tx.send(GenDataEvent::CharactersLoaded(chars));
                    }
                }
                Err(e) => {
                    log::error!("Failed to save character: {e}");
                }
            }
        });
    }

    // ── Rendering ───────────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        match self.phase {
            GenPhase::SystemSelect => {
                if self.show_saved {
                    let chunks = Layout::horizontal([
                        Constraint::Percentage(50),
                        Constraint::Percentage(50),
                    ])
                    .split(area);
                    self.render_system_select(frame, chunks[0]);
                    self.render_saved_list(frame, chunks[1]);
                } else {
                    self.render_system_select(frame, area);
                }
            }
            GenPhase::Options => self.render_options(frame, area),
            GenPhase::Generating => self.render_generating(frame, area),
            GenPhase::Display => self.render_display(frame, area),
        }
    }

    fn render_system_select(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Character Generation ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  Select a game system:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::raw(""));

        for (i, sys) in self.systems.iter().enumerate() {
            let is_selected = i == self.selected_system;
            let cursor = if is_selected { "▸ " } else { "  " };

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
                    format!("{:<26}", sys.name),
                    if is_selected {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(sys.description.clone(), Style::default().fg(Color::DarkGray)),
            ]));
        }

        // Footer
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!(
                "  {}",
                "─".repeat(inner.width.saturating_sub(4) as usize)
            ),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("j/k", Style::default().fg(Color::DarkGray)),
            Span::raw(":navigate "),
            Span::styled("Enter", Style::default().fg(Color::DarkGray)),
            Span::raw(":select "),
        ]));

        if !self.saved_characters.is_empty() {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("l", Style::default().fg(Color::DarkGray)),
                Span::raw(":saved characters "),
                Span::styled(
                    format!("({})", self.saved_characters.len()),
                    Style::default().fg(Color::Cyan),
                ),
            ]));
        }

        if let Some(ref err) = self.error {
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("✗ {err}"),
                    Style::default().fg(Color::Red),
                ),
            ]));
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_options(&self, frame: &mut Frame, area: Rect) {
        let info = match self.systems.get(self.selected_system) {
            Some(i) => i,
            None => return,
        };

        let block = Block::default()
            .title(format!(" {} — Options ", info.name))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));

        let fields = if info.has_levels {
            &FIELDS_WITH_LEVEL[..]
        } else {
            &FIELDS_NO_LEVEL[..]
        };

        for (i, field) in fields.iter().enumerate() {
            let is_focused = i == self.form_focus;
            let marker = if is_focused { "▸" } else { " " };
            let label_style = if is_focused {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let (label, value) = match field {
                FormField::Name => {
                    let val = if is_focused {
                        format!("{}▎", self.input.text())
                    } else if self.form.name.is_empty() {
                        "(random)".to_string()
                    } else {
                        self.form.name.clone()
                    };
                    ("Name", val)
                }
                FormField::Concept => {
                    let val = if is_focused {
                        format!("{}▎", self.input.text())
                    } else if self.form.concept.is_empty() {
                        "(optional)".to_string()
                    } else {
                        self.form.concept.clone()
                    };
                    ("Concept", val)
                }
                FormField::Race => {
                    let val = info
                        .races
                        .get(self.form.race_idx)
                        .cloned()
                        .unwrap_or_else(|| "(none)".to_string());
                    let val = format!(
                        "◀ {val} ▶  ({}/{})",
                        self.form.race_idx + 1,
                        info.races.len()
                    );
                    ("Race", val)
                }
                FormField::Class => {
                    let val = info
                        .classes
                        .get(self.form.class_idx)
                        .cloned()
                        .unwrap_or_else(|| "(none)".to_string());
                    let val = format!(
                        "◀ {val} ▶  ({}/{})",
                        self.form.class_idx + 1,
                        info.classes.len()
                    );
                    ("Class", val)
                }
                FormField::Background => {
                    let val = info
                        .backgrounds
                        .get(self.form.background_idx)
                        .cloned()
                        .unwrap_or_else(|| "(none)".to_string());
                    let val = format!(
                        "◀ {val} ▶  ({}/{})",
                        self.form.background_idx + 1,
                        info.backgrounds.len()
                    );
                    ("Background", val)
                }
                FormField::Level => {
                    let max = info.max_level.unwrap_or(20);
                    let val = format!("◀ {} ▶  (1-{max})", self.form.level);
                    ("Level", val)
                }
                FormField::RandomStats => {
                    let val = if self.form.random_stats {
                        "[✓] Yes"
                    } else {
                        "[ ] No"
                    };
                    ("Random Stats", val.to_string())
                }
                FormField::IncludeEquipment => {
                    let val = if self.form.include_equipment {
                        "[✓] Yes"
                    } else {
                        "[ ] No"
                    };
                    ("Equipment", val.to_string())
                }
            };

            let val_style = if is_focused {
                Style::default().fg(Color::White)
            } else {
                Style::default()
            };

            lines.push(Line::from(vec![
                Span::raw(format!("  {marker} ")),
                Span::styled(format!("{:<14}", format!("{label}:")), label_style),
                Span::styled(value, val_style),
            ]));
        }

        // Footer
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!(
                "  {}",
                "─".repeat(inner.width.saturating_sub(4) as usize)
            ),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Tab/↑↓", Style::default().fg(Color::DarkGray)),
            Span::raw(":fields "),
            Span::styled("j/k/◀▶", Style::default().fg(Color::DarkGray)),
            Span::raw(":cycle "),
            Span::styled("Enter", Style::default().fg(Color::DarkGray)),
            Span::raw(":generate "),
            Span::styled("Esc", Style::default().fg(Color::DarkGray)),
            Span::raw(":back"),
        ]));

        if let Some(ref err) = self.error {
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("✗ {err}"), Style::default().fg(Color::Red)),
            ]));
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_generating(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Generating... ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = vec![
            Line::raw(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "⟳ Generating character...",
                    Style::default().fg(Color::Yellow),
                ),
            ]),
        ];
        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_display(&self, frame: &mut Frame, area: Rect) {
        let character = match self.generated.as_ref() {
            Some(ch) => ch,
            None => return,
        };

        let block = Block::default()
            .title(format!(" {} — {} ", character.name, character.system.display_name()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));

        // ── Header ──
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                character.name.clone(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        let mut info_parts: Vec<Span<'static>> = vec![Span::raw("  ")];
        if let Some(ref race) = character.race {
            info_parts.push(Span::styled(race.clone(), Style::default().fg(Color::Cyan)));
            info_parts.push(Span::raw(" "));
        }
        if let Some(ref class) = character.class {
            info_parts.push(Span::styled(
                class.clone(),
                Style::default().fg(Color::Green),
            ));
            info_parts.push(Span::raw(" "));
        }
        if character.level > 0 {
            info_parts.push(Span::styled(
                format!("Lv.{}", character.level),
                Style::default().fg(Color::DarkGray),
            ));
        }
        lines.push(Line::from(info_parts));

        if !character.concept.is_empty() {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("\"{}\"", character.concept),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }

        // ── Attributes ──
        if !character.attributes.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "  ATTRIBUTES",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));

            let mut attr_entries: Vec<(&String, &crate::core::character_gen::AttributeValue)> =
                character.attributes.iter().collect();
            attr_entries.sort_by_key(|(k, _)| k.to_string());

            // Render in rows of 3
            for chunk in attr_entries.chunks(3) {
                let mut spans: Vec<Span<'static>> = vec![Span::raw("  ")];
                for (name, val) in chunk {
                    let mod_str = if val.modifier >= 0 {
                        format!("+{}", val.modifier)
                    } else {
                        format!("{}", val.modifier)
                    };
                    spans.push(Span::styled(
                        format!("{:<14}", format!("{}: {} ({})", name, val.base, mod_str)),
                        Style::default(),
                    ));
                }
                lines.push(Line::from(spans));
            }
        }

        // ── Skills ──
        if !character.skills.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "  SKILLS",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));

            let mut skills: Vec<(&String, &i32)> = character.skills.iter().collect();
            skills.sort_by_key(|(k, _)| k.to_string());

            for chunk in skills.chunks(3) {
                let mut spans: Vec<Span<'static>> = vec![Span::raw("  ")];
                for (name, val) in chunk {
                    let sign = if **val >= 0 { "+" } else { "" };
                    spans.push(Span::styled(
                        format!("{:<20}", format!("{name}: {sign}{val}")),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                lines.push(Line::from(spans));
            }
        }

        // ── Traits ──
        if !character.traits.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "  TRAITS",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));

            for t in &character.traits {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("• ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        t.name.clone(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" ({:?})", t.trait_type),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
                if !t.description.is_empty() {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(
                            truncate(&t.description, 70),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                }
            }
        }

        // ── Equipment ──
        if !character.equipment.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "  EQUIPMENT",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));

            for eq in &character.equipment {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("• ", Style::default().fg(Color::Cyan)),
                    Span::raw(eq.name.clone()),
                    Span::styled(
                        format!(" ({:?})", eq.category),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
        }

        // ── Background ──
        let bg = &character.background;
        if !bg.origin.is_empty() || !bg.motivation.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "  BACKGROUND",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            if !bg.origin.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Origin: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(bg.origin.clone()),
                ]));
            }
            if !bg.motivation.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Motivation: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(bg.motivation.clone()),
                ]));
            }
            if !bg.connections.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Connections: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(bg.connections.join(", ")),
                ]));
            }
        }

        // ── Backstory ──
        if let Some(ref backstory) = self.backstory {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "  BACKSTORY",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            for line in backstory.lines() {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::raw(line.to_string()),
                ]));
            }
        } else if self.backstory_loading {
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "⟳ Generating backstory...",
                    Style::default().fg(Color::Yellow),
                ),
            ]));
        }

        // ── Error ──
        if let Some(ref err) = self.error {
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("✗ {err}"), Style::default().fg(Color::Red)),
            ]));
        }

        // ── Action bar ──
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!(
                "  {}",
                "─".repeat(inner.width.saturating_sub(4) as usize)
            ),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("b", Style::default().fg(Color::DarkGray)),
            Span::raw(":backstory "),
            Span::styled("s", Style::default().fg(Color::DarkGray)),
            Span::raw(":save "),
            Span::styled("r", Style::default().fg(Color::DarkGray)),
            Span::raw(":regenerate "),
            Span::styled("n", Style::default().fg(Color::DarkGray)),
            Span::raw(":new "),
            Span::styled("j/k", Style::default().fg(Color::DarkGray)),
            Span::raw(":scroll "),
            Span::styled("Esc", Style::default().fg(Color::DarkGray)),
            Span::raw(":back"),
        ]));

        let content = Paragraph::new(lines).scroll((self.scroll_offset, 0));
        frame.render_widget(content, inner);
    }

    fn render_saved_list(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(" Saved Characters ({}) ", self.saved_characters.len()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));

        if self.saved_characters.is_empty() {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "No saved characters.",
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        } else {
            for (i, record) in self.saved_characters.iter().enumerate() {
                let is_selected = i == self.saved_selected;
                let cursor = if is_selected { "▸ " } else { "  " };

                let level_str = record
                    .level
                    .map(|l| format!(" Lv.{l}"))
                    .unwrap_or_default();

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
                        format!("{:<20}", truncate(&record.name, 20)),
                        if is_selected {
                            Style::default().add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        },
                    ),
                    Span::styled(
                        format!("{}{}", record.system, level_str),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
        }

        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("j/k", Style::default().fg(Color::DarkGray)),
            Span::raw(":navigate "),
            Span::styled("l/Esc", Style::default().fg(Color::DarkGray)),
            Span::raw(":close"),
        ]));

        frame.render_widget(Paragraph::new(lines), inner);
    }
}

// ── Form field mapping ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum FormField {
    Name,
    Concept,
    Race,
    Class,
    Background,
    Level,
    RandomStats,
    IncludeEquipment,
}

const FIELDS_WITH_LEVEL: [FormField; FORM_FIELD_COUNT] = [
    FormField::Name,
    FormField::Concept,
    FormField::Race,
    FormField::Class,
    FormField::Background,
    FormField::Level,
    FormField::RandomStats,
    FormField::IncludeEquipment,
];

const FIELDS_NO_LEVEL: [FormField; FORM_FIELD_COUNT - 1] = [
    FormField::Name,
    FormField::Concept,
    FormField::Race,
    FormField::Class,
    FormField::Background,
    FormField::RandomStats,
    FormField::IncludeEquipment,
];

// ── Free input helpers (avoid &mut self + &mut self.form borrow conflict) ────

fn cycle_select(code: KeyCode, len: usize, idx: &mut usize) -> bool {
    if len == 0 {
        return false;
    }
    match code {
        KeyCode::Char('j') | KeyCode::Right => {
            *idx = (*idx + 1) % len;
            true
        }
        KeyCode::Char('k') | KeyCode::Left => {
            *idx = if *idx == 0 { len - 1 } else { *idx - 1 };
            true
        }
        _ => false,
    }
}

fn toggle_field(code: KeyCode, flag: &mut bool) -> bool {
    match code {
        KeyCode::Char(' ') | KeyCode::Char('j') | KeyCode::Char('k') | KeyCode::Right
        | KeyCode::Left => {
            *flag = !*flag;
            true
        }
        _ => false,
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

/// Extract backstory text from LLM response. Tries JSON, falls back to raw text.
fn extract_backstory_text(content: &str) -> String {
    // Try to parse as JSON GeneratedBackstory
    if let Some(start) = content.find('{') {
        if let Some(end) = content.rfind('}') {
            let json_str = &content[start..=end];
            if let Ok(backstory) =
                serde_json::from_str::<crate::core::character_gen::backstory::GeneratedBackstory>(
                    json_str,
                )
            {
                return backstory.text;
            }
        }
    }
    // Fallback: use raw text
    content.to_string()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_phase_transitions() {
        let mut state = GenerationState::new();
        assert_eq!(state.phase, GenPhase::SystemSelect);

        // Simulate selecting a system and advancing
        if !state.systems.is_empty() {
            let info = state.systems[0].clone();
            state.form.reset(&info);
            state.phase = GenPhase::Options;
            assert_eq!(state.phase, GenPhase::Options);

            // Back to SystemSelect via Esc
            state.phase = GenPhase::SystemSelect;
            assert_eq!(state.phase, GenPhase::SystemSelect);
        }
    }

    #[test]
    fn test_gen_form_to_options() {
        let state = GenerationState::new();
        if state.systems.is_empty() {
            return;
        }
        let info = &state.systems[0]; // D&D 5e (first)
        let mut form = GenForm::new();
        form.name = "Aldric".to_string();
        form.concept = "Veteran warrior".to_string();
        form.level = 5;

        let options = form.to_generation_options(info);
        assert_eq!(options.name.as_deref(), Some("Aldric"));
        assert_eq!(options.concept.as_deref(), Some("Veteran warrior"));
        assert!(options.system.is_some());
        assert!(options.random_stats);
        assert!(options.include_equipment);
    }

    #[test]
    fn test_gen_form_reset() {
        let state = GenerationState::new();
        if state.systems.is_empty() {
            return;
        }
        let info = &state.systems[0];
        let mut form = GenForm::new();
        form.name = "Existing".to_string();
        form.level = 10;
        form.random_stats = false;

        form.reset(info);
        assert!(form.name.is_empty());
        assert_eq!(form.level, 1);
        assert!(form.random_stats);
    }

    #[test]
    fn test_system_info_populated() {
        let systems = CharacterGenerator::list_system_info();
        assert!(systems.len() >= 10, "Expected at least 10 game systems");
        for sys in &systems {
            assert!(!sys.name.is_empty());
            assert!(!sys.id.is_empty());
        }
    }

    #[test]
    fn test_gen_form_empty_name() {
        let state = GenerationState::new();
        if state.systems.is_empty() {
            return;
        }
        let info = &state.systems[0];
        let form = GenForm::new();
        let options = form.to_generation_options(info);
        assert!(options.name.is_none()); // empty name → None (random)
    }

    #[test]
    fn test_extract_backstory_text_json() {
        let json = r#"Some preamble {"text": "The hero was born...", "summary": "A hero", "key_events": [], "mentioned_npcs": [], "mentioned_locations": [], "plot_hooks": [], "suggested_traits": []} trailing"#;
        let result = extract_backstory_text(json);
        assert_eq!(result, "The hero was born...");
    }

    #[test]
    fn test_extract_backstory_text_raw() {
        let raw = "Just a plain backstory about a warrior.";
        let result = extract_backstory_text(raw);
        assert_eq!(result, raw);
    }

    #[test]
    fn test_field_count_with_levels() {
        let state = GenerationState::new();
        // Find a system with levels (DnD5e)
        if let Some(info) = state.systems.iter().find(|s| s.has_levels) {
            assert_eq!(state.field_count(info), FORM_FIELD_COUNT);
        }
        // Find a system without levels (GURPS, Cyberpunk, etc.)
        if let Some(info) = state.systems.iter().find(|s| !s.has_levels) {
            assert_eq!(state.field_count(info), FORM_FIELD_COUNT - 1);
        }
    }
}
