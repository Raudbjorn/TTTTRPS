//! Generation Orchestration Layer
//!
//! Phase 4 of the Campaign Generation Overhaul.
//!
//! This module provides the generation pipeline for creating campaign content
//! with LLM assistance, including characters, NPCs, sessions, arcs, and party
//! composition suggestions.
//!
//! ## Components
//!
//! - [`TemplateRegistry`] - YAML template loading and caching
//! - [`GenerationOrchestrator`] - Core orchestrator coordinating LLM and search
//! - [`ContextAssembler`] - Token-budget-aware context construction
//! - [`TrustAssigner`] - Citation-based trust level assignment
//! - [`AcceptanceManager`] - Draft lifecycle management
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌──────────────────┐     ┌────────────────┐
//! │ Tauri Commands  │────▶│ Generation       │────▶│ LLM Router     │
//! │ (API Layer)     │     │ Orchestrator     │     │ (Multi-provider│
//! └─────────────────┘     └──────────────────┘     └────────────────┘
//!                                 │
//!                                 ▼
//!         ┌─────────────────────────────────────────┐
//!         │              Context Assembly           │
//!         │  ┌──────────┐ ┌──────────┐ ┌─────────┐  │
//!         │  │ Campaign │ │ Grounding│ │Templates│  │
//!         │  │ Snapshot │ │ Layer    │ │ Registry│  │
//!         │  └──────────┘ └──────────┘ └─────────┘  │
//!         └─────────────────────────────────────────┘
//!                                 │
//!                                 ▼
//!         ┌─────────────────────────────────────────┐
//!         │           Post-Processing               │
//!         │  ┌──────────┐ ┌──────────┐ ┌─────────┐  │
//!         │  │ Trust    │ │Acceptance│ │ Draft   │  │
//!         │  │ Assigner │ │ Manager  │ │ Storage │  │
//!         │  └──────────┘ └──────────┘ └─────────┘  │
//!         └─────────────────────────────────────────┘
//! ```
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use crate::core::campaign::generation::{
//!     GenerationOrchestrator, TemplateRegistry, GenerationRequest,
//! };
//!
//! // Create orchestrator with dependencies
//! let registry = TemplateRegistry::load_from_dir("resources/templates/generation")?;
//! let orchestrator = GenerationOrchestrator::new(
//!     llm_router,
//!     search_client,
//!     registry,
//!     database,
//! );
//!
//! // Generate an NPC
//! let request = GenerationRequest::npc()
//!     .with_campaign_id("campaign-123")
//!     .with_context("A mysterious merchant in the city of Waterdeep")
//!     .with_importance(NpcImportance::Major);
//!
//! let draft = orchestrator.generate(request).await?;
//! ```

mod templates;
mod orchestrator;
mod context;
mod trust;
mod acceptance;
mod character_gen;
mod npc_gen;
mod session_gen;
mod party_gen;
mod arc_gen;

pub use templates::{
    GenerationTemplate, TemplateRegistry, TemplateError, TemplateVariable,
    TemplateType, TemplateMetadata,
};
pub use orchestrator::{
    GenerationOrchestrator, GenerationRequest, GenerationResponse,
    GenerationType, GenerationError, GenerationConfig,
};
pub use context::{
    ContextAssembler, AssembledContext, TokenBudget, ContextError,
    ContextSection, ContextPriority,
};
pub use trust::{
    TrustAssigner, TrustAssignment, ClaimAnalysis, TrustError,
};
pub use acceptance::{
    AcceptanceManager, AcceptanceError, DraftAction, AppliedEntity, InMemoryDraft,
};
pub use character_gen::{
    CharacterGenerator, CharacterGenerationRequest, CharacterDraft,
    ExtractedEntity,
};
pub use npc_gen::{
    NpcGenerator, NpcGenerationRequest, NpcDraft, NpcImportance,
};
pub use session_gen::{
    SessionGenerator, SessionGenerationRequest, SessionPlanDraft,
    PacingTemplate, EncounterDifficulty,
};
pub use party_gen::{
    PartyAnalyzer, PartyAnalysisRequest, PartySuggestion, GapAnalysis,
};
pub use arc_gen::{
    ArcGenerator, ArcGenerationRequest, ArcDraft, TensionCurve,
    ArcTemplateType,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify all public types are accessible
        let _: TemplateType = TemplateType::CharacterBackground;
        let _: GenerationType = GenerationType::Npc;
        let _: ContextPriority = ContextPriority::High;
    }
}
