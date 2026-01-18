//! Extraction Settings
//!
//! Configurable settings for document extraction using kreuzberg or Claude.
//! These settings control OCR, chunking, quality processing, and language detection.

use serde::{Deserialize, Serialize};

/// Text extraction provider selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TextExtractionProvider {
    /// Use kreuzberg for fast local extraction (default)
    #[default]
    Kreuzberg,
    /// Use Claude API for extraction (better quality, requires API auth)
    ClaudeGate,
}

impl TextExtractionProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Kreuzberg => "kreuzberg",
            Self::ClaudeGate => "claude_gate",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "claude_gate" | "claudegate" | "claude" => Self::ClaudeGate,
            _ => Self::Kreuzberg,
        }
    }

    /// Human-readable display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Kreuzberg => "Kreuzberg (Local)",
            Self::ClaudeGate => "Claude API",
        }
    }
}

/// Token reduction aggressiveness levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TokenReductionLevel {
    /// No token reduction
    #[default]
    Off,
    /// Light reduction - preserve most content
    Light,
    /// Moderate reduction - balance quality and size
    Moderate,
    /// Aggressive reduction - prioritize size
    Aggressive,
    /// Maximum reduction - minimal output
    Maximum,
}

impl TokenReductionLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Light => "light",
            Self::Moderate => "moderate",
            Self::Aggressive => "aggressive",
            Self::Maximum => "maximum",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "light" => Self::Light,
            "moderate" => Self::Moderate,
            "aggressive" => Self::Aggressive,
            "maximum" => Self::Maximum,
            _ => Self::Off,
        }
    }
}

/// OCR backend selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OcrBackend {
    /// Use external tesseract (pdftoppm + tesseract)
    #[default]
    External,
    /// Use kreuzberg's built-in OCR (if available)
    Builtin,
    /// Disable OCR entirely
    Disabled,
}

impl OcrBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::External => "external",
            Self::Builtin => "builtin",
            Self::Disabled => "disabled",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "builtin" => Self::Builtin,
            "disabled" => Self::Disabled,
            _ => Self::External,
        }
    }
}

/// Document extraction settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionSettings {
    // ========== Provider Settings ==========
    /// Text extraction provider to use
    pub text_extraction_provider: TextExtractionProvider,

    // ========== OCR Settings ==========
    /// Whether to enable OCR fallback for scanned documents
    pub ocr_enabled: bool,
    /// OCR backend to use
    pub ocr_backend: OcrBackend,
    /// Force OCR even if text is detected
    pub force_ocr: bool,
    /// OCR language code (e.g., "eng", "deu", "fra")
    pub ocr_language: String,
    /// Minimum text length before triggering OCR fallback
    pub ocr_min_text_threshold: usize,

    // ========== Chunking Settings ==========
    /// Enable kreuzberg's built-in chunking
    pub chunking_enabled: bool,
    /// Maximum characters per chunk
    pub max_chunk_chars: usize,
    /// Overlap between chunks (for context continuity)
    pub chunk_overlap: usize,

    // ========== Quality Settings ==========
    /// Enable text quality post-processing
    pub quality_processing: bool,
    /// Token reduction level
    pub token_reduction: TokenReductionLevel,

    // ========== Language Detection ==========
    /// Enable automatic language detection
    pub language_detection: bool,

    // ========== Image Extraction ==========
    /// Target DPI for image extraction
    pub image_dpi: u32,
    /// Maximum image dimension (width or height)
    pub max_image_dimension: u32,

    // ========== Caching ==========
    /// Enable extraction result caching
    pub use_cache: bool,
    /// Maximum concurrent extractions
    pub max_concurrent_extractions: usize,

    // ========== Large PDF Handling ==========
    /// Page count threshold above which to use chunked extraction
    /// PDFs larger than this will be extracted in chunks to avoid memory pressure
    pub large_pdf_page_threshold: usize,
    /// Number of pages to extract per chunk for large PDFs
    pub large_pdf_chunk_size: usize,
}

impl Default for ExtractionSettings {
    fn default() -> Self {
        Self {
            // Provider default
            text_extraction_provider: TextExtractionProvider::default(),

            // OCR defaults
            ocr_enabled: true,
            ocr_backend: OcrBackend::External,
            force_ocr: false,
            ocr_language: "eng".to_string(),
            ocr_min_text_threshold: 500,

            // Chunking defaults (optimized for RAG)
            chunking_enabled: false, // We use our own TTRPG-aware chunker
            max_chunk_chars: 1000,
            chunk_overlap: 200,

            // Quality defaults
            quality_processing: true,
            token_reduction: TokenReductionLevel::Off,

            // Language detection
            language_detection: true,

            // Image extraction
            image_dpi: 300,
            max_image_dimension: 4096,

            // Caching
            use_cache: true,
            max_concurrent_extractions: 4,

            // Large PDF handling
            large_pdf_page_threshold: 500,
            large_pdf_chunk_size: 100,
        }
    }
}

impl ExtractionSettings {
    /// Create settings optimized for TTRPG rulebook extraction
    pub fn for_rulebooks() -> Self {
        Self {
            text_extraction_provider: TextExtractionProvider::Kreuzberg,
            ocr_enabled: true,
            ocr_backend: OcrBackend::External,
            force_ocr: false,
            ocr_language: "eng".to_string(),
            ocr_min_text_threshold: 500,
            chunking_enabled: false, // Use TTRPG-aware chunker instead
            max_chunk_chars: 1500,   // Larger chunks for rulebook content
            chunk_overlap: 300,
            quality_processing: true,
            token_reduction: TokenReductionLevel::Light, // Light cleanup
            language_detection: true,
            image_dpi: 300,
            max_image_dimension: 4096,
            use_cache: true,
            max_concurrent_extractions: 4,
            large_pdf_page_threshold: 500,
            large_pdf_chunk_size: 100,
        }
    }

    /// Create settings optimized for scanned documents
    pub fn for_scanned_documents() -> Self {
        Self {
            text_extraction_provider: TextExtractionProvider::Kreuzberg,
            ocr_enabled: true,
            ocr_backend: OcrBackend::External,
            force_ocr: true, // Always use OCR for scanned docs
            ocr_language: "eng".to_string(),
            ocr_min_text_threshold: 0,
            chunking_enabled: false,
            max_chunk_chars: 1000,
            chunk_overlap: 200,
            quality_processing: true,
            token_reduction: TokenReductionLevel::Moderate, // Clean up OCR artifacts
            language_detection: true,
            image_dpi: 300,
            max_image_dimension: 4096,
            use_cache: true,
            max_concurrent_extractions: 2, // OCR is CPU intensive
            large_pdf_page_threshold: 500,
            large_pdf_chunk_size: 100,
        }
    }

    /// Create settings for quick extraction (minimal processing)
    pub fn quick() -> Self {
        Self {
            text_extraction_provider: TextExtractionProvider::Kreuzberg,
            ocr_enabled: false,
            ocr_backend: OcrBackend::Disabled,
            force_ocr: false,
            ocr_language: "eng".to_string(),
            ocr_min_text_threshold: 500,
            chunking_enabled: false,
            max_chunk_chars: 1000,
            chunk_overlap: 200,
            quality_processing: false,
            token_reduction: TokenReductionLevel::Off,
            language_detection: false,
            image_dpi: 150,
            max_image_dimension: 2048,
            use_cache: true,
            max_concurrent_extractions: 8,
            large_pdf_page_threshold: 500,
            large_pdf_chunk_size: 100,
        }
    }

    /// Create settings for Claude-based extraction (high quality, slower)
    pub fn with_claude() -> Self {
        Self {
            text_extraction_provider: TextExtractionProvider::ClaudeGate,
            ocr_enabled: false, // Claude handles scanned docs natively
            ocr_backend: OcrBackend::Disabled,
            force_ocr: false,
            ocr_language: "eng".to_string(),
            ocr_min_text_threshold: 500,
            chunking_enabled: false,
            max_chunk_chars: 1500,
            chunk_overlap: 300,
            quality_processing: false, // Claude output is already clean
            token_reduction: TokenReductionLevel::Off,
            language_detection: true,
            image_dpi: 300,
            max_image_dimension: 4096,
            use_cache: true,
            max_concurrent_extractions: 2, // Respect API rate limits
            large_pdf_page_threshold: 500,
            large_pdf_chunk_size: 100,
        }
    }

    /// Convert to kreuzberg ExtractionConfig
    #[cfg(feature = "chunking")]
    pub fn to_kreuzberg_config(&self) -> kreuzberg::ExtractionConfig {
        use kreuzberg::core::config::{ChunkingConfig, PageConfig};

        let mut config = kreuzberg::ExtractionConfig::default();
        config.use_cache = self.use_cache;
        config.force_ocr = self.force_ocr;
        config.max_concurrent_extractions = Some(self.max_concurrent_extractions);

        // Page extraction
        config.pages = Some(PageConfig {
            extract_pages: true,
            insert_page_markers: false,
            marker_format: String::new(),
        });

        // Chunking (if enabled via kreuzberg)
        if self.chunking_enabled {
            config.chunking = Some(ChunkingConfig {
                max_chars: self.max_chunk_chars,
                max_overlap: self.chunk_overlap,
                ..Default::default()
            });
        }

        config
    }

    /// Convert to kreuzberg ExtractionConfig (basic version without chunking feature)
    pub fn to_kreuzberg_config_basic(&self) -> kreuzberg::ExtractionConfig {
        use kreuzberg::core::config::PageConfig;

        let mut config = kreuzberg::ExtractionConfig::default();
        config.use_cache = self.use_cache;
        config.force_ocr = self.force_ocr;
        config.max_concurrent_extractions = Some(self.max_concurrent_extractions);

        // Page extraction
        config.pages = Some(PageConfig {
            extract_pages: true,
            insert_page_markers: false,
            marker_format: String::new(),
        });

        config
    }

    /// Validate settings
    pub fn validate(&self) -> Result<(), String> {
        if self.max_chunk_chars < 100 {
            return Err("max_chunk_chars must be at least 100".to_string());
        }
        if self.chunk_overlap >= self.max_chunk_chars {
            return Err("chunk_overlap must be less than max_chunk_chars".to_string());
        }
        if self.image_dpi < 72 || self.image_dpi > 600 {
            return Err("image_dpi must be between 72 and 600".to_string());
        }
        if self.max_image_dimension < 512 || self.max_image_dimension > 8192 {
            return Err("max_image_dimension must be between 512 and 8192".to_string());
        }
        if self.max_concurrent_extractions < 1 || self.max_concurrent_extractions > 32 {
            return Err("max_concurrent_extractions must be between 1 and 32".to_string());
        }
        Ok(())
    }

    /// Get PDF page count synchronously using pdfinfo.
    /// Returns None if the file is not a PDF or pdfinfo fails.
    pub fn get_pdf_page_count_sync(&self, path: &std::path::Path) -> Option<usize> {
        use std::process::Command;

        let output = Command::new("pdfinfo")
            .arg(path)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.starts_with("Pages:") {
                if let Some(count_str) = line.split_whitespace().nth(1) {
                    if let Ok(count) = count_str.parse::<usize>() {
                        return Some(count);
                    }
                }
            }
        }

        None
    }
}

/// Supported file formats for extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedFormats {
    pub documents: Vec<FormatInfo>,
    pub images: Vec<FormatInfo>,
    pub web: Vec<FormatInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    pub extension: String,
    pub description: String,
    pub requires_ocr: bool,
}

impl SupportedFormats {
    pub fn get_all() -> Self {
        Self {
            documents: vec![
                FormatInfo {
                    extension: "pdf".to_string(),
                    description: "PDF documents".to_string(),
                    requires_ocr: false, // May need OCR for scanned PDFs
                },
                FormatInfo {
                    extension: "txt".to_string(),
                    description: "Plain text".to_string(),
                    requires_ocr: false,
                },
                FormatInfo {
                    extension: "md".to_string(),
                    description: "Markdown".to_string(),
                    requires_ocr: false,
                },
                FormatInfo {
                    extension: "rst".to_string(),
                    description: "reStructuredText".to_string(),
                    requires_ocr: false,
                },
                FormatInfo {
                    extension: "epub".to_string(),
                    description: "EPUB ebooks".to_string(),
                    requires_ocr: false,
                },
                // Note: DOCX/XLSX require office feature which has crc conflict
            ],
            images: vec![
                FormatInfo {
                    extension: "png".to_string(),
                    description: "PNG images".to_string(),
                    requires_ocr: true,
                },
                FormatInfo {
                    extension: "jpg".to_string(),
                    description: "JPEG images".to_string(),
                    requires_ocr: true,
                },
                FormatInfo {
                    extension: "tiff".to_string(),
                    description: "TIFF images".to_string(),
                    requires_ocr: true,
                },
                FormatInfo {
                    extension: "webp".to_string(),
                    description: "WebP images".to_string(),
                    requires_ocr: true,
                },
            ],
            web: vec![
                FormatInfo {
                    extension: "html".to_string(),
                    description: "HTML pages".to_string(),
                    requires_ocr: false,
                },
                FormatInfo {
                    extension: "htm".to_string(),
                    description: "HTML pages".to_string(),
                    requires_ocr: false,
                },
            ],
        }
    }

    pub fn all_extensions(&self) -> Vec<&str> {
        let mut exts: Vec<&str> = Vec::new();
        for f in &self.documents {
            exts.push(&f.extension);
        }
        for f in &self.images {
            exts.push(&f.extension);
        }
        for f in &self.web {
            exts.push(&f.extension);
        }
        exts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = ExtractionSettings::default();
        assert!(settings.ocr_enabled);
        assert!(!settings.force_ocr);
        assert_eq!(settings.ocr_language, "eng");
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_validation() {
        let mut settings = ExtractionSettings::default();

        // Invalid chunk size
        settings.max_chunk_chars = 50;
        assert!(settings.validate().is_err());

        // Invalid overlap
        settings.max_chunk_chars = 1000;
        settings.chunk_overlap = 1500;
        assert!(settings.validate().is_err());

        // Valid settings
        settings.chunk_overlap = 200;
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_token_reduction_serde() {
        let level = TokenReductionLevel::Moderate;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"moderate\"");

        let parsed: TokenReductionLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, TokenReductionLevel::Moderate);
    }

    #[test]
    fn test_supported_formats() {
        let formats = SupportedFormats::get_all();
        assert!(!formats.documents.is_empty());
        assert!(!formats.images.is_empty());
        assert!(!formats.web.is_empty());

        let all_exts = formats.all_extensions();
        assert!(all_exts.contains(&"pdf"));
        assert!(all_exts.contains(&"html"));
    }
}
