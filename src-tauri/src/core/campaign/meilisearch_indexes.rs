//! Meilisearch Index Configurations for Campaign Generation
//!
//! Defines index schemas for:
//! - `ttrpg_campaign_arcs` - Campaign narrative arcs
//! - `ttrpg_session_plans` - Session planning documents
//! - `ttrpg_plot_points` - Enhanced plot points with dependencies
//!
//! TASK-CAMP-001, TASK-CAMP-002, TASK-CAMP-003

use meilisearch_sdk::settings::Settings;
use serde::{Deserialize, Serialize};

// ============================================================================
// Index Names
// ============================================================================

/// Index for campaign narrative arcs
pub const INDEX_CAMPAIGN_ARCS: &str = "ttrpg_campaign_arcs";

/// Index for session plans
pub const INDEX_SESSION_PLANS: &str = "ttrpg_session_plans";

/// Index for enhanced plot points
pub const INDEX_PLOT_POINTS: &str = "ttrpg_plot_points";

// ============================================================================
// Index Configuration Trait
// ============================================================================

/// Configuration for a Meilisearch index
pub trait IndexConfig {
    /// Get the index name
    fn index_name() -> &'static str;

    /// Get the primary key field
    fn primary_key() -> &'static str;

    /// Get the searchable attributes
    fn searchable_attributes() -> Vec<&'static str>;

    /// Get the filterable attributes
    fn filterable_attributes() -> Vec<&'static str>;

    /// Get the sortable attributes
    fn sortable_attributes() -> Vec<&'static str>;

    /// Build Meilisearch settings from configuration
    fn build_settings() -> Settings {
        Settings::new()
            .with_searchable_attributes(Self::searchable_attributes())
            .with_filterable_attributes(Self::filterable_attributes())
            .with_sortable_attributes(Self::sortable_attributes())
    }
}

// ============================================================================
// Campaign Arcs Index (TASK-CAMP-001)
// ============================================================================

/// Index configuration for campaign arcs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignArcsIndexConfig;

impl IndexConfig for CampaignArcsIndexConfig {
    fn index_name() -> &'static str {
        INDEX_CAMPAIGN_ARCS
    }

    fn primary_key() -> &'static str {
        "id"
    }

    fn searchable_attributes() -> Vec<&'static str> {
        vec!["name", "description", "premise"]
    }

    fn filterable_attributes() -> Vec<&'static str> {
        vec![
            "id",
            "campaign_id",
            "arc_type",
            "status",
            "is_main_arc",
        ]
    }

    fn sortable_attributes() -> Vec<&'static str> {
        vec!["name", "display_order", "started_at", "created_at"]
    }
}

// ============================================================================
// Session Plans Index (TASK-CAMP-002)
// ============================================================================

/// Index configuration for session plans
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPlansIndexConfig;

impl IndexConfig for SessionPlansIndexConfig {
    fn index_name() -> &'static str {
        INDEX_SESSION_PLANS
    }

    fn primary_key() -> &'static str {
        "id"
    }

    fn searchable_attributes() -> Vec<&'static str> {
        vec!["title", "summary", "dramatic_questions"]
    }

    fn filterable_attributes() -> Vec<&'static str> {
        vec![
            "id",
            "campaign_id",
            "session_id",
            "arc_id",
            "phase_id",
            "status",
            "is_template",
        ]
    }

    fn sortable_attributes() -> Vec<&'static str> {
        vec!["title", "session_number", "created_at"]
    }
}

// ============================================================================
// Plot Points Index (TASK-CAMP-003)
// ============================================================================

/// Index configuration for enhanced plot points
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotPointsIndexConfig;

impl IndexConfig for PlotPointsIndexConfig {
    fn index_name() -> &'static str {
        INDEX_PLOT_POINTS
    }

    fn primary_key() -> &'static str {
        "id"
    }

    fn searchable_attributes() -> Vec<&'static str> {
        vec!["title", "description", "dramatic_question", "notes"]
    }

    fn filterable_attributes() -> Vec<&'static str> {
        vec![
            "id",
            "campaign_id",
            "arc_id",
            "plot_type",
            "activation_state",
            "status",
            "urgency",
            "tension_level",
            "involved_npcs",
            "involved_locations",
            "tags",
        ]
    }

    fn sortable_attributes() -> Vec<&'static str> {
        vec![
            "title",
            "tension_level",
            "urgency",
            "created_at",
            "activated_at",
        ]
    }
}

// ============================================================================
// All Index Configurations
// ============================================================================

/// Get all campaign generation index names.
///
/// Derived from [`get_index_configs`] to keep the single source of truth.
pub fn all_campaign_indexes() -> Vec<&'static str> {
    get_index_configs().iter().map(|c| c.name).collect()
}

/// Index initialization configuration
#[derive(Debug, Clone)]
pub struct IndexInitConfig {
    /// Index name
    pub name: &'static str,
    /// Primary key field
    pub primary_key: &'static str,
    /// Settings to apply
    pub settings: Settings,
}

/// Get all index initialization configurations
pub fn get_index_configs() -> Vec<IndexInitConfig> {
    vec![
        IndexInitConfig {
            name: CampaignArcsIndexConfig::index_name(),
            primary_key: CampaignArcsIndexConfig::primary_key(),
            settings: CampaignArcsIndexConfig::build_settings(),
        },
        IndexInitConfig {
            name: SessionPlansIndexConfig::index_name(),
            primary_key: SessionPlansIndexConfig::primary_key(),
            settings: SessionPlansIndexConfig::build_settings(),
        },
        IndexInitConfig {
            name: PlotPointsIndexConfig::index_name(),
            primary_key: PlotPointsIndexConfig::primary_key(),
            settings: PlotPointsIndexConfig::build_settings(),
        },
    ]
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_campaign_arcs_index_config() {
        assert_eq!(CampaignArcsIndexConfig::index_name(), INDEX_CAMPAIGN_ARCS);
        assert_eq!(CampaignArcsIndexConfig::primary_key(), "id");
        assert!(CampaignArcsIndexConfig::searchable_attributes().contains(&"name"));
        assert!(CampaignArcsIndexConfig::filterable_attributes().contains(&"campaign_id"));
        assert!(CampaignArcsIndexConfig::sortable_attributes().contains(&"created_at"));
    }

    #[test]
    fn test_session_plans_index_config() {
        assert_eq!(SessionPlansIndexConfig::index_name(), INDEX_SESSION_PLANS);
        assert_eq!(SessionPlansIndexConfig::primary_key(), "id");
        assert!(SessionPlansIndexConfig::searchable_attributes().contains(&"dramatic_questions"));
        assert!(SessionPlansIndexConfig::filterable_attributes().contains(&"is_template"));
    }

    #[test]
    fn test_plot_points_index_config() {
        assert_eq!(PlotPointsIndexConfig::index_name(), INDEX_PLOT_POINTS);
        assert_eq!(PlotPointsIndexConfig::primary_key(), "id");
        assert!(PlotPointsIndexConfig::searchable_attributes().contains(&"dramatic_question"));
        assert!(PlotPointsIndexConfig::filterable_attributes().contains(&"tension_level"));
        assert!(PlotPointsIndexConfig::filterable_attributes().contains(&"urgency"));
    }

    #[test]
    fn test_all_campaign_indexes() {
        let indexes = all_campaign_indexes();
        assert_eq!(indexes.len(), 3);
        assert!(indexes.contains(&INDEX_CAMPAIGN_ARCS));
        assert!(indexes.contains(&INDEX_SESSION_PLANS));
        assert!(indexes.contains(&INDEX_PLOT_POINTS));
    }

    #[test]
    fn test_get_index_configs() {
        let configs = get_index_configs();
        assert_eq!(configs.len(), 3);

        let names: Vec<_> = configs.iter().map(|c| c.name).collect();
        assert!(names.contains(&INDEX_CAMPAIGN_ARCS));
        assert!(names.contains(&INDEX_SESSION_PLANS));
        assert!(names.contains(&INDEX_PLOT_POINTS));
    }
}
