//! Common Test Utilities
//!
//! Shared test helpers, fixtures, and mock implementations used across test modules.
//! This module provides:
//! - Database fixture creation (`fixtures`)
//! - Input validation testing (`validators`)
//! - Credential and audit mocks

pub mod fixtures;
pub mod validators;

pub use fixtures::*;
// validators re-export available when needed: use crate::tests::common::validators::*;
