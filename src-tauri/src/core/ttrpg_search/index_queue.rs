//! Index Queue Module
//!
//! Queue for Meilisearch indexing with retry logic when unavailable.

use serde_json::Value;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ============================================================================
// Types
// ============================================================================

/// A document pending indexing
#[derive(Debug, Clone)]
pub struct PendingDocument {
    /// Document ID
    pub id: String,
    /// Document payload as JSON
    pub payload: Value,
    /// Number of indexing attempts
    pub attempts: u32,
    /// When this document was first queued
    pub created_at: Instant,
    /// Last attempt timestamp
    pub last_attempt: Option<Instant>,
}

impl PendingDocument {
    /// Create a new pending document
    pub fn new(id: String, payload: Value) -> Self {
        Self {
            id,
            payload,
            attempts: 0,
            created_at: Instant::now(),
            last_attempt: None,
        }
    }

    /// Check if this document has exceeded max retries
    pub fn exceeded_retries(&self, max_retries: u32) -> bool {
        self.attempts >= max_retries
    }

    /// Check if ready for retry based on delay
    pub fn ready_for_retry(&self, retry_delay: Duration) -> bool {
        match self.last_attempt {
            Some(last) => last.elapsed() >= retry_delay,
            None => true,
        }
    }

    /// Record an attempt
    pub fn record_attempt(&mut self) {
        self.attempts += 1;
        self.last_attempt = Some(Instant::now());
    }

    /// Get time since creation
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
}

// ============================================================================
// Index Queue
// ============================================================================

/// Thread-safe queue for documents pending indexing
#[derive(Clone)]
pub struct IndexQueue {
    /// Internal queue
    queue: Arc<Mutex<VecDeque<PendingDocument>>>,
    /// Maximum retry attempts
    max_retries: u32,
    /// Delay between retries
    retry_delay: Duration,
    /// Maximum queue size (for backpressure)
    max_size: usize,
}

impl Default for IndexQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl IndexQueue {
    /// Create a new index queue with default settings
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            max_retries: 5,
            retry_delay: Duration::from_secs(30),
            max_size: 10000,
        }
    }

    /// Create with custom settings
    pub fn with_config(max_retries: u32, retry_delay: Duration, max_size: usize) -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            max_retries,
            retry_delay,
            max_size,
        }
    }

    /// Enqueue a document for indexing
    ///
    /// # Returns
    /// * Ok(()) if enqueued
    /// * Err(()) if queue is full
    pub fn enqueue(&self, id: String, payload: Value) -> Result<(), ()> {
        let mut queue = self.queue.lock().unwrap();

        if queue.len() >= self.max_size {
            return Err(());
        }

        queue.push_back(PendingDocument::new(id, payload));
        Ok(())
    }

    /// Dequeue a document ready for processing
    ///
    /// Only returns documents that are ready for retry.
    ///
    /// # Performance Note
    ///
    /// Current implementation is O(N) where N is the queue size. For large queues
    /// with many documents waiting for retry, this could become a bottleneck.
    ///
    /// Potential optimization: Use two data structures:
    /// - A `VecDeque` for new/ready documents (O(1) dequeue from front)
    /// - A `BinaryHeap` or sorted structure for documents with retry delays,
    ///   ordered by next-retry-time for O(log N) insertion and O(1) peek
    ///
    /// For typical usage with <1000 documents, the current O(N) is acceptable.
    pub fn dequeue(&self) -> Option<PendingDocument> {
        let mut queue = self.queue.lock().unwrap();

        // Find first document ready for retry (O(N) scan)
        let pos = queue.iter().position(|doc| {
            !doc.exceeded_retries(self.max_retries) && doc.ready_for_retry(self.retry_delay)
        })?;

        queue.remove(pos)
    }

    /// Requeue a document after failed attempt
    ///
    /// Documents are always added back to the queue even if they exceed
    /// max retries. Use `drain_failed()` to remove failed documents.
    pub fn requeue(&self, mut doc: PendingDocument) {
        doc.record_attempt();
        let mut queue = self.queue.lock().unwrap();
        queue.push_back(doc);
    }

    /// Get current queue length
    pub fn len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.lock().unwrap().is_empty()
    }

    /// Get count of documents that have exceeded max retries
    pub fn failed_count(&self) -> usize {
        self.queue.lock().unwrap()
            .iter()
            .filter(|doc| doc.exceeded_retries(self.max_retries))
            .count()
    }

    /// Remove documents that have exceeded max retries
    ///
    /// # Returns
    /// Vector of failed documents
    pub fn drain_failed(&self) -> Vec<PendingDocument> {
        let mut queue = self.queue.lock().unwrap();
        let max = self.max_retries;

        let (failed, remaining): (Vec<_>, Vec<_>) = queue
            .drain(..)
            .partition(|doc| doc.exceeded_retries(max));

        queue.extend(remaining);
        failed
    }

    /// Clear the entire queue
    pub fn clear(&self) {
        self.queue.lock().unwrap().clear();
    }

    /// Get statistics about the queue
    pub fn stats(&self) -> QueueStats {
        let queue = self.queue.lock().unwrap();

        let total = queue.len();
        let ready = queue.iter()
            .filter(|d| !d.exceeded_retries(self.max_retries) && d.ready_for_retry(self.retry_delay))
            .count();
        let pending = queue.iter()
            .filter(|d| !d.exceeded_retries(self.max_retries) && !d.ready_for_retry(self.retry_delay))
            .count();
        let failed = queue.iter()
            .filter(|d| d.exceeded_retries(self.max_retries))
            .count();

        let oldest_age = queue.iter()
            .map(|d| d.age())
            .max();

        QueueStats {
            total,
            ready,
            pending,
            failed,
            oldest_age,
        }
    }
}

/// Queue statistics
#[derive(Debug, Clone)]
pub struct QueueStats {
    /// Total documents in queue
    pub total: usize,
    /// Documents ready for retry
    pub ready: usize,
    /// Documents waiting for retry delay
    pub pending: usize,
    /// Documents that have exceeded max retries
    pub failed: usize,
    /// Age of oldest document
    pub oldest_age: Option<Duration>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_enqueue_dequeue() {
        let queue = IndexQueue::new();

        queue.enqueue("doc1".to_string(), json!({"content": "test"})).unwrap();
        assert_eq!(queue.len(), 1);

        let doc = queue.dequeue().unwrap();
        assert_eq!(doc.id, "doc1");
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_requeue_increments_attempts() {
        // Use short retry delay so we can immediately dequeue after requeue
        let queue = IndexQueue::with_config(5, Duration::from_millis(1), 100);

        queue.enqueue("doc1".to_string(), json!({})).unwrap();
        let doc = queue.dequeue().unwrap();
        assert_eq!(doc.attempts, 0);

        // requeue() calls record_attempt() internally
        queue.requeue(doc);

        // Wait for retry delay
        std::thread::sleep(Duration::from_millis(5));

        let doc = queue.dequeue().unwrap();
        assert_eq!(doc.attempts, 1);
    }

    #[test]
    fn test_retry_delay() {
        let queue = IndexQueue::with_config(5, Duration::from_millis(100), 100);

        queue.enqueue("doc1".to_string(), json!({})).unwrap();
        let doc = queue.dequeue().unwrap();

        // requeue() calls record_attempt() internally, which sets last_attempt
        queue.requeue(doc);

        // Should not be ready immediately (retry delay hasn't passed)
        let result = queue.dequeue();
        assert!(result.is_none() || result.as_ref().map(|d| d.id.as_str()) != Some("doc1"));

        // Wait for delay
        std::thread::sleep(Duration::from_millis(150));

        // Should be ready now
        let doc = queue.dequeue();
        assert!(doc.is_some());
    }

    #[test]
    fn test_max_retries() {
        let queue = IndexQueue::with_config(2, Duration::from_millis(1), 100);

        queue.enqueue("doc1".to_string(), json!({})).unwrap();

        // Simulate two failed attempts (requeue() calls record_attempt() internally)
        for _ in 0..2 {
            std::thread::sleep(Duration::from_millis(5));
            if let Some(doc) = queue.dequeue() {
                queue.requeue(doc);
            }
        }

        std::thread::sleep(Duration::from_millis(5));

        // After max retries, should not be dequeued
        let doc = queue.dequeue();
        assert!(doc.is_none());

        // But should be in failed count
        assert_eq!(queue.failed_count(), 1);
    }

    #[test]
    fn test_queue_full() {
        let queue = IndexQueue::with_config(5, Duration::from_secs(30), 2);

        assert!(queue.enqueue("doc1".to_string(), json!({})).is_ok());
        assert!(queue.enqueue("doc2".to_string(), json!({})).is_ok());
        assert!(queue.enqueue("doc3".to_string(), json!({})).is_err());
    }

    #[test]
    fn test_drain_failed() {
        // max_retries=1 means after 1 requeue (1 attempt), doc is failed
        let queue = IndexQueue::with_config(1, Duration::from_millis(1), 100);

        queue.enqueue("doc1".to_string(), json!({})).unwrap();

        // Fail the document (requeue() calls record_attempt() internally)
        std::thread::sleep(Duration::from_millis(5));
        if let Some(doc) = queue.dequeue() {
            queue.requeue(doc);
        }

        let failed = queue.drain_failed();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].id, "doc1");
        assert!(queue.is_empty());
    }

    #[test]
    fn test_stats() {
        let queue = IndexQueue::with_config(5, Duration::from_secs(30), 100);

        queue.enqueue("doc1".to_string(), json!({})).unwrap();
        queue.enqueue("doc2".to_string(), json!({})).unwrap();

        let stats = queue.stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.ready, 2);
        assert_eq!(stats.failed, 0);
    }
}
