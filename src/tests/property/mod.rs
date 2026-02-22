//! Property-based tests for TTRPG Assistant
//!
//! This module contains property-based tests using the proptest framework.
//! Property tests verify invariants that should hold for all inputs, rather
//! than testing specific cases.
//!
//! ## Running Property Tests
//!
//! Run all property tests:
//! ```sh
//! cargo test property --release
//! ```
//!
//! Run a specific property test module:
//! ```sh
//! cargo test property::name_generator_props --release
//! ```
//!
//! ## Test Modules
//!
//! - `name_generator_props`: Tests for the name generation system
//!   - Output is valid UTF-8
//!   - Output length is reasonable (1-100 chars)
//!   - Deterministic given same seed
//!   - No offensive content (basic filter)
//!
//! - `cost_calculator_props`: Tests for the LLM cost calculation system
//!   - Cost is non-negative
//!   - Cost increases monotonically with tokens
//!   - Cost is bounded for bounded input
//!   - Zero tokens yields zero cost
//!
//! - `input_validator_props`: Tests for the security input validation system
//!   - Never accepts script tags
//!   - Never accepts SQL keywords in dangerous positions
//!   - Never accepts path traversal sequences
//!   - Accepts all alphanumeric input
//!   - Consistent results for same input
//!
//! - `search_ranking_props`: Tests for search ranking behavior
//!   - Same query returns same order
//!   - More relevant results score higher
//!   - Empty query returns no results
//!   - Ranking is transitive
//!
//! - `token_counter_props`: Tests for token counting estimation
//!   - Count is non-negative
//!   - Empty string yields zero tokens
//!   - Count increases with string length
//!   - Count is within 20% of actual (spot check)
//!
//! ## Property Testing Philosophy
//!
//! Property-based testing helps find edge cases that manual test cases might miss.
//! The proptest framework will:
//!
//! 1. Generate random inputs based on defined strategies
//! 2. Test each property with many different inputs
//! 3. If a failure is found, shrink the input to find the minimal failing case
//! 4. Store failing cases in a regression file for future testing
//!
//! ## Configuration
//!
//! By default, proptest runs 256 cases per property. This can be configured
//! via the `PROPTEST_CASES` environment variable:
//!
//! ```sh
//! PROPTEST_CASES=1000 cargo test property --release
//! ```

mod cost_calculator_props;
mod input_validator_props;
mod name_generator_props;
mod search_ranking_props;
mod token_counter_props;
