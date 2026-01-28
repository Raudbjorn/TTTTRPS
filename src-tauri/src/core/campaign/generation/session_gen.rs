//! Session Plan Generator
//!
//! Phase 4, Task 4.5: Session plan generation with encounter difficulty
//!
//! Generates structured session plans with pacing, encounters, and narrative beats.

use super::orchestrator::{GenerationConfig, GenerationError, GenerationRequest, GenerationType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

/// Pacing template for sessions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PacingTemplate {
    /// Heavy on combat encounters
    CombatHeavy,
    /// Focused on roleplay and social encounters
    RoleplayFocused,
    /// Emphasis on exploration and discovery
    Exploration,
    /// Balanced mix of all elements
    Mixed,
    /// High tension, dramatic confrontations
    Dramatic,
    /// Puzzle and mystery solving
    Mystery,
}

impl Default for PacingTemplate {
    fn default() -> Self {
        PacingTemplate::Mixed
    }
}

impl PacingTemplate {
    /// Get the expected encounter distribution
    pub fn encounter_distribution(&self) -> EncounterDistribution {
        match self {
            PacingTemplate::CombatHeavy => EncounterDistribution {
                combat: 60,
                social: 20,
                exploration: 10,
                puzzle: 10,
            },
            PacingTemplate::RoleplayFocused => EncounterDistribution {
                combat: 15,
                social: 55,
                exploration: 15,
                puzzle: 15,
            },
            PacingTemplate::Exploration => EncounterDistribution {
                combat: 20,
                social: 20,
                exploration: 45,
                puzzle: 15,
            },
            PacingTemplate::Mixed => EncounterDistribution {
                combat: 30,
                social: 30,
                exploration: 20,
                puzzle: 20,
            },
            PacingTemplate::Dramatic => EncounterDistribution {
                combat: 40,
                social: 35,
                exploration: 10,
                puzzle: 15,
            },
            PacingTemplate::Mystery => EncounterDistribution {
                combat: 15,
                social: 30,
                exploration: 20,
                puzzle: 35,
            },
        }
    }
}

/// Distribution of encounter types (percentages)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncounterDistribution {
    pub combat: u8,
    pub social: u8,
    pub exploration: u8,
    pub puzzle: u8,
}

/// Encounter difficulty level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EncounterDifficulty {
    Easy,
    Medium,
    Hard,
    Deadly,
}

impl Default for EncounterDifficulty {
    fn default() -> Self {
        EncounterDifficulty::Medium
    }
}

/// Request for session plan generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionGenerationRequest {
    /// Campaign ID for context
    pub campaign_id: Option<String>,
    /// Expected session duration in hours
    pub session_duration_hours: f32,
    /// Pacing template
    pub pacing_style: PacingTemplate,
    /// Main session objective
    pub objective: String,
    /// Summary of previous session
    pub previous_session: Option<String>,
    /// Currently active plot threads
    pub active_plots: Vec<String>,
    /// GM notes or specific requests
    pub gm_notes: Option<String>,
    /// Party level for encounter balancing
    pub party_level: Option<u8>,
    /// Party size for encounter balancing
    pub party_size: Option<u8>,
    /// Generation configuration
    pub config: GenerationConfig,
}

impl SessionGenerationRequest {
    /// Create a new session plan request
    pub fn new(objective: impl Into<String>) -> Self {
        Self {
            campaign_id: None,
            session_duration_hours: 3.0,
            pacing_style: PacingTemplate::Mixed,
            objective: objective.into(),
            previous_session: None,
            active_plots: Vec::new(),
            gm_notes: None,
            party_level: None,
            party_size: None,
            config: GenerationConfig::default(),
        }
    }

    /// Set campaign ID
    pub fn with_campaign_id(mut self, campaign_id: impl Into<String>) -> Self {
        self.campaign_id = Some(campaign_id.into());
        self
    }

    /// Set session duration
    pub fn with_duration(mut self, hours: f32) -> Self {
        self.session_duration_hours = hours;
        self
    }

    /// Set pacing style
    pub fn with_pacing(mut self, pacing: PacingTemplate) -> Self {
        self.pacing_style = pacing;
        self
    }

    /// Set previous session summary
    pub fn with_previous_session(mut self, summary: impl Into<String>) -> Self {
        self.previous_session = Some(summary.into());
        self
    }

    /// Add active plot threads
    pub fn with_plots(mut self, plots: Vec<String>) -> Self {
        self.active_plots = plots;
        self
    }

    /// Set party info for encounter balancing
    pub fn with_party(mut self, level: u8, size: u8) -> Self {
        self.party_level = Some(level);
        self.party_size = Some(size);
        self
    }

    /// Convert to a GenerationRequest
    pub fn to_generation_request(self) -> GenerationRequest {
        let mut vars = HashMap::new();
        vars.insert("session_duration".to_string(), self.session_duration_hours.to_string());
        vars.insert("pacing_style".to_string(), format!("{:?}", self.pacing_style).to_lowercase());
        vars.insert("objective".to_string(), self.objective);

        if let Some(prev) = self.previous_session {
            vars.insert("previous_session".to_string(), prev);
        }
        if !self.active_plots.is_empty() {
            vars.insert("active_plots".to_string(), self.active_plots.join("\n- "));
        }
        if let Some(notes) = self.gm_notes {
            vars.insert("gm_notes".to_string(), notes);
        }
        if let Some(level) = self.party_level {
            vars.insert("party_level".to_string(), level.to_string());
        }
        if let Some(size) = self.party_size {
            vars.insert("party_size".to_string(), size.to_string());
        }

        let mut request = GenerationRequest::new(GenerationType::SessionPlan)
            .with_variables(vars);

        if let Some(campaign_id) = self.campaign_id {
            request = request.with_campaign_id(campaign_id);
        }

        request.config = self.config;
        request
    }
}

/// Generated session plan draft
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPlanDraft {
    /// Session plan details
    pub plan: SessionPlan,
    /// Raw JSON from generation
    pub raw_data: Option<serde_json::Value>,
}

/// A complete session plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPlan {
    /// Session title
    pub title: String,
    /// Primary objective
    pub objective: String,
    /// Estimated duration in hours
    pub estimated_duration_hours: f32,
    /// Narrative beats
    pub beats: Vec<SessionBeat>,
    /// NPCs involved
    pub npcs_involved: Vec<String>,
    /// Locations used
    pub locations: Vec<String>,
    /// Potential rewards
    pub loot_rewards: Vec<String>,
    /// How this advances the story
    pub plot_advancement: String,
    /// Possible cliffhanger endings
    pub cliffhanger_options: Vec<String>,
}

/// A single beat/scene in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionBeat {
    /// Beat name
    pub name: String,
    /// Beat type in narrative structure
    pub beat_type: BeatType,
    /// Estimated duration in minutes
    pub duration_minutes: u32,
    /// Description of what happens
    pub description: String,
    /// Encounter details if applicable
    pub encounter: Option<EncounterDetails>,
    /// Contingencies for player choices
    pub contingencies: Vec<String>,
}

/// Type of narrative beat
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeatType {
    Opening,
    RisingAction,
    Climax,
    FallingAction,
    Cliffhanger,
}

/// Details of an encounter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncounterDetails {
    /// Type of encounter
    pub encounter_type: EncounterType,
    /// Difficulty level
    pub difficulty: EncounterDifficulty,
    /// Participants (NPCs, monsters)
    pub participants: Vec<String>,
    /// Environmental factors
    pub environment: Option<String>,
}

/// Type of encounter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncounterType {
    Combat,
    Social,
    Exploration,
    Puzzle,
}

// ============================================================================
// Session Generator
// ============================================================================

/// Generator for session plans
pub struct SessionGenerator;

impl SessionGenerator {
    /// Parse a generation response into a SessionPlanDraft
    pub fn parse_response(response: &serde_json::Value) -> Result<SessionPlanDraft, GenerationError> {
        let plan_data = response.get("session_plan").unwrap_or(response);

        let beats = plan_data
            .get("beats")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|b| Self::parse_beat(b))
                    .collect()
            })
            .unwrap_or_default();

        let plan = SessionPlan {
            title: plan_data.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled Session").to_string(),
            objective: plan_data.get("objective").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            estimated_duration_hours: plan_data.get("estimated_duration_hours")
                .and_then(|v| v.as_f64())
                .map(|f| f as f32)
                .unwrap_or(3.0),
            beats,
            npcs_involved: Self::parse_string_array(plan_data.get("npcs_involved")),
            locations: Self::parse_string_array(plan_data.get("locations")),
            loot_rewards: Self::parse_string_array(plan_data.get("loot_rewards")),
            plot_advancement: plan_data.get("plot_advancement").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            cliffhanger_options: Self::parse_string_array(plan_data.get("cliffhanger_options")),
        };

        Ok(SessionPlanDraft {
            plan,
            raw_data: Some(response.clone()),
        })
    }

    fn parse_beat(beat: &serde_json::Value) -> Option<SessionBeat> {
        let encounter = beat.get("encounter").and_then(|e| {
            Some(EncounterDetails {
                encounter_type: match e.get("type").and_then(|v| v.as_str()).unwrap_or("combat") {
                    "social" => EncounterType::Social,
                    "exploration" => EncounterType::Exploration,
                    "puzzle" => EncounterType::Puzzle,
                    _ => EncounterType::Combat,
                },
                difficulty: match e.get("difficulty").and_then(|v| v.as_str()).unwrap_or("medium") {
                    "easy" => EncounterDifficulty::Easy,
                    "hard" => EncounterDifficulty::Hard,
                    "deadly" => EncounterDifficulty::Deadly,
                    _ => EncounterDifficulty::Medium,
                },
                participants: Self::parse_string_array(e.get("participants")),
                environment: e.get("environment").and_then(|v| v.as_str()).map(String::from),
            })
        });

        Some(SessionBeat {
            name: beat.get("name").and_then(|v| v.as_str())?.to_string(),
            beat_type: match beat.get("type").and_then(|v| v.as_str()).unwrap_or("rising_action") {
                "opening" => BeatType::Opening,
                "climax" => BeatType::Climax,
                "falling_action" => BeatType::FallingAction,
                "cliffhanger" => BeatType::Cliffhanger,
                _ => BeatType::RisingAction,
            },
            duration_minutes: beat.get("duration_minutes")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32)
                .unwrap_or(30),
            description: beat.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            encounter,
            contingencies: Self::parse_string_array(beat.get("contingencies")),
        })
    }

    fn parse_string_array(value: Option<&serde_json::Value>) -> Vec<String> {
        value
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default()
    }

    /// Calculate encounter difficulty based on party composition
    pub fn calculate_encounter_difficulty(
        party_level: u8,
        party_size: u8,
        enemy_cr: f32,
        enemy_count: u8,
    ) -> EncounterDifficulty {
        // Guard against division by zero
        if party_level == 0 || party_size == 0 {
            return EncounterDifficulty::Deadly;
        }

        // Simplified D&D 5e encounter difficulty calculation
        let party_power = (party_level as f32) * (party_size as f32) * 100.0;
        let enemy_power = enemy_cr * (enemy_count as f32) * 200.0;
        let ratio = enemy_power / party_power;

        if ratio < 0.5 {
            EncounterDifficulty::Easy
        } else if ratio < 1.0 {
            EncounterDifficulty::Medium
        } else if ratio < 1.5 {
            EncounterDifficulty::Hard
        } else {
            EncounterDifficulty::Deadly
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pacing_template_distribution() {
        let dist = PacingTemplate::CombatHeavy.encounter_distribution();
        assert_eq!(dist.combat, 60);
        assert!(dist.combat > dist.social);

        let dist = PacingTemplate::RoleplayFocused.encounter_distribution();
        assert!(dist.social > dist.combat);
    }

    #[test]
    fn test_session_generation_request() {
        let request = SessionGenerationRequest::new("Infiltrate the noble's mansion")
            .with_campaign_id("camp-123")
            .with_duration(4.0)
            .with_pacing(PacingTemplate::Mystery)
            .with_party(5, 4);

        assert_eq!(request.session_duration_hours, 4.0);
        assert_eq!(request.pacing_style, PacingTemplate::Mystery);
        assert_eq!(request.party_level, Some(5));
    }

    #[test]
    fn test_parse_session_response() {
        let response = serde_json::json!({
            "session_plan": {
                "title": "The Heist",
                "objective": "Steal the artifact",
                "estimated_duration_hours": 4,
                "beats": [
                    {
                        "name": "The Setup",
                        "type": "opening",
                        "duration_minutes": 30,
                        "description": "Party plans the heist",
                        "contingencies": ["What if they want to go in loud?"]
                    },
                    {
                        "name": "The Confrontation",
                        "type": "climax",
                        "duration_minutes": 60,
                        "description": "Face the final guardian",
                        "encounter": {
                            "type": "combat",
                            "difficulty": "hard",
                            "participants": ["Stone Golem", "Guard Captain"]
                        }
                    }
                ],
                "npcs_involved": ["Lord Ashford", "Captain Vance"],
                "locations": ["Ashford Manor", "The Vault"],
                "plot_advancement": "Reveals the location of the second artifact"
            }
        });

        let draft = SessionGenerator::parse_response(&response).unwrap();

        assert_eq!(draft.plan.title, "The Heist");
        assert_eq!(draft.plan.beats.len(), 2);
        assert_eq!(draft.plan.beats[0].beat_type, BeatType::Opening);
        assert!(draft.plan.beats[1].encounter.is_some());
    }

    #[test]
    fn test_encounter_difficulty_calculation() {
        // Easy: weak enemies vs strong party
        let diff = SessionGenerator::calculate_encounter_difficulty(5, 4, 0.5, 2);
        assert_eq!(diff, EncounterDifficulty::Easy);

        // Deadly: strong enemies vs weak party
        let diff = SessionGenerator::calculate_encounter_difficulty(3, 3, 5.0, 2);
        assert_eq!(diff, EncounterDifficulty::Deadly);
    }
}
