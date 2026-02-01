//! Claude API Document Extractor
//!
//! Uses Claude's vision capabilities to extract text from PDFs and images.
//! This provides higher quality extraction than OCR, especially for complex layouts,
//! handwritten text, or documents with embedded images/diagrams.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::path::Path;
use thiserror::Error;

use super::kreuzberg_extractor::{ExtractedContent, Page};
use crate::oauth::claude::{ClaudeClient, FileTokenStorage, TokenStorage};

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum ClaudeExtractionError {
    #[error("Claude API error: {0}")]
    ApiError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("PDF processing error: {0}")]
    PdfError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, ClaudeExtractionError>;

// ============================================================================
// Claude Document Extractor
// ============================================================================

/// Default model for text extraction (fast and cost-effective)
pub const DEFAULT_EXTRACTION_MODEL: &str = "claude-sonnet-4-20250514";

/// Maximum tokens for extraction response
pub const DEFAULT_MAX_TOKENS: u32 = 8192;

/// System prompt for document text extraction
const EXTRACTION_SYSTEM_PROMPT: &str = r#"You are a document text extraction assistant. Your task is to extract all readable text from the provided document while preserving the document's structure.

Guidelines:
- Extract ALL text visible in the document
- Preserve paragraph breaks and section structure
- Maintain the reading order (top to bottom, left to right for Western documents)
- For multi-column layouts, extract column by column
- Preserve headers, titles, and section headings
- Include table content in a readable format
- Preserve bullet points and numbered lists
- Do NOT add any commentary, analysis, or interpretation
- Do NOT summarize - extract the complete text
- If text is unclear or partially visible, extract what you can and mark uncertain sections with [unclear]
- For handwritten text, do your best to transcribe accurately"#;

/// User prompt template for single PDF extraction
const PDF_EXTRACTION_PROMPT: &str = "Extract all text from this PDF document. Preserve the document structure including headers, paragraphs, lists, and tables.";

/// User prompt template for page-by-page extraction
const PAGE_EXTRACTION_PROMPT: &str = "Extract all text from page {page_num} of {total_pages}. Preserve the document structure including headers, paragraphs, lists, and tables.";

/// Claude-based document extractor configuration
#[derive(Debug, Clone)]
pub struct ClaudeExtractorConfig {
    /// Model to use for extraction
    pub model: String,
    /// Maximum tokens for response
    pub max_tokens: u32,
    /// Whether to extract pages individually (for large PDFs)
    pub extract_pages_individually: bool,
    /// Page size threshold for individual extraction (in bytes)
    pub individual_page_threshold_bytes: usize,
    /// Temperature for extraction (lower = more deterministic)
    pub temperature: f32,
}

impl Default for ClaudeExtractorConfig {
    fn default() -> Self {
        Self {
            model: DEFAULT_EXTRACTION_MODEL.to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
            extract_pages_individually: true,
            // 10MB threshold - larger PDFs are extracted page by page
            individual_page_threshold_bytes: 10 * 1024 * 1024,
            temperature: 0.0, // Deterministic extraction
        }
    }
}

/// Claude-based document extractor
///
/// Uses Claude's vision capabilities to extract text from PDFs with high accuracy.
/// This is particularly useful for:
/// - Scanned documents that don't respond well to OCR
/// - Complex layouts (multi-column, mixed text/images)
/// - Documents with handwritten annotations
/// - PDFs with embedded images containing text
pub struct ClaudeDocumentExtractor<S: TokenStorage> {
    client: ClaudeClient<S>,
    config: ClaudeExtractorConfig,
}

impl ClaudeDocumentExtractor<FileTokenStorage> {
    /// Create a new extractor using the app data file token storage.
    ///
    /// The token is stored in `~/.local/share/ttrpg-assistant/oauth-tokens.json`.
    pub fn new() -> Result<Self> {
        let storage = FileTokenStorage::app_data_path()
            .map_err(|e| ClaudeExtractionError::ConfigError(format!("Failed to create token storage: {}", e)))?;

        let client = ClaudeClient::builder()
            .with_storage(storage)
            .build()
            .map_err(|e| ClaudeExtractionError::ConfigError(format!("Failed to create Claude client: {}", e)))?;

        Ok(Self {
            client,
            config: ClaudeExtractorConfig::default(),
        })
    }

    /// Create an extractor with custom configuration.
    pub fn with_config(config: ClaudeExtractorConfig) -> Result<Self> {
        let storage = FileTokenStorage::default_path()
            .map_err(|e| ClaudeExtractionError::ConfigError(format!("Failed to create token storage: {}", e)))?;

        let client = ClaudeClient::builder()
            .with_storage(storage)
            .build()
            .map_err(|e| ClaudeExtractionError::ConfigError(format!("Failed to create Claude client: {}", e)))?;

        Ok(Self { client, config })
    }
}

impl<S: TokenStorage + 'static> ClaudeDocumentExtractor<S> {
    /// Create an extractor with a custom token storage backend.
    pub fn with_storage(storage: S) -> Result<Self> {
        let client = ClaudeClient::builder()
            .with_storage(storage)
            .build()
            .map_err(|e| ClaudeExtractionError::ConfigError(format!("Failed to create Claude client: {}", e)))?;

        Ok(Self {
            client,
            config: ClaudeExtractorConfig::default(),
        })
    }

    /// Create an extractor with custom storage and configuration.
    pub fn with_storage_and_config(storage: S, config: ClaudeExtractorConfig) -> Result<Self> {
        let client = ClaudeClient::builder()
            .with_storage(storage)
            .build()
            .map_err(|e| ClaudeExtractionError::ConfigError(format!("Failed to create Claude client: {}", e)))?;

        Ok(Self { client, config })
    }

    /// Check if the client is authenticated with Claude API.
    pub async fn is_authenticated(&self) -> Result<bool> {
        self.client
            .is_authenticated()
            .await
            .map_err(|e| ClaudeExtractionError::AuthError(e.to_string()))
    }

    /// Get the configuration.
    pub fn config(&self) -> &ClaudeExtractorConfig {
        &self.config
    }

    /// Extract content from a PDF file.
    ///
    /// # Arguments
    /// * `path` - Path to the PDF file
    /// * `progress_callback` - Optional callback for progress updates
    pub async fn extract<F>(&self, path: &Path, progress_callback: Option<F>) -> Result<ExtractedContent>
    where
        F: Fn(f32, &str) + Send + Sync + 'static,
    {
        let path_str = path.to_string_lossy().to_string();

        // Verify file exists and is a supported format
        if !path.exists() {
            return Err(ClaudeExtractionError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", path_str),
            )));
        }

        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if !Self::is_supported_format(&extension) {
            return Err(ClaudeExtractionError::UnsupportedFormat(format!(
                "Format '{}' is not supported for Claude extraction. Supported: pdf, png, jpg, jpeg, gif, webp",
                extension
            )));
        }

        if let Some(ref cb) = progress_callback {
            cb(0.0, &format!("Starting Claude extraction for {:?}", path.file_name().unwrap_or_default()));
        }

        // Read the file
        let file_bytes = tokio::fs::read(path).await?;
        let file_size = file_bytes.len();

        log::info!(
            "Claude extraction: {:?} ({} bytes)",
            path.file_name().unwrap_or_default(),
            file_size
        );

        if let Some(ref cb) = progress_callback {
            cb(0.1, "File loaded, sending to Claude API...");
        }

        // Determine extraction strategy based on file size
        let should_extract_individually = extension == "pdf"
            && file_size > self.config.individual_page_threshold_bytes
            && self.config.extract_pages_individually;

        let (content, pages) = if should_extract_individually {
            // Large PDF - extract page by page
            // Note: This would require PDF manipulation to split pages
            // For now, we fall back to whole-document extraction
            log::warn!(
                "Large PDF ({} bytes) - page-by-page extraction not yet implemented, using whole document",
                file_size
            );
            self.extract_whole_document(&file_bytes, &extension, progress_callback).await?
        } else {
            // Small enough to extract as a whole
            self.extract_whole_document(&file_bytes, &extension, progress_callback).await?
        };

        let char_count = content.len();
        let page_count = pages.as_ref().map(|p| p.len()).unwrap_or(1);

        log::info!(
            "Claude extraction complete: {} chars, {} pages",
            char_count, page_count
        );

        Ok(ExtractedContent {
            source_path: path_str,
            content,
            page_count,
            title: None, // Claude doesn't extract metadata
            author: None,
            mime_type: Self::extension_to_mime(&extension),
            char_count,
            pages,
            detected_language: None, // Could be detected from content if needed
        })
    }

    /// Extract text from a document as a whole.
    async fn extract_whole_document<F>(
        &self,
        bytes: &[u8],
        _extension: &str,
        progress_callback: Option<F>,
    ) -> Result<(String, Option<Vec<Page>>)>
    where
        F: Fn(f32, &str) + Send + Sync + 'static,
    {
        let base64_data = STANDARD.encode(bytes);

        if let Some(ref cb) = progress_callback {
            cb(0.2, "Sending document to Claude...");
        }

        let response = self.client
            .messages()
            .model(&self.config.model)
            .max_tokens(self.config.max_tokens)
            .temperature(self.config.temperature)
            .system(EXTRACTION_SYSTEM_PROMPT)
            .pdf_message(&base64_data, PDF_EXTRACTION_PROMPT)
            .send()
            .await
            .map_err(|e| ClaudeExtractionError::ApiError(e.to_string()))?;

        if let Some(ref cb) = progress_callback {
            cb(0.9, "Processing response...");
        }

        let text = response.text();

        if let Some(ref cb) = progress_callback {
            cb(1.0, "Extraction complete");
        }

        // For whole-document extraction, we don't have page boundaries
        // Return as a single "page" for compatibility
        let pages = vec![Page {
            page_number: 1,
            content: text.clone(),
        }];

        Ok((text, Some(pages)))
    }

    /// Extract text from bytes with a known MIME type.
    pub async fn extract_bytes<F>(
        &self,
        bytes: &[u8],
        mime_type: &str,
        progress_callback: Option<F>,
    ) -> Result<ExtractedContent>
    where
        F: Fn(f32, &str) + Send + Sync + 'static,
    {
        let extension = Self::mime_to_extension(mime_type);

        if !Self::is_supported_format(&extension) {
            return Err(ClaudeExtractionError::UnsupportedFormat(format!(
                "MIME type '{}' is not supported for Claude extraction",
                mime_type
            )));
        }

        let (content, pages) = self.extract_whole_document(bytes, &extension, progress_callback).await?;

        let char_count = content.len();
        let page_count = pages.as_ref().map(|p| p.len()).unwrap_or(1);

        Ok(ExtractedContent {
            source_path: String::new(),
            content,
            page_count,
            title: None,
            author: None,
            mime_type: mime_type.to_string(),
            char_count,
            pages,
            detected_language: None,
        })
    }

    /// Extract PDF pages in parallel using Claude's vision capabilities.
    ///
    /// This method:
    /// 1. Renders PDF pages to PNG images using pdftoppm
    /// 2. Processes pages concurrently (configurable, default 2)
    /// 3. Sends each page image to Claude for text extraction
    /// 4. Returns ordered results
    ///
    /// # Arguments
    /// * `path` - Path to the PDF file
    /// * `concurrency` - Number of pages to process in parallel (default 2)
    /// * `image_dpi` - DPI for page rendering (default 150)
    /// * `on_progress` - Optional progress callback (current_page, total_pages, message)
    pub async fn extract_pages_parallel<F>(
        &self,
        path: &Path,
        concurrency: usize,
        image_dpi: u32,
        on_progress: Option<F>,
    ) -> Result<ExtractedContent>
    where
        F: Fn(usize, usize, &str) + Send + Sync + Clone + 'static,
    {
        use tokio::process::Command;

        let path_str = path.to_string_lossy().to_string();
        let concurrency = concurrency.max(1).min(8);
        let image_dpi = image_dpi.max(72).min(300);

        // Verify PDF exists
        if !path.exists() {
            return Err(ClaudeExtractionError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", path_str),
            )));
        }

        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if extension != "pdf" {
            return Err(ClaudeExtractionError::UnsupportedFormat(
                "Parallel page extraction only supports PDF files".to_string(),
            ));
        }

        // Get page count using pdfinfo
        let pdfinfo_output = Command::new("pdfinfo")
            .arg(path)
            .output()
            .await
            .map_err(|e| ClaudeExtractionError::PdfError(format!("pdfinfo failed: {}", e)))?;

        if !pdfinfo_output.status.success() {
            return Err(ClaudeExtractionError::PdfError(
                "pdfinfo failed to get page count".to_string(),
            ));
        }

        let pdfinfo_str = String::from_utf8_lossy(&pdfinfo_output.stdout);
        let total_pages = pdfinfo_str
            .lines()
            .find(|line| line.starts_with("Pages:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or_else(|| ClaudeExtractionError::PdfError(
                "Could not parse page count from pdfinfo".to_string(),
            ))?;

        log::info!(
            "Claude parallel extraction: {} pages, concurrency={}, dpi={}",
            total_pages, concurrency, image_dpi
        );

        if let Some(ref cb) = on_progress {
            cb(0, total_pages, &format!("Starting extraction of {} pages...", total_pages));
        }

        // Create temp directory for page images
        let temp_dir = tempfile::Builder::new()
            .prefix("claude_pages_")
            .tempdir()
            .map_err(ClaudeExtractionError::IoError)?;

        // Render all pages to images first (more efficient than per-page rendering)
        let prefix = temp_dir.path().join("page");
        let status = Command::new("pdftoppm")
            .arg("-png")
            .arg("-r")
            .arg(image_dpi.to_string())
            .arg(path)
            .arg(&prefix)
            .status()
            .await
            .map_err(|e| ClaudeExtractionError::PdfError(format!("pdftoppm failed: {}", e)))?;

        if !status.success() {
            return Err(ClaudeExtractionError::PdfError(
                "pdftoppm failed to render pages".to_string(),
            ));
        }

        // Collect page image files
        let mut page_files: Vec<(usize, std::path::PathBuf)> = Vec::new();
        let mut read_dir = tokio::fs::read_dir(temp_dir.path()).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let img_path = entry.path();
            if img_path.extension().and_then(|e| e.to_str()) == Some("png") {
                if let Some(stem) = img_path.file_stem().and_then(|s| s.to_str()) {
                    if let Some(idx) = stem.rfind('-') {
                        if let Ok(num) = stem[idx + 1..].parse::<usize>() {
                            page_files.push((num, img_path));
                        }
                    }
                }
            }
        }

        page_files.sort_by_key(|(num, _)| *num);

        // Validate page count matches pdfinfo report
        if total_pages != page_files.len() {
            log::warn!(
                "Page count mismatch: pdfinfo reports {} pages but pdftoppm rendered {} pages",
                total_pages,
                page_files.len()
            );
        }

        if page_files.is_empty() {
            return Err(ClaudeExtractionError::PdfError(
                "No page images generated".to_string(),
            ));
        }

        log::info!("Rendered {} page images, starting parallel Claude extraction", page_files.len());

        // Process pages sequentially with concurrency using semaphore
        // (ClaudeClient doesn't implement Clone, so we process in batches)
        let model = self.config.model.clone();
        let max_tokens = self.config.max_tokens;
        let temperature = self.config.temperature;
        let total = page_files.len();

        let mut pages: Vec<Page> = Vec::new();
        let mut errors: Vec<String> = Vec::new();
        let mut processed = 0;

        // Process in batches of `concurrency` pages
        for chunk in page_files.chunks(concurrency) {
            // Prepare all pages in this batch
            let mut batch_futures = Vec::new();

            for (page_num, img_path) in chunk {
                // Read image
                let img_bytes = tokio::fs::read(&img_path).await
                    .map_err(ClaudeExtractionError::IoError)?;

                let base64_data = STANDARD.encode(&img_bytes);
                let prompt = PAGE_EXTRACTION_PROMPT
                    .replace("{page_num}", &page_num.to_string())
                    .replace("{total_pages}", &total.to_string());

                batch_futures.push((*page_num, base64_data, prompt));
            }

            // Process batch concurrently by sending all requests
            let mut handles = Vec::new();
            for (page_num, base64_data, prompt) in batch_futures {
                let model = model.clone();

                // Send to Claude
                let response_future = self.client
                    .messages()
                    .model(&model)
                    .max_tokens(max_tokens)
                    .temperature(temperature)
                    .system(EXTRACTION_SYSTEM_PROMPT)
                    .image_message(&base64_data, "image/png", &prompt)
                    .send();

                handles.push((page_num, response_future));
            }

            // Await all responses in this batch
            for (page_num, response_future) in handles {
                match response_future.await {
                    Ok(response) => {
                        let text = response.text();
                        log::debug!("Extracted page {} ({} chars)", page_num, text.len());
                        pages.push(Page { page_number: page_num, content: text });
                    }
                    Err(e) => {
                        log::warn!("Failed to extract page {}: {}", page_num, e);
                        errors.push(format!("Page {}: {}", page_num, e));
                    }
                }
                processed += 1;

                if let Some(ref cb) = on_progress {
                    cb(processed, total, &format!("Extracted page {}/{}", processed, total));
                }
            }
        }

        if pages.is_empty() && !errors.is_empty() {
            return Err(ClaudeExtractionError::ApiError(
                format!("All page extractions failed: {}", errors.join("; ")),
            ));
        }

        // Sort by page number
        pages.sort_by_key(|p| p.page_number);

        // Combine content
        let content: String = pages
            .iter()
            .map(|p| p.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");

        let char_count = content.len();
        let page_count = pages.len();

        log::info!(
            "Claude parallel extraction complete: {} pages, {} chars, {} errors",
            page_count, char_count, errors.len()
        );

        if let Some(ref cb) = on_progress {
            cb(page_count, page_count, &format!("Extraction complete: {} pages", page_count));
        }

        Ok(ExtractedContent {
            source_path: path_str,
            content,
            page_count,
            title: None,
            author: None,
            mime_type: "application/pdf".to_string(),
            char_count,
            pages: Some(pages),
            detected_language: None,
        })
    }

    /// Check if a file format is supported for Claude extraction.
    pub fn is_supported_format(extension: &str) -> bool {
        matches!(
            extension.to_lowercase().as_str(),
            "pdf" | "png" | "jpg" | "jpeg" | "gif" | "webp"
        )
    }

    /// Check if a file path is supported.
    pub fn is_supported(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(Self::is_supported_format)
            .unwrap_or(false)
    }

    /// Get supported file extensions.
    pub fn supported_extensions() -> &'static [&'static str] {
        &["pdf", "png", "jpg", "jpeg", "gif", "webp"]
    }

    /// Convert file extension to MIME type.
    fn extension_to_mime(extension: &str) -> String {
        match extension.to_lowercase().as_str() {
            "pdf" => "application/pdf".to_string(),
            "png" => "image/png".to_string(),
            "jpg" | "jpeg" => "image/jpeg".to_string(),
            "gif" => "image/gif".to_string(),
            "webp" => "image/webp".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    }

    /// Convert MIME type to file extension.
    fn mime_to_extension(mime_type: &str) -> String {
        match mime_type {
            "application/pdf" => "pdf".to_string(),
            "image/png" => "png".to_string(),
            "image/jpeg" => "jpg".to_string(),
            "image/gif" => "gif".to_string(),
            "image/webp" => "webp".to_string(),
            _ => "bin".to_string(),
        }
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Extract text from a PDF using Claude API.
///
/// Uses the app data file token storage (`~/.local/share/ttrpg-assistant/oauth-tokens.json`).
pub async fn extract_with_claude(path: &Path) -> Result<String> {
    let extractor = ClaudeDocumentExtractor::new()?;
    let cb: Option<fn(f32, &str)> = None;
    Ok(extractor.extract(path, cb).await?.content)
}

/// Extract structured content from a PDF using Claude API.
pub async fn extract_document_with_claude(path: &Path) -> Result<ExtractedContent> {
    let extractor = ClaudeDocumentExtractor::new()?;
    let cb: Option<fn(f32, &str)> = None;
    extractor.extract(path, cb).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_formats() {
        assert!(ClaudeDocumentExtractor::<FileTokenStorage>::is_supported_format("pdf"));
        assert!(ClaudeDocumentExtractor::<FileTokenStorage>::is_supported_format("PNG"));
        assert!(ClaudeDocumentExtractor::<FileTokenStorage>::is_supported_format("jpg"));
        assert!(ClaudeDocumentExtractor::<FileTokenStorage>::is_supported_format("jpeg"));
        assert!(!ClaudeDocumentExtractor::<FileTokenStorage>::is_supported_format("docx"));
        assert!(!ClaudeDocumentExtractor::<FileTokenStorage>::is_supported_format("txt"));
    }

    #[test]
    fn test_extension_to_mime() {
        assert_eq!(
            ClaudeDocumentExtractor::<FileTokenStorage>::extension_to_mime("pdf"),
            "application/pdf"
        );
        assert_eq!(
            ClaudeDocumentExtractor::<FileTokenStorage>::extension_to_mime("PNG"),
            "image/png"
        );
    }

    #[test]
    fn test_config_defaults() {
        let config = ClaudeExtractorConfig::default();
        assert_eq!(config.model, DEFAULT_EXTRACTION_MODEL);
        assert_eq!(config.max_tokens, DEFAULT_MAX_TOKENS);
        assert!(config.extract_pages_individually);
        assert_eq!(config.temperature, 0.0);
    }

    #[test]
    fn test_is_supported_path() {
        assert!(ClaudeDocumentExtractor::<FileTokenStorage>::is_supported(Path::new("doc.pdf")));
        assert!(ClaudeDocumentExtractor::<FileTokenStorage>::is_supported(Path::new("image.png")));
        assert!(!ClaudeDocumentExtractor::<FileTokenStorage>::is_supported(Path::new("doc.docx")));
    }
}
