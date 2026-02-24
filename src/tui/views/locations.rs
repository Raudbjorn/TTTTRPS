//! Location generator — wizard-style location creation and browsing.
//!
//! Uses `LocationGenerator::generate_quick()` for template-based generation.
//! Displays generated locations with atmosphere, features, inhabitants,
//! secrets, encounters, and loot.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::super::theme;
use crate::core::location_gen::{Location, LocationGenerationOptions};
use crate::tui::services::Services;

// ── Location type list ─────────────────────────────────────────────────────

const LOCATION_TYPES: &[(&str, &str)] = &[
    ("Tavern", "A place of rest, rumors, and quest hooks"),
    ("Dungeon", "Underground complex with traps and treasure"),
    ("Forest", "Woodland area with hidden paths and creatures"),
    ("Mountain", "Elevated terrain with caves and vantage points"),
    ("Castle", "Fortified structure with political intrigue"),
    ("Village", "Small settlement with local problems"),
    ("City", "Large urban center with diverse districts"),
    ("Temple", "Sacred ground with divine or dark powers"),
    ("Ruins", "Remnants of a fallen civilization"),
    ("Swamp", "Treacherous wetland with hidden dangers"),
    ("Desert", "Arid wasteland with buried secrets"),
    ("Coast", "Shoreline with maritime encounters"),
];

// ── Wizard phase ───────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    TypeSelect,
    Preview,
}

// ── State ──────────────────────────────────────────────────────────────────

pub struct LocationViewState {
    phase: Phase,
    selected_type: usize,
    scroll: usize,
    generated: Option<Location>,
}

impl LocationViewState {
    pub fn new() -> Self {
        Self {
            phase: Phase::TypeSelect,
            selected_type: 0,
            scroll: 0,
            generated: None,
        }
    }

    pub fn load(&mut self, _services: &Services) {
        // No async loading needed — generation is on-demand via Enter key
    }

    pub fn poll(&mut self) {
        // No async data to poll
    }

    pub fn handle_input(&mut self, event: &Event, services: &Services) -> bool {
        let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            modifiers,
            ..
        }) = event
        else {
            return false;
        };

        match self.phase {
            Phase::TypeSelect => self.handle_type_select(*modifiers, *code, services),
            Phase::Preview => self.handle_preview(*modifiers, *code, services),
        }
    }

    fn handle_type_select(
        &mut self,
        mods: KeyModifiers,
        code: KeyCode,
        services: &Services,
    ) -> bool {
        match (mods, code) {
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                if self.selected_type + 1 < LOCATION_TYPES.len() {
                    self.selected_type += 1;
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                self.selected_type = self.selected_type.saturating_sub(1);
                true
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                let (type_name, _) = LOCATION_TYPES[self.selected_type];
                let options = LocationGenerationOptions {
                    location_type: Some(type_name.to_lowercase()),
                    include_inhabitants: true,
                    include_secrets: true,
                    include_encounters: true,
                    include_loot: true,
                    ..Default::default()
                };
                let location = services.location_generator.generate_quick(&options);
                self.generated = Some(location);
                self.phase = Phase::Preview;
                self.scroll = 0;
                true
            }
            _ => false,
        }
    }

    fn handle_preview(
        &mut self,
        mods: KeyModifiers,
        code: KeyCode,
        services: &Services,
    ) -> bool {
        match (mods, code) {
            (KeyModifiers::NONE, KeyCode::Esc | KeyCode::Char('q')) => {
                self.phase = Phase::TypeSelect;
                self.generated = None;
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                // Re-generate with same type
                if let Some(ref loc) = self.generated {
                    let options = LocationGenerationOptions {
                        location_type: Some(format!("{:?}", loc.location_type).to_lowercase()),
                        include_inhabitants: true,
                        include_secrets: true,
                        include_encounters: true,
                        include_loot: true,
                        ..Default::default()
                    };
                    self.generated = Some(services.location_generator.generate_quick(&options));
                    self.scroll = 0;
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                self.scroll = self.scroll.saturating_add(1);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                self.scroll = self.scroll.saturating_sub(1);
                true
            }
            _ => false,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        match self.phase {
            Phase::TypeSelect => self.render_type_select(frame, area),
            Phase::Preview => self.render_preview(frame, area),
        }
    }

    fn render_type_select(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        // Left: type list
        let block = theme::block_focused("Location Type");
        let inner = block.inner(chunks[0]);
        frame.render_widget(block, chunks[0]);

        let mut lines: Vec<Line<'static>> = Vec::new();
        for (i, (name, _)) in LOCATION_TYPES.iter().enumerate() {
            let is_selected = i == self.selected_type;
            let marker = if is_selected { "\u{25b8} " } else { "  " };
            let style = if is_selected {
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };
            lines.push(Line::from(Span::styled(
                format!("{marker}{name}"),
                style,
            )));
        }

        frame.render_widget(Paragraph::new(lines), inner);

        // Right: description
        let block = theme::block_default("Description");
        let inner = block.inner(chunks[1]);
        frame.render_widget(block, chunks[1]);

        let (name, desc) = LOCATION_TYPES[self.selected_type];
        let lines = vec![
            Line::raw(""),
            Line::from(Span::styled(
                format!("  {name}"),
                Style::default()
                    .fg(theme::PRIMARY_LIGHT)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                format!("  {desc}"),
                Style::default().fg(theme::TEXT),
            )),
            Line::raw(""),
            Line::raw(""),
            Line::from(Span::styled(
                "  [Enter] generate  [j/k] navigate",
                Style::default().fg(theme::TEXT_DIM),
            )),
        ];

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_preview(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_focused("Generated Location");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(ref loc) = self.generated else {
            return;
        };

        let mut lines: Vec<Line<'static>> = Vec::new();

        // Header
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("  {} ({:?})", loc.name, loc.location_type),
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
        if !loc.tags.is_empty() {
            lines.push(Line::from(vec![
                Span::raw("  Tags: "),
                Span::styled(loc.tags.join(", "), Style::default().fg(theme::TEXT_DIM)),
            ]));
        }

        // Description
        lines.push(Line::raw(""));
        section_header(&mut lines, "DESCRIPTION");
        for part in loc.description.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {part}"),
                Style::default().fg(theme::TEXT),
            )));
        }

        // Atmosphere
        lines.push(Line::raw(""));
        section_header(&mut lines, "ATMOSPHERE");
        lines.push(detail_line("Mood", &loc.atmosphere.mood));
        lines.push(detail_line("Lighting", &loc.atmosphere.lighting));
        if !loc.atmosphere.sounds.is_empty() {
            lines.push(detail_line("Sounds", &loc.atmosphere.sounds.join(", ")));
        }
        if !loc.atmosphere.smells.is_empty() {
            lines.push(detail_line("Smells", &loc.atmosphere.smells.join(", ")));
        }

        // Notable features
        if !loc.notable_features.is_empty() {
            lines.push(Line::raw(""));
            section_header(&mut lines, "NOTABLE FEATURES");
            for feat in &loc.notable_features {
                let flags = match (feat.interactive, feat.hidden) {
                    (true, true) => " [interactive, hidden]",
                    (true, false) => " [interactive]",
                    (false, true) => " [hidden]",
                    (false, false) => "",
                };
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{}{flags}", feat.name),
                        Style::default()
                            .fg(theme::PRIMARY_LIGHT)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(Span::styled(
                    format!("    {}", feat.description),
                    Style::default().fg(theme::TEXT),
                )));
            }
        }

        // Inhabitants
        if !loc.inhabitants.is_empty() {
            lines.push(Line::raw(""));
            section_header(&mut lines, "INHABITANTS");
            for npc in &loc.inhabitants {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{} — {}", npc.name, npc.role),
                        Style::default()
                            .fg(theme::PRIMARY_LIGHT)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" ({:?})", npc.disposition),
                        Style::default().fg(theme::TEXT_DIM),
                    ),
                ]));
                lines.push(Line::from(Span::styled(
                    format!("    {}", npc.description),
                    Style::default().fg(theme::TEXT),
                )));
                if !npc.services.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("    Services: {}", npc.services.join(", ")),
                        Style::default().fg(theme::TEXT_DIM),
                    )));
                }
            }
        }

        // Secrets
        if !loc.secrets.is_empty() {
            lines.push(Line::raw(""));
            section_header(&mut lines, "SECRETS");
            for secret in &loc.secrets {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  [{:?}] ", secret.difficulty_to_discover),
                        Style::default().fg(theme::TEXT_DIM),
                    ),
                    Span::styled(
                        secret.description.clone(),
                        Style::default().fg(theme::TEXT),
                    ),
                ]));
                if !secret.clues.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("    Clues: {}", secret.clues.join("; ")),
                        Style::default().fg(theme::TEXT_DIM),
                    )));
                }
            }
        }

        // Encounters
        if !loc.encounters.is_empty() {
            lines.push(Line::raw(""));
            section_header(&mut lines, "ENCOUNTERS");
            for enc in &loc.encounters {
                let opt = if enc.optional { " (optional)" } else { "" };
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{}{opt}", enc.name),
                        Style::default()
                            .fg(theme::PRIMARY_LIGHT)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" [{:?}]", enc.difficulty),
                        Style::default().fg(theme::TEXT_DIM),
                    ),
                ]));
                lines.push(Line::from(Span::styled(
                    format!("    {}", enc.description),
                    Style::default().fg(theme::TEXT),
                )));
                lines.push(Line::from(Span::styled(
                    format!("    Trigger: {}", enc.trigger),
                    Style::default().fg(theme::TEXT_DIM),
                )));
            }
        }

        // Loot
        if let Some(ref loot) = loc.loot_potential {
            lines.push(Line::raw(""));
            section_header(&mut lines, "LOOT POTENTIAL");
            lines.push(detail_line("Level", &format!("{:?}", loot.treasure_level)));
            if !loot.notable_items.is_empty() {
                lines.push(detail_line("Items", &loot.notable_items.join(", ")));
            }
            if loot.hidden_caches > 0 {
                lines.push(detail_line(
                    "Caches",
                    &format!("{} hidden cache(s)", loot.hidden_caches),
                ));
            }
        }

        // Footer
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  [Esc] back  [r] regenerate  [j/k] scroll",
            Style::default().fg(theme::TEXT_DIM),
        )));

        frame.render_widget(
            Paragraph::new(lines).scroll((self.scroll as u16, 0)),
            inner,
        );
    }
}

fn section_header(lines: &mut Vec<Line<'static>>, title: &str) {
    lines.push(Line::from(Span::styled(
        format!("  {title}"),
        Style::default()
            .fg(theme::PRIMARY)
            .add_modifier(Modifier::BOLD),
    )));
}

fn detail_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {:<12}", label),
            Style::default().fg(theme::TEXT_MUTED),
        ),
        Span::styled(value.to_string(), Style::default().fg(theme::TEXT)),
    ])
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = LocationViewState::new();
        assert_eq!(state.phase, Phase::TypeSelect);
        assert_eq!(state.selected_type, 0);
        assert!(state.generated.is_none());
    }

    #[test]
    fn test_type_selection_bounds() {
        let mut state = LocationViewState::new();
        state.selected_type = 0;
        state.selected_type = state.selected_type.saturating_sub(1);
        assert_eq!(state.selected_type, 0);
        state.selected_type = LOCATION_TYPES.len() - 1;
        assert_eq!(state.selected_type, LOCATION_TYPES.len() - 1);
    }

    #[test]
    fn test_location_types_nonempty() {
        assert!(!LOCATION_TYPES.is_empty());
        for (name, desc) in LOCATION_TYPES {
            assert!(!name.is_empty());
            assert!(!desc.is_empty());
        }
    }
}
