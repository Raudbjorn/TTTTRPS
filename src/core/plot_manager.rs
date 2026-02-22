//! Plot Manager Module
//!
//! Manages campaign plot points, quests, and story arcs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// Types
// ============================================================================

/// Plot point status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlotStatus {
    /// Not yet introduced
    Pending,
    /// Currently active
    Active,
    /// Successfully completed
    Completed,
    /// Failed or abandoned
    Failed,
    /// On hold
    Paused,
}

impl PlotStatus {
    /// Convert to string representation for migration
    pub fn as_str(&self) -> &'static str {
        match self {
            PlotStatus::Pending => "pending",
            PlotStatus::Active => "active",
            PlotStatus::Completed => "completed",
            PlotStatus::Failed => "failed",
            PlotStatus::Paused => "paused",
        }
    }
}

/// Plot point priority
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlotPriority {
    /// Background subplot
    Background,
    /// Side quest
    Side,
    /// Main story arc
    Main,
    /// Critical/urgent
    Critical,
}

impl PlotPriority {
    /// Convert to string representation for migration
    pub fn as_str(&self) -> &'static str {
        match self {
            PlotPriority::Background => "background",
            PlotPriority::Side => "side",
            PlotPriority::Main => "main",
            PlotPriority::Critical => "critical",
        }
    }
}

/// A campaign plot point or quest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotPoint {
    /// Unique identifier
    pub id: String,
    /// Campaign this belongs to
    pub campaign_id: String,
    /// Plot point title
    pub title: String,
    /// Description
    pub description: String,
    /// Current status
    pub status: PlotStatus,
    /// Priority level
    pub priority: PlotPriority,
    /// Involved NPC IDs
    pub involved_npcs: Vec<String>,
    /// Involved location IDs
    pub involved_locations: Vec<String>,
    /// Prerequisite plot point IDs (must be completed first)
    pub prerequisites: Vec<String>,
    /// Plot points that this unlocks
    pub unlocks: Vec<String>,
    /// Potential consequences
    pub consequences: Vec<String>,
    /// Rewards (text descriptions)
    pub rewards: Vec<String>,
    /// Session notes related to this plot
    pub notes: Vec<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
    /// When status changed to active
    pub started_at: Option<DateTime<Utc>>,
    /// When status changed to completed/failed
    pub resolved_at: Option<DateTime<Utc>>,
}

impl PlotPoint {
    pub fn new(campaign_id: &str, title: &str, priority: PlotPriority) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            campaign_id: campaign_id.to_string(),
            title: title.to_string(),
            description: String::new(),
            status: PlotStatus::Pending,
            priority,
            involved_npcs: Vec::new(),
            involved_locations: Vec::new(),
            prerequisites: Vec::new(),
            unlocks: Vec::new(),
            consequences: Vec::new(),
            rewards: Vec::new(),
            notes: Vec::new(),
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            started_at: None,
            resolved_at: None,
        }
    }
}

/// Plot arc (collection of related plot points)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotArc {
    pub id: String,
    pub campaign_id: String,
    pub name: String,
    pub description: String,
    pub plot_points: Vec<String>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Plot Manager
// ============================================================================

/// Manages plot points and story arcs
pub struct PlotManager {
    /// Plot points by ID
    plot_points: RwLock<HashMap<String, PlotPoint>>,
    /// Plot arcs by ID
    arcs: RwLock<HashMap<String, PlotArc>>,
}

impl PlotManager {
    pub fn new() -> Self {
        Self {
            plot_points: RwLock::new(HashMap::new()),
            arcs: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new plot point
    pub fn create(&self, plot: PlotPoint) -> String {
        let id = plot.id.clone();
        let mut plots = self.plot_points.write().unwrap();
        plots.insert(id.clone(), plot);
        id
    }

    /// Get a plot point by ID
    pub fn get(&self, id: &str) -> Option<PlotPoint> {
        let plots = self.plot_points.read().unwrap();
        plots.get(id).cloned()
    }

    /// Update a plot point
    pub fn update(&self, plot: PlotPoint) -> bool {
        let mut plots = self.plot_points.write().unwrap();
        if plots.contains_key(&plot.id) {
            let mut updated = plot;
            updated.updated_at = Utc::now();
            plots.insert(updated.id.clone(), updated);
            true
        } else {
            false
        }
    }

    /// Delete a plot point
    pub fn delete(&self, id: &str) -> bool {
        let mut plots = self.plot_points.write().unwrap();

        // Remove from prerequisites and unlocks of other plots
        if plots.contains_key(id) {
            let plot_id = id.to_string();
            for other in plots.values_mut() {
                other.prerequisites.retain(|p| p != &plot_id);
                other.unlocks.retain(|u| u != &plot_id);
            }
        }

        plots.remove(id).is_some()
    }

    /// List all plot points for a campaign
    pub fn list_by_campaign(&self, campaign_id: &str) -> Vec<PlotPoint> {
        let plots = self.plot_points.read().unwrap();
        plots
            .values()
            .filter(|p| p.campaign_id == campaign_id)
            .cloned()
            .collect()
    }

    /// Get plot points by status
    pub fn get_by_status(&self, campaign_id: &str, status: &PlotStatus) -> Vec<PlotPoint> {
        let plots = self.plot_points.read().unwrap();
        plots
            .values()
            .filter(|p| p.campaign_id == campaign_id && &p.status == status)
            .cloned()
            .collect()
    }

    /// Get active plot points
    pub fn get_active(&self, campaign_id: &str) -> Vec<PlotPoint> {
        self.get_by_status(campaign_id, &PlotStatus::Active)
    }

    /// Transition a plot point to a new status
    pub fn transition_status(&self, id: &str, new_status: PlotStatus) -> bool {
        let mut plots = self.plot_points.write().unwrap();

        if let Some(plot) = plots.get_mut(id) {
            let now = Utc::now();

            // Track timestamps
            match new_status {
                PlotStatus::Active if plot.started_at.is_none() => {
                    plot.started_at = Some(now);
                }
                PlotStatus::Completed | PlotStatus::Failed => {
                    plot.resolved_at = Some(now);
                }
                _ => {}
            }

            plot.status = new_status;
            plot.updated_at = now;

            return true;
        }

        false
    }

    /// Check if prerequisites are met
    pub fn prerequisites_met(&self, plot_id: &str) -> bool {
        let plots = self.plot_points.read().unwrap();

        if let Some(plot) = plots.get(plot_id) {
            for prereq_id in &plot.prerequisites {
                if let Some(prereq) = plots.get(prereq_id) {
                    if prereq.status != PlotStatus::Completed {
                        return false;
                    }
                } else {
                    return false; // Prerequisite doesn't exist
                }
            }
            true
        } else {
            false
        }
    }

    /// Get plot points that are ready to start (prerequisites met, still pending)
    pub fn get_available(&self, campaign_id: &str) -> Vec<PlotPoint> {
        let plots = self.plot_points.read().unwrap();

        plots
            .values()
            .filter(|p| {
                p.campaign_id == campaign_id
                    && p.status == PlotStatus::Pending
                    && p.prerequisites.iter().all(|prereq_id| {
                        plots
                            .get(prereq_id)
                            .map(|prereq| prereq.status == PlotStatus::Completed)
                            .unwrap_or(false)
                    })
            })
            .cloned()
            .collect()
    }

    /// Get plot points involving an NPC
    pub fn get_by_npc(&self, npc_id: &str) -> Vec<PlotPoint> {
        let plots = self.plot_points.read().unwrap();
        plots
            .values()
            .filter(|p| p.involved_npcs.contains(&npc_id.to_string()))
            .cloned()
            .collect()
    }

    /// Get plot points involving a location
    pub fn get_by_location(&self, location_id: &str) -> Vec<PlotPoint> {
        let plots = self.plot_points.read().unwrap();
        plots
            .values()
            .filter(|p| p.involved_locations.contains(&location_id.to_string()))
            .cloned()
            .collect()
    }

    /// Add a note to a plot point
    pub fn add_note(&self, plot_id: &str, note: &str) -> bool {
        let mut plots = self.plot_points.write().unwrap();

        if let Some(plot) = plots.get_mut(plot_id) {
            plot.notes.push(note.to_string());
            plot.updated_at = Utc::now();
            return true;
        }

        false
    }

    /// Create a plot arc
    pub fn create_arc(&self, campaign_id: &str, name: &str, description: &str) -> String {
        let arc = PlotArc {
            id: Uuid::new_v4().to_string(),
            campaign_id: campaign_id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            plot_points: Vec::new(),
            created_at: Utc::now(),
        };

        let id = arc.id.clone();
        let mut arcs = self.arcs.write().unwrap();
        arcs.insert(id.clone(), arc);
        id
    }

    /// Add a plot point to an arc
    pub fn add_to_arc(&self, arc_id: &str, plot_id: &str) -> bool {
        let mut arcs = self.arcs.write().unwrap();

        if let Some(arc) = arcs.get_mut(arc_id) {
            if !arc.plot_points.contains(&plot_id.to_string()) {
                arc.plot_points.push(plot_id.to_string());
                return true;
            }
        }

        false
    }

    /// Get arc by ID
    pub fn get_arc(&self, id: &str) -> Option<PlotArc> {
        let arcs = self.arcs.read().unwrap();
        arcs.get(id).cloned()
    }

    /// List arcs for a campaign
    pub fn list_arcs(&self, campaign_id: &str) -> Vec<PlotArc> {
        let arcs = self.arcs.read().unwrap();
        arcs.values()
            .filter(|a| a.campaign_id == campaign_id)
            .cloned()
            .collect()
    }

    /// Search plot points
    pub fn search(&self, campaign_id: &str, query: &str) -> Vec<PlotPoint> {
        let plots = self.plot_points.read().unwrap();
        let query_lower = query.to_lowercase();

        plots
            .values()
            .filter(|p| {
                p.campaign_id == campaign_id
                    && (p.title.to_lowercase().contains(&query_lower)
                        || p.description.to_lowercase().contains(&query_lower)
                        || p.tags.iter().any(|t| t.to_lowercase().contains(&query_lower)))
            })
            .cloned()
            .collect()
    }
}

impl Default for PlotManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_plot() {
        let manager = PlotManager::new();

        let plot = PlotPoint::new("campaign-1", "Save the Princess", PlotPriority::Main);
        let id = manager.create(plot);

        assert!(!id.is_empty());

        let retrieved = manager.get(&id).unwrap();
        assert_eq!(retrieved.title, "Save the Princess");
        assert_eq!(retrieved.status, PlotStatus::Pending);
    }

    #[test]
    fn test_status_transition() {
        let manager = PlotManager::new();

        let plot = PlotPoint::new("campaign-1", "Quest", PlotPriority::Side);
        let id = manager.create(plot);

        manager.transition_status(&id, PlotStatus::Active);
        let plot = manager.get(&id).unwrap();
        assert_eq!(plot.status, PlotStatus::Active);
        assert!(plot.started_at.is_some());

        manager.transition_status(&id, PlotStatus::Completed);
        let plot = manager.get(&id).unwrap();
        assert_eq!(plot.status, PlotStatus::Completed);
        assert!(plot.resolved_at.is_some());
    }

    #[test]
    fn test_prerequisites() {
        let manager = PlotManager::new();

        let prereq = PlotPoint::new("campaign-1", "Prerequisite", PlotPriority::Side);
        let prereq_id = manager.create(prereq);

        let mut main_plot = PlotPoint::new("campaign-1", "Main Quest", PlotPriority::Main);
        main_plot.prerequisites.push(prereq_id.clone());
        let main_id = manager.create(main_plot);

        // Prerequisites not met
        assert!(!manager.prerequisites_met(&main_id));

        // Complete prerequisite
        manager.transition_status(&prereq_id, PlotStatus::Completed);

        // Now prerequisites are met
        assert!(manager.prerequisites_met(&main_id));
    }
}
