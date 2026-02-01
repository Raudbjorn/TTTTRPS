//! Party Composition Analyzer
//!
//! Phase 4, Task 4.6: Party composition suggestions with gap analysis
//!
//! Analyzes party composition and provides suggestions for addressing gaps.

use super::orchestrator::{GenerationConfig, GenerationError, GenerationRequest, GenerationType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

/// Request for party composition analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyAnalysisRequest {
    /// Campaign ID for context
    pub campaign_id: Option<String>,
    /// Party member details
    pub party_details: Vec<PartyMember>,
    /// Expected campaign type
    pub campaign_type: Option<String>,
    /// Expected challenge types
    pub expected_challenges: Vec<String>,
    /// Game system
    pub game_system: String,
    /// Generation configuration
    pub config: GenerationConfig,
}

/// A party member for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyMember {
    /// Character name (optional)
    pub name: Option<String>,
    /// Character class
    pub class: String,
    /// Subclass if known
    pub subclass: Option<String>,
    /// Character level
    pub level: u8,
    /// Primary role
    pub role: Option<PartyRole>,
}

/// Role in the party
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyRole {
    Tank,
    Healer,
    DamageDealer,
    Support,
    Controller,
    Utility,
    Face,
    Scout,
}

impl PartyAnalysisRequest {
    /// Create a new party analysis request
    pub fn new(party: Vec<PartyMember>) -> Self {
        Self {
            campaign_id: None,
            party_details: party,
            campaign_type: None,
            expected_challenges: vec!["combat".to_string(), "exploration".to_string(), "social".to_string()],
            game_system: "dnd5e".to_string(),
            config: GenerationConfig::default(),
        }
    }

    /// Set campaign ID
    pub fn with_campaign_id(mut self, campaign_id: impl Into<String>) -> Self {
        self.campaign_id = Some(campaign_id.into());
        self
    }

    /// Set campaign type
    pub fn with_campaign_type(mut self, campaign_type: impl Into<String>) -> Self {
        self.campaign_type = Some(campaign_type.into());
        self
    }

    /// Set expected challenges
    pub fn with_challenges(mut self, challenges: Vec<String>) -> Self {
        self.expected_challenges = challenges;
        self
    }

    /// Convert to a GenerationRequest
    pub fn to_generation_request(self) -> GenerationRequest {
        let party_desc: Vec<String> = self.party_details.iter().map(|m| {
            let mut desc = format!("{} {}", m.class, m.level);
            if let Some(ref subclass) = m.subclass {
                desc = format!("{} ({})", desc, subclass);
            }
            if let Some(ref name) = m.name {
                desc = format!("{}: {}", name, desc);
            }
            if let Some(role) = m.role {
                desc = format!("{} - {:?}", desc, role);
            }
            desc
        }).collect();

        let mut vars = HashMap::new();
        vars.insert("party_details".to_string(), party_desc.join("\n"));
        vars.insert("game_system".to_string(), self.game_system);

        if let Some(ct) = self.campaign_type {
            vars.insert("campaign_type".to_string(), ct);
        }
        if !self.expected_challenges.is_empty() {
            vars.insert("expected_challenges".to_string(), self.expected_challenges.join(", "));
        }

        let mut request = GenerationRequest::new(GenerationType::PartyAnalysis)
            .with_variables(vars);

        if let Some(campaign_id) = self.campaign_id {
            request = request.with_campaign_id(campaign_id);
        }

        request.config = self.config;
        request
    }
}

/// Party analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapAnalysis {
    /// Overall balance score (0-100)
    pub overall_balance_score: u8,
    /// Party strengths
    pub strengths: Vec<String>,
    /// Party weaknesses/gaps
    pub weaknesses: Vec<String>,
    /// Combat analysis
    pub combat_analysis: CombatAnalysis,
    /// Utility analysis
    pub utility_analysis: UtilityAnalysis,
    /// Recommendations to address gaps
    pub recommendations: Vec<GapRecommendation>,
}

/// Combat capability analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatAnalysis {
    /// Damage output assessment
    pub damage_output: CapabilityLevel,
    /// Survivability assessment
    pub survivability: CapabilityLevel,
    /// Battlefield control
    pub control: CapabilityLevel,
    /// Notes
    pub notes: Option<String>,
}

/// Utility capability analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilityAnalysis {
    /// Healing capability
    pub healing: CapabilityLevel,
    /// Exploration capability
    pub exploration: CapabilityLevel,
    /// Social capability
    pub social: CapabilityLevel,
    /// Notes
    pub notes: Option<String>,
}

/// Capability level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CapabilityLevel {
    None,
    Limited,
    Adequate,
    Strong,
}

impl Default for CapabilityLevel {
    fn default() -> Self {
        CapabilityLevel::Adequate
    }
}

/// A recommendation to address a gap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapRecommendation {
    /// What gap this addresses
    pub gap: String,
    /// Priority level
    pub priority: RecommendationPriority,
    /// Possible solutions
    pub solutions: Vec<GapSolution>,
}

/// Priority for a recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
}

/// A possible solution to a gap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapSolution {
    /// Type of solution
    pub solution_type: SolutionType,
    /// Specific suggestion
    pub suggestion: String,
}

/// Type of gap solution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SolutionType {
    /// Add an NPC companion
    NpcCompanion,
    /// Provide a magic item
    MagicItem,
    /// Adjust encounter design
    EncounterDesign,
    /// Player multiclass or feat
    PlayerBuild,
    /// Consumable items
    Consumables,
}

/// Suggestion from party analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartySuggestion {
    /// The gap analysis
    pub analysis: GapAnalysis,
    /// Raw data from generation
    pub raw_data: Option<serde_json::Value>,
}

// ============================================================================
// Party Analyzer
// ============================================================================

/// Analyzer for party composition
pub struct PartyAnalyzer;

impl PartyAnalyzer {
    /// Parse a generation response into a PartySuggestion
    pub fn parse_response(response: &serde_json::Value) -> Result<PartySuggestion, GenerationError> {
        let analysis_data = response.get("analysis").unwrap_or(response);

        let combat_analysis = if let Some(ca) = analysis_data.get("combat_analysis") {
            CombatAnalysis {
                damage_output: Self::parse_capability(ca.get("damage_output")),
                survivability: Self::parse_capability(ca.get("survivability")),
                control: Self::parse_capability(ca.get("control")),
                notes: ca.get("notes").and_then(|v| v.as_str()).map(String::from),
            }
        } else {
            CombatAnalysis {
                damage_output: CapabilityLevel::Adequate,
                survivability: CapabilityLevel::Adequate,
                control: CapabilityLevel::Adequate,
                notes: None,
            }
        };

        let utility_analysis = if let Some(ua) = analysis_data.get("utility_analysis") {
            UtilityAnalysis {
                healing: Self::parse_capability(ua.get("healing")),
                exploration: Self::parse_capability(ua.get("exploration")),
                social: Self::parse_capability(ua.get("social")),
                notes: ua.get("notes").and_then(|v| v.as_str()).map(String::from),
            }
        } else {
            UtilityAnalysis {
                healing: CapabilityLevel::Adequate,
                exploration: CapabilityLevel::Adequate,
                social: CapabilityLevel::Adequate,
                notes: None,
            }
        };

        let recommendations = response
            .get("recommendations")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|r| Self::parse_recommendation(r)).collect())
            .unwrap_or_default();

        let analysis = GapAnalysis {
            overall_balance_score: analysis_data
                .get("overall_balance_score")
                .and_then(|v| v.as_u64())
                .map(|n| n.min(100) as u8)
                .unwrap_or(50),
            strengths: Self::parse_string_array(analysis_data.get("strengths")),
            weaknesses: Self::parse_string_array(analysis_data.get("weaknesses")),
            combat_analysis,
            utility_analysis,
            recommendations,
        };

        Ok(PartySuggestion {
            analysis,
            raw_data: Some(response.clone()),
        })
    }

    fn parse_capability(value: Option<&serde_json::Value>) -> CapabilityLevel {
        value
            .and_then(|v| v.as_str())
            .map(|s| match s.to_lowercase().as_str() {
                "none" => CapabilityLevel::None,
                "limited" => CapabilityLevel::Limited,
                "strong" => CapabilityLevel::Strong,
                _ => CapabilityLevel::Adequate,
            })
            .unwrap_or(CapabilityLevel::Adequate)
    }

    fn parse_recommendation(value: &serde_json::Value) -> Option<GapRecommendation> {
        let solutions = value
            .get("solutions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| {
                        Some(GapSolution {
                            solution_type: match s.get("type").and_then(|v| v.as_str()).unwrap_or("encounter_design") {
                                "npc_companion" => SolutionType::NpcCompanion,
                                "magic_item" => SolutionType::MagicItem,
                                "player_build" => SolutionType::PlayerBuild,
                                "consumables" => SolutionType::Consumables,
                                _ => SolutionType::EncounterDesign,
                            },
                            suggestion: s.get("suggestion").and_then(|v| v.as_str())?.to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Some(GapRecommendation {
            gap: value.get("gap").and_then(|v| v.as_str())?.to_string(),
            priority: match value.get("priority").and_then(|v| v.as_str()).unwrap_or("medium") {
                "high" => RecommendationPriority::High,
                "low" => RecommendationPriority::Low,
                _ => RecommendationPriority::Medium,
            },
            solutions,
        })
    }

    fn parse_string_array(value: Option<&serde_json::Value>) -> Vec<String> {
        value
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default()
    }

    /// Perform basic static analysis of party composition
    pub fn static_analysis(party: &[PartyMember]) -> GapAnalysis {
        let mut has_tank = false;
        let mut has_healer = false;
        let mut has_damage = false;
        let mut has_utility = false;
        let mut has_face = false;

        for member in party {
            // Infer roles from class names (D&D 5e centric)
            let class_lower = member.class.to_lowercase();
            match class_lower.as_str() {
                "fighter" | "barbarian" | "paladin" => {
                    has_tank = true;
                    has_damage = true;
                }
                "cleric" | "druid" => {
                    has_healer = true;
                    has_utility = true;
                }
                "rogue" | "ranger" => {
                    has_damage = true;
                    has_utility = true;
                }
                "wizard" | "sorcerer" | "warlock" => has_damage = true,
                "bard" => {
                    has_healer = true;
                    has_face = true;
                    has_utility = true;
                }
                "monk" => has_damage = true,
                _ => {}
            }

            // Also check explicit role if set
            if let Some(role) = member.role {
                match role {
                    PartyRole::Tank => has_tank = true,
                    PartyRole::Healer => has_healer = true,
                    PartyRole::DamageDealer => has_damage = true,
                    PartyRole::Support | PartyRole::Controller => has_utility = true,
                    PartyRole::Face => has_face = true,
                    PartyRole::Utility | PartyRole::Scout => has_utility = true,
                }
            }
        }

        let mut strengths = Vec::new();
        let mut weaknesses = Vec::new();
        let mut recommendations = Vec::new();

        if has_tank {
            strengths.push("Frontline presence".to_string());
        } else {
            weaknesses.push("No dedicated tank".to_string());
            recommendations.push(GapRecommendation {
                gap: "No tank".to_string(),
                priority: RecommendationPriority::High,
                solutions: vec![
                    GapSolution {
                        solution_type: SolutionType::NpcCompanion,
                        suggestion: "Add a fighter or paladin NPC companion".to_string(),
                    },
                    GapSolution {
                        solution_type: SolutionType::EncounterDesign,
                        suggestion: "Avoid enemies with high sustained damage".to_string(),
                    },
                ],
            });
        }

        if has_healer {
            strengths.push("In-combat healing".to_string());
        } else {
            weaknesses.push("Limited healing".to_string());
            recommendations.push(GapRecommendation {
                gap: "No healer".to_string(),
                priority: RecommendationPriority::High,
                solutions: vec![
                    GapSolution {
                        solution_type: SolutionType::Consumables,
                        suggestion: "Provide healing potions liberally".to_string(),
                    },
                    GapSolution {
                        solution_type: SolutionType::MagicItem,
                        suggestion: "Give party a Staff of Healing or similar".to_string(),
                    },
                ],
            });
        }

        if has_damage {
            strengths.push("Good damage output".to_string());
        }

        // Calculate score
        let mut score = 50u8;
        if has_tank { score += 15; }
        if has_healer { score += 15; }
        if has_damage { score += 10; }
        if has_utility { score += 5; }
        if has_face { score += 5; }
        score = score.min(100);

        GapAnalysis {
            overall_balance_score: score,
            strengths,
            weaknesses,
            combat_analysis: CombatAnalysis {
                damage_output: if has_damage { CapabilityLevel::Adequate } else { CapabilityLevel::Limited },
                survivability: if has_tank && has_healer { CapabilityLevel::Strong } else if has_tank || has_healer { CapabilityLevel::Adequate } else { CapabilityLevel::Limited },
                control: CapabilityLevel::Adequate,
                notes: None,
            },
            utility_analysis: UtilityAnalysis {
                healing: if has_healer { CapabilityLevel::Strong } else { CapabilityLevel::None },
                exploration: if has_utility { CapabilityLevel::Adequate } else { CapabilityLevel::Limited },
                social: if has_face { CapabilityLevel::Strong } else { CapabilityLevel::Limited },
                notes: None,
            },
            recommendations,
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
    fn test_party_analysis_request() {
        let party = vec![
            PartyMember {
                name: Some("Thordak".to_string()),
                class: "Fighter".to_string(),
                subclass: Some("Champion".to_string()),
                level: 5,
                role: Some(PartyRole::Tank),
            },
            PartyMember {
                name: None,
                class: "Wizard".to_string(),
                subclass: None,
                level: 5,
                role: Some(PartyRole::DamageDealer),
            },
        ];

        let request = PartyAnalysisRequest::new(party)
            .with_campaign_type("dungeon crawl");

        assert_eq!(request.party_details.len(), 2);
        assert_eq!(request.campaign_type, Some("dungeon crawl".to_string()));
    }

    #[test]
    fn test_static_analysis_balanced() {
        let party = vec![
            PartyMember { name: None, class: "Fighter".to_string(), subclass: None, level: 5, role: None },
            PartyMember { name: None, class: "Cleric".to_string(), subclass: None, level: 5, role: None },
            PartyMember { name: None, class: "Rogue".to_string(), subclass: None, level: 5, role: None },
            PartyMember { name: None, class: "Wizard".to_string(), subclass: None, level: 5, role: None },
        ];

        let analysis = PartyAnalyzer::static_analysis(&party);
        assert!(analysis.overall_balance_score >= 80);
        assert!(!analysis.strengths.is_empty());
    }

    #[test]
    fn test_static_analysis_no_healer() {
        let party = vec![
            PartyMember { name: None, class: "Fighter".to_string(), subclass: None, level: 5, role: None },
            PartyMember { name: None, class: "Rogue".to_string(), subclass: None, level: 5, role: None },
        ];

        let analysis = PartyAnalyzer::static_analysis(&party);
        assert!(analysis.weaknesses.iter().any(|w| w.contains("healing")));
        assert!(!analysis.recommendations.is_empty());
    }

    #[test]
    fn test_parse_response() {
        let response = serde_json::json!({
            "analysis": {
                "overall_balance_score": 75,
                "strengths": ["Strong damage output", "Good tankiness"],
                "weaknesses": ["No healer"],
                "combat_analysis": {
                    "damage_output": "strong",
                    "survivability": "adequate",
                    "control": "limited"
                },
                "utility_analysis": {
                    "healing": "none",
                    "exploration": "adequate",
                    "social": "limited"
                }
            },
            "recommendations": [
                {
                    "gap": "No healing",
                    "priority": "high",
                    "solutions": [
                        {"type": "consumables", "suggestion": "Provide healing potions"}
                    ]
                }
            ]
        });

        let suggestion = PartyAnalyzer::parse_response(&response).unwrap();

        assert_eq!(suggestion.analysis.overall_balance_score, 75);
        assert_eq!(suggestion.analysis.combat_analysis.damage_output, CapabilityLevel::Strong);
        assert_eq!(suggestion.analysis.utility_analysis.healing, CapabilityLevel::None);
        assert_eq!(suggestion.analysis.recommendations.len(), 1);
    }
}
