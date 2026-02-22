//! NPC Generator
//!
//! Phase 4, Task 4.4: NPC generation with importance levels and stat blocks
//!
//! Generates NPCs with personality, motivations, and optional stat blocks.

use super::orchestrator::{GenerationConfig, GenerationError, GenerationRequest, GenerationType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

/// NPC importance level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NpcImportance {
    /// Background NPCs with minimal detail
    Minor,
    /// Supporting NPCs with moderate detail
    Supporting,
    /// Major NPCs with full detail and stats
    Major,
    /// Key antagonists or allies with extensive detail
    Key,
}

impl Default for NpcImportance {
    fn default() -> Self {
        NpcImportance::Supporting
    }
}

impl NpcImportance {
    /// Whether this importance level should include stat blocks
    pub fn include_stats(&self) -> bool {
        matches!(self, NpcImportance::Major | NpcImportance::Key)
    }

    /// Get the level of detail expected
    pub fn detail_level(&self) -> &'static str {
        match self {
            NpcImportance::Minor => "minimal",
            NpcImportance::Supporting => "moderate",
            NpcImportance::Major => "detailed",
            NpcImportance::Key => "extensive",
        }
    }
}

/// Request for NPC generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcGenerationRequest {
    /// Campaign ID for context
    pub campaign_id: Option<String>,
    /// NPC role (merchant, guard, villain, etc.)
    pub npc_role: String,
    /// NPC importance level
    pub importance: NpcImportance,
    /// Location where NPC is found
    pub location: Option<String>,
    /// Description or requirements
    pub description: String,
    /// Whether to include stat block
    pub include_stats: bool,
    /// Game system for stat blocks
    pub game_system: String,
    /// Generation configuration
    pub config: GenerationConfig,
}

impl NpcGenerationRequest {
    /// Create a new NPC generation request
    pub fn new(
        npc_role: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            campaign_id: None,
            npc_role: npc_role.into(),
            importance: NpcImportance::Supporting,
            location: None,
            description: description.into(),
            include_stats: false,
            game_system: "dnd5e".to_string(),
            config: GenerationConfig::default(),
        }
    }

    /// Set campaign ID
    pub fn with_campaign_id(mut self, campaign_id: impl Into<String>) -> Self {
        self.campaign_id = Some(campaign_id.into());
        self
    }

    /// Set importance level
    pub fn with_importance(mut self, importance: NpcImportance) -> Self {
        self.importance = importance;
        self.include_stats = importance.include_stats();
        self
    }

    /// Set location
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Include stat block
    pub fn with_stats(mut self) -> Self {
        self.include_stats = true;
        self
    }

    /// Set game system
    pub fn with_game_system(mut self, system: impl Into<String>) -> Self {
        self.game_system = system.into();
        self
    }

    /// Convert to a GenerationRequest
    pub fn to_generation_request(self) -> GenerationRequest {
        let mut vars = HashMap::new();
        vars.insert("npc_role".to_string(), self.npc_role);
        vars.insert("importance".to_string(), format!("{:?}", self.importance).to_lowercase());
        vars.insert("description".to_string(), self.description);
        vars.insert("include_stats".to_string(), self.include_stats.to_string());
        vars.insert("game_system".to_string(), self.game_system);

        if let Some(location) = self.location {
            vars.insert("location".to_string(), location);
        }

        let mut request = GenerationRequest::new(GenerationType::Npc)
            .with_variables(vars);

        if let Some(campaign_id) = self.campaign_id {
            request = request.with_campaign_id(campaign_id);
        }

        request.config = self.config;
        request
    }
}

/// Generated NPC draft
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcDraft {
    /// NPC core information
    pub npc: NpcCore,
    /// Stat block (if requested)
    pub stat_block: Option<NpcStatBlock>,
    /// Quest hooks involving this NPC
    pub quest_hooks: Vec<String>,
    /// Raw JSON from generation
    pub raw_data: Option<serde_json::Value>,
}

/// Core NPC information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcCore {
    /// Full name
    pub name: String,
    /// Title or epithet
    pub title: Option<String>,
    /// Race/species
    pub race: String,
    /// Gender identity
    pub gender: String,
    /// Approximate age
    pub age: String,
    /// Occupation
    pub occupation: String,
    /// Physical appearance
    pub appearance: String,
    /// Personality details
    pub personality: NpcPersonality,
    /// Voice and speech
    pub voice: NpcVoice,
    /// Current motivation
    pub motivation: String,
    /// Hidden secret
    pub secret: Option<String>,
    /// Relationships to other NPCs
    pub relationships: Vec<NpcRelationship>,
}

/// NPC personality traits
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NpcPersonality {
    pub traits: Vec<String>,
    pub ideal: String,
    pub bond: String,
    pub flaw: String,
}

/// NPC voice and speech patterns
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NpcVoice {
    pub speech_pattern: String,
    pub catchphrase: Option<String>,
    pub accent: Option<String>,
}

/// Relationship to another NPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcRelationship {
    pub name: String,
    pub relationship_type: String,
}

/// NPC stat block (game-system specific)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcStatBlock {
    /// Challenge rating (D&D) or equivalent
    pub cr: Option<String>,
    /// Creature type
    pub creature_type: Option<String>,
    /// Raw stats as JSON for flexibility
    pub stats: serde_json::Value,
}

// ============================================================================
// NPC Generator
// ============================================================================

/// Generator for NPCs
pub struct NpcGenerator;

impl NpcGenerator {
    /// Parse a generation response into an NpcDraft
    pub fn parse_response(response: &serde_json::Value) -> Result<NpcDraft, GenerationError> {
        let npc_data = response.get("npc").unwrap_or(response);

        let personality = if let Some(p) = npc_data.get("personality") {
            NpcPersonality {
                traits: p
                    .get("traits")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default(),
                ideal: p.get("ideal").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                bond: p.get("bond").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                flaw: p.get("flaw").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            }
        } else {
            NpcPersonality::default()
        };

        let voice = if let Some(v) = npc_data.get("voice") {
            NpcVoice {
                speech_pattern: v.get("speech_pattern").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                catchphrase: v.get("catchphrase").and_then(|v| v.as_str()).map(String::from),
                accent: v.get("accent").and_then(|v| v.as_str()).map(String::from),
            }
        } else {
            NpcVoice::default()
        };

        let relationships = npc_data
            .get("relationships")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| {
                        Some(NpcRelationship {
                            name: r.get("name")?.as_str()?.to_string(),
                            relationship_type: r.get("type").and_then(|v| v.as_str()).unwrap_or("associate").to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let npc = NpcCore {
            name: npc_data.get("name").and_then(|v| v.as_str()).unwrap_or("Unnamed NPC").to_string(),
            title: npc_data.get("title").and_then(|v| v.as_str()).map(String::from),
            race: npc_data.get("race").and_then(|v| v.as_str()).unwrap_or("Human").to_string(),
            gender: npc_data.get("gender").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
            age: npc_data.get("age").and_then(|v| v.as_str()).unwrap_or("Adult").to_string(),
            occupation: npc_data.get("occupation").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            appearance: npc_data.get("appearance").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            personality,
            voice,
            motivation: npc_data.get("motivation").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            secret: npc_data.get("secret").and_then(|v| v.as_str()).map(String::from),
            relationships,
        };

        let stat_block = response.get("stat_block").map(|sb| {
            NpcStatBlock {
                cr: sb.get("cr").and_then(|v| v.as_str()).map(String::from),
                creature_type: sb.get("type").and_then(|v| v.as_str()).map(String::from),
                stats: sb.get("stats").cloned().unwrap_or(serde_json::json!({})),
            }
        });

        let quest_hooks = response
            .get("quest_hooks")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        Ok(NpcDraft {
            npc,
            stat_block,
            quest_hooks,
            raw_data: Some(response.clone()),
        })
    }

    /// Convert an NpcDraft to database-ready format
    pub fn to_database_record(
        draft: &NpcDraft,
        campaign_id: Option<&str>,
    ) -> crate::database::NpcRecord {
        crate::database::NpcRecord {
            id: uuid::Uuid::new_v4().to_string(),
            campaign_id: campaign_id.map(String::from),
            name: draft.npc.name.clone(),
            role: draft.npc.occupation.clone(),
            personality_id: None,
            personality_json: serde_json::to_string(&draft.npc.personality).unwrap_or_default(),
            data_json: draft.raw_data.as_ref().map(|d| d.to_string()),
            stats_json: draft.stat_block.as_ref().map(|sb| serde_json::to_string(sb).unwrap_or_default()),
            notes: None,
            location_id: None,
            voice_profile_id: None,
            quest_hooks: Some(serde_json::to_string(&draft.quest_hooks).unwrap_or_default()),
            created_at: chrono::Utc::now().to_rfc3339(),
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
    fn test_npc_importance_include_stats() {
        assert!(!NpcImportance::Minor.include_stats());
        assert!(!NpcImportance::Supporting.include_stats());
        assert!(NpcImportance::Major.include_stats());
        assert!(NpcImportance::Key.include_stats());
    }

    #[test]
    fn test_npc_generation_request() {
        let request = NpcGenerationRequest::new("tavern keeper", "A gruff dwarf who secretly works for the thieves guild")
            .with_campaign_id("camp-123")
            .with_importance(NpcImportance::Major)
            .with_location("The Rusty Anchor tavern");

        assert_eq!(request.npc_role, "tavern keeper");
        assert_eq!(request.importance, NpcImportance::Major);
        assert!(request.include_stats);
    }

    #[test]
    fn test_to_generation_request() {
        let npc_request = NpcGenerationRequest::new("guard", "A city guard")
            .with_importance(NpcImportance::Minor);

        let gen_request = npc_request.to_generation_request();
        assert_eq!(gen_request.generation_type, GenerationType::Npc);
        assert_eq!(gen_request.variables.get("npc_role"), Some(&"guard".to_string()));
    }

    #[test]
    fn test_parse_npc_response() {
        let response = serde_json::json!({
            "npc": {
                "name": "Grumbar Ironfoot",
                "title": "The Reluctant Keeper",
                "race": "Dwarf",
                "gender": "Male",
                "age": "Middle-aged",
                "occupation": "Tavern keeper",
                "appearance": "Stocky with a braided beard",
                "personality": {
                    "traits": ["Gruff", "Secretly kind"],
                    "ideal": "Freedom",
                    "bond": "His tavern",
                    "flaw": "Trust issues"
                },
                "voice": {
                    "speech_pattern": "Short, clipped sentences",
                    "catchphrase": "What'll it be?",
                    "accent": "Scottish"
                },
                "motivation": "Protect his establishment",
                "secret": "Former thieves guild member",
                "relationships": [
                    {"name": "Silvar", "type": "old friend"}
                ]
            },
            "stat_block": {
                "cr": "1/2",
                "type": "Humanoid",
                "stats": {"str": 14, "dex": 10, "con": 14}
            },
            "quest_hooks": ["Help clear rats from the cellar", "Investigate the guild"]
        });

        let draft = NpcGenerator::parse_response(&response).unwrap();

        assert_eq!(draft.npc.name, "Grumbar Ironfoot");
        assert_eq!(draft.npc.race, "Dwarf");
        assert_eq!(draft.npc.personality.traits.len(), 2);
        assert!(draft.stat_block.is_some());
        assert_eq!(draft.quest_hooks.len(), 2);
    }

    #[test]
    fn test_minimal_response_parsing() {
        let response = serde_json::json!({
            "npc": {
                "name": "Simple Guard"
            }
        });

        let draft = NpcGenerator::parse_response(&response).unwrap();
        assert_eq!(draft.npc.name, "Simple Guard");
        assert_eq!(draft.npc.race, "Human"); // Default
        assert!(draft.stat_block.is_none());
    }
}
