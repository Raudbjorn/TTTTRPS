use crate::core::models::ContentChunk;
use uuid::Uuid;
use std::collections::HashMap;

pub struct SemanticChunker {
    max_chunk_size: usize,
    overlap: usize,
}

impl SemanticChunker {
    pub fn new(max_chunk_size: usize, overlap: usize) -> Self {
        Self {
            max_chunk_size,
            overlap,
        }
    }

    pub fn chunk_text(&self, text: &str, source_id: &str) -> Vec<ContentChunk> {
        // Basic sliding window for now.
        // True semantic chunking would use embeddings to find topic shifts.
        // For MVP, we split by paragraphs and aggregate.

        let mut chunks = Vec::new();
        let paragraphs: Vec<&str> = text.split("\n\n").collect();

        let mut current_chunk = String::new();
        let mut start_page = 1; // Simplification, need page awareness from PDF parser

        for para in paragraphs {
            if current_chunk.len() + para.len() > self.max_chunk_size {
                if !current_chunk.is_empty() {
                    chunks.push(self.create_chunk(current_chunk.clone(), source_id, start_page));
                    // Handle overlap
                    let overlap_start = current_chunk.len().saturating_sub(self.overlap);
                    current_chunk = current_chunk[overlap_start..].to_string();
                }
            }
            current_chunk.push_str(para);
            current_chunk.push_str("\n\n");
        }

        if !current_chunk.trim().is_empty() {
            chunks.push(self.create_chunk(current_chunk, source_id, start_page));
        }

        chunks
    }

    fn create_chunk(&self, content: String, source_id: &str, page: i32) -> ContentChunk {
        ContentChunk {
            id: Uuid::new_v4().to_string(),
            source_id: source_id.to_string(),
            content: content.trim().to_string(),
            page_number: page,
            section: None,
            chunk_type: "text".to_string(),
            metadata: HashMap::new(),
        }
    }
}
