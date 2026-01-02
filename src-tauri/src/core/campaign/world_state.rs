//! World State Tracking Module (TASK-007)
//!
//! Provides in-game date tracking, world events timeline, location state changes,
//! NPC relationship tracking, and custom state fields.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;
use uuid::Uuid;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum WorldStateError {
    #[error("Campaign not found: {0}")]
    CampaignNotFound(String),

    #[error("Event not found: {0}")]
    EventNotFound(String),

    #[error("Location not found: {0}")]
    LocationNotFound(String),

    #[error("Invalid date format: {0}")]
    InvalidDateFormat(String),

    #[error("Custom field not found: {0}")]
    CustomFieldNotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub type Result<T> = std::result::Result<T, WorldStateError>;

// ============================================================================
// In-Game Date System
// ============================================================================

/// Represents an in-game date (fantasy calendar support)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InGameDate {
    /// Year (can be negative for ancient history)
    pub year: i32,
    /// Month (1-12 or custom calendar)
    pub month: u8,
    /// Day of month
    pub day: u8,
    /// Optional era name (e.g., "Age of Dragons", "Third Era")
    pub era: Option<String>,
    /// Calendar system name (e.g., "Harptos", "Gregorian", "Custom")
    pub calendar: String,
    /// Optional time of day
    pub time: Option<InGameTime>,
}

/// Time of day in-game
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InGameTime {
    pub hour: u8,
    pub minute: u8,
    /// Named time of day (Dawn, Midday, Dusk, Midnight, etc.)
    pub period: Option<String>,
}

impl Default for InGameDate {
    fn default() -> Self {
        Self {
            year: 1,
            month: 1,
            day: 1,
            era: None,
            calendar: "Standard".to_string(),
            time: None,
        }
    }
}

impl InGameDate {
    /// Create a new date
    pub fn new(year: i32, month: u8, day: u8) -> Self {
        Self {
            year,
            month,
            day,
            ..Default::default()
        }
    }

    /// Create a date with era
    pub fn with_era(year: i32, month: u8, day: u8, era: &str) -> Self {
        Self {
            year,
            month,
            day,
            era: Some(era.to_string()),
            ..Default::default()
        }
    }

    /// Format as display string
    pub fn display(&self) -> String {
        let base = format!("{}/{}/{}", self.day, self.month, self.year);
        match (&self.era, &self.time) {
            (Some(era), Some(time)) => format!("{} {} - {}:{:02}", base, era, time.hour, time.minute),
            (Some(era), None) => format!("{} {}", base, era),
            (None, Some(time)) => format!("{} - {}:{:02}", base, time.hour, time.minute),
            (None, None) => base,
        }
    }

    /// Advance by days
    pub fn advance_days(&mut self, days: i32) {
        // Simple implementation - doesn't handle month lengths
        let total_days = self.day as i32 + days;
        if total_days > 30 {
            self.month += (total_days / 30) as u8;
            self.day = (total_days % 30) as u8;
            if self.day == 0 {
                self.day = 30;
                self.month -= 1;
            }
        } else if total_days <= 0 {
            // Handle negative days
            self.month = self.month.saturating_sub(1);
            if self.month == 0 {
                self.month = 12;
                self.year -= 1;
            }
            self.day = (30 + total_days) as u8;
        } else {
            self.day = total_days as u8;
        }

        // Handle month overflow
        while self.month > 12 {
            self.month -= 12;
            self.year += 1;
        }
    }
}

// ============================================================================
// World Events
// ============================================================================

/// Type of world event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorldEventType {
    /// Battle, war, conflict
    Combat,
    /// Political change, alliance, treaty
    Political,
    /// Environmental (earthquake, storm, etc.)
    Natural,
    /// Economic (trade, market crash)
    Economic,
    /// Religious (prophecy, miracle)
    Religious,
    /// Magical (ritual, anomaly)
    Magical,
    /// Social (festival, plague)
    Social,
    /// Personal (NPC death, birth, marriage)
    Personal,
    /// Discovery (artifact, location)
    Discovery,
    /// Session-related (players did X)
    Session,
    /// Custom event type
    Custom(String),
}

impl Default for WorldEventType {
    fn default() -> Self {
        Self::Session
    }
}

/// Impact level of an event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventImpact {
    /// Affects only individuals
    Personal,
    /// Affects a neighborhood/small area
    Local,
    /// Affects a city or region
    Regional,
    /// Affects a nation
    National,
    /// Affects the world
    Global,
    /// Affects multiple planes/dimensions
    Cosmic,
}

impl Default for EventImpact {
    fn default() -> Self {
        Self::Local
    }
}

/// A world event on the timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEvent {
    /// Unique identifier
    pub id: String,
    /// Campaign this event belongs to
    pub campaign_id: String,
    /// When this happened in-game
    pub in_game_date: InGameDate,
    /// When this was recorded (real time)
    pub recorded_at: DateTime<Utc>,
    /// Event title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Type of event
    pub event_type: WorldEventType,
    /// Impact level
    pub impact: EventImpact,
    /// Locations involved
    pub location_ids: Vec<String>,
    /// NPCs involved
    pub npc_ids: Vec<String>,
    /// Player characters involved
    pub pc_ids: Vec<String>,
    /// Consequences/effects of this event
    pub consequences: Vec<String>,
    /// Session number when this occurred
    pub session_number: Option<u32>,
    /// Is this event public knowledge?
    pub is_public: bool,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl WorldEvent {
    pub fn new(
        campaign_id: &str,
        title: &str,
        description: &str,
        in_game_date: InGameDate,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            campaign_id: campaign_id.to_string(),
            in_game_date,
            recorded_at: Utc::now(),
            title: title.to_string(),
            description: description.to_string(),
            event_type: WorldEventType::default(),
            impact: EventImpact::default(),
            location_ids: vec![],
            npc_ids: vec![],
            pc_ids: vec![],
            consequences: vec![],
            session_number: None,
            is_public: true,
            metadata: HashMap::new(),
        }
    }

    /// Builder pattern for event type
    pub fn with_type(mut self, event_type: WorldEventType) -> Self {
        self.event_type = event_type;
        self
    }

    /// Builder pattern for impact
    pub fn with_impact(mut self, impact: EventImpact) -> Self {
        self.impact = impact;
        self
    }

    /// Builder pattern for locations
    pub fn at_locations(mut self, location_ids: Vec<String>) -> Self {
        self.location_ids = location_ids;
        self
    }

    /// Builder pattern for NPCs
    pub fn involving_npcs(mut self, npc_ids: Vec<String>) -> Self {
        self.npc_ids = npc_ids;
        self
    }
}

// ============================================================================
// Location State
// ============================================================================

/// Current state/condition of a location
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LocationCondition {
    Pristine,
    Normal,
    Damaged,
    Ruined,
    Destroyed,
    Occupied,
    Abandoned,
    UnderSiege,
    Cursed,
    Blessed,
    Custom(String),
}

impl Default for LocationCondition {
    fn default() -> Self {
        Self::Normal
    }
}

/// State of a location at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationState {
    /// Location identifier
    pub location_id: String,
    /// Location name (for display)
    pub name: String,
    /// Current condition
    pub condition: LocationCondition,
    /// Current ruler/owner
    pub ruler: Option<String>,
    /// Current faction control
    pub controlling_faction: Option<String>,
    /// Population estimate
    pub population: Option<u64>,
    /// Notable NPCs currently here
    pub notable_npcs: Vec<String>,
    /// Active effects/conditions
    pub active_effects: Vec<String>,
    /// Resources available
    pub resources: HashMap<String, i32>,
    /// Custom properties
    pub properties: HashMap<String, serde_json::Value>,
    /// Last updated
    pub updated_at: DateTime<Utc>,
    /// Last in-game date when state was accurate
    pub last_accurate_date: InGameDate,
}

impl LocationState {
    pub fn new(location_id: &str, name: &str) -> Self {
        Self {
            location_id: location_id.to_string(),
            name: name.to_string(),
            condition: LocationCondition::default(),
            ruler: None,
            controlling_faction: None,
            population: None,
            notable_npcs: vec![],
            active_effects: vec![],
            resources: HashMap::new(),
            properties: HashMap::new(),
            updated_at: Utc::now(),
            last_accurate_date: InGameDate::default(),
        }
    }
}

// ============================================================================
// NPC Relationship State
// ============================================================================

/// Disposition level (-100 to +100)
pub type Disposition = i32;

/// Relationship state between NPCs or NPC-to-faction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcRelationshipState {
    /// Source NPC ID
    pub npc_id: String,
    /// Target NPC ID or faction ID
    pub target_id: String,
    /// Target type (NPC, Faction, Player)
    pub target_type: String,
    /// Current disposition (-100 to +100)
    pub disposition: Disposition,
    /// Relationship type label
    pub relationship_type: String,
    /// How well they know each other (0-100)
    pub familiarity: u8,
    /// Recent interactions affecting the relationship
    pub recent_interactions: Vec<InteractionRecord>,
    /// Notes about the relationship
    pub notes: String,
}

/// Record of an interaction that affected a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionRecord {
    pub in_game_date: InGameDate,
    pub description: String,
    pub disposition_change: i32,
    pub session_number: Option<u32>,
}

// ============================================================================
// World State Container
// ============================================================================

/// Complete world state for a campaign
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    /// Campaign identifier
    pub campaign_id: String,
    /// Current in-game date
    pub current_date: InGameDate,
    /// Timeline of events
    pub events: Vec<WorldEvent>,
    /// Location states
    pub locations: HashMap<String, LocationState>,
    /// NPC relationship states
    pub npc_relationships: Vec<NpcRelationshipState>,
    /// Custom state fields (flexible key-value storage)
    pub custom_fields: HashMap<String, serde_json::Value>,
    /// Last real-world update time
    pub updated_at: DateTime<Utc>,
    /// Calendar configuration
    pub calendar_config: CalendarConfig,
}

/// Configuration for the in-game calendar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarConfig {
    pub name: String,
    pub months_per_year: u8,
    pub days_per_month: Vec<u8>,
    pub month_names: Vec<String>,
    pub week_days: Vec<String>,
    pub eras: Vec<String>,
}

impl Default for CalendarConfig {
    fn default() -> Self {
        Self {
            name: "Standard".to_string(),
            months_per_year: 12,
            days_per_month: vec![30; 12], // Simple 30-day months
            month_names: vec![
                "January".to_string(),
                "February".to_string(),
                "March".to_string(),
                "April".to_string(),
                "May".to_string(),
                "June".to_string(),
                "July".to_string(),
                "August".to_string(),
                "September".to_string(),
                "October".to_string(),
                "November".to_string(),
                "December".to_string(),
            ],
            week_days: vec![
                "Sunday".to_string(),
                "Monday".to_string(),
                "Tuesday".to_string(),
                "Wednesday".to_string(),
                "Thursday".to_string(),
                "Friday".to_string(),
                "Saturday".to_string(),
            ],
            eras: vec!["Common Era".to_string()],
        }
    }
}

impl WorldState {
    pub fn new(campaign_id: &str) -> Self {
        Self {
            campaign_id: campaign_id.to_string(),
            current_date: InGameDate::default(),
            events: vec![],
            locations: HashMap::new(),
            npc_relationships: vec![],
            custom_fields: HashMap::new(),
            updated_at: Utc::now(),
            calendar_config: CalendarConfig::default(),
        }
    }
}

// ============================================================================
// World State Manager
// ============================================================================

/// Manages world state for all campaigns
pub struct WorldStateManager {
    /// Campaign ID -> WorldState
    states: RwLock<HashMap<String, WorldState>>,
}

impl Default for WorldStateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldStateManager {
    pub fn new() -> Self {
        Self {
            states: RwLock::new(HashMap::new()),
        }
    }

    // ========================================================================
    // World State CRUD
    // ========================================================================

    /// Initialize world state for a campaign
    pub fn initialize(&self, campaign_id: &str) -> WorldState {
        let state = WorldState::new(campaign_id);
        self.states
            .write()
            .unwrap()
            .insert(campaign_id.to_string(), state.clone());
        state
    }

    /// Get world state for a campaign
    pub fn get_state(&self, campaign_id: &str) -> Option<WorldState> {
        self.states.read().unwrap().get(campaign_id).cloned()
    }

    /// Get or create world state
    pub fn get_or_create(&self, campaign_id: &str) -> WorldState {
        let states = self.states.read().unwrap();
        if let Some(state) = states.get(campaign_id) {
            return state.clone();
        }
        drop(states);
        self.initialize(campaign_id)
    }

    /// Update entire world state
    pub fn update_state(&self, state: WorldState) -> Result<()> {
        self.states
            .write()
            .unwrap()
            .insert(state.campaign_id.clone(), state);
        Ok(())
    }

    /// Delete world state for a campaign
    pub fn delete_state(&self, campaign_id: &str) {
        self.states.write().unwrap().remove(campaign_id);
    }

    // ========================================================================
    // Date Operations
    // ========================================================================

    /// Set the current in-game date
    pub fn set_current_date(&self, campaign_id: &str, date: InGameDate) -> Result<()> {
        let mut states = self.states.write().unwrap();
        let state = states
            .get_mut(campaign_id)
            .ok_or_else(|| WorldStateError::CampaignNotFound(campaign_id.to_string()))?;
        state.current_date = date;
        state.updated_at = Utc::now();
        Ok(())
    }

    /// Advance the current date by days
    pub fn advance_date(&self, campaign_id: &str, days: i32) -> Result<InGameDate> {
        let mut states = self.states.write().unwrap();
        let state = states
            .get_mut(campaign_id)
            .ok_or_else(|| WorldStateError::CampaignNotFound(campaign_id.to_string()))?;
        state.current_date.advance_days(days);
        state.updated_at = Utc::now();
        Ok(state.current_date.clone())
    }

    /// Get current date
    pub fn get_current_date(&self, campaign_id: &str) -> Result<InGameDate> {
        self.states
            .read()
            .unwrap()
            .get(campaign_id)
            .map(|s| s.current_date.clone())
            .ok_or_else(|| WorldStateError::CampaignNotFound(campaign_id.to_string()))
    }

    // ========================================================================
    // Event Operations
    // ========================================================================

    /// Add a world event
    pub fn add_event(&self, campaign_id: &str, mut event: WorldEvent) -> Result<WorldEvent> {
        event.campaign_id = campaign_id.to_string();
        let mut states = self.states.write().unwrap();
        let state = states
            .get_mut(campaign_id)
            .ok_or_else(|| WorldStateError::CampaignNotFound(campaign_id.to_string()))?;

        state.events.push(event.clone());
        state.updated_at = Utc::now();
        Ok(event)
    }

    /// Get event by ID
    pub fn get_event(&self, campaign_id: &str, event_id: &str) -> Option<WorldEvent> {
        self.states
            .read()
            .unwrap()
            .get(campaign_id)
            .and_then(|s| s.events.iter().find(|e| e.id == event_id).cloned())
    }

    /// List events (optionally filtered)
    pub fn list_events(
        &self,
        campaign_id: &str,
        event_type: Option<WorldEventType>,
        limit: Option<usize>,
    ) -> Vec<WorldEvent> {
        self.states
            .read()
            .unwrap()
            .get(campaign_id)
            .map(|s| {
                let mut events: Vec<_> = s
                    .events
                    .iter()
                    .filter(|e| event_type.as_ref().map_or(true, |t| &e.event_type == t))
                    .cloned()
                    .collect();
                // Sort by in-game date (most recent first)
                events.sort_by(|a, b| {
                    b.in_game_date
                        .year
                        .cmp(&a.in_game_date.year)
                        .then(b.in_game_date.month.cmp(&a.in_game_date.month))
                        .then(b.in_game_date.day.cmp(&a.in_game_date.day))
                });
                if let Some(n) = limit {
                    events.truncate(n);
                }
                events
            })
            .unwrap_or_default()
    }

    /// Delete an event
    pub fn delete_event(&self, campaign_id: &str, event_id: &str) -> Result<()> {
        let mut states = self.states.write().unwrap();
        let state = states
            .get_mut(campaign_id)
            .ok_or_else(|| WorldStateError::CampaignNotFound(campaign_id.to_string()))?;

        let pos = state
            .events
            .iter()
            .position(|e| e.id == event_id)
            .ok_or_else(|| WorldStateError::EventNotFound(event_id.to_string()))?;

        state.events.remove(pos);
        state.updated_at = Utc::now();
        Ok(())
    }

    // ========================================================================
    // Location State Operations
    // ========================================================================

    /// Set location state
    pub fn set_location_state(&self, campaign_id: &str, location: LocationState) -> Result<()> {
        let mut states = self.states.write().unwrap();
        let state = states
            .get_mut(campaign_id)
            .ok_or_else(|| WorldStateError::CampaignNotFound(campaign_id.to_string()))?;

        state
            .locations
            .insert(location.location_id.clone(), location);
        state.updated_at = Utc::now();
        Ok(())
    }

    /// Get location state
    pub fn get_location_state(&self, campaign_id: &str, location_id: &str) -> Option<LocationState> {
        self.states
            .read()
            .unwrap()
            .get(campaign_id)
            .and_then(|s| s.locations.get(location_id).cloned())
    }

    /// List all locations
    pub fn list_locations(&self, campaign_id: &str) -> Vec<LocationState> {
        self.states
            .read()
            .unwrap()
            .get(campaign_id)
            .map(|s| s.locations.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Update location condition
    pub fn update_location_condition(
        &self,
        campaign_id: &str,
        location_id: &str,
        condition: LocationCondition,
    ) -> Result<()> {
        let mut states = self.states.write().unwrap();
        let state = states
            .get_mut(campaign_id)
            .ok_or_else(|| WorldStateError::CampaignNotFound(campaign_id.to_string()))?;

        let location = state
            .locations
            .get_mut(location_id)
            .ok_or_else(|| WorldStateError::LocationNotFound(location_id.to_string()))?;

        location.condition = condition;
        location.updated_at = Utc::now();
        state.updated_at = Utc::now();
        Ok(())
    }

    // ========================================================================
    // NPC Relationship Operations
    // ========================================================================

    /// Set NPC relationship
    pub fn set_npc_relationship(&self, campaign_id: &str, relationship: NpcRelationshipState) -> Result<()> {
        let mut states = self.states.write().unwrap();
        let state = states
            .get_mut(campaign_id)
            .ok_or_else(|| WorldStateError::CampaignNotFound(campaign_id.to_string()))?;

        // Update existing or add new
        if let Some(existing) = state
            .npc_relationships
            .iter_mut()
            .find(|r| r.npc_id == relationship.npc_id && r.target_id == relationship.target_id)
        {
            *existing = relationship;
        } else {
            state.npc_relationships.push(relationship);
        }

        state.updated_at = Utc::now();
        Ok(())
    }

    /// Get relationships for an NPC
    pub fn get_npc_relationships(&self, campaign_id: &str, npc_id: &str) -> Vec<NpcRelationshipState> {
        self.states
            .read()
            .unwrap()
            .get(campaign_id)
            .map(|s| {
                s.npc_relationships
                    .iter()
                    .filter(|r| r.npc_id == npc_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Modify disposition between NPCs
    pub fn modify_disposition(
        &self,
        campaign_id: &str,
        npc_id: &str,
        target_id: &str,
        delta: i32,
        interaction: Option<InteractionRecord>,
    ) -> Result<Disposition> {
        let mut states = self.states.write().unwrap();
        let state = states
            .get_mut(campaign_id)
            .ok_or_else(|| WorldStateError::CampaignNotFound(campaign_id.to_string()))?;

        if let Some(rel) = state
            .npc_relationships
            .iter_mut()
            .find(|r| r.npc_id == npc_id && r.target_id == target_id)
        {
            rel.disposition = (rel.disposition + delta).clamp(-100, 100);
            if let Some(interaction) = interaction {
                rel.recent_interactions.push(interaction);
                // Keep only last 10 interactions
                if rel.recent_interactions.len() > 10 {
                    rel.recent_interactions.remove(0);
                }
            }
            state.updated_at = Utc::now();
            Ok(rel.disposition)
        } else {
            Err(WorldStateError::EventNotFound(format!(
                "Relationship between {} and {}",
                npc_id, target_id
            )))
        }
    }

    // ========================================================================
    // Custom Fields Operations
    // ========================================================================

    /// Set a custom field
    pub fn set_custom_field(
        &self,
        campaign_id: &str,
        key: &str,
        value: serde_json::Value,
    ) -> Result<()> {
        let mut states = self.states.write().unwrap();
        let state = states
            .get_mut(campaign_id)
            .ok_or_else(|| WorldStateError::CampaignNotFound(campaign_id.to_string()))?;

        state.custom_fields.insert(key.to_string(), value);
        state.updated_at = Utc::now();
        Ok(())
    }

    /// Get a custom field
    pub fn get_custom_field(&self, campaign_id: &str, key: &str) -> Option<serde_json::Value> {
        self.states
            .read()
            .unwrap()
            .get(campaign_id)
            .and_then(|s| s.custom_fields.get(key).cloned())
    }

    /// List all custom fields
    pub fn list_custom_fields(&self, campaign_id: &str) -> HashMap<String, serde_json::Value> {
        self.states
            .read()
            .unwrap()
            .get(campaign_id)
            .map(|s| s.custom_fields.clone())
            .unwrap_or_default()
    }

    /// Delete a custom field
    pub fn delete_custom_field(&self, campaign_id: &str, key: &str) -> Result<()> {
        let mut states = self.states.write().unwrap();
        let state = states
            .get_mut(campaign_id)
            .ok_or_else(|| WorldStateError::CampaignNotFound(campaign_id.to_string()))?;

        state
            .custom_fields
            .remove(key)
            .ok_or_else(|| WorldStateError::CustomFieldNotFound(key.to_string()))?;

        state.updated_at = Utc::now();
        Ok(())
    }

    // ========================================================================
    // Calendar Configuration
    // ========================================================================

    /// Set calendar configuration
    pub fn set_calendar_config(&self, campaign_id: &str, config: CalendarConfig) -> Result<()> {
        let mut states = self.states.write().unwrap();
        let state = states
            .get_mut(campaign_id)
            .ok_or_else(|| WorldStateError::CampaignNotFound(campaign_id.to_string()))?;

        state.calendar_config = config;
        state.updated_at = Utc::now();
        Ok(())
    }

    /// Get calendar configuration
    pub fn get_calendar_config(&self, campaign_id: &str) -> Option<CalendarConfig> {
        self.states
            .read()
            .unwrap()
            .get(campaign_id)
            .map(|s| s.calendar_config.clone())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_game_date() {
        let mut date = InGameDate::new(1492, 6, 15);
        assert_eq!(date.display(), "15/6/1492");

        date.advance_days(20);
        assert_eq!(date.day, 5);
        assert_eq!(date.month, 7);
    }

    #[test]
    fn test_world_state_manager() {
        let manager = WorldStateManager::new();

        // Initialize
        let state = manager.initialize("camp-1");
        assert_eq!(state.campaign_id, "camp-1");

        // Set date
        manager
            .set_current_date("camp-1", InGameDate::new(1492, 6, 1))
            .unwrap();

        let date = manager.get_current_date("camp-1").unwrap();
        assert_eq!(date.year, 1492);
    }

    #[test]
    fn test_events() {
        let manager = WorldStateManager::new();
        manager.initialize("camp-1");

        let event = WorldEvent::new(
            "camp-1",
            "Dragon Attack",
            "A dragon attacked the village",
            InGameDate::new(1492, 6, 15),
        )
        .with_type(WorldEventType::Combat)
        .with_impact(EventImpact::Regional);

        let saved = manager.add_event("camp-1", event).unwrap();
        assert!(!saved.id.is_empty());

        let events = manager.list_events("camp-1", None, None);
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_location_state() {
        let manager = WorldStateManager::new();
        manager.initialize("camp-1");

        let mut location = LocationState::new("loc-1", "Neverwinter");
        location.condition = LocationCondition::Normal;
        location.population = Some(100_000);

        manager.set_location_state("camp-1", location).unwrap();

        let retrieved = manager.get_location_state("camp-1", "loc-1").unwrap();
        assert_eq!(retrieved.name, "Neverwinter");
        assert_eq!(retrieved.population, Some(100_000));
    }

    #[test]
    fn test_custom_fields() {
        let manager = WorldStateManager::new();
        manager.initialize("camp-1");

        manager
            .set_custom_field("camp-1", "moon_phase", serde_json::json!("full"))
            .unwrap();
        manager
            .set_custom_field("camp-1", "weather", serde_json::json!({"type": "rain", "intensity": 3}))
            .unwrap();

        let moon = manager.get_custom_field("camp-1", "moon_phase").unwrap();
        assert_eq!(moon, serde_json::json!("full"));

        let fields = manager.list_custom_fields("camp-1");
        assert_eq!(fields.len(), 2);
    }
}
