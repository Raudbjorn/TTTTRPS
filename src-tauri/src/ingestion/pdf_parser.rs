//! PDF Parser Module
//!
//! Extracts text and structure from PDF documents for ingestion.
//! Supports fallback extraction when primary parser fails or produces garbled output.

use std::path::Path;
use lopdf::Document;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum PDFError {
    #[error("Failed to load PDF: {0}")]
    LoadError(String),

    #[error("Failed to extract text: {0}")]
    ExtractionError(String),

    #[error("Page not found: {0}")]
    PageNotFound(u32),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Password required for encrypted PDF")]
    PasswordRequired,

    #[error("Invalid password for PDF")]
    InvalidPassword,

    #[error("Fallback extraction failed: {0}")]
    FallbackFailed(String),
}

pub type Result<T> = std::result::Result<T, PDFError>;

/// Progress callback type for OCR extraction (current_page, total_pages)
pub type OcrProgressCallback = Box<dyn Fn(usize, usize) + Send + Sync>;

// ============================================================================
// Extracted Content Types
// ============================================================================

/// A page of extracted content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedPage {
    /// Page number (1-indexed)
    pub page_number: u32,
    /// Raw text content
    pub text: String,
    /// Detected paragraphs
    pub paragraphs: Vec<String>,
    /// Detected headers (heuristic based on formatting)
    pub headers: Vec<String>,
}

/// Complete extracted document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedDocument {
    /// Source file path
    pub source_path: String,
    /// Number of pages
    pub page_count: usize,
    /// Extracted pages
    pub pages: Vec<ExtractedPage>,
    /// Document metadata
    pub metadata: DocumentMetadata,
}

/// Document metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Vec<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
}

// ============================================================================
// PDF Parser
// ============================================================================

pub struct PDFParser;

impl PDFParser {
    /// Extract text from a PDF file (simple extraction)
    pub fn extract_text(path: &Path) -> Result<String> {
        let doc = Document::load(path)
            .map_err(|e| PDFError::LoadError(e.to_string()))?;

        let mut text = String::new();
        for (page_num, _page_id) in doc.get_pages() {
            let content = doc.extract_text(&[page_num])
                .map_err(|e| PDFError::ExtractionError(e.to_string()))?;
            text.push_str(&content);
            text.push('\n');
        }
        Ok(text)
    }

    /// Extract structured content from a PDF file
    pub fn extract_structured(path: &Path) -> Result<ExtractedDocument> {
        let doc = Document::load(path)
            .map_err(|e| PDFError::LoadError(e.to_string()))?;

        let page_ids: Vec<_> = doc.get_pages().into_iter().collect();
        let page_count = page_ids.len();
        let mut pages = Vec::with_capacity(page_count);

        for (page_num, _page_id) in page_ids {
            let text = doc.extract_text(&[page_num])
                .map_err(|e| PDFError::ExtractionError(e.to_string()))?;

            let (paragraphs, headers) = Self::analyze_text_structure(&text);

            pages.push(ExtractedPage {
                page_number: page_num,
                text,
                paragraphs,
                headers,
            });
        }

        // Extract metadata
        let metadata = Self::extract_metadata(&doc);

        Ok(ExtractedDocument {
            source_path: path.to_string_lossy().to_string(),
            page_count,
            pages,
            metadata,
        })
    }

    /// Analyze text structure to identify paragraphs and headers
    fn analyze_text_structure(text: &str) -> (Vec<String>, Vec<String>) {
        let mut paragraphs = Vec::new();
        let mut headers = Vec::new();

        let lines: Vec<&str> = text.lines().collect();
        let mut current_paragraph = String::new();

        for line in lines {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                // End of paragraph
                if !current_paragraph.is_empty() {
                    paragraphs.push(current_paragraph.trim().to_string());
                    current_paragraph.clear();
                }
                continue;
            }

            // Heuristic header detection:
            // - Short lines (< 100 chars) that are all caps or title case
            // - Lines that don't end with punctuation
            let is_potential_header = trimmed.len() < 100
                && !trimmed.ends_with('.')
                && !trimmed.ends_with(',')
                && !trimmed.ends_with(';')
                && (Self::is_title_case(trimmed) || Self::is_all_caps(trimmed));

            if is_potential_header && current_paragraph.is_empty() {
                headers.push(trimmed.to_string());
            }

            // Add to current paragraph
            if !current_paragraph.is_empty() {
                current_paragraph.push(' ');
            }
            current_paragraph.push_str(trimmed);
        }

        // Don't forget last paragraph
        if !current_paragraph.is_empty() {
            paragraphs.push(current_paragraph.trim().to_string());
        }

        (paragraphs, headers)
    }

    /// Check if text is title case (first letter of each word capitalized)
    fn is_title_case(text: &str) -> bool {
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return false;
        }

        // At least 50% of significant words should be capitalized
        let significant_words: Vec<&&str> = words
            .iter()
            .filter(|w| w.len() > 3)  // Skip small words
            .collect();

        if significant_words.is_empty() {
            return false;
        }

        let capitalized = significant_words
            .iter()
            .filter(|w| {
                w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            })
            .count();

        capitalized >= significant_words.len() / 2
    }

    /// Check if text is all caps
    fn is_all_caps(text: &str) -> bool {
        let letters: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
        !letters.is_empty() && letters.iter().all(|c| c.is_uppercase())
    }

    /// Extract document metadata
    fn extract_metadata(doc: &Document) -> DocumentMetadata {
        let mut metadata = DocumentMetadata::default();

        // Helper function to convert bytes to string
        fn bytes_to_string(bytes: &[u8]) -> Option<String> {
            std::str::from_utf8(bytes).ok().map(|s| s.to_string())
        }

        // Try to get info dictionary
        if let Ok(info_ref) = doc.trailer.get(b"Info") {
            if let Ok(info_ref) = info_ref.as_reference() {
                if let Ok(info_dict) = doc.get_object(info_ref) {
                    if let Ok(dict) = info_dict.as_dict() {
                        // Extract common fields
                        if let Ok(title) = dict.get(b"Title") {
                            if let Ok(s) = title.as_str() {
                                metadata.title = bytes_to_string(s);
                            }
                        }
                        if let Ok(author) = dict.get(b"Author") {
                            if let Ok(s) = author.as_str() {
                                metadata.author = bytes_to_string(s);
                            }
                        }
                        if let Ok(subject) = dict.get(b"Subject") {
                            if let Ok(s) = subject.as_str() {
                                metadata.subject = bytes_to_string(s);
                            }
                        }
                        if let Ok(keywords) = dict.get(b"Keywords") {
                            if let Ok(s) = keywords.as_str() {
                                if let Some(kw_str) = bytes_to_string(s) {
                                    metadata.keywords = kw_str
                                        .split(&[',', ';'][..])
                                        .map(|k| k.trim().to_string())
                                        .filter(|k| !k.is_empty())
                                        .collect();
                                }
                            }
                        }
                        if let Ok(creator) = dict.get(b"Creator") {
                            if let Ok(s) = creator.as_str() {
                                metadata.creator = bytes_to_string(s);
                            }
                        }
                        if let Ok(producer) = dict.get(b"Producer") {
                            if let Ok(s) = producer.as_str() {
                                metadata.producer = bytes_to_string(s);
                            }
                        }
                    }
                }
            }
        }

        metadata
    }

    /// Get full text with page markers
    pub fn extract_text_with_pages(path: &Path) -> Result<Vec<(u32, String)>> {
        let doc = Document::load(path)
            .map_err(|e| PDFError::LoadError(e.to_string()))?;

        let mut pages = Vec::new();
        for (page_num, _page_id) in doc.get_pages() {
            let text = doc.extract_text(&[page_num])
                .map_err(|e| PDFError::ExtractionError(e.to_string()))?;
            pages.push((page_num, text));
        }
        Ok(pages)
    }

    /// Get page count without extracting content
    pub fn get_page_count(path: &Path) -> Result<usize> {
        let doc = Document::load(path)
            .map_err(|e| PDFError::LoadError(e.to_string()))?;
        Ok(doc.get_pages().len())
    }

    // ========================================================================
    // Enhanced Extraction with Fallback (Task 0.1, 0.2)
    // ========================================================================

    /// Extract with automatic fallback if lopdf fails or produces low-quality output.
    ///
    /// This method tries the primary `lopdf` extraction first, validates the output
    /// quality, and falls back to `pdf-extract` if the result is garbled or empty.
    ///
    /// # Arguments
    /// * `path` - Path to the PDF file
    /// * `password` - Optional password for encrypted PDFs
    ///
    /// # Returns
    /// * `Result<ExtractedDocument>` - The extracted document or an error
    pub fn extract_with_fallback(
        path: &Path,
        password: Option<&str>,
    ) -> Result<ExtractedDocument> {
        // Use dual-extraction approach: try both methods, take best result
        Self::extract_dual(path, password)
    }

    /// Dual extraction: Run both lopdf and pdf-extract, return whichever extracts more content.
    ///
    /// This approach is more robust than sequential fallback because some PDFs work better
    /// with one extractor vs the other regardless of "quality" metrics.
    ///
    /// # Arguments
    /// * `path` - Path to the PDF file
    /// * `password` - Optional password for encrypted PDFs
    pub fn extract_dual(
        path: &Path,
        password: Option<&str>,
    ) -> Result<ExtractedDocument> {
        // Try lopdf
        let lopdf_result = Self::extract_structured_with_password(path, password);

        // Try pdf-extract (runs in parallel conceptually, but we're single-threaded here)
        let pdf_extract_result = Self::extract_with_pdf_extract(path);

        // Evaluate both results
        let lopdf_chars = lopdf_result.as_ref()
            .map(Self::count_printable_chars)
            .unwrap_or(0);

        let pdf_extract_chars = pdf_extract_result.as_ref()
            .map(Self::count_printable_chars)
            .unwrap_or(0);

        log::info!(
            "PDF extraction for {:?}: lopdf={} chars, pdf-extract={} chars",
            path.file_name().unwrap_or_default(),
            lopdf_chars,
            pdf_extract_chars
        );

        // Choose the result with more content, with quality checks
        // For scanned/image PDFs, we return empty documents gracefully rather than failing
        match (lopdf_result, pdf_extract_result) {
            (Ok(lopdf_doc), Ok(pdfextract_doc)) => {
                // Both succeeded - choose the one with more printable content
                if lopdf_chars >= pdf_extract_chars && lopdf_chars > 0 {
                    if Self::is_extraction_quality_acceptable(&lopdf_doc) {
                        log::info!("Using lopdf extraction ({} chars)", lopdf_chars);
                    } else {
                        log::warn!("Using lopdf extraction ({} chars) - quality check failed but has most content", lopdf_chars);
                    }
                    Ok(lopdf_doc)
                } else if pdf_extract_chars > 0 {
                    if Self::is_extraction_quality_acceptable(&pdfextract_doc) {
                        log::info!("Using pdf-extract extraction ({} chars)", pdf_extract_chars);
                    } else {
                        log::warn!("Using pdf-extract extraction ({} chars) - quality check failed but has most content", pdf_extract_chars);
                    }
                    Ok(pdfextract_doc)
                } else {
                    // Both have 0 chars - likely a scanned/image PDF
                    // Return lopdf result for page structure (page count) even if empty
                    log::warn!(
                        "PDF {:?} appears to be scanned/image-based: both extractors returned 0 text. \
                         OCR would be required to extract content. Returning empty document with page structure.",
                        path.file_name().unwrap_or_default()
                    );
                    Ok(lopdf_doc)
                }
            }
            (Ok(lopdf_doc), Err(e)) => {
                // Only lopdf succeeded
                if lopdf_chars > 0 {
                    log::info!("Using lopdf extraction ({} chars, pdf-extract failed: {})", lopdf_chars, e);
                    Ok(lopdf_doc)
                } else {
                    // lopdf loaded the PDF but got no text - likely scanned
                    log::warn!(
                        "PDF {:?} appears to be scanned/image-based: lopdf extracted 0 text, pdf-extract failed ({}). \
                         OCR would be required. Returning empty document with page structure.",
                        path.file_name().unwrap_or_default(),
                        e
                    );
                    Ok(lopdf_doc)
                }
            }
            (Err(e), Ok(pdfextract_doc)) => {
                // Only pdf-extract succeeded
                if pdf_extract_chars > 0 {
                    log::info!("Using pdf-extract extraction ({} chars, lopdf failed: {})", pdf_extract_chars, e);
                    Ok(pdfextract_doc)
                } else {
                    // pdf-extract loaded but got no text - likely scanned
                    log::warn!(
                        "PDF {:?} appears to be scanned/image-based: pdf-extract extracted 0 text, lopdf failed ({}). \
                         OCR would be required. Returning empty document.",
                        path.file_name().unwrap_or_default(),
                        e
                    );
                    Ok(pdfextract_doc)
                }
            }
            (Err(e1), Err(e2)) => {
                // Both failed to even load the PDF
                log::error!("Both extractors failed to load PDF: lopdf: {}, pdf-extract: {}", e1, e2);
                Err(PDFError::ExtractionError(format!(
                    "Failed to load PDF. lopdf: {}; pdf-extract: {}",
                    e1, e2
                )))
            }
        }
    }

    /// Count printable characters in an extracted document
    fn count_printable_chars(doc: &ExtractedDocument) -> usize {
        doc.pages.iter()
            .map(|p| p.text.chars().filter(|c| {
                c.is_ascii_alphanumeric()
                    || c.is_ascii_punctuation()
                    || c.is_whitespace()
                    || c.is_alphabetic()
            }).count())
            .sum()
    }

    /// Extract structured content with optional password support.
    ///
    /// # Arguments
    /// * `path` - Path to the PDF file
    /// * `password` - Optional password for encrypted PDFs
    pub fn extract_structured_with_password(
        path: &Path,
        password: Option<&str>,
    ) -> Result<ExtractedDocument> {
        // Load the document
        let mut doc = match Document::load(path) {
            Ok(doc) => doc,
            Err(e) => {
                let err_str = e.to_string();
                // lopdf may throw error when loading encrypted PDF without password
                if err_str.contains("encrypted") || err_str.contains("password") {
                    return Err(PDFError::PasswordRequired);
                }
                return Err(PDFError::LoadError(err_str));
            }
        };

        // Check if document is encrypted and handle decryption
        if doc.is_encrypted() {
            match password {
                Some(pwd) => {
                    doc.decrypt(pwd).map_err(|e| {
                        let err_str = e.to_string();
                        if err_str.contains("password") || err_str.contains("decrypt") {
                            PDFError::InvalidPassword
                        } else {
                            PDFError::LoadError(format!("Decryption failed: {}", e))
                        }
                    })?;
                }
                None => {
                    return Err(PDFError::PasswordRequired);
                }
            }
        }

        Self::extract_from_document(doc, path)
    }

    /// Check extraction quality by analyzing the ratio of printable characters.
    ///
    /// Returns `true` if the extraction produced usable text, `false` if the
    /// output appears garbled (high ratio of non-printable characters).
    fn is_extraction_quality_acceptable(doc: &ExtractedDocument) -> bool {
        let total_text: String = doc.pages.iter()
            .map(|p| p.text.as_str())
            .collect();

        if total_text.is_empty() {
            return false;
        }

        // Check for high ratio of non-printable/garbled characters
        let total_chars = total_text.len();
        let printable_count = total_text.chars()
            .filter(|c| {
                c.is_ascii_alphanumeric()
                    || c.is_ascii_punctuation()
                    || c.is_whitespace()
                    // Allow common Unicode characters (accented letters, etc.)
                    || c.is_alphabetic()
            })
            .count();

        let printable_ratio = printable_count as f32 / total_chars as f32;

        // Also check for reasonable word structure
        let word_count = total_text.split_whitespace().count();
        let avg_word_len = if word_count > 0 {
            total_text.split_whitespace()
                .map(|w| w.len())
                .sum::<usize>() as f32 / word_count as f32
        } else {
            0.0
        };

        // Quality criteria:
        // - At least 85% printable characters
        // - Average word length between 2 and 15 (catches garbled output)
        // - At least some words extracted
        printable_ratio > 0.85
            && avg_word_len > 2.0
            && avg_word_len < 15.0
            && word_count > 10
    }

    /// Fallback extraction using the `pdf-extract` crate.
    ///
    /// This is used when `lopdf` fails or produces garbled output. Note that
    /// `pdf-extract` doesn't preserve page boundaries, so the result will be
    /// a single-page document.
    fn extract_with_pdf_extract(path: &Path) -> Result<ExtractedDocument> {
        let bytes = std::fs::read(path)?;
        let text = pdf_extract::extract_text_from_mem(&bytes)
            .map_err(|e| PDFError::FallbackFailed(format!("pdf-extract failed: {}", e)))?;

        if text.trim().is_empty() {
            return Err(PDFError::FallbackFailed(
                "pdf-extract returned empty content".to_string()
            ));
        }

        let (paragraphs, headers) = Self::analyze_text_structure(&text);

        // pdf-extract doesn't preserve page boundaries, create single-page doc
        Ok(ExtractedDocument {
            source_path: path.to_string_lossy().to_string(),
            page_count: 1,
            pages: vec![ExtractedPage {
                page_number: 1,
                text: text.clone(),
                paragraphs,
                headers,
            }],
            metadata: DocumentMetadata::default(),
        })
    }

    /// Internal helper to extract from a loaded Document.
    fn extract_from_document(doc: Document, path: &Path) -> Result<ExtractedDocument> {
        let page_ids: Vec<_> = doc.get_pages().into_iter().collect();
        let page_count = page_ids.len();
        let mut pages = Vec::with_capacity(page_count);

        for (page_num, _page_id) in page_ids {
            let text = doc.extract_text(&[page_num])
                .map_err(|e| PDFError::ExtractionError(e.to_string()))?;

            let (paragraphs, headers) = Self::analyze_text_structure(&text);

            pages.push(ExtractedPage {
                page_number: page_num,
                text,
                paragraphs,
                headers,
            });
        }

        let metadata = Self::extract_metadata(&doc);

        Ok(ExtractedDocument {
            source_path: path.to_string_lossy().to_string(),
            page_count,
            pages,
            metadata,
        })
    }

    // ========================================================================
    // OCR Extraction (for scanned/image-based PDFs)
    // ========================================================================

    /// Check if OCR tools (pdftoppm and tesseract) are available on the system
    pub fn is_ocr_available() -> bool {
        let pdftoppm = std::process::Command::new("pdftoppm")
            .arg("-v")
            .output()
            .is_ok();

        let tesseract = std::process::Command::new("tesseract")
            .arg("--version")
            .output()
            .is_ok();

        if !pdftoppm {
            log::debug!("pdftoppm not found - install poppler-utils for OCR support");
        }
        if !tesseract {
            log::debug!("tesseract not found - install tesseract-ocr for OCR support");
        }

        pdftoppm && tesseract
    }

    /// Extract text from a scanned/image-based PDF using OCR.
    ///
    /// Requires `pdftoppm` (from poppler-utils) and `tesseract` to be installed.
    /// This is significantly slower than text extraction but works on scanned PDFs.
    ///
    /// # Arguments
    /// * `path` - Path to the PDF file
    /// * `dpi` - Resolution for rendering (default: 300, higher = better quality but slower)
    /// * `lang` - Tesseract language code (default: "eng")
    pub fn extract_with_ocr(
        path: &Path,
        dpi: Option<u32>,
        lang: Option<&str>,
    ) -> Result<ExtractedDocument> {
        Self::extract_with_ocr_progress(path, dpi, lang, None)
    }

    /// Extract text from a scanned/image-based PDF using OCR with progress reporting.
    ///
    /// # Arguments
    /// * `path` - Path to the PDF file
    /// * `dpi` - Resolution for rendering (default: 300, higher = better quality but slower)
    /// * `lang` - Tesseract language code (default: "eng")
    /// * `progress_callback` - Optional callback for progress updates (current_page, total_pages)
    pub fn extract_with_ocr_progress(
        path: &Path,
        dpi: Option<u32>,
        lang: Option<&str>,
        progress_callback: Option<OcrProgressCallback>,
    ) -> Result<ExtractedDocument> {
        let dpi = dpi.unwrap_or(300);
        let lang = lang.unwrap_or("eng");

        // Check if OCR tools are available
        if !Self::is_ocr_available() {
            return Err(PDFError::ExtractionError(
                "OCR requires pdftoppm (poppler-utils) and tesseract-ocr to be installed".to_string()
            ));
        }

        // Create temp directory for images
        let temp_dir = tempfile::tempdir()
            .map_err(|e| PDFError::IoError(e))?;

        let temp_path = temp_dir.path();
        let output_prefix = temp_path.join("page");

        log::info!("Starting OCR extraction for {:?} (dpi={}, lang={})",
            path.file_name().unwrap_or_default(), dpi, lang);

        // Convert PDF to images using pdftoppm
        let pdftoppm_output = std::process::Command::new("pdftoppm")
            .arg("-png")
            .arg("-r")
            .arg(dpi.to_string())
            .arg(path)
            .arg(&output_prefix)
            .output()
            .map_err(|e| PDFError::ExtractionError(format!("Failed to run pdftoppm: {}", e)))?;

        if !pdftoppm_output.status.success() {
            let stderr = String::from_utf8_lossy(&pdftoppm_output.stderr);
            return Err(PDFError::ExtractionError(format!(
                "pdftoppm failed: {}", stderr
            )));
        }

        // Find generated image files and sort them
        let mut image_files: Vec<_> = std::fs::read_dir(temp_path)
            .map_err(|e| PDFError::IoError(e))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "png").unwrap_or(false))
            .collect();

        image_files.sort_by_key(|e| e.path());

        if image_files.is_empty() {
            return Err(PDFError::ExtractionError(
                "pdftoppm produced no images".to_string()
            ));
        }

        let total_pages = image_files.len();
        log::info!("Rendered {} pages, starting OCR...", total_pages);

        let mut pages = Vec::with_capacity(total_pages);
        let mut total_chars = 0usize;

        for (i, entry) in image_files.iter().enumerate() {
            let image_path = entry.path();
            let page_num = (i + 1) as u32;

            // Report progress via callback
            if let Some(ref callback) = progress_callback {
                callback(i + 1, total_pages);
            }

            // Run tesseract on the image
            let tesseract_output = std::process::Command::new("tesseract")
                .arg(&image_path)
                .arg("stdout")
                .arg("-l")
                .arg(lang)
                .arg("--psm")
                .arg("1") // Automatic page segmentation with OSD
                .output()
                .map_err(|e| PDFError::ExtractionError(format!(
                    "Failed to run tesseract on page {}: {}", page_num, e
                )))?;

            if !tesseract_output.status.success() {
                let stderr = String::from_utf8_lossy(&tesseract_output.stderr);
                log::warn!("Tesseract warning on page {}: {}", page_num, stderr);
            }

            let text = String::from_utf8_lossy(&tesseract_output.stdout).to_string();
            total_chars += text.len();

            let (paragraphs, headers) = Self::analyze_text_structure(&text);

            pages.push(ExtractedPage {
                page_number: page_num,
                text,
                paragraphs,
                headers,
            });

            // Progress logging for large documents
            if (i + 1) % 10 == 0 {
                log::info!("OCR progress: {}/{} pages", i + 1, total_pages);
            }
        }

        log::info!(
            "OCR complete for {:?}: {} pages, {} chars extracted",
            path.file_name().unwrap_or_default(),
            pages.len(),
            total_chars
        );

        Ok(ExtractedDocument {
            source_path: path.to_string_lossy().to_string(),
            page_count: pages.len(),
            pages,
            metadata: DocumentMetadata::default(),
        })
    }

    /// Extract with OCR fallback: tries text extraction first, falls back to OCR if empty.
    ///
    /// This is the recommended method for handling mixed PDF collections where some
    /// may be scanned and others have embedded text.
    pub fn extract_with_ocr_fallback(
        path: &Path,
        password: Option<&str>,
    ) -> Result<ExtractedDocument> {
        Self::extract_with_ocr_fallback_progress(path, password, None)
    }

    /// Extract with OCR fallback and progress reporting.
    ///
    /// # Arguments
    /// * `path` - Path to the PDF file
    /// * `password` - Optional password for encrypted PDFs
    /// * `progress_callback` - Optional callback for OCR progress (current_page, total_pages)
    /// Check if extracted content is garbage (failed font decoding)
    /// Common patterns: "Identity-H Unimplemented", repeated glyph names, etc.
    fn is_garbage_content(result: &ExtractedDocument) -> bool {
        let full_text: String = result.pages.iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        // Check for common garbage patterns from failed CID font decoding
        let garbage_patterns = [
            "Identity-H Unimplemented",
            "Identity-V Unimplemented",
            "CIDFont Unimplemented",
            "ToUnicode Unimplemented",
        ];

        for pattern in garbage_patterns {
            // If garbage pattern appears frequently, it's failed decoding
            let count = full_text.matches(pattern).count();
            if count > 10 {
                log::warn!(
                    "Detected garbage pattern '{}' {} times - font decoding failed",
                    pattern, count
                );
                return true;
            }
        }

        // Check for excessive repetition (same phrase repeated many times)
        // This catches other garbage patterns we haven't explicitly listed
        let words: Vec<&str> = full_text.split_whitespace().take(1000).collect();
        if words.len() >= 100 {
            let mut word_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
            for word in &words {
                *word_counts.entry(word).or_insert(0) += 1;
            }
            // If any single word is more than 30% of the sample, suspicious
            let max_count = word_counts.values().max().copied().unwrap_or(0);
            if max_count > words.len() / 3 {
                log::warn!(
                    "Detected excessive word repetition ({}/{} words) - likely garbage",
                    max_count, words.len()
                );
                return true;
            }
        }

        false
    }

    pub fn extract_with_ocr_fallback_progress(
        path: &Path,
        password: Option<&str>,
        progress_callback: Option<OcrProgressCallback>,
    ) -> Result<ExtractedDocument> {
        // First try regular extraction
        let result = Self::extract_dual(path, password)?;

        // Check if we got meaningful content using page-count-aware threshold
        let total_chars = Self::count_printable_chars(&result);
        let page_count = result.page_count.max(1);
        let chars_per_page = total_chars / page_count;

        // Expect at least 200 chars per page for a text-based PDF
        // TTRPG rulebooks typically have 1000-3000 chars per page
        let min_chars_per_page = 200;

        // Check for garbage content from failed font decoding
        let is_garbage = Self::is_garbage_content(&result);

        log::info!(
            "PDF {:?}: {} pages, {} total chars, {} chars/page (threshold: {} chars/page, garbage: {})",
            path.file_name().unwrap_or_default(),
            page_count, total_chars, chars_per_page, min_chars_per_page, is_garbage
        );

        if chars_per_page >= min_chars_per_page && !is_garbage {
            // Got enough meaningful text per page, use regular extraction
            log::info!("Text extraction successful, skipping OCR");
            return Ok(result);
        }

        // Reason for needing OCR
        let ocr_reason = if is_garbage {
            "garbage content from failed font decoding"
        } else {
            "too few chars per page (likely scanned)"
        };

        // Need OCR - either too few chars or garbage content
        if Self::is_ocr_available() {
            log::info!(
                "Falling back to OCR for {:?}: {} (extracted {} chars)",
                path.file_name().unwrap_or_default(),
                ocr_reason,
                total_chars
            );

            match Self::extract_with_ocr_progress(path, None, None, progress_callback) {
                Ok(ocr_result) => {
                    let ocr_chars = Self::count_printable_chars(&ocr_result);
                    if ocr_chars > total_chars {
                        log::info!(
                            "OCR extracted {} chars (vs {} from text extraction)",
                            ocr_chars, total_chars
                        );
                        return Ok(ocr_result);
                    } else {
                        log::warn!(
                            "OCR didn't improve extraction ({} vs {} chars), using original",
                            ocr_chars, total_chars
                        );
                    }
                }
                Err(e) => {
                    log::warn!("OCR failed: {}, using original extraction result", e);
                }
            }
        } else {
            log::warn!(
                "PDF {:?} appears to be scanned but OCR tools not available. \
                 Install poppler-utils and tesseract-ocr for OCR support.",
                path.file_name().unwrap_or_default()
            );
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_title_case() {
        assert!(PDFParser::is_title_case("Chapter One: The Beginning"));
        assert!(PDFParser::is_title_case("THE GOBLIN CAVE"));
        assert!(!PDFParser::is_title_case("this is not title case"));
    }

    #[test]
    fn test_is_all_caps() {
        assert!(PDFParser::is_all_caps("CHAPTER ONE"));
        assert!(PDFParser::is_all_caps("GOBLIN CAVE"));
        assert!(!PDFParser::is_all_caps("Not All Caps"));
    }

    #[test]
    fn test_analyze_structure() {
        let text = "CHAPTER ONE\n\nThis is the first paragraph.\n\nThis is the second paragraph.";
        let (paragraphs, headers) = PDFParser::analyze_text_structure(text);

        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0], "CHAPTER ONE");
        // Paragraphs include the header line, so we have at least 2 content paragraphs
        assert!(paragraphs.len() >= 2, "Expected at least 2 paragraphs, got {}", paragraphs.len());
    }

    #[test]
    fn test_extraction_quality_acceptable() {
        // Good quality extraction
        let good_doc = ExtractedDocument {
            source_path: "test.pdf".to_string(),
            page_count: 1,
            pages: vec![ExtractedPage {
                page_number: 1,
                text: "This is a good quality extraction with normal English text. It has multiple sentences and proper structure. The words are of reasonable length and everything looks fine.".to_string(),
                paragraphs: vec![],
                headers: vec![],
            }],
            metadata: DocumentMetadata::default(),
        };
        assert!(PDFParser::is_extraction_quality_acceptable(&good_doc));

        // Empty extraction
        let empty_doc = ExtractedDocument {
            source_path: "test.pdf".to_string(),
            page_count: 1,
            pages: vec![ExtractedPage {
                page_number: 1,
                text: "".to_string(),
                paragraphs: vec![],
                headers: vec![],
            }],
            metadata: DocumentMetadata::default(),
        };
        assert!(!PDFParser::is_extraction_quality_acceptable(&empty_doc));

        // Garbled extraction (mostly non-printable characters)
        let garbled_doc = ExtractedDocument {
            source_path: "test.pdf".to_string(),
            page_count: 1,
            pages: vec![ExtractedPage {
                page_number: 1,
                text: "□□□□□□□□□□□□□□□□□□□□".to_string(),
                paragraphs: vec![],
                headers: vec![],
            }],
            metadata: DocumentMetadata::default(),
        };
        assert!(!PDFParser::is_extraction_quality_acceptable(&garbled_doc));
    }
}
