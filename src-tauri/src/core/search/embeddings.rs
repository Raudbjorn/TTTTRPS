//! Embedding Provider Module
//!
//! Defines the EmbeddingProvider trait and embedding cache for vector search.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::RwLock;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum EmbeddingError {
    #[error("Provider not configured: {0}")]
    NotConfigured(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Rate limited: retry after {0} seconds")]
    RateLimited(u64),

    #[error("Cache error: {0}")]
    CacheError(String),
}

pub type Result<T> = std::result::Result<T, EmbeddingError>;

// ============================================================================
// Embedding Configuration
// ============================================================================

/// Configuration for embedding providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Provider type: "ollama", "openai"
    pub provider: String,
    /// Model name for embeddings
    pub model: String,
    /// API endpoint (for Ollama or custom OpenAI)
    pub endpoint: Option<String>,
    /// API key (for OpenAI)
    pub api_key: Option<String>,
    /// Embedding dimensions (for validation)
    pub dimensions: Option<usize>,
    /// Batch size for processing
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

fn default_batch_size() -> usize {
    32
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            model: "nomic-embed-text".to_string(),
            endpoint: Some("http://localhost:11434".to_string()),
            api_key: None,
            dimensions: Some(768),
            batch_size: 32,
        }
    }
}

// ============================================================================
// Embedding Provider Trait
// ============================================================================

/// Trait for embedding providers
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embedding for a single text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate embeddings for multiple texts (batch)
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    /// Get the embedding dimensions
    fn dimensions(&self) -> usize;

    /// Get provider name
    fn name(&self) -> &str;

    /// Check if provider is healthy/available
    async fn health_check(&self) -> bool;
}

// ============================================================================
// Embedding Cache
// ============================================================================

/// Cache entry for embeddings
#[derive(Clone)]
struct CacheEntry {
    embedding: Vec<f32>,
    created_at: Instant,
    access_count: u32,
}

/// LRU cache for embeddings
pub struct EmbeddingCache {
    cache: RwLock<HashMap<String, CacheEntry>>,
    max_entries: usize,
    ttl: Duration,
    persist_path: Option<PathBuf>,
}

impl EmbeddingCache {
    /// Create a new embedding cache
    pub fn new(max_entries: usize, ttl_seconds: u64) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            max_entries,
            ttl: Duration::from_secs(ttl_seconds),
            persist_path: None,
        }
    }

    /// Create cache with persistence
    pub fn with_persistence(max_entries: usize, ttl_seconds: u64, path: PathBuf) -> Self {
        let mut cache = Self::new(max_entries, ttl_seconds);
        cache.persist_path = Some(path);
        cache
    }

    /// Compute cache key from text
    fn cache_key(text: &str, model: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        model.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Get embedding from cache
    pub async fn get(&self, text: &str, model: &str) -> Option<Vec<f32>> {
        let key = Self::cache_key(text, model);
        let cache = self.cache.read().await;

        if let Some(entry) = cache.get(&key) {
            if entry.created_at.elapsed() < self.ttl {
                return Some(entry.embedding.clone());
            }
        }
        None
    }

    /// Store embedding in cache
    pub async fn set(&self, text: &str, model: &str, embedding: Vec<f32>) {
        let key = Self::cache_key(text, model);
        let mut cache = self.cache.write().await;

        // Evict if at capacity
        if cache.len() >= self.max_entries {
            self.evict_lru(&mut cache);
        }

        cache.insert(
            key,
            CacheEntry {
                embedding,
                created_at: Instant::now(),
                access_count: 0,
            },
        );
    }

    /// Get or compute embedding
    pub async fn get_or_compute<F, Fut>(
        &self,
        text: &str,
        model: &str,
        compute: F,
    ) -> Result<Vec<f32>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Vec<f32>>>,
    {
        // Check cache first
        if let Some(embedding) = self.get(text, model).await {
            return Ok(embedding);
        }

        // Compute and cache
        let embedding = compute().await?;
        self.set(text, model, embedding.clone()).await;
        Ok(embedding)
    }

    /// Evict oldest entries using LRU
    fn evict_lru(&self, cache: &mut HashMap<String, CacheEntry>) {
        let to_remove = cache.len() / 4; // Remove 25%

        let mut entries: Vec<_> = cache.iter().collect();
        entries.sort_by(|a, b| {
            // Sort by access count (ascending), then by age (oldest first)
            a.1.access_count
                .cmp(&b.1.access_count)
                .then(b.1.created_at.cmp(&a.1.created_at))
        });

        let keys_to_remove: Vec<String> = entries.into_iter()
            .take(to_remove)
            .map(|(k, _)| k.clone())
            .collect();

        for key in keys_to_remove {
            cache.remove(&key);
        }
    }

    /// Clear expired entries
    pub async fn clear_expired(&self) {
        let mut cache = self.cache.write().await;
        cache.retain(|_, entry| entry.created_at.elapsed() < self.ttl);
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let expired = cache
            .values()
            .filter(|e| e.created_at.elapsed() >= self.ttl)
            .count();

        CacheStats {
            total_entries: cache.len(),
            expired_entries: expired,
            max_entries: self.max_entries,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub max_entries: usize,
}

// ============================================================================
// Cached Embedding Provider Wrapper
// ============================================================================

/// Wrapper that adds caching to any embedding provider
pub struct CachedEmbeddingProvider {
    provider: Arc<dyn EmbeddingProvider>,
    cache: Arc<EmbeddingCache>,
}

impl CachedEmbeddingProvider {
    pub fn new(provider: Arc<dyn EmbeddingProvider>, cache: Arc<EmbeddingCache>) -> Self {
        Self { provider, cache }
    }
}

#[async_trait]
impl EmbeddingProvider for CachedEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let provider = self.provider.clone();
        let text_owned = text.to_string();

        self.cache
            .get_or_compute(text, self.provider.name(), || async move {
                provider.embed(&text_owned).await
            })
            .await
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        let mut uncached_indices = Vec::new();
        let mut uncached_texts = Vec::new();

        // Check cache for each text
        for (i, text) in texts.iter().enumerate() {
            if let Some(embedding) = self.cache.get(text, self.provider.name()).await {
                results.push(Some(embedding));
            } else {
                results.push(None);
                uncached_indices.push(i);
                uncached_texts.push(*text);
            }
        }

        // Compute uncached embeddings in batch
        if !uncached_texts.is_empty() {
            let uncached_refs: Vec<&str> = uncached_texts.to_vec();
            let computed = self.provider.embed_batch(&uncached_refs).await?;

            // Cache and fill in results
            for (idx, (text, embedding)) in uncached_indices
                .into_iter()
                .zip(uncached_texts.into_iter().zip(computed.into_iter()))
            {
                self.cache.set(text, self.provider.name(), embedding.clone()).await;
                results[idx] = Some(embedding);
            }
        }

        // Unwrap all results (all should be Some now)
        Ok(results.into_iter().map(|r| r.unwrap()).collect())
    }

    fn dimensions(&self) -> usize {
        self.provider.dimensions()
    }

    fn name(&self) -> &str {
        self.provider.name()
    }

    async fn health_check(&self) -> bool {
        self.provider.health_check().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_key_generation() {
        let key1 = EmbeddingCache::cache_key("hello world", "nomic-embed-text");
        let key2 = EmbeddingCache::cache_key("hello world", "nomic-embed-text");
        let key3 = EmbeddingCache::cache_key("different text", "nomic-embed-text");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[tokio::test]
    async fn test_cache_set_get() {
        let cache = EmbeddingCache::new(100, 3600);
        let embedding = vec![1.0, 2.0, 3.0];

        cache.set("test text", "model", embedding.clone()).await;
        let retrieved = cache.get("test text", "model").await;

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), embedding);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = EmbeddingCache::new(100, 3600);
        let result = cache.get("nonexistent", "model").await;
        assert!(result.is_none());
    }
}
