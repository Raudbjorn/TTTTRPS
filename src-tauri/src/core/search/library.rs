//! Library Document Repository
//!
//! Repository pattern for library document metadata persistence in Meilisearch.

use std::collections::{HashMap, HashSet};

use meilisearch_sdk::search::SearchResults;

use super::config::{
    all_indexes, INDEX_CHAT, INDEX_FICTION, INDEX_LIBRARY_METADATA, INDEX_RULES,
    TASK_TIMEOUT_SHORT_SECS,
};
use super::error::{Result, SearchError};
use super::models::{LibraryDocumentMetadata, SearchDocument};

/// Repository trait for library document operations
///
/// This trait decouples library document persistence from the core search client,
/// following the repository pattern.
#[async_trait::async_trait]
pub trait LibraryRepository: Send + Sync {
    /// Save library document metadata
    async fn save_library_document(&self, doc: &LibraryDocumentMetadata) -> Result<()>;

    /// List all library documents
    async fn list_library_documents(&self) -> Result<Vec<LibraryDocumentMetadata>>;

    /// Get a single library document by ID
    async fn get_library_document(&self, doc_id: &str)
        -> Result<Option<LibraryDocumentMetadata>>;

    /// Delete a library document (metadata only)
    async fn delete_library_document(&self, doc_id: &str) -> Result<()>;

    /// Delete library document and all its content chunks
    async fn delete_library_document_with_content(&self, doc_id: &str) -> Result<()>;

    /// Get library document count
    async fn library_document_count(&self) -> Result<u64>;

    /// Rebuild library metadata from existing content indices
    async fn rebuild_library_metadata(&self) -> Result<Vec<LibraryDocumentMetadata>>;
}

/// Implementation helper functions for LibraryRepository
pub struct LibraryRepositoryImpl<'a> {
    client: &'a meilisearch_sdk::client::Client,
    #[allow(dead_code)] // Reserved for future use (e.g., raw HTTP calls)
    host: &'a str,
}

impl<'a> LibraryRepositoryImpl<'a> {
    pub fn new(client: &'a meilisearch_sdk::client::Client, host: &'a str) -> Self {
        Self { client, host }
    }

    /// Save library document metadata to Meilisearch
    pub async fn save(&self, doc: &LibraryDocumentMetadata) -> Result<()> {
        let index = self.client.index(INDEX_LIBRARY_METADATA);
        let task = index.add_documents(&[doc], Some("id")).await?;
        task.wait_for_completion(
            self.client,
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)),
        )
        .await?;
        log::info!(
            "Saved library document metadata: {} ({})",
            doc.name,
            doc.id
        );
        Ok(())
    }

    /// List all library documents from Meilisearch
    ///
    /// Paginates through all documents to avoid truncation at any fixed limit.
    pub async fn list(&self) -> Result<Vec<LibraryDocumentMetadata>> {
        let index = self.client.index(INDEX_LIBRARY_METADATA);
        let mut all_docs = Vec::new();
        let mut offset = 0;
        const PAGE_SIZE: usize = 1000;

        loop {
            // Use search with empty query to get documents, sorted by ingested_at
            let results: SearchResults<LibraryDocumentMetadata> = index
                .search()
                .with_query("")
                .with_limit(PAGE_SIZE)
                .with_offset(offset)
                .with_sort(&["ingested_at:desc"])
                .execute()
                .await?;

            let batch_size = results.hits.len();
            all_docs.extend(results.hits.into_iter().map(|hit| hit.result));

            // If we got fewer than PAGE_SIZE results, we've fetched all documents
            if batch_size < PAGE_SIZE {
                break;
            }

            offset += PAGE_SIZE;
        }

        Ok(all_docs)
    }

    /// Get a single library document by ID
    pub async fn get(&self, doc_id: &str) -> Result<Option<LibraryDocumentMetadata>> {
        let index = self.client.index(INDEX_LIBRARY_METADATA);

        match index
            .get_document::<LibraryDocumentMetadata>(doc_id)
            .await
        {
            Ok(doc) => Ok(Some(doc)),
            Err(meilisearch_sdk::errors::Error::Meilisearch(e))
                if e.error_code == meilisearch_sdk::errors::ErrorCode::DocumentNotFound =>
            {
                Ok(None)
            }
            Err(e) => Err(SearchError::from(e)),
        }
    }

    /// Delete a library document from Meilisearch (metadata only)
    pub async fn delete(&self, doc_id: &str) -> Result<()> {
        let index = self.client.index(INDEX_LIBRARY_METADATA);
        let task = index.delete_document(doc_id).await?;
        task.wait_for_completion(
            self.client,
            Some(std::time::Duration::from_millis(100)),
            Some(std::time::Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)),
        )
        .await?;
        log::info!("Deleted library document metadata: {}", doc_id);
        Ok(())
    }

    /// Delete an index entirely (helper for delete_with_content)
    pub async fn delete_index(&self, name: &str) -> Result<()> {
        match self.client.delete_index(name).await {
            Ok(task) => {
                task.wait_for_completion(
                    self.client,
                    Some(std::time::Duration::from_millis(100)),
                    Some(std::time::Duration::from_secs(TASK_TIMEOUT_SHORT_SECS)),
                )
                .await?;
                log::info!("Deleted index '{}'", name);
                Ok(())
            }
            Err(meilisearch_sdk::errors::Error::Meilisearch(err))
                if err.error_code == meilisearch_sdk::errors::ErrorCode::IndexNotFound =>
            {
                // Index doesn't exist - that's fine for deletion
                log::debug!("Index '{}' already doesn't exist", name);
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Delete library document and all its content chunks from the content index
    ///
    /// Each document has its own dedicated indexes:
    /// - Chunks index: named same as doc_id (the slug)
    /// - Raw index: named "{doc_id}-raw"
    pub async fn delete_with_content(&self, doc_id: &str) -> Result<()> {
        // Delete the chunks index (named same as the doc_id/slug)
        self.delete_index(doc_id).await?;

        // Delete the raw index (named "{doc_id}-raw")
        let raw_index = format!("{}-raw", doc_id);
        self.delete_index(&raw_index).await?;

        log::info!(
            "Deleted indexes '{}' and '{}' for document",
            doc_id,
            raw_index
        );

        // Delete the metadata from library_metadata
        self.delete(doc_id).await?;

        Ok(())
    }

    /// Get library document count
    pub async fn count(&self) -> Result<u64> {
        let index = self.client.index(INDEX_LIBRARY_METADATA);
        let stats = index.get_stats().await?;
        Ok(stats.number_of_documents as u64)
    }

    /// Get document count for an index (helper)
    async fn document_count(&self, index_name: &str) -> Result<u64> {
        let index = self.client.index(index_name);
        let stats = index.get_stats().await?;
        Ok(stats.number_of_documents as u64)
    }

    /// Rebuild library metadata from existing content indices.
    ///
    /// Scans all content indices for unique sources and creates metadata entries
    /// for any sources that don't already have entries in library_metadata.
    /// Derives page_count from max(page_number) in chunks.
    pub async fn rebuild_metadata(&self) -> Result<Vec<LibraryDocumentMetadata>> {
        let content_indices = all_indexes();
        // Track: (index_name, chunk_count, char_count, max_page_number)
        let mut discovered_sources: HashMap<String, (String, u32, u64, u32)> = HashMap::new();

        // Get existing library documents to avoid duplicates
        let existing = match self.list().await {
            Ok(docs) => docs,
            Err(e) => {
                log::error!("Failed to list existing library documents during rebuild: {}", e);
                return Err(e);
            }
        };
        let existing_names: HashSet<String> = existing.iter().map(|d| d.name.clone()).collect();

        log::info!(
            "Rebuilding library metadata, {} existing entries",
            existing.len()
        );

        for index_name in content_indices {
            // Check if index exists
            if self.document_count(index_name).await.unwrap_or(0) == 0 {
                continue;
            }

            log::info!("Scanning index '{}' for sources...", index_name);

            // Get all unique sources by querying with facets
            // Meilisearch doesn't have direct facet aggregation, so we'll paginate through docs
            let index = self.client.index(index_name);
            let mut offset = 0;
            let limit = 1000;

            loop {
                let results: SearchResults<SearchDocument> = index
                    .search()
                    .with_query("")
                    .with_limit(limit)
                    .with_offset(offset)
                    .execute()
                    .await?;

                if results.hits.is_empty() {
                    break;
                }

                for hit in results.hits {
                    let doc = hit.result;
                    let entry = discovered_sources
                        .entry(doc.source.clone())
                        .or_insert((index_name.to_string(), 0, 0, 0));
                    entry.1 += 1; // chunk count
                    entry.2 += doc.content.len() as u64; // character count
                    // Track max page number to estimate total pages
                    if let Some(page_num) = doc.page_number {
                        entry.3 = entry.3.max(page_num);
                    }
                }

                offset += limit;

                // Safety limit
                if offset > 50000 {
                    log::warn!("Reached safety limit scanning index '{}'", index_name);
                    break;
                }
            }
        }

        log::info!(
            "Found {} unique sources across all indices",
            discovered_sources.len()
        );

        // Create metadata for new sources
        let mut created = Vec::new();
        for (source_name, (index_name, chunk_count, char_count, max_page)) in discovered_sources {
            if existing_names.contains(&source_name) {
                log::debug!("Skipping existing source: {}", source_name);
                continue;
            }

            // Infer source type from index or filename
            let source_type = if index_name == INDEX_RULES {
                "rulebook"
            } else if index_name == INDEX_FICTION {
                "fiction"
            } else if index_name == INDEX_CHAT {
                "chat"
            } else if source_name.to_lowercase().ends_with(".pdf") {
                "rulebook" // PDFs are typically rulebooks
            } else if source_name.to_lowercase().ends_with(".epub") {
                "fiction" // EPUBs are typically fiction
            } else {
                "documents"
            }
            .to_string();

            // Derive page count from max page number seen in chunks
            // If no page numbers found, estimate from chunk count (assuming ~4 chunks/page)
            let page_count = if max_page > 0 {
                max_page
            } else {
                (chunk_count / 4).max(1)
            };

            let metadata = LibraryDocumentMetadata {
                id: uuid::Uuid::new_v4().to_string(),
                name: source_name.clone(),
                source_type,
                file_path: None, // Unknown for legacy docs
                page_count,
                chunk_count,
                character_count: char_count,
                content_index: index_name,
                status: "ready".to_string(),
                error_message: None,
                ingested_at: chrono::Utc::now().to_rfc3339(),
                // TTRPG metadata - user-editable, not set during ingestion
                game_system: None,
                setting: None,
                content_type: None,
                publisher: None,
            };

            if let Err(e) = self.save(&metadata).await {
                log::warn!("Failed to save metadata for '{}': {}", source_name, e);
            } else {
                log::info!("Created metadata for legacy source: {}", source_name);
                created.push(metadata);
            }
        }

        log::info!("Created {} new library metadata entries", created.len());
        Ok(created)
    }
}
