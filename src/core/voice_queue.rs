//! Voice Pre-generation Queue Module
//!
//! Manages a queue for pre-generating voice audio files to reduce latency.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

// ============================================================================
// Types
// ============================================================================

/// Voice generation job status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Waiting in queue
    Pending,
    /// Currently being processed
    Processing,
    /// Successfully completed
    Completed,
    /// Failed with error
    Failed,
    /// Canceled by user
    #[serde(alias = "Cancelled")]
    Canceled,
}

/// Priority level for jobs
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobPriority {
    /// Background pre-generation
    Low,
    /// Normal priority
    Normal,
    /// User-requested
    High,
    /// Immediate (skip queue)
    Urgent,
}

/// Voice generation job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceJob {
    /// Unique job ID
    pub id: String,
    /// Text to synthesize
    pub text: String,
    /// Voice/speaker ID
    pub voice_id: String,
    /// Job priority
    pub priority: JobPriority,
    /// Current status
    pub status: JobStatus,
    /// Optional campaign context
    pub campaign_id: Option<String>,
    /// Optional NPC context
    pub npc_id: Option<String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Started processing timestamp
    pub started_at: Option<DateTime<Utc>>,
    /// Completed timestamp
    pub completed_at: Option<DateTime<Utc>>,
    /// Result path (if completed)
    pub result_path: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Retry count
    pub retry_count: u32,
    /// Max retries
    pub max_retries: u32,
}

impl VoiceJob {
    pub fn new(text: &str, voice_id: &str, priority: JobPriority) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            text: text.to_string(),
            voice_id: voice_id.to_string(),
            priority,
            status: JobStatus::Pending,
            campaign_id: None,
            npc_id: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            result_path: None,
            error: None,
            retry_count: 0,
            max_retries: 3,
        }
    }

    pub fn with_context(mut self, campaign_id: Option<&str>, npc_id: Option<&str>) -> Self {
        self.campaign_id = campaign_id.map(|s| s.to_string());
        self.npc_id = npc_id.map(|s| s.to_string());
        self
    }
}

/// Queue statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueueStats {
    /// Total jobs processed
    pub total_processed: u64,
    /// Jobs completed successfully
    pub total_completed: u64,
    /// Jobs that failed
    pub total_failed: u64,
    /// Current queue depth
    pub queue_depth: usize,
    /// Jobs currently processing
    pub processing_count: usize,
    /// Average processing time (ms)
    pub avg_processing_time_ms: f64,
    /// Cache hit rate
    pub cache_hit_rate: f64,
}

/// Cached audio result
#[derive(Debug, Clone)]
pub struct CachedAudio {
    pub text_hash: String,
    pub voice_id: String,
    pub path: String,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u32,
}

// ============================================================================
// Voice Queue
// ============================================================================

/// Pre-generation queue for voice synthesis
pub struct VoiceQueue {
    /// Priority queues (one per priority level)
    queues: RwLock<HashMap<JobPriority, VecDeque<VoiceJob>>>,
    /// Jobs by ID for lookup
    jobs: RwLock<HashMap<String, VoiceJob>>,
    /// Audio cache (text_hash -> cached audio)
    cache: RwLock<HashMap<String, CachedAudio>>,
    /// Queue statistics
    stats: RwLock<QueueStats>,
    /// Maximum queue size per priority
    max_queue_size: usize,
    /// Maximum cache size
    max_cache_size: usize,
    /// Processing times for averaging
    processing_times: RwLock<VecDeque<u64>>,
}

impl VoiceQueue {
    pub fn new() -> Self {
        let mut queues = HashMap::new();
        queues.insert(JobPriority::Low, VecDeque::new());
        queues.insert(JobPriority::Normal, VecDeque::new());
        queues.insert(JobPriority::High, VecDeque::new());
        queues.insert(JobPriority::Urgent, VecDeque::new());

        Self {
            queues: RwLock::new(queues),
            jobs: RwLock::new(HashMap::new()),
            cache: RwLock::new(HashMap::new()),
            stats: RwLock::new(QueueStats::default()),
            max_queue_size: 1000,
            max_cache_size: 500,
            processing_times: RwLock::new(VecDeque::with_capacity(100)),
        }
    }

    /// Enqueue a new voice generation job
    pub fn enqueue(&self, job: VoiceJob) -> Result<String, String> {
        // Check cache first
        let cache_key = self.compute_cache_key(&job.text, &job.voice_id);
        if let Some(cached) = self.get_cached(&cache_key) {
            // Update cache hit stats
            let mut stats = self.stats.write().unwrap();
            let total = stats.total_completed + stats.total_failed + 1;
            stats.cache_hit_rate = (stats.cache_hit_rate * (total - 1) as f64 + 1.0) / total as f64;

            return Ok(cached.path);
        }

        // Check queue size
        {
            let queues = self.queues.read().unwrap();
            if let Some(queue) = queues.get(&job.priority) {
                if queue.len() >= self.max_queue_size {
                    return Err("Queue is full".to_string());
                }
            }
        }

        let job_id = job.id.clone();

        // Add to queue
        {
            let mut queues = self.queues.write().unwrap();
            if let Some(queue) = queues.get_mut(&job.priority) {
                queue.push_back(job.clone());
            }
        }

        // Track job
        {
            let mut jobs = self.jobs.write().unwrap();
            jobs.insert(job_id.clone(), job);
        }

        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.queue_depth += 1;
        }

        Ok(job_id)
    }

    /// Get the next job to process (highest priority first)
    pub fn dequeue(&self) -> Option<VoiceJob> {
        let mut queues = self.queues.write().unwrap();

        // Check priorities from highest to lowest
        for priority in &[JobPriority::Urgent, JobPriority::High, JobPriority::Normal, JobPriority::Low] {
            if let Some(queue) = queues.get_mut(priority) {
                if let Some(mut job) = queue.pop_front() {
                    job.status = JobStatus::Processing;
                    job.started_at = Some(Utc::now());

                    // Update tracked job
                    {
                        let mut jobs = self.jobs.write().unwrap();
                        jobs.insert(job.id.clone(), job.clone());
                    }

                    // Update stats
                    {
                        let mut stats = self.stats.write().unwrap();
                        stats.queue_depth = stats.queue_depth.saturating_sub(1);
                        stats.processing_count += 1;
                    }

                    return Some(job);
                }
            }
        }

        None
    }

    /// Mark a job as completed
    pub fn complete(&self, job_id: &str, result_path: &str) {
        let mut jobs = self.jobs.write().unwrap();

        if let Some(job) = jobs.get_mut(job_id) {
            let now = Utc::now();
            let processing_time = job.started_at
                .map(|started| (now - started).num_milliseconds() as u64)
                .unwrap_or(0);

            job.status = JobStatus::Completed;
            job.completed_at = Some(now);
            job.result_path = Some(result_path.to_string());

            // Add to cache
            let cache_key = self.compute_cache_key(&job.text, &job.voice_id);
            self.add_to_cache(cache_key, &job.voice_id, result_path);

            // Update processing times
            {
                let mut times = self.processing_times.write().unwrap();
                times.push_back(processing_time);
                if times.len() > 100 {
                    times.pop_front();
                }
            }

            // Update stats
            {
                let mut stats = self.stats.write().unwrap();
                stats.total_processed += 1;
                stats.total_completed += 1;
                stats.processing_count = stats.processing_count.saturating_sub(1);

                // Recalculate average processing time
                let times = self.processing_times.read().unwrap();
                if !times.is_empty() {
                    stats.avg_processing_time_ms = times.iter().sum::<u64>() as f64 / times.len() as f64;
                }
            }
        }
    }

    /// Mark a job as failed
    pub fn fail(&self, job_id: &str, error: &str) {
        let mut jobs = self.jobs.write().unwrap();

        if let Some(job) = jobs.get_mut(job_id) {
            job.retry_count += 1;

            if job.retry_count < job.max_retries {
                // Re-queue for retry
                job.status = JobStatus::Pending;
                job.started_at = None;

                let mut queues = self.queues.write().unwrap();
                if let Some(queue) = queues.get_mut(&job.priority) {
                    queue.push_back(job.clone());
                }

                let mut stats = self.stats.write().unwrap();
                stats.queue_depth += 1;
                stats.processing_count = stats.processing_count.saturating_sub(1);
            } else {
                // Max retries exceeded
                job.status = JobStatus::Failed;
                job.completed_at = Some(Utc::now());
                job.error = Some(error.to_string());

                let mut stats = self.stats.write().unwrap();
                stats.total_processed += 1;
                stats.total_failed += 1;
                stats.processing_count = stats.processing_count.saturating_sub(1);
            }
        }
    }

    /// Cancel a job
    pub fn cancel(&self, job_id: &str) -> bool {
        let mut jobs = self.jobs.write().unwrap();

        if let Some(job) = jobs.get_mut(job_id) {
            if job.status == JobStatus::Pending {
                job.status = JobStatus::Canceled;
                job.completed_at = Some(Utc::now());

                // Remove from queue
                let mut queues = self.queues.write().unwrap();
                if let Some(queue) = queues.get_mut(&job.priority) {
                    queue.retain(|j| j.id != job_id);
                }

                let mut stats = self.stats.write().unwrap();
                stats.queue_depth = stats.queue_depth.saturating_sub(1);

                return true;
            }
        }

        false
    }

    /// Get job status
    pub fn get_job(&self, job_id: &str) -> Option<VoiceJob> {
        let jobs = self.jobs.read().unwrap();
        jobs.get(job_id).cloned()
    }

    /// Get queue statistics
    pub fn get_stats(&self) -> QueueStats {
        self.stats.read().unwrap().clone()
    }

    /// Get all pending jobs for a campaign
    pub fn get_campaign_jobs(&self, campaign_id: &str) -> Vec<VoiceJob> {
        let jobs = self.jobs.read().unwrap();
        jobs.values()
            .filter(|j| {
                j.campaign_id.as_deref() == Some(campaign_id)
                    && (j.status == JobStatus::Pending || j.status == JobStatus::Processing)
            })
            .cloned()
            .collect()
    }

    /// Pre-generate audio for common NPC phrases
    pub fn pregenerate_npc(
        &self,
        npc_id: &str,
        voice_id: &str,
        phrases: &[&str],
        campaign_id: Option<&str>,
    ) -> Vec<String> {
        phrases
            .iter()
            .filter_map(|phrase| {
                let job = VoiceJob::new(phrase, voice_id, JobPriority::Low)
                    .with_context(campaign_id, Some(npc_id));
                self.enqueue(job).ok()
            })
            .collect()
    }

    /// Compute cache key from text and voice
    fn compute_cache_key(&self, text: &str, voice_id: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        voice_id.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Get cached audio if available
    fn get_cached(&self, cache_key: &str) -> Option<CachedAudio> {
        let mut cache = self.cache.write().unwrap();

        if let Some(cached) = cache.get_mut(cache_key) {
            cached.last_accessed = Utc::now();
            cached.access_count += 1;
            return Some(cached.clone());
        }

        None
    }

    /// Add to cache
    fn add_to_cache(&self, cache_key: String, voice_id: &str, path: &str) {
        let mut cache = self.cache.write().unwrap();

        // Evict if necessary (LRU)
        if cache.len() >= self.max_cache_size {
            let lru_key = cache
                .iter()
                .min_by_key(|(_, v)| v.last_accessed)
                .map(|(k, _)| k.clone());

            if let Some(key) = lru_key {
                cache.remove(&key);
            }
        }

        let now = Utc::now();
        cache.insert(
            cache_key.clone(),
            CachedAudio {
                text_hash: cache_key,
                voice_id: voice_id.to_string(),
                path: path.to_string(),
                created_at: now,
                last_accessed: now,
                access_count: 1,
            },
        );
    }

    /// Clear expired cache entries
    pub fn cleanup_cache(&self, max_age_hours: i64) {
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours);
        let mut cache = self.cache.write().unwrap();
        cache.retain(|_, v| v.last_accessed > cutoff);
    }

    /// Clear all jobs (for testing or reset)
    pub fn clear(&self) {
        let mut queues = self.queues.write().unwrap();
        for queue in queues.values_mut() {
            queue.clear();
        }

        let mut jobs = self.jobs.write().unwrap();
        jobs.clear();

        let mut stats = self.stats.write().unwrap();
        *stats = QueueStats::default();
    }
}

impl Default for VoiceQueue {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Queue Worker (trait for integration)
// ============================================================================

/// Trait for voice synthesis backends
pub trait VoiceSynthesizer: Send + Sync {
    /// Synthesize text to audio file
    fn synthesize(&self, text: &str, voice_id: &str) -> Result<String, String>;
}

/// Queue worker that processes jobs
pub struct QueueWorker {
    queue: Arc<VoiceQueue>,
    synthesizer: Arc<dyn VoiceSynthesizer>,
    running: Arc<RwLock<bool>>,
}

impl QueueWorker {
    pub fn new(queue: Arc<VoiceQueue>, synthesizer: Arc<dyn VoiceSynthesizer>) -> Self {
        Self {
            queue,
            synthesizer,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start processing jobs
    pub fn start(&self) {
        let mut running = self.running.write().unwrap();
        *running = true;
    }

    /// Stop processing jobs
    pub fn stop(&self) {
        let mut running = self.running.write().unwrap();
        *running = false;
    }

    /// Process one job (call in a loop from a worker thread)
    pub fn process_one(&self) -> bool {
        if !*self.running.read().unwrap() {
            return false;
        }

        if let Some(job) = self.queue.dequeue() {
            match self.synthesizer.synthesize(&job.text, &job.voice_id) {
                Ok(path) => {
                    self.queue.complete(&job.id, &path);
                    true
                }
                Err(err) => {
                    self.queue.fail(&job.id, &err);
                    true
                }
            }
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enqueue_dequeue() {
        let queue = VoiceQueue::new();

        let job = VoiceJob::new("Hello world", "voice-1", JobPriority::Normal);
        let job_id = queue.enqueue(job).unwrap();

        assert!(!job_id.is_empty());

        let stats = queue.get_stats();
        assert_eq!(stats.queue_depth, 1);

        let dequeued = queue.dequeue().unwrap();
        assert_eq!(dequeued.text, "Hello world");
        assert_eq!(dequeued.status, JobStatus::Processing);
    }

    #[test]
    fn test_priority_ordering() {
        let queue = VoiceQueue::new();

        // Add low priority first
        queue.enqueue(VoiceJob::new("Low", "voice-1", JobPriority::Low)).unwrap();
        // Add high priority second
        queue.enqueue(VoiceJob::new("High", "voice-1", JobPriority::High)).unwrap();
        // Add normal priority third
        queue.enqueue(VoiceJob::new("Normal", "voice-1", JobPriority::Normal)).unwrap();

        // Should dequeue in priority order
        assert_eq!(queue.dequeue().unwrap().text, "High");
        assert_eq!(queue.dequeue().unwrap().text, "Normal");
        assert_eq!(queue.dequeue().unwrap().text, "Low");
    }

    #[test]
    fn test_complete_and_cache() {
        let queue = VoiceQueue::new();

        let job = VoiceJob::new("Test phrase", "voice-1", JobPriority::Normal);
        let job_id = queue.enqueue(job.clone()).unwrap();

        let dequeued = queue.dequeue().unwrap();
        queue.complete(&dequeued.id, "/path/to/audio.wav");

        let completed = queue.get_job(&job_id).unwrap();
        assert_eq!(completed.status, JobStatus::Completed);
        assert_eq!(completed.result_path, Some("/path/to/audio.wav".to_string()));

        // Enqueue same text again - should hit cache
        let job2 = VoiceJob::new("Test phrase", "voice-1", JobPriority::Normal);
        let result = queue.enqueue(job2);

        // Cache hit returns the path directly
        assert_eq!(result, Ok("/path/to/audio.wav".to_string()));
    }

    #[test]
    fn test_retry_on_failure() {
        let queue = VoiceQueue::new();

        let mut job = VoiceJob::new("Test", "voice-1", JobPriority::Normal);
        job.max_retries = 2;
        let job_id = queue.enqueue(job).unwrap();

        // First attempt
        queue.dequeue();
        queue.fail(&job_id, "Error 1");

        let job = queue.get_job(&job_id).unwrap();
        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(job.retry_count, 1);

        // Second attempt
        queue.dequeue();
        queue.fail(&job_id, "Error 2");

        let job = queue.get_job(&job_id).unwrap();
        assert_eq!(job.status, JobStatus::Failed);
        assert_eq!(job.retry_count, 2);
    }

    #[test]
    fn test_cancel() {
        let queue = VoiceQueue::new();

        let job = VoiceJob::new("Test", "voice-1", JobPriority::Normal);
        let job_id = queue.enqueue(job).unwrap();

        assert!(queue.cancel(&job_id));

        let job = queue.get_job(&job_id).unwrap();
        assert_eq!(job.status, JobStatus::Canceled);
    }
}
