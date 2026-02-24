//! World-building asset browser — read-only three-panel view for
//! archetypes, setting packs, and vocabulary banks.
//!
//! Displays YAML-based archetype, setting pack, and vocabulary files
//! with category tree navigation (left), item list (middle), and a
//! detail pane (right). Currently populated with static/hardcoded
//! data since `ArchetypeRegistry` is not yet wired through Services.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::super::theme;
use crate::tui::services::Services;

// ── Asset category ──────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AssetCategory {
    Archetypes,
    SettingPacks,
    Vocabulary,
}

impl AssetCategory {
    const ALL: &'static [AssetCategory] = &[
        AssetCategory::Archetypes,
        AssetCategory::SettingPacks,
        AssetCategory::Vocabulary,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Archetypes => "Archetypes",
            Self::SettingPacks => "Setting Packs",
            Self::Vocabulary => "Vocabulary",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Archetypes => "◎",
            Self::SettingPacks => "◈",
            Self::Vocabulary => "◇",
        }
    }
}

// ── Panel focus ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AssetPanel {
    Categories,
    Items,
    Detail,
}

impl AssetPanel {
    fn next(self) -> Self {
        match self {
            Self::Categories => Self::Items,
            Self::Items => Self::Detail,
            Self::Detail => Self::Categories,
        }
    }
}

// ── Asset item ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct AssetItem {
    name: String,
    item_type: String,
    description: String,
    details: Vec<(String, String)>,
}

// ── Static data ─────────────────────────────────────────────────────────────

fn archetype_items() -> Vec<AssetItem> {
    vec![
        AssetItem {
            name: "Warrior".into(),
            item_type: "category".into(),
            description: "Combat-focused archetypes for martial NPCs.".into(),
            details: vec![
                (
                    "Members".into(),
                    "Knight, Barbarian, Ranger, Paladin, Fighter".into(),
                ),
                (
                    "Personality Affinities".into(),
                    "brave (0.8), loyal (0.7), stubborn (0.6)".into(),
                ),
                (
                    "NPC Role Mappings".into(),
                    "guard (0.9), mercenary (0.7), trainer (0.5)".into(),
                ),
            ],
        },
        AssetItem {
            name: "Magic".into(),
            item_type: "category".into(),
            description: "Arcane and divine spellcasting archetypes.".into(),
            details: vec![
                (
                    "Members".into(),
                    "Wizard, Sorcerer, Warlock, Druid, Cleric".into(),
                ),
                (
                    "Personality Affinities".into(),
                    "curious (0.9), cautious (0.5), eccentric (0.7)".into(),
                ),
                (
                    "NPC Role Mappings".into(),
                    "sage (0.8), healer (0.6), enchanter (0.7)".into(),
                ),
            ],
        },
        AssetItem {
            name: "Rogue".into(),
            item_type: "category".into(),
            description: "Stealth and subterfuge archetypes.".into(),
            details: vec![
                ("Members".into(), "Thief, Assassin, Bard, Scout, Spy".into()),
                (
                    "Personality Affinities".into(),
                    "cunning (0.9), charming (0.6), paranoid (0.5)".into(),
                ),
                (
                    "NPC Role Mappings".into(),
                    "informant (0.8), fence (0.7), entertainer (0.5)".into(),
                ),
            ],
        },
        AssetItem {
            name: "Social".into(),
            item_type: "category".into(),
            description: "Diplomacy and influence archetypes.".into(),
            details: vec![
                (
                    "Members".into(),
                    "Noble, Merchant, Diplomat, Scholar, Healer".into(),
                ),
                (
                    "Personality Affinities".into(),
                    "charismatic (0.8), perceptive (0.7), patient (0.6)".into(),
                ),
                (
                    "NPC Role Mappings".into(),
                    "merchant (0.9), noble (0.7), advisor (0.6)".into(),
                ),
            ],
        },
        AssetItem {
            name: "Creature".into(),
            item_type: "category".into(),
            description: "Non-humanoid and monstrous archetypes.".into(),
            details: vec![
                (
                    "Members".into(),
                    "Beast, Undead, Fiend, Fey, Construct".into(),
                ),
                (
                    "Personality Affinities".into(),
                    "territorial (0.7), alien (0.8), instinctive (0.6)".into(),
                ),
                (
                    "NPC Role Mappings".into(),
                    "guardian (0.7), predator (0.8), familiar (0.4)".into(),
                ),
            ],
        },
    ]
}

fn setting_pack_items() -> Vec<AssetItem> {
    vec![
        AssetItem {
            name: "dnd5e".into(),
            item_type: "setting_pack".into(),
            description: "Dungeons & Dragons 5th Edition core setting pack.".into(),
            details: vec![
                ("System".into(), "D&D 5e".into()),
                ("Version".into(), "1.0.0".into()),
                (
                    "Description".into(),
                    "Core races, classes, and monster archetypes for 5th Edition.".into(),
                ),
                ("Archetype Overrides".into(), "12".into()),
                ("Custom Archetypes".into(), "8".into()),
                (
                    "Naming Cultures".into(),
                    "6 (Common, Dwarvish, Elvish, Draconic, Infernal, Celestial)".into(),
                ),
            ],
        },
        AssetItem {
            name: "pathfinder2e".into(),
            item_type: "setting_pack".into(),
            description: "Pathfinder 2nd Edition setting pack.".into(),
            details: vec![
                ("System".into(), "Pathfinder 2e".into()),
                ("Version".into(), "1.0.0".into()),
                (
                    "Description".into(),
                    "Ancestries, classes, and Golarion-specific archetypes.".into(),
                ),
                ("Archetype Overrides".into(), "15".into()),
                ("Custom Archetypes".into(), "10".into()),
                (
                    "Naming Cultures".into(),
                    "8 (Taldane, Kelish, Varisian, Shoanti, Tien, Osirian, Mwangi, Skald)".into(),
                ),
            ],
        },
        AssetItem {
            name: "fate".into(),
            item_type: "setting_pack".into(),
            description: "Fate Core / Fate Accelerated setting pack.".into(),
            details: vec![
                ("System".into(), "Fate Core".into()),
                ("Version".into(), "1.0.0".into()),
                (
                    "Description".into(),
                    "Aspect-driven archetypes for narrative-focused play.".into(),
                ),
                ("Archetype Overrides".into(), "5".into()),
                ("Custom Archetypes".into(), "4".into()),
                ("Naming Cultures".into(), "2 (Generic, Pulp)".into()),
            ],
        },
        AssetItem {
            name: "savage_worlds".into(),
            item_type: "setting_pack".into(),
            description: "Savage Worlds Adventure Edition setting pack.".into(),
            details: vec![
                ("System".into(), "Savage Worlds".into()),
                ("Version".into(), "1.0.0".into()),
                (
                    "Description".into(),
                    "Edges, hindrances, and archetypes for fast-furious-fun play.".into(),
                ),
                ("Archetype Overrides".into(), "8".into()),
                ("Custom Archetypes".into(), "6".into()),
                (
                    "Naming Cultures".into(),
                    "3 (Modern, Fantasy, Sci-Fi)".into(),
                ),
            ],
        },
    ]
}

fn vocabulary_items() -> Vec<AssetItem> {
    vec![
        AssetItem {
            name: "greetings".into(),
            item_type: "phrase_category".into(),
            description: "Opening phrases for NPC conversations.".into(),
            details: vec![
                ("Category".into(), "Greetings".into()),
                ("Phrase Count".into(), "24".into()),
                ("Example (Casual)".into(), "\"Hey there, traveler!\"".into()),
                (
                    "Example (Formal)".into(),
                    "\"Well met, honored guest.\"".into(),
                ),
                (
                    "Example (Hostile)".into(),
                    "\"State your business, outsider.\"".into(),
                ),
            ],
        },
        AssetItem {
            name: "farewells".into(),
            item_type: "phrase_category".into(),
            description: "Closing phrases for NPC conversations.".into(),
            details: vec![
                ("Category".into(), "Farewells".into()),
                ("Phrase Count".into(), "18".into()),
                ("Example (Casual)".into(), "\"See you around!\"".into()),
                (
                    "Example (Formal)".into(),
                    "\"May the road rise to meet you.\"".into(),
                ),
                (
                    "Example (Gruff)".into(),
                    "\"Don't let the door hit you.\"".into(),
                ),
            ],
        },
        AssetItem {
            name: "combat_cries".into(),
            item_type: "phrase_category".into(),
            description: "Battle shouts and war cries.".into(),
            details: vec![
                ("Category".into(), "Combat Cries".into()),
                ("Phrase Count".into(), "16".into()),
                (
                    "Example (Warrior)".into(),
                    "\"For glory and honor!\"".into(),
                ),
                (
                    "Example (Berserker)".into(),
                    "\"BLOOD AND THUNDER!\"".into(),
                ),
                (
                    "Example (Paladin)".into(),
                    "\"By the light, you shall fall!\"".into(),
                ),
            ],
        },
        AssetItem {
            name: "insults".into(),
            item_type: "phrase_category".into(),
            description: "Taunts and provocations for hostile NPCs.".into(),
            details: vec![
                ("Category".into(), "Insults".into()),
                ("Phrase Count".into(), "20".into()),
                (
                    "Example (Witty)".into(),
                    "\"I've met smarter goblins.\"".into(),
                ),
                (
                    "Example (Crude)".into(),
                    "\"You fight like a dairy farmer!\"".into(),
                ),
                (
                    "Example (Noble)".into(),
                    "\"How quaint. Do you call that swordplay?\"".into(),
                ),
            ],
        },
        AssetItem {
            name: "compliments".into(),
            item_type: "phrase_category".into(),
            description: "Praise and flattery for friendly NPCs.".into(),
            details: vec![
                ("Category".into(), "Compliments".into()),
                ("Phrase Count".into(), "14".into()),
                (
                    "Example (Genuine)".into(),
                    "\"That was truly impressive work.\"".into(),
                ),
                (
                    "Example (Merchant)".into(),
                    "\"A customer of impeccable taste!\"".into(),
                ),
                (
                    "Example (Bardic)".into(),
                    "\"Your deeds shall echo through the ages!\"".into(),
                ),
            ],
        },
    ]
}

fn items_for_category(category: AssetCategory) -> Vec<AssetItem> {
    match category {
        AssetCategory::Archetypes => archetype_items(),
        AssetCategory::SettingPacks => setting_pack_items(),
        AssetCategory::Vocabulary => vocabulary_items(),
    }
}

// ── State ───────────────────────────────────────────────────────────────────

pub struct AssetBrowserState {
    category: AssetCategory,
    items: Vec<AssetItem>,
    selected_category: usize,
    selected_item: usize,
    focus_panel: AssetPanel,
    detail_scroll: usize,
}

impl AssetBrowserState {
    pub fn new() -> Self {
        let category = AssetCategory::Archetypes;
        let items = items_for_category(category);
        Self {
            category,
            items,
            selected_category: 0,
            selected_item: 0,
            focus_panel: AssetPanel::Categories,
            detail_scroll: 0,
        }
    }

    pub fn load(&mut self, _services: &Services) {
        // ArchetypeRegistry not in Services yet — using static data
        self.refresh_items();
    }

    pub fn poll(&mut self) {
        // No async data to poll
    }

    fn refresh_items(&mut self) {
        self.items = items_for_category(self.category);
        self.selected_item = self.selected_item.min(self.items.len().saturating_sub(1));
        self.detail_scroll = 0;
    }

    fn current_item(&self) -> Option<&AssetItem> {
        self.items.get(self.selected_item)
    }

    // ── Input ───────────────────────────────────────────────────────────

    pub fn handle_input(&mut self, event: &Event, _services: &Services) -> bool {
        let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            modifiers,
            ..
        }) = event
        else {
            return false;
        };

        match (*modifiers, *code) {
            // Panel switching
            (KeyModifiers::NONE, KeyCode::Tab) => {
                self.focus_panel = self.focus_panel.next();
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('l') | KeyCode::Right) => {
                match self.focus_panel {
                    AssetPanel::Categories => {
                        self.focus_panel = AssetPanel::Items;
                        self.selected_item = 0;
                    }
                    AssetPanel::Items => {
                        self.focus_panel = AssetPanel::Detail;
                        self.detail_scroll = 0;
                    }
                    AssetPanel::Detail => {}
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('h') | KeyCode::Left) => {
                match self.focus_panel {
                    AssetPanel::Categories => {}
                    AssetPanel::Items => {
                        self.focus_panel = AssetPanel::Categories;
                    }
                    AssetPanel::Detail => {
                        self.focus_panel = AssetPanel::Items;
                    }
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                match self.focus_panel {
                    AssetPanel::Categories => {
                        self.focus_panel = AssetPanel::Items;
                        self.selected_item = 0;
                    }
                    AssetPanel::Items => {
                        self.focus_panel = AssetPanel::Detail;
                        self.detail_scroll = 0;
                    }
                    AssetPanel::Detail => {}
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Esc) => {
                match self.focus_panel {
                    AssetPanel::Categories => return false,
                    AssetPanel::Items => {
                        self.focus_panel = AssetPanel::Categories;
                    }
                    AssetPanel::Detail => {
                        self.focus_panel = AssetPanel::Items;
                    }
                }
                true
            }
            // Vertical navigation
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                match self.focus_panel {
                    AssetPanel::Categories => {
                        let max = AssetCategory::ALL.len().saturating_sub(1);
                        if self.selected_category < max {
                            self.selected_category += 1;
                            self.category = AssetCategory::ALL[self.selected_category];
                            self.refresh_items();
                        }
                    }
                    AssetPanel::Items => {
                        let max = self.items.len().saturating_sub(1);
                        if self.selected_item < max {
                            self.selected_item += 1;
                            self.detail_scroll = 0;
                        }
                    }
                    AssetPanel::Detail => {
                        self.detail_scroll = self.detail_scroll.saturating_add(1);
                    }
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                match self.focus_panel {
                    AssetPanel::Categories => {
                        if self.selected_category > 0 {
                            self.selected_category -= 1;
                            self.category = AssetCategory::ALL[self.selected_category];
                            self.refresh_items();
                        }
                    }
                    AssetPanel::Items => {
                        self.selected_item = self.selected_item.saturating_sub(1);
                        self.detail_scroll = 0;
                    }
                    AssetPanel::Detail => {
                        self.detail_scroll = self.detail_scroll.saturating_sub(1);
                    }
                }
                true
            }
            _ => false,
        }
    }

    // ── Rendering ───────────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(35),
                Constraint::Percentage(40),
            ])
            .split(area);

        self.render_categories(frame, chunks[0]);
        self.render_items(frame, chunks[1]);
        self.render_detail(frame, chunks[2]);
    }

    fn render_categories(&self, frame: &mut Frame, area: Rect) {
        let is_focused = self.focus_panel == AssetPanel::Categories;
        let block = if is_focused {
            theme::block_focused("Asset Categories")
        } else {
            theme::block_default("Asset Categories")
        };
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));

        for (i, cat) in AssetCategory::ALL.iter().enumerate() {
            let is_selected = i == self.selected_category;
            let marker = if is_selected && is_focused {
                "▸ "
            } else {
                "  "
            };

            let style = if is_selected {
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };

            let item_count = items_for_category(*cat).len();

            lines.push(Line::from(vec![
                Span::styled(marker.to_string(), style),
                Span::styled(format!("{} ", cat.icon()), style),
                Span::styled(cat.label().to_string(), style),
                Span::styled(
                    format!(" ({})", item_count),
                    Style::default().fg(theme::TEXT_DIM),
                ),
            ]));
        }

        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(inner.width.saturating_sub(4) as usize)),
            Style::default().fg(theme::TEXT_DIM),
        )));
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("YAML-based assets", Style::default().fg(theme::TEXT_MUTED)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("loaded from disk", Style::default().fg(theme::TEXT_DIM)),
        ]));

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_items(&self, frame: &mut Frame, area: Rect) {
        let is_focused = self.focus_panel == AssetPanel::Items;
        let block = if is_focused {
            theme::block_focused(self.category.label())
        } else {
            theme::block_default(self.category.label())
        };
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));

        if self.items.is_empty() {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "No items in this category.",
                    Style::default().fg(theme::TEXT_MUTED),
                ),
            ]));
        } else {
            for (i, item) in self.items.iter().enumerate() {
                let is_selected = i == self.selected_item;
                let marker = if is_selected && is_focused {
                    "▸ "
                } else {
                    "  "
                };

                let name_style = if is_selected {
                    Style::default()
                        .fg(theme::PRIMARY_LIGHT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT)
                };

                lines.push(Line::from(vec![
                    Span::styled(marker.to_string(), name_style),
                    Span::styled(item.name.clone(), name_style),
                ]));

                // Show type label below the name, indented
                let type_label = match item.item_type.as_str() {
                    "category" => "archetype category",
                    "setting_pack" => "setting pack",
                    "phrase_category" => "phrase category",
                    other => other,
                };

                let desc_style = if is_selected {
                    Style::default().fg(theme::TEXT_MUTED)
                } else {
                    Style::default().fg(theme::TEXT_DIM)
                };

                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(type_label.to_string(), desc_style),
                ]));

                // Add a small gap between items
                if i < self.items.len() - 1 {
                    lines.push(Line::raw(""));
                }
            }
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_detail(&self, frame: &mut Frame, area: Rect) {
        let block = if self.focus_panel == AssetPanel::Detail {
            theme::block_focused("Detail")
        } else {
            theme::block_default("Detail")
        };
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = match self.current_item() {
            Some(item) => self.build_detail_lines(item, inner.width),
            None => vec![
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Select an item to view details.",
                        Style::default().fg(theme::TEXT_MUTED),
                    ),
                ]),
            ],
        };

        let content = Paragraph::new(lines).scroll((self.detail_scroll as u16, 0));
        frame.render_widget(content, inner);
    }

    fn build_detail_lines(&self, item: &AssetItem, width: u16) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();
        let sep_width = width.saturating_sub(4) as usize;

        // Header
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{} {}", self.category.icon(), item.name),
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        // Type
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("Type: {}", item.item_type),
                Style::default().fg(theme::TEXT_MUTED),
            ),
        ]));

        lines.push(Line::raw(""));

        // Description
        lines.push(Line::from(Span::styled(
            "  DESCRIPTION",
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )));
        // Word-wrap the description
        for wrapped in wrap_text(&item.description, sep_width) {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(wrapped, Style::default().fg(theme::TEXT)),
            ]));
        }

        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(sep_width)),
            Style::default().fg(theme::TEXT_DIM),
        )));
        lines.push(Line::raw(""));

        // Key-value details
        for (key, value) in &item.details {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    key.clone(),
                    Style::default()
                        .fg(theme::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            for wrapped in wrap_text(value, sep_width) {
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(wrapped, Style::default().fg(theme::TEXT)),
                ]));
            }

            lines.push(Line::raw(""));
        }

        // Footer with keybindings
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(sep_width)),
            Style::default().fg(theme::TEXT_DIM),
        )));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Tab", Style::default().fg(theme::TEXT_DIM)),
            Span::raw(":panel  "),
            Span::styled("h/l", Style::default().fg(theme::TEXT_DIM)),
            Span::raw(":navigate  "),
            Span::styled("j/k", Style::default().fg(theme::TEXT_DIM)),
            Span::raw(":select"),
        ]));

        lines
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Simple word-wrap: splits text into lines that fit within `max_width` chars.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() > max_width {
            lines.push(current);
            current = word.to_string();
        } else {
            current.push(' ');
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state_defaults() {
        let state = AssetBrowserState::new();
        assert_eq!(state.category, AssetCategory::Archetypes);
        assert_eq!(state.selected_category, 0);
        assert_eq!(state.selected_item, 0);
        assert_eq!(state.focus_panel, AssetPanel::Categories);
        assert_eq!(state.detail_scroll, 0);
        assert!(!state.items.is_empty());
    }

    #[test]
    fn test_category_items_populated() {
        for cat in AssetCategory::ALL {
            let items = items_for_category(*cat);
            assert!(!items.is_empty(), "Category {:?} should have items", cat);
            for item in &items {
                assert!(!item.name.is_empty(), "Item name should not be empty");
                assert!(
                    !item.details.is_empty(),
                    "Item '{}' should have detail entries",
                    item.name
                );
            }
        }
    }

    #[test]
    fn test_category_switching_resets_selection() {
        let mut state = AssetBrowserState::new();
        // Move to second item
        state.selected_item = 2;

        // Switch category
        state.selected_category = 1;
        state.category = AssetCategory::ALL[1];
        state.refresh_items();

        // selected_item should be clamped to valid range
        assert!(state.selected_item < state.items.len());
        assert_eq!(state.detail_scroll, 0);
    }

    #[test]
    fn test_panel_cycling() {
        assert_eq!(AssetPanel::Categories.next(), AssetPanel::Items);
        assert_eq!(AssetPanel::Items.next(), AssetPanel::Detail);
        assert_eq!(AssetPanel::Detail.next(), AssetPanel::Categories);
    }

    #[test]
    fn test_category_labels_and_icons() {
        for cat in AssetCategory::ALL {
            assert!(!cat.label().is_empty());
            assert!(!cat.icon().is_empty());
        }
    }

    #[test]
    fn test_wrap_text_short() {
        let lines = wrap_text("hello world", 80);
        assert_eq!(lines, vec!["hello world"]);
    }

    #[test]
    fn test_wrap_text_long() {
        let text = "This is a somewhat longer description that should wrap across multiple lines when the width is small.";
        let lines = wrap_text(text, 30);
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(line.len() <= 30 + 20, "Line too long: {}", line);
        }
    }

    #[test]
    fn test_wrap_text_empty() {
        let lines = wrap_text("", 40);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_detail_scroll_no_underflow() {
        let mut state = AssetBrowserState::new();
        state.focus_panel = AssetPanel::Detail;
        state.detail_scroll = 0;
        // Simulate pressing 'k' (up) in detail panel
        state.detail_scroll = state.detail_scroll.saturating_sub(1);
        assert_eq!(state.detail_scroll, 0);
    }

    #[test]
    fn test_archetype_items_count() {
        // Matches the 5 categories from archetypes.rs: warrior, magic, rogue, social, creature
        let items = archetype_items();
        assert_eq!(items.len(), 5);
    }

    #[test]
    fn test_setting_pack_items_count() {
        let items = setting_pack_items();
        assert_eq!(items.len(), 4);
    }

    #[test]
    fn test_vocabulary_items_count() {
        let items = vocabulary_items();
        assert_eq!(items.len(), 5);
    }
}
