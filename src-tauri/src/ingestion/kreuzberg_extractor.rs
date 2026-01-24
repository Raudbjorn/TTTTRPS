//! Kreuzberg Document Extractor
//!
//! Unified document extraction using the kreuzberg crate.
//! Supports PDF, EPUB, DOCX, MOBI, images, and 50+ other formats.

use kreuzberg::core::config::PageConfig;
use kreuzberg::ExtractionConfig;
use std::path::Path;
use thiserror::Error;
use tokio::process::Command;

use super::extraction_settings::{ExtractionSettings, OcrBackend};

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum ExtractionError {
    #[error("Kreuzberg extraction failed: {0}")]
    KreuzbergError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}

impl From<kreuzberg::KreuzbergError> for ExtractionError {
    fn from(e: kreuzberg::KreuzbergError) -> Self {
        ExtractionError::KreuzbergError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ExtractionError>;

// ============================================================================
// Extracted Document Types
// ============================================================================

/// Unified extracted document result
#[derive(Debug, Clone)]
pub struct ExtractedContent {
    /// Source file path
    pub source_path: String,
    /// Extracted text content
    pub content: String,
    /// Number of pages/chapters/sections
    pub page_count: usize,
    /// Document title (if available)
    pub title: Option<String>,
    /// Document author (if available)
    pub author: Option<String>,
    /// MIME type detected
    pub mime_type: String,
    /// Character count
    pub char_count: usize,
    /// Extracted pages (if enabled)
    pub pages: Option<Vec<Page>>,
    /// Detected language (if language detection enabled)
    pub detected_language: Option<String>,
}

/// Content of a single page
#[derive(Debug, Clone)]
pub struct Page {
    /// Page number (1-indexed)
    pub page_number: usize,
    /// Text content of the page
    pub content: String,
}

// ============================================================================
// Document Extractor
// ============================================================================

/// Kreuzberg-based document extractor
pub struct DocumentExtractor {
    config: ExtractionConfig,
    settings: ExtractionSettings,
}

impl Default for DocumentExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentExtractor {
    /// Create a new document extractor with default configuration
    pub fn new() -> Self {
        Self {
            config: ExtractionConfig::default(),
            settings: ExtractionSettings::default(),
        }
    }

    /// Create an extractor with custom settings
    pub fn with_settings(settings: ExtractionSettings) -> Self {
        let config = settings.to_kreuzberg_config_basic();
        Self { config, settings }
    }

    /// Create an extractor with OCR fallback enabled for scanned documents.
    /// Note: OCR is handled via external pdftoppm + tesseract due to
    /// kreuzberg OCR dependency conflicts with other crates.
    pub fn with_ocr() -> Self {
        let mut settings = ExtractionSettings::default();
        settings.ocr_enabled = true;
        settings.ocr_backend = OcrBackend::External;
        Self::with_settings(settings)
    }

    /// Create an extractor with forced OCR (always use external OCR)
    pub fn with_forced_ocr() -> Self {
        let mut settings = ExtractionSettings::default();
        settings.ocr_enabled = true;
        settings.force_ocr = true;
        settings.ocr_backend = OcrBackend::External;
        Self::with_settings(settings)
    }

    /// Create an extractor for text checking only (no OCR fallback).
    /// Use this to quickly check if a PDF has extractable text.
    pub fn text_check_only() -> Self {
        let mut settings = ExtractionSettings::default();
        settings.ocr_enabled = false;
        settings.ocr_backend = OcrBackend::Disabled;
        Self::with_settings(settings)
    }

    /// Create an extractor optimized for TTRPG rulebooks
    pub fn for_rulebooks() -> Self {
        Self::with_settings(ExtractionSettings::for_rulebooks())
    }

    /// Create an extractor optimized for scanned documents
    pub fn for_scanned_documents() -> Self {
        Self::with_settings(ExtractionSettings::for_scanned_documents())
    }

    /// Get the current extraction settings
    pub fn settings(&self) -> &ExtractionSettings {
        &self.settings
    }

    /// Extract a range of pages from a PDF using qpdf
    async fn extract_pdf_page_range(
        &self,
        source: &Path,
        start_page: usize,
        end_page: usize,
        temp_dir: &Path,
    ) -> Result<std::path::PathBuf> {
        let chunk_file = temp_dir.join(format!("chunk_{}_{}.pdf", start_page, end_page));

        // Use qpdf to extract page range
        let status = Command::new("qpdf")
            .arg(source)
            .arg("--pages")
            .arg(".")
            .arg(format!("{}-{}", start_page, end_page))
            .arg("--")
            .arg(&chunk_file)
            .status()
            .await
            .map_err(ExtractionError::IoError)?;

        if !status.success() {
            return Err(ExtractionError::KreuzbergError(format!(
                "qpdf failed to extract pages {}-{}",
                start_page, end_page
            )));
        }

        Ok(chunk_file)
    }

    /// Extract large PDF in chunks to avoid memory pressure
    async fn extract_large_pdf<F>(
        &self,
        path: &Path,
        total_pages: usize,
        progress_callback: Option<F>,
    ) -> Result<ExtractedContent>
    where
        F: Fn(f32, &str) + Send + Sync + 'static,
    {
        let path_str = path.to_string_lossy().to_string();
        let chunk_size = self.settings.large_pdf_chunk_size;
        let num_chunks = total_pages.div_ceil(chunk_size);

        log::info!(
            "Large PDF detected ({} pages), extracting in {} chunks of {} pages",
            total_pages, num_chunks, chunk_size
        );

        let temp_dir = tempfile::Builder::new()
            .prefix("pdf_chunks_")
            .tempdir()
            .map_err(ExtractionError::IoError)?;

        let mut all_pages: Vec<Page> = Vec::with_capacity(total_pages);
        let mut full_content = String::new();
        let mut title: Option<String> = None;
        let mut author: Option<String> = None;
        let mut detected_language: Option<String> = None;

        // Configure kreuzberg for chunk extraction
        let mut config = self.config.clone();
        config.pages = Some(PageConfig {
            extract_pages: true,
            insert_page_markers: false,
            marker_format: "".to_string(),
        });

        for chunk_idx in 0..num_chunks {
            let start_page = chunk_idx * chunk_size + 1;
            let end_page = ((chunk_idx + 1) * chunk_size).min(total_pages);

            if let Some(ref cb) = progress_callback {
                let progress = (chunk_idx as f32 / num_chunks as f32) * 0.9;
                cb(progress, &format!("Extracting pages {}-{}/{}", start_page, end_page, total_pages));
            }

            // Extract chunk to temp file
            let chunk_path = self.extract_pdf_page_range(
                path,
                start_page,
                end_page,
                temp_dir.path(),
            ).await?;

            // Extract text from chunk
            let result = kreuzberg::extract_file(&chunk_path, None, &config).await?;

            // Capture metadata from first chunk
            if chunk_idx == 0 {
                title = result.metadata.title.clone();
                author = result.metadata.authors
                    .as_ref()
                    .map(|authors| authors.join(", "));
                detected_language = result.metadata.language.clone();
            }

            // Add content
            full_content.push_str(&result.content);
            full_content.push('\n');

            // Add pages with corrected page numbers
            if let Some(pages) = result.pages {
                for (idx, p) in pages.into_iter().enumerate() {
                    all_pages.push(Page {
                        page_number: start_page + idx,
                        content: p.content,
                    });
                }
            }

            // Clean up chunk file
            let _ = tokio::fs::remove_file(&chunk_path).await;
        }

        if let Some(ref cb) = progress_callback {
            cb(0.95, "Finalizing extraction...");
        }

        let char_count = full_content.len();

        log::info!(
            "Large PDF extraction complete: {} pages, {} chars",
            all_pages.len(), char_count
        );

        Ok(ExtractedContent {
            source_path: path_str,
            content: full_content,
            page_count: all_pages.len(),
            title,
            author,
            mime_type: "application/pdf".to_string(),
            char_count,
            pages: Some(all_pages),
            detected_language,
        })
    }

    /// Extract content from a file (async)
    pub async fn extract<F>(&self, path: &Path, progress_callback: Option<F>) -> Result<ExtractedContent>
    where F: Fn(f32, &str) + Send + Sync + 'static
    {
        let path_str = path.to_string_lossy().to_string();

        log::info!("Extracting document with kreuzberg (async): {:?}", path.file_name().unwrap_or_default());

        if let Some(ref cb) = progress_callback {
            cb(0.0, &format!("Starting extraction for {:?}", path.file_name().unwrap_or_default()));
        }

        // Check for large PDF - use chunked extraction to avoid memory pressure
        let is_pdf_file = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("pdf"))
            .unwrap_or(false);

        if is_pdf_file {
            // Get page count to check if we should use chunked extraction
            if let Ok(page_count) = self.get_pdf_page_count(path).await {
                if page_count > self.settings.large_pdf_page_threshold {
                    log::info!(
                        "Large PDF detected ({} pages > {} threshold), using chunked extraction",
                        page_count,
                        self.settings.large_pdf_page_threshold
                    );
                    return self.extract_large_pdf(path, page_count, progress_callback).await;
                }
            }
        }

        // Enable page extraction for granular results
        let mut config = self.config.clone();
        config.pages = Some(PageConfig {
            extract_pages: true,
            insert_page_markers: false, // We'll handle chunking based on page structs
            marker_format: "".to_string(),
        });

        // Use async extraction
        let result = kreuzberg::extract_file(path, None, &config).await?;

        // Page count from pages Vec length, or default to 1
        let page_count = result.pages
            .as_ref()
            .map(|p| p.len())
            .unwrap_or(1);

        let title = result.metadata.title.clone();
        // Authors is a Vec, join them if present
        let author = result.metadata.authors
            .as_ref()
            .map(|authors| authors.join(", "));
        let mime_type = result.mime_type.clone();
        let char_count = result.content.len();

        log::info!(
            "Kreuzberg extraction complete: {} pages, {} chars, mime={}",
            page_count, char_count, mime_type
        );

        if let Some(ref cb) = progress_callback {
            cb(0.1, &format!("Parsing complete ({} chars detected)", char_count));
        }

        // Map internal PageContent to our needs if necessary, but ExtractedContent just holds the raw text currently.
        // We really want to expose the pages up the chain.
        // For now, let's keep the struct signatures compatible but populate the content rich fields.

        // Check for fallback OCR
        // If we have minimal text for a PDF, and OCR is enabled, try our manual fallback
        let is_pdf = mime_type == "application/pdf";
        let low_text = char_count < self.settings.ocr_min_text_threshold;
        let ocr_enabled = self.settings.ocr_enabled && self.settings.ocr_backend != OcrBackend::Disabled;
        let should_ocr = self.settings.force_ocr || (is_pdf && low_text && ocr_enabled);

        if should_ocr {
            log::warn!("Minimal text found ({}) in PDF. Attempting async fallback OCR...", char_count);

            if let Some(ref cb) = progress_callback {
                cb(0.15, "Low text detected - Starting OCR fallback...");
            }

            // We pass the current result in case OCR fails or returns nothing
            let fallback_result = self.extract_with_fallback_ocr(path, page_count, progress_callback).await;

            match fallback_result {
                Ok(ocr_content) => {
                    if ocr_content.char_count > char_count {
                        log::info!("Fallback OCR successful: {} chars (was {})", ocr_content.char_count, char_count);
                        return Ok(ocr_content);
                    } else {
                        log::warn!("Fallback OCR produced less/same text. Keeping original.");
                    }
                }
                Err(e) => {
                    log::error!("Fallback OCR failed: {}. Keeping original kreuzberg result.", e);
                }
            }
        }

        // Extract detected language from metadata if available
        let detected_language = result.metadata.language.clone();

        Ok(ExtractedContent {
            source_path: path_str,
            content: result.content,
            page_count,
            title,
            author,
            mime_type,
            char_count,
            pages: result.pages.map(|pages| pages.into_iter().map(|p| Page {
                page_number: p.page_number,
                content: p.content,
            }).collect()),
            detected_language,
        })
    }

    /// Fallback OCR using pdftoppm + tesseract (async)
    async fn extract_with_fallback_ocr<F>(&self, path: &Path, _expected_pages: usize, progress_callback: Option<F>) -> Result<ExtractedContent>
    where F: Fn(f32, &str) + Send + Sync + 'static
    {
        let temp_dir = tempfile::Builder::new()
            .prefix("ocr_")
            .tempdir()
            .map_err(ExtractionError::IoError)?;

        let temp_path = temp_dir.path();
        let prefix = "page";

        // 1. Convert PDF to images (pdftoppm)
        // pdftoppm -png -r 300 input.pdf output_prefix
        log::info!("Running pdftoppm (async) on {:?}", path);
        if let Some(ref cb) = progress_callback {
            cb(0.15, "Converting PDF to images for OCR...");
        }

        let status = Command::new("pdftoppm")
            .arg("-png")
            .arg("-r")
            .arg("300")
            .arg(path)
            .arg(temp_path.join(prefix))
            .status()
            .await
            .map_err(ExtractionError::IoError)?;

        if !status.success() {
            return Err(ExtractionError::KreuzbergError("pdftoppm failed".to_string()));
        }

        // 2. Perform OCR on each image
        let mut pages = Vec::new();
        let mut full_text = String::new();

        // pdftoppm generates page-1.png, page-2.png, etc.
        // We'll iterate up to expected_pages (or find files)
        // Using expected_pages is safer for ordering

        // Sometimes page count is wrong from kreuzberg, so we should allow some flexibility?
        // But pdftoppm is reliable.
        // Let's iterate 1..=expected_pages + check for existence.
        // If expected_pages was 1 (default), checking 1.. matches.

        // Actually best to read the directory to discover ACTUAL pages generated by pdftoppm
        let mut image_files = Vec::new();
        let mut read_dir = tokio::fs::read_dir(temp_path).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("png") {
                // Parse page number from filename: page-1.png -> 1
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                     if let Some(idx) = stem.rfind('-') {
                         if let Ok(num) = stem[idx+1..].parse::<usize>() {
                             image_files.push((num, path));
                         }
                     }
                }
            }
        }

        // Sort by page number
        image_files.sort_by_key(|k| k.0);

        if image_files.is_empty() {
             return Err(ExtractionError::KreuzbergError("No images generated by pdftoppm".to_string()));
        }

        log::info!("OCR processing {} pages...", image_files.len());
        let total_pages = image_files.len();

        for (i, (page_num, img_path)) in image_files.into_iter().enumerate() {
            // Update progress: 0.2 to 0.8 range for OCR
            if let Some(ref cb) = progress_callback {
                let p = 0.2 + ((i as f32 / total_pages as f32) * 0.6);
                cb(p, &format!("OCR processing page {}/{}", page_num, total_pages));
            }

            // Run tesseract with configured language
            let output = Command::new("tesseract")
                .arg(&img_path)
                .arg("stdout")
                .arg("-l")
                .arg(&self.settings.ocr_language)
                .output()
                .await
                .map_err(ExtractionError::IoError)?;

            if !output.status.success() {
                 log::warn!("Tesseract failed for page {}", page_num);
                 continue;
            }

            let page_text = String::from_utf8_lossy(&output.stdout).to_string();

            // Clean up text slightly (remove form feed?)
            let cleaned_text = page_text.replace('\x0c', "");

            full_text.push_str(&cleaned_text);
            full_text.push('\n'); // Add newline between pages

            pages.push(Page {
                page_number: page_num,
                content: cleaned_text,
            });

            // Optional: emit finer logging
            if page_num % 10 == 0 {
                log::debug!("OCR processed page {}", page_num);
            }
        }

        Ok(ExtractedContent {
             source_path: path.to_string_lossy().to_string(),
             content: full_text.clone(),
             page_count: pages.len(),
             title: None, // Lost metadata during OCR
             author: None,
             mime_type: "application/pdf".to_string(),
             char_count: full_text.len(),
             pages: Some(pages),
             detected_language: Some(self.settings.ocr_language.clone()), // OCR language used
        })
    }

    /// Incremental OCR extraction with per-page callback for resumable ingestion.
    ///
    /// This method extracts pages one at a time, calling the provided callback
    /// after each page is OCR'd. This allows the caller to persist each page
    /// immediately, enabling resume from the last successfully persisted page.
    ///
    /// # Arguments
    /// * `path` - Path to the PDF file
    /// * `start_page` - 1-indexed page number to start from (for resuming)
    /// * `total_pages` - Total expected pages (0 to auto-detect)
    /// * `on_page` - Callback called after each page: (page_number, content) -> Result
    ///
    /// # Returns
    /// Number of pages successfully processed
    pub async fn extract_pages_incrementally<F>(
        &self,
        path: &Path,
        start_page: usize,
        total_pages: usize,
        mut on_page: F,
    ) -> Result<usize>
    where
        F: FnMut(usize, String) -> std::result::Result<(), String> + Send,
    {
        let temp_dir = tempfile::Builder::new()
            .prefix("ocr_incr_")
            .tempdir()
            .map_err(ExtractionError::IoError)?;

        let temp_path = temp_dir.path();

        // If total_pages is 0, we need to get page count first
        let actual_total = if total_pages == 0 {
            self.get_pdf_page_count(path).await.unwrap_or(1)
        } else {
            total_pages
        };

        if start_page > actual_total {
            log::info!("Start page {} > total pages {}, nothing to extract", start_page, actual_total);
            return Ok(0);
        }

        log::info!(
            "Incremental OCR: pages {}-{} of {} from {:?}",
            start_page, actual_total, actual_total, path.file_name().unwrap_or_default()
        );

        let mut pages_processed = 0;

        // Process pages in small batches to balance efficiency vs. resumability
        // Batch size of 10 pages means we lose at most ~10 pages of work on interrupt
        const BATCH_SIZE: usize = 10;

        let mut current_page = start_page;
        while current_page <= actual_total {
            let batch_end = (current_page + BATCH_SIZE - 1).min(actual_total);

            log::info!("OCR batch: pages {}-{}", current_page, batch_end);

            // Convert this batch of pages to images
            let prefix = format!("batch_{}", current_page);
            let status = Command::new("pdftoppm")
                .arg("-png")
                .arg("-r")
                .arg("300")
                .arg("-f")
                .arg(current_page.to_string())
                .arg("-l")
                .arg(batch_end.to_string())
                .arg(path)
                .arg(temp_path.join(&prefix))
                .status()
                .await
                .map_err(ExtractionError::IoError)?;

            if !status.success() {
                log::error!("pdftoppm failed for pages {}-{}", current_page, batch_end);
                return Err(ExtractionError::KreuzbergError(
                    format!("pdftoppm failed for pages {}-{}", current_page, batch_end)
                ));
            }

            // Find and sort the generated images
            let mut image_files = Vec::new();
            let mut read_dir = tokio::fs::read_dir(temp_path).await?;

            while let Some(entry) = read_dir.next_entry().await? {
                let img_path = entry.path();
                if img_path.extension().and_then(|e| e.to_str()) == Some("png") {
                    if let Some(stem) = img_path.file_stem().and_then(|s| s.to_str()) {
                        // Match our batch prefix
                        if stem.starts_with(&prefix) {
                            if let Some(idx) = stem.rfind('-') {
                                if let Ok(num) = stem[idx+1..].parse::<usize>() {
                                    image_files.push((num, img_path));
                                }
                            }
                        }
                    }
                }
            }

            image_files.sort_by_key(|k| k.0);

            // OCR each image and call the callback
            for (page_num, img_path) in image_files {
                let output = Command::new("tesseract")
                    .arg(&img_path)
                    .arg("stdout")
                    .arg("-l")
                    .arg(&self.settings.ocr_language)
                    .output()
                    .await
                    .map_err(ExtractionError::IoError)?;

                if !output.status.success() {
                    log::warn!("Tesseract failed for page {}, skipping", page_num);
                    continue;
                }

                let page_text = String::from_utf8_lossy(&output.stdout).to_string();
                let cleaned_text = page_text.replace('\x0c', ""); // Remove form feed

                // Call the callback to persist this page
                on_page(page_num, cleaned_text)
                    .map_err(|e| ExtractionError::KreuzbergError(
                        format!("Failed to persist page {}: {}", page_num, e)
                    ))?;

                pages_processed += 1;

                if page_num % 10 == 0 {
                    log::info!("OCR progress: {}/{} pages", page_num, actual_total);
                }

                // Clean up the image file to save disk space
                let _ = tokio::fs::remove_file(&img_path).await;
            }

            current_page = batch_end + 1;
        }

        log::info!("Incremental OCR complete: {} pages processed", pages_processed);
        Ok(pages_processed)
    }

    /// Extract a single wave of pages concurrently (for resumable extraction).
    ///
    /// Processes `concurrency` batches in parallel, returns sorted pages.
    /// Caller should write to storage, then call again with updated start_page.
    ///
    /// # Returns
    /// (pages_extracted, next_start_page) - pages sorted by number, and where to resume
    pub async fn extract_one_wave(
        &self,
        path: &Path,
        start_page: usize,
        total_pages: usize,
        concurrency: usize,
    ) -> Result<(Vec<(usize, String)>, usize)> {
        const BATCH_SIZE: usize = 10;
        let concurrency = concurrency.max(1).min(8);

        if start_page > total_pages {
            return Ok((Vec::new(), start_page));
        }

        let mut tasks = Vec::new();

        // Launch concurrent batch tasks for this wave
        for batch_idx in 0..concurrency {
            let batch_start = start_page + (batch_idx * BATCH_SIZE);
            if batch_start > total_pages {
                break;
            }
            let batch_end = (batch_start + BATCH_SIZE - 1).min(total_pages);

            let path_clone = path.to_path_buf();
            let ocr_language = self.settings.ocr_language.clone();

            let task = tokio::spawn(async move {
                Self::process_batch_static(&path_clone, batch_start, batch_end, &ocr_language).await
            });

            tasks.push((batch_start, batch_end, task));
        }

        if tasks.is_empty() {
            return Ok((Vec::new(), start_page));
        }

        let batch_count = tasks.len();
        log::info!("OCR wave: {} concurrent batches starting at page {}", batch_count, start_page);

        // Collect results
        let mut wave_pages: Vec<(usize, String)> = Vec::new();

        for (batch_start, batch_end, task) in tasks {
            match task.await {
                Ok(Ok(pages)) => {
                    wave_pages.extend(pages);
                }
                Ok(Err(e)) => {
                    log::error!("Batch {}-{} failed: {}", batch_start, batch_end, e);
                }
                Err(e) => {
                    log::error!("Batch {}-{} task panicked: {}", batch_start, batch_end, e);
                }
            }
        }

        // Sort by page number
        wave_pages.sort_by_key(|(num, _)| *num);

        let next_start = start_page + (batch_count * BATCH_SIZE);
        Ok((wave_pages, next_start))
    }

    /// Static helper to process a single batch (used by concurrent extraction)
    async fn process_batch_static(
        path: &Path,
        batch_start: usize,
        batch_end: usize,
        ocr_language: &str,
    ) -> Result<Vec<(usize, String)>> {
        let temp_dir = tempfile::Builder::new()
            .prefix(&format!("ocr_batch_{}_", batch_start))
            .tempdir()
            .map_err(ExtractionError::IoError)?;

        let temp_path = temp_dir.path();
        let prefix = format!("p{}", batch_start);

        // Convert batch to images
        let status = Command::new("pdftoppm")
            .arg("-png")
            .arg("-r")
            .arg("300")
            .arg("-f")
            .arg(batch_start.to_string())
            .arg("-l")
            .arg(batch_end.to_string())
            .arg(path)
            .arg(temp_path.join(&prefix))
            .status()
            .await
            .map_err(ExtractionError::IoError)?;

        if !status.success() {
            return Err(ExtractionError::KreuzbergError(
                format!("pdftoppm failed for pages {}-{}", batch_start, batch_end)
            ));
        }

        // Find and sort images
        let mut image_files = Vec::new();
        let mut read_dir = tokio::fs::read_dir(temp_path).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let img_path = entry.path();
            if img_path.extension().and_then(|e| e.to_str()) == Some("png") {
                if let Some(stem) = img_path.file_stem().and_then(|s| s.to_str()) {
                    if stem.starts_with(&prefix) {
                        if let Some(idx) = stem.rfind('-') {
                            if let Ok(num) = stem[idx+1..].parse::<usize>() {
                                image_files.push((num, img_path));
                            }
                        }
                    }
                }
            }
        }

        image_files.sort_by_key(|k| k.0);

        // OCR each image
        let mut results = Vec::new();
        for (page_num, img_path) in image_files {
            let output = Command::new("tesseract")
                .arg(&img_path)
                .arg("stdout")
                .arg("-l")
                .arg(ocr_language)
                .output()
                .await
                .map_err(ExtractionError::IoError)?;

            if output.status.success() {
                let page_text = String::from_utf8_lossy(&output.stdout).to_string();
                let cleaned_text = page_text.replace('\x0c', "");
                results.push((page_num, cleaned_text));
            } else {
                log::warn!("Tesseract failed for page {}", page_num);
            }
        }

        // temp_dir is cleaned up automatically when dropped
        Ok(results)
    }

    /// Get PDF page count using pdfinfo
    async fn get_pdf_page_count(&self, path: &Path) -> Result<usize> {
        let output = Command::new("pdfinfo")
            .arg(path)
            .output()
            .await
            .map_err(ExtractionError::IoError)?;

        if !output.status.success() {
            return Err(ExtractionError::KreuzbergError("pdfinfo failed".to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.starts_with("Pages:") {
                if let Some(count_str) = line.split_whitespace().nth(1) {
                    if let Ok(count) = count_str.parse::<usize>() {
                        return Ok(count);
                    }
                }
            }
        }

        Err(ExtractionError::KreuzbergError("Could not parse page count from pdfinfo".to_string()))
    }

    /// Extract content from bytes with a specified MIME type (async)
    pub async fn extract_bytes<F>(&self, bytes: &[u8], mime_type: &str, _progress_callback: Option<F>) -> Result<ExtractedContent>
    where F: Fn(f32, &str) + Send + Sync + 'static
    {
        log::info!("Extracting from bytes with kreuzberg (async), mime={}", mime_type);

        let mut config = self.config.clone();
        config.pages = Some(PageConfig {
            extract_pages: true,
            insert_page_markers: false,
            marker_format: "".to_string(),
        });

        let result = kreuzberg::extract_bytes(bytes, mime_type, &config).await?;

        // Page count from pages Vec length, or default to 1
        let page_count = result.pages
            .as_ref()
            .map(|p| p.len())
            .unwrap_or(1);

        let title = result.metadata.title.clone();
        // Authors is a Vec, join them if present
        let author = result.metadata.authors
            .as_ref()
            .map(|authors| authors.join(", "));
        let detected_mime = result.mime_type.clone();
        let char_count = result.content.len();

        log::info!(
            "Kreuzberg extraction complete: {} pages, {} chars",
            page_count, char_count
        );

        let detected_language = result.metadata.language.clone();

        Ok(ExtractedContent {
            source_path: String::new(),
            content: result.content,
            page_count,
            title,
            author,
            mime_type: detected_mime,
            char_count,
            pages: result.pages.map(|pages| pages.into_iter().map(|p| Page {
                page_number: p.page_number,
                content: p.content,
            }).collect()),
            detected_language,
        })
    }

    /// Check if a file format is supported
    pub fn is_supported(path: &Path) -> bool {
        let extension = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        matches!(extension.as_str(),
            // Documents
            "pdf" | "doc" | "docx" | "odt" | "rtf" |
            // Ebooks
            "epub" | "mobi" | "azw" | "azw3" | "fb2" |
            // Spreadsheets
            "xls" | "xlsx" | "ods" | "csv" |
            // Presentations
            "ppt" | "pptx" | "odp" |
            // Text
            "txt" | "md" | "markdown" | "rst" | "adoc" |
            // Web
            "html" | "htm" | "xml" | "json" | "yaml" | "yml" |
            // Images (for OCR)
            "png" | "jpg" | "jpeg" | "tiff" | "tif" | "bmp" | "gif" | "webp" |
            // Email
            "eml" | "msg"
        )
    }

    /// Get supported file extensions
    pub fn supported_extensions() -> &'static [&'static str] {
        &[
            "pdf", "doc", "docx", "odt", "rtf",
            "epub", "mobi", "azw", "azw3", "fb2",
            "xls", "xlsx", "ods", "csv",
            "ppt", "pptx", "odp",
            "txt", "md", "markdown", "rst", "adoc",
            "html", "htm", "xml", "json", "yaml", "yml",
            "png", "jpg", "jpeg", "tiff", "tif", "bmp", "gif", "webp",
            "eml", "msg",
        ]
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Extract text from a file using default configuration
pub async fn extract_text(path: &Path) -> Result<String> {
    let extractor = DocumentExtractor::new();
    // No callback
    let cb: Option<fn(f32, &str)> = None;
    Ok(extractor.extract(path, cb).await?.content)
}

/// Extract text from a file with OCR fallback
pub async fn extract_text_with_ocr(path: &Path) -> Result<String> {
    let extractor = DocumentExtractor::with_ocr();
    let cb: Option<fn(f32, &str)> = None;
    Ok(extractor.extract(path, cb).await?.content)
}

/// Extract structured content from a file
pub async fn extract_document(path: &Path) -> Result<ExtractedContent> {
    let extractor = DocumentExtractor::new();
    let cb: Option<fn(f32, &str)> = None;
    extractor.extract(path, cb).await
}

/// Extract structured content with OCR fallback
pub async fn extract_document_with_ocr(path: &Path) -> Result<ExtractedContent> {
    let extractor = DocumentExtractor::with_ocr();
    let cb: Option<fn(f32, &str)> = None;
    extractor.extract(path, cb).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_extensions() {
        assert!(DocumentExtractor::is_supported(Path::new("test.pdf")));
        assert!(DocumentExtractor::is_supported(Path::new("test.epub")));
        assert!(DocumentExtractor::is_supported(Path::new("test.docx")));
        assert!(DocumentExtractor::is_supported(Path::new("test.mobi")));
        assert!(DocumentExtractor::is_supported(Path::new("test.txt")));
        assert!(!DocumentExtractor::is_supported(Path::new("test.exe")));
        assert!(!DocumentExtractor::is_supported(Path::new("test.zip")));
    }

    #[tokio::test]
    #[ignore] // Run with: cargo test test_extract_real_pdf -- --ignored
    async fn test_extract_real_pdf() {
        let pdf_path = Path::new("/home/svnbjrn/Delta-Green-Agents-Handbook.pdf");
        if !pdf_path.exists() {
            println!("Test PDF not found, skipping");
            return;
        }

        // Try without OCR first (for text-based PDFs)
        let extractor = DocumentExtractor::new();
        let cb: Option<fn(f32, &str)> = None;
        let result = extractor.extract(pdf_path, cb).await;

        match result {
            Ok(content) => {
                println!("=== Extraction successful ===");
                println!("Source: {}", content.source_path);
                println!("MIME type: {}", content.mime_type);
                println!("Pages: {}", content.page_count);
                println!("Characters: {}", content.char_count);
                println!("Title: {:?}", content.title);
                println!("Author: {:?}", content.author);

                if content.char_count > 0 {
                    let preview_len = content.content.len().min(3000);
                    println!("\n=== First {} chars ===\n{}", preview_len, &content.content[..preview_len]);
                } else {
                    println!("\n=== NO TEXT EXTRACTED (scanned PDF needs OCR) ===");
                }

                // Only assert page count since scanned PDFs may have no text
                assert!(content.page_count > 0, "Expected at least 1 page");

                if content.char_count < 1000 {
                    println!("\nWARNING: Minimal text extracted. This PDF may be scanned/image-based.");
                    println!("OCR feature is disabled due to dependency conflicts.");
                }
            }
            Err(e) => {
                panic!("Extraction failed: {}", e);
            }
        }
    }
}
