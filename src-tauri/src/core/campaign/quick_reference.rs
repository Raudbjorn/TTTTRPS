//! Quick Reference Card Manager
//!
//! Phase 9 of the Campaign Generation Overhaul.
//!
//! Provides quick reference card rendering, card tray management, and hover preview
//! generation for campaign entities (NPCs, Locations, Items, Plot Points, Scenes).
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                    QuickReferenceCardManager                     │
//! │  ┌────────────────┐  ┌───────────────┐  ┌───────────────────┐   │
//! │  │ Card Renderer  │  │ Card Tray     │  │ Hover Preview     │   │
//! │  │ (HTML/Text)    │  │ (Pin/Unpin)   │  │ Generator         │   │
//! │  └────────────────┘  └───────────────┘  └───────────────────┘   │
//! └──────────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                        Database Layer                            │
//! │  ┌────────────────┐  ┌───────────────┐  ┌───────────────────┐   │
//! │  │ Entity Tables  │  │ Pinned Cards  │  │ Card Cache        │   │
//! │  │ (NPC, Location)│  │ (max 6)       │  │ (HTML TTL)        │   │
//! │  └────────────────┘  └───────────────┘  └───────────────────┘   │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use crate::core::campaign::quick_reference::{
//!     QuickReferenceCardManager, CardEntityType, DisclosureLevel,
//! };
//!
//! let manager = QuickReferenceCardManager::new(database);
//!
//! // Render an NPC card
//! let card = manager.render_entity_card(
//!     CardEntityType::Npc,
//!     "npc-123",
//!     DisclosureLevel::Summary,
//! ).await?;
//!
//! // Pin a card to the session tray
//! manager.pin_card("session-456", CardEntityType::Npc, "npc-123").await?;
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};

use crate::database::{
    CharacterOps, Database, CardEntityType, DisclosureLevel, LocationOps, NpcOps,
    PinnedCardRecord, CardCacheRecord, NpcRecord, LocationRecord, QuickReferenceOps,
};
use super::cheat_sheet::HtmlExporter;

/// Maximum number of pinned cards per session
pub const MAX_PINNED_CARDS: usize = 6;

/// Default cache TTL in hours
pub const DEFAULT_CACHE_TTL_HOURS: i64 = 24;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during quick reference operations
#[derive(Debug, Clone, Error)]
pub enum QuickReferenceError {
    #[error("Entity not found: {entity_type} {entity_id}")]
    EntityNotFound {
        entity_type: String,
        entity_id: String,
    },

    #[error("Maximum pinned cards ({max}) reached for session")]
    MaxPinnedCardsReached { max: usize },

    #[error("Card already pinned: {entity_type} {entity_id}")]
    AlreadyPinned {
        entity_type: String,
        entity_id: String,
    },

    #[error("Card not pinned: {entity_type} {entity_id}")]
    NotPinned {
        entity_type: String,
        entity_id: String,
    },

    #[error("Invalid display order: {order}")]
    InvalidDisplayOrder { order: i32 },

    #[error("Database error: {0}")]
    Database(String),

    #[error("Render error: {0}")]
    Render(String),
}

impl From<crate::database::quick_reference::QuickReferenceError> for QuickReferenceError {
    fn from(e: crate::database::quick_reference::QuickReferenceError) -> Self {
        QuickReferenceError::Database(e.to_string())
    }
}


impl From<sqlx::Error> for QuickReferenceError {
    fn from(err: sqlx::Error) -> Self {
        QuickReferenceError::Database(err.to_string())
    }
}

// ============================================================================
// Rendered Card Types
// ============================================================================

/// Rendered entity card for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedCard {
    /// Entity type
    pub entity_type: CardEntityType,
    /// Entity ID
    pub entity_id: String,
    /// Disclosure level used for rendering
    pub disclosure_level: DisclosureLevel,
    /// Entity name/title
    pub title: String,
    /// Entity subtitle (e.g., role, location type)
    pub subtitle: Option<String>,
    /// HTML content for the card body
    pub html_content: String,
    /// Plain text content (for accessibility/search)
    pub text_content: String,
    /// Whether this card is pinned in the current session
    pub is_pinned: bool,
    /// Pin ID if pinned
    pub pin_id: Option<String>,
    /// Quick stats for the header
    pub quick_stats: Vec<QuickStat>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Quick stat for card header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickStat {
    pub label: String,
    pub value: String,
    pub icon: Option<String>,
}

/// Hover preview (minimal card for tooltips)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverPreview {
    pub entity_type: CardEntityType,
    pub entity_id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub summary: String,
    pub quick_stats: Vec<QuickStat>,
}

// ============================================================================
// Card Tray Types
// ============================================================================

/// Card tray state for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardTray {
    pub session_id: String,
    pub cards: Vec<PinnedCard>,
    pub max_cards: usize,
    pub slots_remaining: usize,
}

/// Pinned card with rendered content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedCard {
    pub pin_id: String,
    pub entity_type: CardEntityType,
    pub entity_id: String,
    pub display_order: i32,
    pub disclosure_level: DisclosureLevel,
    pub rendered: RenderedCard,
    pub pinned_at: String,
}

// ============================================================================
// QuickReferenceCardManager
// ============================================================================

/// Manager for quick reference card operations
pub struct QuickReferenceCardManager<'a> {
    database: &'a Database,
}

impl<'a> QuickReferenceCardManager<'a> {
    /// Create a new QuickReferenceCardManager
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    // =========================================================================
    // Card Rendering
    // =========================================================================

    /// Render an entity card at a specific disclosure level
    pub async fn render_entity_card(
        &self,
        entity_type: CardEntityType,
        entity_id: &str,
        disclosure_level: DisclosureLevel,
        session_id: Option<&str>,
    ) -> Result<RenderedCard, QuickReferenceError> {
        // Check cache first
        if let Ok(Some(cached)) = self.database
            .get_card_cache(entity_type.as_str(), entity_id, disclosure_level.as_str())
            .await
        {
            if !cached.is_expired() {
                debug!(
                    entity_type = %entity_type,
                    entity_id = %entity_id,
                    "Using cached card HTML"
                );
                return self.build_rendered_card_from_cache(
                    entity_type,
                    entity_id,
                    disclosure_level,
                    &cached,
                    session_id,
                ).await;
            }
        }

        // Render based on entity type
        let card = match entity_type {
            CardEntityType::Npc => self.render_npc_card(entity_id, disclosure_level).await?,
            CardEntityType::Location => self.render_location_card(entity_id, disclosure_level).await?,
            CardEntityType::Item => self.render_item_card(entity_id, disclosure_level).await?,
            CardEntityType::PlotPoint => self.render_plot_point_card(entity_id, disclosure_level).await?,
            CardEntityType::Scene => self.render_scene_card(entity_id, disclosure_level).await?,
            CardEntityType::Character => self.render_character_card(entity_id, disclosure_level).await?,
        };

        // Cache the rendered HTML
        let cache = CardCacheRecord::new(
            entity_type,
            entity_id.to_string(),
            disclosure_level,
            card.html_content.clone(),
            DEFAULT_CACHE_TTL_HOURS,
        );
        if let Err(e) = self.database.save_card_cache(&cache).await {
            warn!(error = %e, "Failed to cache card HTML");
        }

        // Check if pinned
        let mut card = card;
        if let Some(sid) = session_id {
            if let Ok(pinned) = self.database.is_entity_pinned(sid, entity_type.as_str(), entity_id).await {
                card.is_pinned = pinned;
                if pinned {
                    // Get the pin ID
                    if let Ok(cards) = self.database.get_pinned_cards(sid).await {
                        card.pin_id = cards.iter()
                            .find(|c| c.entity_type == entity_type.as_str() && c.entity_id == entity_id)
                            .map(|c| c.id.clone());
                    }
                }
            }
        }

        Ok(card)
    }

    /// Generate a hover preview for an entity
    pub async fn generate_hover_preview(
        &self,
        entity_type: CardEntityType,
        entity_id: &str,
    ) -> Result<HoverPreview, QuickReferenceError> {
        // Always use minimal disclosure for hover previews
        let card = self.render_entity_card(
            entity_type,
            entity_id,
            DisclosureLevel::Minimal,
            None,
        ).await?;

        Ok(HoverPreview {
            entity_type: card.entity_type,
            entity_id: card.entity_id,
            title: card.title,
            subtitle: card.subtitle,
            summary: self.extract_summary(&card.text_content),
            quick_stats: card.quick_stats,
        })
    }

    // =========================================================================
    // Card Tray Management
    // =========================================================================

    /// Get the card tray for a session
    pub async fn get_card_tray(&self, session_id: &str) -> Result<CardTray, QuickReferenceError> {
        let pinned_records = self.database.get_pinned_cards(session_id).await?;

        let mut cards = Vec::with_capacity(pinned_records.len());
        for record in pinned_records {
            let entity_type = record.entity_type_enum()
                .map_err(|e| QuickReferenceError::Render(e))?;
            let disclosure_level = record.disclosure_level_enum()
                .map_err(|e| QuickReferenceError::Render(e))?;

            let rendered = self.render_entity_card(
                entity_type,
                &record.entity_id,
                disclosure_level,
                Some(session_id),
            ).await?;

            cards.push(PinnedCard {
                pin_id: record.id,
                entity_type,
                entity_id: record.entity_id,
                display_order: record.display_order,
                disclosure_level,
                rendered,
                pinned_at: record.pinned_at,
            });
        }

        let slots_remaining = MAX_PINNED_CARDS.saturating_sub(cards.len());

        Ok(CardTray {
            session_id: session_id.to_string(),
            cards,
            max_cards: MAX_PINNED_CARDS,
            slots_remaining,
        })
    }

    /// Pin a card to the session tray
    ///
    /// Uses optimistic insertion with unique constraint handling to prevent TOCTOU race
    /// conditions. The database has a unique constraint on (session_id, entity_type, entity_id).
    pub async fn pin_card(
        &self,
        session_id: &str,
        entity_type: CardEntityType,
        entity_id: &str,
        disclosure_level: Option<DisclosureLevel>,
    ) -> Result<PinnedCard, QuickReferenceError> {
        // Check max cards first to get count for display_order
        // (The database transaction checks limit again for safety)
        let count = self.database.count_pinned_cards(session_id)
            .await
            .map_err(|e| QuickReferenceError::Database(e.to_string()))?;

        // Create the pinned card record
        let disclosure = disclosure_level.unwrap_or_default();
        let record = PinnedCardRecord::new(
            session_id.to_string(),
            entity_type.clone(),
            entity_id.to_string(),
            count, // Next available slot
        ).with_disclosure_level(disclosure);

        // Attempt to insert with limit - the transaction ensures brand-consistent checks
        match self.database.pin_card_with_limit(&record, MAX_PINNED_CARDS as i32).await {
            Ok(()) => {}
            Err(e) => {
                let err_str = e.to_string().to_lowercase();
                // Check for limit error (mapped to Protocol error in database layer)
                if err_str.contains("maximum pinned cards") {
                    return Err(QuickReferenceError::MaxPinnedCardsReached {
                        max: MAX_PINNED_CARDS,
                    });
                }
                // Check if this is a unique constraint violation
                if err_str.contains("unique") || err_str.contains("constraint") {
                    return Err(QuickReferenceError::AlreadyPinned {
                        entity_type: entity_type.to_string(),
                        entity_id: entity_id.to_string(),
                    });
                }
                return Err(QuickReferenceError::Database(e.to_string()));
            }
        }

        info!(
            session_id = %session_id,
            entity_type = %entity_type,
            entity_id = %entity_id,
            "Pinned card to tray"
        );

        // Render and return the pinned card
        let rendered = self.render_entity_card(
            entity_type,
            entity_id,
            disclosure,
            Some(session_id),
        ).await?;

        Ok(PinnedCard {
            pin_id: record.id,
            entity_type,
            entity_id: entity_id.to_string(),
            display_order: count,
            disclosure_level: disclosure,
            rendered,
            pinned_at: record.pinned_at,
        })
    }

    /// Unpin a card from the session tray
    pub async fn unpin_card(
        &self,
        session_id: &str,
        entity_type: CardEntityType,
        entity_id: &str,
    ) -> Result<(), QuickReferenceError> {
        self.database
            .unpin_and_reorder(session_id, entity_type.as_str(), entity_id)
            .await?;

        info!(
            session_id = %session_id,
            entity_type = %entity_type,
            entity_id = %entity_id,
            "Unpinned card from tray"
        );

        Ok(())
    }

    /// Reorder pinned cards in the tray
    pub async fn reorder_cards(
        &self,
        session_id: &str,
        card_ids_in_order: Vec<String>,
    ) -> Result<CardTray, QuickReferenceError> {
        // Validate all IDs exist and belong to this session
        let current = self.database.get_pinned_cards(session_id).await?;
        let current_ids: std::collections::HashSet<_> = current.iter().map(|c| &c.id).collect();

        for id in &card_ids_in_order {
            if !current_ids.contains(id) {
                return Err(QuickReferenceError::NotPinned {
                    entity_type: "unknown".to_string(),
                    entity_id: id.clone(),
                });
            }
        }

        self.database
            .reorder_pinned_cards(session_id, &card_ids_in_order)
            .await?;

        debug!(session_id = %session_id, "Reordered pinned cards");

        self.get_card_tray(session_id).await
    }

    /// Update the disclosure level of a pinned card
    pub async fn update_card_disclosure(
        &self,
        pin_id: &str,
        disclosure_level: DisclosureLevel,
    ) -> Result<(), QuickReferenceError> {
        self.database
            .update_pinned_card_disclosure(pin_id, disclosure_level.as_str())
            .await?;

        debug!(pin_id = %pin_id, level = %disclosure_level, "Updated card disclosure level");

        Ok(())
    }

    // =========================================================================
    // Entity-Specific Rendering
    // =========================================================================

    async fn render_npc_card(
        &self,
        entity_id: &str,
        disclosure_level: DisclosureLevel,
    ) -> Result<RenderedCard, QuickReferenceError> {
        let npc = self.database.get_npc(entity_id).await?
            .ok_or_else(|| QuickReferenceError::EntityNotFound {
                entity_type: "npc".to_string(),
                entity_id: entity_id.to_string(),
            })?;

        let (html_content, text_content, quick_stats, tags) =
            NpcCardRenderer::render(&npc, disclosure_level);

        Ok(RenderedCard {
            entity_type: CardEntityType::Npc,
            entity_id: entity_id.to_string(),
            disclosure_level,
            title: npc.name.clone(),
            subtitle: Some(npc.role.clone()),
            html_content,
            text_content,
            is_pinned: false,
            pin_id: None,
            quick_stats,
            tags,
        })
    }

    async fn render_location_card(
        &self,
        entity_id: &str,
        disclosure_level: DisclosureLevel,
    ) -> Result<RenderedCard, QuickReferenceError> {
        let location = self.database.get_location(entity_id).await?
            .ok_or_else(|| QuickReferenceError::EntityNotFound {
                entity_type: "location".to_string(),
                entity_id: entity_id.to_string(),
            })?;

        let (html_content, text_content, quick_stats, tags) =
            LocationCardRenderer::render(&location, disclosure_level);

        Ok(RenderedCard {
            entity_type: CardEntityType::Location,
            entity_id: entity_id.to_string(),
            disclosure_level,
            title: location.name.clone(),
            subtitle: Some(location.location_type.clone()),
            html_content,
            text_content,
            is_pinned: false,
            pin_id: None,
            quick_stats,
            tags,
        })
    }

    async fn render_item_card(
        &self,
        entity_id: &str,
        disclosure_level: DisclosureLevel,
    ) -> Result<RenderedCard, QuickReferenceError> {
        // Items are typically stored in character data or as custom entities
        // For now, return a placeholder
        let escaped_id = HtmlExporter::escape_html(entity_id);
        Ok(RenderedCard {
            entity_type: CardEntityType::Item,
            entity_id: entity_id.to_string(),
            disclosure_level,
            title: format!("Item {}", escaped_id),
            subtitle: Some("Item".to_string()),
            html_content: format!("<div class=\"card-item\"><p>Item details for {}</p></div>", escaped_id),
            text_content: format!("Item details for {}", escaped_id),
            is_pinned: false,
            pin_id: None,
            quick_stats: vec![],
            tags: vec!["item".to_string()],
        })
    }

    async fn render_plot_point_card(
        &self,
        entity_id: &str,
        disclosure_level: DisclosureLevel,
    ) -> Result<RenderedCard, QuickReferenceError> {
        // Plot points are stored in plot_points table
        // For now, return a placeholder
        let escaped_id = HtmlExporter::escape_html(entity_id);
        Ok(RenderedCard {
            entity_type: CardEntityType::PlotPoint,
            entity_id: entity_id.to_string(),
            disclosure_level,
            title: format!("Plot Point {}", escaped_id),
            subtitle: Some("Plot".to_string()),
            html_content: format!("<div class=\"card-plot\"><p>Plot point details for {}</p></div>", escaped_id),
            text_content: format!("Plot point details for {}", escaped_id),
            is_pinned: false,
            pin_id: None,
            quick_stats: vec![],
            tags: vec!["plot".to_string()],
        })
    }

    async fn render_scene_card(
        &self,
        entity_id: &str,
        disclosure_level: DisclosureLevel,
    ) -> Result<RenderedCard, QuickReferenceError> {
        // Scenes are typically part of session plans
        let escaped_id = HtmlExporter::escape_html(entity_id);
        Ok(RenderedCard {
            entity_type: CardEntityType::Scene,
            entity_id: entity_id.to_string(),
            disclosure_level,
            title: format!("Scene {}", escaped_id),
            subtitle: Some("Scene".to_string()),
            html_content: format!("<div class=\"card-scene\"><p>Scene details for {}</p></div>", escaped_id),
            text_content: format!("Scene details for {}", escaped_id),
            is_pinned: false,
            pin_id: None,
            quick_stats: vec![],
            tags: vec!["scene".to_string()],
        })
    }

    async fn render_character_card(
        &self,
        entity_id: &str,
        disclosure_level: DisclosureLevel,
    ) -> Result<RenderedCard, QuickReferenceError> {
        let character = self.database.get_character(entity_id).await?
            .ok_or_else(|| QuickReferenceError::EntityNotFound {
                entity_type: "character".to_string(),
                entity_id: entity_id.to_string(),
            })?;

        let (html_content, text_content, quick_stats, tags) =
            CharacterCardRenderer::render(&character, disclosure_level);

        Ok(RenderedCard {
            entity_type: CardEntityType::Character,
            entity_id: entity_id.to_string(),
            disclosure_level,
            title: character.name.clone(),
            subtitle: character.level.map(|l| format!("Level {}", l)),
            html_content,
            text_content,
            is_pinned: false,
            pin_id: None,
            quick_stats,
            tags,
        })
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    async fn build_rendered_card_from_cache(
        &self,
        entity_type: CardEntityType,
        entity_id: &str,
        disclosure_level: DisclosureLevel,
        cached: &CardCacheRecord,
        session_id: Option<&str>,
    ) -> Result<RenderedCard, QuickReferenceError> {
        // We still need to fetch the entity for title/subtitle and quick_stats
        let (title, subtitle, quick_stats, tags) = match entity_type {
            CardEntityType::Npc => {
                let npc = self.database.get_npc(entity_id).await?
                    .ok_or_else(|| QuickReferenceError::EntityNotFound {
                        entity_type: "npc".to_string(),
                        entity_id: entity_id.to_string(),
                    })?;
                // Generate quick_stats from entity data (matching non-cached path)
                let stats = if disclosure_level != DisclosureLevel::Minimal {
                    vec![QuickStat {
                        label: "Role".to_string(),
                        value: npc.role.clone(),
                        icon: Some("user".to_string()),
                    }]
                } else {
                    vec![]
                };
                (npc.name, Some(npc.role), stats, vec!["npc".to_string()])
            }
            CardEntityType::Location => {
                let loc = self.database.get_location(entity_id).await?
                    .ok_or_else(|| QuickReferenceError::EntityNotFound {
                        entity_type: "location".to_string(),
                        entity_id: entity_id.to_string(),
                    })?;
                // Generate quick_stats from entity data (matching non-cached path)
                let stats = if disclosure_level != DisclosureLevel::Minimal {
                    vec![QuickStat {
                        label: "Type".to_string(),
                        value: loc.location_type.clone(),
                        icon: Some("map-pin".to_string()),
                    }]
                } else {
                    vec![]
                };
                (loc.name, Some(loc.location_type), stats, vec!["location".to_string()])
            }
            _ => (entity_id.to_string(), None, vec![], vec![]),
        };

        let mut is_pinned = false;
        let mut pin_id = None;
        if let Some(sid) = session_id {
            is_pinned = self.database
                .is_entity_pinned(sid, entity_type.as_str(), entity_id)
                .await?;
            if is_pinned {
                if let Ok(cards) = self.database.get_pinned_cards(sid).await {
                    pin_id = cards.iter()
                        .find(|c| c.entity_type == entity_type.as_str() && c.entity_id == entity_id)
                        .map(|c| c.id.clone());
                }
            }
        }

        Ok(RenderedCard {
            entity_type,
            entity_id: entity_id.to_string(),
            disclosure_level,
            title,
            subtitle,
            html_content: cached.html_content.clone(),
            text_content: self.html_to_text(&cached.html_content),
            is_pinned,
            pin_id,
            quick_stats,
            tags,
        })
    }

    fn extract_summary(&self, text: &str) -> String {
        // Extract first 100 characters as summary (character-based, not byte-based)
        let trimmed = text.trim();
        let char_count = trimmed.chars().count();
        if char_count <= 100 {
            trimmed.to_string()
        } else {
            // Take first 97 characters and add ellipsis
            let truncated: String = trimmed.chars().take(97).collect();
            format!("{}...", truncated)
        }
    }

    fn html_to_text(&self, html: &str) -> String {
        // Simple HTML to text conversion (strip tags and decode basic entities)
        let mut text = String::new();
        let mut in_tag = false;
        let mut i = 0;
        let chars: Vec<char> = html.chars().collect();

        while i < chars.len() {
            match chars[i] {
                '<' => in_tag = true,
                '>' => in_tag = false,
                '&' if !in_tag => {
                    // Try to decode entity
                    if let Some((decoded, end_idx)) = self.decode_html_entity(&chars[i..]) {
                        text.push(decoded);
                        i += end_idx;
                    } else {
                        text.push('&');
                    }
                }
                _ if !in_tag => text.push(chars[i]),
                _ => {}
            }
            i += 1;
        }
        text.trim().to_string()
    }

    /// Decode basic HTML entities
    fn decode_html_entity(&self, slice: &[char]) -> Option<(char, usize)> {
        if slice.len() < 3 { return None; }

        let mut end = 0;
        for j in 1..std::cmp::min(10, slice.len()) {
            if slice[j] == ';' {
                end = j;
                break;
            }
        }

        if end == 0 { return None; }

        let entity: String = slice[1..end].iter().collect();
        match entity.as_str() {
            "amp" => Some(('&', end)),
            "lt" => Some(('<', end)),
            "gt" => Some(('>', end)),
            "quot" => Some(('"', end)),
            "apos" | "#39" => Some(('\'', end)),
            "nbsp" => Some((' ', end)),
            _ if entity.starts_with("#x") => {
                u32::from_str_radix(&entity[2..], 16).ok()
                    .and_then(std::char::from_u32)
                    .map(|c| (c, end))
            }
            _ if entity.starts_with('#') => {
                entity[1..].parse::<u32>().ok()
                    .and_then(std::char::from_u32)
                    .map(|c| (c, end))
            }
            _ => None,
        }
    }
}

// ============================================================================
// Card Renderers
// ============================================================================

/// NPC card renderer
pub struct NpcCardRenderer;

impl NpcCardRenderer {
    pub fn render(
        npc: &NpcRecord,
        level: DisclosureLevel,
    ) -> (String, String, Vec<QuickStat>, Vec<String>) {
        let mut html = String::new();
        let mut text = String::new();
        let mut stats = Vec::new();
        let mut tags = vec!["npc".to_string()];

        // Escape user-controlled fields for HTML safety
        let escaped_role = HtmlExporter::escape_html(&npc.role);
        let escaped_name = HtmlExporter::escape_html(&npc.name);

        // Add role tag (sanitize for tag use - only alphanumeric and hyphens)
        tags.push(npc.role.to_lowercase().replace(' ', "-"));

        match level {
            DisclosureLevel::Minimal => {
                html.push_str(&format!(
                    r#"<div class="card-npc minimal">
                        <div class="card-header">
                            <span class="card-role">{}</span>
                        </div>
                    </div>"#,
                    escaped_role
                ));
                text.push_str(&format!("{} - {}", escaped_name, escaped_role));
            }
            DisclosureLevel::Summary => {
                // Parse personality for summary
                let personality: serde_json::Value = serde_json::from_str(&npc.personality_json)
                    .unwrap_or_default();
                let traits = personality.get("traits")
                    .and_then(|t| t.as_array())
                    .map(|a| a.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", "))
                    .unwrap_or_default();
                let escaped_traits = HtmlExporter::escape_html(&traits);

                html.push_str(&format!(
                    r#"<div class="card-npc summary">
                        <div class="card-header">
                            <span class="card-role">{}</span>
                        </div>
                        <div class="card-body">
                            <p class="card-traits"><strong>Traits:</strong> {}</p>
                            {}
                        </div>
                    </div>"#,
                    escaped_role,
                    if escaped_traits.is_empty() { "Unknown".to_string() } else { escaped_traits.clone() },
                    npc.notes.as_ref().map(|n| format!(
                        "<p class=\"card-notes\">{}</p>",
                        HtmlExporter::escape_html(n)
                    )).unwrap_or_default()
                ));

                text.push_str(&format!("{} - {} - {}", escaped_name, escaped_role, escaped_traits));
                if let Some(notes) = &npc.notes {
                    text.push_str(&format!(" - {}", HtmlExporter::escape_html(notes)));
                }

                stats.push(QuickStat {
                    label: "Role".to_string(),
                    value: npc.role.clone(),
                    icon: Some("user".to_string()),
                });
            }
            DisclosureLevel::Complete => {
                // Full NPC details
                let personality: serde_json::Value = serde_json::from_str(&npc.personality_json)
                    .unwrap_or_default();
                let escaped_personality = HtmlExporter::escape_html(
                    &serde_json::to_string_pretty(&personality).unwrap_or_default()
                );

                html.push_str(&format!(
                    r#"<div class="card-npc complete">
                        <div class="card-header">
                            <span class="card-role">{}</span>
                        </div>
                        <div class="card-body">
                            <div class="card-section">
                                <h4>Personality</h4>
                                <pre class="card-json">{}</pre>
                            </div>
                            {}
                            {}
                            {}
                        </div>
                    </div>"#,
                    escaped_role,
                    escaped_personality,
                    npc.notes.as_ref().map(|n| format!(
                        "<div class=\"card-section\"><h4>Notes</h4><p>{}</p></div>",
                        HtmlExporter::escape_html(n)
                    )).unwrap_or_default(),
                    npc.quest_hooks.as_ref().map(|h| format!(
                        "<div class=\"card-section\"><h4>Quest Hooks</h4><pre>{}</pre></div>",
                        HtmlExporter::escape_html(h)
                    )).unwrap_or_default(),
                    npc.stats_json.as_ref().map(|s| format!(
                        "<div class=\"card-section\"><h4>Stats</h4><pre>{}</pre></div>",
                        HtmlExporter::escape_html(s)
                    )).unwrap_or_default(),
                ));

                text.push_str(&format!("{} - {} - Full details", escaped_name, escaped_role));

                stats.push(QuickStat {
                    label: "Role".to_string(),
                    value: npc.role.clone(),
                    icon: Some("user".to_string()),
                });
            }
        }

        (html, text, stats, tags)
    }
}

/// Location card renderer
pub struct LocationCardRenderer;

impl LocationCardRenderer {
    pub fn render(
        location: &LocationRecord,
        level: DisclosureLevel,
    ) -> (String, String, Vec<QuickStat>, Vec<String>) {
        let mut html = String::new();
        let mut text = String::new();
        let mut stats = Vec::new();
        let mut tags = vec!["location".to_string()];

        // Escape user-controlled fields for HTML safety
        let escaped_type = HtmlExporter::escape_html(&location.location_type);
        let escaped_name = HtmlExporter::escape_html(&location.name);

        tags.push(location.location_type.to_lowercase().replace(' ', "-"));

        match level {
            DisclosureLevel::Minimal => {
                html.push_str(&format!(
                    r#"<div class="card-location minimal">
                        <div class="card-header">
                            <span class="card-type">{}</span>
                        </div>
                    </div>"#,
                    escaped_type
                ));
                text.push_str(&format!("{} - {}", escaped_name, escaped_type));
            }
            DisclosureLevel::Summary => {
                html.push_str(&format!(
                    r#"<div class="card-location summary">
                        <div class="card-header">
                            <span class="card-type">{}</span>
                        </div>
                        <div class="card-body">
                            {}
                        </div>
                    </div>"#,
                    escaped_type,
                    location.description.as_ref()
                        .map(|d| format!(
                            "<p class=\"card-description\">{}</p>",
                            HtmlExporter::escape_html(d)
                        ))
                        .unwrap_or_default()
                ));

                text.push_str(&format!("{} - {}", escaped_name, escaped_type));
                if let Some(desc) = &location.description {
                    text.push_str(&format!(" - {}", HtmlExporter::escape_html(desc)));
                }

                stats.push(QuickStat {
                    label: "Type".to_string(),
                    value: location.location_type.clone(),
                    icon: Some("map-pin".to_string()),
                });
            }
            DisclosureLevel::Complete => {
                let features: Vec<String> = serde_json::from_str(&location.features_json)
                    .unwrap_or_default();
                let secrets: Vec<String> = serde_json::from_str(&location.secrets_json)
                    .unwrap_or_default();

                html.push_str(&format!(
                    r#"<div class="card-location complete">
                        <div class="card-header">
                            <span class="card-type">{}</span>
                        </div>
                        <div class="card-body">
                            {}
                            {}
                            {}
                        </div>
                    </div>"#,
                    escaped_type,
                    location.description.as_ref()
                        .map(|d| format!(
                            "<div class=\"card-section\"><h4>Description</h4><p>{}</p></div>",
                            HtmlExporter::escape_html(d)
                        ))
                        .unwrap_or_default(),
                    if !features.is_empty() {
                        format!(
                            "<div class=\"card-section\"><h4>Features</h4><ul>{}</ul></div>",
                            features.iter()
                                .map(|f| format!("<li>{}</li>", HtmlExporter::escape_html(f)))
                                .collect::<String>()
                        )
                    } else { String::new() },
                    if !secrets.is_empty() {
                        format!(
                            "<div class=\"card-section card-secrets\"><h4>Secrets</h4><ul>{}</ul></div>",
                            secrets.iter()
                                .map(|s| format!("<li>{}</li>", HtmlExporter::escape_html(s)))
                                .collect::<String>()
                        )
                    } else { String::new() },
                ));

                text.push_str(&format!("{} - {} - Full details", escaped_name, escaped_type));

                stats.push(QuickStat {
                    label: "Type".to_string(),
                    value: location.location_type.clone(),
                    icon: Some("map-pin".to_string()),
                });
            }
        }

        (html, text, stats, tags)
    }
}

/// Character card renderer
pub struct CharacterCardRenderer;

impl CharacterCardRenderer {
    pub fn render(
        character: &crate::database::CharacterRecord,
        level: DisclosureLevel,
    ) -> (String, String, Vec<QuickStat>, Vec<String>) {
        let mut html = String::new();
        let mut text = String::new();
        let mut stats = Vec::new();
        let tags = vec!["character".to_string(), character.character_type.clone()];

        // Escape user-controlled fields for HTML safety
        let escaped_name = HtmlExporter::escape_html(&character.name);
        let escaped_system = HtmlExporter::escape_html(&character.system);

        match level {
            DisclosureLevel::Minimal => {
                html.push_str(&format!(
                    r#"<div class="card-character minimal">
                        <div class="card-header">
                            {}
                        </div>
                    </div>"#,
                    character.level.map(|l| format!("<span class=\"card-level\">Level {}</span>", l))
                        .unwrap_or_default()
                ));
                text.push_str(&escaped_name);
                if let Some(lvl) = character.level {
                    text.push_str(&format!(" - Level {}", lvl));
                }
            }
            DisclosureLevel::Summary | DisclosureLevel::Complete => {
                // Parse character data for more info
                let data: serde_json::Value = serde_json::from_str(&character.data_json)
                    .unwrap_or_default();
                let escaped_data = HtmlExporter::escape_html(
                    &serde_json::to_string_pretty(&data).unwrap_or_default()
                );

                html.push_str(&format!(
                    r#"<div class="card-character {}">
                        <div class="card-header">
                            {}
                            <span class="card-system">{}</span>
                        </div>
                        <div class="card-body">
                            <pre class="card-json">{}</pre>
                        </div>
                    </div>"#,
                    if level == DisclosureLevel::Summary { "summary" } else { "complete" },
                    character.level.map(|l| format!("<span class=\"card-level\">Level {}</span>", l))
                        .unwrap_or_default(),
                    escaped_system,
                    escaped_data
                ));

                text.push_str(&format!("{} - {}", escaped_name, escaped_system));

                if let Some(lvl) = character.level {
                    stats.push(QuickStat {
                        label: "Level".to_string(),
                        value: lvl.to_string(),
                        icon: Some("trending-up".to_string()),
                    });
                }
                stats.push(QuickStat {
                    label: "System".to_string(),
                    value: character.system.clone(),
                    icon: Some("book".to_string()),
                });
            }
        }

        (html, text, stats, tags)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_reference_error_display() {
        let err = QuickReferenceError::EntityNotFound {
            entity_type: "npc".to_string(),
            entity_id: "npc-123".to_string(),
        };
        assert!(err.to_string().contains("npc"));
        assert!(err.to_string().contains("npc-123"));
    }

    #[test]
    fn test_quick_reference_error_max_cards() {
        let err = QuickReferenceError::MaxPinnedCardsReached { max: 6 };
        assert!(err.to_string().contains("6"));
    }

    #[test]
    fn test_rendered_card_structure() {
        let card = RenderedCard {
            entity_type: CardEntityType::Npc,
            entity_id: "npc-1".to_string(),
            disclosure_level: DisclosureLevel::Summary,
            title: "Test NPC".to_string(),
            subtitle: Some("Merchant".to_string()),
            html_content: "<div>Test</div>".to_string(),
            text_content: "Test".to_string(),
            is_pinned: false,
            pin_id: None,
            quick_stats: vec![],
            tags: vec!["npc".to_string()],
        };

        assert_eq!(card.entity_type, CardEntityType::Npc);
        assert_eq!(card.title, "Test NPC");
        assert!(!card.is_pinned);
    }

    #[test]
    fn test_hover_preview_structure() {
        let preview = HoverPreview {
            entity_type: CardEntityType::Location,
            entity_id: "loc-1".to_string(),
            title: "Test Location".to_string(),
            subtitle: Some("Tavern".to_string()),
            summary: "A cozy tavern...".to_string(),
            quick_stats: vec![
                QuickStat {
                    label: "Type".to_string(),
                    value: "Tavern".to_string(),
                    icon: Some("map-pin".to_string()),
                },
            ],
        };

        assert_eq!(preview.entity_type, CardEntityType::Location);
        assert_eq!(preview.quick_stats.len(), 1);
    }

    #[test]
    fn test_card_tray_structure() {
        let tray = CardTray {
            session_id: "session-1".to_string(),
            cards: vec![],
            max_cards: MAX_PINNED_CARDS,
            slots_remaining: MAX_PINNED_CARDS,
        };

        assert_eq!(tray.max_cards, 6);
        assert_eq!(tray.slots_remaining, 6);
    }

    #[test]
    fn test_npc_card_renderer_minimal() {
        let npc = NpcRecord {
            id: "npc-1".to_string(),
            campaign_id: Some("camp-1".to_string()),
            name: "Bob".to_string(),
            role: "Merchant".to_string(),
            personality_id: None,
            personality_json: r#"{"traits": ["friendly", "honest"]}"#.to_string(),
            data_json: None,
            stats_json: None,
            notes: None,
            location_id: None,
            voice_profile_id: None,
            quest_hooks: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let (html, text, _stats, tags) = NpcCardRenderer::render(&npc, DisclosureLevel::Minimal);

        assert!(html.contains("Merchant"));
        assert!(text.contains("Bob"));
        assert!(text.contains("Merchant"));
        assert!(tags.contains(&"npc".to_string()));
    }

    #[test]
    fn test_npc_card_renderer_summary() {
        let npc = NpcRecord {
            id: "npc-1".to_string(),
            campaign_id: Some("camp-1".to_string()),
            name: "Alice".to_string(),
            role: "Innkeeper".to_string(),
            personality_id: None,
            personality_json: r#"{"traits": ["welcoming", "curious"]}"#.to_string(),
            data_json: None,
            stats_json: None,
            notes: Some("Knows local gossip".to_string()),
            location_id: None,
            voice_profile_id: None,
            quest_hooks: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let (html, _text, stats, _tags) = NpcCardRenderer::render(&npc, DisclosureLevel::Summary);

        assert!(html.contains("welcoming"));
        assert!(html.contains("curious"));
        assert!(html.contains("Knows local gossip"));
        assert!(!stats.is_empty());
    }

    #[test]
    fn test_location_card_renderer() {
        let location = LocationRecord {
            id: "loc-1".to_string(),
            campaign_id: "camp-1".to_string(),
            name: "The Dancing Dragon".to_string(),
            location_type: "Tavern".to_string(),
            description: Some("A lively tavern in the town square.".to_string()),
            parent_id: None,
            connections_json: "[]".to_string(),
            npcs_present_json: "[]".to_string(),
            features_json: r#"["Large fireplace", "Stage for performers"]"#.to_string(),
            secrets_json: r#"["Hidden basement", "Secret door"]"#.to_string(),
            attributes_json: "{}".to_string(),
            tags_json: "[]".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        let (html, _, _, tags) = LocationCardRenderer::render(&location, DisclosureLevel::Complete);

        assert!(html.contains("Large fireplace"));
        assert!(html.contains("Secret door"));
        assert!(tags.contains(&"tavern".to_string()));
    }
}
