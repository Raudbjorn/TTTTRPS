//! Events for voice synthesis queue.

use serde::Serialize;

use super::types::{JobProgress, JobStatus, QueueStats};

// ============================================================================
// Queue Event Emitter Trait
// ============================================================================

/// Trait for emitting queue events (replaces direct Tauri AppHandle dependency).
///
/// Implementations can forward events to TUI widgets, log them, or ignore them.
pub trait QueueEventEmitter: Send + Sync {
    fn emit_json(&self, channel: &str, payload: serde_json::Value);
}

/// Helper to emit typed events through the trait
pub fn emit_event(emitter: Option<&dyn QueueEventEmitter>, channel: &str, payload: &impl Serialize) {
    if let Some(e) = emitter {
        if let Ok(value) = serde_json::to_value(payload) {
            e.emit_json(channel, value);
        }
    }
}

/// A no-op emitter that discards all events (useful for tests and headless mode)
pub struct NoopEmitter;

impl QueueEventEmitter for NoopEmitter {
    fn emit_json(&self, _channel: &str, _payload: serde_json::Value) {}
}

// ============================================================================
// Queue Events
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
