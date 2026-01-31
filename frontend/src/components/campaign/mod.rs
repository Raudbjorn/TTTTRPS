//! Campaign Management UI Components (TASK-008, TASK-028)
//!
//! Provides UI components for campaign dashboard, version history,
//! world state editing, and entity relationship visualization.
//!
//! Design metaphors:
//! - Spotify: Campaigns as "Albums" with cover art, genre, sessions as "tracks"

pub mod campaign_dashboard;
pub mod campaign_card;
pub mod campaign_create_modal;
pub mod entity_browser;
pub mod version_history;
pub mod world_state_editor;
pub mod relationship_graph;
pub mod relationship_editor;
pub mod random_table;

// Re-exports
pub use campaign_dashboard::CampaignDashboard;
pub use campaign_card::{CampaignCard, CampaignCardCompact, CampaignCardMini, CampaignGenre};
pub use campaign_create_modal::CampaignCreateModal;
pub use entity_browser::EntityBrowser;
pub use version_history::VersionHistory;
pub use world_state_editor::WorldStateEditor;
pub use relationship_graph::RelationshipGraph;
pub use relationship_editor::RelationshipEditor;
pub use random_table::{RandomTableDisplay, RollHistorySidebar, DiceRollerWidget};
