pub mod ai;
pub mod audio;
pub mod auth;
pub mod campaign;
pub mod core;
pub mod library;
pub mod mechanics;
pub mod search;
pub mod system;
pub mod world;

#[cfg(test)]
mod tests;

// Re-export everything to maintain backward compatibility
pub use ai::*;
pub use audio::*;
pub use auth::*;
pub use campaign::*;
pub use core::*;
pub use library::*;
pub use mechanics::*;
pub use search::*;
pub use system::*;
pub use world::*;
