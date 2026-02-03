use super::core::{invoke, invoke_no_args, invoke_void};
use serde::{Deserialize, Serialize};

// ============================================================================
// Document Ingestion
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    pub page_count: usize,
    pub character_count: usize,
    pub source_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestOptions {
    pub source_type: String,
    pub campaign_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestProgress {
    pub stage: String,
    pub progress: f32,
    pub message: String,
    pub source_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoPhaseIngestResult {
    pub slug: String,
    pub source_name: String,
    pub raw_index: String,
    pub chunks_index: String,
    pub page_count: usize,
    pub chunk_count: usize,
    pub total_chars: usize,
    pub game_system: Option<String>,
    pub content_category: Option<String>,
}

/// Parse PDF and return stats (does NOT index)
pub async fn ingest_pdf(path: String) -> Result<IngestResult, String> {
    #[derive(Serialize)]
    struct Args {
        path: String,
    }
    invoke("ingest_pdf", &Args { path }).await
}

/// Ingest document into Meilisearch (indexes the content)
pub async fn ingest_document(
    path: String,
    options: Option<IngestOptions>,
) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        path: String,
        options: Option<IngestOptions>,
    }
    invoke("ingest_document", &Args { path, options }).await
}

/// Ingest document using two-phase pipeline with per-document indexes.
pub async fn ingest_document_two_phase(
    path: String,
    title_override: Option<String>,
) -> Result<TwoPhaseIngestResult, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        path: String,
        title_override: Option<String>,
    }
    invoke(
        "ingest_document_two_phase",
        &Args {
            path,
            title_override,
        },
    )
    .await
}

// ============================================================================
// Library Metadata
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDocument {
    pub id: String,
    pub name: String,
    pub source_type: String,
    #[serde(default)]
    pub file_path: Option<String>,
    pub page_count: u32,
    pub chunk_count: u32,
    pub character_count: u64,
    pub content_index: String,
    pub status: String,
    #[serde(default)]
    pub error_message: Option<String>,
    pub ingested_at: String,
    // TTRPG metadata (user-editable)
    #[serde(default)]
    pub game_system: Option<String>,
    #[serde(default)]
    pub setting: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub publisher: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLibraryDocumentRequest {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setting: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
}

/// List all documents from the library (persisted in Meilisearch)
pub async fn list_library_documents() -> Result<Vec<LibraryDocument>, String> {
    invoke_no_args("list_library_documents").await
}

/// Delete a document from the library (removes metadata and content chunks)
pub async fn delete_library_document(id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        id: String,
    }
    invoke_void("delete_library_document", &Args { id }).await
}

/// Update a library document's TTRPG metadata
pub async fn update_library_document(
    request: UpdateLibraryDocumentRequest,
) -> Result<LibraryDocument, String> {
    #[derive(Serialize)]
    struct Args {
        request: UpdateLibraryDocumentRequest,
    }
    invoke("update_library_document", &Args { request }).await
}

/// Rebuild library metadata from existing content indices.
pub async fn rebuild_library_metadata() -> Result<usize, String> {
    invoke_no_args("rebuild_library_metadata").await
}

/// Clear a document's content and re-ingest from the original file.
pub async fn clear_and_reingest_document(id: String) -> Result<IngestResult, String> {
    #[derive(Serialize)]
    struct Args {
        id: String,
    }
    invoke("clear_and_reingest_document", &Args { id }).await
}

// ============================================================================
// Extraction Settings
// ============================================================================

/// Token reduction aggressiveness levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TokenReductionLevel {
    #[default]
    Off,
    Light,
    Moderate,
    Aggressive,
    Maximum,
}

/// OCR backend selection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OcrBackend {
    #[default]
    External,
    Builtin,
    Disabled,
}

/// Text extraction provider selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TextExtractionProvider {
    /// Use kreuzberg for fast local extraction (default)
    #[default]
    Kreuzberg,
    /// Use Claude API for extraction (better quality, requires API auth)
    Claude,
}

impl TextExtractionProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Kreuzberg => "kreuzberg",
            Self::Claude => "claude",
        }
    }
}

/// Document extraction settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionSettings {
    // Provider Settings
    pub text_extraction_provider: TextExtractionProvider,
    // OCR Settings
    pub ocr_enabled: bool,
    pub ocr_backend: OcrBackend,
    pub force_ocr: bool,
    pub ocr_language: String,
    pub ocr_min_text_threshold: usize,
    // Chunking Settings
    pub chunking_enabled: bool,
    pub max_chunk_chars: usize,
    pub chunk_overlap: usize,
    // Quality Settings
    pub quality_processing: bool,
    pub token_reduction: TokenReductionLevel,
    // Language Detection
    pub language_detection: bool,
    // Image Extraction
    pub image_dpi: u32,
    pub max_image_dimension: u32,
    // Caching
    pub use_cache: bool,
    pub max_concurrent_extractions: usize,
    // Large PDF Handling
    pub large_pdf_page_threshold: usize,
    pub large_pdf_chunk_size: usize,
}

impl Default for ExtractionSettings {
    fn default() -> Self {
        Self {
            text_extraction_provider: TextExtractionProvider::default(),
            ocr_enabled: true,
            ocr_backend: OcrBackend::External,
            force_ocr: false,
            ocr_language: "eng".to_string(),
            ocr_min_text_threshold: 500,
            chunking_enabled: false,
            max_chunk_chars: 1000,
            chunk_overlap: 200,
            quality_processing: true,
            token_reduction: TokenReductionLevel::Off,
            language_detection: true,
            image_dpi: 300,
            max_image_dimension: 4096,
            use_cache: true,
            max_concurrent_extractions: 4,
            large_pdf_page_threshold: 500,
            large_pdf_chunk_size: 100,
        }
    }
}

/// Extraction preset with name and description
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionPreset {
    pub name: String,
    pub description: String,
    pub settings: ExtractionSettings,
}

/// Supported file format info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    pub extension: String,
    pub description: String,
    pub requires_ocr: bool,
}

/// All supported file formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedFormats {
    pub documents: Vec<FormatInfo>,
    pub images: Vec<FormatInfo>,
    pub web: Vec<FormatInfo>,
}

/// OCR availability status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrAvailability {
    pub tesseract_installed: bool,
    pub pdftoppm_installed: bool,
    pub available_languages: Vec<String>,
    pub external_ocr_ready: bool,
}

/// Get current extraction settings
pub async fn get_extraction_settings() -> Result<ExtractionSettings, String> {
    invoke_no_args("get_extraction_settings").await
}

/// Save extraction settings
pub async fn save_extraction_settings(settings: ExtractionSettings) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        settings: ExtractionSettings,
    }
    invoke_void("save_extraction_settings", &Args { settings }).await
}

/// Get supported file formats for extraction
pub async fn get_supported_formats() -> Result<SupportedFormats, String> {
    invoke_no_args("get_supported_formats").await
}

/// Get extraction settings presets
pub async fn get_extraction_presets() -> Result<Vec<ExtractionPreset>, String> {
    invoke_no_args("get_extraction_presets").await
}

/// Check OCR availability on the system
pub async fn check_ocr_availability() -> Result<OcrAvailability, String> {
    invoke_no_args("check_ocr_availability").await
}
