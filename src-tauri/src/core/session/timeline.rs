//! Session Timeline Module (TASK-014)
//!
//! Provides timeline event tracking for game sessions, enabling
//! comprehensive session history and summary generation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

// ============================================================================
// Timeline Event Types
// ============================================================================

/// Types of events that can occur during a session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TimelineEventType {
    /// Session lifecycle
    SessionStart,
    SessionPause,
    SessionResume,
    SessionEnd,

    /// Combat events
    CombatStart,
    CombatEnd,
    CombatRoundStart,
    CombatTurnStart,
    CombatDamage,
    CombatHealing,
    CombatDeath,

    /// Notes and documentation
    NoteAdded,
    NoteEdited,
    NoteDeleted,

    /// NPC interactions
    NPCInteraction,
    NPCDialogue,
    NPCMood,

    /// Location and scene
    LocationChange,
    SceneChange,

    /// Player actions
    PlayerAction,
    PlayerRoll,
    SkillCheck,
    SavingThrow,

    /// Conditions and status
    ConditionApplied,
    ConditionRemoved,
    ConditionExpired,

    /// Items and treasure
    ItemAcquired,
    ItemUsed,
    ItemLost,

    /// Custom/misc
    Custom(String),
}

/// Severity/importance level of timeline events
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum EventSeverity {
    /// Background events (auto-tracking)
    Trace,
    /// Normal session flow
    Info,
    /// Notable moments
    Notable,
    /// Important story beats
    Important,
    /// Critical/defining moments
    Critical,
}

impl Default for EventSeverity {
    fn default() -> Self {
        Self::Info
    }
}

// ============================================================================
// Timeline Event
// ============================================================================

/// A single event in the session timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    /// Unique identifier
    pub id: String,
    /// Session this event belongs to
    pub session_id: String,
    /// Type of event
    pub event_type: TimelineEventType,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// Human-readable title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Event importance
    pub severity: EventSeverity,
    /// Entity references (NPC IDs, location IDs, etc.)
    pub entity_refs: Vec<EntityRef>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Reference to an entity involved in an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRef {
    /// Type of entity (npc, location, item, combatant, etc.)
    pub entity_type: String,
    /// ID of the entity
    pub entity_id: String,
    /// Display name
    pub name: String,
    /// Role in the event (actor, target, etc.)
    pub role: Option<String>,
}

impl TimelineEvent {
    /// Create a new timeline event
    pub fn new(
        session_id: impl Into<String>,
        event_type: TimelineEventType,
        title: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            event_type,
            timestamp: Utc::now(),
            title: title.into(),
            description: description.into(),
            severity: EventSeverity::default(),
            entity_refs: Vec::new(),
            metadata: HashMap::new(),
            tags: Vec::new(),
        }
    }

    /// Builder: set severity
    pub fn with_severity(mut self, severity: EventSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Builder: add entity reference
    pub fn with_entity(mut self, entity_type: impl Into<String>, entity_id: impl Into<String>, name: impl Into<String>) -> Self {
        self.entity_refs.push(EntityRef {
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            name: name.into(),
            role: None,
        });
        self
    }

    /// Builder: add entity reference with role
    pub fn with_entity_role(
        mut self,
        entity_type: impl Into<String>,
        entity_id: impl Into<String>,
        name: impl Into<String>,
        role: impl Into<String>,
    ) -> Self {
        self.entity_refs.push(EntityRef {
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            name: name.into(),
            role: Some(role.into()),
        });
        self
    }

    /// Builder: add metadata
    pub fn with_meta(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.metadata.insert(key.into(), v);
        }
        self
    }

    /// Builder: add tags
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(|t| t.into()));
        self
    }

    /// Builder: set custom timestamp
    pub fn at(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = timestamp;
        self
    }
}

// ============================================================================
// Session Timeline
// ============================================================================

/// Container for session timeline events
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionTimeline {
    /// Session ID this timeline belongs to
    pub session_id: String,
    /// All events in chronological order
    events: Vec<TimelineEvent>,
    /// Quick lookup by event type
    #[serde(skip)]
    type_index: HashMap<TimelineEventType, Vec<usize>>,
}

impl SessionTimeline {
    /// Create a new timeline for a session
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            events: Vec::new(),
            type_index: HashMap::new(),
        }
    }

    /// Add an event to the timeline
    pub fn add_event(&mut self, event: TimelineEvent) -> &TimelineEvent {
        let idx = self.events.len();

        // Update type index
        self.type_index
            .entry(event.event_type.clone())
            .or_default()
            .push(idx);

        self.events.push(event);
        &self.events[idx]
    }

    /// Quick event creation helpers
    pub fn log(&mut self, event_type: TimelineEventType, title: &str, description: &str) -> &TimelineEvent {
        let event = TimelineEvent::new(&self.session_id, event_type, title, description);
        self.add_event(event)
    }

    /// Log a notable event
    pub fn log_notable(&mut self, event_type: TimelineEventType, title: &str, description: &str) -> &TimelineEvent {
        let event = TimelineEvent::new(&self.session_id, event_type, title, description)
            .with_severity(EventSeverity::Notable);
        self.add_event(event)
    }

    /// Log an important event
    pub fn log_important(&mut self, event_type: TimelineEventType, title: &str, description: &str) -> &TimelineEvent {
        let event = TimelineEvent::new(&self.session_id, event_type, title, description)
            .with_severity(EventSeverity::Important);
        self.add_event(event)
    }

    /// Log a critical event
    pub fn log_critical(&mut self, event_type: TimelineEventType, title: &str, description: &str) -> &TimelineEvent {
        let event = TimelineEvent::new(&self.session_id, event_type, title, description)
            .with_severity(EventSeverity::Critical);
        self.add_event(event)
    }

    /// Get all events
    pub fn events(&self) -> &[TimelineEvent] {
        &self.events
    }

    /// Get event by ID
    pub fn get_event(&self, event_id: &str) -> Option<&TimelineEvent> {
        self.events.iter().find(|e| e.id == event_id)
    }

    /// Get events by type
    pub fn events_by_type(&self, event_type: &TimelineEventType) -> Vec<&TimelineEvent> {
        self.type_index
            .get(event_type)
            .map(|indices| indices.iter().filter_map(|&i| self.events.get(i)).collect())
            .unwrap_or_default()
    }

    /// Get events in a time range
    pub fn events_in_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<&TimelineEvent> {
        self.events
            .iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .collect()
    }

    /// Get events by severity (at or above)
    pub fn events_by_severity(&self, min_severity: EventSeverity) -> Vec<&TimelineEvent> {
        self.events
            .iter()
            .filter(|e| e.severity >= min_severity)
            .collect()
    }

    /// Get events involving an entity
    pub fn events_for_entity(&self, entity_id: &str) -> Vec<&TimelineEvent> {
        self.events
            .iter()
            .filter(|e| e.entity_refs.iter().any(|r| r.entity_id == entity_id))
            .collect()
    }

    /// Get events with a specific tag
    pub fn events_with_tag(&self, tag: &str) -> Vec<&TimelineEvent> {
        self.events
            .iter()
            .filter(|e| e.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// Get combat events for current or most recent combat
    pub fn combat_events(&self) -> Vec<&TimelineEvent> {
        self.events
            .iter()
            .filter(|e| matches!(
                e.event_type,
                TimelineEventType::CombatStart |
                TimelineEventType::CombatEnd |
                TimelineEventType::CombatRoundStart |
                TimelineEventType::CombatTurnStart |
                TimelineEventType::CombatDamage |
                TimelineEventType::CombatHealing |
                TimelineEventType::CombatDeath
            ))
            .collect()
    }

    /// Get last N events
    pub fn recent_events(&self, count: usize) -> Vec<&TimelineEvent> {
        self.events.iter().rev().take(count).collect()
    }

    /// Rebuild type index (call after deserialization)
    pub fn rebuild_index(&mut self) {
        self.type_index.clear();
        for (idx, event) in self.events.iter().enumerate() {
            self.type_index
                .entry(event.event_type.clone())
                .or_default()
                .push(idx);
        }
    }

    /// Get total event count
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if timeline is empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

// ============================================================================
// Session Summary Generation
// ============================================================================

/// Generated session summary from timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineSummary {
    /// Session ID
    pub session_id: String,
    /// Session duration
    pub duration_minutes: i64,
    /// Number of events
    pub total_events: usize,
    /// Combat summary
    pub combat: CombatSummary,
    /// Key moments (notable+ severity)
    pub key_moments: Vec<KeyMoment>,
    /// NPCs encountered
    pub npcs_encountered: Vec<EntityRef>,
    /// Locations visited
    pub locations_visited: Vec<EntityRef>,
    /// Items acquired
    pub items_acquired: Vec<String>,
    /// Conditions applied
    pub conditions_applied: Vec<String>,
    /// Custom tags used
    pub tags_used: Vec<String>,
}

/// Summary of combat activity
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CombatSummary {
    /// Number of combat encounters
    pub encounters: usize,
    /// Total rounds of combat
    pub total_rounds: u32,
    /// Total damage dealt (if tracked)
    pub damage_dealt: Option<i32>,
    /// Total healing done (if tracked)
    pub healing_done: Option<i32>,
    /// Number of deaths/knockouts
    pub deaths: usize,
}

/// A key moment from the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMoment {
    /// Event title
    pub title: String,
    /// Event description
    pub description: String,
    /// When it occurred (relative to session start)
    pub time_offset_minutes: i64,
    /// Severity level
    pub severity: EventSeverity,
    /// Event type
    pub event_type: TimelineEventType,
}

impl SessionTimeline {
    /// Generate a summary of the session from the timeline
    pub fn generate_summary(&self) -> TimelineSummary {
        let start_time = self.events.first().map(|e| e.timestamp);
        let end_time = self.events.last().map(|e| e.timestamp);

        let duration_minutes = match (start_time, end_time) {
            (Some(start), Some(end)) => (end - start).num_minutes(),
            _ => 0,
        };

        // Combat summary
        let combat_starts = self.events_by_type(&TimelineEventType::CombatStart).len();
        let combat_rounds = self.events_by_type(&TimelineEventType::CombatRoundStart).len();
        let deaths = self.events_by_type(&TimelineEventType::CombatDeath).len();

        let damage_events = self.events_by_type(&TimelineEventType::CombatDamage);
        let healing_events = self.events_by_type(&TimelineEventType::CombatHealing);

        let damage_dealt: Option<i32> = if damage_events.is_empty() {
            None
        } else {
            Some(damage_events.iter()
                .filter_map(|e| e.metadata.get("amount").and_then(|v| v.as_i64()))
                .sum::<i64>() as i32)
        };

        let healing_done: Option<i32> = if healing_events.is_empty() {
            None
        } else {
            Some(healing_events.iter()
                .filter_map(|e| e.metadata.get("amount").and_then(|v| v.as_i64()))
                .sum::<i64>() as i32)
        };

        // Key moments
        let key_moments: Vec<KeyMoment> = self.events_by_severity(EventSeverity::Notable)
            .iter()
            .map(|e| {
                let time_offset = start_time
                    .map(|s| (e.timestamp - s).num_minutes())
                    .unwrap_or(0);
                KeyMoment {
                    title: e.title.clone(),
                    description: e.description.clone(),
                    time_offset_minutes: time_offset,
                    severity: e.severity,
                    event_type: e.event_type.clone(),
                }
            })
            .collect();

        // Collect unique NPCs
        let mut npcs_encountered: Vec<EntityRef> = Vec::new();
        let mut seen_npcs: std::collections::HashSet<String> = std::collections::HashSet::new();
        for event in self.events_by_type(&TimelineEventType::NPCInteraction) {
            for entity in &event.entity_refs {
                if entity.entity_type == "npc" && !seen_npcs.contains(&entity.entity_id) {
                    seen_npcs.insert(entity.entity_id.clone());
                    npcs_encountered.push(entity.clone());
                }
            }
        }

        // Collect unique locations
        let mut locations_visited: Vec<EntityRef> = Vec::new();
        let mut seen_locations: std::collections::HashSet<String> = std::collections::HashSet::new();
        for event in self.events_by_type(&TimelineEventType::LocationChange) {
            for entity in &event.entity_refs {
                if entity.entity_type == "location" && !seen_locations.contains(&entity.entity_id) {
                    seen_locations.insert(entity.entity_id.clone());
                    locations_visited.push(entity.clone());
                }
            }
        }

        // Collect items acquired
        let items_acquired: Vec<String> = self.events_by_type(&TimelineEventType::ItemAcquired)
            .iter()
            .filter_map(|e| e.metadata.get("item_name").and_then(|v| v.as_str()))
            .map(|s| s.to_string())
            .collect();

        // Collect conditions applied
        let mut conditions_applied: Vec<String> = self.events_by_type(&TimelineEventType::ConditionApplied)
            .iter()
            .filter_map(|e| e.metadata.get("condition_name").and_then(|v| v.as_str()))
            .map(|s| s.to_string())
            .collect();
        conditions_applied.sort();
        conditions_applied.dedup();

        // Collect all tags used
        let mut tags_used: Vec<String> = self.events
            .iter()
            .flat_map(|e| e.tags.iter().cloned())
            .collect();
        tags_used.sort();
        tags_used.dedup();

        TimelineSummary {
            session_id: self.session_id.clone(),
            duration_minutes,
            total_events: self.events.len(),
            combat: CombatSummary {
                encounters: combat_starts,
                total_rounds: combat_rounds as u32,
                damage_dealt,
                healing_done,
                deaths,
            },
            key_moments,
            npcs_encountered,
            locations_visited,
            items_acquired,
            conditions_applied,
            tags_used,
        }
    }

    /// Generate a text narrative of the session for AI consumption
    pub fn generate_narrative(&self) -> String {
        let summary = self.generate_summary();
        let mut narrative = String::new();

        // Header
        narrative.push_str(&format!(
            "Session Summary ({})\n",
            self.session_id
        ));
        narrative.push_str(&format!(
            "Duration: {} minutes | Events: {}\n\n",
            summary.duration_minutes,
            summary.total_events
        ));

        // Key moments
        if !summary.key_moments.is_empty() {
            narrative.push_str("KEY MOMENTS:\n");
            for moment in &summary.key_moments {
                narrative.push_str(&format!(
                    "- [{:?}] {} - {}\n",
                    moment.severity,
                    moment.title,
                    moment.description
                ));
            }
            narrative.push('\n');
        }

        // Combat
        if summary.combat.encounters > 0 {
            narrative.push_str(&format!(
                "COMBAT: {} encounter(s), {} total rounds",
                summary.combat.encounters,
                summary.combat.total_rounds
            ));
            if let Some(damage) = summary.combat.damage_dealt {
                narrative.push_str(&format!(", {} damage dealt", damage));
            }
            if let Some(healing) = summary.combat.healing_done {
                narrative.push_str(&format!(", {} healing done", healing));
            }
            if summary.combat.deaths > 0 {
                narrative.push_str(&format!(", {} death(s)", summary.combat.deaths));
            }
            narrative.push_str("\n\n");
        }

        // NPCs
        if !summary.npcs_encountered.is_empty() {
            narrative.push_str("NPCs ENCOUNTERED: ");
            let names: Vec<&str> = summary.npcs_encountered.iter().map(|n| n.name.as_str()).collect();
            narrative.push_str(&names.join(", "));
            narrative.push_str("\n\n");
        }

        // Locations
        if !summary.locations_visited.is_empty() {
            narrative.push_str("LOCATIONS VISITED: ");
            let names: Vec<&str> = summary.locations_visited.iter().map(|l| l.name.as_str()).collect();
            narrative.push_str(&names.join(", "));
            narrative.push_str("\n\n");
        }

        // Items
        if !summary.items_acquired.is_empty() {
            narrative.push_str("ITEMS ACQUIRED: ");
            narrative.push_str(&summary.items_acquired.join(", "));
            narrative.push_str("\n\n");
        }

        narrative
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_event_creation() {
        let event = TimelineEvent::new(
            "session-1",
            TimelineEventType::CombatStart,
            "Battle Begins",
            "The party engages the goblins"
        )
        .with_severity(EventSeverity::Notable)
        .with_entity("npc", "goblin-1", "Goblin Warleader")
        .with_meta("combatants", 5);

        assert_eq!(event.session_id, "session-1");
        assert_eq!(event.severity, EventSeverity::Notable);
        assert_eq!(event.entity_refs.len(), 1);
        assert!(event.metadata.contains_key("combatants"));
    }

    #[test]
    fn test_timeline_add_and_query() {
        let mut timeline = SessionTimeline::new("session-1");

        timeline.log(TimelineEventType::SessionStart, "Session Started", "The adventure begins");
        timeline.log_notable(TimelineEventType::CombatStart, "Combat!", "Roll initiative");
        timeline.log(TimelineEventType::CombatDamage, "Hit!", "Goblin takes 8 damage");
        timeline.log_important(TimelineEventType::CombatEnd, "Victory", "The party prevails");

        assert_eq!(timeline.len(), 4);
        assert_eq!(timeline.events_by_type(&TimelineEventType::CombatStart).len(), 1);
        assert_eq!(timeline.events_by_severity(EventSeverity::Notable).len(), 2);
    }

    #[test]
    fn test_summary_generation() {
        let mut timeline = SessionTimeline::new("session-1");

        timeline.log(TimelineEventType::SessionStart, "Start", "Session begins");
        timeline.log_notable(TimelineEventType::CombatStart, "Combat", "Battle starts");
        timeline.log(TimelineEventType::CombatRoundStart, "Round 1", "First round");
        timeline.log(TimelineEventType::CombatRoundStart, "Round 2", "Second round");
        timeline.log(TimelineEventType::CombatEnd, "End", "Combat ends");
        timeline.log(TimelineEventType::SessionEnd, "Finish", "Session over");

        let summary = timeline.generate_summary();
        assert_eq!(summary.combat.encounters, 1);
        assert_eq!(summary.combat.total_rounds, 2);
        assert!(summary.key_moments.len() >= 1);
    }
}
