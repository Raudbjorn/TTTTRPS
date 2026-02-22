//! Personality Application Layer
//!
//! Handles applying personality profiles to chat responses, NPC dialogue,
//! and narration tone. Manages active personalities per campaign/session.
//!
//! Features:
//! - Personality injection into LLM system prompts
//! - NPC dialogue styling based on linked personality
//! - Narration tone matching to active personality
//! - Per-campaign/session default personalities
//! - Personality preview before selection

use crate::core::personality_base::{PersonalityProfile, PersonalityStore, PersonalityError, SpeechPatterns};
use crate::core::llm::{LLMClient, ChatMessage, ChatRequest, MessageRole};
use std::collections::HashMap;
use std::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::Utc;

// ============================================================================
// Personality Application Types
// ============================================================================

/// Narrative tone for personality application
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum NarrativeTone {
    #[default]
    Neutral,
    Dramatic,
    Casual,
    Mysterious,
    Humorous,
    Epic,
    Gritty,
    Whimsical,
    Horror,
    Romantic,
}

impl NarrativeTone {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Dramatic => "dramatic",
            Self::Casual => "casual",
            Self::Mysterious => "mysterious",
            Self::Humorous => "humorous",
            Self::Epic => "epic",
            Self::Gritty => "gritty",
            Self::Whimsical => "whimsical",
            Self::Horror => "horror",
            Self::Romantic => "romantic",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "dramatic" => Self::Dramatic,
            "casual" => Self::Casual,
            "mysterious" => Self::Mysterious,
            "humorous" => Self::Humorous,
            "epic" => Self::Epic,
            "gritty" => Self::Gritty,
            "whimsical" => Self::Whimsical,
            "horror" => Self::Horror,
            "romantic" => Self::Romantic,
            _ => Self::Neutral,
        }
    }

    pub fn get_description(&self) -> &'static str {
        match self {
            Self::Neutral => "Balanced, straightforward narration without strong emotional coloring",
            Self::Dramatic => "High-stakes, tension-filled narration with emphasis on emotional impact",
            Self::Casual => "Relaxed, conversational tone with lighter language",
            Self::Mysterious => "Enigmatic, atmospheric narration with hints and foreshadowing",
            Self::Humorous => "Light-hearted, witty narration with comedic timing",
            Self::Epic => "Grand, sweeping narration befitting heroic tales",
            Self::Gritty => "Raw, realistic narration emphasizing harshness and struggle",
            Self::Whimsical => "Playful, fantastical narration with wonder and charm",
            Self::Horror => "Unsettling, dread-inducing narration with building tension",
            Self::Romantic => "Emotionally rich narration focusing on relationships and feelings",
        }
    }
}

/// Vocabulary complexity level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum VocabularyLevel {
    Simple,
    #[default]
    Standard,
    Elevated,
    Archaic,
    Technical,
}

impl VocabularyLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Standard => "standard",
            Self::Elevated => "elevated",
            Self::Archaic => "archaic",
            Self::Technical => "technical",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "simple" => Self::Simple,
            "elevated" => Self::Elevated,
            "archaic" => Self::Archaic,
            "technical" => Self::Technical,
            _ => Self::Standard,
        }
    }

    pub fn get_guidance(&self) -> &'static str {
        match self {
            Self::Simple => "Use common words, short sentences, and straightforward expressions",
            Self::Standard => "Use clear, accessible language appropriate for general audiences",
            Self::Elevated => "Use sophisticated vocabulary and complex sentence structures",
            Self::Archaic => "Use archaic language patterns, thee/thou, forsooth, verily, etc.",
            Self::Technical => "Use precise terminology and domain-specific jargon",
        }
    }
}

/// Narrative perspective style
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum NarrativeStyle {
    #[default]
    ThirdPersonLimited,
    ThirdPersonOmniscient,
    SecondPerson,
    FirstPerson,
}

impl NarrativeStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ThirdPersonLimited => "third_person_limited",
            Self::ThirdPersonOmniscient => "third_person_omniscient",
            Self::SecondPerson => "second_person",
            Self::FirstPerson => "first_person",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().replace('_', " ").as_str() {
            "third person omniscient" | "omniscient" => Self::ThirdPersonOmniscient,
            "second person" | "you" => Self::SecondPerson,
            "first person" | "i" => Self::FirstPerson,
            _ => Self::ThirdPersonLimited,
        }
    }

    pub fn get_guidance(&self) -> &'static str {
        match self {
            Self::ThirdPersonLimited => "Narrate from outside, following one character's perspective (he/she/they)",
            Self::ThirdPersonOmniscient => "Narrate from an all-knowing perspective, able to see all characters' thoughts",
            Self::SecondPerson => "Narrate directly to the player as 'you' for immersive engagement",
            Self::FirstPerson => "Narrate as a character within the story, using 'I' and 'we'",
        }
    }
}

/// Verbosity level for responses
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum VerbosityLevel {
    Terse,
    #[default]
    Standard,
    Verbose,
    Elaborate,
}

impl VerbosityLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Terse => "terse",
            Self::Standard => "standard",
            Self::Verbose => "verbose",
            Self::Elaborate => "elaborate",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "terse" | "brief" | "short" => Self::Terse,
            "verbose" | "detailed" => Self::Verbose,
            "elaborate" | "extensive" => Self::Elaborate,
            _ => Self::Standard,
        }
    }

    pub fn get_token_guidance(&self) -> (u32, u32) {
        match self {
            Self::Terse => (50, 150),
            Self::Standard => (100, 300),
            Self::Verbose => (200, 500),
            Self::Elaborate => (400, 1000),
        }
    }
}

/// Genre conventions for personality styling
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum GenreConvention {
    #[default]
    HighFantasy,
    DarkFantasy,
    SwordAndSorcery,
    UrbanFantasy,
    SciFi,
    Horror,
    Steampunk,
    Western,
    Noir,
    Cyberpunk,
    Historical,
}

impl GenreConvention {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HighFantasy => "high_fantasy",
            Self::DarkFantasy => "dark_fantasy",
            Self::SwordAndSorcery => "sword_and_sorcery",
            Self::UrbanFantasy => "urban_fantasy",
            Self::SciFi => "sci_fi",
            Self::Horror => "horror",
            Self::Steampunk => "steampunk",
            Self::Western => "western",
            Self::Noir => "noir",
            Self::Cyberpunk => "cyberpunk",
            Self::Historical => "historical",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().replace(['_', '-'], " ").as_str() {
            "dark fantasy" => Self::DarkFantasy,
            "sword and sorcery" | "sword sorcery" => Self::SwordAndSorcery,
            "urban fantasy" => Self::UrbanFantasy,
            "sci fi" | "scifi" | "science fiction" => Self::SciFi,
            "horror" => Self::Horror,
            "steampunk" => Self::Steampunk,
            "western" => Self::Western,
            "noir" | "detective" => Self::Noir,
            "cyberpunk" => Self::Cyberpunk,
            "historical" => Self::Historical,
            _ => Self::HighFantasy,
        }
    }

    pub fn get_conventions(&self) -> &'static str {
        match self {
            Self::HighFantasy => "Noble heroes, clear good vs evil, magic wonder, ancient prophecies, epic quests",
            Self::DarkFantasy => "Moral ambiguity, harsh consequences, grim atmosphere, horror elements, survival",
            Self::SwordAndSorcery => "Personal stakes, gritty action, rogues and warriors, treasure hunting, low magic",
            Self::UrbanFantasy => "Modern setting with magic, hidden supernatural world, contemporary language",
            Self::SciFi => "Technology focus, scientific concepts, futuristic elements, space/cyber themes",
            Self::Horror => "Building dread, psychological tension, vulnerability, the unknown, fear",
            Self::Steampunk => "Victorian aesthetics, clockwork and steam, inventors, alternate history",
            Self::Western => "Frontier justice, outlaws and lawmen, honor codes, rugged individualism",
            Self::Noir => "Moral ambiguity, cynicism, crime, femmes fatales, rain-slicked streets",
            Self::Cyberpunk => "High tech low life, corporate dystopia, hackers, augmentation, neon",
            Self::Historical => "Period accuracy, authentic language, cultural customs, real-world grounding",
        }
    }
}

/// Extended personality settings for application
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersonalitySettings {
    /// Narrative tone
    pub tone: NarrativeTone,
    /// Vocabulary complexity
    pub vocabulary: VocabularyLevel,
    /// Narrative perspective
    pub narrative_style: NarrativeStyle,
    /// Response verbosity
    pub verbosity: VerbosityLevel,
    /// Genre conventions to follow
    pub genre: GenreConvention,
    /// Custom speech patterns (catchphrases, etc.)
    pub custom_patterns: Vec<String>,
    /// Whether to use dialectal variations
    pub use_dialect: bool,
    /// Dialect description if enabled
    pub dialect: Option<String>,
}

impl PersonalitySettings {
    pub fn to_prompt_section(&self) -> String {
        let mut prompt = String::new();

        prompt.push_str("NARRATIVE STYLE SETTINGS:\n");
        prompt.push_str(&format!("- Tone: {} - {}\n", self.tone.as_str(), self.tone.get_description()));
        prompt.push_str(&format!("- Vocabulary: {} - {}\n", self.vocabulary.as_str(), self.vocabulary.get_guidance()));
        prompt.push_str(&format!("- Perspective: {} - {}\n", self.narrative_style.as_str(), self.narrative_style.get_guidance()));
        prompt.push_str(&format!("- Verbosity: {} (target {}-{} tokens)\n",
            self.verbosity.as_str(),
            self.verbosity.get_token_guidance().0,
            self.verbosity.get_token_guidance().1
        ));
        prompt.push_str(&format!("- Genre: {} - {}\n", self.genre.as_str(), self.genre.get_conventions()));

        if self.use_dialect {
            if let Some(dialect) = &self.dialect {
                prompt.push_str(&format!("- Dialect: {}\n", dialect));
            }
        }

        if !self.custom_patterns.is_empty() {
            prompt.push_str("- Incorporate these speech patterns:\n");
            for pattern in &self.custom_patterns {
                prompt.push_str(&format!("  * \"{}\"\n", pattern));
            }
        }

        prompt
    }
}

/// Active personality context for a campaign or session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePersonalityContext {
    /// Campaign ID this context belongs to
    pub campaign_id: String,
    /// Session ID (optional, for session-specific overrides)
    pub session_id: Option<String>,
    /// Default narrator personality ID
    pub narrator_personality_id: Option<String>,
    /// Map of NPC IDs to their personality IDs
    pub npc_personalities: HashMap<String, String>,
    /// Map of location IDs to their ambient personality IDs
    pub location_personalities: HashMap<String, String>,
    /// Current scene mood modifier
    pub scene_mood: Option<SceneMood>,
    /// Whether personality is actively being applied
    pub active: bool,
    /// Extended personality settings
    pub settings: PersonalitySettings,
    /// Timestamp of last update
    pub updated_at: String,
}

impl Default for ActivePersonalityContext {
    fn default() -> Self {
        Self {
            campaign_id: String::new(),
            session_id: None,
            narrator_personality_id: None,
            npc_personalities: HashMap::new(),
            location_personalities: HashMap::new(),
            scene_mood: None,
            active: true,
            settings: PersonalitySettings::default(),
            updated_at: Utc::now().to_rfc3339(),
        }
    }
}

/// Scene mood that modifies personality application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneMood {
    pub tone: String,  // "tense", "relaxed", "mysterious", "combat", etc.
    pub intensity: u8, // 1-10
    pub description: String,
}

/// Options for applying personality to content
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersonalityApplicationOptions {
    /// The personality profile ID to use
    pub personality_id: Option<String>,
    /// Override scene mood
    pub scene_mood: Option<SceneMood>,
    /// Content type being generated
    pub content_type: ContentType,
    /// Whether to include internal thoughts
    pub include_thoughts: bool,
    /// Maximum response length
    pub max_length: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ContentType {
    #[default]
    Dialogue,
    Narration,
    InternalThought,
    Description,
    Action,
}

/// Result of personality application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyledContent {
    /// The styled content
    pub content: String,
    /// The personality used
    pub personality_id: Option<String>,
    /// Any style notes for the GM
    pub style_notes: Vec<String>,
}

/// Preview of how a personality would affect content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityPreview {
    pub personality_id: String,
    pub personality_name: String,
    /// Sample greetings in this personality
    pub sample_greetings: Vec<String>,
    /// Sample responses to common prompts
    pub sample_responses: HashMap<String, String>,
    /// Key characteristics summary
    pub characteristics: Vec<String>,
}

// ============================================================================
// Personality Application Manager
// ============================================================================

pub struct PersonalityApplicationManager {
    /// Active personality contexts by campaign ID
    contexts: RwLock<HashMap<String, ActivePersonalityContext>>,
    /// Session-specific overrides (session_id -> context)
    session_contexts: RwLock<HashMap<String, ActivePersonalityContext>>,
    /// Reference to the personality store
    store: std::sync::Arc<PersonalityStore>,
}

impl PersonalityApplicationManager {
    pub fn new(store: std::sync::Arc<PersonalityStore>) -> Self {
        Self {
            contexts: RwLock::new(HashMap::new()),
            session_contexts: RwLock::new(HashMap::new()),
            store,
        }
    }

    /// Get reference to the personality store
    pub fn store(&self) -> &std::sync::Arc<PersonalityStore> {
        &self.store
    }

    // ========================================================================
    // Context Management
    // ========================================================================

    /// Get or create a personality context for a campaign
    pub fn get_context(&self, campaign_id: &str) -> ActivePersonalityContext {
        let contexts = self.contexts.read().unwrap();
        contexts.get(campaign_id).cloned().unwrap_or_else(|| {
            ActivePersonalityContext {
                campaign_id: campaign_id.to_string(),
                ..Default::default()
            }
        })
    }

    /// Get personality context for a session, falling back to campaign context
    pub fn get_session_context(&self, session_id: &str, campaign_id: &str) -> ActivePersonalityContext {
        // Check for session-specific override first
        let session_contexts = self.session_contexts.read().unwrap();
        if let Some(ctx) = session_contexts.get(session_id) {
            return ctx.clone();
        }
        drop(session_contexts);

        // Fall back to campaign context
        self.get_context(campaign_id)
    }

    /// Set session-specific personality context
    pub fn set_session_context(&self, session_id: &str, context: ActivePersonalityContext) {
        let mut session_contexts = self.session_contexts.write().unwrap();
        session_contexts.insert(session_id.to_string(), context);
    }

    /// Clear session-specific context (use campaign defaults)
    pub fn clear_session_context(&self, session_id: &str) {
        let mut session_contexts = self.session_contexts.write().unwrap();
        session_contexts.remove(session_id);
    }

    /// Update the personality context for a campaign
    pub fn set_context(&self, context: ActivePersonalityContext) {
        let mut contexts = self.contexts.write().unwrap();
        contexts.insert(context.campaign_id.clone(), context);
    }

    /// Set the active personality for a session
    pub fn set_active_personality(&self, session_id: &str, personality_id: Option<String>, campaign_id: &str) {
        let mut session_contexts = self.session_contexts.write().unwrap();

        // Get existing or create new from campaign context
        let context = session_contexts.entry(session_id.to_string())
            .or_insert_with(|| {
                let campaign_ctx = self.get_context(campaign_id);
                ActivePersonalityContext {
                    session_id: Some(session_id.to_string()),
                    ..campaign_ctx
                }
            });

        context.narrator_personality_id = personality_id;
        context.updated_at = Utc::now().to_rfc3339();
    }

    /// Get the active personality ID for a session
    pub fn get_active_personality_id(&self, session_id: &str, campaign_id: &str) -> Option<String> {
        let ctx = self.get_session_context(session_id, campaign_id);
        ctx.narrator_personality_id
    }

    /// Set the narrator personality for a campaign
    pub fn set_narrator_personality(&self, campaign_id: &str, personality_id: Option<String>) {
        let mut contexts = self.contexts.write().unwrap();
        let context = contexts.entry(campaign_id.to_string())
            .or_insert_with(|| ActivePersonalityContext {
                campaign_id: campaign_id.to_string(),
                ..Default::default()
            });
        context.narrator_personality_id = personality_id;
        context.updated_at = Utc::now().to_rfc3339();
    }

    /// Assign a personality to an NPC
    pub fn assign_npc_personality(&self, campaign_id: &str, npc_id: &str, personality_id: &str) {
        let mut contexts = self.contexts.write().unwrap();
        let context = contexts.entry(campaign_id.to_string())
            .or_insert_with(|| ActivePersonalityContext {
                campaign_id: campaign_id.to_string(),
                ..Default::default()
            });
        context.npc_personalities.insert(npc_id.to_string(), personality_id.to_string());
        context.updated_at = Utc::now().to_rfc3339();
    }

    /// Remove personality assignment from an NPC
    pub fn unassign_npc_personality(&self, campaign_id: &str, npc_id: &str) {
        let mut contexts = self.contexts.write().unwrap();
        if let Some(context) = contexts.get_mut(campaign_id) {
            context.npc_personalities.remove(npc_id);
            context.updated_at = Utc::now().to_rfc3339();
        }
    }

    /// Set the scene mood
    pub fn set_scene_mood(&self, campaign_id: &str, mood: Option<SceneMood>) {
        let mut contexts = self.contexts.write().unwrap();
        let context = contexts.entry(campaign_id.to_string())
            .or_insert_with(|| ActivePersonalityContext {
                campaign_id: campaign_id.to_string(),
                ..Default::default()
            });
        context.scene_mood = mood;
        context.updated_at = Utc::now().to_rfc3339();
    }

    /// Update personality settings for a campaign
    pub fn set_personality_settings(&self, campaign_id: &str, settings: PersonalitySettings) {
        let mut contexts = self.contexts.write().unwrap();
        let context = contexts.entry(campaign_id.to_string())
            .or_insert_with(|| ActivePersonalityContext {
                campaign_id: campaign_id.to_string(),
                ..Default::default()
            });
        context.settings = settings;
        context.updated_at = Utc::now().to_rfc3339();
    }

    /// Toggle personality application on/off
    pub fn set_personality_active(&self, campaign_id: &str, active: bool) {
        let mut contexts = self.contexts.write().unwrap();
        let context = contexts.entry(campaign_id.to_string())
            .or_insert_with(|| ActivePersonalityContext {
                campaign_id: campaign_id.to_string(),
                ..Default::default()
            });
        context.active = active;
        context.updated_at = Utc::now().to_rfc3339();
    }

    /// List all campaign contexts
    pub fn list_contexts(&self) -> Vec<ActivePersonalityContext> {
        let contexts = self.contexts.read().unwrap();
        contexts.values().cloned().collect()
    }

    // ========================================================================
    // Personality Application
    // ========================================================================

    /// Build a system prompt with personality injection
    pub fn build_personality_prompt(
        &self,
        personality_id: Option<&str>,
        content_type: ContentType,
        scene_mood: Option<&SceneMood>,
    ) -> Result<String, PersonalityError> {
        self.build_personality_prompt_with_settings(personality_id, content_type, scene_mood, None)
    }

    /// Build a system prompt with personality and extended settings
    pub fn build_personality_prompt_with_settings(
        &self,
        personality_id: Option<&str>,
        content_type: ContentType,
        scene_mood: Option<&SceneMood>,
        settings: Option<&PersonalitySettings>,
    ) -> Result<String, PersonalityError> {
        let mut prompt = String::new();

        // Add personality if specified
        if let Some(pid) = personality_id {
            let profile = self.store.get(pid)?;
            prompt.push_str(&profile.to_system_prompt());
            prompt.push_str("\n\n");
        }

        // Add extended settings if provided
        if let Some(s) = settings {
            prompt.push_str(&s.to_prompt_section());
            prompt.push('\n');
        }

        // Add content type guidance
        prompt.push_str("CONTENT TYPE INSTRUCTIONS:\n");
        match content_type {
            ContentType::Dialogue => {
                prompt.push_str("Generate spoken dialogue. Use quotation marks for speech. \
                                 Include appropriate speech tags and body language descriptions.\n");
            }
            ContentType::Narration => {
                prompt.push_str("Generate narrative description. \
                                 Set the scene and atmosphere with vivid, engaging prose.\n");
            }
            ContentType::InternalThought => {
                prompt.push_str("Generate internal thoughts in italics. \
                                 Show the character's inner monologue and emotional state.\n");
            }
            ContentType::Description => {
                prompt.push_str("Generate sensory-rich description. \
                                 Include sight, sound, smell, touch, and atmosphere.\n");
            }
            ContentType::Action => {
                prompt.push_str("Generate action description. Be dynamic, visceral, and engaging.\n");
            }
        }

        // Add scene mood modifier
        if let Some(mood) = scene_mood {
            prompt.push_str("\nSCENE MOOD:\n");
            prompt.push_str(&format!("- Tone: {} (intensity {}/10)\n", mood.tone, mood.intensity));
            prompt.push_str(&format!("- Context: {}\n", mood.description));
            prompt.push_str("Adjust your response to match this mood throughout.\n");
        }

        Ok(prompt)
    }

    /// Get a complete system prompt for a session
    pub fn get_session_system_prompt(
        &self,
        session_id: &str,
        campaign_id: &str,
        content_type: ContentType,
    ) -> Result<String, PersonalityError> {
        let ctx = self.get_session_context(session_id, campaign_id);

        if !ctx.active {
            return Ok(String::new());
        }

        self.build_personality_prompt_with_settings(
            ctx.narrator_personality_id.as_deref(),
            content_type,
            ctx.scene_mood.as_ref(),
            Some(&ctx.settings),
        )
    }

    /// Generate a complete personality prompt from just the personality ID
    pub fn get_personality_prompt(&self, personality_id: &str) -> Result<String, PersonalityError> {
        let profile = self.store.get(personality_id)?;
        Ok(profile.to_system_prompt())
    }

    /// Apply personality styling to raw text using LLM transformation
    pub async fn apply_personality_to_text(
        &self,
        text: &str,
        personality_id: &str,
        llm_client: &LLMClient,
    ) -> Result<String, PersonalityError> {
        let profile = self.store.get(personality_id)?;
        let system_prompt = format!(
            "{}\n\nREWRITE TASK:\nRewrite the following text in this character's voice and style. \
             Preserve the meaning but transform the language to match this personality.\n\n\
             Original text:\n{}",
            profile.to_system_prompt(),
            text
        );

        let request = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: "Rewrite this text in character.".to_string(),
                    images: None,
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            system_prompt: Some(system_prompt),
            temperature: Some(0.7),
            max_tokens: Some(500),
            provider: None,
            tools: None,
            tool_choice: None,
        };

        let response = llm_client.chat(request).await
            .map_err(PersonalityError::LLMError)?;

        Ok(response.content)
    }

    /// Apply personality styling to raw content
    pub fn style_content(
        &self,
        raw_content: &str,
        options: &PersonalityApplicationOptions,
    ) -> Result<StyledContent, PersonalityError> {
        let profile = if let Some(pid) = &options.personality_id {
            Some(self.store.get(pid)?)
        } else {
            None
        };

        let styled = if let Some(p) = &profile {
            self.apply_speech_patterns(raw_content, &p.speech_patterns, &options.content_type)
        } else {
            raw_content.to_string()
        };

        let style_notes = profile.as_ref()
            .map(|p| self.generate_style_notes(p, &options.content_type))
            .unwrap_or_default();

        Ok(StyledContent {
            content: styled,
            personality_id: options.personality_id.clone(),
            style_notes,
        })
    }

    /// Apply speech patterns to text
    fn apply_speech_patterns(
        &self,
        content: &str,
        _patterns: &SpeechPatterns,
        content_type: &ContentType,
    ) -> String {
        let result = content.to_string();

        // Only apply to dialogue
        if *content_type != ContentType::Dialogue {
            return result;
        }

        // Add common phrases occasionally (simple implementation)
        // A more sophisticated version would use the LLM

        // For now, just return the content as-is
        // The personality is primarily applied through the system prompt
        result
    }

    /// Generate style notes for the GM
    fn generate_style_notes(
        &self,
        profile: &PersonalityProfile,
        _content_type: &ContentType,
    ) -> Vec<String> {
        let mut notes = vec![];

        // Formality note
        let formality_desc = match profile.speech_patterns.formality {
            1..=3 => "very casual",
            4..=6 => "moderately formal",
            7..=9 => "quite formal",
            10 => "extremely formal",
            _ => "neutral",
        };
        notes.push(format!("Speech: {} ({})", formality_desc, profile.speech_patterns.pacing));

        // Key traits
        for trait_item in profile.traits.iter().take(2) {
            if trait_item.intensity >= 7 {
                notes.push(format!("Strong {}: {}", trait_item.trait_name, trait_item.manifestation));
            }
        }

        // Common phrases reminder
        if !profile.speech_patterns.common_phrases.is_empty() {
            let phrase = &profile.speech_patterns.common_phrases[0];
            notes.push(format!("Catchphrase: \"{}\"", phrase));
        }

        notes
    }

    // ========================================================================
    // Preview and Testing
    // ========================================================================

    /// Generate a preview of how a personality would affect dialogue
    pub fn preview_personality(&self, personality_id: &str) -> Result<PersonalityPreview, PersonalityError> {
        let profile = self.store.get(personality_id)?;

        let mut sample_responses = HashMap::new();

        // Generate sample responses based on behavioral tendencies
        sample_responses.insert(
            "greeting".to_string(),
            format!("(Approaches with {} demeanor)", profile.behavioral_tendencies.general_attitude),
        );
        sample_responses.insert(
            "help_request".to_string(),
            profile.behavioral_tendencies.help_response.clone(),
        );
        sample_responses.insert(
            "conflict".to_string(),
            profile.behavioral_tendencies.conflict_response.clone(),
        );
        sample_responses.insert(
            "stranger".to_string(),
            profile.behavioral_tendencies.stranger_response.clone(),
        );
        sample_responses.insert(
            "authority".to_string(),
            profile.behavioral_tendencies.authority_response.clone(),
        );

        let mut characteristics = vec![
            format!("Formality: {}/10", profile.speech_patterns.formality),
            format!("Pacing: {}", profile.speech_patterns.pacing),
            format!("Vocabulary: {}", profile.speech_patterns.vocabulary_style),
            format!("Attitude: {}", profile.behavioral_tendencies.general_attitude),
        ];

        // Add key traits
        for trait_item in profile.traits.iter().take(3) {
            characteristics.push(format!("{} ({}/10)", trait_item.trait_name, trait_item.intensity));
        }

        // Add dialect if present
        if let Some(dialect) = &profile.speech_patterns.dialect_notes {
            characteristics.push(format!("Dialect: {}", dialect));
        }

        Ok(PersonalityPreview {
            personality_id: personality_id.to_string(),
            personality_name: profile.name.clone(),
            sample_greetings: profile.speech_patterns.common_phrases.clone(),
            sample_responses,
            characteristics,
        })
    }

    /// Generate an extended preview with example phrases
    pub fn preview_personality_extended(&self, personality_id: &str) -> Result<ExtendedPersonalityPreview, PersonalityError> {
        let profile = self.store.get(personality_id)?;
        let basic_preview = self.preview_personality(personality_id)?;

        Ok(ExtendedPersonalityPreview {
            basic: basic_preview,
            system_prompt_preview: profile.to_system_prompt(),
            example_phrases: profile.example_phrases.clone(),
            knowledge_areas: profile.knowledge_areas.clone(),
            tags: profile.tags.clone(),
            source: profile.source.clone(),
        })
    }

    /// Test a personality by generating a response to a prompt
    pub async fn test_personality(
        &self,
        personality_id: &str,
        test_prompt: &str,
        llm_client: &LLMClient,
    ) -> Result<String, PersonalityError> {
        let profile = self.store.get(personality_id)?;
        let system_prompt = profile.to_system_prompt();

        let request = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: test_prompt.to_string(),
                    images: None,
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            system_prompt: Some(system_prompt),
            temperature: Some(0.8),
            max_tokens: Some(500),
            provider: None,
            tools: None,
            tool_choice: None,
        };

        let response = llm_client.chat(request).await
            .map_err(PersonalityError::LLMError)?;

        Ok(response.content)
    }

    /// Generate a quick preview response for personality selection UI
    pub async fn generate_preview_response(
        &self,
        personality_id: &str,
        llm_client: &LLMClient,
    ) -> Result<PreviewResponse, PersonalityError> {
        let profile = self.store.get(personality_id)?;
        let system_prompt = profile.to_system_prompt();

        // Generate a sample greeting
        let greeting_request = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: "A traveler approaches you. Greet them briefly.".to_string(),
                    images: None,
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            system_prompt: Some(system_prompt.clone()),
            temperature: Some(0.8),
            max_tokens: Some(100),
            provider: None,
            tools: None,
            tool_choice: None,
        };

        let greeting = llm_client.chat(greeting_request).await
            .map(|r| r.content)
            .unwrap_or_else(|_| profile.speech_patterns.common_phrases.first()
                .cloned()
                .unwrap_or_else(|| "Greetings.".to_string()));

        Ok(PreviewResponse {
            personality_id: personality_id.to_string(),
            personality_name: profile.name.clone(),
            sample_greeting: greeting,
            formality_level: profile.speech_patterns.formality,
            key_traits: profile.traits.iter()
                .take(3)
                .map(|t| t.trait_name.clone())
                .collect(),
        })
    }
}

/// Extended preview with full details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedPersonalityPreview {
    pub basic: PersonalityPreview,
    pub system_prompt_preview: String,
    pub example_phrases: Vec<String>,
    pub knowledge_areas: Vec<String>,
    pub tags: Vec<String>,
    pub source: Option<String>,
}

/// Quick preview response for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewResponse {
    pub personality_id: String,
    pub personality_name: String,
    pub sample_greeting: String,
    pub formality_level: u8,
    pub key_traits: Vec<String>,
}

// ============================================================================
// NPC Dialogue Styler
// ============================================================================

/// Specialized styler for NPC dialogue with full personality support
pub struct NPCDialogueStyler {
    manager: std::sync::Arc<PersonalityApplicationManager>,
}

impl NPCDialogueStyler {
    pub fn new(manager: std::sync::Arc<PersonalityApplicationManager>) -> Self {
        Self { manager }
    }

    /// Generate styled dialogue for an NPC
    pub fn style_npc_dialogue(
        &self,
        npc_id: &str,
        campaign_id: &str,
        raw_dialogue: &str,
    ) -> Result<StyledContent, PersonalityError> {
        let context = self.manager.get_context(campaign_id);

        let personality_id = context.npc_personalities.get(npc_id).cloned();

        let options = PersonalityApplicationOptions {
            personality_id,
            scene_mood: context.scene_mood,
            content_type: ContentType::Dialogue,
            include_thoughts: false,
            max_length: None,
        };

        self.manager.style_content(raw_dialogue, &options)
    }

    /// Build a complete system prompt for NPC chat
    pub fn build_npc_system_prompt(
        &self,
        npc_id: &str,
        campaign_id: &str,
        additional_context: Option<&str>,
    ) -> Result<String, PersonalityError> {
        let context = self.manager.get_context(campaign_id);
        let personality_id = context.npc_personalities.get(npc_id);

        let mut prompt = self.manager.build_personality_prompt(
            personality_id.map(|s| s.as_str()),
            ContentType::Dialogue,
            context.scene_mood.as_ref(),
        )?;

        if let Some(ctx) = additional_context {
            prompt.push_str(&format!("\nADDITIONAL CONTEXT:\n{}\n", ctx));
        }

        Ok(prompt)
    }
}

// ============================================================================
// Narration Tone Manager
// ============================================================================

/// Manager for applying narrative tone and style
pub struct NarrationStyleManager {
    manager: std::sync::Arc<PersonalityApplicationManager>,
}

impl NarrationStyleManager {
    pub fn new(manager: std::sync::Arc<PersonalityApplicationManager>) -> Self {
        Self { manager }
    }

    /// Get the system prompt for narration with campaign tone
    pub fn build_narration_prompt(
        &self,
        campaign_id: &str,
        narration_type: NarrationType,
    ) -> Result<String, PersonalityError> {
        let context = self.manager.get_context(campaign_id);

        let content_type = match narration_type {
            NarrationType::SceneDescription => ContentType::Description,
            NarrationType::Action => ContentType::Action,
            NarrationType::Transition => ContentType::Narration,
            NarrationType::Atmosphere => ContentType::Description,
        };

        let mut prompt = self.manager.build_personality_prompt(
            context.narrator_personality_id.as_deref(),
            content_type,
            context.scene_mood.as_ref(),
        )?;

        // Add narration-specific guidance
        prompt.push_str(&format!("\nNARRATION TYPE: {:?}\n", narration_type));

        Ok(prompt)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NarrationType {
    SceneDescription,
    Action,
    Transition,
    Atmosphere,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_management() {
        let store = std::sync::Arc::new(PersonalityStore::new());
        let manager = PersonalityApplicationManager::new(store);

        // Set and get context
        manager.set_narrator_personality("campaign1", Some("narrator1".to_string()));

        let context = manager.get_context("campaign1");
        assert_eq!(context.narrator_personality_id, Some("narrator1".to_string()));

        // Non-existent campaign should return default
        let context2 = manager.get_context("nonexistent");
        assert!(context2.narrator_personality_id.is_none());
    }

    #[test]
    fn test_scene_mood() {
        let store = std::sync::Arc::new(PersonalityStore::new());
        let manager = PersonalityApplicationManager::new(store);

        let mood = SceneMood {
            tone: "tense".to_string(),
            intensity: 8,
            description: "The party faces a dangerous foe".to_string(),
        };

        manager.set_scene_mood("campaign1", Some(mood.clone()));

        let context = manager.get_context("campaign1");
        assert!(context.scene_mood.is_some());
        assert_eq!(context.scene_mood.unwrap().tone, "tense");
    }
}
