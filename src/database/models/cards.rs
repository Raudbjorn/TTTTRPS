//! Quick Reference Cards Models
//!
//! Database records for pinned cards, cheat sheet preferences, and card caching.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================================================
// Card Entity Type Enum
// ============================================================================

/// Entity type for quick reference cards
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardEntityType {
    Npc,
    Location,
    Item,
    PlotPoint,
    Scene,
    Character,
}

impl CardEntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CardEntityType::Npc => "npc",
            CardEntityType::Location => "location",
            CardEntityType::Item => "item",
            CardEntityType::PlotPoint => "plot_point",
            CardEntityType::Scene => "scene",
            CardEntityType::Character => "character",
        }
    }
}

impl std::fmt::Display for CardEntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for CardEntityType {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "npc" => Ok(CardEntityType::Npc),
            "location" => Ok(CardEntityType::Location),
            "item" => Ok(CardEntityType::Item),
            "plot_point" => Ok(CardEntityType::PlotPoint),
            "scene" => Ok(CardEntityType::Scene),
            "character" => Ok(CardEntityType::Character),
            _ => Err(format!("Unknown card entity type: {}", s)),
        }
    }
}

// ============================================================================
// Disclosure Level Enum
// ============================================================================

/// Disclosure level for progressive detail display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisclosureLevel {
    /// Minimal - name and type only
    Minimal,
    /// Summary - key details for quick reference
    Summary,
    /// Complete - full entity details
    Complete,
}

impl DisclosureLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            DisclosureLevel::Minimal => "minimal",
            DisclosureLevel::Summary => "summary",
            DisclosureLevel::Complete => "complete",
        }
    }
}

impl Default for DisclosureLevel {
    fn default() -> Self {
        DisclosureLevel::Summary
    }
}

impl std::fmt::Display for DisclosureLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for DisclosureLevel {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "minimal" => Ok(DisclosureLevel::Minimal),
            "summary" => Ok(DisclosureLevel::Summary),
            "complete" => Ok(DisclosureLevel::Complete),
            _ => Err(format!("Unknown disclosure level: {}", s)),
        }
    }
}

// ============================================================================
// Include Status Enum
// ============================================================================

/// Include status for cheat sheet preferences
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncludeStatus {
    /// Always include in cheat sheet
    Always,
    /// Automatically determined based on context
    Auto,
    /// Never include in cheat sheet
    Never,
}

impl IncludeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            IncludeStatus::Always => "always",
            IncludeStatus::Auto => "auto",
            IncludeStatus::Never => "never",
        }
    }
}

impl Default for IncludeStatus {
    fn default() -> Self {
        IncludeStatus::Auto
    }
}

impl std::fmt::Display for IncludeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for IncludeStatus {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "always" => Ok(IncludeStatus::Always),
            "auto" => Ok(IncludeStatus::Auto),
            "never" => Ok(IncludeStatus::Never),
            _ => Err(format!("Unknown include status: {}", s)),
        }
    }
}

// ============================================================================
// Preference Type Enum
// ============================================================================

/// Preference type for cheat sheet customization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreferenceType {
    /// Preference for a specific entity
    Entity,
    /// Preference for an entity type category
    Category,
    /// Global default preference
    Global,
}

impl PreferenceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PreferenceType::Entity => "entity",
            PreferenceType::Category => "category",
            PreferenceType::Global => "global",
        }
    }
}

impl std::fmt::Display for PreferenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for PreferenceType {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "entity" => Ok(PreferenceType::Entity),
            "category" => Ok(PreferenceType::Category),
            "global" => Ok(PreferenceType::Global),
            _ => Err(format!("Unknown preference type: {}", s)),
        }
    }
}

// ============================================================================
// Pinned Card Record
// ============================================================================

/// Pinned card database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PinnedCardRecord {
    pub id: String,
    pub session_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub display_order: i32,
    pub disclosure_level: String,
    pub pinned_at: String,
}

impl PinnedCardRecord {
    pub fn new(
        session_id: String,
        entity_type: CardEntityType,
        entity_id: String,
        display_order: i32,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id,
            entity_type: entity_type.to_string(),
            entity_id,
            display_order,
            disclosure_level: DisclosureLevel::default().to_string(),
            pinned_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Get entity type as typed enum
    pub fn entity_type_enum(&self) -> Result<CardEntityType, String> {
        CardEntityType::try_from(self.entity_type.as_str())
    }

    /// Get disclosure level as typed enum
    pub fn disclosure_level_enum(&self) -> Result<DisclosureLevel, String> {
        DisclosureLevel::try_from(self.disclosure_level.as_str())
    }

    /// Set disclosure level
    pub fn with_disclosure_level(mut self, level: DisclosureLevel) -> Self {
        self.disclosure_level = level.to_string();
        self
    }
}

// ============================================================================
// Cheat Sheet Preference Record
// ============================================================================

/// Cheat sheet preference database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CheatSheetPreferenceRecord {
    pub id: String,
    pub campaign_id: String,
    pub session_id: Option<String>,
    pub preference_type: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub include_status: String,
    pub default_disclosure_level: String,
    pub priority: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl CheatSheetPreferenceRecord {
    pub fn new(campaign_id: String, preference_type: PreferenceType) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            campaign_id,
            session_id: None,
            preference_type: preference_type.to_string(),
            entity_type: None,
            entity_id: None,
            include_status: IncludeStatus::default().to_string(),
            default_disclosure_level: DisclosureLevel::default().to_string(),
            priority: 50,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Get preference type as typed enum
    pub fn preference_type_enum(&self) -> Result<PreferenceType, String> {
        PreferenceType::try_from(self.preference_type.as_str())
    }

    /// Get include status as typed enum
    pub fn include_status_enum(&self) -> Result<IncludeStatus, String> {
        IncludeStatus::try_from(self.include_status.as_str())
    }

    /// Get disclosure level as typed enum
    pub fn disclosure_level_enum(&self) -> Result<DisclosureLevel, String> {
        DisclosureLevel::try_from(self.default_disclosure_level.as_str())
    }

    /// Set session scope
    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Set entity type for category preference
    pub fn with_entity_type(mut self, entity_type: CardEntityType) -> Self {
        self.entity_type = Some(entity_type.to_string());
        self
    }

    /// Set specific entity for entity preference
    pub fn with_entity(mut self, entity_type: CardEntityType, entity_id: String) -> Self {
        self.entity_type = Some(entity_type.to_string());
        self.entity_id = Some(entity_id);
        self
    }

    /// Set include status
    pub fn with_include_status(mut self, status: IncludeStatus) -> Self {
        self.include_status = status.to_string();
        self
    }

    /// Set default disclosure level
    pub fn with_disclosure_level(mut self, level: DisclosureLevel) -> Self {
        self.default_disclosure_level = level.to_string();
        self
    }

    /// Set priority (0-100, higher = more important)
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority.clamp(0, 100);
        self
    }
}

// ============================================================================
// Card Cache Record
// ============================================================================

/// Card cache database record for pre-rendered HTML
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CardCacheRecord {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub disclosure_level: String,
    pub html_content: String,
    pub generated_at: String,
    pub expires_at: String,
}

impl CardCacheRecord {
    pub fn new(
        entity_type: CardEntityType,
        entity_id: String,
        disclosure_level: DisclosureLevel,
        html_content: String,
        ttl_hours: i64,
    ) -> Self {
        let now = chrono::Utc::now();
        let expires = now + chrono::Duration::hours(ttl_hours);
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            entity_type: entity_type.to_string(),
            entity_id,
            disclosure_level: disclosure_level.to_string(),
            html_content,
            generated_at: now.to_rfc3339(),
            expires_at: expires.to_rfc3339(),
        }
    }

    /// Check if the cache entry has expired
    pub fn is_expired(&self) -> bool {
        chrono::DateTime::parse_from_rfc3339(&self.expires_at)
            .map(|exp| exp < chrono::Utc::now())
            .unwrap_or(true)
    }

    /// Get entity type as typed enum
    pub fn entity_type_enum(&self) -> Result<CardEntityType, String> {
        CardEntityType::try_from(self.entity_type.as_str())
    }

    /// Get disclosure level as typed enum
    pub fn disclosure_level_enum(&self) -> Result<DisclosureLevel, String> {
        DisclosureLevel::try_from(self.disclosure_level.as_str())
    }
}
