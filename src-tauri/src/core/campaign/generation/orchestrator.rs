//! Generation Orchestrator - Core coordination for LLM-powered generation
//!
//! Phase 4, Task 4.2: Implement GenerationOrchestrator core
//!
//! The orchestrator coordinates template loading, context assembly, LLM calls,
//! and trust assignment for all generation operations.

use super::context::{ContextAssembler, ContextError};
use super::templates::{GenerationTemplate, TemplateRegistry, TemplateError, TemplateType};
use super::trust::{TrustAssigner, TrustAssignment};
use crate::core::campaign::grounding::{FlavourSearcher, RulebookLinker, UsageTracker};
use crate::core::campaign::pipeline::{CampaignIntent, PipelineError};
use crate::core::llm::{ChatMessage, ChatRequest, ChatResponse, LLMRouter};
use crate::core::search::SearchClient;
use crate::database::{CampaignOps, Citation, Database};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during generation
#[derive(Debug, thiserror::Error)]
pub enum GenerationError {
    #[error("Template error: {0}")]
    Template(#[from] TemplateError),

    #[error("Context error: {0}")]
    Context(#[from] ContextError),

    #[error("Pipeline error: {0}")]
    Pipeline(#[from] PipelineError),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Campaign not found: {0}")]
    CampaignNotFound(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<String> for GenerationError {
    fn from(s: String) -> Self {
        GenerationError::Internal(s)
    }
}

// ============================================================================
// Generation Types
// ============================================================================

/// Type of content being generated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationType {
    /// Character background
    CharacterBackground,
    /// NPC generation
    Npc,
    /// Session plan
    SessionPlan,
    /// Party composition analysis
    PartyAnalysis,
    /// Arc outline
    ArcOutline,
    /// Location
    Location,
    /// Quest hook
    QuestHook,
    /// Encounter
    Encounter,
    /// Custom/free-form
    Custom,
}

impl GenerationType {
    /// Get the corresponding template type
    pub fn template_type(&self) -> TemplateType {
        match self {
            GenerationType::CharacterBackground => TemplateType::CharacterBackground,
            GenerationType::Npc => TemplateType::NpcGeneration,
            GenerationType::SessionPlan => TemplateType::SessionPlan,
            GenerationType::PartyAnalysis => TemplateType::PartyComposition,
            GenerationType::ArcOutline => TemplateType::ArcOutline,
            GenerationType::Location => TemplateType::LocationGeneration,
            GenerationType::QuestHook => TemplateType::QuestHook,
            GenerationType::Encounter => TemplateType::Encounter,
            GenerationType::Custom => TemplateType::Custom,
        }
    }

    /// Get the entity type string for database storage
    pub fn entity_type(&self) -> &'static str {
        match self {
            GenerationType::CharacterBackground => "character_background",
            GenerationType::Npc => "npc",
            GenerationType::SessionPlan => "session_plan",
            GenerationType::PartyAnalysis => "party_analysis",
            GenerationType::ArcOutline => "arc",
            GenerationType::Location => "location",
            GenerationType::QuestHook => "quest_hook",
            GenerationType::Encounter => "encounter",
            GenerationType::Custom => "custom",
        }
    }
}

/// Configuration for generation operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    /// Temperature for LLM calls (0.0-1.0)
    pub temperature: Option<f32>,
    /// Maximum tokens for output
    pub max_tokens: Option<u32>,
    /// Preferred provider
    pub provider: Option<String>,
    /// Preferred model
    pub model: Option<String>,
    /// Token budget for context
    pub token_budget: Option<u32>,
    /// Whether to include grounding citations
    pub include_citations: bool,
    /// Whether to save drafts to database
    pub save_drafts: bool,
    /// Whether to stream responses
    pub stream: bool,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            temperature: None,
            max_tokens: None,
            provider: None,
            model: None,
            token_budget: Some(8000),
            include_citations: true,
            save_drafts: true,
            stream: false,
        }
    }
}

/// Request for content generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    /// Type of content to generate
    pub generation_type: GenerationType,
    /// Campaign ID for context
    pub campaign_id: Option<String>,
    /// Wizard ID if part of wizard flow
    pub wizard_id: Option<String>,
    /// Template variables
    pub variables: HashMap<String, String>,
    /// Custom template name (for custom generation)
    pub custom_template: Option<String>,
    /// Additional context to include
    pub additional_context: Option<String>,
    /// Generation configuration
    pub config: GenerationConfig,
}

impl GenerationRequest {
    /// Create a new generation request
    pub fn new(generation_type: GenerationType) -> Self {
        Self {
            generation_type,
            campaign_id: None,
            wizard_id: None,
            variables: HashMap::new(),
            custom_template: None,
            additional_context: None,
            config: GenerationConfig::default(),
        }
    }

    /// Create a character background generation request
    pub fn character_background() -> Self {
        Self::new(GenerationType::CharacterBackground)
    }

    /// Create an NPC generation request
    pub fn npc() -> Self {
        Self::new(GenerationType::Npc)
    }

    /// Create a session plan generation request
    pub fn session_plan() -> Self {
        Self::new(GenerationType::SessionPlan)
    }

    /// Create a party analysis request
    pub fn party_analysis() -> Self {
        Self::new(GenerationType::PartyAnalysis)
    }

    /// Create an arc outline request
    pub fn arc_outline() -> Self {
        Self::new(GenerationType::ArcOutline)
    }

    /// Set the campaign ID
    pub fn with_campaign_id(mut self, campaign_id: impl Into<String>) -> Self {
        self.campaign_id = Some(campaign_id.into());
        self
    }

    /// Set the wizard ID
    pub fn with_wizard_id(mut self, wizard_id: impl Into<String>) -> Self {
        self.wizard_id = Some(wizard_id.into());
        self
    }

    /// Add a template variable
    pub fn with_variable(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }

    /// Add multiple template variables
    pub fn with_variables(mut self, vars: HashMap<String, String>) -> Self {
        self.variables.extend(vars);
        self
    }

    /// Set additional context
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.additional_context = Some(context.into());
        self
    }

    /// Set a custom template
    pub fn with_custom_template(mut self, name: impl Into<String>) -> Self {
        self.custom_template = Some(name.into());
        self
    }

    /// Configure to not save drafts
    pub fn without_saving(mut self) -> Self {
        self.config.save_drafts = false;
        self
    }

    /// Enable streaming
    pub fn with_streaming(mut self) -> Self {
        self.config.stream = true;
        self
    }

    /// Set preferred model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.config.model = Some(model.into());
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.config.temperature = Some(temp);
        self
    }
}

/// Response from a generation operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResponse {
    /// Unique ID for this generation
    pub id: String,
    /// Type of content generated
    pub generation_type: GenerationType,
    /// Raw LLM response content
    pub raw_content: String,
    /// Parsed structured content (if applicable)
    pub parsed_content: Option<serde_json::Value>,
    /// Trust assignment for the content
    pub trust: TrustAssignment,
    /// Citations found/used
    pub citations: Vec<Citation>,
    /// Draft ID if saved to database
    pub draft_id: Option<String>,
    /// Token usage
    pub usage: Option<TokenUsage>,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Model used
    pub model: String,
    /// Provider used
    pub provider: String,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

// ============================================================================
// Generation Orchestrator
// ============================================================================

/// Main orchestrator for content generation
pub struct GenerationOrchestrator {
    /// LLM router for making generation calls
    llm_router: Arc<RwLock<LLMRouter>>,
    /// Search client for grounding (reserved for future use)
    #[allow(dead_code)]
    search_client: Arc<SearchClient>,
    /// Template registry
    template_registry: Arc<RwLock<TemplateRegistry>>,
    /// Database for drafts and campaigns
    database: Database,
    /// Context assembler (reserved for future use)
    #[allow(dead_code)]
    context_assembler: ContextAssembler,
    /// Trust assigner
    trust_assigner: TrustAssigner,
    /// Rulebook linker for citations
    rulebook_linker: Option<Arc<RulebookLinker>>,
    /// Flavour searcher for lore
    flavour_searcher: Option<Arc<FlavourSearcher>>,
    /// Usage tracker for content tracking
    usage_tracker: Option<Arc<UsageTracker>>,
}

impl GenerationOrchestrator {
    /// Create a new orchestrator with required dependencies
    pub fn new(
        llm_router: Arc<RwLock<LLMRouter>>,
        search_client: Arc<SearchClient>,
        registry: TemplateRegistry,
        database: Database,
    ) -> Self {
        Self {
            llm_router,
            search_client: search_client.clone(),
            template_registry: Arc::new(RwLock::new(registry)),
            database: database.clone(),
            context_assembler: ContextAssembler::new(database.clone()),
            trust_assigner: TrustAssigner::new(),
            rulebook_linker: None,
            flavour_searcher: None,
            usage_tracker: None,
        }
    }

    /// Set the template registry
    pub fn with_template_registry(mut self, registry: Arc<RwLock<TemplateRegistry>>) -> Self {
        self.template_registry = registry;
        self
    }

    /// Set the rulebook linker for citations
    pub fn with_rulebook_linker(mut self, linker: Arc<RulebookLinker>) -> Self {
        self.rulebook_linker = Some(linker);
        self
    }

    /// Set the flavour searcher for lore
    pub fn with_flavour_searcher(mut self, searcher: Arc<FlavourSearcher>) -> Self {
        self.flavour_searcher = Some(searcher);
        self
    }

    /// Set the usage tracker
    pub fn with_usage_tracker(mut self, tracker: Arc<UsageTracker>) -> Self {
        self.usage_tracker = Some(tracker);
        self
    }

    /// Generate content based on the request
    pub async fn generate(
        &self,
        request: GenerationRequest,
    ) -> Result<GenerationResponse, GenerationError> {
        let start = std::time::Instant::now();
        let gen_id = uuid::Uuid::new_v4().to_string();

        // 1. Load the template
        let template = self.load_template(&request).await?;

        // 2. Load campaign context if campaign_id is provided
        let campaign_context = if let Some(ref campaign_id) = request.campaign_id {
            self.load_campaign_context(campaign_id).await?
        } else {
            None
        };

        // 3. Build context with assembled campaign data
        let mut variables = request.variables.clone();
        if let Some(ref ctx) = campaign_context {
            variables.extend(ctx.to_variables());
        }
        if let Some(ref additional) = request.additional_context {
            variables.insert("additional_context".to_string(), additional.clone());
        }

        // 4. Render prompts
        let system_prompt = template.render_system_prompt(&variables)?;
        let user_prompt = template.render_user_prompt(&variables)?;

        // 5. Build LLM request
        let messages = vec![
            ChatMessage::system(&system_prompt),
            ChatMessage::user(&user_prompt),
        ];

        let mut chat_request = ChatRequest::new(messages);
        if let Some(temp) = request.config.temperature.or(template.temperature) {
            chat_request = chat_request.with_temperature(temp);
        }
        if let Some(max) = request.config.max_tokens.or(template.max_tokens) {
            chat_request = chat_request.with_max_tokens(max);
        }
        if let Some(ref provider) = request.config.provider.clone().or_else(|| template.metadata.recommended_model.clone()) {
            chat_request = chat_request.with_provider(provider);
        }

        // 6. Call LLM
        let response = self.call_llm(chat_request).await?;

        // 7. Parse response
        let parsed_content = self.parse_response(&response.content, &request.generation_type);

        // 8. Find and assign citations
        let citations = if request.config.include_citations {
            self.find_citations(&response.content, request.campaign_id.as_deref())
                .await
        } else {
            Vec::new()
        };

        // 9. Assign trust level
        let trust = self.trust_assigner.assign(
            &response.content,
            &citations,
            parsed_content.as_ref(),
        );

        // 10. Save draft if configured
        let draft_id = if request.config.save_drafts {
            self.save_draft(
                &gen_id,
                &request,
                &response.content,
                &parsed_content,
                &trust,
                &citations,
            )
            .await?
        } else {
            None
        };

        let latency_ms = start.elapsed().as_millis() as u64;

        Ok(GenerationResponse {
            id: gen_id,
            generation_type: request.generation_type,
            raw_content: response.content,
            parsed_content,
            trust,
            citations,
            draft_id,
            usage: response.usage.map(|u| TokenUsage {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
                total_tokens: u.input_tokens + u.output_tokens,
            }),
            latency_ms,
            model: response.model,
            provider: response.provider,
        })
    }

    /// Load the appropriate template for a request
    async fn load_template(
        &self,
        request: &GenerationRequest,
    ) -> Result<GenerationTemplate, GenerationError> {
        let registry = self.template_registry.read().await;

        if let Some(ref custom_name) = request.custom_template {
            registry
                .get_custom(custom_name)
                .await
                .ok_or_else(|| GenerationError::Template(TemplateError::NotFound(custom_name.clone())))
        } else {
            Ok(registry
                .get_or_default(request.generation_type.template_type())
                .await)
        }
    }

    /// Load campaign context for a campaign ID
    async fn load_campaign_context(
        &self,
        campaign_id: &str,
    ) -> Result<Option<CampaignContext>, GenerationError> {
        // Load campaign from database
        let campaign = self
            .database
            .get_campaign(campaign_id)
            .await
            .map_err(|e| GenerationError::Database(e.to_string()))?;

        if campaign.is_none() {
            return Err(GenerationError::CampaignNotFound(campaign_id.to_string()));
        }

        let campaign = campaign.unwrap();

        // TODO: Load campaign intent when database method is available
        // For now, intent is always None
        let intent = None;

        Ok(Some(CampaignContext {
            id: campaign.id,
            name: campaign.name,
            system: campaign.system,
            description: campaign.description,
            setting: campaign.setting,
            intent,
        }))
    }

    /// Call the LLM with the request
    async fn call_llm(
        &self,
        request: ChatRequest,
    ) -> Result<ChatResponse, GenerationError> {
        let router = self.llm_router.read().await;
        router
            .chat(request)
            .await
            .map_err(|e| GenerationError::Llm(e.to_string()))
    }

    /// Parse the LLM response into structured content
    fn parse_response(
        &self,
        content: &str,
        gen_type: &GenerationType,
    ) -> Option<serde_json::Value> {
        // First priority: Try to extract from markdown code blocks
        if let Some(start) = content.find("```json") {
            if let Some(end) = content[start + 7..].find("```") {
                let json_str = content[start + 7..start + 7 + end].trim();
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    return Some(value);
                }
            }
        }

        // Second priority: Try each '{' occurrence to find valid JSON
        // This handles cases with multiple objects or prose before JSON
        for (idx, _) in content.match_indices('{') {
            // Find matching closing brace by counting brace depth
            let substring = &content[idx..];
            let mut depth = 0;
            let mut end_idx = None;
            let mut in_string = false;
            let mut escaped = false;

            for (i, ch) in substring.char_indices() {
                if escaped {
                    escaped = false;
                    continue;
                }

                match ch {
                    '\\' if in_string => escaped = true,
                    '"' => in_string = !in_string,
                    '{' if !in_string => depth += 1,
                    '}' if !in_string => {
                        depth -= 1;
                        if depth == 0 {
                            end_idx = Some(i);
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if let Some(end) = end_idx {
                let json_str = &substring[..=end];
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    return Some(value);
                }
            }
        }

        // For some types, wrap the content in a simple structure
        match gen_type {
            GenerationType::Custom => Some(serde_json::json!({
                "content": content
            })),
            _ => None,
        }
    }

    /// Find citations in the generated content
    async fn find_citations(
        &self,
        content: &str,
        _campaign_id: Option<&str>,
    ) -> Vec<Citation> {
        let mut citations = Vec::new();

        // Use rulebook linker if available
        if let Some(ref linker) = self.rulebook_linker {
            let references = linker.find_references(content);
            for reference in references {
                // Link without rulebook filter for now - could add campaign-specific filtering later
                if let Ok(linked) = linker.link_to_rulebook(&reference.raw_text, None).await {
                    if let Some(best) = linked.first() {
                        let citation = linker.build_citation(&reference, Some(best));
                        citations.push(citation);
                    }
                }
            }
        }

        citations
    }

    /// Save a draft to the database
    ///
    /// Note: Database persistence is deferred to a future implementation phase.
    /// Returns None to indicate no draft was actually persisted.
    ///
    /// # Returns
    /// - `Ok(Some(draft_id))` - Draft was persisted successfully
    /// - `Ok(None)` - Persistence not yet implemented
    /// - `Err(_)` - Error during persistence attempt
    async fn save_draft(
        &self,
        _gen_id: &str,
        _request: &GenerationRequest,
        _raw_content: &str,
        _parsed_content: &Option<serde_json::Value>,
        _trust: &TrustAssignment,
        _citations: &[Citation],
    ) -> Result<Option<String>, GenerationError> {
        // TODO: Implement actual database persistence when generation_drafts table
        // and related database methods are added.
        //
        // Future implementation would:
        // 1. Create GenerationDraftRecord with trust and citations
        // 2. Insert into generation_drafts table
        // 3. Insert citation records into source_citations table
        // 4. Return Ok(Some(draft_id))

        tracing::debug!("Draft persistence deferred - no draft ID generated");

        Ok(None)
    }
}

// ============================================================================
// Campaign Context
// ============================================================================

/// Loaded campaign context for generation
#[derive(Debug, Clone)]
struct CampaignContext {
    id: String,
    name: String,
    system: String,
    description: Option<String>,
    setting: Option<String>,
    intent: Option<CampaignIntent>,
}

impl CampaignContext {
    /// Convert to template variables
    fn to_variables(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();

        vars.insert("campaign_id".to_string(), self.id.clone());
        vars.insert("campaign_name".to_string(), self.name.clone());
        vars.insert("game_system".to_string(), self.system.clone());

        if let Some(ref desc) = self.description {
            vars.insert("campaign_description".to_string(), desc.clone());
        }

        if let Some(ref setting) = self.setting {
            vars.insert("campaign_setting".to_string(), setting.clone());
        }

        if let Some(ref intent) = self.intent {
            vars.insert("fantasy".to_string(), intent.fantasy.clone());
            vars.insert("themes".to_string(), intent.themes.join(", "));
            vars.insert("tone".to_string(), intent.tone_keywords.join(", "));
            vars.insert(
                "player_experiences".to_string(),
                intent.player_experiences.join(", "),
            );
            vars.insert("constraints".to_string(), intent.constraints.join(", "));
            vars.insert("avoid".to_string(), intent.avoid.join(", "));
        }

        // Build campaign_context summary
        let mut context_parts = vec![format!("Campaign: {}", self.name)];
        if let Some(ref setting) = self.setting {
            context_parts.push(format!("Setting: {}", setting));
        }
        if let Some(ref desc) = self.description {
            context_parts.push(desc.clone());
        }
        vars.insert("campaign_context".to_string(), context_parts.join("\n"));

        vars
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generation_type_template_type() {
        assert_eq!(
            GenerationType::CharacterBackground.template_type(),
            TemplateType::CharacterBackground
        );
        assert_eq!(
            GenerationType::Npc.template_type(),
            TemplateType::NpcGeneration
        );
        assert_eq!(
            GenerationType::SessionPlan.template_type(),
            TemplateType::SessionPlan
        );
    }

    #[test]
    fn test_generation_request_builder() {
        let request = GenerationRequest::npc()
            .with_campaign_id("camp-123")
            .with_variable("npc_role", "merchant")
            .with_variable("importance", "major")
            .with_context("A bustling marketplace")
            .with_temperature(0.8);

        assert_eq!(request.generation_type, GenerationType::Npc);
        assert_eq!(request.campaign_id, Some("camp-123".to_string()));
        assert_eq!(request.variables.get("npc_role"), Some(&"merchant".to_string()));
        assert_eq!(request.config.temperature, Some(0.8));
    }

    #[test]
    fn test_generation_config_defaults() {
        let config = GenerationConfig::default();
        assert!(config.include_citations);
        assert!(config.save_drafts);
        assert!(!config.stream);
        assert_eq!(config.token_budget, Some(8000));
    }

    #[test]
    fn test_campaign_context_to_variables() {
        let context = CampaignContext {
            id: "camp-1".to_string(),
            name: "Test Campaign".to_string(),
            system: "dnd5e".to_string(),
            description: Some("A dark fantasy adventure".to_string()),
            setting: Some("Forgotten Realms".to_string()),
            intent: Some(CampaignIntent {
                fantasy: "dark fantasy".to_string(),
                themes: vec!["corruption".to_string(), "redemption".to_string()],
                tone_keywords: vec!["gritty".to_string(), "hopeful".to_string()],
                player_experiences: vec!["mystery".to_string()],
                constraints: vec!["no gore".to_string()],
                avoid: vec!["romance".to_string()],
            }),
        };

        let vars = context.to_variables();
        assert_eq!(vars.get("campaign_id"), Some(&"camp-1".to_string()));
        assert_eq!(vars.get("fantasy"), Some(&"dark fantasy".to_string()));
        assert!(vars.get("themes").unwrap().contains("corruption"));
    }

    #[test]
    fn test_parse_json_from_content() {
        // Test direct JSON
        let content = r#"Here is the NPC: {"name": "Bob", "role": "merchant"}"#;
        let json_start = content.find('{').unwrap();
        let json_end = content.rfind('}').unwrap();
        let json_str = &content[json_start..=json_end];
        let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(parsed["name"], "Bob");
    }
}
