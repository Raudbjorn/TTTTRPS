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

pub mod auto_save;
pub mod conversation_panel;
pub mod draft_recovery;
pub mod error_display;
pub mod help_tooltip;
pub mod interview_mode;
pub mod offline_indicator;
pub mod step_progress;
pub mod steps;
pub mod wizard_shell;

// Re-exports - Core components
pub use conversation_panel::ConversationPanel;
pub use step_progress::{StepProgress, StepProgressVertical};
pub use steps::*;
pub use wizard_shell::WizardShell;

// Re-exports - Integration components (Phase 7)
pub use auto_save::{use_auto_save, AutoSaveIndicator, AutoSaveState, AutoSaveStatus};
pub use draft_recovery::{use_draft_recovery, DraftBadge, DraftRecoveryModal};
pub use error_display::{
    ButtonLoadingVariant, EmptyState, ErrorBanner, ErrorSeverity, FeatureUnavailable, InlineError,
    InlineLoading, LoadingButton, LoadingOverlay, Skeleton, SkeletonCard,
};
pub use help_tooltip::{
    terminology, ContextualHelp, HelpExpander, HelpIcon, LabelWithHelp, RichTooltip, Tooltip,
    TooltipPosition,
};
pub use interview_mode::{
    get_default_questions, get_inspiration, FieldType, InspirationPrompt, InterviewAnswer,
    InterviewMode, InterviewQuestion, InterviewState, QuestionCategory, SuggestionChip,
};
pub use offline_indicator::{
    use_llm_availability, AiFeatureDisabledNotice, AiStatusDot, AiUnavailableBanner,
    LlmAvailability, OfflineQueue, QueuedRequestsIndicator,
};
