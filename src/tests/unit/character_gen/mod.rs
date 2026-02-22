//! Character Generation Unit Tests
//!
//! This module contains unit tests for character generation across multiple TTRPG systems.
//!
//! ## Test Coverage
//!
//! ### D&D 5th Edition (`dnd5e_tests`)
//! - Character creation with valid inputs
//! - Attribute generation (standard array, rolled)
//! - Class/subclass feature assignment
//! - Race ability bonuses
//! - Proficiency calculation
//! - Equipment generation
//!
//! ### Pathfinder 2nd Edition (`pf2e_tests`)
//! - Ancestry selection and bonuses
//! - Class features
//! - Action economy validation (reactions)
//! - Dedication feat validation
//! - Proficiency rank calculations
//!
//! ### Call of Cthulhu 7th Edition (`coc_tests`)
//! - Occupation selection
//! - Skill point allocation
//! - Sanity and luck calculation
//! - Characteristic rolling (3d6*5, (2d6+6)*5)
//! - Derived statistics (HP, Sanity, Magic Points)
//! - Backstory generation elements
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all character generation tests
//! cargo test tests::unit::character_gen
//!
//! # Run D&D 5e tests only
//! cargo test tests::unit::character_gen::dnd5e_tests
//!
//! # Run PF2e tests only
//! cargo test tests::unit::character_gen::pf2e_tests
//!
//! # Run Call of Cthulhu tests only
//! cargo test tests::unit::character_gen::coc_tests
//! ```

mod dnd5e_tests;
mod pf2e_tests;
mod coc_tests;
