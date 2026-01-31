//! Campaign Creation Wizard Components
//!
//! Multi-step wizard for creating new campaigns with AI assistance.
//! Implements a progressive disclosure pattern for campaign configuration.
//!
//! # Components
//! - `WizardShell` - Main container with navigation and step progress
//! - `StepProgress` - Visual step indicator rail
//! - Step components for each wizard phase
//! - `ConversationPanel` - AI chat assistant panel
//! - `DraftRecoveryModal` - Recovery UI for incomplete drafts
//! - `AutoSaveIndicator` - Visual feedback for auto-save status
//! - `AiUnavailableBanner` - Offline/unavailable AI indicator
//! - Tooltips and help components for TTRPG terminology
//! - Error display components for consistent error handling

pub mod wizard_shell;
pub mod step_progress;
pub mod steps;
pub mod conversation_panel;
pub mod draft_recovery;
pub mod auto_save;
pub mod offline_indicator;
pub mod help_tooltip;
pub mod error_display;
pub mod interview_mode;

// Re-exports - Core components
pub use wizard_shell::WizardShell;
pub use step_progress::{StepProgress, StepProgressVertical};
pub use conversation_panel::ConversationPanel;
pub use steps::*;

// Re-exports - Integration components (Phase 7)
pub use draft_recovery::{DraftRecoveryModal, DraftBadge, use_draft_recovery};
pub use auto_save::{AutoSaveIndicator, AutoSaveStatus, AutoSaveState, use_auto_save};
pub use offline_indicator::{
    AiUnavailableBanner, AiStatusDot, AiFeatureDisabledNotice,
    LlmAvailability, use_llm_availability, OfflineQueue, QueuedRequestsIndicator,
};
pub use help_tooltip::{
    Tooltip, RichTooltip, HelpIcon, HelpExpander, ContextualHelp, LabelWithHelp,
    TooltipPosition, terminology,
};
pub use error_display::{
    ErrorBanner, InlineError, LoadingOverlay, InlineLoading, LoadingButton,
    Skeleton, SkeletonCard, FeatureUnavailable, EmptyState,
    ErrorSeverity, ButtonLoadingVariant,
};
pub use interview_mode::{
    InterviewMode, InterviewQuestion, InterviewAnswer, InterviewState,
    SuggestionChip, FieldType, QuestionCategory, InspirationPrompt,
    get_default_questions, get_inspiration,
};
