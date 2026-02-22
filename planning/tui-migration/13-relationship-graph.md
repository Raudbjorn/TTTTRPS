# 13 — Relationship Graph

**Gap addressed:** #9 (MISSING — no segment)

## Overview

`core/campaign/relationships.rs` implements a directed graph of entity relationships with bidirectional support, strength scoring, and graph traversal. Currently **in-memory only** (`RwLock<HashMap<String, Vec<EntityRelationship>>>`). The SurrealDB `npc_relation` table is defined in schema but not yet wired to this module.

## Entity Types (9 + Custom)

`PC`, `NPC` (default), `Location`, `Faction`, `Item`, `Event`, `Quest`, `Deity`, `Creature`, `Custom(String)`

## Relationship Types (30 + Custom)

| Category | Types |
|----------|-------|
| Personal | Ally, Enemy, Romantic, Family, Mentor, Acquaintance (default) |
| Professional | Employee, BusinessPartner, Patron, Teacher, Protector |
| Organizational | MemberOf, LeaderOf, AlliedWith, AtWarWith, VassalOf |
| Spatial | LocatedAt, ConnectedTo, PartOf, Controls |
| Object | Owns, Seeks, Created, Destroyed |
| Quest/Plot | QuestGiver, QuestTarget, RelatedTo |
| Divine | Worships, BlessedBy, CursedBy |
| Custom | Custom(String) |

**Bidirectional types** (own inverse): Ally, Enemy, Romantic, Family, Acquaintance, BusinessPartner, AlliedWith, AtWarWith, ConnectedTo, RelatedTo

## Key Types

**EntityRelationship:**
- `id`, `campaign_id`, `source_id`, `source_type`, `source_name`
- `target_id`, `target_type`, `target_name`
- `relationship_type`, `strength` (Weak=25, Moderate=50, Strong=75, Unbreakable=100, Custom(u8))
- `is_active`, `is_known`, `description`
- `started_at`, `ended_at` (optional timestamps)
- `tags: Vec<String>`, `metadata: HashMap<String, Value>`

**RelationshipManagerConfig:**
- `auto_create_inverse: bool` (default: false)
- `max_relationships_per_campaign: usize` (default: 10000)

## Graph Types

**EntityGraph:** `nodes: Vec<GraphNode>`, `edges: Vec<GraphEdge>`, `stats: GraphStats`

**GraphNode:** id, name, entity_type, color, connection_count, is_hub (>5 connections), data

**GraphEdge:** id, source, target, label, strength (u8), bidirectional, is_active, color

**GraphStats:** node_count, edge_count, entity_type_counts, relationship_type_counts, most_connected_entities (top 5)

## Graph Queries

- `get_entity_graph(campaign_id, include_inactive)` → full campaign graph
- `get_ego_graph(campaign_id, entity_id, depth)` → BFS traversal from entity
- `get_relationships_between(campaign_id, entity_a, entity_b)` → direct links
- `get_relationships_by_type`, `search_by_tag`

## Validation Rules

- No self-relationships (`source_id == target_id` → `SelfRelationship`)
- No duplicates (same source+target+type → `DuplicateRelationship`)
- Capacity limit (max_relationships_per_campaign)

## Storage Migration Note

Currently in-memory only. The SurrealDB `npc_relation` table exists in schema but `RelationshipManager` does not persist to it. TUI should plan for eventual SurrealDB persistence — the API shape won't change.

## TUI Requirements

1. **Relationship list** — campaign-scoped, filterable by type/entity/strength
2. **Relationship editor** — create/edit with source/target picker, type dropdown, strength slider
3. **Entity graph viewer** — ego graph (centered on entity) or full campaign graph
4. **Graph stats** — node/edge counts, most connected entities, type distribution
5. **Tag search** — filter relationships by tags
6. **Active/known toggles** — filter visible relationships
