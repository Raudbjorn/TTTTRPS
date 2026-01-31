//! Tauri events for voice synthesis queue.

use serde::Serialize;

use super::types::{JobProgress, JobStatus, QueueStats};

// ============================================================================
// Queue Events (Tauri)
// ============================================================================

/// Event emitted when a job is submitted
#[derive(Debug, Clone, Serialize)]
pub struct JobSubmittedEvent {
    pub job_id: String,
    pub priority: String,
    pub text_preview: String,
    pub char_count: usize,
}

/// Event emitted when job status changes
#[derive(Debug, Clone, Serialize)]
pub struct JobStatusEvent {
    pub job_id: String,
    pub status: JobStatus,
    pub progress: JobProgress,
    pub result_path: Option<String>,
    pub error: Option<String>,
}

/// Event emitted for queue statistics updates
#[derive(Debug, Clone, Serialize)]
pub struct QueueStatsEvent {
    pub stats: QueueStats,
}

/// Event channel names
pub mod channels {
    pub const JOB_SUBMITTED: &str = "synthesis:job-submitted";
    pub const JOB_STARTED: &str = "synthesis:job-started";
    pub const JOB_PROGRESS: &str = "synthesis:job-progress";
    pub const JOB_COMPLETED: &str = "synthesis:job-completed";
    pub const JOB_FAILED: &str = "synthesis:job-failed";
    pub const JOB_CANCELED: &str = "synthesis:job-canceled";
    pub const QUEUE_STATS: &str = "synthesis:queue-stats";
    pub const QUEUE_PAUSED: &str = "synthesis:queue-paused";
    pub const QUEUE_RESUMED: &str = "synthesis:queue-resumed";
}
