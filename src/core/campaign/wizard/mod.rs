//! Wizard State Machine for Campaign Creation
//!
//! Manages the campaign creation wizard lifecycle, persisting progress and enabling recovery.
//!
//! # Overview
//!
//! The wizard guides users through campaign creation in discrete steps:
//! 1. Basics - Name, system, description
//! 2. Intent - Core fantasy, themes, constraints
//! 3. Scope - Session count, duration, arc structure
//! 4. Players - Player count and experience level
//! 5. Party Composition - Optional party analysis
//! 6. Arc Structure - Optional narrative arc planning
//! 7. Initial Content - Optional starting NPCs, locations
//! 8. Review - Final confirmation before creation
//!
//! # Design Principles
//!
//! - **Persistence**: Wizard state survives app restarts via SQLite
//! - **Progressive**: Users can move forward/backward preserving data
//! - **Skippable**: Optional steps can be bypassed
//! - **Recoverable**: Incomplete wizards can be resumed
//! - **AI-Assisted**: Optional AI suggestions at each step

mod types;
mod manager;

pub use types::*;
pub use manager::*;
