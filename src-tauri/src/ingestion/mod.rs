pub mod adaptive_learning;
pub mod pdf_parser;
pub mod epub_parser;
pub mod mobi_parser;
pub mod docx_parser;
pub mod personality;
pub mod flavor;
pub mod character_gen;
pub mod rulebook_linker;
pub mod chunker;

pub use adaptive_learning::AdaptiveLearningSystem;
pub use pdf_parser::{PDFParser, ExtractedDocument, ExtractedPage};
pub use epub_parser::{EPUBParser, ExtractedEPUB, ExtractedChapter};
pub use mobi_parser::{MOBIParser, ExtractedMOBI, ExtractedSection};
pub use docx_parser::{DOCXParser, ExtractedDOCX};
pub use chunker::{SemanticChunker, ChunkConfig, ContentChunk};
