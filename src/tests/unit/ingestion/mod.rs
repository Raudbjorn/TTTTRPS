//! Ingestion Pipeline Unit Tests
//!
//! This module contains unit tests for the document ingestion pipeline components:
//! - EPUB Parser: Chapter extraction, TOC parsing, and CSS stripping
//! - DOCX Parser: Text, table, and heading extraction from Word documents
//! - Semantic Chunker: Text chunking with overlap and boundary detection
//!
//! Note: PDF extraction now uses kreuzberg (see kreuzberg_extractor.rs)

// mod epub_parser_tests; // Deleted
// mod docx_parser_tests; // Deleted
mod chunker_tests;

