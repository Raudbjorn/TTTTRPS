//! System-Specific Character Generators
//!
//! Each module implements the SystemGenerator trait for a specific TTRPG system.

pub mod dnd5e;
pub mod pf2e;
pub mod coc;
pub mod cyberpunk;
pub mod shadowrun;
pub mod fate;
pub mod wod;
pub mod dungeon_world;
pub mod gurps;
pub mod warhammer;

// Re-exports for convenience
pub use dnd5e::DnD5eGenerator;
pub use pf2e::Pathfinder2eGenerator;
pub use coc::CallOfCthulhuGenerator;
pub use cyberpunk::CyberpunkGenerator;
pub use shadowrun::ShadowrunGenerator;
pub use fate::FateCoreGenerator;
pub use wod::WorldOfDarknessGenerator;
pub use dungeon_world::DungeonWorldGenerator;
pub use gurps::GURPSGenerator;
pub use warhammer::WarhammerGenerator;
