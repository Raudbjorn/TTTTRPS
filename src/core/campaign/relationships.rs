//! Entity Relationships Module (TASK-009)
//!
//! Provides entity relationship management for NPCs, locations, factions, items,
//! and other campaign entities. Supports graph visualization.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use thiserror::Error;
use uuid::Uuid;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum RelationshipError {
    #[error("Campaign not found: {0}")]
    CampaignNotFound(String),

    #[error("Entity not found: {0}")]
    EntityNotFound(String),

    #[error("Relationship not found: {0}")]
    RelationshipNotFound(String),

    #[error("Self-relationship not allowed")]
    SelfRelationship,

    #[error("Duplicate relationship")]
    DuplicateRelationship,

    #[error("Invalid relationship type for entities")]
    InvalidRelationshipType,
}

pub type Result<T> = std::result::Result<T, RelationshipError>;

// ============================================================================
// Entity Types
// ============================================================================

/// Type of entity that can participate in relationships
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[derive(Default)]
pub enum EntityType {
    /// Player character
    PC,
    /// Non-player character
    #[default]
    NPC,
    /// Location (city, dungeon, building, etc.)
    Location,
    /// Faction or organization
    Faction,
    /// Item or artifact
    Item,
    /// Event in the timeline
    Event,
    /// Quest or plotline
    Quest,
    /// Deity or divine entity
    Deity,
    /// Creature type or monster
    Creature,
    /// Custom entity type
    Custom(String),
}


impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PC => write!(f, "PC"),
            Self::NPC => write!(f, "NPC"),
            Self::Location => write!(f, "Location"),
            Self::Faction => write!(f, "Faction"),
            Self::Item => write!(f, "Item"),
            Self::Event => write!(f, "Event"),
            Self::Quest => write!(f, "Quest"),
            Self::Deity => write!(f, "Deity"),
            Self::Creature => write!(f, "Creature"),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

// ============================================================================
// Relationship Types
// ============================================================================

/// Type of relationship between entities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum RelationshipType {
    // Personal relationships
    /// Friend
    Ally,
    /// Enemy or rival
    Enemy,
    /// Romantic partner
    Romantic,
    /// Family member
    Family,
    /// Mentor/mentee
    Mentor,
    /// Acquaintance
    #[default]
    Acquaintance,

    // Professional relationships
    /// Employer/employee
    Employee,
    /// Business partner
    BusinessPartner,
    /// Client/patron
    Patron,
    /// Teacher/student
    Teacher,
    /// Guard/protector
    Protector,

    // Organizational relationships
    /// Member of (faction/organization)
    MemberOf,
    /// Leader of
    LeaderOf,
    /// Ally to (faction)
    AlliedWith,
    /// At war with
    AtWarWith,
    /// Vassal/subject
    VassalOf,

    // Spatial relationships
    /// Located at
    LocatedAt,
    /// Connected to (for locations)
    ConnectedTo,
    /// Part of (region/area)
    PartOf,
    /// Controls/governs
    Controls,

    // Object relationships
    /// Owns
    Owns,
    /// Seeks/wants
    Seeks,
    /// Created/made
    Created,
    /// Destroyed
    Destroyed,

    // Quest/plot relationships
    /// Gives quest
    QuestGiver,
    /// Quest target
    QuestTarget,
    /// Related to event
    RelatedTo,

    // Divine/magical
    /// Worships
    Worships,
    /// Blessed by
    BlessedBy,
    /// Cursed by
    CursedBy,

    /// Custom relationship type
    Custom(String),
}


impl std::fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ally => write!(f, "Ally"),
            Self::Enemy => write!(f, "Enemy"),
            Self::Romantic => write!(f, "Romantic"),
            Self::Family => write!(f, "Family"),
            Self::Mentor => write!(f, "Mentor"),
            Self::Acquaintance => write!(f, "Acquaintance"),
            Self::Employee => write!(f, "Employee"),
            Self::BusinessPartner => write!(f, "Business Partner"),
            Self::Patron => write!(f, "Patron"),
            Self::Teacher => write!(f, "Teacher"),
            Self::Protector => write!(f, "Protector"),
            Self::MemberOf => write!(f, "Member Of"),
            Self::LeaderOf => write!(f, "Leader Of"),
            Self::AlliedWith => write!(f, "Allied With"),
            Self::AtWarWith => write!(f, "At War With"),
            Self::VassalOf => write!(f, "Vassal Of"),
            Self::LocatedAt => write!(f, "Located At"),
            Self::ConnectedTo => write!(f, "Connected To"),
            Self::PartOf => write!(f, "Part Of"),
            Self::Controls => write!(f, "Controls"),
            Self::Owns => write!(f, "Owns"),
            Self::Seeks => write!(f, "Seeks"),
            Self::Created => write!(f, "Created"),
            Self::Destroyed => write!(f, "Destroyed"),
            Self::QuestGiver => write!(f, "Quest Giver"),
            Self::QuestTarget => write!(f, "Quest Target"),
            Self::RelatedTo => write!(f, "Related To"),
            Self::Worships => write!(f, "Worships"),
            Self::BlessedBy => write!(f, "Blessed By"),
            Self::CursedBy => write!(f, "Cursed By"),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

impl RelationshipType {
    /// Check if this relationship type is bidirectional
    pub fn is_bidirectional(&self) -> bool {
        matches!(
            self,
            Self::Ally
                | Self::Enemy
                | Self::Romantic
                | Self::Family
                | Self::Acquaintance
                | Self::BusinessPartner
                | Self::AlliedWith
                | Self::AtWarWith
                | Self::ConnectedTo
                | Self::RelatedTo
        )
    }

    /// Get the inverse relationship type (if applicable)
    pub fn inverse(&self) -> Option<Self> {
        match self {
            Self::Mentor => Some(Self::Custom("Mentee".to_string())),
            Self::Teacher => Some(Self::Custom("Student".to_string())),
            Self::Employee => Some(Self::Custom("Employer".to_string())),
            Self::Patron => Some(Self::Custom("Client".to_string())),
            Self::MemberOf => Some(Self::Custom("Has Member".to_string())),
            Self::LeaderOf => Some(Self::Custom("Led By".to_string())),
            Self::VassalOf => Some(Self::Custom("Liege Of".to_string())),
            Self::LocatedAt => Some(Self::Custom("Location Of".to_string())),
            Self::PartOf => Some(Self::Custom("Contains".to_string())),
            Self::Controls => Some(Self::Custom("Controlled By".to_string())),
            Self::Owns => Some(Self::Custom("Owned By".to_string())),
            Self::Created => Some(Self::Custom("Created By".to_string())),
            Self::QuestGiver => Some(Self::QuestTarget),
            Self::Worships => Some(Self::Custom("Worshipped By".to_string())),
            Self::BlessedBy => Some(Self::Custom("Blesses".to_string())),
            Self::CursedBy => Some(Self::Custom("Curses".to_string())),
            Self::Protector => Some(Self::Custom("Protected By".to_string())),
            // Bidirectional relationships are their own inverse
            _ if self.is_bidirectional() => Some(self.clone()),
            _ => None,
        }
    }
}

// ============================================================================
// Relationship Strength
// ============================================================================

/// Strength/intensity of a relationship
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum RelationshipStrength {
    /// Barely acquainted
    Weak,
    /// Normal relationship
    #[default]
    Moderate,
    /// Strong relationship
    Strong,
    /// Unbreakable bond
    Unbreakable,
    /// Custom strength with numeric value (0-100)
    Custom(u8),
}


impl RelationshipStrength {
    /// Get numeric value (0-100)
    pub fn value(&self) -> u8 {
        match self {
            Self::Weak => 25,
            Self::Moderate => 50,
            Self::Strong => 75,
            Self::Unbreakable => 100,
            Self::Custom(v) => *v,
        }
    }
}

// ============================================================================
// Entity Relationship
// ============================================================================

/// A relationship between two entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRelationship {
    /// Unique identifier
    pub id: String,
    /// Campaign this relationship belongs to
    pub campaign_id: String,
    /// Source entity ID
    pub source_id: String,
    /// Source entity type
    pub source_type: EntityType,
    /// Source entity name (for display)
    pub source_name: String,
    /// Target entity ID
    pub target_id: String,
    /// Target entity type
    pub target_type: EntityType,
    /// Target entity name (for display)
    pub target_name: String,
    /// Type of relationship
    pub relationship_type: RelationshipType,
    /// Strength of the relationship
    pub strength: RelationshipStrength,
    /// Is this relationship currently active?
    pub is_active: bool,
    /// Is this relationship known to the players?
    pub is_known: bool,
    /// Description/notes about the relationship
    pub description: String,
    /// When this relationship started (in-game)
    pub started_at: Option<String>,
    /// When this relationship ended (in-game)
    pub ended_at: Option<String>,
    /// Tags for filtering
    pub tags: Vec<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// When this was created
    pub created_at: DateTime<Utc>,
    /// When this was last updated
    pub updated_at: DateTime<Utc>,
}

impl EntityRelationship {
    /// Create a new relationship
    pub fn new(
        campaign_id: &str,
        source_id: &str,
        source_type: EntityType,
        source_name: &str,
        target_id: &str,
        target_type: EntityType,
        target_name: &str,
        relationship_type: RelationshipType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            campaign_id: campaign_id.to_string(),
            source_id: source_id.to_string(),
            source_type,
            source_name: source_name.to_string(),
            target_id: target_id.to_string(),
            target_type,
            target_name: target_name.to_string(),
            relationship_type,
            strength: RelationshipStrength::default(),
            is_active: true,
            is_known: true,
            description: String::new(),
            started_at: None,
            ended_at: None,
            tags: vec![],
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Builder: set strength
    pub fn with_strength(mut self, strength: RelationshipStrength) -> Self {
        self.strength = strength;
        self
    }

    /// Builder: set description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Builder: mark as secret
    pub fn as_secret(mut self) -> Self {
        self.is_known = false;
        self
    }

    /// Create the inverse relationship (if applicable)
    pub fn create_inverse(&self) -> Option<Self> {
        self.relationship_type.inverse().map(|inv_type| {
            let mut inverse = Self::new(
                &self.campaign_id,
                &self.target_id,
                self.target_type.clone(),
                &self.target_name,
                &self.source_id,
                self.source_type.clone(),
                &self.source_name,
                inv_type,
            );
            inverse.strength = self.strength.clone();
            inverse.is_active = self.is_active;
            inverse.is_known = self.is_known;
            inverse.description = self.description.clone();
            inverse.started_at = self.started_at.clone();
            inverse.ended_at = self.ended_at.clone();
            inverse
        })
    }
}

/// Summary of a relationship (for listing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSummary {
    pub id: String,
    pub source_id: String,
    pub source_name: String,
    pub source_type: EntityType,
    pub target_id: String,
    pub target_name: String,
    pub target_type: EntityType,
    pub relationship_type: RelationshipType,
    pub strength: RelationshipStrength,
    pub is_active: bool,
}

impl From<&EntityRelationship> for RelationshipSummary {
    fn from(r: &EntityRelationship) -> Self {
        Self {
            id: r.id.clone(),
            source_id: r.source_id.clone(),
            source_name: r.source_name.clone(),
            source_type: r.source_type.clone(),
            target_id: r.target_id.clone(),
            target_name: r.target_name.clone(),
            target_type: r.target_type.clone(),
            relationship_type: r.relationship_type.clone(),
            strength: r.strength.clone(),
            is_active: r.is_active,
        }
    }
}

// ============================================================================
// Graph Types for Visualization
// ============================================================================

/// A node in the entity graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Entity ID
    pub id: String,
    /// Display name
    pub name: String,
    /// Entity type
    pub entity_type: EntityType,
    /// Color hint for visualization
    pub color: String,
    /// Number of connections
    pub connection_count: usize,
    /// Is this entity a "hub" (many connections)?
    pub is_hub: bool,
    /// Custom data for visualization
    pub data: HashMap<String, serde_json::Value>,
}

/// An edge in the entity graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Relationship ID
    pub id: String,
    /// Source node ID
    pub source: String,
    /// Target node ID
    pub target: String,
    /// Relationship type label
    pub label: String,
    /// Relationship strength (0-100)
    pub strength: u8,
    /// Is this a bidirectional relationship?
    pub bidirectional: bool,
    /// Is this relationship active?
    pub is_active: bool,
    /// Color hint for visualization
    pub color: String,
}

/// Complete entity graph for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityGraph {
    /// All nodes in the graph
    pub nodes: Vec<GraphNode>,
    /// All edges in the graph
    pub edges: Vec<GraphEdge>,
    /// Stats about the graph
    pub stats: GraphStats,
}

/// Statistics about an entity graph
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub entity_type_counts: HashMap<String, usize>,
    pub relationship_type_counts: HashMap<String, usize>,
    pub most_connected_entities: Vec<(String, usize)>,
}

// ============================================================================
// Relationship Manager
// ============================================================================

/// Manages entity relationships for all campaigns
pub struct RelationshipManager {
    /// Campaign ID -> Vec<Relationship>
    relationships: RwLock<HashMap<String, Vec<EntityRelationship>>>,
    /// Configuration
    config: RelationshipManagerConfig,
}

/// Configuration for the relationship manager
#[derive(Debug, Clone)]
pub struct RelationshipManagerConfig {
    /// Auto-create inverse relationships for bidirectional types
    pub auto_create_inverse: bool,
    /// Maximum relationships per campaign
    pub max_relationships_per_campaign: usize,
}

impl Default for RelationshipManagerConfig {
    fn default() -> Self {
        Self {
            auto_create_inverse: false,
            max_relationships_per_campaign: 10000,
        }
    }
}

impl Default for RelationshipManager {
    fn default() -> Self {
        Self::new(RelationshipManagerConfig::default())
    }
}

impl RelationshipManager {
    pub fn new(config: RelationshipManagerConfig) -> Self {
        Self {
            relationships: RwLock::new(HashMap::new()),
            config,
        }
    }

    // ========================================================================
    // CRUD Operations
    // ========================================================================

    /// Create a new relationship
    pub fn create_relationship(&self, relationship: EntityRelationship) -> Result<EntityRelationship> {
        // Validate: no self-relationships
        if relationship.source_id == relationship.target_id {
            return Err(RelationshipError::SelfRelationship);
        }

        let mut rels = self.relationships.write().unwrap();
        let campaign_rels = rels
            .entry(relationship.campaign_id.clone())
            .or_default();

        // Check for duplicates
        if campaign_rels.iter().any(|r| {
            r.source_id == relationship.source_id
                && r.target_id == relationship.target_id
                && r.relationship_type == relationship.relationship_type
        }) {
            return Err(RelationshipError::DuplicateRelationship);
        }

        // Check max limit
        if campaign_rels.len() >= self.config.max_relationships_per_campaign {
            return Err(RelationshipError::RelationshipNotFound(
                "Max relationships reached".to_string(),
            ));
        }

        campaign_rels.push(relationship.clone());

        // Auto-create inverse if configured and applicable
        if self.config.auto_create_inverse {
            if let Some(inverse) = relationship.create_inverse() {
                if !campaign_rels.iter().any(|r| {
                    r.source_id == inverse.source_id
                        && r.target_id == inverse.target_id
                        && r.relationship_type == inverse.relationship_type
                }) {
                    campaign_rels.push(inverse);
                }
            }
        }

        Ok(relationship)
    }

    /// Get a relationship by ID
    pub fn get_relationship(&self, campaign_id: &str, relationship_id: &str) -> Option<EntityRelationship> {
        self.relationships
            .read()
            .unwrap()
            .get(campaign_id)
            .and_then(|rels| rels.iter().find(|r| r.id == relationship_id).cloned())
    }

    /// Update a relationship
    pub fn update_relationship(&self, relationship: EntityRelationship) -> Result<()> {
        let mut rels = self.relationships.write().unwrap();
        let campaign_rels = rels
            .get_mut(&relationship.campaign_id)
            .ok_or_else(|| RelationshipError::CampaignNotFound(relationship.campaign_id.clone()))?;

        let pos = campaign_rels
            .iter()
            .position(|r| r.id == relationship.id)
            .ok_or_else(|| RelationshipError::RelationshipNotFound(relationship.id.clone()))?;

        campaign_rels[pos] = relationship;
        Ok(())
    }

    /// Delete a relationship
    pub fn delete_relationship(&self, campaign_id: &str, relationship_id: &str) -> Result<()> {
        let mut rels = self.relationships.write().unwrap();
        let campaign_rels = rels
            .get_mut(campaign_id)
            .ok_or_else(|| RelationshipError::CampaignNotFound(campaign_id.to_string()))?;

        let pos = campaign_rels
            .iter()
            .position(|r| r.id == relationship_id)
            .ok_or_else(|| RelationshipError::RelationshipNotFound(relationship_id.to_string()))?;

        campaign_rels.remove(pos);
        Ok(())
    }

    /// Delete all relationships for a campaign
    pub fn delete_all_relationships(&self, campaign_id: &str) {
        self.relationships.write().unwrap().remove(campaign_id);
    }

    // ========================================================================
    // Query Operations
    // ========================================================================

    /// List all relationships for a campaign
    pub fn list_relationships(&self, campaign_id: &str) -> Vec<RelationshipSummary> {
        self.relationships
            .read()
            .unwrap()
            .get(campaign_id)
            .map(|rels| rels.iter().map(RelationshipSummary::from).collect())
            .unwrap_or_default()
    }

    /// Get all relationships for an entity
    pub fn get_entity_relationships(&self, campaign_id: &str, entity_id: &str) -> Vec<EntityRelationship> {
        self.relationships
            .read()
            .unwrap()
            .get(campaign_id)
            .map(|rels| {
                rels.iter()
                    .filter(|r| r.source_id == entity_id || r.target_id == entity_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get outgoing relationships from an entity
    pub fn get_outgoing_relationships(&self, campaign_id: &str, entity_id: &str) -> Vec<EntityRelationship> {
        self.relationships
            .read()
            .unwrap()
            .get(campaign_id)
            .map(|rels| {
                rels.iter()
                    .filter(|r| r.source_id == entity_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get incoming relationships to an entity
    pub fn get_incoming_relationships(&self, campaign_id: &str, entity_id: &str) -> Vec<EntityRelationship> {
        self.relationships
            .read()
            .unwrap()
            .get(campaign_id)
            .map(|rels| {
                rels.iter()
                    .filter(|r| r.target_id == entity_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find relationships between two specific entities
    pub fn get_relationships_between(
        &self,
        campaign_id: &str,
        entity_a: &str,
        entity_b: &str,
    ) -> Vec<EntityRelationship> {
        self.relationships
            .read()
            .unwrap()
            .get(campaign_id)
            .map(|rels| {
                rels.iter()
                    .filter(|r| {
                        (r.source_id == entity_a && r.target_id == entity_b)
                            || (r.source_id == entity_b && r.target_id == entity_a)
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Filter relationships by type
    pub fn get_relationships_by_type(
        &self,
        campaign_id: &str,
        relationship_type: &RelationshipType,
    ) -> Vec<EntityRelationship> {
        self.relationships
            .read()
            .unwrap()
            .get(campaign_id)
            .map(|rels| {
                rels.iter()
                    .filter(|r| &r.relationship_type == relationship_type)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Search relationships by tag
    pub fn search_by_tag(&self, campaign_id: &str, tag: &str) -> Vec<EntityRelationship> {
        self.relationships
            .read()
            .unwrap()
            .get(campaign_id)
            .map(|rels| {
                rels.iter()
                    .filter(|r| r.tags.iter().any(|t| t.to_lowercase().contains(&tag.to_lowercase())))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    // ========================================================================
    // Graph Generation
    // ========================================================================

    /// Generate an entity graph for visualization
    pub fn get_entity_graph(&self, campaign_id: &str, include_inactive: bool) -> EntityGraph {
        let rels = self.relationships.read().unwrap();
        let campaign_rels = match rels.get(campaign_id) {
            Some(r) => r,
            None => {
                return EntityGraph {
                    nodes: vec![],
                    edges: vec![],
                    stats: GraphStats::default(),
                }
            }
        };

        // Collect unique entities
        let mut entity_map: HashMap<String, GraphNode> = HashMap::new();
        let mut connection_counts: HashMap<String, usize> = HashMap::new();

        for rel in campaign_rels {
            if !include_inactive && !rel.is_active {
                continue;
            }

            // Source entity
            *connection_counts.entry(rel.source_id.clone()).or_insert(0) += 1;
            entity_map.entry(rel.source_id.clone()).or_insert_with(|| GraphNode {
                id: rel.source_id.clone(),
                name: rel.source_name.clone(),
                entity_type: rel.source_type.clone(),
                color: entity_type_color(&rel.source_type),
                connection_count: 0,
                is_hub: false,
                data: HashMap::new(),
            });

            // Target entity
            *connection_counts.entry(rel.target_id.clone()).or_insert(0) += 1;
            entity_map.entry(rel.target_id.clone()).or_insert_with(|| GraphNode {
                id: rel.target_id.clone(),
                name: rel.target_name.clone(),
                entity_type: rel.target_type.clone(),
                color: entity_type_color(&rel.target_type),
                connection_count: 0,
                is_hub: false,
                data: HashMap::new(),
            });
        }

        // Update connection counts and identify hubs
        let avg_connections = if entity_map.is_empty() {
            0.0
        } else {
            connection_counts.values().sum::<usize>() as f64 / entity_map.len() as f64
        };

        let mut nodes: Vec<GraphNode> = entity_map
            .into_iter()
            .map(|(id, mut node)| {
                let count = *connection_counts.get(&id).unwrap_or(&0);
                node.connection_count = count;
                node.is_hub = count as f64 > avg_connections * 2.0;
                node
            })
            .collect();

        // Create edges
        let edges: Vec<GraphEdge> = campaign_rels
            .iter()
            .filter(|r| include_inactive || r.is_active)
            .map(|r| GraphEdge {
                id: r.id.clone(),
                source: r.source_id.clone(),
                target: r.target_id.clone(),
                label: r.relationship_type.to_string(),
                strength: r.strength.value(),
                bidirectional: r.relationship_type.is_bidirectional(),
                is_active: r.is_active,
                color: relationship_type_color(&r.relationship_type),
            })
            .collect();

        // Calculate stats
        let mut entity_type_counts: HashMap<String, usize> = HashMap::new();
        let mut relationship_type_counts: HashMap<String, usize> = HashMap::new();

        for node in &nodes {
            *entity_type_counts
                .entry(node.entity_type.to_string())
                .or_insert(0) += 1;
        }

        for edge in &edges {
            *relationship_type_counts
                .entry(edge.label.clone())
                .or_insert(0) += 1;
        }

        // Sort nodes by connection count for "most connected"
        nodes.sort_by(|a, b| b.connection_count.cmp(&a.connection_count));
        let most_connected: Vec<(String, usize)> = nodes
            .iter()
            .take(5)
            .map(|n| (n.name.clone(), n.connection_count))
            .collect();

        let node_count = nodes.len();
        let edge_count = edges.len();

        EntityGraph {
            nodes,
            edges,
            stats: GraphStats {
                node_count,
                edge_count,
                entity_type_counts,
                relationship_type_counts,
                most_connected_entities: most_connected,
            },
        }
    }

    /// Get a subgraph centered on an entity (ego graph)
    pub fn get_ego_graph(&self, campaign_id: &str, entity_id: &str, depth: usize) -> EntityGraph {
        let full_graph = self.get_entity_graph(campaign_id, false);

        if depth == 0 {
            return EntityGraph {
                nodes: vec![],
                edges: vec![],
                stats: GraphStats::default(),
            };
        }

        // BFS to find entities within depth
        let mut visited: HashSet<String> = HashSet::new();
        let mut current_level: HashSet<String> = HashSet::new();
        current_level.insert(entity_id.to_string());
        visited.insert(entity_id.to_string());

        for _ in 0..depth {
            let mut next_level: HashSet<String> = HashSet::new();
            for edge in &full_graph.edges {
                if current_level.contains(&edge.source) && !visited.contains(&edge.target) {
                    next_level.insert(edge.target.clone());
                }
                if current_level.contains(&edge.target) && !visited.contains(&edge.source) {
                    next_level.insert(edge.source.clone());
                }
            }
            visited.extend(next_level.iter().cloned());
            current_level = next_level;
        }

        // Filter graph to visited nodes
        let nodes: Vec<GraphNode> = full_graph
            .nodes
            .into_iter()
            .filter(|n| visited.contains(&n.id))
            .collect();

        let edges: Vec<GraphEdge> = full_graph
            .edges
            .into_iter()
            .filter(|e| visited.contains(&e.source) && visited.contains(&e.target))
            .collect();

        EntityGraph {
            stats: GraphStats {
                node_count: nodes.len(),
                edge_count: edges.len(),
                ..Default::default()
            },
            nodes,
            edges,
        }
    }

    /// Get relationship count for a campaign
    pub fn relationship_count(&self, campaign_id: &str) -> usize {
        self.relationships
            .read()
            .unwrap()
            .get(campaign_id)
            .map(|r| r.len())
            .unwrap_or(0)
    }
}

// ============================================================================
// Color Helpers
// ============================================================================

fn entity_type_color(entity_type: &EntityType) -> String {
    match entity_type {
        EntityType::PC => "#3b82f6".to_string(),     // blue
        EntityType::NPC => "#8b5cf6".to_string(),    // purple
        EntityType::Location => "#10b981".to_string(), // emerald
        EntityType::Faction => "#f59e0b".to_string(),  // amber
        EntityType::Item => "#ec4899".to_string(),   // pink
        EntityType::Event => "#6366f1".to_string(),  // indigo
        EntityType::Quest => "#14b8a6".to_string(),  // teal
        EntityType::Deity => "#f97316".to_string(),  // orange
        EntityType::Creature => "#ef4444".to_string(), // red
        EntityType::Custom(_) => "#6b7280".to_string(), // gray
    }
}

fn relationship_type_color(rel_type: &RelationshipType) -> String {
    match rel_type {
        RelationshipType::Ally | RelationshipType::AlliedWith => "#22c55e".to_string(), // green
        RelationshipType::Enemy | RelationshipType::AtWarWith => "#ef4444".to_string(), // red
        RelationshipType::Romantic => "#ec4899".to_string(), // pink
        RelationshipType::Family => "#8b5cf6".to_string(),   // purple
        RelationshipType::MemberOf | RelationshipType::LeaderOf => "#f59e0b".to_string(), // amber
        RelationshipType::LocatedAt | RelationshipType::Controls => "#10b981".to_string(), // emerald
        _ => "#6b7280".to_string(), // gray
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_relationship() {
        let manager = RelationshipManager::default();

        let rel = EntityRelationship::new(
            "camp-1",
            "npc-1",
            EntityType::NPC,
            "Gandalf",
            "npc-2",
            EntityType::NPC,
            "Frodo",
            RelationshipType::Mentor,
        );

        let created = manager.create_relationship(rel).unwrap();
        assert!(!created.id.is_empty());

        let retrieved = manager.get_relationship("camp-1", &created.id).unwrap();
        assert_eq!(retrieved.source_name, "Gandalf");
    }

    #[test]
    fn test_self_relationship_error() {
        let manager = RelationshipManager::default();

        let rel = EntityRelationship::new(
            "camp-1",
            "npc-1",
            EntityType::NPC,
            "Test",
            "npc-1", // Same ID
            EntityType::NPC,
            "Test",
            RelationshipType::Ally,
        );

        assert!(matches!(
            manager.create_relationship(rel),
            Err(RelationshipError::SelfRelationship)
        ));
    }

    #[test]
    fn test_entity_graph() {
        let manager = RelationshipManager::default();

        // Create some relationships
        manager
            .create_relationship(EntityRelationship::new(
                "camp-1",
                "npc-1",
                EntityType::NPC,
                "Alice",
                "loc-1",
                EntityType::Location,
                "Neverwinter",
                RelationshipType::LocatedAt,
            ))
            .unwrap();

        manager
            .create_relationship(EntityRelationship::new(
                "camp-1",
                "npc-2",
                EntityType::NPC,
                "Bob",
                "loc-1",
                EntityType::Location,
                "Neverwinter",
                RelationshipType::LocatedAt,
            ))
            .unwrap();

        manager
            .create_relationship(EntityRelationship::new(
                "camp-1",
                "npc-1",
                EntityType::NPC,
                "Alice",
                "npc-2",
                EntityType::NPC,
                "Bob",
                RelationshipType::Ally,
            ))
            .unwrap();

        let graph = manager.get_entity_graph("camp-1", false);
        assert_eq!(graph.stats.node_count, 3);
        assert_eq!(graph.stats.edge_count, 3);
    }

    #[test]
    fn test_bidirectional_relationships() {
        assert!(RelationshipType::Ally.is_bidirectional());
        assert!(RelationshipType::Family.is_bidirectional());
        assert!(!RelationshipType::Mentor.is_bidirectional());
        assert!(!RelationshipType::LocatedAt.is_bidirectional());
    }

    #[test]
    fn test_relationship_inverse() {
        let mentor_inverse = RelationshipType::Mentor.inverse();
        assert!(mentor_inverse.is_some());

        let ally_inverse = RelationshipType::Ally.inverse();
        assert_eq!(ally_inverse, Some(RelationshipType::Ally));
    }
}
