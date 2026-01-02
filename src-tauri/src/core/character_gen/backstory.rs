//! AI-Powered Backstory Generation
//!
//! Uses LLM to generate rich character backstories that integrate with
//! character traits, campaign settings, and world lore.

use crate::core::llm::{LLMClient, LLMConfig, ChatMessage, ChatRequest, MessageRole};
use crate::core::character_gen::{Character, GameSystem, BackstoryLength, CharacterGenError, Result};
use serde::{Deserialize, Serialize};
use super::prompts::{BackstoryPromptBuilder, BackstoryTemplates, estimate_tokens, recommended_temperature};

// ============================================================================
// Backstory Generation Types
// ============================================================================

/// Request for generating a character backstory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackstoryRequest {
    /// The character to generate a backstory for
    pub character: Character,
    /// Desired length of the backstory
    #[serde(default)]
    pub length: BackstoryLength,
    /// Campaign setting to match (optional)
    pub campaign_setting: Option<String>,
    /// Tone/style preferences
    #[serde(default)]
    pub style: BackstoryStyle,
    /// Specific elements to include
    #[serde(default)]
    pub include_elements: Vec<String>,
    /// Elements to avoid
    #[serde(default)]
    pub exclude_elements: Vec<String>,
}

/// Style preferences for backstory generation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackstoryStyle {
    /// Tone: "heroic", "tragic", "comedic", "mysterious", "gritty", "dark", "epic"
    pub tone: Option<String>,
    /// Perspective: "first_person", "third_person", "journal"
    pub perspective: Option<String>,
    /// Focus: "personal", "political", "adventurous", "philosophical", "professional"
    pub focus: Option<String>,
    /// Custom instructions for the LLM
    pub custom_instructions: Option<String>,
}

/// A complete generated backstory with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedBackstory {
    /// The full backstory text
    pub text: String,
    /// A brief summary (1-2 sentences)
    pub summary: String,
    /// Key events extracted from the backstory
    pub key_events: Vec<String>,
    /// Important NPCs mentioned in the backstory
    pub mentioned_npcs: Vec<BackstoryNPC>,
    /// Locations mentioned in the backstory
    pub mentioned_locations: Vec<String>,
    /// Potential plot hooks for GMs
    pub plot_hooks: Vec<String>,
    /// Suggested personality traits based on backstory
    pub suggested_traits: Vec<String>,
    /// Generation metadata
    #[serde(default)]
    pub metadata: BackstoryMetadata,
}

/// Metadata about the backstory generation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackstoryMetadata {
    /// The model used for generation
    pub model: Option<String>,
    /// Generation seed (if supported)
    pub seed: Option<u64>,
    /// Number of tokens used
    pub tokens_used: Option<u32>,
    /// Generation timestamp
    pub generated_at: Option<String>,
    /// Style used
    pub style: Option<BackstoryStyle>,
    /// Length setting used
    pub length: Option<String>,
}

/// An NPC mentioned in a backstory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackstoryNPC {
    /// NPC's name
    pub name: String,
    /// Relationship to the character (e.g., "mentor", "rival", "parent")
    pub relationship: String,
    /// Current status: "alive", "dead", "unknown", "estranged", "missing"
    pub status: String,
}

/// Options for backstory regeneration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegenerationOptions {
    /// Section to regenerate: "opening", "childhood", "formative_event", "climax", "resolution"
    pub section: Option<String>,
    /// User feedback for improvement
    pub feedback: Option<String>,
    /// Variation seed for different results
    pub seed: Option<u64>,
    /// Preserve specific elements from original
    pub preserve: Vec<String>,
}

/// Result of a backstory edit operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditResult {
    /// The updated backstory
    pub backstory: GeneratedBackstory,
    /// Summary of changes made
    pub changes_summary: String,
    /// Elements that were preserved
    pub preserved_elements: Vec<String>,
}

// ============================================================================
// Backstory Generator
// ============================================================================

/// Main backstory generator that orchestrates LLM calls
pub struct BackstoryGenerator {
    llm_client: LLMClient,
}

impl BackstoryGenerator {
    /// Create a new backstory generator with the given LLM config
    pub fn new(llm_config: LLMConfig) -> Self {
        Self {
            llm_client: LLMClient::new(llm_config),
        }
    }

    /// Generate a backstory for a character
    pub async fn generate(&self, request: &BackstoryRequest) -> Result<GeneratedBackstory> {
        let system_prompt = BackstoryPromptBuilder::build_system_prompt(request);
        let user_prompt = BackstoryPromptBuilder::build_user_prompt(request);
        let max_tokens = estimate_tokens(&request.length);
        let temperature = recommended_temperature(&request.length);

        let chat_request = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: user_prompt,
                },
            ],
            system_prompt: Some(system_prompt),
            temperature: Some(temperature),
            max_tokens: Some(max_tokens),
            provider: None,
        };

        let response = self.llm_client.chat(chat_request).await
            .map_err(|e| CharacterGenError::LLMError(e.to_string()))?;

        let mut backstory = self.parse_response(&response.content)?;

        // Add metadata
        backstory.metadata = BackstoryMetadata {
            model: Some(response.model),
            seed: None,
            tokens_used: response.usage.map(|u| u.input_tokens + u.output_tokens),
            generated_at: Some(chrono::Utc::now().to_rfc3339()),
            style: Some(request.style.clone()),
            length: Some(format!("{:?}", request.length)),
        };

        Ok(backstory)
    }

    /// Generate multiple backstory variations for selection
    pub async fn generate_variations(
        &self,
        request: &BackstoryRequest,
        count: usize,
    ) -> Result<Vec<GeneratedBackstory>> {
        let count = count.min(5); // Cap at 5 variations
        let mut variations = Vec::with_capacity(count);

        // Generate variations with different temperatures
        let base_temp = recommended_temperature(&request.length);
        let temp_variations: Vec<f32> = (0..count)
            .map(|i| (base_temp + (i as f32 * 0.05)).min(1.0))
            .collect();

        for (i, temp) in temp_variations.into_iter().enumerate() {
            let system_prompt = BackstoryPromptBuilder::build_system_prompt(request);
            let user_prompt = if i == 0 {
                BackstoryPromptBuilder::build_user_prompt(request)
            } else {
                format!(
                    "{}\n\nPlease provide a DIFFERENT take on this character's history. \
                     Variation #{}: try a {} approach.",
                    BackstoryPromptBuilder::build_user_prompt(request),
                    i + 1,
                    Self::variation_descriptor(i)
                )
            };

            let chat_request = ChatRequest {
                messages: vec![
                    ChatMessage {
                        role: MessageRole::User,
                        content: user_prompt,
                    },
                ],
                system_prompt: Some(system_prompt),
                temperature: Some(temp),
                max_tokens: Some(estimate_tokens(&request.length)),
                provider: None,
            };

            match self.llm_client.chat(chat_request).await {
                Ok(response) => {
                    if let Ok(backstory) = self.parse_response(&response.content) {
                        variations.push(backstory);
                    }
                }
                Err(e) => {
                    // Log but continue with other variations
                    eprintln!("Variation {} failed: {}", i, e);
                }
            }
        }

        if variations.is_empty() {
            return Err(CharacterGenError::BackstoryError(
                "Failed to generate any backstory variations".to_string()
            ));
        }

        Ok(variations)
    }

    /// Regenerate a specific section of a backstory
    pub async fn regenerate_section(
        &self,
        original: &GeneratedBackstory,
        options: &RegenerationOptions,
    ) -> Result<GeneratedBackstory> {
        let section = options.section.as_deref().unwrap_or("middle");
        let prompt = BackstoryPromptBuilder::build_regeneration_prompt(
            &original.text,
            section,
            options.feedback.as_deref(),
        );

        let system_prompt = "You are a creative writer specializing in TTRPG character backstories. \
             When regenerating sections, maintain consistency with the unchanged parts while \
             improving and expanding the requested section. Always respond with valid JSON \
             in the same format as the original.".to_string();

        let chat_request = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: prompt,
                },
            ],
            system_prompt: Some(system_prompt),
            temperature: Some(0.8),
            max_tokens: Some(2000),
            provider: None,
        };

        let response = self.llm_client.chat(chat_request).await
            .map_err(|e| CharacterGenError::LLMError(e.to_string()))?;

        self.parse_response(&response.content)
    }

    /// Edit an existing backstory based on user feedback
    pub async fn edit_backstory(
        &self,
        original: &GeneratedBackstory,
        edit_instructions: &str,
    ) -> Result<EditResult> {
        let prompt = BackstoryPromptBuilder::build_edit_prompt(&original.text, edit_instructions);

        let system_prompt = "You are a creative writer specializing in TTRPG character backstories. \
             When editing backstories, carefully apply the requested changes while maintaining \
             overall narrative coherence. Preserve elements not mentioned in the edit instructions. \
             Always respond with valid JSON in the same format.".to_string();

        let chat_request = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: prompt,
                },
            ],
            system_prompt: Some(system_prompt),
            temperature: Some(0.7),
            max_tokens: Some(2000),
            provider: None,
        };

        let response = self.llm_client.chat(chat_request).await
            .map_err(|e| CharacterGenError::LLMError(e.to_string()))?;

        let backstory = self.parse_response(&response.content)?;

        // Identify preserved elements
        let preserved = Self::identify_preserved_elements(original, &backstory);

        Ok(EditResult {
            backstory,
            changes_summary: format!("Applied edit: {}", edit_instructions),
            preserved_elements: preserved,
        })
    }

    /// Expand a brief backstory into a more detailed version
    pub async fn expand_backstory(
        &self,
        original: &GeneratedBackstory,
        target_length: BackstoryLength,
    ) -> Result<GeneratedBackstory> {
        let prompt = format!(
            "Here is a brief character backstory:\n\n{}\n\n\
             Please expand this into a {:?} backstory ({}-{} words).\n\
             Add more detail to each section:\n\
             - Expand key events with specific scenes and dialogue\n\
             - Develop NPCs with more personality and history\n\
             - Add sensory details and emotional depth\n\
             - Include additional plot hooks\n\n\
             Return the expanded backstory in JSON format.",
            original.text,
            target_length,
            target_length.word_count().0,
            target_length.word_count().1
        );

        let chat_request = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: prompt,
                },
            ],
            system_prompt: Some(self.default_system_prompt()),
            temperature: Some(0.8),
            max_tokens: Some(estimate_tokens(&target_length)),
            provider: None,
        };

        let response = self.llm_client.chat(chat_request).await
            .map_err(|e| CharacterGenError::LLMError(e.to_string()))?;

        self.parse_response(&response.content)
    }

    /// Condense a detailed backstory into a briefer version
    pub async fn condense_backstory(
        &self,
        original: &GeneratedBackstory,
        target_length: BackstoryLength,
    ) -> Result<GeneratedBackstory> {
        let prompt = format!(
            "Here is a detailed character backstory:\n\n{}\n\n\
             Please condense this into a {:?} backstory ({}-{} words).\n\
             Prioritize:\n\
             - The most important events\n\
             - Key relationships\n\
             - Core motivation\n\
             - Essential plot hooks\n\n\
             Return the condensed backstory in JSON format.",
            original.text,
            target_length,
            target_length.word_count().0,
            target_length.word_count().1
        );

        let chat_request = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: prompt,
                },
            ],
            system_prompt: Some(self.default_system_prompt()),
            temperature: Some(0.7),
            max_tokens: Some(estimate_tokens(&target_length)),
            provider: None,
        };

        let response = self.llm_client.chat(chat_request).await
            .map_err(|e| CharacterGenError::LLMError(e.to_string()))?;

        self.parse_response(&response.content)
    }

    /// Generate additional plot hooks for an existing backstory
    pub async fn generate_plot_hooks(
        &self,
        backstory: &GeneratedBackstory,
        character: &Character,
        count: usize,
    ) -> Result<Vec<String>> {
        let count = count.min(10).max(1);

        let prompt = format!(
            "Based on this character backstory:\n\n{}\n\n\
             Character: {} ({} {})\n\n\
             Generate {} new plot hooks that a GM could use to involve this character \
             in adventures. Each hook should:\n\
             - Connect to something in the backstory\n\
             - Create dramatic tension\n\
             - Be actionable in a session\n\
             - Feel organic to the character\n\n\
             Respond with a JSON array of strings.",
            backstory.text,
            character.name,
            character.race.as_deref().unwrap_or("Unknown"),
            character.class.as_deref().unwrap_or("Adventurer"),
            count
        );

        let chat_request = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: prompt,
                },
            ],
            system_prompt: Some(
                "You are a creative GM assistant. Generate plot hooks as a JSON array of strings.".to_string()
            ),
            temperature: Some(0.85),
            max_tokens: Some(800),
            provider: None,
        };

        let response = self.llm_client.chat(chat_request).await
            .map_err(|e| CharacterGenError::LLMError(e.to_string()))?;

        // Parse as array
        let hooks: Vec<String> = self.parse_string_array(&response.content)?;

        Ok(hooks)
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    fn default_system_prompt(&self) -> String {
        "You are a creative writer specializing in TTRPG character backstories. \
         Write engaging narratives that fit the game system and provide plot hooks. \
         Always respond with valid JSON in the specified format.".to_string()
    }

    fn variation_descriptor(index: usize) -> &'static str {
        match index {
            0 => "straightforward",
            1 => "more dramatic",
            2 => "with darker themes",
            3 => "focusing on relationships",
            4 => "emphasizing mystery",
            _ => "unique",
        }
    }

    fn parse_response(&self, content: &str) -> Result<GeneratedBackstory> {
        // Try to extract JSON from the response
        let json_str = Self::extract_json(content);

        // Try to parse as JSON
        match serde_json::from_str::<GeneratedBackstory>(json_str) {
            Ok(backstory) => Ok(backstory),
            Err(json_err) => {
                // If JSON parsing fails, try to create a basic structure from raw text
                // This handles cases where the LLM doesn't return perfect JSON
                if content.len() > 50 {
                    Ok(GeneratedBackstory {
                        text: content.to_string(),
                        summary: Self::extract_summary(content),
                        key_events: Self::extract_events(content),
                        mentioned_npcs: vec![],
                        mentioned_locations: vec![],
                        plot_hooks: BackstoryTemplates::get_example_hooks(&GameSystem::DnD5e)
                            .iter()
                            .take(2)
                            .map(|s| s.to_string())
                            .collect(),
                        suggested_traits: vec![],
                        metadata: BackstoryMetadata::default(),
                    })
                } else {
                    Err(CharacterGenError::BackstoryError(
                        format!("Failed to parse backstory: {}", json_err)
                    ))
                }
            }
        }
    }

    fn parse_string_array(&self, content: &str) -> Result<Vec<String>> {
        let json_str = Self::extract_json(content);

        serde_json::from_str::<Vec<String>>(json_str)
            .or_else(|_| {
                // Try to extract from content even if not valid JSON
                Ok(content.lines()
                    .filter(|l| !l.trim().is_empty())
                    .filter(|l| !l.starts_with('{') && !l.starts_with('['))
                    .map(|l| l.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '-' || c == ' '))
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect())
            })
    }

    fn extract_json(content: &str) -> &str {
        // Find JSON object or array
        if let Some(start) = content.find('{') {
            if let Some(end) = content.rfind('}') {
                return &content[start..=end];
            }
        }
        if let Some(start) = content.find('[') {
            if let Some(end) = content.rfind(']') {
                return &content[start..=end];
            }
        }
        content
    }

    fn extract_summary(content: &str) -> String {
        // Get first sentence or two
        let sentences: Vec<&str> = content.split('.').take(2).collect();
        if sentences.is_empty() {
            "A character with a rich history.".to_string()
        } else {
            format!("{}.", sentences.join(".").trim())
        }
    }

    fn extract_events(content: &str) -> Vec<String> {
        // Simple heuristic: look for numbered items or key phrases
        let event_markers = [
            "born", "discovered", "trained", "lost", "found", "became",
            "joined", "left", "fought", "learned", "met", "escaped"
        ];

        content.sentences()
            .filter(|s| event_markers.iter().any(|m| s.to_lowercase().contains(m)))
            .take(5)
            .map(|s| s.to_string())
            .collect()
    }

    fn identify_preserved_elements(original: &GeneratedBackstory, new: &GeneratedBackstory) -> Vec<String> {
        let mut preserved = Vec::new();

        // Check NPCs
        for npc in &original.mentioned_npcs {
            if new.mentioned_npcs.iter().any(|n| n.name == npc.name) {
                preserved.push(format!("NPC: {}", npc.name));
            }
        }

        // Check locations
        for loc in &original.mentioned_locations {
            if new.mentioned_locations.contains(loc) {
                preserved.push(format!("Location: {}", loc));
            }
        }

        preserved
    }
}

// ============================================================================
// String Extension for Sentence Splitting
// ============================================================================

trait SentenceIterator {
    fn sentences(&self) -> impl Iterator<Item = &str>;
}

impl SentenceIterator for str {
    fn sentences(&self) -> impl Iterator<Item = &str> {
        self.split('.')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
    }
}

// ============================================================================
// Prompt Templates (Legacy compatibility - now in prompts.rs)
// ============================================================================

pub mod templates {
    use super::BackstoryTemplates;

    /// Get a genre-appropriate opening for a backstory
    pub fn get_opening_hook(genre: &str) -> &'static str {
        BackstoryTemplates::get_opening_hook(Some(genre))
    }

    /// Get backstory structure elements
    pub fn backstory_elements() -> Vec<&'static str> {
        vec![
            "childhood",
            "formative event",
            "mentor or influential figure",
            "first adventure",
            "defining tragedy or triumph",
            "current motivation",
            "unresolved conflict",
            "secret or hidden past",
        ]
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backstory_style_default() {
        let style = BackstoryStyle::default();
        assert!(style.tone.is_none());
        assert!(style.perspective.is_none());
        assert!(style.focus.is_none());
    }

    #[test]
    fn test_backstory_length_word_counts() {
        assert_eq!(BackstoryLength::Brief.word_count(), (50, 100));
        assert_eq!(BackstoryLength::Medium.word_count(), (150, 300));
        assert_eq!(BackstoryLength::Detailed.word_count(), (400, 600));
    }

    #[test]
    fn test_extract_json() {
        let content = "Here is the backstory:\n{\"text\": \"test\"}";
        let json = BackstoryGenerator::extract_json(content);
        assert_eq!(json, "{\"text\": \"test\"}");
    }

    #[test]
    fn test_extract_summary() {
        let content = "Born in a small village. Trained as a warrior. Became a hero.";
        let summary = BackstoryGenerator::extract_summary(content);
        assert!(summary.contains("Born"));
    }

    #[test]
    fn test_variation_descriptors() {
        assert_eq!(BackstoryGenerator::variation_descriptor(0), "straightforward");
        assert_eq!(BackstoryGenerator::variation_descriptor(1), "more dramatic");
        assert_eq!(BackstoryGenerator::variation_descriptor(5), "unique");
    }

    #[test]
    fn test_sentence_iterator() {
        let text = "First sentence. Second sentence. Third sentence.";
        let sentences: Vec<&str> = text.sentences().collect();
        assert_eq!(sentences.len(), 3);
        assert_eq!(sentences[0], "First sentence");
    }

    #[test]
    fn test_backstory_metadata_default() {
        let metadata = BackstoryMetadata::default();
        assert!(metadata.model.is_none());
        assert!(metadata.seed.is_none());
    }
}
