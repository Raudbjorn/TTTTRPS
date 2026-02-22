//! Layout Detection Module
//!
//! Provides layout analysis for complex PDF documents, including:
//! - Multi-column boundary detection
//! - Boxed/shaded region detection (sidebars, callouts, read-aloud text)
//! - Table structure extraction with multi-page continuation support
//!
//! This module is designed to handle the complex layouts common in TTRPG
//! rulebooks, which often feature two-column text, stat block boxes,
//! sidebar notes, and tables that span multiple pages.

pub mod column_detector;
pub mod region_detector;
pub mod table_extractor;

pub use column_detector::{ColumnDetector, ColumnBoundary, TextBlock};
pub use region_detector::{RegionDetector, DetectedRegion, RegionType, RegionBounds};
pub use table_extractor::{TableExtractor, ExtractedTable};
