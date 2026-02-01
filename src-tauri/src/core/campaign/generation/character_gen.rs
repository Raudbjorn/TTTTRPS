//! Character Background Generator
//!
//! Phase 4, Task 4.3: Character background generation with NPC/location extraction
//!
//! Generates rich character backstories with integrated NPCs, locations, and plot hooks.

use super::orchestrator::{GenerationConfig, GenerationError, GenerationRequest, GenerationType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

/// Request for character background generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterGenerationRequest {
    /// Campaign ID for context
    pub campaign_id: Option<String>,
    /// Character name
    pub character_name: String,
    /// Character class
    pub character_class: String,
    /// Character race/species
    pub character_race: String,
    /// Character starting level
    pub character_level: Option<u8>,
    /// Player's specific requests
    pub player_request: String,
    /// Additional context or constraints
    pub additional_context: Option<String>,
    /// Generation configuration
    pub config: GenerationConfig,
}

impl CharacterGenerationRequest {
    /// Create a new character generation request
    pub fn new(
        character_name: impl Into<String>,
        character_class: impl Into<String>,
        character_race: impl Into<String>,
        player_request: impl Into<String>,
    ) -> Self {
        Self {
            campaign_id: None,
            character_name: character_name.into(),
            character_class: character_class.into(),
            character_race: character_race.into(),
            character_level: Some(1),
            player_request: player_request.into(),
            additional_context: None,
            config: GenerationConfig::default(),
        }
    }

    /// Set campaign ID
    pub fn with_campaign_id(mut self, campaign_id: impl Into<String>) -> Self {
        self.campaign_id = Some(campaign_id.into());
        self
    }

    /// Set character level
    pub fn with_level(mut self, level: u8) -> Self {
        self.character_level = Some(level);
        self
    }

    /// Add additional context
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.additional_context = Some(context.into());
        self
    }

    /// Convert to a GenerationRequest
    pub fn to_generation_request(self) -> GenerationRequest {
        let mut vars = HashMap::new();
        vars.insert("character_name".to_string(), self.character_name);
        vars.insert("character_class".to_string(), self.character_class);
        vars.insert("character_race".to_string(), self.character_race);
        vars.insert(
            "character_level".to_string(),
            self.character_level.unwrap_or(1).to_string(),
        );
        vars.insert("player_request".to_string(), self.player_request);
        if let Some(ctx) = self.additional_context {
            vars.insert("additional_context".to_string(), ctx);
        }

        let mut request = GenerationRequest::new(GenerationType::CharacterBackground)
            .with_variables(vars);

        if let Some(campaign_id) = self.campaign_id {
            request = request.with_campaign_id(campaign_id);
        }

        request.config = self.config;
        request
    }
}

/// Generated character background draft
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterDraft {
    /// Generated background summary
    pub background: CharacterBackground,
    /// Extracted relationships (potential NPCs)
    pub relationships: Vec<ExtractedRelationship>,
    /// Extracted locations
    pub locations: Vec<ExtractedLocation>,
    /// Plot hooks for the GM
    pub plot_hooks: Vec<PlotHook>,
    /// Secrets the character has
    pub secrets: Vec<String>,
    /// Raw JSON from generation
    pub raw_data: Option<serde_json::Value>,
}

/// The core character background
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterBackground {
    /// Brief summary
    pub summary: String,
    /// Origin story
    pub origin: String,
    /// Key formative event
    pub formative_event: String,
    /// Current motivation
    pub motivation: String,
    /// Personality traits
    pub personality_traits: Vec<String>,
    /// Guiding ideal
    pub ideal: String,
    /// Important bond
    pub bond: String,
    /// Character flaw
    pub flaw: String,
}

impl Default for CharacterBackground {
    fn default() -> Self {
        Self {
            summary: String::new(),
            origin: String::new(),
            formative_event: String::new(),
            motivation: String::new(),
            personality_traits: Vec::new(),
            ideal: String::new(),
            bond: String::new(),
            flaw: String::new(),
        }
    }
}

/// An extracted entity (NPC or location) from generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    /// Entity type (npc, location)
    pub entity_type: String,
    /// Entity name
    pub name: String,
    /// Brief description
    pub description: Option<String>,
    /// Relationship to the character (for NPCs)
    pub relationship: Option<String>,
    /// Significance in the backstory
    pub significance: String,
    /// Whether this entity could be recurring
    pub recurring: bool,
    /// Whether to create as a draft
    pub should_create: bool,
}

/// A relationship extracted from the backstory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelationship {
    /// NPC name
    pub name: String,
    /// Relationship type (friend, mentor, rival, family, etc.)
    pub relationship_type: String,
    /// Current status (alive, dead, unknown)
    pub status: String,
    /// Brief description
    pub description: String,
    /// Whether this NPC has plot hook potential
    pub plot_hook_potential: bool,
}

/// A location mentioned in the backstory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedLocation {
    /// Location name
    pub name: String,
    /// Why it's significant
    pub significance: String,
    /// Whether the character could return
    pub revisitable: bool,
}

/// A plot hook from the backstory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotHook {
    /// Hook title
    pub title: String,
    /// Description of what could happen
    pub description: String,
    /// Urgency level
    pub urgency: PlotHookUrgency,
}

/// Urgency level for a plot hook
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlotHookUrgency {
    Low,
    Medium,
    High,
}

impl Default for PlotHookUrgency {
    fn default() -> Self {
        PlotHookUrgency::Medium
    }
}

// ============================================================================
// Character Generator
// ============================================================================

/// Generator for character backgrounds
pub struct CharacterGenerator;

impl CharacterGenerator {
    /// Parse a generation response into a CharacterDraft
    pub fn parse_response(
        response: &serde_json::Value,
    ) -> Result<CharacterDraft, GenerationError> {
        // Parse background
        let background = if let Some(bg) = response.get("background") {
            CharacterBackground {
                summary: bg
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                origin: bg
                    .get("origin")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                formative_event: bg
                    .get("formative_event")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                motivation: bg
                    .get("motivation")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                personality_traits: bg
                    .get("personality_traits")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                ideal: bg
                    .get("ideal")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                bond: bg
                    .get("bond")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                flaw: bg
                    .get("flaw")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            }
        } else {
            CharacterBackground::default()
        };

        // Parse relationships
        let relationships = response
            .get("relationships")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| {
                        Some(ExtractedRelationship {
                            name: r.get("name")?.as_str()?.to_string(),
                            relationship_type: r
                                .get("relationship")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            status: r
                                .get("status")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            description: r
                                .get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            plot_hook_potential: r
                                .get("plot_hook_potential")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Parse locations
        let locations = response
            .get("locations")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|l| {
                        Some(ExtractedLocation {
                            name: l.get("name")?.as_str()?.to_string(),
                            significance: l
                                .get("significance")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            revisitable: l
                                .get("revisitable")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(true),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Parse plot hooks
        let plot_hooks = response
            .get("plot_hooks")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|h| {
                        Some(PlotHook {
                            title: h.get("title")?.as_str()?.to_string(),
                            description: h
                                .get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            urgency: match h
                                .get("urgency")
                                .and_then(|v| v.as_str())
                                .unwrap_or("medium")
                            {
                                "low" => PlotHookUrgency::Low,
                                "high" => PlotHookUrgency::High,
                                _ => PlotHookUrgency::Medium,
                            },
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Parse secrets
        let secrets = response
            .get("secrets")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(CharacterDraft {
            background,
            relationships,
            locations,
            plot_hooks,
            secrets,
            raw_data: Some(response.clone()),
        })
    }

    /// Extract potential entities to create from the draft
    pub fn extract_entities(draft: &CharacterDraft) -> Vec<ExtractedEntity> {
        let mut entities = Vec::new();

        // Extract NPCs from relationships
        for rel in &draft.relationships {
            entities.push(ExtractedEntity {
                entity_type: "npc".to_string(),
                name: rel.name.clone(),
                description: Some(rel.description.clone()),
                relationship: Some(rel.relationship_type.clone()),
                significance: format!("{} - {}", rel.relationship_type, rel.status),
                recurring: rel.plot_hook_potential,
                should_create: rel.plot_hook_potential && {
                    let s = rel.status.to_lowercase();
                    s != "dead" && s != "deceased"
                },
            });
        }

        // Extract locations
        for loc in &draft.locations {
            entities.push(ExtractedEntity {
                entity_type: "location".to_string(),
                name: loc.name.clone(),
                description: Some(loc.significance.clone()),
                relationship: None,
                significance: loc.significance.clone(),
                recurring: loc.revisitable,
                should_create: loc.revisitable,
            });
        }

        entities
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_generation_request() {
        let request = CharacterGenerationRequest::new(
            "Elara Nightwood",
            "Ranger",
            "Half-Elf",
            "A mysterious past involving a secret organization",
        )
        .with_campaign_id("camp-123")
        .with_level(5);

        assert_eq!(request.character_name, "Elara Nightwood");
        assert_eq!(request.character_level, Some(5));
        assert!(request.campaign_id.is_some());
    }

    #[test]
    fn test_to_generation_request() {
        let char_request = CharacterGenerationRequest::new(
            "Test",
            "Fighter",
            "Human",
            "Simple backstory",
        )
        .with_level(3);

        let gen_request = char_request.to_generation_request();
        assert_eq!(gen_request.generation_type, GenerationType::CharacterBackground);
        assert_eq!(gen_request.variables.get("character_name"), Some(&"Test".to_string()));
        assert_eq!(gen_request.variables.get("character_level"), Some(&"3".to_string()));
    }

    #[test]
    fn test_parse_response() {
        let response = serde_json::json!({
            "background": {
                "summary": "A troubled ranger seeking redemption",
                "origin": "Born in the wilderness",
                "formative_event": "Lost their mentor to evil forces",
                "motivation": "To protect the innocent",
                "personality_traits": ["Cautious", "Loyal"],
                "ideal": "Justice",
                "bond": "Their forest home",
                "flaw": "Distrusts civilization"
            },
            "relationships": [
                {
                    "name": "Old Mentor",
                    "relationship": "mentor",
                    "status": "dead",
                    "description": "Taught them everything",
                    "plot_hook_potential": true
                }
            ],
            "locations": [
                {
                    "name": "The Whispering Woods",
                    "significance": "Where they grew up",
                    "revisitable": true
                }
            ],
            "plot_hooks": [
                {
                    "title": "The Mentor's Legacy",
                    "description": "Discover what happened to the mentor",
                    "urgency": "medium"
                }
            ],
            "secrets": ["Has a twin sibling they never knew about"]
        });

        let draft = CharacterGenerator::parse_response(&response).unwrap();

        assert_eq!(draft.background.summary, "A troubled ranger seeking redemption");
        assert_eq!(draft.relationships.len(), 1);
        assert_eq!(draft.relationships[0].name, "Old Mentor");
        assert_eq!(draft.locations.len(), 1);
        assert_eq!(draft.plot_hooks.len(), 1);
        assert_eq!(draft.secrets.len(), 1);
    }

    #[test]
    fn test_extract_entities() {
        let draft = CharacterDraft {
            background: CharacterBackground::default(),
            relationships: vec![ExtractedRelationship {
                name: "Mentor".to_string(),
                relationship_type: "mentor".to_string(),
                status: "alive".to_string(),
                description: "A wise teacher".to_string(),
                plot_hook_potential: true,
            }],
            locations: vec![ExtractedLocation {
                name: "Home Village".to_string(),
                significance: "Where they grew up".to_string(),
                revisitable: true,
            }],
            plot_hooks: vec![],
            secrets: vec![],
            raw_data: None,
        };

        let entities = CharacterGenerator::extract_entities(&draft);
        assert_eq!(entities.len(), 2);

        let npc = &entities[0];
        assert_eq!(npc.entity_type, "npc");
        assert_eq!(npc.name, "Mentor");
        assert!(npc.should_create);

        let location = &entities[1];
        assert_eq!(location.entity_type, "location");
        assert_eq!(location.name, "Home Village");
    }
}
