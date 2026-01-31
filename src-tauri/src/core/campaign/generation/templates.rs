//! Template Registry - YAML template loading and management
//!
//! Phase 4, Task 4.1: Create generation template system
//!
//! Provides a registry for loading and caching generation templates from YAML files.
//! Templates define the structure and variables for LLM prompts.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during template operations
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("Template not found: {0}")]
    NotFound(String),

    #[error("Invalid template: {0}")]
    Invalid(String),

    #[error("Template parse error: {0}")]
    Parse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML error: {0}")]
    Yaml(String),

    #[error("Variable not provided: {0}")]
    MissingVariable(String),

    #[error("Template render error: {0}")]
    Render(String),
}

impl From<serde_yaml_ng::Error> for TemplateError {
    fn from(e: serde_yaml_ng::Error) -> Self {
        TemplateError::Yaml(e.to_string())
    }
}

// ============================================================================
// Template Types
// ============================================================================

/// Type of generation template
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateType {
    /// Character background generation
    CharacterBackground,
    /// NPC generation
    NpcGeneration,
    /// Session plan generation
    SessionPlan,
    /// Party composition analysis
    PartyComposition,
    /// Arc outline generation
    ArcOutline,
    /// Location generation
    LocationGeneration,
    /// Quest hook generation
    QuestHook,
    /// Encounter generation
    Encounter,
    /// Campaign summary generation
    CampaignSummary,
    /// Campaign pitch/preview generation
    CampaignPitch,
    /// Custom template
    Custom,
}

impl TemplateType {
    /// Get the default template filename for this type
    pub fn default_filename(&self) -> &'static str {
        match self {
            TemplateType::CharacterBackground => "character_background.yaml",
            TemplateType::NpcGeneration => "npc_generation.yaml",
            TemplateType::SessionPlan => "session_plan.yaml",
            TemplateType::PartyComposition => "party_composition.yaml",
            TemplateType::ArcOutline => "arc_outline.yaml",
            TemplateType::LocationGeneration => "location_generation.yaml",
            TemplateType::QuestHook => "quest_hook.yaml",
            TemplateType::Encounter => "encounter.yaml",
            TemplateType::CampaignSummary => "campaign_summary.yaml",
            TemplateType::CampaignPitch => "campaign_pitch.yaml",
            TemplateType::Custom => "custom.yaml",
        }
    }
}

impl std::fmt::Display for TemplateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TemplateType::CharacterBackground => "character_background",
            TemplateType::NpcGeneration => "npc_generation",
            TemplateType::SessionPlan => "session_plan",
            TemplateType::PartyComposition => "party_composition",
            TemplateType::ArcOutline => "arc_outline",
            TemplateType::LocationGeneration => "location_generation",
            TemplateType::QuestHook => "quest_hook",
            TemplateType::Encounter => "encounter",
            TemplateType::CampaignSummary => "campaign_summary",
            TemplateType::CampaignPitch => "campaign_pitch",
            TemplateType::Custom => "custom",
        };
        write!(f, "{}", s)
    }
}

/// Template variable definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    /// Variable name (used in template as {{name}})
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Whether this variable is required
    pub required: bool,
    /// Default value if not provided
    pub default: Option<String>,
    /// Example value for documentation
    pub example: Option<String>,
}

/// Template metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMetadata {
    /// Template name
    pub name: String,
    /// Template version
    pub version: String,
    /// Template description
    pub description: String,
    /// Author or source
    pub author: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Recommended model (e.g., "claude-3-5-sonnet", "gpt-4o")
    pub recommended_model: Option<String>,
    /// Estimated output tokens
    pub estimated_output_tokens: Option<u32>,
}

impl Default for TemplateMetadata {
    fn default() -> Self {
        Self {
            name: "Unnamed Template".to_string(),
            version: "1.0.0".to_string(),
            description: String::new(),
            author: None,
            tags: Vec::new(),
            recommended_model: None,
            estimated_output_tokens: None,
        }
    }
}

/// A generation template loaded from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationTemplate {
    /// Template metadata
    pub metadata: TemplateMetadata,
    /// Template type
    pub template_type: TemplateType,
    /// System prompt template
    pub system_prompt: String,
    /// User prompt template
    pub user_prompt: String,
    /// Variable definitions
    pub variables: Vec<TemplateVariable>,
    /// Output format instructions (appended to system prompt)
    pub output_format: Option<String>,
    /// Example output for few-shot learning
    pub example_output: Option<String>,
    /// Temperature recommendation
    pub temperature: Option<f32>,
    /// Max tokens recommendation
    pub max_tokens: Option<u32>,
}

impl GenerationTemplate {
    /// Create a new template with minimal required fields
    pub fn new(
        template_type: TemplateType,
        system_prompt: impl Into<String>,
        user_prompt: impl Into<String>,
    ) -> Self {
        Self {
            metadata: TemplateMetadata {
                name: template_type.to_string(),
                ..Default::default()
            },
            template_type,
            system_prompt: system_prompt.into(),
            user_prompt: user_prompt.into(),
            variables: Vec::new(),
            output_format: None,
            example_output: None,
            temperature: None,
            max_tokens: None,
        }
    }

    /// Render the system prompt with provided variables
    pub fn render_system_prompt(
        &self,
        variables: &HashMap<String, String>,
    ) -> Result<String, TemplateError> {
        let mut prompt = self.render_template(&self.system_prompt, variables)?;

        // Append output format if present
        if let Some(ref format) = self.output_format {
            prompt.push_str("\n\n");
            prompt.push_str(format);
        }

        Ok(prompt)
    }

    /// Render the user prompt with provided variables
    pub fn render_user_prompt(
        &self,
        variables: &HashMap<String, String>,
    ) -> Result<String, TemplateError> {
        self.render_template(&self.user_prompt, variables)
    }

    /// Render a template string with variables
    fn render_template(
        &self,
        template: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String, TemplateError> {
        let mut result = template.to_string();

        // Check for required variables
        for var_def in &self.variables {
            let placeholder = format!("{{{{{}}}}}", var_def.name);
            if template.contains(&placeholder) {
                if let Some(value) = variables.get(&var_def.name) {
                    result = result.replace(&placeholder, value);
                } else if let Some(ref default) = var_def.default {
                    result = result.replace(&placeholder, default);
                } else if var_def.required {
                    return Err(TemplateError::MissingVariable(var_def.name.clone()));
                } else {
                    // Optional variable with no default - remove placeholder
                    result = result.replace(&placeholder, "");
                }
            }
        }

        // Also replace any variables in the map that weren't in definitions
        // (for flexibility with dynamic variables)
        for (key, value) in variables {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }

        Ok(result)
    }

    /// Validate that all required variables are provided
    pub fn validate_variables(&self, variables: &HashMap<String, String>) -> Result<(), TemplateError> {
        for var_def in &self.variables {
            if var_def.required && !variables.contains_key(&var_def.name) && var_def.default.is_none() {
                return Err(TemplateError::MissingVariable(var_def.name.clone()));
            }
        }
        Ok(())
    }

    /// Get list of required variable names
    pub fn required_variables(&self) -> Vec<&str> {
        self.variables
            .iter()
            .filter(|v| v.required && v.default.is_none())
            .map(|v| v.name.as_str())
            .collect()
    }
}

// ============================================================================
// Template Registry
// ============================================================================

/// Registry for loading and caching generation templates
pub struct TemplateRegistry {
    /// Cached templates by type
    templates: Arc<RwLock<HashMap<TemplateType, GenerationTemplate>>>,
    /// Custom templates by name
    custom_templates: Arc<RwLock<HashMap<String, GenerationTemplate>>>,
    /// Base directory for templates
    base_dir: Option<PathBuf>,
}

impl TemplateRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            templates: Arc::new(RwLock::new(HashMap::new())),
            custom_templates: Arc::new(RwLock::new(HashMap::new())),
            base_dir: None,
        }
    }

    /// Create a registry with default templates loaded from a directory
    pub async fn load_from_dir(dir: impl AsRef<Path>) -> Result<Self, TemplateError> {
        let dir = dir.as_ref();
        let registry = Self {
            templates: Arc::new(RwLock::new(HashMap::new())),
            custom_templates: Arc::new(RwLock::new(HashMap::new())),
            base_dir: Some(dir.to_path_buf()),
        };

        // Load all default template types
        for template_type in [
            TemplateType::CharacterBackground,
            TemplateType::NpcGeneration,
            TemplateType::SessionPlan,
            TemplateType::PartyComposition,
            TemplateType::ArcOutline,
        ] {
            let path = dir.join(template_type.default_filename());
            if path.exists() {
                let template = registry.load_template_file(&path).await?;
                registry.templates.write().await.insert(template_type, template);
            }
        }

        Ok(registry)
    }

    /// Create a registry with embedded default templates
    pub async fn with_defaults() -> Self {
        let registry = Self::new();

        // Populate defaults using async write lock
        {
            let mut templates = registry.templates.write().await;
            for template_type in [
                TemplateType::CharacterBackground,
                TemplateType::NpcGeneration,
                TemplateType::SessionPlan,
                TemplateType::PartyComposition,
                TemplateType::ArcOutline,
                TemplateType::CampaignSummary,
                TemplateType::CampaignPitch,
            ] {
                templates.insert(template_type, Self::default_template(template_type));
            }
        }

        registry
    }

    /// Load a template from a YAML file
    async fn load_template_file(&self, path: &Path) -> Result<GenerationTemplate, TemplateError> {
        let content = tokio::fs::read_to_string(path).await?;
        let template: GenerationTemplate = serde_yaml_ng::from_str(&content)?;
        Ok(template)
    }

    /// Register a template for a specific type
    pub async fn register(&self, template: GenerationTemplate) {
        self.templates
            .write()
            .await
            .insert(template.template_type, template);
    }

    /// Register a custom template by name
    pub async fn register_custom(&self, name: impl Into<String>, template: GenerationTemplate) {
        self.custom_templates
            .write()
            .await
            .insert(name.into(), template);
    }

    /// Get a template by type
    pub async fn get(&self, template_type: TemplateType) -> Option<GenerationTemplate> {
        self.templates.read().await.get(&template_type).cloned()
    }

    /// Get a template by type, falling back to embedded default
    pub async fn get_or_default(&self, template_type: TemplateType) -> GenerationTemplate {
        if let Some(template) = self.get(template_type).await {
            return template;
        }

        // Return embedded default
        Self::default_template(template_type)
    }

    /// Get a custom template by name
    pub async fn get_custom(&self, name: &str) -> Option<GenerationTemplate> {
        self.custom_templates.read().await.get(name).cloned()
    }

    /// List all registered template types
    pub async fn list_types(&self) -> Vec<TemplateType> {
        self.templates.read().await.keys().copied().collect()
    }

    /// List all custom template names
    pub async fn list_custom(&self) -> Vec<String> {
        self.custom_templates.read().await.keys().cloned().collect()
    }

    /// Reload templates from disk (if base_dir is set)
    pub async fn reload(&self) -> Result<(), TemplateError> {
        if let Some(ref dir) = self.base_dir {
            for template_type in [
                TemplateType::CharacterBackground,
                TemplateType::NpcGeneration,
                TemplateType::SessionPlan,
                TemplateType::PartyComposition,
                TemplateType::ArcOutline,
                TemplateType::CampaignSummary,
                TemplateType::CampaignPitch,
            ] {
                let path = dir.join(template_type.default_filename());
                if path.exists() {
                    let template = self.load_template_file(&path).await?;
                    self.templates.write().await.insert(template_type, template);
                }
            }
        }
        Ok(())
    }

    /// Get the default embedded template for a type
    pub fn default_template(template_type: TemplateType) -> GenerationTemplate {
        match template_type {
            TemplateType::CharacterBackground => Self::default_character_background(),
            TemplateType::NpcGeneration => Self::default_npc_generation(),
            TemplateType::SessionPlan => Self::default_session_plan(),
            TemplateType::PartyComposition => Self::default_party_composition(),
            TemplateType::ArcOutline => Self::default_arc_outline(),
            TemplateType::CampaignSummary => Self::default_campaign_summary(),
            TemplateType::CampaignPitch => Self::default_campaign_pitch(),
            _ => GenerationTemplate::new(
                template_type,
                "You are a helpful TTRPG assistant.",
                "{{prompt}}",
            ),
        }
    }

    fn default_character_background() -> GenerationTemplate {
        GenerationTemplate {
            metadata: TemplateMetadata {
                name: "Character Background Generator".to_string(),
                version: "1.0.0".to_string(),
                description: "Generate rich character backstories with plot hooks".to_string(),
                author: Some("TTRPG Assistant".to_string()),
                tags: vec!["character".to_string(), "backstory".to_string(), "roleplay".to_string()],
                recommended_model: Some("claude-3-5-sonnet".to_string()),
                estimated_output_tokens: Some(1500),
            },
            template_type: TemplateType::CharacterBackground,
            system_prompt: r#"You are an expert TTRPG character designer specializing in creating compelling backstories that integrate with campaign settings.

Your role is to generate character backgrounds that:
1. Feel authentic to the game system and setting
2. Include meaningful relationships (family, mentors, rivals)
3. Provide plot hooks the GM can use
4. Balance tragedy and triumph
5. Leave room for character growth

Campaign Context:
{{campaign_context}}

Campaign Intent:
- Fantasy: {{fantasy}}
- Themes: {{themes}}
- Tone: {{tone}}
- Avoid: {{avoid}}"#.to_string(),
            user_prompt: r#"Generate a character background for:

Character Details:
- Name: {{character_name}}
- Class: {{character_class}}
- Race: {{character_race}}
- Level: {{character_level}}

Player Request:
{{player_request}}

Additional Context:
{{additional_context}}"#.to_string(),
            variables: vec![
                TemplateVariable {
                    name: "campaign_context".to_string(),
                    description: "Summary of the campaign setting and current state".to_string(),
                    required: false,
                    default: Some("A fantasy world with standard D&D elements".to_string()),
                    example: Some("The Sword Coast during the late 1400s DR".to_string()),
                },
                TemplateVariable {
                    name: "fantasy".to_string(),
                    description: "Core fantasy of the campaign".to_string(),
                    required: false,
                    default: Some("high fantasy adventure".to_string()),
                    example: Some("grim political thriller".to_string()),
                },
                TemplateVariable {
                    name: "themes".to_string(),
                    description: "Campaign themes".to_string(),
                    required: false,
                    default: Some("adventure, heroism".to_string()),
                    example: Some("corruption of power, found family".to_string()),
                },
                TemplateVariable {
                    name: "tone".to_string(),
                    description: "Campaign tone".to_string(),
                    required: false,
                    default: Some("heroic, adventurous".to_string()),
                    example: Some("dark, gritty, with moments of hope".to_string()),
                },
                TemplateVariable {
                    name: "avoid".to_string(),
                    description: "Topics to avoid".to_string(),
                    required: false,
                    default: Some("".to_string()),
                    example: Some("graphic violence, romance".to_string()),
                },
                TemplateVariable {
                    name: "character_name".to_string(),
                    description: "Character's name".to_string(),
                    required: true,
                    default: None,
                    example: Some("Elara Nightwood".to_string()),
                },
                TemplateVariable {
                    name: "character_class".to_string(),
                    description: "Character's class".to_string(),
                    required: true,
                    default: None,
                    example: Some("Ranger".to_string()),
                },
                TemplateVariable {
                    name: "character_race".to_string(),
                    description: "Character's race".to_string(),
                    required: true,
                    default: None,
                    example: Some("Half-Elf".to_string()),
                },
                TemplateVariable {
                    name: "character_level".to_string(),
                    description: "Starting level".to_string(),
                    required: false,
                    default: Some("1".to_string()),
                    example: Some("5".to_string()),
                },
                TemplateVariable {
                    name: "player_request".to_string(),
                    description: "Player's specific requests for the background".to_string(),
                    required: true,
                    default: None,
                    example: Some("I want my character to have a mysterious past involving a secret organization".to_string()),
                },
                TemplateVariable {
                    name: "additional_context".to_string(),
                    description: "Any additional context or constraints".to_string(),
                    required: false,
                    default: Some("".to_string()),
                    example: Some("The character should be from a noble family that fell from grace".to_string()),
                },
            ],
            output_format: Some(r#"Output Format (JSON):
{
  "background": {
    "summary": "A 2-3 sentence summary",
    "origin": "Where and how they grew up",
    "formative_event": "A key event that shaped them",
    "motivation": "What drives them now",
    "personality_traits": ["trait1", "trait2"],
    "ideal": "Their guiding principle",
    "bond": "What they're tied to",
    "flaw": "A weakness or vulnerability"
  },
  "relationships": [
    {
      "name": "NPC name",
      "relationship": "friend/mentor/rival/family",
      "status": "alive/dead/unknown",
      "description": "Brief description",
      "plot_hook_potential": true
    }
  ],
  "locations": [
    {
      "name": "Place name",
      "significance": "Why it matters",
      "revisitable": true
    }
  ],
  "plot_hooks": [
    {
      "title": "Hook title",
      "description": "What could happen",
      "urgency": "low/medium/high"
    }
  ],
  "secrets": ["Things the character might not know or reveal"]
}"#.to_string()),
            example_output: None,
            temperature: Some(0.8),
            max_tokens: Some(2000),
        }
    }

    fn default_npc_generation() -> GenerationTemplate {
        GenerationTemplate {
            metadata: TemplateMetadata {
                name: "NPC Generator".to_string(),
                version: "1.0.0".to_string(),
                description: "Generate NPCs with personality, motivations, and stat blocks".to_string(),
                author: Some("TTRPG Assistant".to_string()),
                tags: vec!["npc".to_string(), "character".to_string(), "encounter".to_string()],
                recommended_model: Some("claude-3-5-sonnet".to_string()),
                estimated_output_tokens: Some(1200),
            },
            template_type: TemplateType::NpcGeneration,
            system_prompt: r#"You are an expert TTRPG NPC designer who creates memorable, three-dimensional characters.

Your NPCs should:
1. Have clear motivations and goals
2. Possess distinctive personality traits and speech patterns
3. Fit naturally into the campaign setting
4. Provide opportunities for player interaction
5. Have appropriate stat blocks when requested

Campaign Context:
{{campaign_context}}

Campaign Intent:
- Fantasy: {{fantasy}}
- Themes: {{themes}}
- Tone: {{tone}}"#.to_string(),
            user_prompt: r#"Generate an NPC with the following specifications:

Role: {{npc_role}}
Importance: {{importance}}
Location: {{location}}

Description/Request:
{{description}}

Stat Block Required: {{include_stats}}
Game System: {{game_system}}"#.to_string(),
            variables: vec![
                TemplateVariable {
                    name: "campaign_context".to_string(),
                    description: "Campaign setting summary".to_string(),
                    required: false,
                    default: Some("Standard fantasy setting".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "fantasy".to_string(),
                    description: "Campaign fantasy".to_string(),
                    required: false,
                    default: Some("high fantasy".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "themes".to_string(),
                    description: "Campaign themes".to_string(),
                    required: false,
                    default: Some("adventure".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "tone".to_string(),
                    description: "Campaign tone".to_string(),
                    required: false,
                    default: Some("heroic".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "npc_role".to_string(),
                    description: "NPC's role (merchant, guard, villain, etc.)".to_string(),
                    required: true,
                    default: None,
                    example: Some("tavern keeper".to_string()),
                },
                TemplateVariable {
                    name: "importance".to_string(),
                    description: "NPC importance level".to_string(),
                    required: false,
                    default: Some("minor".to_string()),
                    example: Some("major".to_string()),
                },
                TemplateVariable {
                    name: "location".to_string(),
                    description: "Where the NPC is found".to_string(),
                    required: false,
                    default: Some("".to_string()),
                    example: Some("The Rusty Anchor tavern in Waterdeep".to_string()),
                },
                TemplateVariable {
                    name: "description".to_string(),
                    description: "User's description or requirements".to_string(),
                    required: true,
                    default: None,
                    example: Some("A gruff dwarf who secretly works for the thieves guild".to_string()),
                },
                TemplateVariable {
                    name: "include_stats".to_string(),
                    description: "Whether to include stat block".to_string(),
                    required: false,
                    default: Some("false".to_string()),
                    example: Some("true".to_string()),
                },
                TemplateVariable {
                    name: "game_system".to_string(),
                    description: "Game system for stat blocks".to_string(),
                    required: false,
                    default: Some("dnd5e".to_string()),
                    example: Some("pathfinder2e".to_string()),
                },
            ],
            output_format: Some(r#"Output Format (JSON):
{
  "npc": {
    "name": "Full name",
    "title": "Optional title or epithet",
    "race": "Species/ancestry",
    "gender": "Gender identity",
    "age": "Approximate age or age category",
    "occupation": "What they do",
    "appearance": "Physical description",
    "personality": {
      "traits": ["trait1", "trait2"],
      "ideal": "Guiding principle",
      "bond": "Important connection",
      "flaw": "Weakness"
    },
    "voice": {
      "speech_pattern": "How they talk",
      "catchphrase": "A memorable phrase",
      "accent": "Optional accent notes"
    },
    "motivation": "What they want",
    "secret": "Something hidden",
    "relationships": [
      {"name": "NPC name", "type": "relationship type"}
    ]
  },
  "stat_block": {
    "cr": "Challenge rating if applicable",
    "type": "Creature type",
    "stats": {}
  },
  "quest_hooks": ["Potential quests involving this NPC"]
}"#.to_string()),
            example_output: None,
            temperature: Some(0.7),
            max_tokens: Some(1500),
        }
    }

    fn default_session_plan() -> GenerationTemplate {
        GenerationTemplate {
            metadata: TemplateMetadata {
                name: "Session Plan Generator".to_string(),
                version: "1.0.0".to_string(),
                description: "Generate structured session plans with pacing and encounters".to_string(),
                author: Some("TTRPG Assistant".to_string()),
                tags: vec!["session".to_string(), "planning".to_string(), "encounter".to_string()],
                recommended_model: Some("claude-3-5-sonnet".to_string()),
                estimated_output_tokens: Some(2000),
            },
            template_type: TemplateType::SessionPlan,
            system_prompt: r#"You are an expert TTRPG session planner who creates engaging, well-paced adventures.

Your session plans should:
1. Balance combat, roleplay, and exploration
2. Include contingencies for player choices
3. Have clear objectives and dramatic beats
4. Fit within the specified time budget
5. Advance the campaign's plot arcs

Campaign Context:
{{campaign_context}}

Party Composition:
{{party_info}}

Current Plot State:
{{plot_state}}"#.to_string(),
            user_prompt: r#"Create a session plan for:

Session Duration: {{session_duration}} hours
Pacing Style: {{pacing_style}}
Main Objective: {{objective}}

Previous Session Summary:
{{previous_session}}

Active Plot Threads:
{{active_plots}}

GM Notes:
{{gm_notes}}"#.to_string(),
            variables: vec![
                TemplateVariable {
                    name: "campaign_context".to_string(),
                    description: "Campaign setting".to_string(),
                    required: false,
                    default: Some("Fantasy adventure campaign".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "party_info".to_string(),
                    description: "Party composition details".to_string(),
                    required: false,
                    default: Some("A balanced party of adventurers".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "plot_state".to_string(),
                    description: "Current state of campaign plots".to_string(),
                    required: false,
                    default: Some("".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "session_duration".to_string(),
                    description: "Expected session length in hours".to_string(),
                    required: false,
                    default: Some("3".to_string()),
                    example: Some("4".to_string()),
                },
                TemplateVariable {
                    name: "pacing_style".to_string(),
                    description: "Pacing preference".to_string(),
                    required: false,
                    default: Some("balanced".to_string()),
                    example: Some("combat-heavy".to_string()),
                },
                TemplateVariable {
                    name: "objective".to_string(),
                    description: "Main session objective".to_string(),
                    required: true,
                    default: None,
                    example: Some("The party must infiltrate the noble's mansion".to_string()),
                },
                TemplateVariable {
                    name: "previous_session".to_string(),
                    description: "Summary of previous session".to_string(),
                    required: false,
                    default: Some("".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "active_plots".to_string(),
                    description: "Currently active plot threads".to_string(),
                    required: false,
                    default: Some("".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "gm_notes".to_string(),
                    description: "GM's specific notes or requests".to_string(),
                    required: false,
                    default: Some("".to_string()),
                    example: None,
                },
            ],
            output_format: Some(r#"Output Format (JSON):
{
  "session_plan": {
    "title": "Session title",
    "objective": "Primary objective",
    "estimated_duration_hours": 3,
    "beats": [
      {
        "name": "Beat name",
        "type": "opening/rising_action/climax/falling_action/cliffhanger",
        "duration_minutes": 30,
        "description": "What happens",
        "encounter": {
          "type": "combat/social/exploration/puzzle",
          "difficulty": "easy/medium/hard/deadly",
          "participants": ["list of NPCs/monsters"]
        },
        "contingencies": ["What if players do X", "What if players do Y"]
      }
    ],
    "npcs_involved": ["NPC names"],
    "locations": ["Location names"],
    "loot_rewards": ["Potential rewards"],
    "plot_advancement": "How this advances the story",
    "cliffhanger_options": ["Possible session endings"]
  }
}"#.to_string()),
            example_output: None,
            temperature: Some(0.7),
            max_tokens: Some(2500),
        }
    }

    fn default_party_composition() -> GenerationTemplate {
        GenerationTemplate {
            metadata: TemplateMetadata {
                name: "Party Composition Analyzer".to_string(),
                version: "1.0.0".to_string(),
                description: "Analyze party composition and suggest improvements".to_string(),
                author: Some("TTRPG Assistant".to_string()),
                tags: vec!["party".to_string(), "analysis".to_string(), "balance".to_string()],
                recommended_model: Some("claude-3-5-sonnet".to_string()),
                estimated_output_tokens: Some(1000),
            },
            template_type: TemplateType::PartyComposition,
            system_prompt: r#"You are an expert TTRPG party composition analyst who helps GMs understand party dynamics.

Your analysis should:
1. Identify party strengths and weaknesses
2. Suggest ways to address gaps (NPCs, items, encounter design)
3. Consider both combat and non-combat capabilities
4. Be constructive and solution-oriented

Game System: {{game_system}}"#.to_string(),
            user_prompt: r#"Analyze this party composition:

{{party_details}}

Campaign Type: {{campaign_type}}
Expected Challenges: {{expected_challenges}}"#.to_string(),
            variables: vec![
                TemplateVariable {
                    name: "game_system".to_string(),
                    description: "Game system".to_string(),
                    required: false,
                    default: Some("D&D 5e".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "party_details".to_string(),
                    description: "Party member details".to_string(),
                    required: true,
                    default: None,
                    example: Some("Fighter 5, Wizard 5, Rogue 5".to_string()),
                },
                TemplateVariable {
                    name: "campaign_type".to_string(),
                    description: "Type of campaign".to_string(),
                    required: false,
                    default: Some("balanced adventure".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "expected_challenges".to_string(),
                    description: "Expected challenge types".to_string(),
                    required: false,
                    default: Some("combat, exploration, social".to_string()),
                    example: None,
                },
            ],
            output_format: Some(r#"Output Format (JSON):
{
  "analysis": {
    "overall_balance_score": 75,
    "strengths": ["List of party strengths"],
    "weaknesses": ["List of gaps or weaknesses"],
    "combat_analysis": {
      "damage_output": "assessment",
      "survivability": "assessment",
      "control": "assessment"
    },
    "utility_analysis": {
      "healing": "none/limited/adequate/strong",
      "exploration": "assessment",
      "social": "assessment"
    }
  },
  "recommendations": [
    {
      "gap": "What's missing",
      "priority": "high/medium/low",
      "solutions": [
        {"type": "npc_companion", "suggestion": "..."},
        {"type": "magic_item", "suggestion": "..."},
        {"type": "encounter_design", "suggestion": "..."}
      ]
    }
  ]
}"#.to_string()),
            example_output: None,
            temperature: Some(0.5),
            max_tokens: Some(1200),
        }
    }

    fn default_arc_outline() -> GenerationTemplate {
        GenerationTemplate {
            metadata: TemplateMetadata {
                name: "Arc Outline Generator".to_string(),
                version: "1.0.0".to_string(),
                description: "Generate narrative arc outlines with tension curves".to_string(),
                author: Some("TTRPG Assistant".to_string()),
                tags: vec!["arc".to_string(), "narrative".to_string(), "planning".to_string()],
                recommended_model: Some("claude-3-5-sonnet".to_string()),
                estimated_output_tokens: Some(1800),
            },
            template_type: TemplateType::ArcOutline,
            system_prompt: r#"You are an expert narrative designer for TTRPG campaigns who creates compelling story arcs.

Your arcs should:
1. Follow strong narrative structure (setup, rising action, climax, resolution)
2. Include meaningful player agency moments
3. Build appropriate tension
4. Connect to the campaign's themes
5. Provide multiple possible outcomes

Campaign Intent:
- Fantasy: {{fantasy}}
- Themes: {{themes}}
- Player Experiences: {{player_experiences}}
- Avoid: {{avoid}}"#.to_string(),
            user_prompt: r#"Create an arc outline for:

Arc Concept: {{arc_concept}}
Arc Type: {{arc_type}}
Estimated Sessions: {{estimated_sessions}}
Party Level Range: {{level_range}}

Current Campaign State:
{{campaign_state}}

Key NPCs Available:
{{available_npcs}}

Locations:
{{available_locations}}"#.to_string(),
            variables: vec![
                TemplateVariable {
                    name: "fantasy".to_string(),
                    description: "Campaign fantasy".to_string(),
                    required: false,
                    default: Some("high fantasy adventure".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "themes".to_string(),
                    description: "Campaign themes".to_string(),
                    required: false,
                    default: Some("heroism, adventure".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "player_experiences".to_string(),
                    description: "Desired player experiences".to_string(),
                    required: false,
                    default: Some("discovery, combat, roleplay".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "avoid".to_string(),
                    description: "Topics to avoid".to_string(),
                    required: false,
                    default: Some("".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "arc_concept".to_string(),
                    description: "High-level arc concept".to_string(),
                    required: true,
                    default: None,
                    example: Some("A necromancer threatens the kingdom".to_string()),
                },
                TemplateVariable {
                    name: "arc_type".to_string(),
                    description: "Type of arc".to_string(),
                    required: false,
                    default: Some("three_act".to_string()),
                    example: Some("mystery".to_string()),
                },
                TemplateVariable {
                    name: "estimated_sessions".to_string(),
                    description: "Estimated number of sessions".to_string(),
                    required: false,
                    default: Some("5-8".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "level_range".to_string(),
                    description: "Party level range".to_string(),
                    required: false,
                    default: Some("1-5".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "campaign_state".to_string(),
                    description: "Current campaign state".to_string(),
                    required: false,
                    default: Some("".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "available_npcs".to_string(),
                    description: "NPCs available to use".to_string(),
                    required: false,
                    default: Some("".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "available_locations".to_string(),
                    description: "Locations available".to_string(),
                    required: false,
                    default: Some("".to_string()),
                    example: None,
                },
            ],
            output_format: Some(r#"Output Format (JSON):
{
  "arc": {
    "title": "Arc title",
    "tagline": "One-line description",
    "type": "Arc type/structure",
    "themes": ["Themes explored"],
    "estimated_sessions": 6,
    "level_range": {"start": 1, "end": 5}
  },
  "phases": [
    {
      "name": "Phase name",
      "type": "setup/rising_action/climax/falling_action/resolution",
      "sessions": 2,
      "tension_level": 3,
      "objectives": ["What should happen"],
      "key_scenes": ["Important moments"],
      "decision_points": ["Player choice moments"],
      "npcs_introduced": ["New NPCs"],
      "locations": ["Where it happens"]
    }
  ],
  "tension_curve": {
    "points": [
      {"session": 1, "tension": 2, "event": "Hook"},
      {"session": 3, "tension": 5, "event": "Midpoint twist"},
      {"session": 5, "tension": 9, "event": "Climax"}
    ]
  },
  "possible_outcomes": [
    {
      "name": "Outcome name",
      "likelihood": "likely/possible/unlikely",
      "consequences": "What happens next"
    }
  ],
  "antagonist": {
    "name": "Antagonist name",
    "motivation": "Why they're doing this",
    "resources": ["What they have"],
    "escalation_plan": ["How they respond to party interference"]
  }
}"#.to_string()),
            example_output: None,
            temperature: Some(0.8),
            max_tokens: Some(2200),
        }
    }

    fn default_campaign_pitch() -> GenerationTemplate {
        GenerationTemplate {
            metadata: TemplateMetadata {
                name: "Campaign Pitch Generator".to_string(),
                version: "1.0.0".to_string(),
                description: "Generate a compelling pitch or preview for a campaign".to_string(),
                author: Some("TTRPG Assistant".to_string()),
                tags: vec!["campaign".to_string(), "pitch".to_string(), "introduction".to_string()],
                recommended_model: Some("claude-3-5-sonnet".to_string()),
                estimated_output_tokens: Some(1000),
            },
            template_type: TemplateType::CampaignPitch,
            system_prompt: r#"You are a master storyteller and TTRPG campaign designer.
Your goal is to create a compelling "pitch" or "back-of-the-book" blurb for a campaign.

The pitch should:
1. Highlight the central conflict and stakes
2. Evoke the campaign's unique tone and atmosphere
3. Hint at the mysteries or challenges ahead
4. Speak directly to the players' potential role in the world

Campaign Context:
{{campaign_context}}"#.to_string(),
            user_prompt: r#"Create a campaign pitch based on:
Fantasy: {{fantasy}}
Themes: {{themes}}
Tone: {{tone}}
Key Elements: {{key_elements}}"#.to_string(),
            variables: vec![
                TemplateVariable {
                    name: "campaign_context".to_string(),
                    description: "High-level summary of the setting".to_string(),
                    required: true,
                    default: None,
                    example: None,
                },
                TemplateVariable {
                    name: "fantasy".to_string(),
                    description: "Core fantasy premise".to_string(),
                    required: true,
                    default: None,
                    example: None,
                },
                TemplateVariable {
                    name: "themes".to_string(),
                    description: "Major campaign themes".to_string(),
                    required: false,
                    default: Some("adventure".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "tone".to_string(),
                    description: "Desired tone".to_string(),
                    required: false,
                    default: Some("heroic".to_string()),
                    example: None,
                },
                TemplateVariable {
                    name: "key_elements".to_string(),
                    description: "Specific elements to include".to_string(),
                    required: false,
                    default: Some("".to_string()),
                    example: None,
                },
            ],
            output_format: Some(r#"Output Format (JSON):
{
  "title": "Evocative campaign title",
  "tagline": "Short, punchy hook",
  "pitch": "The 2-3 paragraph main pitch",
  "key_hooks": ["Hook 1", "Hook 2"],
  "estimated_levels": "e.g., 1-10",
  "tone_keywords": ["keyword1", "keyword2"]
}"#.to_string()),
            example_output: None,
            temperature: Some(0.85),
            max_tokens: Some(1000),
        }
    }

    fn default_campaign_summary() -> GenerationTemplate {
        GenerationTemplate {
            metadata: TemplateMetadata {
                name: "Campaign Summary Generator".to_string(),
                version: "1.0.0".to_string(),
                description: "Summarize campaign progress and current state".to_string(),
                author: Some("TTRPG Assistant".to_string()),
                tags: vec!["campaign".to_string(), "summary".to_string()],
                recommended_model: Some("claude-3-5-sonnet".to_string()),
                estimated_output_tokens: Some(1500),
            },
            template_type: TemplateType::CampaignSummary,
            system_prompt: r#"You are a chronicler of epic adventures.
Your task is to provide a concise yet comprehensive summary of a campaign's current state.

Focus on:
1. Major completed milestones
2. Active plot threads and mysteries
3. Current party status and notable allies/enemies
4. Significant changes to the world state

Campaign Context:
{{campaign_context}}"#.to_string(),
            user_prompt: r#"Summarize the campaign based on the following session history:
{{session_summaries}}

Active Plots:
{{active_plots}}"#.to_string(),
            variables: vec![
                TemplateVariable {
                    name: "campaign_context".to_string(),
                    description: "Original campaign premise".to_string(),
                    required: true,
                    default: None,
                    example: None,
                },
                TemplateVariable {
                    name: "session_summaries".to_string(),
                    description: "Bullet points of previous sessions".to_string(),
                    required: true,
                    default: None,
                    example: None,
                },
                TemplateVariable {
                    name: "active_plots".to_string(),
                    description: "Currently unresolved plot threads".to_string(),
                    required: false,
                    default: Some("".to_string()),
                    example: None,
                },
            ],
            output_format: Some(r#"Output Format (JSON):
{
  "summary": "Main narrative summary",
  "milestones": ["Completed milestone 1", "Completed milestone 2"],
  "active_threads": [
    {"title": "Thread name", "status": "Current status/clues"}
  ],
  "world_changes": ["Change 1", "Change 2"],
  "dramatic_question": "The main question currently facing the party"
}"#.to_string()),
            example_output: None,
            temperature: Some(0.7),
            max_tokens: Some(1500),
        }
    }
}


impl Default for TemplateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_type_display() {
        assert_eq!(TemplateType::CharacterBackground.to_string(), "character_background");
        assert_eq!(TemplateType::NpcGeneration.to_string(), "npc_generation");
    }

    #[test]
    fn test_template_rendering() {
        let template = GenerationTemplate::new(
            TemplateType::Custom,
            "System prompt with {{variable}}",
            "User prompt with {{name}} and {{value}}",
        );

        let mut vars = HashMap::new();
        vars.insert("variable".to_string(), "test".to_string());
        vars.insert("name".to_string(), "Alice".to_string());
        vars.insert("value".to_string(), "42".to_string());

        let system = template.render_system_prompt(&vars).unwrap();
        let user = template.render_user_prompt(&vars).unwrap();

        assert_eq!(system, "System prompt with test");
        assert_eq!(user, "User prompt with Alice and 42");
    }

    #[test]
    fn test_missing_required_variable() {
        let mut template = GenerationTemplate::new(
            TemplateType::Custom,
            "Hello {{name}}",
            "World",
        );
        template.variables.push(TemplateVariable {
            name: "name".to_string(),
            description: "A name".to_string(),
            required: true,
            default: None,
            example: None,
        });

        let vars = HashMap::new();
        let result = template.render_system_prompt(&vars);

        assert!(matches!(result, Err(TemplateError::MissingVariable(_))));
    }

    #[test]
    fn test_default_value_used() {
        let mut template = GenerationTemplate::new(
            TemplateType::Custom,
            "Hello {{name}}",
            "World",
        );
        template.variables.push(TemplateVariable {
            name: "name".to_string(),
            description: "A name".to_string(),
            required: true,
            default: Some("World".to_string()),
            example: None,
        });

        let vars = HashMap::new();
        let result = template.render_system_prompt(&vars).unwrap();

        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_required_variables_list() {
        let mut template = GenerationTemplate::new(
            TemplateType::Custom,
            "Test",
            "Test",
        );
        template.variables = vec![
            TemplateVariable {
                name: "required1".to_string(),
                description: "".to_string(),
                required: true,
                default: None,
                example: None,
            },
            TemplateVariable {
                name: "optional".to_string(),
                description: "".to_string(),
                required: false,
                default: None,
                example: None,
            },
            TemplateVariable {
                name: "required_with_default".to_string(),
                description: "".to_string(),
                required: true,
                default: Some("default".to_string()),
                example: None,
            },
        ];

        let required = template.required_variables();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "required1");
    }

    #[test]
    fn test_default_templates() {
        let char_template = TemplateRegistry::default_template(TemplateType::CharacterBackground);
        assert_eq!(char_template.template_type, TemplateType::CharacterBackground);
        assert!(!char_template.system_prompt.is_empty());
        assert!(!char_template.variables.is_empty());

        let npc_template = TemplateRegistry::default_template(TemplateType::NpcGeneration);
        assert_eq!(npc_template.template_type, TemplateType::NpcGeneration);

        let session_template = TemplateRegistry::default_template(TemplateType::SessionPlan);
        assert_eq!(session_template.template_type, TemplateType::SessionPlan);
    }

    #[tokio::test]
    async fn test_registry_operations() {
        let registry = TemplateRegistry::new();

        // Initially empty
        let types = registry.list_types().await;
        assert!(types.is_empty());

        // Register a template
        let template = TemplateRegistry::default_template(TemplateType::NpcGeneration);
        registry.register(template).await;

        // Now has one type
        let types = registry.list_types().await;
        assert_eq!(types.len(), 1);
        assert!(types.contains(&TemplateType::NpcGeneration));

        // Can retrieve it
        let retrieved = registry.get(TemplateType::NpcGeneration).await;
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_get_or_default() {
        let registry = TemplateRegistry::new();

        // Not registered, should return default
        let template = registry.get_or_default(TemplateType::CharacterBackground).await;
        assert_eq!(template.template_type, TemplateType::CharacterBackground);
    }
}
