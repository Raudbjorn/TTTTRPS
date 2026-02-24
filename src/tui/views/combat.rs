//! Combat Tracker view.
//!
//! Phases: NoCombat → InitiativeEntry → Active → Ended.
//! Uses backend `CombatState`, `Combatant`, `ConditionTemplates`.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::core::campaign::dice::{DiceNotation, DiceRoller};
use crate::core::session::combat::{Combatant, CombatantType, CombatState};
use crate::core::session::conditions::ConditionTemplates;
use crate::tui::theme;
use crate::tui::widgets::input_buffer::InputBuffer;

// ============================================================================
// Phase State Machine
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CombatPhase {
    NoCombat,
    InitiativeEntry,
    Active,
    Ended,
}

/// Which field the initiative-entry form is editing.
#[derive(Debug, Clone, Copy, PartialEq)]
enum EntryField {
    Name,
    Initiative,
    HitPoints,
    Type,
}

impl EntryField {
    fn next(self) -> Self {
        match self {
            Self::Name => Self::Initiative,
            Self::Initiative => Self::HitPoints,
            Self::HitPoints => Self::Type,
            Self::Type => Self::Name,
        }
    }
    fn prev(self) -> Self {
        match self {
            Self::Name => Self::Type,
            Self::Initiative => Self::Name,
            Self::HitPoints => Self::Initiative,
            Self::Type => Self::HitPoints,
        }
    }
}

/// Active-phase sub-mode for numeric inputs (damage/heal).
#[derive(Debug, Clone, Copy, PartialEq)]
enum ActiveInput {
    None,
    Damage,
    Heal,
    Condition,
}

// ============================================================================
// Combat View State
// ============================================================================

pub struct CombatViewState {
    phase: CombatPhase,
    combat: CombatState,
    roller: DiceRoller,

    // InitiativeEntry
    entry_name: InputBuffer,
    entry_init: InputBuffer,
    entry_hp: InputBuffer,
    entry_type: CombatantType,
    entry_field: EntryField,
    entry_error: Option<String>,

    // Active phase
    selected_idx: usize,
    log_scroll: usize,
    active_input: ActiveInput,
    input_buf: InputBuffer,
    condition_cursor: usize,
}

impl CombatViewState {
    pub fn new() -> Self {
        Self {
            phase: CombatPhase::NoCombat,
            combat: CombatState::new(),
            roller: DiceRoller::new(),
            entry_name: InputBuffer::new(),
            entry_init: InputBuffer::new(),
            entry_hp: InputBuffer::new(),
            entry_type: CombatantType::Player,
            entry_field: EntryField::Name,
            entry_error: None,
            selected_idx: 0,
            log_scroll: 0,
            active_input: ActiveInput::None,
            input_buf: InputBuffer::new(),
            condition_cursor: 0,
        }
    }

    // ────────────────────────────────────────────────────────────────────
    // Input handling
    // ────────────────────────────────────────────────────────────────────

    pub fn handle_input(&mut self, event: &Event) -> bool {
        match self.phase {
            CombatPhase::NoCombat => self.handle_no_combat(event),
            CombatPhase::InitiativeEntry => self.handle_initiative_entry(event),
            CombatPhase::Active => self.handle_active(event),
            CombatPhase::Ended => self.handle_ended(event),
        }
    }

    fn handle_no_combat(&mut self, event: &Event) -> bool {
        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            ..
        }) = event
        {
            match code {
                KeyCode::Enter | KeyCode::Char('n') => {
                    self.combat = CombatState::new();
                    self.phase = CombatPhase::InitiativeEntry;
                    self.reset_entry_form();
                    true
                }
                _ => false,
            }
        } else {
            false
        }
    }

    fn handle_initiative_entry(&mut self, event: &Event) -> bool {
        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            modifiers,
            ..
        }) = event
        {
            // Ctrl+S or F5 to start combat
            if (*modifiers == KeyModifiers::CONTROL && *code == KeyCode::Char('s'))
                || *code == KeyCode::F(5)
            {
                if self.combat.combatants.len() >= 2 {
                    self.combat.sort_initiative();
                    self.phase = CombatPhase::Active;
                    self.selected_idx = 0;
                } else {
                    self.entry_error = Some("Need at least 2 combatants".into());
                }
                return true;
            }

            match code {
                KeyCode::Esc => {
                    if self.combat.combatants.is_empty() {
                        self.phase = CombatPhase::NoCombat;
                    } else {
                        // Remove last added combatant
                        self.combat.combatants.pop();
                    }
                    true
                }
                KeyCode::Tab => {
                    self.entry_field = self.entry_field.next();
                    true
                }
                KeyCode::BackTab => {
                    self.entry_field = self.entry_field.prev();
                    true
                }
                KeyCode::Enter => {
                    self.submit_combatant();
                    true
                }
                KeyCode::Char('r') if *modifiers == KeyModifiers::CONTROL => {
                    // Roll initiative: d20 + modifier
                    let result = self.roller.roll(
                        &DiceNotation::parse("d20").unwrap_or_else(|_| {
                            DiceNotation::new(1, crate::core::campaign::dice::DiceType::D20, 0)
                                .unwrap()
                        }),
                    );
                    self.entry_init.clear();
                    for c in result.total.to_string().chars() {
                        self.entry_init.insert_char(c);
                    }
                    true
                }
                _ => {
                    // Route to active field
                    match self.entry_field {
                        EntryField::Name => self.handle_input_buffer(&mut_ref_name(), event),
                        EntryField::Initiative => {
                            self.handle_input_buffer(&mut_ref_init(), event)
                        }
                        EntryField::HitPoints => self.handle_input_buffer(&mut_ref_hp(), event),
                        EntryField::Type => {
                            if let KeyCode::Char(c) = code {
                                self.entry_type = match c {
                                    'p' | 'P' => CombatantType::Player,
                                    'm' | 'M' => CombatantType::Monster,
                                    'n' | 'N' => CombatantType::NPC,
                                    'a' | 'A' => CombatantType::Ally,
                                    'e' | 'E' => CombatantType::Environment,
                                    _ => self.entry_type.clone(),
                                };
                            }
                            true
                        }
                    }
                }
            }
        } else {
            false
        }
    }

    fn handle_active(&mut self, event: &Event) -> bool {
        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            modifiers,
            ..
        }) = event
        {
            // Sub-mode: numeric input
            if self.active_input != ActiveInput::None {
                return self.handle_active_input(code, modifiers);
            }

            match code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if !self.combat.combatants.is_empty() {
                        self.selected_idx =
                            (self.selected_idx + 1) % self.combat.combatants.len();
                    }
                    true
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if !self.combat.combatants.is_empty() {
                        self.selected_idx = if self.selected_idx == 0 {
                            self.combat.combatants.len() - 1
                        } else {
                            self.selected_idx - 1
                        };
                    }
                    true
                }
                KeyCode::Char(' ') => {
                    // Next turn
                    self.combat.next_turn();
                    self.selected_idx = self.combat.current_turn;
                    true
                }
                KeyCode::Char('D') => {
                    self.active_input = ActiveInput::Damage;
                    self.input_buf.clear();
                    true
                }
                KeyCode::Char('h') if *modifiers == KeyModifiers::NONE => {
                    self.active_input = ActiveInput::Heal;
                    self.input_buf.clear();
                    true
                }
                KeyCode::Char('c') => {
                    self.active_input = ActiveInput::Condition;
                    self.condition_cursor = 0;
                    true
                }
                KeyCode::Char('d') => {
                    // Remove selected combatant
                    if let Some(c) = self.combat.combatants.get(self.selected_idx) {
                        let id = c.id.clone();
                        self.combat.remove_combatant(&id);
                        if self.selected_idx >= self.combat.combatants.len()
                            && !self.combat.combatants.is_empty()
                        {
                            self.selected_idx = self.combat.combatants.len() - 1;
                        }
                    }
                    true
                }
                KeyCode::Char('n') => {
                    // Add combatant mid-combat
                    self.phase = CombatPhase::InitiativeEntry;
                    self.reset_entry_form();
                    true
                }
                KeyCode::Char('e') => {
                    // End combat
                    self.combat.end();
                    self.phase = CombatPhase::Ended;
                    true
                }
                KeyCode::Char('[') => {
                    // Scroll log up
                    self.log_scroll = self.log_scroll.saturating_add(1);
                    true
                }
                KeyCode::Char(']') => {
                    // Scroll log down
                    self.log_scroll = self.log_scroll.saturating_sub(1);
                    true
                }
                _ => false,
            }
        } else {
            false
        }
    }

    fn handle_active_input(&mut self, code: &KeyCode, _modifiers: &KeyModifiers) -> bool {
        match self.active_input {
            ActiveInput::Condition => match code {
                KeyCode::Esc => {
                    self.active_input = ActiveInput::None;
                    true
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    let names = ConditionTemplates::list_names();
                    if self.condition_cursor == 0 {
                        self.condition_cursor = names.len() - 1;
                    } else {
                        self.condition_cursor -= 1;
                    }
                    true
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let names = ConditionTemplates::list_names();
                    self.condition_cursor = (self.condition_cursor + 1) % names.len();
                    true
                }
                KeyCode::Enter => {
                    let names = ConditionTemplates::list_names();
                    if let Some(name) = names.get(self.condition_cursor) {
                        if let Some(condition) = ConditionTemplates::by_name(name) {
                            if let Some(combatant) =
                                self.combat.combatants.get_mut(self.selected_idx)
                            {
                                let _ = combatant.condition_tracker.add_condition(condition);
                            }
                        }
                    }
                    self.active_input = ActiveInput::None;
                    true
                }
                _ => true,
            },
            _ => {
                // Damage / Heal numeric input
                match code {
                    KeyCode::Esc => {
                        self.active_input = ActiveInput::None;
                        true
                    }
                    KeyCode::Enter => {
                        let text = self.input_buf.text().trim().to_string();
                        if let Ok(amount) = text.parse::<i32>() {
                            if let Some(combatant) =
                                self.combat.combatants.get_mut(self.selected_idx)
                            {
                                match self.active_input {
                                    ActiveInput::Damage => {
                                        combatant.apply_damage(amount);
                                    }
                                    ActiveInput::Heal => {
                                        combatant.heal(amount);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        self.active_input = ActiveInput::None;
                        true
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() || *c == '-' => {
                        self.input_buf.insert_char(*c);
                        true
                    }
                    KeyCode::Backspace => {
                        self.input_buf.backspace();
                        true
                    }
                    _ => true,
                }
            }
        }
    }

    fn handle_ended(&mut self, event: &Event) -> bool {
        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            ..
        }) = event
        {
            match code {
                KeyCode::Enter | KeyCode::Char('n') => {
                    self.phase = CombatPhase::NoCombat;
                    true
                }
                _ => false,
            }
        } else {
            false
        }
    }

    // ────────────────────────────────────────────────────────────────────
    // Helpers
    // ────────────────────────────────────────────────────────────────────

    /// Route key events to an InputBuffer by field.
    fn handle_input_buffer(&mut self, field: &EntryFieldRef, event: &Event) -> bool {
        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            ..
        }) = event
        {
            let buf = match field {
                EntryFieldRef::Name => &mut self.entry_name,
                EntryFieldRef::Init => &mut self.entry_init,
                EntryFieldRef::Hp => &mut self.entry_hp,
            };
            match code {
                KeyCode::Char(c) => {
                    buf.insert_char(*c);
                    self.entry_error = None;
                }
                KeyCode::Backspace => buf.backspace(),
                KeyCode::Delete => buf.delete(),
                KeyCode::Left => buf.move_left(),
                KeyCode::Right => buf.move_right(),
                KeyCode::Home => buf.move_home(),
                KeyCode::End => buf.move_end(),
                _ => {}
            }
            true
        } else {
            false
        }
    }

    fn submit_combatant(&mut self) {
        let name = self.entry_name.text().trim().to_string();
        if name.is_empty() {
            self.entry_error = Some("Name required".into());
            return;
        }

        let init: i32 = self
            .entry_init
            .text()
            .trim()
            .parse()
            .unwrap_or(0);

        let hp: Option<i32> = {
            let t = self.entry_hp.text().trim().to_string();
            if t.is_empty() {
                None
            } else {
                t.parse().ok()
            }
        };

        let mut combatant = Combatant::new(name, init, self.entry_type.clone());
        combatant.current_hp = hp;
        combatant.max_hp = hp;
        self.combat.add_combatant(combatant);
        self.reset_entry_form();
    }

    fn reset_entry_form(&mut self) {
        self.entry_name.clear();
        self.entry_init.clear();
        self.entry_hp.clear();
        self.entry_type = CombatantType::Player;
        self.entry_field = EntryField::Name;
        self.entry_error = None;
    }

    // ────────────────────────────────────────────────────────────────────
    // Rendering
    // ────────────────────────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        match self.phase {
            CombatPhase::NoCombat => self.render_no_combat(frame, area),
            CombatPhase::InitiativeEntry => self.render_initiative_entry(frame, area),
            CombatPhase::Active => self.render_active(frame, area),
            CombatPhase::Ended => self.render_ended(frame, area),
        }
    }

    fn render_no_combat(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_default("Combat Tracker");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = vec![
            Line::raw(""),
            Line::from(Span::styled(
                "⚔ No Active Combat",
                Style::default()
                    .fg(theme::TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(vec![
                Span::raw("Press "),
                Span::styled("n", theme::key_hint()),
                Span::raw(" or "),
                Span::styled("Enter", theme::key_hint()),
                Span::raw(" to start a new encounter"),
            ]),
        ];

        frame.render_widget(
            Paragraph::new(lines).alignment(Alignment::Center),
            inner,
        );
    }

    fn render_initiative_entry(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_focused("Add Combatants");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::vertical([
            Constraint::Length(3), // Name
            Constraint::Length(3), // Initiative
            Constraint::Length(3), // HP
            Constraint::Length(2), // Type selector
            Constraint::Length(1), // Error
            Constraint::Length(2), // Hint
            Constraint::Min(3),   // Current roster
        ])
        .split(inner);

        // Name field
        self.render_entry_field(frame, chunks[0], "Name", &self.entry_name, self.entry_field == EntryField::Name);
        // Initiative field
        self.render_entry_field(frame, chunks[1], "Initiative", &self.entry_init, self.entry_field == EntryField::Initiative);
        // HP field
        self.render_entry_field(frame, chunks[2], "HP (optional)", &self.entry_hp, self.entry_field == EntryField::HitPoints);

        // Type selector
        let types = [
            ("P", "Player", CombatantType::Player),
            ("M", "Monster", CombatantType::Monster),
            ("N", "NPC", CombatantType::NPC),
            ("A", "Ally", CombatantType::Ally),
            ("E", "Env", CombatantType::Environment),
        ];
        let type_spans: Vec<Span> = types
            .iter()
            .flat_map(|(key, label, ct)| {
                let is_sel = self.entry_type == *ct;
                let is_focused = self.entry_field == EntryField::Type;
                let style = if is_sel {
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD)
                } else if is_focused {
                    Style::default().fg(theme::TEXT)
                } else {
                    Style::default().fg(theme::TEXT_DIM)
                };
                vec![
                    Span::styled(format!("[{key}]"), theme::key_hint()),
                    Span::styled(format!("{label} "), style),
                ]
            })
            .collect();
        frame.render_widget(
            Paragraph::new(Line::from(type_spans)),
            chunks[3],
        );

        // Error
        if let Some(ref err) = self.entry_error {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    format!(" {err}"),
                    Style::default().fg(theme::ERROR),
                )),
                chunks[4],
            );
        }

        // Hints
        let hint = Line::from(vec![
            Span::styled("Enter", theme::key_hint()),
            Span::styled(":add ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled("Tab", theme::key_hint()),
            Span::styled(":field ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled("Ctrl+R", theme::key_hint()),
            Span::styled(":roll init ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled("Ctrl+S/F5", theme::key_hint()),
            Span::styled(":start", Style::default().fg(theme::TEXT_DIM)),
        ]);
        frame.render_widget(Paragraph::new(hint), chunks[5]);

        // Current roster
        self.render_roster(frame, chunks[6]);
    }

    fn render_entry_field(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        buf: &InputBuffer,
        focused: bool,
    ) {
        let border_style = if focused {
            Style::default().fg(theme::PRIMARY_LIGHT)
        } else {
            Style::default().fg(theme::TEXT_DIM)
        };
        let block = Block::default()
            .title(format!(" {label} "))
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let text = buf.text();
        let style = if text.is_empty() {
            Style::default().fg(theme::TEXT_DIM)
        } else {
            Style::default().fg(theme::TEXT)
        };
        let display = if text.is_empty() { label } else { text };
        frame.render_widget(
            Paragraph::new(Span::styled(display.to_string(), style)),
            inner,
        );

        if focused {
            frame.set_cursor_position((
                inner.x + buf.cursor_position() as u16,
                inner.y,
            ));
        }
    }

    fn render_roster(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(" Roster ({}) ", self.combat.combatants.len()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_DIM));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.combat.combatants.is_empty() {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    " No combatants yet",
                    Style::default().fg(theme::TEXT_DIM),
                )),
                inner,
            );
            return;
        }

        let lines: Vec<Line> = self
            .combat
            .combatants
            .iter()
            .map(|c| {
                let type_icon = combatant_icon(&c.combatant_type);
                let hp_str = c
                    .current_hp
                    .map(|hp| format!(" [{hp}HP]"))
                    .unwrap_or_default();
                Line::from(vec![
                    Span::styled(
                        format!(" {type_icon} "),
                        Style::default().fg(type_color(&c.combatant_type)),
                    ),
                    Span::styled(
                        format!("{:>3} ", c.initiative),
                        Style::default().fg(theme::PRIMARY_LIGHT),
                    ),
                    Span::styled(&c.name, Style::default().fg(theme::TEXT)),
                    Span::styled(hp_str, Style::default().fg(theme::TEXT_MUTED)),
                ])
            })
            .collect();

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_active(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_focused("Combat Tracker");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Horizontal split: left 45% initiative list, right 55% detail+log
        let h_chunks = Layout::horizontal([
            Constraint::Percentage(45),
            Constraint::Percentage(55),
        ])
        .split(inner);

        self.render_initiative_list(frame, h_chunks[0]);

        // Right side: detail (top) + log (bottom)
        let v_chunks = Layout::vertical([
            Constraint::Length(2), // Round/turn header
            Constraint::Min(5),   // Detail
            Constraint::Length(self.log_height(h_chunks[1].height)), // Log
        ])
        .split(h_chunks[1]);

        self.render_round_header(frame, v_chunks[0]);
        self.render_combatant_detail(frame, v_chunks[1]);
        self.render_combat_log(frame, v_chunks[2]);

        // Overlay: condition picker
        if self.active_input == ActiveInput::Condition {
            self.render_condition_picker(frame, inner);
        }

        // Overlay: damage/heal input
        if self.active_input == ActiveInput::Damage || self.active_input == ActiveInput::Heal {
            self.render_numeric_input(frame, inner);
        }
    }

    fn log_height(&self, available: u16) -> u16 {
        let log_entries = self.combat.events.len().min(10) as u16;
        (log_entries + 2).min(available / 3).max(4)
    }

    fn render_round_header(&self, frame: &mut Frame, area: Rect) {
        let current_name = self
            .combat
            .current_combatant()
            .map(|c| c.name.as_str())
            .unwrap_or("—");

        let line = Line::from(vec![
            Span::styled(
                format!(" Round {} ", self.combat.round),
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("│ ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled("Turn: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                current_name.to_string(),
                Style::default()
                    .fg(theme::PRIMARY_LIGHT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        let hint = Line::from(vec![
            Span::styled("Space", theme::key_hint()),
            Span::styled(":next ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled("D", theme::key_hint()),
            Span::styled(":dmg ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled("h", theme::key_hint()),
            Span::styled(":heal ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled("c", theme::key_hint()),
            Span::styled(":cond ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled("e", theme::key_hint()),
            Span::styled(":end", Style::default().fg(theme::TEXT_DIM)),
        ]);

        frame.render_widget(Paragraph::new(vec![line, hint]), area);
    }

    fn render_initiative_list(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Initiative ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::PRIMARY));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines: Vec<Line> = self
            .combat
            .combatants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let is_current = i == self.combat.current_turn;
                let is_selected = i == self.selected_idx;
                let type_icon = combatant_icon(&c.combatant_type);

                let prefix = if is_current && is_selected {
                    "▸▶"
                } else if is_current {
                    " ▶"
                } else if is_selected {
                    "▸ "
                } else {
                    "  "
                };

                let name_style = if !c.is_active {
                    Style::default()
                        .fg(theme::TEXT_DIM)
                        .add_modifier(Modifier::CROSSED_OUT)
                } else if is_current {
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD)
                } else if is_selected {
                    Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT)
                };

                let hp_span = hp_display(c);
                let conditions = condition_icons(c);

                let mut spans = vec![
                    Span::styled(prefix.to_string(), Style::default().fg(theme::ACCENT)),
                    Span::styled(
                        format!("{type_icon} "),
                        Style::default().fg(type_color(&c.combatant_type)),
                    ),
                    Span::styled(format!("{:>3} ", c.initiative), Style::default().fg(theme::PRIMARY_LIGHT)),
                    Span::styled(
                        truncate_name(&c.name, (area.width as usize).saturating_sub(16)),
                        name_style,
                    ),
                ];
                spans.push(hp_span);
                if !conditions.is_empty() {
                    spans.push(Span::styled(
                        format!(" {conditions}"),
                        Style::default().fg(theme::WARNING),
                    ));
                }

                Line::from(spans)
            })
            .collect();

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_combatant_detail(&self, frame: &mut Frame, area: Rect) {
        let combatant = self.combat.combatants.get(self.selected_idx);
        let block = Block::default()
            .title(" Detail ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_DIM));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(c) = combatant else {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    " No combatant selected",
                    Style::default().fg(theme::TEXT_DIM),
                )),
                inner,
            );
            return;
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    format!("{} ", combatant_icon(&c.combatant_type)),
                    Style::default().fg(type_color(&c.combatant_type)),
                ),
                Span::styled(
                    &c.name,
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  ({})", type_label(&c.combatant_type)),
                    Style::default().fg(theme::TEXT_MUTED),
                ),
            ]),
        ];

        // HP bar
        if let (Some(current), Some(max)) = (c.current_hp, c.max_hp) {
            let pct = if max > 0 {
                (current as f64 / max as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let bar_width = (inner.width as usize).saturating_sub(14).min(30);
            let filled = (pct * bar_width as f64) as usize;
            let empty = bar_width - filled;
            let color = if pct > 0.5 {
                theme::SUCCESS
            } else if pct > 0.25 {
                theme::WARNING
            } else {
                theme::ERROR
            };

            let temp_str = c.temp_hp.filter(|&t| t > 0).map(|t| format!(" +{t}tmp")).unwrap_or_default();

            lines.push(Line::from(vec![
                Span::styled(" HP: ", Style::default().fg(theme::TEXT_MUTED)),
                Span::styled(
                    format!("{current}/{max}"),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(temp_str, Style::default().fg(theme::INFO)),
                Span::raw(" "),
                Span::styled("█".repeat(filled), Style::default().fg(color)),
                Span::styled("░".repeat(empty), Style::default().fg(theme::TEXT_DIM)),
            ]));
        }

        // AC
        if let Some(ac) = c.armor_class {
            lines.push(Line::from(vec![
                Span::styled(" AC: ", Style::default().fg(theme::TEXT_MUTED)),
                Span::styled(ac.to_string(), Style::default().fg(theme::INFO)),
            ]));
        }

        // Init
        lines.push(Line::from(vec![
            Span::styled(" Init: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                c.initiative.to_string(),
                Style::default().fg(theme::PRIMARY_LIGHT),
            ),
        ]));

        // Conditions
        let conditions = c.condition_tracker.conditions();
        if !conditions.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                " Conditions:",
                Style::default()
                    .fg(theme::WARNING)
                    .add_modifier(Modifier::BOLD),
            )));
            for cond in conditions {
                let remaining = cond.remaining_text();
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  • {}", cond.name),
                        Style::default().fg(theme::TEXT),
                    ),
                    Span::styled(
                        format!(" ({remaining})"),
                        Style::default().fg(theme::TEXT_MUTED),
                    ),
                ]));
            }
        }

        // Notes
        if !c.notes.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::styled(" Notes: ", Style::default().fg(theme::TEXT_MUTED)),
                Span::styled(&c.notes, Style::default().fg(theme::TEXT)),
            ]));
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_combat_log(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(format!(" Log ({}) ", self.combat.events.len()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_DIM));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.combat.events.is_empty() {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    " No events yet",
                    Style::default().fg(theme::TEXT_DIM),
                )),
                inner,
            );
            return;
        }

        let lines: Vec<Line> = self
            .combat
            .events
            .iter()
            .rev()
            .map(|e| {
                Line::from(vec![
                    Span::styled(
                        format!("R{}.{} ", e.round, e.turn + 1),
                        Style::default().fg(theme::TEXT_DIM),
                    ),
                    Span::styled(&e.description, Style::default().fg(theme::TEXT_MUTED)),
                ])
            })
            .collect();

        let visible = inner.height as usize;
        let scroll = if lines.len() > visible {
            self.log_scroll.min(lines.len() - visible)
        } else {
            0
        };

        frame.render_widget(
            Paragraph::new(lines.clone()).scroll((scroll as u16, 0)),
            inner,
        );

        if lines.len() > visible {
            let mut scrollbar_state =
                ScrollbarState::new(lines.len().saturating_sub(visible)).position(scroll);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .thumb_style(Style::default().fg(theme::PRIMARY_LIGHT))
                    .track_style(Style::default().fg(theme::TEXT_DIM)),
                inner,
                &mut scrollbar_state,
            );
        }
    }

    fn render_condition_picker(&self, frame: &mut Frame, area: Rect) {
        let names = ConditionTemplates::list_names();
        let height = (names.len() as u16 + 3).min(area.height.saturating_sub(4));
        let width = 35.min(area.width.saturating_sub(4));
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let modal = Rect::new(x, y, width, height);

        frame.render_widget(Clear, modal);
        let block = Block::default()
            .title(" Apply Condition ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::WARNING))
            .style(Style::default().bg(theme::BG_BASE));

        let inner = block.inner(modal);
        frame.render_widget(block, modal);

        let lines: Vec<Line> = names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let is_sel = i == self.condition_cursor;
                let prefix = if is_sel { "▸ " } else { "  " };
                let style = if is_sel {
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT)
                };
                Line::from(Span::styled(format!("{prefix}{name}"), style))
            })
            .collect();

        let visible = inner.height as usize;
        let scroll = if self.condition_cursor >= visible {
            self.condition_cursor - visible + 1
        } else {
            0
        };

        frame.render_widget(
            Paragraph::new(lines).scroll((scroll as u16, 0)),
            inner,
        );
    }

    fn render_numeric_input(&self, frame: &mut Frame, area: Rect) {
        let label = match self.active_input {
            ActiveInput::Damage => "Damage Amount",
            ActiveInput::Heal => "Heal Amount",
            _ => return,
        };
        let color = match self.active_input {
            ActiveInput::Damage => theme::ERROR,
            ActiveInput::Heal => theme::SUCCESS,
            _ => theme::TEXT,
        };

        let width = 30.min(area.width.saturating_sub(4));
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + area.height / 2 - 2;
        let modal = Rect::new(x, y, width, 4);

        frame.render_widget(Clear, modal);
        let block = Block::default()
            .title(format!(" {label} "))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color))
            .style(Style::default().bg(theme::BG_BASE));

        let inner = block.inner(modal);
        frame.render_widget(block, modal);

        let text = self.input_buf.text();
        let display = if text.is_empty() { "0" } else { text };
        frame.render_widget(
            Paragraph::new(Span::styled(
                display.to_string(),
                Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD),
            )),
            inner,
        );

        frame.set_cursor_position((
            inner.x + self.input_buf.cursor_position() as u16,
            inner.y,
        ));
    }

    fn render_ended(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_default("Combat Ended");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let total_rounds = self.combat.round;
        let total_combatants = self.combat.combatants.len();
        let alive = self
            .combat
            .combatants
            .iter()
            .filter(|c| c.is_active && c.current_hp.map(|hp| hp > 0).unwrap_or(true))
            .count();
        let events = self.combat.events.len();

        let lines = vec![
            Line::raw(""),
            Line::from(Span::styled(
                "⚔ Combat Complete",
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(vec![
                Span::styled("Rounds: ", Style::default().fg(theme::TEXT_MUTED)),
                Span::styled(
                    total_rounds.to_string(),
                    Style::default().fg(theme::PRIMARY_LIGHT),
                ),
            ]),
            Line::from(vec![
                Span::styled("Combatants: ", Style::default().fg(theme::TEXT_MUTED)),
                Span::styled(
                    format!("{total_combatants} ({alive} standing)"),
                    Style::default().fg(theme::TEXT),
                ),
            ]),
            Line::from(vec![
                Span::styled("Events: ", Style::default().fg(theme::TEXT_MUTED)),
                Span::styled(events.to_string(), Style::default().fg(theme::TEXT)),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::raw("Press "),
                Span::styled("n", theme::key_hint()),
                Span::raw(" or "),
                Span::styled("Enter", theme::key_hint()),
                Span::raw(" to return"),
            ]),
        ];

        frame.render_widget(
            Paragraph::new(lines).alignment(Alignment::Center),
            inner,
        );
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Helper enum to indirect field references without self-borrow conflicts.
enum EntryFieldRef {
    Name,
    Init,
    Hp,
}

fn mut_ref_name() -> EntryFieldRef {
    EntryFieldRef::Name
}
fn mut_ref_init() -> EntryFieldRef {
    EntryFieldRef::Init
}
fn mut_ref_hp() -> EntryFieldRef {
    EntryFieldRef::Hp
}

fn combatant_icon(ct: &CombatantType) -> &'static str {
    match ct {
        CombatantType::Player => "🛡",
        CombatantType::Monster => "👹",
        CombatantType::NPC => "👤",
        CombatantType::Ally => "🤝",
        CombatantType::Environment => "🌍",
    }
}

fn type_color(ct: &CombatantType) -> ratatui::style::Color {
    match ct {
        CombatantType::Player => theme::INFO,
        CombatantType::Monster => theme::ERROR,
        CombatantType::NPC => theme::NPC,
        CombatantType::Ally => theme::SUCCESS,
        CombatantType::Environment => theme::TEXT_MUTED,
    }
}

fn type_label(ct: &CombatantType) -> &'static str {
    match ct {
        CombatantType::Player => "Player",
        CombatantType::Monster => "Monster",
        CombatantType::NPC => "NPC",
        CombatantType::Ally => "Ally",
        CombatantType::Environment => "Env",
    }
}

fn hp_display(c: &Combatant) -> Span<'static> {
    match (c.current_hp, c.max_hp) {
        (Some(current), Some(max)) => {
            let pct = if max > 0 {
                current as f64 / max as f64
            } else {
                0.0
            };
            let color = if pct > 0.5 {
                theme::SUCCESS
            } else if pct > 0.25 {
                theme::WARNING
            } else {
                theme::ERROR
            };
            Span::styled(format!(" {current}/{max}"), Style::default().fg(color))
        }
        _ => Span::raw(""),
    }
}

fn condition_icons(c: &Combatant) -> String {
    let conditions = c.condition_tracker.conditions();
    if conditions.is_empty() {
        return String::new();
    }
    conditions
        .iter()
        .map(|cond| {
            // Use first 2 chars of name as compact icon
            let name = &cond.name;
            if name.len() >= 2 {
                name[..2].to_uppercase()
            } else {
                name.to_uppercase()
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn truncate_name(name: &str, max: usize) -> String {
    if name.len() <= max {
        name.to_string()
    } else if max > 2 {
        format!("{}…", &name[..max - 1])
    } else {
        name[..max].to_string()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::session::combat::CombatStatus;

    #[test]
    fn test_initial_phase() {
        let state = CombatViewState::new();
        assert_eq!(state.phase, CombatPhase::NoCombat);
        assert!(state.combat.combatants.is_empty());
    }

    #[test]
    fn test_start_combat_transition() {
        let mut state = CombatViewState::new();
        let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        state.handle_input(&event);
        assert_eq!(state.phase, CombatPhase::InitiativeEntry);
    }

    #[test]
    fn test_add_combatant() {
        let mut state = CombatViewState::new();
        state.phase = CombatPhase::InitiativeEntry;

        // Type name
        for c in "Goblin".chars() {
            state.entry_name.insert_char(c);
        }
        for c in "15".chars() {
            state.entry_init.insert_char(c);
        }
        for c in "30".chars() {
            state.entry_hp.insert_char(c);
        }
        state.entry_type = CombatantType::Monster;
        state.submit_combatant();

        assert_eq!(state.combat.combatants.len(), 1);
        assert_eq!(state.combat.combatants[0].name, "Goblin");
        assert_eq!(state.combat.combatants[0].initiative, 15);
        assert_eq!(state.combat.combatants[0].current_hp, Some(30));
        assert_eq!(state.combat.combatants[0].combatant_type, CombatantType::Monster);
    }

    #[test]
    fn test_submit_empty_name_errors() {
        let mut state = CombatViewState::new();
        state.phase = CombatPhase::InitiativeEntry;
        state.submit_combatant();
        assert!(state.entry_error.is_some());
        assert!(state.combat.combatants.is_empty());
    }

    #[test]
    fn test_start_requires_two_combatants() {
        let mut state = CombatViewState::new();
        state.phase = CombatPhase::InitiativeEntry;

        // Add one combatant
        state.entry_name.insert_char('A');
        state.entry_init.insert_char('5');
        state.submit_combatant();

        // Try to start
        let event = Event::Key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        state.handle_input(&event);
        assert_eq!(state.phase, CombatPhase::InitiativeEntry);
        assert!(state.entry_error.is_some());
    }

    #[test]
    fn test_transition_to_active() {
        let mut state = CombatViewState::new();
        state.phase = CombatPhase::InitiativeEntry;

        // Add two combatants
        state.entry_name.insert_char('A');
        state.entry_init.insert_char('5');
        state.submit_combatant();

        state.entry_name.insert_char('B');
        for c in "10".chars() {
            state.entry_init.insert_char(c);
        }
        state.submit_combatant();

        let event = Event::Key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        state.handle_input(&event);
        assert_eq!(state.phase, CombatPhase::Active);
        // B has higher initiative
        assert_eq!(state.combat.combatants[0].name, "B");
    }

    #[test]
    fn test_next_turn() {
        let mut state = setup_active_combat();
        assert_eq!(state.combat.current_turn, 0);

        let space = Event::Key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        state.handle_input(&space);
        assert_eq!(state.combat.current_turn, 1);
    }

    #[test]
    fn test_end_combat() {
        let mut state = setup_active_combat();
        let event = Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        state.handle_input(&event);
        assert_eq!(state.phase, CombatPhase::Ended);
        assert_eq!(state.combat.status, CombatStatus::Ended);
    }

    #[test]
    fn test_ended_returns_to_no_combat() {
        let mut state = CombatViewState::new();
        state.phase = CombatPhase::Ended;
        let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        state.handle_input(&event);
        assert_eq!(state.phase, CombatPhase::NoCombat);
    }

    #[test]
    fn test_selection_wraps() {
        let mut state = setup_active_combat();
        let count = state.combat.combatants.len();
        assert_eq!(state.selected_idx, 0);

        // Go down past end
        for _ in 0..count {
            let event = Event::Key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
            state.handle_input(&event);
        }
        assert_eq!(state.selected_idx, 0);

        // Go up from 0
        let event = Event::Key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        state.handle_input(&event);
        assert_eq!(state.selected_idx, count - 1);
    }

    #[test]
    fn test_entry_field_cycling() {
        assert_eq!(EntryField::Name.next(), EntryField::Initiative);
        assert_eq!(EntryField::Initiative.next(), EntryField::HitPoints);
        assert_eq!(EntryField::HitPoints.next(), EntryField::Type);
        assert_eq!(EntryField::Type.next(), EntryField::Name);

        assert_eq!(EntryField::Name.prev(), EntryField::Type);
        assert_eq!(EntryField::Type.prev(), EntryField::HitPoints);
    }

    #[test]
    fn test_type_helpers() {
        assert_eq!(combatant_icon(&CombatantType::Player), "🛡");
        assert_eq!(type_label(&CombatantType::Monster), "Monster");
    }

    #[test]
    fn test_truncate_name() {
        assert_eq!(truncate_name("Goblin", 10), "Goblin");
        assert_eq!(truncate_name("Goblin King of the Mountain", 10), "Goblin Ki…");
        assert_eq!(truncate_name("AB", 2), "AB");
    }

    /// Helper to set up an active combat with 3 combatants.
    fn setup_active_combat() -> CombatViewState {
        let mut state = CombatViewState::new();
        state.phase = CombatPhase::Active;
        state.combat = CombatState::new();

        let mut fighter = Combatant::new("Fighter", 18, CombatantType::Player);
        fighter.current_hp = Some(50);
        fighter.max_hp = Some(50);
        state.combat.add_combatant(fighter);

        let mut goblin = Combatant::new("Goblin", 15, CombatantType::Monster);
        goblin.current_hp = Some(20);
        goblin.max_hp = Some(20);
        state.combat.add_combatant(goblin);

        let mut wizard = Combatant::new("Wizard", 12, CombatantType::Player);
        wizard.current_hp = Some(30);
        wizard.max_hp = Some(30);
        state.combat.add_combatant(wizard);

        state
    }
}
