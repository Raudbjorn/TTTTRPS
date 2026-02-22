//! Tests for the voice synthesis queue.

use super::*;
use crate::core::voice::types::VoiceProviderType;

fn create_test_job(text: &str) -> SynthesisJob {
    SynthesisJob::new(text, "profile-1", VoiceProviderType::OpenAI, "alloy")
}

#[tokio::test]
async fn test_submit_job() {
    let queue = SynthesisQueue::with_defaults();

    let job = create_test_job("Hello world");
    let job_id = queue.submit(job, None).await.unwrap();

    assert!(!job_id.is_empty());

    let retrieved = queue.get_job(&job_id).await.unwrap();
    assert_eq!(retrieved.text, "Hello world");
    assert_eq!(retrieved.status, JobStatus::Pending);
}

#[tokio::test]
async fn test_priority_ordering() {
    let queue = SynthesisQueue::with_defaults();

    // Submit low priority first
    let low = create_test_job("Low priority").with_priority(JobPriority::Low);
    let low_id = queue.submit(low, None).await.unwrap();

    // Submit high priority second
    let high = create_test_job("High priority").with_priority(JobPriority::High);
    let high_id = queue.submit(high, None).await.unwrap();

    // Submit immediate priority third
    let immediate = create_test_job("Immediate priority").with_priority(JobPriority::Immediate);
    let immediate_id = queue.submit(immediate, None).await.unwrap();

    // Next job should be immediate priority
    let next = queue.next_job().await.unwrap();
    assert_eq!(next.id, immediate_id);

    // Then high priority
    let next = queue.next_job().await.unwrap();
    assert_eq!(next.id, high_id);

    // Then low priority
    let next = queue.next_job().await.unwrap();
    assert_eq!(next.id, low_id);
}

#[tokio::test]
async fn test_cancel_job() {
    let queue = SynthesisQueue::with_defaults();

    let job = create_test_job("To be canceled");
    let job_id = queue.submit(job, None).await.unwrap();

    queue.cancel(&job_id, None).await.unwrap();

    let status = queue.get_status(&job_id).await.unwrap();
    assert!(matches!(status, JobStatus::Canceled));
}

#[tokio::test]
async fn test_pause_resume() {
    let queue = SynthesisQueue::with_defaults();

    let job = create_test_job("Test");
    queue.submit(job, None).await.unwrap();

    // Pause queue
    queue.pause(None).await;
    assert!(queue.is_paused().await);

    // Should not return jobs when paused
    let next = queue.next_job().await;
    assert!(next.is_none());

    // Resume queue
    queue.resume(None).await;
    assert!(!queue.is_paused().await);

    // Should return job now
    let next = queue.next_job().await;
    assert!(next.is_some());
}

#[tokio::test]
async fn test_queue_stats() {
    let queue = SynthesisQueue::with_defaults();

    for i in 0..5 {
        let job = create_test_job(&format!("Job {}", i));
        queue.submit(job, None).await.unwrap();
    }

    let stats = queue.stats().await;
    assert_eq!(stats.total_submitted, 5);
    assert_eq!(stats.pending_count, 5);
}

#[tokio::test]
async fn test_session_batch() {
    let queue = SynthesisQueue::with_defaults();

    let texts = vec![
        (
            "Hello".to_string(),
            "profile-1".to_string(),
            "alloy".to_string(),
        ),
        (
            "World".to_string(),
            "profile-2".to_string(),
            "echo".to_string(),
        ),
    ];

    let job_ids = queue
        .pregen_session("session-123", texts, VoiceProviderType::OpenAI, None)
        .await
        .unwrap();

    assert_eq!(job_ids.len(), 2);

    // Check session filter
    let session_jobs = queue.list_by_session("session-123").await;
    assert_eq!(session_jobs.len(), 2);
}

#[tokio::test]
async fn test_job_lifecycle() {
    let queue = SynthesisQueue::with_defaults();

    let job = create_test_job("Lifecycle test");
    let job_id = queue.submit(job, None).await.unwrap();

    // Get next job (simulating worker)
    let _job = queue.next_job().await.unwrap();

    // Start processing
    queue.mark_started(&job_id, None).await.unwrap();
    let status = queue.get_status(&job_id).await.unwrap();
    assert!(matches!(status, JobStatus::Processing));

    // Update progress
    queue
        .update_progress(&job_id, 0.5, "Synthesizing", None)
        .await
        .unwrap();
    let progress = queue.get_progress(&job_id).await.unwrap();
    assert_eq!(progress.progress, 0.5);
    assert_eq!(progress.stage, "Synthesizing");

    // Complete
    queue
        .mark_completed(&job_id, "/path/to/audio.mp3", None)
        .await
        .unwrap();
    let status = queue.get_status(&job_id).await.unwrap();
    assert!(matches!(status, JobStatus::Completed));

    let job = queue.get_job(&job_id).await.unwrap();
    assert_eq!(job.result_path, Some("/path/to/audio.mp3".to_string()));
}

#[tokio::test]
async fn test_job_with_tags() {
    let queue = SynthesisQueue::with_defaults();

    let job = create_test_job("Tagged job")
        .for_campaign("campaign-1")
        .for_session("session-1")
        .for_npc("npc-1")
        .with_tag("custom-tag");

    let job_id = queue.submit(job, None).await.unwrap();

    let retrieved = queue.get_job(&job_id).await.unwrap();
    assert!(retrieved
        .tags
        .contains(&"campaign:campaign-1".to_string()));
    assert!(retrieved.tags.contains(&"session:session-1".to_string()));
    assert!(retrieved.tags.contains(&"npc:npc-1".to_string()));
    assert!(retrieved.tags.contains(&"custom-tag".to_string()));
}

#[tokio::test]
async fn test_max_queue_size() {
    let config = QueueConfig {
        max_queue_size: 2,
        ..Default::default()
    };
    let queue = SynthesisQueue::new(config);

    // Fill queue
    queue.submit(create_test_job("Job 1"), None).await.unwrap();
    queue.submit(create_test_job("Job 2"), None).await.unwrap();

    // Third should fail
    let result = queue.submit(create_test_job("Job 3"), None).await;
    assert!(matches!(result, Err(QueueError::QueueFull)));
}

#[tokio::test]
async fn test_clear_history() {
    let queue = SynthesisQueue::with_defaults();

    let job = create_test_job("Test");
    let job_id = queue.submit(job, None).await.unwrap();

    // Simulate completion
    let _job = queue.next_job().await.unwrap();
    queue.mark_started(&job_id, None).await.unwrap();
    queue
        .mark_completed(&job_id, "/path/to/audio.mp3", None)
        .await
        .unwrap();

    // History should have the job
    let history = queue.list_history(None).await;
    assert_eq!(history.len(), 1);

    // Clear history
    queue.clear_history().await;

    let history = queue.list_history(None).await;
    assert_eq!(history.len(), 0);
}
