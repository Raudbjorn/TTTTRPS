//! Session Summary Module
//!
//! Generates AI-powered session summaries and recaps.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// ============================================================================
// Types
// ============================================================================

/// Session summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Session ID
    pub session_id: String,
    /// Campaign ID
    pub campaign_id: String,
    /// Generated summary text
    pub summary: String,
    /// Key events
    pub key_events: Vec<String>,
    /// Combat outcomes
    pub combat_outcomes: Vec<CombatOutcome>,
    /// NPCs encountered
    pub npcs_encountered: Vec<String>,
    /// Locations visited
    pub locations_visited: Vec<String>,
    /// Loot acquired
    pub loot_acquired: Vec<String>,
    /// XP awarded (if applicable)
    pub xp_awarded: Option<u32>,
    /// "Previously on..." recap text
    pub recap: String,
    /// Generated at
    pub generated_at: DateTime<Utc>,
}

/// Combat outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatOutcome {
    /// Description of the combat
    pub description: String,
    /// Whether the party won
    pub victory: bool,
    /// Casualties (if any)
    pub casualties: Vec<String>,
    /// Notable moments
    pub notable_moments: Vec<String>,
}

/// Session summary options
#[derive(Debug, Clone, Default)]
pub struct SummaryOptions {
    /// Include combat details
    pub include_combat: bool,
    /// Include NPC interactions
    pub include_npcs: bool,
    /// Include loot/rewards
    pub include_loot: bool,
    /// Maximum length (in words, approximate)
    pub max_length: Option<usize>,
    /// Style: brief, detailed, narrative
    pub style: SummaryStyle,
}

/// Summary style
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SummaryStyle {
    /// Short bullet points
    Brief,
    /// Detailed breakdown
    #[default]
    Detailed,
    /// Story-like narrative
    Narrative,
}

// ============================================================================
// Session Summarizer
// ============================================================================

/// Generates session summaries
pub struct SessionSummarizer;

impl SessionSummarizer {
    pub fn new() -> Self {
        Self
    }

    /// Generate a summary prompt for the LLM
    pub fn generate_prompt(&self, session_data: &SessionData, options: &SummaryOptions) -> String {
        let style_instruction = match options.style {
            SummaryStyle::Brief => "Create a brief summary using bullet points. Keep it under 200 words.",
            SummaryStyle::Detailed => "Create a detailed summary covering all major events. Include specifics about combat, roleplay, and discoveries.",
            SummaryStyle::Narrative => "Create a narrative summary written like a story recap. Use evocative language befitting a fantasy tale.",
        };

        let mut prompt = format!(
            r#"You are a TTRPG session summarizer. Generate a summary for this gaming session.

{}

SESSION DATA:
- Session Number: {}
- Duration: {} hours
- Date: {}

LOG ENTRIES:
{}

"#,
            style_instruction,
            session_data.session_number,
            session_data.duration_hours,
            session_data.date,
            session_data.log_entries.join("\n"),
        );

        if options.include_combat && !session_data.combats.is_empty() {
            prompt.push_str("\nCOMBAT ENCOUNTERS:\n");
            for combat in &session_data.combats {
                prompt.push_str(&format!("- {}\n", combat));
            }
        }

        if options.include_npcs && !session_data.npcs.is_empty() {
            prompt.push_str("\nNPCs ENCOUNTERED:\n");
            for npc in &session_data.npcs {
                prompt.push_str(&format!("- {}\n", npc));
            }
        }

        if options.include_loot && !session_data.loot.is_empty() {
            prompt.push_str("\nLOOT/REWARDS:\n");
            for item in &session_data.loot {
                prompt.push_str(&format!("- {}\n", item));
            }
        }

        prompt.push_str(r#"
Please provide:
1. A session summary
2. A list of key events (3-5 bullet points)
3. A "Previously on..." recap suitable for reading at the start of the next session

Format your response as JSON:
{
  "summary": "...",
  "key_events": ["...", "..."],
  "recap": "..."
}
"#);

        prompt
    }

    /// Parse LLM response into a summary
    pub fn parse_response(
        &self,
        response: &str,
        session_id: &str,
        campaign_id: &str,
    ) -> Result<SessionSummary, String> {
        // Try to extract JSON from response
        let json_start = response.find('{');
        let json_end = response.rfind('}');

        if let (Some(start), Some(end)) = (json_start, json_end) {
            let json_str = &response[start..=end];

            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                return Ok(SessionSummary {
                    session_id: session_id.to_string(),
                    campaign_id: campaign_id.to_string(),
                    summary: parsed["summary"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                    key_events: parsed["key_events"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default(),
                    combat_outcomes: Vec::new(),
                    npcs_encountered: Vec::new(),
                    locations_visited: Vec::new(),
                    loot_acquired: Vec::new(),
                    xp_awarded: None,
                    recap: parsed["recap"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                    generated_at: Utc::now(),
                });
            }
        }

        // Fallback: use raw response as summary
        Ok(SessionSummary {
            session_id: session_id.to_string(),
            campaign_id: campaign_id.to_string(),
            summary: response.to_string(),
            key_events: Vec::new(),
            combat_outcomes: Vec::new(),
            npcs_encountered: Vec::new(),
            locations_visited: Vec::new(),
            loot_acquired: Vec::new(),
            xp_awarded: None,
            recap: String::new(),
            generated_at: Utc::now(),
        })
    }

    /// Generate a quick recap for the start of a session
    pub fn generate_recap_prompt(&self, previous_summaries: &[SessionSummary]) -> String {
        if previous_summaries.is_empty() {
            return "This is the first session. No previous recap needed.".to_string();
        }

        let mut prompt = String::from(
            "Based on these previous session summaries, create a brief 'Previously on...' \
             recap that can be read aloud at the start of the next session. \
             Keep it engaging and under 150 words.\n\n",
        );

        // Use last 3 sessions
        for summary in previous_summaries.iter().rev().take(3) {
            prompt.push_str(&format!("SESSION SUMMARY:\n{}\n\n", summary.summary));
        }

        prompt
    }
}

impl Default for SessionSummarizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Session data for summary generation
#[derive(Debug, Clone, Default)]
pub struct SessionData {
    pub session_number: u32,
    pub duration_hours: f32,
    pub date: String,
    pub log_entries: Vec<String>,
    pub combats: Vec<String>,
    pub npcs: Vec<String>,
    pub loot: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_prompt() {
        let summarizer = SessionSummarizer::new();

        let session_data = SessionData {
            session_number: 5,
            duration_hours: 4.0,
            date: "2024-01-15".to_string(),
            log_entries: vec![
                "Party entered the dungeon".to_string(),
                "Fought goblins".to_string(),
                "Found treasure".to_string(),
            ],
            combats: vec!["Goblin ambush - victory".to_string()],
            npcs: vec!["Merchant Bob".to_string()],
            loot: vec!["50 gold".to_string()],
        };

        let options = SummaryOptions {
            include_combat: true,
            include_npcs: true,
            include_loot: true,
            style: SummaryStyle::Detailed,
            ..Default::default()
        };

        let prompt = summarizer.generate_prompt(&session_data, &options);

        assert!(prompt.contains("Session Number: 5"));
        assert!(prompt.contains("goblin"));
        assert!(prompt.contains("Merchant Bob"));
    }

    #[test]
    fn test_parse_response() {
        let summarizer = SessionSummarizer::new();

        let response = r#"
        Here's the summary:
        {
            "summary": "The party explored the dungeon and defeated goblins.",
            "key_events": ["Entered dungeon", "Defeated goblins", "Found treasure"],
            "recap": "Previously on our adventure..."
        }
        "#;

        let result = summarizer.parse_response(response, "session-1", "campaign-1");
        assert!(result.is_ok());

        let summary = result.unwrap();
        assert!(summary.summary.contains("dungeon"));
        assert_eq!(summary.key_events.len(), 3);
    }
}
