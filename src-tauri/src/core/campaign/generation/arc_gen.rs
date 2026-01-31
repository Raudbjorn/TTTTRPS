//! Arc Outline Generator
//!
//! Phase 4, Task 4.7: Arc outline generation with tension curves
//!
//! Generates narrative arc outlines with phase structures and tension tracking.

use super::orchestrator::{GenerationConfig, GenerationError, GenerationRequest, GenerationType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

/// Arc template type (narrative structure)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArcTemplateType {
    /// Classic hero's journey
    HerosJourney,
    /// Three-act structure
    ThreeAct,
    /// Five-act structure
    FiveAct,
    /// Mystery/investigation
    Mystery,
    /// Political intrigue
    PoliticalIntrigue,
    /// Dungeon delve/exploration
    DungeonDelve,
    /// Custom structure
    Custom,
}

impl Default for ArcTemplateType {
    fn default() -> Self {
        ArcTemplateType::ThreeAct
    }
}

impl ArcTemplateType {
    /// Get the snake_case string representation for templates
    pub fn as_str(&self) -> &'static str {
        match self {
            ArcTemplateType::HerosJourney => "heros_journey",
            ArcTemplateType::ThreeAct => "three_act",
            ArcTemplateType::FiveAct => "five_act",
            ArcTemplateType::Mystery => "mystery",
            ArcTemplateType::PoliticalIntrigue => "political_intrigue",
            ArcTemplateType::DungeonDelve => "dungeon_delve",
            ArcTemplateType::Custom => "custom",
        }
    }

    /// Get the default phase names for this template
    pub fn default_phases(&self) -> Vec<&'static str> {
        match self {
            ArcTemplateType::HerosJourney => vec![
                "Ordinary World",
                "Call to Adventure",
                "Crossing the Threshold",
                "Tests & Allies",
                "Approach",
                "Ordeal",
                "Reward",
                "Return",
            ],
            ArcTemplateType::ThreeAct => vec!["Setup", "Confrontation", "Resolution"],
            ArcTemplateType::FiveAct => vec![
                "Exposition",
                "Rising Action",
                "Climax",
                "Falling Action",
                "Resolution",
            ],
            ArcTemplateType::Mystery => vec![
                "The Hook",
                "Investigation",
                "Red Herrings",
                "Revelation",
                "Confrontation",
            ],
            ArcTemplateType::PoliticalIntrigue => vec![
                "The Web",
                "Allegiances",
                "Betrayal",
                "Power Play",
                "New Order",
            ],
            ArcTemplateType::DungeonDelve => vec![
                "Descent",
                "Exploration",
                "Trials",
                "The Heart",
                "Escape",
            ],
            ArcTemplateType::Custom => vec!["Phase 1", "Phase 2", "Phase 3"],
        }
    }
}

/// Request for arc outline generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcGenerationRequest {
    /// Campaign ID for context
    pub campaign_id: Option<String>,
    /// High-level arc concept
    pub arc_concept: String,
    /// Arc template type
    pub arc_type: ArcTemplateType,
    /// Estimated number of sessions
    pub estimated_sessions: Option<String>,
    /// Party level range
    pub level_range: Option<(u8, u8)>,
    /// Current campaign state
    pub campaign_state: Option<String>,
    /// Available NPCs to use
    pub available_npcs: Vec<String>,
    /// Available locations
    pub available_locations: Vec<String>,
    /// Generation configuration
    pub config: GenerationConfig,
}

impl ArcGenerationRequest {
    /// Create a new arc generation request
    pub fn new(arc_concept: impl Into<String>) -> Self {
        Self {
            campaign_id: None,
            arc_concept: arc_concept.into(),
            arc_type: ArcTemplateType::ThreeAct,
            estimated_sessions: Some("5-8".to_string()),
            level_range: None,
            campaign_state: None,
            available_npcs: Vec::new(),
            available_locations: Vec::new(),
            config: GenerationConfig::default(),
        }
    }

    /// Set campaign ID
    pub fn with_campaign_id(mut self, campaign_id: impl Into<String>) -> Self {
        self.campaign_id = Some(campaign_id.into());
        self
    }

    /// Set arc type
    pub fn with_arc_type(mut self, arc_type: ArcTemplateType) -> Self {
        self.arc_type = arc_type;
        self
    }

    /// Set estimated sessions
    pub fn with_sessions(mut self, sessions: impl Into<String>) -> Self {
        self.estimated_sessions = Some(sessions.into());
        self
    }

    /// Set level range
    pub fn with_level_range(mut self, start: u8, end: u8) -> Self {
        self.level_range = Some((start, end));
        self
    }

    /// Set available NPCs
    pub fn with_npcs(mut self, npcs: Vec<String>) -> Self {
        self.available_npcs = npcs;
        self
    }

    /// Set available locations
    pub fn with_locations(mut self, locations: Vec<String>) -> Self {
        self.available_locations = locations;
        self
    }

    /// Convert to a GenerationRequest
    pub fn to_generation_request(self) -> GenerationRequest {
        let mut vars = HashMap::new();
        vars.insert("arc_concept".to_string(), self.arc_concept);
        vars.insert("arc_type".to_string(), self.arc_type.as_str().to_string());

        if let Some(sessions) = self.estimated_sessions {
            vars.insert("estimated_sessions".to_string(), sessions);
        }
        if let Some((start, end)) = self.level_range {
            vars.insert("level_range".to_string(), format!("{}-{}", start, end));
        }
        if let Some(state) = self.campaign_state {
            vars.insert("campaign_state".to_string(), state);
        }
        if !self.available_npcs.is_empty() {
            vars.insert("available_npcs".to_string(), self.available_npcs.join(", "));
        }
        if !self.available_locations.is_empty() {
            vars.insert("available_locations".to_string(), self.available_locations.join(", "));
        }

        let mut request = GenerationRequest::new(GenerationType::ArcOutline)
            .with_variables(vars);

        if let Some(campaign_id) = self.campaign_id {
            request = request.with_campaign_id(campaign_id);
        }

        request.config = self.config;
        request
    }
}

/// Generated arc draft
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcDraft {
    /// Arc overview
    pub arc: ArcOverview,
    /// Arc phases
    pub phases: Vec<ArcPhase>,
    /// Tension curve
    pub tension_curve: TensionCurve,
    /// Possible outcomes
    pub possible_outcomes: Vec<ArcOutcome>,
    /// Main antagonist details
    pub antagonist: Option<ArcAntagonist>,
    /// Raw data from generation
    pub raw_data: Option<serde_json::Value>,
}

/// Arc overview information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcOverview {
    /// Arc title
    pub title: String,
    /// One-line description
    pub tagline: String,
    /// Arc type/structure
    pub arc_type: String,
    /// Themes explored
    pub themes: Vec<String>,
    /// Estimated sessions
    pub estimated_sessions: u32,
    /// Level range
    pub level_range: LevelRange,
}

/// Level range for an arc
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelRange {
    pub start: u8,
    pub end: u8,
}

impl Default for LevelRange {
    fn default() -> Self {
        Self { start: 1, end: 5 }
    }
}

/// A phase within an arc
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcPhase {
    /// Phase name
    pub name: String,
    /// Phase type in narrative structure
    pub phase_type: PhaseType,
    /// Estimated sessions for this phase
    pub sessions: u32,
    /// Tension level (1-10)
    pub tension_level: u8,
    /// Phase objectives
    pub objectives: Vec<String>,
    /// Key scenes
    pub key_scenes: Vec<String>,
    /// Player decision points
    pub decision_points: Vec<String>,
    /// NPCs introduced
    pub npcs_introduced: Vec<String>,
    /// Locations used
    pub locations: Vec<String>,
}

/// Phase type in narrative structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseType {
    Setup,
    RisingAction,
    Climax,
    FallingAction,
    Resolution,
}

/// Tension curve data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensionCurve {
    /// Points on the tension curve
    pub points: Vec<TensionPoint>,
}

impl TensionCurve {
    /// Create an empty tension curve
    pub fn empty() -> Self {
        Self { points: Vec::new() }
    }

    /// Get the maximum tension level
    pub fn max_tension(&self) -> u8 {
        self.points.iter().map(|p| p.tension).max().unwrap_or(0)
    }

    /// Get the session where climax occurs
    pub fn climax_session(&self) -> Option<u32> {
        self.points
            .iter()
            .max_by_key(|p| p.tension)
            .map(|p| p.session)
    }
}

/// A point on the tension curve
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensionPoint {
    /// Session number
    pub session: u32,
    /// Tension level (1-10)
    pub tension: u8,
    /// Event causing this tension
    pub event: String,
}

/// A possible arc outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcOutcome {
    /// Outcome name
    pub name: String,
    /// How likely this is
    pub likelihood: OutcomeLikelihood,
    /// Consequences
    pub consequences: String,
}

/// Likelihood of an outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutcomeLikelihood {
    Likely,
    Possible,
    Unlikely,
}

/// Arc antagonist details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcAntagonist {
    /// Antagonist name
    pub name: String,
    /// Motivation
    pub motivation: String,
    /// Resources and allies
    pub resources: Vec<String>,
    /// How they respond to party interference
    pub escalation_plan: Vec<String>,
}

// ============================================================================
// Arc Generator
// ============================================================================

/// Generator for arc outlines
pub struct ArcGenerator;

impl ArcGenerator {
    /// Parse a generation response into an ArcDraft
    pub fn parse_response(response: &serde_json::Value) -> Result<ArcDraft, GenerationError> {
        let arc_data = response.get("arc").unwrap_or(response);

        let arc = ArcOverview {
            title: arc_data.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled Arc").to_string(),
            tagline: arc_data.get("tagline").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            arc_type: arc_data.get("type").and_then(|v| v.as_str()).unwrap_or("three_act").to_string(),
            themes: Self::parse_string_array(arc_data.get("themes")),
            estimated_sessions: arc_data.get("estimated_sessions")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32)
                .unwrap_or(5),
            level_range: arc_data.get("level_range").map(|lr| {
                LevelRange {
                    start: lr.get("start").and_then(|v| v.as_u64()).map(|n| n as u8).unwrap_or(1),
                    end: lr.get("end").and_then(|v| v.as_u64()).map(|n| n as u8).unwrap_or(5),
                }
            }).unwrap_or_default(),
        };

        let phases = response.get("phases")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|p| Self::parse_phase(p)).collect())
            .unwrap_or_default();

        let tension_curve = response.get("tension_curve").map(|tc| {
            TensionCurve {
                points: tc.get("points")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter().filter_map(|p| {
                            Some(TensionPoint {
                                session: p.get("session").and_then(|v| v.as_u64()).map(|n| n as u32)?,
                                tension: p.get("tension").and_then(|v| v.as_u64()).map(|n| n as u8)?,
                                event: p.get("event").and_then(|v| v.as_str())?.to_string(),
                            })
                        }).collect()
                    })
                    .unwrap_or_default()
            }
        }).unwrap_or_else(TensionCurve::empty);

        let possible_outcomes = response.get("possible_outcomes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|o| {
                    Some(ArcOutcome {
                        name: o.get("name").and_then(|v| v.as_str())?.to_string(),
                        likelihood: match o.get("likelihood").and_then(|v| v.as_str()).unwrap_or("possible") {
                            "likely" => OutcomeLikelihood::Likely,
                            "unlikely" => OutcomeLikelihood::Unlikely,
                            _ => OutcomeLikelihood::Possible,
                        },
                        consequences: o.get("consequences").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    })
                }).collect()
            })
            .unwrap_or_default();

        let antagonist = response.get("antagonist").map(|a| {
            ArcAntagonist {
                name: a.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
                motivation: a.get("motivation").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                resources: Self::parse_string_array(a.get("resources")),
                escalation_plan: Self::parse_string_array(a.get("escalation_plan")),
            }
        });

        Ok(ArcDraft {
            arc,
            phases,
            tension_curve,
            possible_outcomes,
            antagonist,
            raw_data: Some(response.clone()),
        })
    }

    fn parse_phase(phase: &serde_json::Value) -> Option<ArcPhase> {
        Some(ArcPhase {
            name: phase.get("name").and_then(|v| v.as_str())?.to_string(),
            phase_type: match phase.get("type").and_then(|v| v.as_str()).unwrap_or("rising_action") {
                "setup" => PhaseType::Setup,
                "climax" => PhaseType::Climax,
                "falling_action" => PhaseType::FallingAction,
                "resolution" => PhaseType::Resolution,
                _ => PhaseType::RisingAction,
            },
            sessions: phase.get("sessions").and_then(|v| v.as_u64()).map(|n| n as u32).unwrap_or(1),
            tension_level: phase.get("tension_level").and_then(|v| v.as_u64()).map(|n| n as u8).unwrap_or(5),
            objectives: Self::parse_string_array(phase.get("objectives")),
            key_scenes: Self::parse_string_array(phase.get("key_scenes")),
            decision_points: Self::parse_string_array(phase.get("decision_points")),
            npcs_introduced: Self::parse_string_array(phase.get("npcs_introduced")),
            locations: Self::parse_string_array(phase.get("locations")),
        })
    }

    fn parse_string_array(value: Option<&serde_json::Value>) -> Vec<String> {
        value
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default()
    }

    /// Generate a default tension curve for an arc type
    ///
    /// Deduplicates session numbers to ensure each session appears only once,
    /// keeping the later event when duplicates occur.
    pub fn default_tension_curve(arc_type: ArcTemplateType, total_sessions: u32) -> TensionCurve {
        // Guard against zero sessions to prevent division by zero
        if total_sessions == 0 {
            return TensionCurve { points: Vec::new() };
        }

        let raw_points = match arc_type {
            ArcTemplateType::ThreeAct => {
                let mid = total_sessions / 2;
                let climax = (total_sessions * 3) / 4;
                vec![
                    TensionPoint { session: 1, tension: 3, event: "Hook".to_string() },
                    TensionPoint { session: mid.max(2), tension: 6, event: "Midpoint twist".to_string() },
                    TensionPoint { session: climax.max(mid.max(2) + 1), tension: 9, event: "Climax".to_string() },
                    TensionPoint { session: total_sessions, tension: 4, event: "Resolution".to_string() },
                ]
            }
            ArcTemplateType::HerosJourney => {
                let ordeal = (total_sessions / 2).max(3);
                let supreme = ((total_sessions * 3) / 4).max(ordeal + 1);
                vec![
                    TensionPoint { session: 1, tension: 2, event: "Ordinary World".to_string() },
                    TensionPoint { session: 2.min(total_sessions).max(1), tension: 4, event: "Call to Adventure".to_string() },
                    TensionPoint { session: ordeal, tension: 7, event: "Ordeal".to_string() },
                    TensionPoint { session: supreme, tension: 9, event: "Supreme Ordeal".to_string() },
                    TensionPoint { session: total_sessions, tension: 5, event: "Return".to_string() },
                ]
            }
            ArcTemplateType::Mystery => {
                let investigation = (total_sessions / 3).max(2);
                let red_herring = ((total_sessions * 2) / 3).max(investigation + 1);
                let truth = (total_sessions.saturating_sub(1)).max(red_herring + 1);
                vec![
                    TensionPoint { session: 1, tension: 5, event: "The Crime".to_string() },
                    TensionPoint { session: investigation, tension: 4, event: "Investigation begins".to_string() },
                    TensionPoint { session: red_herring, tension: 7, event: "Red herring revealed".to_string() },
                    TensionPoint { session: truth.min(total_sessions.saturating_sub(1)).max(1), tension: 9, event: "Truth discovered".to_string() },
                    TensionPoint { session: total_sessions, tension: 8, event: "Confrontation".to_string() },
                ]
            }
            _ => {
                // Default rising tension
                (1..=total_sessions)
                    .map(|s| {
                        let tension = ((s as f32 / total_sessions as f32) * 8.0 + 2.0) as u8;
                        TensionPoint {
                            session: s,
                            tension: tension.min(10),
                            event: format!("Session {}", s),
                        }
                    })
                    .collect()
            }
        };

        // Deduplicate by session number, keeping later events in case of conflict
        let mut seen = std::collections::HashSet::new();
        let points: Vec<TensionPoint> = raw_points
            .into_iter()
            .rev()
            .filter(|p| seen.insert(p.session))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        TensionCurve { points }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_template_phases() {
        let phases = ArcTemplateType::ThreeAct.default_phases();
        assert_eq!(phases.len(), 3);
        assert_eq!(phases[0], "Setup");

        let phases = ArcTemplateType::HerosJourney.default_phases();
        assert_eq!(phases.len(), 8);
    }

    #[test]
    fn test_arc_generation_request() {
        let request = ArcGenerationRequest::new("A necromancer threatens the kingdom")
            .with_campaign_id("camp-123")
            .with_arc_type(ArcTemplateType::HerosJourney)
            .with_level_range(1, 10)
            .with_sessions("8-12");

        assert_eq!(request.arc_type, ArcTemplateType::HerosJourney);
        assert_eq!(request.level_range, Some((1, 10)));
    }

    #[test]
    fn test_parse_arc_response() {
        let response = serde_json::json!({
            "arc": {
                "title": "The Necromancer's Gambit",
                "tagline": "Death stirs in the forgotten crypts",
                "type": "three_act",
                "themes": ["death", "sacrifice", "redemption"],
                "estimated_sessions": 8,
                "level_range": {"start": 3, "end": 7}
            },
            "phases": [
                {
                    "name": "The Rising Dead",
                    "type": "setup",
                    "sessions": 2,
                    "tension_level": 4,
                    "objectives": ["Discover the undead threat"],
                    "key_scenes": ["First zombie attack"],
                    "decision_points": ["Help the village or flee?"],
                    "npcs_introduced": ["Village Elder"],
                    "locations": ["Grimhollow Village"]
                }
            ],
            "tension_curve": {
                "points": [
                    {"session": 1, "tension": 3, "event": "First signs"},
                    {"session": 6, "tension": 9, "event": "Confronting the Necromancer"}
                ]
            },
            "possible_outcomes": [
                {"name": "Victory", "likelihood": "likely", "consequences": "Kingdom saved"}
            ],
            "antagonist": {
                "name": "Malachar the Undying",
                "motivation": "Achieve immortality at any cost",
                "resources": ["Undead army", "Ancient tome"],
                "escalation_plan": ["Send scouts", "Attack village", "Summon champion"]
            }
        });

        let draft = ArcGenerator::parse_response(&response).unwrap();

        assert_eq!(draft.arc.title, "The Necromancer's Gambit");
        assert_eq!(draft.arc.estimated_sessions, 8);
        assert_eq!(draft.phases.len(), 1);
        assert_eq!(draft.tension_curve.points.len(), 2);
        assert!(draft.antagonist.is_some());
    }

    #[test]
    fn test_tension_curve_methods() {
        let curve = TensionCurve {
            points: vec![
                TensionPoint { session: 1, tension: 3, event: "Start".to_string() },
                TensionPoint { session: 5, tension: 9, event: "Climax".to_string() },
                TensionPoint { session: 7, tension: 5, event: "Resolution".to_string() },
            ],
        };

        assert_eq!(curve.max_tension(), 9);
        assert_eq!(curve.climax_session(), Some(5));
    }

    #[test]
    fn test_default_tension_curve() {
        let curve = ArcGenerator::default_tension_curve(ArcTemplateType::ThreeAct, 8);
        assert!(!curve.points.is_empty());
        assert!(curve.max_tension() >= 8);
    }
}
