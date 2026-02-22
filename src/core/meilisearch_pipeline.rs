//! Meilisearch Ingestion Pipeline
//!
//! Handles document parsing, chunking, and indexing into Meilisearch.
//! Supports multiple extraction providers: kreuzberg (local) or Claude API.
//!
//! ## Two-Phase Pipeline
//!
//! The pipeline uses a two-phase workflow:
//! - **Phase 1 (Extract)**: Extract raw pages to `<slug>-raw` index
//! - **Phase 2 (Chunk)**: Create semantic chunks in `<slug>` index
//!
//! This enables page number attribution in search results by tracking
//! which raw pages each chunk was derived from.
//!
//! ## Migration Note
//!
//! This module has been migrated from HTTP-based `meilisearch_sdk` to the
//! embedded `meilisearch_lib`. All operations are now synchronous and use
//! the `MeilisearchLib` API directly.

use crate::core::search::{LibraryDocumentMetadata, SearchError, INDEX_LIBRARY_METADATA};
use crate::ingestion::claude_extractor::ClaudeDocumentExtractor;
use crate::ingestion::extraction_settings::TextExtractionProvider;
use crate::ingestion::kreuzberg_extractor::DocumentExtractor;
use chrono::Utc;
use meilisearch_lib::{FilterableAttributesRule, MeilisearchLib, SearchQuery, Settings, Setting};
use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

// Re-export types that external code may need (preserving backward compatibility)
pub use crate::ingestion::slugs::{
    generate_source_slug, slugify, raw_index_name, chunks_index_name, MAX_SLUG_LENGTH,
};
pub use crate::ingestion::pipeline_models::{
    PipelineChunkConfig as ChunkConfig, PipelineConfig, RawDocument, ChunkedDocument,
    PageMetadata, TTRPGMetadata, ClassificationContext, ClassificationResult,
    ExtractionResult, ChunkingResult,
};

// ============================================================================
// Meilisearch Pipeline
// ============================================================================

pub struct MeilisearchPipeline {
    config: PipelineConfig,
}

impl MeilisearchPipeline {
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(PipelineConfig::default())
    }

    /// Get the current pipeline configuration.
    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    // ========================================================================
    // Two-Phase Pipeline: Extract → Raw → Chunk
    // ========================================================================

    /// Phase 1: Extract document content and store raw pages in `<slug>-raw` index.
    ///
    /// This creates a per-document index with one document per page, preserving
    /// the original page structure for provenance tracking.
    ///
    /// **Incremental/Resumable**: For OCR-based extraction, pages are written to
    /// Meilisearch as they are processed. If interrupted, the next run will
    /// resume from the last successfully persisted page.
    ///
    /// # Arguments
    /// * `meili` - Embedded Meilisearch library instance
    /// * `path` - Path to the document file
    /// * `title_override` - Optional custom title (otherwise derived from filename)
    ///
    /// # Returns
    /// `ExtractionResult` with slug, page count, and detected metadata
    pub async fn extract_to_raw(
        &self,
        meili: &MeilisearchLib,
        path: &Path,
        title_override: Option<&str>,
    ) -> Result<ExtractionResult, SearchError> {
        // Generate deterministic slug from filename
        let slug = generate_source_slug(path, title_override);
        let raw_index = raw_index_name(&slug);
        let chunks_index = chunks_index_name(&slug);

        let source_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        log::info!(
            "Two-phase ingestion: '{}' → raw='{}', chunks='{}'",
            source_name,
            raw_index,
            chunks_index
        );

        // FAIL-FAST: Create both indexes BEFORE expensive extraction
        // This ensures we have somewhere to persist results before doing OCR
        // Also configures sortable attributes needed for incremental extraction
        log::info!("Creating raw index '{}' (if not exists)...", raw_index);
        ensure_raw_index(meili, &raw_index).map_err(|e| {
            SearchError::ConfigError(format!(
                "Failed to create raw index '{}': {}. Aborting before extraction.",
                raw_index, e
            ))
        })?;

        log::info!("Creating chunks index '{}' (if not exists)...", chunks_index);
        ensure_chunks_index(meili, &chunks_index).map_err(|e| {
            SearchError::ConfigError(format!(
                "Failed to create chunks index '{}': {}. Aborting before extraction.",
                chunks_index, e
            ))
        })?;

        log::info!("Indexes ready. Starting document extraction...");

        // Determine source type from file extension
        let source_type = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .unwrap_or_else(|| "unknown".to_string());

        // Write initial library_metadata entry with status="processing"
        // Uses slug as ID for deterministic upsert behavior
        let initial_metadata = LibraryDocumentMetadata {
            id: slug.clone(),
            name: source_name.clone(),
            source_type: source_type.clone(),
            file_path: Some(path.to_string_lossy().to_string()),
            page_count: 0,
            chunk_count: 0,
            character_count: 0,
            content_index: chunks_index.clone(),
            status: "processing".to_string(),
            error_message: None,
            ingested_at: Utc::now().to_rfc3339(),
            // TTRPG metadata - user-editable, not set during ingestion
            game_system: None,
            setting: None,
            content_type: None,
            publisher: None,
        };

        if let Err(e) = save_library_document(meili, &initial_metadata) {
            log::warn!(
                "Failed to create initial library_metadata entry for '{}': {}",
                slug,
                e
            );
            // Continue anyway - this is not fatal
        } else {
            log::info!(
                "Created library_metadata entry '{}' with status=processing",
                slug
            );
        }

        // Dispatch based on extraction provider
        match self.config.extraction_settings.text_extraction_provider {
            TextExtractionProvider::Claude => {
                // Use Claude API for extraction
                log::info!("Using Claude API for extraction of '{}'", source_name);
                return self
                    .extract_to_raw_with_claude(
                        meili,
                        path,
                        &slug,
                        &raw_index,
                        &chunks_index,
                        &source_name,
                        &source_type,
                    )
                    .await;
            }
            TextExtractionProvider::Kreuzberg => {
                // Continue with kreuzberg extraction below
                log::info!("Using Kreuzberg (local) for extraction of '{}'", source_name);
            }
        }

        // First, try fast extraction with kreuzberg WITHOUT OCR (to check text content)
        // Using text_check_only() to avoid triggering the expensive OCR fallback
        let extractor = DocumentExtractor::text_check_only();
        let cb: Option<fn(f32, &str)> = None;
        let extracted = extractor.extract(path, cb).await.map_err(|e| {
            SearchError::ConfigError(format!("Document extraction failed: {}", e))
        })?;

        // Check if we got meaningful text (OCR threshold is typically 5000 chars)
        let is_pdf = extracted.mime_type == "application/pdf";
        let low_text = extracted.char_count < 5000;
        let needs_ocr = is_pdf && low_text;

        if needs_ocr {
            // Use incremental OCR extraction with per-page persistence
            // This writes pages to Meilisearch as they're OCR'd, enabling resumability
            log::info!(
                "Low text ({} chars) detected - using incremental OCR for '{}'",
                extracted.char_count,
                source_name
            );

            return self
                .extract_to_raw_incremental(
                    meili,
                    path,
                    &slug,
                    &raw_index,
                    &chunks_index,
                    &source_name,
                    &source_type,
                )
                .await;
        }

        // Fast path: text extracted successfully, store all pages using helper
        self.store_extracted_content(
            meili,
            path,
            &slug,
            &raw_index,
            &chunks_index,
            &source_name,
            &source_type,
            extracted,
        )
        .await
    }

    /// Incremental OCR extraction with per-page persistence for resumability.
    ///
    /// This method:
    /// 1. Queries the raw index for already-extracted pages
    /// 2. Resumes from the last page if partially complete
    /// 3. Writes each page to Meilisearch immediately after OCR
    async fn extract_to_raw_incremental(
        &self,
        meili: &MeilisearchLib,
        path: &Path,
        slug: &str,
        raw_index: &str,
        chunks_index: &str,
        source_name: &str,
        source_type: &str,
    ) -> Result<ExtractionResult, SearchError> {
        // Query existing pages to find where to resume
        let existing_page_count = get_highest_page_number(meili, raw_index);
        let start_page = existing_page_count + 1;

        log::info!(
            "Incremental extraction: {} existing pages, starting from page {}",
            existing_page_count,
            start_page
        );

        // Get total page count
        let extractor = DocumentExtractor::with_ocr();
        let total_pages = extractor
            .settings()
            .get_pdf_page_count_sync(path)
            .unwrap_or(0);

        if total_pages == 0 {
            return Err(SearchError::ConfigError(
                "Could not determine PDF page count".to_string(),
            ));
        }

        if start_page > total_pages {
            log::info!("All {} pages already extracted, skipping OCR", total_pages);

            // Still need to return metadata - fetch sample from existing pages
            let content_sample = get_content_sample(meili, raw_index);
            let ttrpg_metadata = TTRPGMetadata::extract(path, &content_sample, "document");

            // Update library_metadata with status="ready" (already complete)
            let final_metadata = LibraryDocumentMetadata {
                id: slug.to_string(),
                name: source_name.to_string(),
                source_type: source_type.to_string(),
                file_path: Some(path.to_string_lossy().to_string()),
                page_count: total_pages as u32,
                chunk_count: 0,
                character_count: 0,
                content_index: chunks_index.to_string(),
                status: "ready".to_string(),
                error_message: None,
                ingested_at: Utc::now().to_rfc3339(),
                game_system: None,
                setting: None,
                content_type: None,
                publisher: None,
            };

            if let Err(e) = save_library_document(meili, &final_metadata) {
                log::warn!("Failed to update library_metadata for '{}': {}", slug, e);
            }

            return Ok(ExtractionResult {
                slug: slug.to_string(),
                source_name: source_name.to_string(),
                raw_index: raw_index.to_string(),
                page_count: total_pages,
                total_chars: 0, // We don't recalculate for resumed extractions
                ttrpg_metadata,
            });
        }

        log::info!(
            "OCR extraction: pages {}-{} of {} for '{}'",
            start_page,
            total_pages,
            total_pages,
            source_name
        );

        // Auto-detect concurrency based on available CPU parallelism
        // Override with TTRPG_OCR_CONCURRENCY env var if needed
        let cpu_count = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4);
        let concurrency = std::env::var("TTRPG_OCR_CONCURRENCY")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or_else(|| {
                // Default: cpu_count / 2, clamped to 2-8
                (cpu_count / 2).max(2).min(8)
            });

        log::info!(
            "OCR concurrency: {} (available parallelism: {}, set TTRPG_OCR_CONCURRENCY to override)",
            concurrency,
            cpu_count
        );

        let mut total_chars_extracted = 0usize;
        let mut pages_written = 0usize;
        let mut current_page = start_page;

        // Process wave by wave with immediate persistence
        while current_page <= total_pages {
            let (wave_pages, next_page) = extractor
                .extract_one_wave(path, current_page, total_pages, concurrency)
                .await
                .map_err(|e| SearchError::ConfigError(format!("OCR wave failed: {}", e)))?;

            if wave_pages.is_empty() {
                break;
            }

            // Convert to RawDocuments and write immediately
            let raw_docs: Vec<serde_json::Value> = wave_pages
                .into_iter()
                .map(|(page_num, content)| {
                    total_chars_extracted += content.len();
                    let doc = RawDocument::new(slug, page_num as u32, content);
                    serde_json::to_value(&doc).unwrap_or_default()
                })
                .collect();

            let doc_count = raw_docs.len();

            match meili.add_documents(raw_index, raw_docs, Some("id".to_string())) {
                Ok(task) => {
                    let _ = meili.wait_for_task(task.uid, Some(Duration::from_secs(60)));
                    pages_written += doc_count;
                    log::info!(
                        "Wave complete: indexed {} pages (total: {}/{})",
                        doc_count,
                        pages_written + existing_page_count,
                        total_pages
                    );
                }
                Err(e) => {
                    log::error!("Failed to index wave: {}", e);
                    // Continue to next wave - partial progress is still saved
                }
            }

            current_page = next_page;
        }

        // Build final result
        let final_page_count = existing_page_count + pages_written;
        log::info!(
            "Incremental OCR complete: {} new pages extracted, {} total in index",
            pages_written,
            final_page_count
        );

        // Get content sample for metadata detection
        let content_sample = get_content_sample(meili, raw_index);
        let ttrpg_metadata = TTRPGMetadata::extract(path, &content_sample, "document");

        // Update library_metadata with final stats and status="ready"
        let final_metadata = LibraryDocumentMetadata {
            id: slug.to_string(),
            name: source_name.to_string(),
            source_type: source_type.to_string(),
            file_path: Some(path.to_string_lossy().to_string()),
            page_count: final_page_count as u32,
            chunk_count: 0, // Will be updated after chunking phase
            character_count: total_chars_extracted as u64,
            content_index: chunks_index.to_string(),
            status: "ready".to_string(),
            error_message: None,
            ingested_at: Utc::now().to_rfc3339(),
            game_system: None,
            setting: None,
            content_type: None,
            publisher: None,
        };

        if let Err(e) = save_library_document(meili, &final_metadata) {
            log::warn!("Failed to update library_metadata for '{}': {}", slug, e);
        } else {
            log::info!("Updated library_metadata '{}' with status=ready", slug);
        }

        Ok(ExtractionResult {
            slug: slug.to_string(),
            source_name: source_name.to_string(),
            raw_index: raw_index.to_string(),
            page_count: final_page_count,
            total_chars: total_chars_extracted,
            ttrpg_metadata,
        })
    }

    /// Extract document using Claude API.
    ///
    /// This uses Claude's vision capabilities for high-quality text extraction,
    /// especially useful for:
    /// - Scanned documents
    /// - Complex layouts (multi-column, mixed text/images)
    /// - Documents with handwritten annotations
    async fn extract_to_raw_with_claude(
        &self,
        meili: &MeilisearchLib,
        path: &Path,
        slug: &str,
        raw_index: &str,
        chunks_index: &str,
        source_name: &str,
        source_type: &str,
    ) -> Result<ExtractionResult, SearchError> {
        // Check if Claude extraction supports this format
        if !ClaudeDocumentExtractor::<crate::oauth::claude::FileTokenStorage>::is_supported(path) {
            log::warn!(
                "Claude extraction does not support '{}', falling back to kreuzberg",
                source_type
            );
            // Fall back to kreuzberg for unsupported formats
            let extractor = DocumentExtractor::with_ocr();
            let cb: Option<fn(f32, &str)> = None;
            let extracted = extractor.extract(path, cb).await.map_err(|e| {
                SearchError::ConfigError(format!("Document extraction failed: {}", e))
            })?;

            return self
                .store_extracted_content(
                    meili,
                    path,
                    slug,
                    raw_index,
                    chunks_index,
                    source_name,
                    source_type,
                    extracted,
                )
                .await;
        }

        // Create Claude extractor
        let claude_extractor = ClaudeDocumentExtractor::new().map_err(|e| {
            SearchError::ConfigError(format!("Failed to create Claude extractor: {}", e))
        })?;

        // Check authentication
        let is_authenticated = claude_extractor.is_authenticated().await.map_err(|e| {
            SearchError::ConfigError(format!("Claude auth check failed: {}", e))
        })?;

        if !is_authenticated {
            log::warn!("Claude API not authenticated, falling back to kreuzberg extraction");
            let extractor = DocumentExtractor::with_ocr();
            let cb: Option<fn(f32, &str)> = None;
            let extracted = extractor.extract(path, cb).await.map_err(|e| {
                SearchError::ConfigError(format!("Document extraction failed: {}", e))
            })?;

            return self
                .store_extracted_content(
                    meili,
                    path,
                    slug,
                    raw_index,
                    chunks_index,
                    source_name,
                    source_type,
                    extracted,
                )
                .await;
        }

        log::info!("Extracting '{}' using Claude API...", source_name);

        // Perform Claude extraction
        let cb: Option<fn(f32, &str)> = None;
        let extracted = claude_extractor.extract(path, cb).await.map_err(|e| {
            SearchError::ConfigError(format!("Claude extraction failed: {}", e))
        })?;

        self.store_extracted_content(
            meili,
            path,
            slug,
            raw_index,
            chunks_index,
            source_name,
            source_type,
            extracted,
        )
        .await
    }

    /// Import pre-extracted layout JSON (Anthropic format) into the raw index.
    ///
    /// This is useful for documents that have already been extracted using
    /// external tools or the Anthropic PDF extraction API. The JSON schema
    /// follows the Anthropic layout format with version, metadata, and pages.
    ///
    /// # Arguments
    /// * `meili` - Embedded Meilisearch library instance
    /// * `path` - Path to the layout JSON file
    /// * `title_override` - Optional custom title (otherwise derived from filename or metadata)
    ///
    /// # Returns
    /// `ExtractionResult` with slug, page count, and detected metadata
    pub async fn import_layout_json(
        &self,
        meili: &MeilisearchLib,
        path: &Path,
        title_override: Option<&str>,
    ) -> Result<ExtractionResult, SearchError> {
        use crate::ingestion::layout_json::LayoutDocument;

        // Load and parse the layout JSON
        let layout_doc = LayoutDocument::from_file(path)
            .map_err(|e| SearchError::ConfigError(format!("Failed to parse layout JSON: {}", e)))?;

        // Use title from metadata or override
        let doc_title = title_override
            .map(|s| s.to_string())
            .or_else(|| layout_doc.title().map(|s| s.to_string()))
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            });

        // Generate deterministic slug
        let slug = generate_source_slug(path, Some(&doc_title));
        let raw_index = raw_index_name(&slug);
        let chunks_index = chunks_index_name(&slug);

        let source_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        log::info!(
            "Layout JSON import: '{}' ({} pages) → raw='{}', chunks='{}'",
            doc_title,
            layout_doc.page_count(),
            raw_index,
            chunks_index
        );

        // Create indexes
        ensure_raw_index(meili, &raw_index).map_err(|e| {
            SearchError::ConfigError(format!("Failed to create raw index '{}': {}", raw_index, e))
        })?;

        ensure_chunks_index(meili, &chunks_index).map_err(|e| {
            SearchError::ConfigError(format!(
                "Failed to create chunks index '{}': {}",
                chunks_index, e
            ))
        })?;

        // Convert layout pages to raw documents
        let metadata_page_count = layout_doc.page_count();
        let pages = layout_doc.to_pages();
        let page_count = pages.len();

        if metadata_page_count != page_count {
            log::warn!(
                "Layout page count mismatch for document '{}': metadata page_count = {}, derived pages.len() = {}. Using derived pages.len() as canonical for indexing.",
                slug,
                metadata_page_count,
                page_count
            );
        }

        let mut total_chars = 0;
        let mut raw_documents = Vec::new();

        for page in &pages {
            total_chars += page.content.len();
            let raw_doc = RawDocument::new(&slug, page.page_number as u32, page.content.clone());
            raw_documents.push(raw_doc);
        }

        // Combine content sample for metadata detection
        let content_sample: String = pages
            .iter()
            .take(20)
            .map(|p| p.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        // Detect TTRPG metadata
        let ttrpg_metadata = TTRPGMetadata::extract(path, &content_sample, "document");

        log::info!(
            "Layout JSON parsed: {} pages, {} chars, game_system={:?}",
            page_count,
            total_chars,
            ttrpg_metadata.game_system
        );

        // Store raw documents in Meilisearch
        let json_docs: Vec<serde_json::Value> = raw_documents
            .iter()
            .map(|doc| serde_json::to_value(doc).unwrap_or_default())
            .collect();

        let task = meili.add_documents(&raw_index, json_docs, Some("id".to_string())).map_err(|e| {
            SearchError::MeilisearchError(format!("Failed to add raw documents: {}", e))
        })?;

        // Wait for indexing to complete
        meili
            .wait_for_task(task.uid, Some(Duration::from_secs(60)))
            .map_err(|e| {
                SearchError::MeilisearchError(format!("Failed to complete indexing: {}", e))
            })?;

        // Update library_metadata
        let final_metadata = LibraryDocumentMetadata {
            id: slug.clone(),
            name: doc_title.clone(),
            source_type: "layout_json".to_string(),
            file_path: Some(path.to_string_lossy().to_string()),
            page_count: page_count as u32,
            chunk_count: 0,
            character_count: total_chars as u64,
            content_index: chunks_index.clone(),
            status: "ready".to_string(),
            error_message: None,
            ingested_at: Utc::now().to_rfc3339(),
            game_system: None,
            setting: None,
            content_type: None,
            publisher: None,
        };

        if let Err(e) = save_library_document(meili, &final_metadata) {
            log::warn!("Failed to save library_metadata for '{}': {}", slug, e);
        }

        Ok(ExtractionResult {
            slug,
            source_name,
            raw_index,
            page_count,
            total_chars,
            ttrpg_metadata,
        })
    }

    /// Store extracted content into the raw index.
    ///
    /// Shared helper used by both kreuzberg and Claude extraction paths.
    async fn store_extracted_content(
        &self,
        meili: &MeilisearchLib,
        path: &Path,
        slug: &str,
        raw_index: &str,
        chunks_index: &str,
        source_name: &str,
        source_type: &str,
        extracted: crate::ingestion::kreuzberg_extractor::ExtractedContent,
    ) -> Result<ExtractionResult, SearchError> {
        let total_chars = extracted.char_count;
        let mut page_count = 0;
        let mut raw_documents = Vec::new();

        // Convert extracted pages to RawDocuments
        if let Some(pages) = extracted.pages {
            for page in pages {
                let raw_doc = RawDocument::new(slug, page.page_number as u32, page.content);
                raw_documents.push(raw_doc);
                page_count += 1;
            }
        } else {
            // Single page fallback (no page structure)
            let raw_doc = RawDocument::new(slug, 1, extracted.content.clone());
            raw_documents.push(raw_doc);
            page_count = 1;
        }

        // Combine content sample for metadata detection
        let content_sample: String = raw_documents
            .iter()
            .take(20)
            .map(|d| d.raw_content.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        // Detect TTRPG metadata
        let ttrpg_metadata = TTRPGMetadata::extract(path, &content_sample, "document");

        log::info!(
            "Extracted {} pages from '{}': system={:?}, category={:?}",
            page_count,
            source_name,
            ttrpg_metadata.game_system,
            ttrpg_metadata.content_category
        );

        // Store raw documents in Meilisearch
        let json_docs: Vec<serde_json::Value> = raw_documents
            .iter()
            .map(|doc| serde_json::to_value(doc).unwrap_or_default())
            .collect();

        let task = meili
            .add_documents(raw_index, json_docs, Some("id".to_string()))
            .map_err(|e| {
                SearchError::MeilisearchError(format!("Failed to add raw documents: {}", e))
            })?;

        // Wait for indexing to complete
        meili
            .wait_for_task(task.uid, Some(Duration::from_secs(60)))
            .map_err(|e| SearchError::MeilisearchError(format!("Raw indexing failed: {}", e)))?;

        log::info!("Stored {} raw pages in '{}'", page_count, raw_index);

        // Update library_metadata with final stats and status="ready"
        let final_metadata = LibraryDocumentMetadata {
            id: slug.to_string(),
            name: source_name.to_string(),
            source_type: source_type.to_string(),
            file_path: Some(path.to_string_lossy().to_string()),
            page_count: page_count as u32,
            chunk_count: 0, // Will be updated after chunking phase
            character_count: total_chars as u64,
            content_index: chunks_index.to_string(),
            status: "ready".to_string(),
            error_message: None,
            ingested_at: Utc::now().to_rfc3339(),
            // TTRPG metadata - user-editable, not set during ingestion
            game_system: None,
            setting: None,
            content_type: None,
            publisher: None,
        };

        if let Err(e) = save_library_document(meili, &final_metadata) {
            log::warn!("Failed to update library_metadata for '{}': {}", slug, e);
        } else {
            log::info!("Updated library_metadata '{}' with status=ready", slug);
        }

        Ok(ExtractionResult {
            slug: slug.to_string(),
            source_name: source_name.to_string(),
            raw_index: raw_index.to_string(),
            page_count,
            total_chars,
            ttrpg_metadata,
        })
    }

    /// Phase 2: Create semantic chunks from raw pages and store in `<slug>` index.
    ///
    /// Reads from the `<slug>-raw` index, applies semantic chunking that may span
    /// multiple pages, and stores chunks with provenance tracking (source_raw_ids).
    ///
    /// # Arguments
    /// * `meili` - Embedded Meilisearch library instance
    /// * `extraction` - Result from `extract_to_raw()`
    ///
    /// # Returns
    /// `ChunkingResult` with chunk count and pages consumed
    pub async fn chunk_from_raw(
        &self,
        meili: &MeilisearchLib,
        extraction: &ExtractionResult,
    ) -> Result<ChunkingResult, SearchError> {
        let slug = &extraction.slug;
        let raw_index = &extraction.raw_index;
        let chunks_index = chunks_index_name(slug);

        log::info!("Chunking from '{}' to '{}'", raw_index, chunks_index);

        // Ensure chunks index exists with proper settings
        ensure_chunks_index(meili, &chunks_index).map_err(|e| {
            SearchError::ConfigError(format!("Failed to create chunks index: {}", e))
        })?;

        // Fetch all raw documents from the raw index
        let (_total, docs) = meili.get_documents(raw_index, 0, 10000).map_err(|e| {
            SearchError::MeilisearchError(format!("Failed to fetch raw docs: {}", e))
        })?;

        // Convert JSON values back to RawDocument
        let raw_docs: Vec<RawDocument> = docs
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();

        if raw_docs.is_empty() {
            return Err(SearchError::DocumentNotFound(format!(
                "No raw documents found in '{}'",
                raw_index
            )));
        }

        let pages_consumed = raw_docs.len();

        // Sort by page number
        let mut sorted_docs = raw_docs;
        sorted_docs.sort_by_key(|d| d.page_number);

        // Create chunks with provenance tracking
        let chunks =
            self.create_chunks_with_provenance(slug, &sorted_docs, &extraction.ttrpg_metadata);

        let chunk_count = chunks.len();

        // Store chunks in Meilisearch
        let json_chunks: Vec<serde_json::Value> = chunks
            .iter()
            .map(|c| serde_json::to_value(c).unwrap_or_default())
            .collect();

        let task = meili
            .add_documents(&chunks_index, json_chunks, Some("id".to_string()))
            .map_err(|e| {
                SearchError::MeilisearchError(format!("Failed to add chunks: {}", e))
            })?;

        meili
            .wait_for_task(task.uid, Some(Duration::from_secs(60)))
            .map_err(|e| SearchError::MeilisearchError(format!("Chunk indexing failed: {}", e)))?;

        log::info!(
            "Created {} chunks from {} pages in '{}'",
            chunk_count,
            pages_consumed,
            chunks_index
        );

        Ok(ChunkingResult {
            slug: slug.clone(),
            chunks_index,
            chunk_count,
            pages_consumed,
        })
    }

    /// Combined two-phase ingestion: extract + chunk in one call.
    ///
    /// Convenience method that runs both phases sequentially.
    pub async fn ingest_two_phase(
        &self,
        meili: &MeilisearchLib,
        path: &Path,
        title_override: Option<&str>,
    ) -> Result<(ExtractionResult, ChunkingResult), SearchError> {
        let extraction = self.extract_to_raw(meili, path, title_override).await?;
        let chunking = self.chunk_from_raw(meili, &extraction).await?;
        Ok((extraction, chunking))
    }

    /// Create semantic chunks from raw documents with provenance tracking.
    ///
    /// Each chunk records which raw document IDs it was derived from,
    /// enabling page number attribution in search results.
    fn create_chunks_with_provenance(
        &self,
        slug: &str,
        raw_docs: &[RawDocument],
        metadata: &TTRPGMetadata,
    ) -> Vec<ChunkedDocument> {
        let config = &self.config.chunk_config;
        let mut chunks = Vec::new();
        let mut chunk_index = 0u32;

        let mut current_content = String::new();
        let mut current_source_ids: Vec<String> = Vec::new();

        // Create shared classification context for all chunks (efficiency)
        let classification_ctx = ClassificationContext::new();

        for doc in raw_docs {
            let doc_content = doc.raw_content.trim();
            if doc_content.is_empty() {
                continue;
            }

            // Check if adding this page would exceed max chunk size
            let would_exceed =
                current_content.len() + doc_content.len() + 1 > config.chunk_size * 2;

            // If we have content and would exceed, save current chunk
            if !current_content.is_empty() && would_exceed {
                // Create chunk from accumulated content with v2 enhanced metadata
                let chunk = ChunkedDocument::new(
                    slug,
                    chunk_index,
                    std::mem::take(&mut current_content),
                    std::mem::take(&mut current_source_ids),
                )
                .with_ttrpg_metadata(metadata)
                .with_classification_context(&classification_ctx);

                chunks.push(chunk);
                chunk_index += 1;
            }

            // Add page to current accumulation
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(doc_content);
            current_source_ids.push(doc.id.clone());

            // If single page exceeds target, split it into smaller chunks
            while current_content.len() > config.chunk_size {
                // Find a good split point (sentence boundary, paragraph, or forced)
                // find_split_point always returns a valid char boundary
                let split_at = find_split_point(&current_content, config.chunk_size);

                // Safe slicing - split_at is guaranteed to be a char boundary
                let chunk_content = current_content[..split_at].to_string();
                let chunk = ChunkedDocument::new(
                    slug,
                    chunk_index,
                    chunk_content,
                    current_source_ids.clone(), // Same source IDs for split chunks
                )
                .with_ttrpg_metadata(metadata)
                .with_classification_context(&classification_ctx);

                chunks.push(chunk);
                chunk_index += 1;

                // Keep overlap for continuity - ensure overlap_start is on a char boundary
                let overlap_start = snap_to_char_boundary(
                    &current_content,
                    split_at.saturating_sub(config.chunk_overlap),
                );
                current_content = current_content[overlap_start..].to_string();
            }
        }

        // Don't forget the last chunk
        if current_content.len() >= config.min_chunk_size {
            let chunk = ChunkedDocument::new(slug, chunk_index, current_content, current_source_ids)
                .with_ttrpg_metadata(metadata)
                .with_classification_context(&classification_ctx);
            chunks.push(chunk);
        }

        chunks
    }
}

impl Default for MeilisearchPipeline {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ============================================================================
// Text Splitting Utilities
// ============================================================================

/// Find the nearest valid UTF-8 char boundary at or before the given byte index.
/// This prevents panics when slicing strings with multi-byte characters.
fn snap_to_char_boundary(text: &str, index: usize) -> usize {
    if index >= text.len() {
        return text.len();
    }
    // Walk backwards to find a valid char boundary
    let mut pos = index;
    while pos > 0 && !text.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

/// Find a good split point in text, preferring sentence/paragraph boundaries.
/// Always returns a valid UTF-8 char boundary.
fn find_split_point(text: &str, target: usize) -> usize {
    // Snap target to a valid char boundary first
    let safe_target = snap_to_char_boundary(text, target);
    let search_range = text.get(..safe_target).unwrap_or(text);

    // Prefer paragraph break
    if let Some(pos) = search_range.rfind("\n\n") {
        if pos > safe_target / 2 {
            return pos + 2;
        }
    }

    // Then sentence boundary
    for pattern in [". ", "! ", "? ", ".\n", "!\n", "?\n"] {
        if let Some(pos) = search_range.rfind(pattern) {
            if pos > safe_target / 2 {
                return pos + pattern.len();
            }
        }
    }

    // Fallback to word boundary
    if let Some(pos) = search_range.rfind(' ') {
        return pos + 1;
    }

    // Last resort: hard cut at the safe boundary
    safe_target
}

// ============================================================================
// MeilisearchLib Helper Functions
// ============================================================================

/// Ensure a raw index exists with proper settings for page storage.
///
/// Creates the index if it doesn't exist and configures sortable attributes
/// for page_number ordering (needed for incremental extraction).
fn ensure_raw_index(meili: &MeilisearchLib, uid: &str) -> Result<(), SearchError> {
    // Create index if it doesn't exist
    if !meili
        .index_exists(uid)
        .map_err(|e| SearchError::MeilisearchError(e.to_string()))?
    {
        let task = meili
            .create_index(uid, Some("id".to_string()))
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;
        meili
            .wait_for_task(task.uid, Some(Duration::from_secs(30)))
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;
    }

    // Configure sortable attributes for page ordering
    let mut sortable: BTreeSet<String> = BTreeSet::new();
    sortable.insert("page_number".to_string());

    let settings: Settings<meilisearch_lib::Unchecked> = Settings {
        sortable_attributes: Setting::Set(sortable),
        ..Default::default()
    };

    let task = meili
        .update_settings(uid, settings)
        .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;
    meili
        .wait_for_task(task.uid, Some(Duration::from_secs(30)))
        .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

    Ok(())
}

/// Ensure a chunks index exists with proper settings for search.
///
/// Creates the index if it doesn't exist and configures searchable and
/// filterable attributes for optimal search performance.
fn ensure_chunks_index(meili: &MeilisearchLib, uid: &str) -> Result<(), SearchError> {
    // Create index if it doesn't exist
    if !meili
        .index_exists(uid)
        .map_err(|e| SearchError::MeilisearchError(e.to_string()))?
    {
        let task = meili
            .create_index(uid, Some("id".to_string()))
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;
        meili
            .wait_for_task(task.uid, Some(Duration::from_secs(30)))
            .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;
    }

    // Configure searchable, filterable, and sortable attributes
    let filterable: Vec<FilterableAttributesRule> = vec![
        FilterableAttributesRule::Field("source".to_string()),
        FilterableAttributesRule::Field("game_system".to_string()),
        FilterableAttributesRule::Field("content_category".to_string()),
    ];

    let mut sortable: BTreeSet<String> = BTreeSet::new();
    sortable.insert("chunk_index".to_string());

    let settings: Settings<meilisearch_lib::Unchecked> = Settings {
        searchable_attributes: Setting::Set(vec!["content".to_string()]).into(),
        filterable_attributes: Setting::Set(filterable),
        sortable_attributes: Setting::Set(sortable),
        ..Default::default()
    };

    let task = meili
        .update_settings(uid, settings)
        .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;
    meili
        .wait_for_task(task.uid, Some(Duration::from_secs(30)))
        .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

    Ok(())
}

/// Save a library document metadata entry.
fn save_library_document(
    meili: &MeilisearchLib,
    metadata: &LibraryDocumentMetadata,
) -> Result<(), SearchError> {
    let doc = serde_json::to_value(metadata)
        .map_err(|e| SearchError::ConfigError(format!("Failed to serialize metadata: {}", e)))?;

    let task = meili
        .add_documents(INDEX_LIBRARY_METADATA, vec![doc], Some("id".to_string()))
        .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

    meili
        .wait_for_task(task.uid, Some(Duration::from_secs(30)))
        .map_err(|e| SearchError::MeilisearchError(e.to_string()))?;

    Ok(())
}

/// Query the raw index to find the highest page number already extracted.
/// Returns 0 if no pages exist.
fn get_highest_page_number(meili: &MeilisearchLib, raw_index: &str) -> usize {
    // Use search with sort to find highest page number
    let query = SearchQuery::empty()
        .with_sort(vec!["page_number:desc".to_string()])
        .with_pagination(0, 1);

    match meili.search(raw_index, query) {
        Ok(results) => {
            if let Some(hit) = results.hits.first() {
                if let Some(page_num) = hit.document.get("page_number") {
                    if let Some(n) = page_num.as_u64() {
                        return n as usize;
                    }
                }
            }
            0
        }
        Err(e) => {
            log::warn!("Could not query existing pages from '{}': {}", raw_index, e);
            0
        }
    }
}

/// Get a content sample from the raw index for metadata detection.
fn get_content_sample(meili: &MeilisearchLib, raw_index: &str) -> String {
    let query = SearchQuery::empty()
        .with_sort(vec!["page_number:asc".to_string()])
        .with_pagination(0, 20);

    match meili.search(raw_index, query) {
        Ok(results) => results
            .hits
            .iter()
            .filter_map(|hit| hit.document.get("raw_content").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join(" "),
        Err(_) => String::new(),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestion::slugs::{generate_source_slug, slugify, MAX_SLUG_LENGTH};

    // ========================================================================
    // Slug Generation Tests (verify imports work)
    // ========================================================================

    #[test]
    fn test_slugify_basic() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("simple"), "simple");
        assert_eq!(slugify("UPPERCASE"), "uppercase");
    }

    #[test]
    fn test_slugify_special_characters() {
        assert_eq!(
            slugify("Delta Green - Handler's Guide"),
            "delta-green-handlers-guide"
        );
        assert_eq!(slugify("D&D 5th Edition"), "dd-5th-edition");
        assert_eq!(
            slugify("Call of Cthulhu (7th Ed)"),
            "call-of-cthulhu-7th-ed"
        );
        assert_eq!(
            slugify("Monster Manual: Expanded"),
            "monster-manual-expanded"
        );
    }

    #[test]
    fn test_slugify_long_input() {
        let long_input = "This Is A Very Long Title That Exceeds The Maximum Slug Length And Should Be Truncated At A Word Boundary";
        let slug = slugify(long_input);
        assert!(slug.len() <= MAX_SLUG_LENGTH);
        assert!(!slug.ends_with('-'));
    }

    #[test]
    fn test_generate_source_slug_from_path() {
        let path = Path::new("/home/user/rpg/Delta Green - Handler's Guide.pdf");
        assert_eq!(
            generate_source_slug(path, None),
            "delta-green-handlers-guide"
        );

        let path = Path::new("Monster_Manual_5e.pdf");
        assert_eq!(generate_source_slug(path, None), "monster-manual-5e");
    }

    // ========================================================================
    // Split Point Tests
    // ========================================================================

    #[test]
    fn test_find_split_point_paragraph() {
        let text = "First paragraph.\n\nSecond paragraph continues here.";
        let split = find_split_point(text, 30);
        assert!(split > 0);
        assert!(split <= 30);
    }

    #[test]
    fn test_find_split_point_sentence() {
        let text = "First sentence. Second sentence. Third sentence.";
        let split = find_split_point(text, 30);
        assert!(text[..split].ends_with(". ") || text[..split].ends_with(' '));
    }
}
