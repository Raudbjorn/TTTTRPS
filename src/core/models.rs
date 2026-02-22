use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export ContentChunk from chunker module
pub use crate::ingestion::chunker::ContentChunk;

// Re-export Campaign types from campaign_manager
pub use crate::core::campaign_manager::{Campaign, CampaignSettings, SessionNote, CampaignSnapshot, ThemeWeights};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDocument {
    pub id: String,
    pub title: String,
    pub system: String,
    pub source_type: String, // "rulebook", "flavor"
    pub metadata: HashMap<String, String>,
}
