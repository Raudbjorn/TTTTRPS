//! Campaign Intelligence Pipeline (CIP) Core Types
//!
//! This module provides the foundational types for the Campaign Intelligence Pipeline,
//! which is the architectural spine for all campaign generation features.
//!
//! # Pipeline Overview
//!
//! ```text
//! Input → Context Assembly → Generation Engine → Normalization → Acceptance → Artifacts
//! ```
//!
//! # Key Types
//!
//! - [`TrustLevel`] - Indicates reliability of generated content (Canonical, Derived, Creative, Unverified)
//! - [`CanonStatus`] - Lifecycle status for progressive commitment (Draft → Approved → Canonical → Deprecated)
//! - [`CampaignIntent`] - Stable anchor for tone and creative vision
//! - [`EntityDraft<T>`] - Generic wrapper for draft entities with status and trust tracking
//!
//! # Design Principles
//!
//! 1. **Single Intelligence Loop** - All generation flows through one pipeline
//! 2. **CampaignManager Owns Truth** - All canonical state lives in one place
//! 3. **Progressive Commitment** - Content moves through stages before becoming "real"
//! 4. **Trust is Explicit** - Generated content carries trust levels
//! 5. **Intent Anchors Generation** - CampaignIntent prevents tone drift

// Re-export core types from database models for convenience
pub use crate::database::{
    // Trust and Status
    TrustLevel,
    CanonStatus,

    // Campaign Intent
    CampaignIntent,
    CampaignIntentRecord,

    // Entity Drafts
    EntityDraft,
    GenerationDraftRecord,

    // Status Tracking
    CanonStatusLogRecord,
    AcceptanceEventRecord,
    AcceptanceDecision,

    // Citations
    Citation,
    SourceCitationRecord,
    SourceType,
    SourceLocation,

    // Wizard State
    WizardStep,
    WizardStateRecord,

    // Conversation
    ConversationPurpose,
    ConversationRole,
    ConversationThreadRecord,
    ConversationMessageRecord,
    Suggestion,
    SuggestionStatus,

    // Party
    PartyCompositionRecord,
};

/// Trust thresholds for content classification
#[derive(Debug, Clone, Copy)]
pub struct TrustThresholds {
    /// Minimum confidence for Canonical classification (default: 0.95)
    pub canonical_confidence: f64,
    /// Minimum confidence for Derived classification (default: 0.75)
    pub derived_confidence: f64,
    /// Creative is the default for anything below derived threshold
    pub creative_confidence: f64,
}

impl Default for TrustThresholds {
    fn default() -> Self {
        Self {
            canonical_confidence: 0.95,
            derived_confidence: 0.75,
            creative_confidence: 0.0,
        }
    }
}

impl TrustThresholds {
    /// Determine trust level from confidence score
    pub fn classify(&self, confidence: f64, has_verified_source: bool) -> TrustLevel {
        if has_verified_source && confidence >= self.canonical_confidence {
            TrustLevel::Canonical
        } else if confidence >= self.derived_confidence {
            TrustLevel::Derived
        } else if confidence > 0.0 {
            TrustLevel::Unverified
        } else {
            TrustLevel::Creative
        }
    }
}

/// Result type for pipeline operations
pub type PipelineResult<T> = Result<T, PipelineError>;

/// Errors that can occur in pipeline operations
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid state transition: cannot transition from {from} to {to}")]
    InvalidTransition { from: String, to: String },

    #[error("Entity not found: {entity_type} with id {id}")]
    NotFound { entity_type: String, id: String },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Draft is locked and cannot be modified")]
    DraftLocked,

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<String> for PipelineError {
    fn from(s: String) -> Self {
        PipelineError::Internal(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_thresholds_classify() {
        let thresholds = TrustThresholds::default();

        // High confidence with verified source = Canonical
        assert_eq!(
            thresholds.classify(0.98, true),
            TrustLevel::Canonical
        );

        // High confidence without verified source = Derived
        assert_eq!(
            thresholds.classify(0.98, false),
            TrustLevel::Derived
        );

        // Medium confidence = Derived
        assert_eq!(
            thresholds.classify(0.80, false),
            TrustLevel::Derived
        );

        // Low confidence = Unverified
        assert_eq!(
            thresholds.classify(0.50, false),
            TrustLevel::Unverified
        );

        // Zero confidence = Creative
        assert_eq!(
            thresholds.classify(0.0, false),
            TrustLevel::Creative
        );
    }

    #[test]
    fn test_pipeline_error_display() {
        let err = PipelineError::InvalidTransition {
            from: "draft".to_string(),
            to: "canonical".to_string(),
        };
        assert!(err.to_string().contains("draft"));
        assert!(err.to_string().contains("canonical"));

        let err = PipelineError::NotFound {
            entity_type: "npc".to_string(),
            id: "123".to_string(),
        };
        assert!(err.to_string().contains("npc"));
        assert!(err.to_string().contains("123"));
    }
}
