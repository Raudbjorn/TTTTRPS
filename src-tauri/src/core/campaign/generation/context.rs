//! Context Assembler - Token-budget-aware context construction
//!
//! Phase 4, Task 4.8: Implement ContextAssembler
//!
//! Assembles full context from campaign snapshot, CampaignIntent, grounded
//! rules/lore, and conversation window while respecting token budgets.

use crate::core::campaign::pipeline::CampaignIntent;
use crate::database::{CampaignOps, CampaignRecord, Database};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during context assembly
#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Token budget exceeded: {used} > {budget}")]
    BudgetExceeded { used: u32, budget: u32 },

    #[error("Required context missing: {0}")]
    MissingContext(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

// ============================================================================
// Types
// ============================================================================

/// Priority level for context sections
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextPriority {
    /// Must always be included
    Critical,
    /// Should be included if space allows
    High,
    /// Nice to have
    Medium,
    /// Only if plenty of budget remaining
    Low,
}

impl Default for ContextPriority {
    fn default() -> Self {
        ContextPriority::Medium
    }
}

/// A section of assembled context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSection {
    /// Section identifier
    pub id: String,
    /// Section name for display
    pub name: String,
    /// Section content
    pub content: String,
    /// Priority for budget allocation
    pub priority: ContextPriority,
    /// Estimated token count
    pub estimated_tokens: u32,
    /// Source of this context (campaign, grounding, user, etc.)
    pub source: String,
}

impl ContextSection {
    /// Create a new context section
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        let content = content.into();
        let estimated_tokens = estimate_tokens(&content);

        Self {
            id: id.into(),
            name: name.into(),
            content,
            priority: ContextPriority::Medium,
            estimated_tokens,
            source: "unknown".to_string(),
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: ContextPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set source
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }
}

/// Token budget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    /// Total available tokens
    pub total: u32,
    /// Reserved for system prompt
    pub system_reserve: u32,
    /// Reserved for user prompt
    pub user_reserve: u32,
    /// Reserved for output
    pub output_reserve: u32,
    /// Minimum tokens per section
    pub min_section_tokens: u32,
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            total: 8000,
            system_reserve: 500,
            user_reserve: 1000,
            output_reserve: 2000,
            min_section_tokens: 50,
        }
    }
}

impl TokenBudget {
    /// Create a new token budget with specified total
    pub fn new(total: u32) -> Self {
        Self {
            total,
            ..Default::default()
        }
    }

    /// Get available tokens for context after reserves
    pub fn available_for_context(&self) -> u32 {
        self.total
            .saturating_sub(self.system_reserve)
            .saturating_sub(self.user_reserve)
            .saturating_sub(self.output_reserve)
    }

    /// Create budget for a smaller context window
    pub fn compact() -> Self {
        Self {
            total: 4000,
            system_reserve: 300,
            user_reserve: 500,
            output_reserve: 1000,
            min_section_tokens: 30,
        }
    }

    /// Create budget for a larger context window
    pub fn large() -> Self {
        Self {
            total: 16000,
            system_reserve: 800,
            user_reserve: 2000,
            output_reserve: 4000,
            min_section_tokens: 100,
        }
    }
}

/// Assembled context ready for prompt construction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledContext {
    /// Campaign context summary
    pub campaign_summary: Option<String>,
    /// Campaign intent data
    pub intent: Option<CampaignIntent>,
    /// Grounded rules/mechanics
    pub grounded_rules: Vec<ContextSection>,
    /// Grounded lore/setting
    pub grounded_lore: Vec<ContextSection>,
    /// Conversation history
    pub conversation_window: Vec<ContextSection>,
    /// Additional custom sections
    pub custom_sections: Vec<ContextSection>,
    /// Total estimated tokens used
    pub total_tokens: u32,
    /// Token budget used
    pub budget: TokenBudget,
    /// Sections that were trimmed due to budget
    pub trimmed_sections: Vec<String>,
}

impl AssembledContext {
    /// Create empty assembled context
    pub fn empty(budget: TokenBudget) -> Self {
        Self {
            campaign_summary: None,
            intent: None,
            grounded_rules: Vec::new(),
            grounded_lore: Vec::new(),
            conversation_window: Vec::new(),
            custom_sections: Vec::new(),
            total_tokens: 0,
            budget,
            trimmed_sections: Vec::new(),
        }
    }

    /// Convert to template variables
    pub fn to_variables(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();

        if let Some(ref summary) = self.campaign_summary {
            vars.insert("campaign_context".to_string(), summary.clone());
        }

        if let Some(ref intent) = self.intent {
            vars.insert("fantasy".to_string(), intent.fantasy.clone());
            vars.insert("themes".to_string(), intent.themes.join(", "));
            vars.insert("tone".to_string(), intent.tone_keywords.join(", "));
            vars.insert("player_experiences".to_string(), intent.player_experiences.join(", "));
            vars.insert("constraints".to_string(), intent.constraints.join(", "));
            vars.insert("avoid".to_string(), intent.avoid.join(", "));
        }

        // Combine grounded content
        if !self.grounded_rules.is_empty() {
            let rules: Vec<String> = self.grounded_rules.iter().map(|s| s.content.clone()).collect();
            vars.insert("grounded_rules".to_string(), rules.join("\n\n"));
        }

        if !self.grounded_lore.is_empty() {
            let lore: Vec<String> = self.grounded_lore.iter().map(|s| s.content.clone()).collect();
            vars.insert("grounded_lore".to_string(), lore.join("\n\n"));
        }

        vars
    }

    /// Get all sections as a flat list
    pub fn all_sections(&self) -> Vec<&ContextSection> {
        let mut sections = Vec::new();
        sections.extend(self.grounded_rules.iter());
        sections.extend(self.grounded_lore.iter());
        sections.extend(self.conversation_window.iter());
        sections.extend(self.custom_sections.iter());
        sections
    }
}

// ============================================================================
// Context Assembler
// ============================================================================

/// Assembles context from various sources within token budget
pub struct ContextAssembler {
    database: Database,
}

impl ContextAssembler {
    /// Create a new context assembler
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    /// Assemble context for a campaign
    pub async fn assemble(
        &self,
        campaign_id: &str,
        budget: TokenBudget,
        options: AssemblyOptions,
    ) -> Result<AssembledContext, ContextError> {
        let mut context = AssembledContext::empty(budget.clone());
        let available_tokens = budget.available_for_context();
        let mut used_tokens = 0u32;

        // 1. Load campaign summary (Critical priority)
        if let Some(campaign) = self.load_campaign(campaign_id).await? {
            let summary = self.build_campaign_summary(&campaign);
            let tokens = estimate_tokens(&summary);
            if used_tokens + tokens <= available_tokens {
                context.campaign_summary = Some(summary);
                used_tokens += tokens;
            }
        }

        // 2. Load campaign intent (Critical priority)
        if let Some(intent) = self.load_intent(campaign_id).await? {
            let intent_text = format!(
                "Fantasy: {}\nThemes: {}\nTone: {}",
                intent.fantasy,
                intent.themes.join(", "),
                intent.tone_keywords.join(", ")
            );
            let tokens = estimate_tokens(&intent_text);
            if used_tokens + tokens <= available_tokens {
                context.intent = Some(intent);
                used_tokens += tokens;
            }
        }

        // 3. Add grounded rules if requested (High priority)
        if options.include_rules {
            let rules_budget = (available_tokens - used_tokens) / 3;
            let rules = self.load_grounded_rules(campaign_id, rules_budget).await?;
            for section in rules {
                if used_tokens + section.estimated_tokens <= available_tokens {
                    used_tokens += section.estimated_tokens;
                    context.grounded_rules.push(section);
                } else {
                    context.trimmed_sections.push(section.id);
                }
            }
        }

        // 4. Add grounded lore if requested (Medium priority)
        if options.include_lore {
            let lore_budget = (available_tokens - used_tokens) / 2;
            let lore = self.load_grounded_lore(campaign_id, lore_budget).await?;
            for section in lore {
                if used_tokens + section.estimated_tokens <= available_tokens {
                    used_tokens += section.estimated_tokens;
                    context.grounded_lore.push(section);
                } else {
                    context.trimmed_sections.push(section.id);
                }
            }
        }

        // 5. Add conversation window if provided (High priority)
        if let Some(thread_id) = options.conversation_thread_id {
            let conv_budget = (available_tokens - used_tokens).min(options.max_conversation_tokens);
            let messages = self.load_conversation_window(&thread_id, conv_budget).await?;
            for section in messages {
                if used_tokens + section.estimated_tokens <= available_tokens {
                    used_tokens += section.estimated_tokens;
                    context.conversation_window.push(section);
                }
            }
        }

        // 6. Add custom sections by priority
        let mut custom_sections = options.custom_sections;
        custom_sections.sort_by(|a, b| a.priority.cmp(&b.priority));

        for section in custom_sections {
            if used_tokens + section.estimated_tokens <= available_tokens {
                used_tokens += section.estimated_tokens;
                context.custom_sections.push(section);
            } else if section.priority <= ContextPriority::High {
                // Try to truncate high priority sections
                let remaining = available_tokens.saturating_sub(used_tokens);
                if remaining >= budget.min_section_tokens {
                    let truncated = self.truncate_section(&section, remaining);
                    used_tokens += truncated.estimated_tokens;
                    context.custom_sections.push(truncated);
                } else {
                    context.trimmed_sections.push(section.id);
                }
            } else {
                context.trimmed_sections.push(section.id);
            }
        }

        context.total_tokens = used_tokens;
        Ok(context)
    }

    /// Load campaign record
    async fn load_campaign(&self, campaign_id: &str) -> Result<Option<CampaignRecord>, ContextError> {
        self.database
            .get_campaign(campaign_id)
            .await
            .map_err(|e| ContextError::Database(e.to_string()))
    }

    /// Load campaign intent
    ///
    /// Note: Database method not yet implemented - returns None for now.
    async fn load_intent(&self, _campaign_id: &str) -> Result<Option<CampaignIntent>, ContextError> {
        // TODO: Implement when get_campaign_intent database method is available
        Ok(None)
    }

    /// Build a campaign summary from the record
    fn build_campaign_summary(&self, campaign: &CampaignRecord) -> String {
        let mut parts = vec![format!("Campaign: {}", campaign.name)];
        parts.push(format!("System: {}", campaign.system));

        if let Some(ref setting) = campaign.setting {
            parts.push(format!("Setting: {}", setting));
        }

        if let Some(ref desc) = campaign.description {
            parts.push(desc.clone());
        }

        if let Some(ref date) = campaign.current_in_game_date {
            parts.push(format!("Current Date: {}", date));
        }

        parts.join("\n")
    }

    /// Load grounded rules within budget
    async fn load_grounded_rules(
        &self,
        _campaign_id: &str,
        _budget: u32,
    ) -> Result<Vec<ContextSection>, ContextError> {
        // TODO: Integrate with RulebookLinker to fetch relevant rules
        // For now, return empty
        Ok(Vec::new())
    }

    /// Load grounded lore within budget
    async fn load_grounded_lore(
        &self,
        _campaign_id: &str,
        _budget: u32,
    ) -> Result<Vec<ContextSection>, ContextError> {
        // TODO: Integrate with FlavourSearcher to fetch relevant lore
        // For now, return empty
        Ok(Vec::new())
    }

    /// Load conversation window within budget
    ///
    /// Note: Database method not yet implemented - returns empty for now.
    async fn load_conversation_window(
        &self,
        _thread_id: &str,
        _budget: u32,
    ) -> Result<Vec<ContextSection>, ContextError> {
        // TODO: Implement when get_conversation_messages database method is available
        // Would load recent conversation messages within token budget
        Ok(Vec::new())
    }

    /// Truncate a section to fit within token budget
    fn truncate_section(&self, section: &ContextSection, max_tokens: u32) -> ContextSection {
        let target_chars = (max_tokens as usize) * 4; // Rough estimate: 4 chars per token
        let truncated_content = if section.content.chars().count() > target_chars {
            // Find safe byte boundary using char_indices
            let safe_idx = section.content
                .char_indices()
                .nth(target_chars)
                .map(|(i, _)| i)
                .unwrap_or(section.content.len());
            let mut content = section.content[..safe_idx].to_string();
            content.push_str("...[truncated]");
            content
        } else {
            section.content.clone()
        };

        ContextSection {
            id: section.id.clone(),
            name: section.name.clone(),
            content: truncated_content.clone(),
            priority: section.priority,
            estimated_tokens: estimate_tokens(&truncated_content),
            source: section.source.clone(),
        }
    }
}

/// Options for context assembly
#[derive(Debug, Clone, Default)]
pub struct AssemblyOptions {
    /// Include grounded rules from rulebooks
    pub include_rules: bool,
    /// Include grounded lore from setting materials
    pub include_lore: bool,
    /// Conversation thread ID for history
    pub conversation_thread_id: Option<String>,
    /// Maximum tokens for conversation history
    pub max_conversation_tokens: u32,
    /// Custom context sections to include
    pub custom_sections: Vec<ContextSection>,
}

impl AssemblyOptions {
    /// Create default options
    pub fn new() -> Self {
        Self {
            include_rules: true,
            include_lore: true,
            conversation_thread_id: None,
            max_conversation_tokens: 1000,
            custom_sections: Vec::new(),
        }
    }

    /// Include conversation history
    pub fn with_conversation(mut self, thread_id: impl Into<String>) -> Self {
        self.conversation_thread_id = Some(thread_id.into());
        self
    }

    /// Add a custom section
    pub fn with_section(mut self, section: ContextSection) -> Self {
        self.custom_sections.push(section);
        self
    }

    /// Disable rules grounding
    pub fn without_rules(mut self) -> Self {
        self.include_rules = false;
        self
    }

    /// Disable lore grounding
    pub fn without_lore(mut self) -> Self {
        self.include_lore = false;
        self
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Estimate token count from text (rough approximation)
pub fn estimate_tokens(text: &str) -> u32 {
    // Rough estimate: ~4 characters per token for English text
    // This is a conservative estimate; actual tokenization varies by model
    let chars = text.chars().count();
    ((chars as f32) / 4.0).ceil() as u32
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_estimation() {
        // 100 characters should be roughly 25 tokens
        let text = "a".repeat(100);
        let tokens = estimate_tokens(&text);
        assert_eq!(tokens, 25);

        // Empty string
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_context_section_creation() {
        let section = ContextSection::new("test", "Test Section", "This is some content")
            .with_priority(ContextPriority::High)
            .with_source("test");

        assert_eq!(section.id, "test");
        assert_eq!(section.priority, ContextPriority::High);
        assert!(section.estimated_tokens > 0);
    }

    #[test]
    fn test_token_budget_available() {
        let budget = TokenBudget::default();
        let available = budget.available_for_context();
        assert!(available > 0);
        assert!(available < budget.total);
    }

    #[test]
    fn test_context_priority_ordering() {
        assert!(ContextPriority::Critical < ContextPriority::High);
        assert!(ContextPriority::High < ContextPriority::Medium);
        assert!(ContextPriority::Medium < ContextPriority::Low);
    }

    #[test]
    fn test_assembled_context_to_variables() {
        let mut context = AssembledContext::empty(TokenBudget::default());
        context.campaign_summary = Some("Test Campaign Summary".to_string());
        context.intent = Some(CampaignIntent {
            fantasy: "dark fantasy".to_string(),
            themes: vec!["corruption".to_string()],
            tone_keywords: vec!["gritty".to_string()],
            player_experiences: vec!["mystery".to_string()],
            constraints: vec![],
            avoid: vec![],
        });

        let vars = context.to_variables();
        assert_eq!(vars.get("fantasy"), Some(&"dark fantasy".to_string()));
        assert!(vars.contains_key("campaign_context"));
    }

    #[test]
    fn test_assembly_options_builder() {
        let section = ContextSection::new("custom", "Custom", "Content");
        let options = AssemblyOptions::new()
            .with_conversation("thread-123")
            .with_section(section)
            .without_rules();

        assert_eq!(options.conversation_thread_id, Some("thread-123".to_string()));
        assert!(!options.include_rules);
        assert!(options.include_lore);
        assert_eq!(options.custom_sections.len(), 1);
    }
}
