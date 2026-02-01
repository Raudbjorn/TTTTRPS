//! Session Manager Unit Tests
//!
//! Comprehensive tests for SessionManager covering combat mechanics,
//! conditions, notes, and session lifecycle.
//!
//! Submodules:
//! - `lifecycle`: Session creation, pausing, resuming, ending tests
//! - `combat_mechanics`: Initiative, turns, HP modification, combatant management
//! - `conditions`: Condition application, duration, stacking, immunities
//! - `notes`: Session notes, timeline events, snapshots

mod combat_mechanics;
mod conditions;
mod lifecycle;
mod notes;
