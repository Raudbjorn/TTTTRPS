# Collaborative Features - Deferred Planning

This directory contains planning documents for **multi-user and collaborative features** that have been intentionally deferred from the initial Campaign Generation Overhaul.

## Rationale

These features require distributed systems considerations (conflict resolution, real-time sync, multi-user permissions) that add significant complexity. The decision was made to:

1. **Focus first** on making excellent single-GM campaign creation and management
2. **Defer** collaborative features until the core experience is solid
3. **Architect** the core system to not preclude future collaboration (data models, IDs, etc.)

## Deferred Features

### 1. Collaborative Campaign Creation

**Concept:** Multiple GMs can edit the same campaign simultaneously.

**Key Challenges:**
- Real-time conflict resolution (two GMs edit same NPC)
- Operational transformation or CRDTs for concurrent edits
- Permission models (owner, editor, viewer)
- Offline edit handling and sync

**Potential Approaches:**
- WebSocket-based real-time sync
- Async collaboration (lock-edit-unlock pattern)
- Git-like branching/merging for campaigns

### 2. Player-Facing Companion App

**Concept:** Players have limited read-only access to their own data.

**Key Challenges:**
- What players can see vs GM secrets
- Character sheet sync (if GM manages sheets)
- Session journal from player perspective
- Known NPCs/locations filtering

**Potential Approaches:**
- Share links with limited scope
- Separate player accounts with invitation flow
- Read-only API with filtering

### 3. Live Session Logging

**Concept:** Auto-capture session events from voice transcription during play.

**Key Challenges:**
- Speech-to-text accuracy in noisy environments
- Speaker identification (which player said what)
- Separating in-game dialogue from out-of-game chatter
- Privacy considerations

**Potential Approaches:**
- Integration with Discord/voice chat services
- Local transcription with whisper.cpp
- Manual triggers ("start recording this scene")

### 4. Shared Campaign Templates

**Concept:** GMs can share campaign frameworks publicly.

**Key Challenges:**
- Content moderation
- Licensing and attribution
- Version updates when templates change
- Privacy (scrubbing personal content before sharing)

**Potential Approaches:**
- Community hub/marketplace
- Import/export with sanitization
- Template versioning system

## Dependencies on Core Campaign

These features depend on having these core capabilities first:

- [x] Campaign data model (in progress)
- [ ] Session and arc management
- [ ] NPC and location systems
- [ ] State persistence and recovery
- [ ] Content generation

## When to Revisit

Consider implementing collaborative features when:

1. Core campaign UX is stable and well-tested
2. User feedback indicates collaboration is high priority
3. Team has capacity for distributed systems work
4. Clear business case for multi-user scenarios

## Architecture Notes for Future

To keep the door open for collaboration, the core campaign system should:

- Use UUIDs for all entities (supports distributed ID generation)
- Include `updated_at` timestamps (supports conflict detection)
- Avoid global state in memory (supports multiple sessions)
- Keep campaign state JSON-serializable (supports sync)
- Document any assumptions about single-user access

---

*Created: 2025-01-25*
*Status: Deferred - revisit after Campaign Generation MVP*
