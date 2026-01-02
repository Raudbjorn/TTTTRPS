//! Campaign Management Module
//!
//! Provides campaign versioning, world state tracking, and entity relationship management.

pub mod versioning;
pub mod world_state;
pub mod relationships;

// Re-exports for convenience
pub use versioning::{
    CampaignVersion, VersionType, CampaignDiff, DiffEntry, DiffOperation, VersionManager,
};
pub use world_state::{
    WorldState, WorldEvent, WorldEventType, LocationState, NpcRelationshipState,
    InGameDate, WorldStateManager,
};
pub use relationships::{
    EntityRelationship, RelationshipType, EntityType, RelationshipStrength,
    RelationshipManager, EntityGraph, GraphNode, GraphEdge,
};
