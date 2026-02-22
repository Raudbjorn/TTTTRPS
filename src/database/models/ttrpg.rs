//! TTRPG Game Mechanics Models
//!
//! Database records for NPCs, combat states, TTRPG documents, stat blocks,
//! random tables, and roll history.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================================================
// NPC Record
// ============================================================================

/// NPC database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NpcRecord {
    pub id: String,
    pub campaign_id: Option<String>,
    pub name: String,
    pub role: String,
    pub personality_id: Option<String>,
    pub personality_json: String,
    pub data_json: Option<String>,
    pub stats_json: Option<String>,
    pub notes: Option<String>,
    pub location_id: Option<String>,
    pub voice_profile_id: Option<String>,
    pub quest_hooks: Option<String>,  // JSON array
    pub created_at: String,
}

impl NpcRecord {
    pub fn new(id: String, name: String, role: String) -> Self {
        Self {
            id,
            campaign_id: None,
            name,
            role,
            personality_id: None,
            personality_json: "{}".to_string(),
            data_json: None,
            stats_json: None,
            notes: None,
            location_id: None,
            voice_profile_id: None,
            quest_hooks: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Link to a campaign
    pub fn with_campaign(mut self, campaign_id: String) -> Self {
        self.campaign_id = Some(campaign_id);
        self
    }

    /// Set role
    pub fn with_role(mut self, role: String) -> Self {
        self.role = role;
        self
    }

    /// Set location
    pub fn with_location(mut self, location_id: String) -> Self {
        self.location_id = Some(location_id);
        self
    }

    /// Set voice profile
    pub fn with_voice(mut self, voice_profile_id: String) -> Self {
        self.voice_profile_id = Some(voice_profile_id);
        self
    }

    /// Parse quest hooks from JSON
    pub fn quest_hooks_vec(&self) -> Vec<String> {
        self.quest_hooks
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }
}

// ============================================================================
// Combat State Record
// ============================================================================

/// Combat state database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CombatStateRecord {
    pub id: String,
    pub session_id: String,
    pub name: Option<String>,
    pub round: i32,
    pub current_turn: i32,
    pub is_active: bool,
    pub combatants: String,    // JSON array of combatant data
    pub conditions: String,    // JSON array of active conditions
    pub environment: Option<String>, // JSON for environmental effects
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub ended_at: Option<String>,
}

impl CombatStateRecord {
    pub fn new(id: String, session_id: String, combatants: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            session_id,
            name: None,
            round: 1,
            current_turn: 0,
            is_active: true,
            combatants,
            conditions: "[]".to_string(),
            environment: None,
            notes: None,
            created_at: now.clone(),
            updated_at: now,
            ended_at: None,
        }
    }

    /// Set combat name
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// End the combat
    pub fn end(&mut self) {
        self.is_active = false;
        self.ended_at = Some(chrono::Utc::now().to_rfc3339());
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}

// ============================================================================
// TTRPG Document Record
// ============================================================================

/// TTRPG document element record (monsters, spells, items, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TTRPGDocumentRecord {
    pub id: String,
    pub source_document_id: String,
    pub name: String,
    pub element_type: String,  // "monster", "spell", "item", "class_feature", "feat", etc.
    pub game_system: String,   // "dnd5e", "pathfinder2e", etc.
    pub content: String,       // Full text content
    pub attributes_json: Option<String>,  // JSON for type-specific attributes
    pub challenge_rating: Option<f64>,
    pub level: Option<i32>,
    pub page_number: Option<i32>,
    pub confidence: f64,       // Extraction confidence (0.0-1.0)
    pub meilisearch_id: Option<String>,  // Reference to search index
    pub created_at: String,
    pub updated_at: String,
}

impl TTRPGDocumentRecord {
    pub fn new(
        id: String,
        source_document_id: String,
        name: String,
        element_type: String,
        game_system: String,
        content: String,
        confidence: f64,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            source_document_id,
            name,
            element_type,
            game_system,
            content,
            attributes_json: None,
            challenge_rating: None,
            level: None,
            page_number: None,
            confidence,
            meilisearch_id: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Set challenge rating
    pub fn with_cr(mut self, cr: f64) -> Self {
        self.challenge_rating = Some(cr);
        self
    }

    /// Set level
    pub fn with_level(mut self, level: i32) -> Self {
        self.level = Some(level);
        self
    }

    /// Set page number
    pub fn with_page(mut self, page: i32) -> Self {
        self.page_number = Some(page);
        self
    }

    /// Set attributes JSON
    pub fn with_attributes(mut self, attributes: serde_json::Value) -> Self {
        self.attributes_json = Some(serde_json::to_string(&attributes).unwrap_or_default());
        self
    }

    /// Set Meilisearch ID
    pub fn with_meilisearch_id(mut self, id: String) -> Self {
        self.meilisearch_id = Some(id);
        self
    }

    /// Parse attributes from JSON
    pub fn attributes(&self) -> Option<serde_json::Value> {
        self.attributes_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
    }
}

/// TTRPG document attribute record (for normalized attribute storage)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TTRPGDocumentAttribute {
    pub id: i32,
    pub document_id: String,
    pub attribute_type: String,
    pub attribute_value: String,
}

/// TTRPG ingestion job record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TTRPGIngestionJob {
    pub id: String,
    pub document_id: String,
    pub status: String,  // "pending", "processing", "completed", "failed"
    pub total_pages: i32,
    pub processed_pages: i32,
    pub elements_found: i32,
    pub errors_json: Option<String>,  // JSON array of error messages
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
}

impl TTRPGIngestionJob {
    pub fn new(id: String, document_id: String, total_pages: i32) -> Self {
        Self {
            id,
            document_id,
            status: "pending".to_string(),
            total_pages,
            processed_pages: 0,
            elements_found: 0,
            errors_json: None,
            started_at: None,
            completed_at: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Mark job as started
    pub fn start(&mut self) {
        self.status = "processing".to_string();
        self.started_at = Some(chrono::Utc::now().to_rfc3339());
    }

    /// Update progress
    pub fn update_progress(&mut self, processed_pages: i32, elements_found: i32) {
        self.processed_pages = processed_pages;
        self.elements_found = elements_found;
    }

    /// Mark job as completed
    pub fn complete(&mut self) {
        self.status = "completed".to_string();
        self.completed_at = Some(chrono::Utc::now().to_rfc3339());
    }

    /// Mark job as failed
    pub fn fail(&mut self, errors: &[String]) {
        self.status = "failed".to_string();
        self.errors_json = Some(serde_json::to_string(errors).unwrap_or_default());
        self.completed_at = Some(chrono::Utc::now().to_rfc3339());
    }

    /// Get progress percentage
    pub fn progress_percent(&self) -> f64 {
        if self.total_pages == 0 {
            0.0
        } else {
            (self.processed_pages as f64 / self.total_pages as f64) * 100.0
        }
    }

    /// Parse errors from JSON
    pub fn errors(&self) -> Vec<String> {
        self.errors_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }
}

// ============================================================================
// Random Table Types
// ============================================================================

/// Random table type for categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RandomTableType {
    /// Standard roll with equal probability ranges
    Standard,
    /// Weighted probabilities
    Weighted,
    /// d66-style table (read as tens/ones)
    D66,
    /// Nested tables that chain to other tables
    Nested,
    /// Oracle-style yes/no with degrees
    Oracle,
}

impl RandomTableType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RandomTableType::Standard => "standard",
            RandomTableType::Weighted => "weighted",
            RandomTableType::D66 => "d66",
            RandomTableType::Nested => "nested",
            RandomTableType::Oracle => "oracle",
        }
    }
}

impl std::fmt::Display for RandomTableType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for RandomTableType {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "standard" => Ok(RandomTableType::Standard),
            "weighted" => Ok(RandomTableType::Weighted),
            "d66" => Ok(RandomTableType::D66),
            "nested" => Ok(RandomTableType::Nested),
            "oracle" => Ok(RandomTableType::Oracle),
            _ => Err(format!("Unknown random table type: {}", s)),
        }
    }
}

impl Default for RandomTableType {
    fn default() -> Self {
        RandomTableType::Standard
    }
}

/// Result type for table entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableResultType {
    /// Plain text result
    Text,
    /// Roll on another table
    NestedRoll,
    /// Multiple rolls combined
    MultiRoll,
    /// Conditional based on context
    Conditional,
}

impl TableResultType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TableResultType::Text => "text",
            TableResultType::NestedRoll => "nested_roll",
            TableResultType::MultiRoll => "multi_roll",
            TableResultType::Conditional => "conditional",
        }
    }
}

impl std::fmt::Display for TableResultType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for TableResultType {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "text" => Ok(TableResultType::Text),
            "nested_roll" => Ok(TableResultType::NestedRoll),
            "multi_roll" => Ok(TableResultType::MultiRoll),
            "conditional" => Ok(TableResultType::Conditional),
            _ => Err(format!("Unknown table result type: {}", s)),
        }
    }
}

impl Default for TableResultType {
    fn default() -> Self {
        TableResultType::Text
    }
}

// ============================================================================
// Random Table Records
// ============================================================================

/// Random table database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RandomTableRecord {
    pub id: String,
    pub campaign_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub table_type: String,
    pub dice_notation: String,
    pub category: Option<String>,
    pub tags: String,  // JSON array
    pub is_system: i32,
    pub is_nested: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl RandomTableRecord {
    pub fn new(name: String, dice_notation: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            campaign_id: None,
            name,
            description: None,
            table_type: RandomTableType::Standard.to_string(),
            dice_notation,
            category: None,
            tags: "[]".to_string(),
            is_system: 0,
            is_nested: 0,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn table_type_enum(&self) -> Result<RandomTableType, String> {
        RandomTableType::try_from(self.table_type.as_str())
    }

    pub fn is_system_table(&self) -> bool {
        self.is_system != 0
    }

    pub fn is_nested_table(&self) -> bool {
        self.is_nested != 0
    }

    pub fn with_campaign(mut self, campaign_id: String) -> Self {
        self.campaign_id = Some(campaign_id);
        self
    }

    pub fn with_type(mut self, table_type: RandomTableType) -> Self {
        self.table_type = table_type.to_string();
        self
    }

    pub fn with_category(mut self, category: String) -> Self {
        self.category = Some(category);
        self
    }

    pub fn with_tags(mut self, tags: &[String]) -> Self {
        self.tags = serde_json::to_string(tags).unwrap_or_default();
        self
    }

    pub fn as_system(mut self) -> Self {
        self.is_system = 1;
        self
    }

    pub fn as_nested(mut self) -> Self {
        self.is_nested = 1;
        self
    }

    pub fn tags_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.tags).unwrap_or_default()
    }
}

/// Random table entry database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RandomTableEntryRecord {
    pub id: String,
    pub table_id: String,
    pub range_start: i32,
    pub range_end: i32,
    pub weight: f64,
    pub result_text: String,
    pub result_type: String,
    pub nested_table_id: Option<String>,
    pub metadata: Option<String>,  // JSON
    pub display_order: i32,
}

impl RandomTableEntryRecord {
    pub fn new(table_id: String, range_start: i32, range_end: i32, result_text: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            table_id,
            range_start,
            range_end,
            weight: 1.0,
            result_text,
            result_type: TableResultType::Text.to_string(),
            nested_table_id: None,
            metadata: None,
            display_order: 0,
        }
    }

    pub fn result_type_enum(&self) -> Result<TableResultType, String> {
        TableResultType::try_from(self.result_type.as_str())
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    pub fn with_nested_table(mut self, nested_table_id: String) -> Self {
        self.nested_table_id = Some(nested_table_id);
        self.result_type = TableResultType::NestedRoll.to_string();
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(serde_json::to_string(&metadata).unwrap_or_default());
        self
    }

    pub fn with_order(mut self, order: i32) -> Self {
        self.display_order = order;
        self
    }

    /// Check if a roll value falls within this entry's range
    pub fn matches_roll(&self, roll: i32) -> bool {
        roll >= self.range_start && roll <= self.range_end
    }
}

// ============================================================================
// Roll History Record
// ============================================================================

/// Roll history database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RollHistoryRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub campaign_id: Option<String>,
    pub table_id: Option<String>,
    pub dice_notation: String,
    pub raw_roll: i32,
    pub modifier: i32,
    pub final_result: i32,
    pub entry_id: Option<String>,
    pub result_text: Option<String>,
    pub context: Option<String>,
    pub rolled_at: String,
}

impl RollHistoryRecord {
    pub fn new(dice_notation: String, raw_roll: i32, modifier: i32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: None,
            campaign_id: None,
            table_id: None,
            dice_notation,
            raw_roll,
            modifier,
            final_result: raw_roll + modifier,
            entry_id: None,
            result_text: None,
            context: None,
            rolled_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn with_campaign(mut self, campaign_id: String) -> Self {
        self.campaign_id = Some(campaign_id);
        self
    }

    pub fn with_table_result(mut self, table_id: String, entry_id: String, result_text: String) -> Self {
        self.table_id = Some(table_id);
        self.entry_id = Some(entry_id);
        self.result_text = Some(result_text);
        self
    }

    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }
}

// ============================================================================
// Stat Block (Legacy, for backwards compatibility)
// ============================================================================

/// NPC/Monster stat block (simplified)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatBlock {
    pub armor_class: Option<i32>,
    pub hit_points: Option<i32>,
    pub speed: Option<String>,
    pub abilities: Option<AbilityScores>,
    pub skills: Option<Vec<String>>,
    pub damage_resistances: Option<Vec<String>>,
    pub damage_immunities: Option<Vec<String>>,
    pub condition_immunities: Option<Vec<String>>,
    pub senses: Option<Vec<String>>,
    pub languages: Option<Vec<String>>,
    pub challenge_rating: Option<f64>,
    pub actions: Option<Vec<StatBlockAction>>,
    pub legendary_actions: Option<Vec<StatBlockAction>>,
    pub special_abilities: Option<Vec<StatBlockAction>>,
}

/// Ability scores
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AbilityScores {
    pub strength: i32,
    pub dexterity: i32,
    pub constitution: i32,
    pub intelligence: i32,
    pub wisdom: i32,
    pub charisma: i32,
}

/// Stat block action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatBlockAction {
    pub name: String,
    pub description: String,
    pub attack_bonus: Option<i32>,
    pub damage: Option<String>,
    pub damage_type: Option<String>,
}

// ============================================================================
// Combat Record (Legacy alias)
// ============================================================================

/// Combat record (alias for CombatStateRecord for backwards compatibility)
pub type CombatRecord = CombatStateRecord;
