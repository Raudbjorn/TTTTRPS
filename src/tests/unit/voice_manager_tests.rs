//! Voice Manager Unit Tests (Phase 2c - Comprehensive)
//!
//! Tests for voice profile CRUD operations, voice provider detection,
//! TTS queue management, voice caching, audio playback state, and
//! provider-specific error handling.
//!
//! This module tests both mock implementations (for isolated unit tests)
//! and integration with actual crate types where applicable.

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use std::collections::HashMap;
    use std::path::PathBuf;
    use chrono::Utc;

    // ============================================================================
    // SECTION 1: Voice Profile Management Tests
    // ============================================================================
    // Tests for voice profile CRUD operations using actual crate types

    mod voice_profile_tests {
        use super::*;
        use crate::core::voice::profiles::{
            AgeRange, Gender, ProfileMetadata, VoiceProfile, VoiceProfileManager, ProfileError,
        };
        use crate::core::voice::types::VoiceProviderType;

        #[test]
        fn test_voice_profile_creation() {
            let profile = VoiceProfile::new("Test Voice", VoiceProviderType::OpenAI, "alloy");

            assert!(!profile.id.is_empty(), "Profile should have a UUID");
            assert_eq!(profile.name, "Test Voice");
            assert_eq!(profile.provider, VoiceProviderType::OpenAI);
            assert_eq!(profile.voice_id, "alloy");
            assert!(!profile.is_preset);
        }

        #[test]
        fn test_voice_profile_preset_creation() {
            let metadata = ProfileMetadata::new(AgeRange::Adult, Gender::Male)
                .with_trait("wise")
                .with_description("A test preset");

            let preset = VoiceProfile::preset(
                "test-preset",
                "Test Preset",
                VoiceProviderType::OpenAI,
                "echo",
                metadata,
            );

            assert_eq!(preset.id, "test-preset");
            assert!(preset.is_preset);
            assert_eq!(preset.metadata.age_range, AgeRange::Adult);
            assert_eq!(preset.metadata.gender, Gender::Male);
        }

        #[test]
        fn test_profile_manager_create_profile() {
            let mut manager = VoiceProfileManager::new();
            let profile = VoiceProfile::new("Test Voice", VoiceProviderType::OpenAI, "alloy");

            let result = manager.create_profile(profile.clone());
            assert!(result.is_ok());

            let id = result.unwrap();
            let retrieved = manager.get_profile(&id);
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().name, "Test Voice");
        }

        #[test]
        fn test_profile_manager_update_profile() {
            let mut manager = VoiceProfileManager::new();
            let mut profile = VoiceProfile::new("Original Name", VoiceProviderType::OpenAI, "alloy");
            let id = manager.create_profile(profile.clone()).unwrap();

            // Update the profile
            profile.id = id.clone();
            profile.name = "Updated Name".to_string();

            let result = manager.update_profile(profile);
            assert!(result.is_ok());

            let updated = manager.get_profile(&id).unwrap();
            assert_eq!(updated.name, "Updated Name");
        }

        #[test]
        fn test_profile_manager_delete_profile() {
            let mut manager = VoiceProfileManager::new();
            let profile = VoiceProfile::new("To Delete", VoiceProviderType::OpenAI, "alloy");
            let id = manager.create_profile(profile).unwrap();

            assert!(manager.get_profile(&id).is_some());

            let result = manager.delete_profile(&id);
            assert!(result.is_ok());
            assert!(manager.get_profile(&id).is_none());
        }

        #[test]
        fn test_profile_manager_get_profile_by_id() {
            let mut manager = VoiceProfileManager::new();
            let profile = VoiceProfile::new("Find Me", VoiceProviderType::ElevenLabs, "voice123");
            let id = manager.create_profile(profile).unwrap();

            let found = manager.get_profile(&id);
            assert!(found.is_some());
            assert_eq!(found.unwrap().name, "Find Me");

            // Non-existent profile
            let not_found = manager.get_profile("nonexistent-id");
            assert!(not_found.is_none());
        }

        #[test]
        fn test_profile_manager_list_profiles() {
            let mut manager = VoiceProfileManager::new();

            manager.create_profile(VoiceProfile::new("Voice 1", VoiceProviderType::OpenAI, "alloy")).unwrap();
            manager.create_profile(VoiceProfile::new("Voice 2", VoiceProviderType::ElevenLabs, "voice123")).unwrap();
            manager.create_profile(VoiceProfile::new("Voice 3", VoiceProviderType::Ollama, "llama")).unwrap();

            let profiles = manager.list_profiles();
            assert_eq!(profiles.len(), 3);
        }

        #[test]
        fn test_profile_manager_cannot_modify_preset() {
            let mut manager = VoiceProfileManager::new();
            let presets = manager.list_presets();
            assert!(!presets.is_empty(), "Should have preset profiles");

            let preset_id = &presets[0].id.clone();

            // Try to get mutable reference
            let result = manager.get_profile_mut(preset_id);
            assert!(matches!(result, Err(ProfileError::CannotModifyPreset(_))));

            // Try to delete
            let result = manager.delete_profile(preset_id);
            assert!(matches!(result, Err(ProfileError::CannotModifyPreset(_))));
        }

        #[test]
        fn test_profile_npc_linking() {
            let mut manager = VoiceProfileManager::new();
            let profile = VoiceProfile::new("NPC Voice", VoiceProviderType::OpenAI, "echo");
            let profile_id = manager.create_profile(profile).unwrap();

            let npc_id = "npc-goblin-123";

            // Link NPC to profile
            let result = manager.link_to_npc(&profile_id, npc_id);
            assert!(result.is_ok());

            // Verify link
            let linked = manager.get_profile_for_npc(npc_id);
            assert!(linked.is_some());
            assert_eq!(linked.unwrap().id, profile_id);

            // Unlink
            manager.unlink_from_npc(npc_id).unwrap();
            assert!(manager.get_profile_for_npc(npc_id).is_none());
        }

        #[test]
        fn test_profile_search() {
            let mut manager = VoiceProfileManager::new();

            let mut profile = VoiceProfile::new("Gruff Warrior", VoiceProviderType::OpenAI, "onyx");
            profile.metadata.personality_traits = vec!["gruff".to_string(), "battle-hardened".to_string()];
            manager.create_profile(profile).unwrap();

            let results = manager.search("gruff");
            assert!(!results.is_empty());
            assert!(results.iter().any(|p| p.name == "Gruff Warrior"));
        }

        #[test]
        fn test_profile_filter_by_gender() {
            let manager = VoiceProfileManager::new();
            let male_profiles = manager.filter_by_gender(Gender::Male);
            let female_profiles = manager.filter_by_gender(Gender::Female);

            // Presets should include both genders
            assert!(!male_profiles.is_empty() || !female_profiles.is_empty());
        }

        #[test]
        fn test_profile_filter_by_age() {
            let manager = VoiceProfileManager::new();
            let adult_profiles = manager.filter_by_age(AgeRange::Adult);
            let elderly_profiles = manager.filter_by_age(AgeRange::Elderly);

            assert!(!adult_profiles.is_empty(), "Should have adult profiles in presets");
            assert!(!elderly_profiles.is_empty(), "Should have elderly profiles in presets");
        }

        #[test]
        fn test_profile_filter_by_provider() {
            let manager = VoiceProfileManager::new();

            // All presets use OpenAI
            let openai_profiles = manager.filter_by_provider(VoiceProviderType::OpenAI);
            assert!(!openai_profiles.is_empty());
        }

        #[test]
        fn test_profile_stats() {
            let mut manager = VoiceProfileManager::new();

            manager.create_profile(VoiceProfile::new("User Voice 1", VoiceProviderType::OpenAI, "alloy")).unwrap();
            manager.create_profile(VoiceProfile::new("User Voice 2", VoiceProviderType::ElevenLabs, "voice123")).unwrap();

            let profile = VoiceProfile::new("NPC Voice", VoiceProviderType::OpenAI, "echo");
            let profile_id = manager.create_profile(profile).unwrap();
            manager.link_to_npc(&profile_id, "npc-1").unwrap();

            let stats = manager.stats();
            assert_eq!(stats.total_user_profiles, 3);
            assert!(stats.total_presets >= 13);
            assert_eq!(stats.linked_npcs, 1);
        }

        #[test]
        fn test_profile_export_import() {
            let mut manager = VoiceProfileManager::new();

            manager.create_profile(VoiceProfile::new("Export Test 1", VoiceProviderType::OpenAI, "alloy")).unwrap();
            manager.create_profile(VoiceProfile::new("Export Test 2", VoiceProviderType::ElevenLabs, "voice123")).unwrap();

            let json = manager.export_profiles().unwrap();
            assert!(!json.is_empty());
            assert!(json.contains("Export Test 1"));

            // Create a new manager and import
            let mut new_manager = VoiceProfileManager::new();
            let count = new_manager.import_profiles(&json).unwrap();
            assert_eq!(count, 2);
        }
    }

    // ============================================================================
    // SECTION 2: Voice Provider Detection Tests
    // ============================================================================
    // Tests for voice provider detection using actual crate types

    mod voice_provider_detection_tests {
        use super::*;
        use crate::core::voice::types::{VoiceProviderType, ProviderStatus, VoiceProviderDetection};

        #[test]
        fn test_provider_type_default_endpoints() {
            assert_eq!(
                VoiceProviderType::Ollama.default_endpoint(),
                Some("http://localhost:11434")
            );
            assert_eq!(
                VoiceProviderType::Chatterbox.default_endpoint(),
                Some("http://localhost:8000")
            );
            assert_eq!(
                VoiceProviderType::GptSoVits.default_endpoint(),
                Some("http://localhost:9880")
            );
            assert_eq!(
                VoiceProviderType::FishSpeech.default_endpoint(),
                Some("http://localhost:7860")
            );
            assert_eq!(
                VoiceProviderType::Dia.default_endpoint(),
                Some("http://localhost:8003")
            );

            // Cloud providers have no default endpoint
            assert_eq!(VoiceProviderType::ElevenLabs.default_endpoint(), None);
            assert_eq!(VoiceProviderType::OpenAI.default_endpoint(), None);
            assert_eq!(VoiceProviderType::FishAudio.default_endpoint(), None);
        }

        #[test]
        fn test_provider_type_is_local() {
            // Local providers
            assert!(VoiceProviderType::Ollama.is_local());
            assert!(VoiceProviderType::Chatterbox.is_local());
            assert!(VoiceProviderType::GptSoVits.is_local());
            assert!(VoiceProviderType::XttsV2.is_local());
            assert!(VoiceProviderType::FishSpeech.is_local());
            assert!(VoiceProviderType::Dia.is_local());
            assert!(VoiceProviderType::Piper.is_local());
            assert!(VoiceProviderType::Coqui.is_local());

            // Cloud providers
            assert!(!VoiceProviderType::ElevenLabs.is_local());
            assert!(!VoiceProviderType::OpenAI.is_local());
            assert!(!VoiceProviderType::FishAudio.is_local());
        }

        #[test]
        fn test_provider_type_display_names() {
            assert_eq!(VoiceProviderType::ElevenLabs.display_name(), "ElevenLabs");
            assert_eq!(VoiceProviderType::OpenAI.display_name(), "OpenAI TTS");
            assert_eq!(VoiceProviderType::Ollama.display_name(), "Ollama");
            assert_eq!(VoiceProviderType::Chatterbox.display_name(), "Chatterbox");
            assert_eq!(VoiceProviderType::GptSoVits.display_name(), "GPT-SoVITS");
            assert_eq!(VoiceProviderType::XttsV2.display_name(), "XTTS-v2 (Coqui)");
            assert_eq!(VoiceProviderType::FishSpeech.display_name(), "Fish Speech");
            assert_eq!(VoiceProviderType::Dia.display_name(), "Dia");
            assert_eq!(VoiceProviderType::Piper.display_name(), "Piper (Local)");
            assert_eq!(VoiceProviderType::FishAudio.display_name(), "Fish Audio (Cloud)");
        }

        #[test]
        fn test_provider_status_structure() {
            let status = ProviderStatus {
                provider: VoiceProviderType::Ollama,
                available: true,
                endpoint: Some("http://localhost:11434".to_string()),
                version: Some("0.1.0".to_string()),
                error: None,
            };

            assert!(status.available);
            assert!(status.error.is_none());
            assert!(status.version.is_some());
        }

        #[test]
        fn test_provider_status_unavailable() {
            let status = ProviderStatus {
                provider: VoiceProviderType::Chatterbox,
                available: false,
                endpoint: Some("http://localhost:8000".to_string()),
                version: None,
                error: Some("Not running (connection refused)".to_string()),
            };

            assert!(!status.available);
            assert!(status.error.is_some());
            assert!(status.error.as_ref().unwrap().contains("connection refused"));
        }

        #[test]
        fn test_voice_provider_detection_structure() {
            let detection = VoiceProviderDetection {
                providers: vec![
                    ProviderStatus {
                        provider: VoiceProviderType::Ollama,
                        available: true,
                        endpoint: Some("http://localhost:11434".to_string()),
                        version: Some("0.1.0".to_string()),
                        error: None,
                    },
                    ProviderStatus {
                        provider: VoiceProviderType::Chatterbox,
                        available: false,
                        endpoint: Some("http://localhost:8000".to_string()),
                        version: None,
                        error: Some("Connection refused".to_string()),
                    },
                ],
                detected_at: Some(Utc::now().to_rfc3339()),
            };

            assert_eq!(detection.providers.len(), 2);
            assert!(detection.detected_at.is_some());
        }

        #[tokio::test]
        async fn test_detect_providers_returns_all_local() {
            use crate::core::voice::detection::detect_providers;

            let detection = detect_providers().await;

            // Should return status for all local providers
            assert!(detection.providers.len() >= 5);
            assert!(detection.detected_at.is_some());

            // Verify expected providers are in the list
            let provider_types: Vec<_> = detection.providers.iter().map(|p| &p.provider).collect();
            assert!(provider_types.iter().any(|p| **p == VoiceProviderType::Ollama));
            assert!(provider_types.iter().any(|p| **p == VoiceProviderType::Chatterbox));
        }
    }

    // ============================================================================
    // SECTION 3: TTS Queue Tests
    // ============================================================================
    // Tests for TTS queue management using actual crate types

    mod tts_queue_tests {
        use super::*;
        use crate::core::voice::queue::{
            SynthesisQueue, SynthesisJob, JobPriority, JobStatus, JobProgress,
            QueueConfig, QueueError,
        };
        use crate::core::voice::types::VoiceProviderType;

        fn create_test_job(text: &str) -> SynthesisJob {
            SynthesisJob::new(text, "profile-1", VoiceProviderType::OpenAI, "alloy")
        }

        #[tokio::test]
        async fn test_queue_addition() {
            let queue = SynthesisQueue::with_defaults();

            let job = create_test_job("Hello world");
            let job_id = queue.submit(job, None).await.unwrap();

            assert!(!job_id.is_empty());

            let retrieved = queue.get_job(&job_id).await.unwrap();
            assert_eq!(retrieved.text, "Hello world");
            assert_eq!(retrieved.status, JobStatus::Pending);
        }

        #[tokio::test]
        async fn test_queue_fifo_ordering() {
            let queue = SynthesisQueue::with_defaults();

            // Submit jobs with same priority
            let job1 = create_test_job("First").with_priority(JobPriority::Normal);
            let job2 = create_test_job("Second").with_priority(JobPriority::Normal);
            let job3 = create_test_job("Third").with_priority(JobPriority::Normal);

            let id1 = queue.submit(job1, None).await.unwrap();
            // Small delay to ensure different timestamps
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            let _id2 = queue.submit(job2, None).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            let _id3 = queue.submit(job3, None).await.unwrap();

            // First job should be dequeued first (FIFO within same priority)
            let next = queue.next_job().await.unwrap();
            assert_eq!(next.id, id1);
            assert_eq!(next.text, "First");
        }

        #[tokio::test]
        async fn test_queue_priority_override() {
            let queue = SynthesisQueue::with_defaults();

            // Submit low priority first
            let low = create_test_job("Low priority").with_priority(JobPriority::Low);
            let _low_id = queue.submit(low, None).await.unwrap();

            // Submit immediate priority second
            let immediate = create_test_job("Immediate").with_priority(JobPriority::Immediate);
            let immediate_id = queue.submit(immediate, None).await.unwrap();

            // Submit high priority third
            let high = create_test_job("High priority").with_priority(JobPriority::High);
            let high_id = queue.submit(high, None).await.unwrap();

            // Immediate priority should come first
            let next = queue.next_job().await.unwrap();
            assert_eq!(next.id, immediate_id);

            // Then high priority
            let next = queue.next_job().await.unwrap();
            assert_eq!(next.id, high_id);
        }

        #[tokio::test]
        async fn test_queue_cancellation() {
            let queue = SynthesisQueue::with_defaults();

            let job = create_test_job("To be canceled");
            let job_id = queue.submit(job, None).await.unwrap();

            let result = queue.cancel(&job_id, None).await;
            assert!(result.is_ok());

            let status = queue.get_status(&job_id).await.unwrap();
            assert!(matches!(status, JobStatus::Canceled));
        }

        #[tokio::test]
        async fn test_queue_cannot_cancel_completed() {
            let queue = SynthesisQueue::with_defaults();

            let job = create_test_job("Will complete");
            let job_id = queue.submit(job, None).await.unwrap();

            // Simulate job lifecycle
            let _next = queue.next_job().await.unwrap();
            queue.mark_started(&job_id, None).await.unwrap();
            queue.mark_completed(&job_id, "/path/to/audio.mp3", None).await.unwrap();

            // Try to cancel completed job
            let result = queue.cancel(&job_id, None).await;
            assert!(matches!(result, Err(QueueError::InvalidState(_))));
        }

        #[tokio::test]
        async fn test_queue_max_size() {
            let config = QueueConfig {
                max_queue_size: 2,
                ..Default::default()
            };
            let queue = SynthesisQueue::new(config);

            queue.submit(create_test_job("Job 1"), None).await.unwrap();
            queue.submit(create_test_job("Job 2"), None).await.unwrap();

            // Third should fail
            let result = queue.submit(create_test_job("Job 3"), None).await;
            assert!(matches!(result, Err(QueueError::QueueFull)));
        }

        #[tokio::test]
        async fn test_queue_pause_resume() {
            let queue = SynthesisQueue::with_defaults();

            queue.submit(create_test_job("Test"), None).await.unwrap();

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
        async fn test_queue_job_lifecycle() {
            let queue = SynthesisQueue::with_defaults();

            let job = create_test_job("Lifecycle test");
            let job_id = queue.submit(job, None).await.unwrap();

            // Get next job
            let _job = queue.next_job().await.unwrap();

            // Start processing
            queue.mark_started(&job_id, None).await.unwrap();
            let status = queue.get_status(&job_id).await.unwrap();
            assert!(matches!(status, JobStatus::Processing));

            // Update progress
            queue.update_progress(&job_id, 0.5, "Synthesizing", None).await.unwrap();
            let progress = queue.get_progress(&job_id).await.unwrap();
            assert_eq!(progress.progress, 0.5);
            assert_eq!(progress.stage, "Synthesizing");

            // Complete
            queue.mark_completed(&job_id, "/path/to/audio.mp3", None).await.unwrap();
            let status = queue.get_status(&job_id).await.unwrap();
            assert!(matches!(status, JobStatus::Completed));
        }

        #[tokio::test]
        async fn test_queue_job_with_metadata() {
            let queue = SynthesisQueue::with_defaults();

            let job = create_test_job("Tagged job")
                .for_campaign("campaign-1")
                .for_session("session-1")
                .for_npc("npc-1")
                .with_tag("custom-tag");

            let job_id = queue.submit(job, None).await.unwrap();

            let retrieved = queue.get_job(&job_id).await.unwrap();
            assert!(retrieved.tags.contains(&"campaign:campaign-1".to_string()));
            assert!(retrieved.tags.contains(&"session:session-1".to_string()));
            assert!(retrieved.tags.contains(&"npc:npc-1".to_string()));
            assert!(retrieved.tags.contains(&"custom-tag".to_string()));
        }

        #[tokio::test]
        async fn test_queue_list_by_session() {
            let queue = SynthesisQueue::with_defaults();

            queue.submit(create_test_job("Job 1").for_session("session-A"), None).await.unwrap();
            queue.submit(create_test_job("Job 2").for_session("session-A"), None).await.unwrap();
            queue.submit(create_test_job("Job 3").for_session("session-B"), None).await.unwrap();

            let session_a_jobs = queue.list_by_session("session-A").await;
            assert_eq!(session_a_jobs.len(), 2);

            let session_b_jobs = queue.list_by_session("session-B").await;
            assert_eq!(session_b_jobs.len(), 1);
        }

        #[tokio::test]
        async fn test_queue_clear_history() {
            let queue = SynthesisQueue::with_defaults();

            let job = create_test_job("Test");
            let job_id = queue.submit(job, None).await.unwrap();

            // Complete the job
            let _job = queue.next_job().await.unwrap();
            queue.mark_started(&job_id, None).await.unwrap();
            queue.mark_completed(&job_id, "/path/to/audio.mp3", None).await.unwrap();

            // History should have the job
            let history = queue.list_history(None).await;
            assert_eq!(history.len(), 1);

            // Clear history
            queue.clear_history().await;

            let history = queue.list_history(None).await;
            assert_eq!(history.len(), 0);
        }

        #[test]
        fn test_job_status_is_terminal() {
            assert!(!JobStatus::Pending.is_terminal());
            assert!(!JobStatus::Processing.is_terminal());
            assert!(JobStatus::Completed.is_terminal());
            assert!(JobStatus::Failed("error".to_string()).is_terminal());
            assert!(JobStatus::Canceled.is_terminal());
        }

        #[test]
        fn test_job_status_can_cancel() {
            assert!(JobStatus::Pending.can_cancel());
            assert!(JobStatus::Processing.can_cancel());
            assert!(!JobStatus::Completed.can_cancel());
            assert!(!JobStatus::Failed("error".to_string()).can_cancel());
            assert!(!JobStatus::Canceled.can_cancel());
        }

        #[test]
        fn test_job_priority_ordering() {
            assert!((JobPriority::Immediate as u8) > (JobPriority::High as u8));
            assert!((JobPriority::High as u8) > (JobPriority::Normal as u8));
            assert!((JobPriority::Normal as u8) > (JobPriority::Low as u8));
            assert!((JobPriority::Low as u8) > (JobPriority::Batch as u8));
        }
    }

    // ============================================================================
    // SECTION 4: Voice Caching Tests
    // ============================================================================
    // Tests for voice caching using actual crate types

    mod voice_caching_tests {
        use super::*;
        use crate::core::voice::cache::{
            AudioCache, CacheEntry, CacheConfig, CacheKeyParams, CacheStats,
        };
        use crate::core::voice::types::{VoiceProviderType, VoiceSettings, OutputFormat};
        use tempfile::TempDir;

        async fn create_test_cache() -> (AudioCache, TempDir) {
            let temp_dir = TempDir::new().unwrap();
            let cache = AudioCache::with_defaults(temp_dir.path().to_path_buf())
                .await
                .unwrap();
            (cache, temp_dir)
        }

        #[tokio::test]
        async fn test_cache_miss() {
            let (cache, _temp) = create_test_cache().await;

            let result = cache.get("nonexistent-key").await;
            assert!(result.is_none());

            let stats = cache.stats().await;
            assert_eq!(stats.misses, 1);
            assert_eq!(stats.hits, 0);
        }

        #[tokio::test]
        async fn test_cache_hit() {
            let (cache, _temp) = create_test_cache().await;

            let key = "test-audio-key";
            let data = vec![0u8; 1024]; // 1KB of test data

            cache.put(key, &data, OutputFormat::Mp3, &[]).await.unwrap();

            let result = cache.get(key).await;
            assert!(result.is_some());

            let stats = cache.stats().await;
            assert_eq!(stats.hits, 1);
        }

        #[tokio::test]
        async fn test_cache_invalidation() {
            let (cache, _temp) = create_test_cache().await;

            cache.put("key1", &vec![0u8; 1024], OutputFormat::Mp3, &[]).await.unwrap();
            cache.put("key2", &vec![0u8; 1024], OutputFormat::Mp3, &[]).await.unwrap();

            assert!(cache.contains("key1").await);

            cache.remove("key1").await.unwrap();

            assert!(!cache.contains("key1").await);
            assert!(cache.contains("key2").await);
        }

        #[tokio::test]
        async fn test_cache_clear() {
            let (cache, _temp) = create_test_cache().await;

            cache.put("key1", &vec![0u8; 1024], OutputFormat::Mp3, &[]).await.unwrap();
            cache.put("key2", &vec![0u8; 1024], OutputFormat::Mp3, &[]).await.unwrap();

            assert!(!cache.is_empty().await);

            cache.clear().await.unwrap();

            assert!(cache.is_empty().await);
            assert_eq!(cache.current_size(), 0);
        }

        #[tokio::test]
        async fn test_cache_clear_by_tag() {
            let (cache, _temp) = create_test_cache().await;

            cache.put(
                "session1-audio1",
                &vec![0u8; 100],
                OutputFormat::Mp3,
                &["session:123".to_string()],
            ).await.unwrap();

            cache.put(
                "session1-audio2",
                &vec![0u8; 100],
                OutputFormat::Mp3,
                &["session:123".to_string()],
            ).await.unwrap();

            cache.put(
                "session2-audio1",
                &vec![0u8; 100],
                OutputFormat::Mp3,
                &["session:456".to_string()],
            ).await.unwrap();

            assert_eq!(cache.len().await, 3);

            let removed = cache.clear_by_tag("session:123").await.unwrap();
            assert_eq!(removed, 2);
            assert_eq!(cache.len().await, 1);
        }

        #[tokio::test]
        async fn test_cache_stats() {
            let (cache, _temp) = create_test_cache().await;

            cache.put("key1", &vec![0u8; 1024], OutputFormat::Mp3, &[]).await.unwrap();

            // Hit
            cache.get("key1").await;
            // Miss
            cache.get("nonexistent").await;

            let stats = cache.stats().await;
            assert_eq!(stats.entry_count, 1);
            assert_eq!(stats.hits, 1);
            assert_eq!(stats.misses, 1);
            assert_eq!(stats.current_size_bytes, 1024);
            assert_eq!(stats.hit_rate, 0.5);
        }

        #[test]
        fn test_cache_key_generation_deterministic() {
            let settings = VoiceSettings::default();

            let params1 = CacheKeyParams::new(
                "Hello world",
                VoiceProviderType::OpenAI,
                "alloy",
                &settings,
                OutputFormat::Mp3,
            );

            let params2 = CacheKeyParams::new(
                "Hello world",
                VoiceProviderType::OpenAI,
                "alloy",
                &settings,
                OutputFormat::Mp3,
            );

            // Same params should produce same key
            assert_eq!(params1.to_key(), params2.to_key());
        }

        #[test]
        fn test_cache_key_generation_unique() {
            let settings = VoiceSettings::default();

            let params1 = CacheKeyParams::new(
                "Hello world",
                VoiceProviderType::OpenAI,
                "alloy",
                &settings,
                OutputFormat::Mp3,
            );

            let params2 = CacheKeyParams::new(
                "Different text",
                VoiceProviderType::OpenAI,
                "alloy",
                &settings,
                OutputFormat::Mp3,
            );

            let params3 = CacheKeyParams::new(
                "Hello world",
                VoiceProviderType::ElevenLabs,
                "alloy",
                &settings,
                OutputFormat::Mp3,
            );

            // Different text
            assert_ne!(params1.to_key(), params2.to_key());

            // Different provider
            assert_ne!(params1.to_key(), params3.to_key());
        }

        #[test]
        fn test_cache_key_includes_format() {
            let settings = VoiceSettings::default();

            let mp3_params = CacheKeyParams::new(
                "Hello",
                VoiceProviderType::OpenAI,
                "alloy",
                &settings,
                OutputFormat::Mp3,
            );

            let wav_params = CacheKeyParams::new(
                "Hello",
                VoiceProviderType::OpenAI,
                "alloy",
                &settings,
                OutputFormat::Wav,
            );

            assert_ne!(mp3_params.to_key(), wav_params.to_key());
            assert!(mp3_params.to_key().ends_with(".mp3"));
            assert!(wav_params.to_key().ends_with(".wav"));
        }

        #[test]
        fn test_cache_entry_access_tracking() {
            let mut entry = CacheEntry::new(
                "test-key".to_string(),
                PathBuf::from("/cache/test.mp3"),
                1024,
                OutputFormat::Mp3,
            );

            assert_eq!(entry.access_count, 1);

            entry.record_access();
            assert_eq!(entry.access_count, 2);

            entry.record_access();
            assert_eq!(entry.access_count, 3);
        }

        #[test]
        fn test_cache_entry_tags() {
            let mut entry = CacheEntry::new(
                "test-key".to_string(),
                PathBuf::from("/cache/test.mp3"),
                1024,
                OutputFormat::Mp3,
            );

            entry.add_tag("session:123");
            entry.add_tag("npc:goblin");

            assert!(entry.has_tag("session:123"));
            assert!(entry.has_tag("npc:goblin"));
            assert!(!entry.has_tag("nonexistent"));
        }

        #[tokio::test]
        async fn test_cache_custom_config() {
            let temp_dir = TempDir::new().unwrap();
            let config = CacheConfig {
                max_size_bytes: 10 * 1024, // 10 KB
                auto_eviction: true,
                min_age_for_eviction_secs: 0,
                track_stats: true,
            };

            let cache = AudioCache::new(temp_dir.path().to_path_buf(), config).await.unwrap();
            assert_eq!(cache.max_size(), 10 * 1024);
        }
    }

    // ============================================================================
    // SECTION 5: Audio Playback State Tests
    // ============================================================================
    // Tests for audio playback state management using mock types

    mod audio_playback_tests {
        use super::*;

        #[derive(Debug, Clone, PartialEq)]
        enum PlaybackState {
            Idle,
            Loading,
            Playing,
            Paused,
            Stopped,
            Error(String),
        }

        struct MockAudioPlayer {
            state: PlaybackState,
            current_audio_path: Option<PathBuf>,
            volume: f32,
            position_ms: u64,
            duration_ms: u64,
        }

        impl MockAudioPlayer {
            fn new() -> Self {
                Self {
                    state: PlaybackState::Idle,
                    current_audio_path: None,
                    volume: 1.0,
                    position_ms: 0,
                    duration_ms: 0,
                }
            }

            fn load(&mut self, path: PathBuf) -> Result<(), String> {
                self.state = PlaybackState::Loading;
                self.current_audio_path = Some(path);
                self.duration_ms = 5000;
                self.position_ms = 0;
                self.state = PlaybackState::Stopped;
                Ok(())
            }

            fn play(&mut self) -> Result<(), String> {
                match self.state {
                    PlaybackState::Stopped | PlaybackState::Paused => {
                        self.state = PlaybackState::Playing;
                        Ok(())
                    }
                    PlaybackState::Idle => Err("No audio loaded".to_string()),
                    PlaybackState::Loading => Err("Still loading".to_string()),
                    PlaybackState::Playing => Ok(()),
                    PlaybackState::Error(_) => Err("Player in error state".to_string()),
                }
            }

            fn pause(&mut self) -> Result<(), String> {
                if self.state == PlaybackState::Playing {
                    self.state = PlaybackState::Paused;
                    Ok(())
                } else {
                    Err("Not playing".to_string())
                }
            }

            fn stop(&mut self) -> Result<(), String> {
                match self.state {
                    PlaybackState::Playing | PlaybackState::Paused => {
                        self.state = PlaybackState::Stopped;
                        self.position_ms = 0;
                        Ok(())
                    }
                    _ => Err("Nothing to stop".to_string()),
                }
            }

            fn set_volume(&mut self, volume: f32) {
                self.volume = volume.clamp(0.0, 1.0);
            }

            fn seek(&mut self, position_ms: u64) -> Result<(), String> {
                if self.current_audio_path.is_some() {
                    self.position_ms = position_ms.min(self.duration_ms);
                    Ok(())
                } else {
                    Err("No audio loaded".to_string())
                }
            }
        }

        #[test]
        fn test_playback_state_transitions() {
            let mut player = MockAudioPlayer::new();

            assert_eq!(player.state, PlaybackState::Idle);

            // Can't play without loading
            assert!(player.play().is_err());

            // Load audio
            player.load(PathBuf::from("/audio/test.mp3")).unwrap();
            assert_eq!(player.state, PlaybackState::Stopped);

            // Play
            player.play().unwrap();
            assert_eq!(player.state, PlaybackState::Playing);

            // Pause
            player.pause().unwrap();
            assert_eq!(player.state, PlaybackState::Paused);

            // Resume
            player.play().unwrap();
            assert_eq!(player.state, PlaybackState::Playing);

            // Stop
            player.stop().unwrap();
            assert_eq!(player.state, PlaybackState::Stopped);
            assert_eq!(player.position_ms, 0);
        }

        #[test]
        fn test_volume_control() {
            let mut player = MockAudioPlayer::new();

            assert_eq!(player.volume, 1.0);

            player.set_volume(0.5);
            assert_eq!(player.volume, 0.5);

            // Test clamping above max
            player.set_volume(1.5);
            assert_eq!(player.volume, 1.0);

            // Test clamping below min
            player.set_volume(-0.5);
            assert_eq!(player.volume, 0.0);
        }

        #[test]
        fn test_seek_functionality() {
            let mut player = MockAudioPlayer::new();

            // Can't seek without audio
            assert!(player.seek(1000).is_err());

            player.load(PathBuf::from("/audio/test.mp3")).unwrap();

            player.seek(2500).unwrap();
            assert_eq!(player.position_ms, 2500);

            // Seek beyond duration should clamp
            player.seek(10000).unwrap();
            assert_eq!(player.position_ms, player.duration_ms);
        }

        #[test]
        fn test_error_state_handling() {
            let mut player = MockAudioPlayer::new();
            player.state = PlaybackState::Error("Test error".to_string());

            let result = player.play();
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("error state"));
        }
    }

    // ============================================================================
    // SECTION 6: Provider-Specific Error Handling Tests
    // ============================================================================
    // Tests for provider-specific error handling

    mod provider_error_tests {
        use super::*;
        use crate::core::voice::types::VoiceError;

        #[test]
        fn test_voice_error_not_configured() {
            let error = VoiceError::NotConfigured("ElevenLabs".to_string());
            let error_string = format!("{}", error);
            assert!(error_string.contains("not configured"));
            assert!(error_string.contains("ElevenLabs"));
        }

        #[test]
        fn test_voice_error_api_error() {
            let error = VoiceError::ApiError("Invalid API key".to_string());
            let error_string = format!("{}", error);
            assert!(error_string.contains("API error"));
        }

        #[test]
        fn test_voice_error_rate_limit() {
            let error = VoiceError::RateLimitExceeded;
            let error_string = format!("{}", error);
            assert!(error_string.to_lowercase().contains("rate limit"));
        }

        #[test]
        fn test_voice_error_quota_exceeded() {
            let error = VoiceError::QuotaExceeded;
            let error_string = format!("{}", error);
            assert!(error_string.to_lowercase().contains("quota"));
        }

        #[test]
        fn test_voice_error_invalid_voice_id() {
            let error = VoiceError::InvalidVoiceId("invalid_voice_123".to_string());
            let error_string = format!("{}", error);
            assert!(error_string.contains("invalid_voice_123"));
        }

        // Provider-specific mock errors for detailed testing
        #[derive(Debug, Clone)]
        #[allow(dead_code)]
        enum DetailedVoiceError {
            ElevenLabsRateLimit { retry_after_secs: Option<u64> },
            ElevenLabsQuota { usage: u64, limit: u64 },
            OpenAIInvalidModel(String),
            OllamaConnectionRefused(String),
            ChatterboxInvalidReferenceAudio,
            UnsupportedFormat { provider: String, format: String },
        }

        impl DetailedVoiceError {
            fn is_retryable(&self) -> bool {
                matches!(
                    self,
                    Self::ElevenLabsRateLimit { .. } | Self::OllamaConnectionRefused(_)
                )
            }

            fn provider_name(&self) -> &'static str {
                match self {
                    Self::ElevenLabsRateLimit { .. } | Self::ElevenLabsQuota { .. } => "ElevenLabs",
                    Self::OpenAIInvalidModel(_) => "OpenAI",
                    Self::OllamaConnectionRefused(_) => "Ollama",
                    Self::ChatterboxInvalidReferenceAudio => "Chatterbox",
                    Self::UnsupportedFormat { provider, .. } => {
                        // Return static str based on known providers
                        match provider.as_str() {
                            "Piper" => "Piper",
                            "OpenAI" => "OpenAI",
                            _ => "Unknown",
                        }
                    }
                }
            }
        }

        #[test]
        fn test_elevenlabs_rate_limit_error() {
            let error = DetailedVoiceError::ElevenLabsRateLimit {
                retry_after_secs: Some(60),
            };
            assert!(error.is_retryable());
            assert_eq!(error.provider_name(), "ElevenLabs");
        }

        #[test]
        fn test_elevenlabs_quota_error() {
            let error = DetailedVoiceError::ElevenLabsQuota {
                usage: 10000,
                limit: 10000,
            };
            assert!(!error.is_retryable());
            assert_eq!(error.provider_name(), "ElevenLabs");
        }

        #[test]
        fn test_openai_invalid_model_error() {
            let error = DetailedVoiceError::OpenAIInvalidModel("invalid-model".to_string());
            assert!(!error.is_retryable());
            assert_eq!(error.provider_name(), "OpenAI");
        }

        #[test]
        fn test_ollama_connection_error() {
            let error = DetailedVoiceError::OllamaConnectionRefused(
                "Connection refused to localhost:11434".to_string()
            );
            assert!(error.is_retryable());
            assert_eq!(error.provider_name(), "Ollama");
        }

        #[test]
        fn test_chatterbox_invalid_reference() {
            let error = DetailedVoiceError::ChatterboxInvalidReferenceAudio;
            assert!(!error.is_retryable());
            assert_eq!(error.provider_name(), "Chatterbox");
        }

        #[test]
        fn test_unsupported_format_error() {
            let error = DetailedVoiceError::UnsupportedFormat {
                provider: "Piper".to_string(),
                format: "flac".to_string(),
            };
            assert!(!error.is_retryable());
            assert_eq!(error.provider_name(), "Piper");
        }
    }

    // ============================================================================
    // SECTION 7: Voice Manager Integration Tests
    // ============================================================================
    // Tests for the VoiceManager combining multiple subsystems

    mod voice_manager_integration_tests {
        use super::*;
        use crate::core::voice::types::{VoiceConfig, VoiceProviderType, QueuedVoice, VoiceStatus};
        use crate::core::voice::manager::VoiceManager;

        fn create_disabled_config() -> VoiceConfig {
            VoiceConfig {
                provider: VoiceProviderType::Disabled,
                ..Default::default()
            }
        }

        #[test]
        fn test_voice_manager_creation() {
            let config = create_disabled_config();
            let manager = VoiceManager::new(config);

            assert!(manager.queue.is_empty());
            assert!(!manager.is_playing);
        }

        #[test]
        fn test_voice_manager_queue_operations() {
            let config = create_disabled_config();
            let mut manager = VoiceManager::new(config);

            // Add to queue
            let item = manager.add_to_queue("Hello world".to_string(), "voice-1".to_string());
            assert!(!item.id.is_empty());
            assert_eq!(item.text, "Hello world");
            assert!(matches!(item.status, VoiceStatus::Pending));

            // Get queue
            let queue = manager.get_queue();
            assert_eq!(queue.len(), 1);

            // Get next pending
            let next = manager.get_next_pending();
            assert!(next.is_some());
            assert_eq!(next.unwrap().text, "Hello world");

            // Update status
            manager.update_status(&item.id, VoiceStatus::Processing);
            let updated = manager.get_queue().into_iter().find(|i| i.id == item.id).unwrap();
            assert!(matches!(updated.status, VoiceStatus::Processing));

            // Remove from queue
            manager.remove_from_queue(&item.id);
            assert!(manager.queue.is_empty());
        }

        #[test]
        fn test_voice_manager_multiple_queue_items() {
            let config = create_disabled_config();
            let mut manager = VoiceManager::new(config);

            manager.add_to_queue("First".to_string(), "voice-1".to_string());
            manager.add_to_queue("Second".to_string(), "voice-1".to_string());
            manager.add_to_queue("Third".to_string(), "voice-2".to_string());

            assert_eq!(manager.get_queue().len(), 3);

            // Get next pending returns first item
            let next = manager.get_next_pending().unwrap();
            assert_eq!(next.text, "First");
        }

        #[test]
        fn test_voice_manager_queue_status_workflow() {
            let config = create_disabled_config();
            let mut manager = VoiceManager::new(config);

            let item = manager.add_to_queue("Test".to_string(), "voice-1".to_string());

            // Workflow: Pending -> Processing -> Playing -> Completed
            assert!(matches!(
                manager.get_queue()[0].status,
                VoiceStatus::Pending
            ));

            manager.update_status(&item.id, VoiceStatus::Processing);
            assert!(matches!(
                manager.get_queue()[0].status,
                VoiceStatus::Processing
            ));

            manager.update_status(&item.id, VoiceStatus::Playing);
            assert!(matches!(
                manager.get_queue()[0].status,
                VoiceStatus::Playing
            ));

            manager.update_status(&item.id, VoiceStatus::Completed);
            assert!(matches!(
                manager.get_queue()[0].status,
                VoiceStatus::Completed
            ));
        }

        #[test]
        fn test_voice_manager_failed_status() {
            let config = create_disabled_config();
            let mut manager = VoiceManager::new(config);

            let item = manager.add_to_queue("Test".to_string(), "voice-1".to_string());

            manager.update_status(&item.id, VoiceStatus::Failed("API error".to_string()));

            let queue = manager.get_queue();
            match &queue[0].status {
                VoiceStatus::Failed(msg) => assert!(msg.contains("API error")),
                _ => panic!("Expected Failed status"),
            }
        }

        #[test]
        fn test_voice_manager_cache_dir() {
            let mut config = create_disabled_config();
            config.cache_dir = Some(PathBuf::from("/custom/cache/dir"));

            let manager = VoiceManager::new(config);

            assert_eq!(manager.cache_dir(), &PathBuf::from("/custom/cache/dir"));
        }

        #[test]
        fn test_voice_manager_default_cache_dir() {
            let config = create_disabled_config();
            let manager = VoiceManager::new(config);

            assert_eq!(manager.cache_dir(), &PathBuf::from("./voice_cache"));
        }
    }

    // ============================================================================
    // SECTION 8: Preset Tests
    // ============================================================================
    // Tests for voice presets

    mod preset_tests {
        use super::*;
        use crate::core::voice::presets::{
            get_dm_presets, get_presets_by_tag, get_preset_by_id, get_openai_voice_ids,
        };
        use crate::core::voice::profiles::{AgeRange, Gender};

        #[test]
        fn test_dm_presets_count() {
            let presets = get_dm_presets();
            assert!(presets.len() >= 13, "Should have at least 13 DM personas");
        }

        #[test]
        fn test_all_presets_have_required_fields() {
            for preset in get_dm_presets() {
                assert!(!preset.id.is_empty(), "Preset should have an ID");
                assert!(!preset.name.is_empty(), "Preset should have a name");
                assert!(!preset.voice_id.is_empty(), "Preset should have a voice_id");
                assert!(preset.is_preset, "Preset should be marked as preset");
                assert!(
                    !preset.metadata.personality_traits.is_empty(),
                    "Preset should have personality traits"
                );
                assert!(
                    preset.metadata.description.is_some(),
                    "Preset should have a description"
                );
                assert!(
                    !preset.metadata.tags.is_empty(),
                    "Preset should have at least one tag"
                );
            }
        }

        #[test]
        fn test_presets_by_tag() {
            let fantasy = get_presets_by_tag("fantasy");
            assert!(!fantasy.is_empty(), "Should have fantasy presets");

            let horror = get_presets_by_tag("horror");
            assert!(!horror.is_empty(), "Should have horror presets");

            let narrator = get_presets_by_tag("narrator");
            assert!(!narrator.is_empty(), "Should have narrator presets");
        }

        #[test]
        fn test_preset_by_id() {
            let preset = get_preset_by_id("preset-wise-sage");
            assert!(preset.is_some(), "Should find preset by ID");
            assert_eq!(preset.unwrap().name, "The Wise Sage");

            let nonexistent = get_preset_by_id("nonexistent-preset");
            assert!(nonexistent.is_none());
        }

        #[test]
        fn test_unique_preset_ids() {
            let presets = get_dm_presets();
            let mut ids: std::collections::HashSet<String> = std::collections::HashSet::new();

            for preset in presets {
                assert!(
                    ids.insert(preset.id.clone()),
                    "Preset IDs should be unique: {}",
                    preset.id
                );
            }
        }

        #[test]
        fn test_gender_variety_in_presets() {
            let presets = get_dm_presets();
            let has_male = presets.iter().any(|p| matches!(p.metadata.gender, Gender::Male));
            let has_female = presets.iter().any(|p| matches!(p.metadata.gender, Gender::Female));
            let has_neutral = presets.iter().any(|p| matches!(p.metadata.gender, Gender::Neutral));

            assert!(has_male, "Should have male voices");
            assert!(has_female, "Should have female voices");
            assert!(has_neutral, "Should have neutral voices");
        }

        #[test]
        fn test_age_variety_in_presets() {
            let presets = get_dm_presets();
            let has_adult = presets
                .iter()
                .any(|p| matches!(p.metadata.age_range, AgeRange::Adult));
            let has_elderly = presets
                .iter()
                .any(|p| matches!(p.metadata.age_range, AgeRange::Elderly));
            let has_young_adult = presets
                .iter()
                .any(|p| matches!(p.metadata.age_range, AgeRange::YoungAdult));

            assert!(has_adult, "Should have adult voices");
            assert!(has_elderly, "Should have elderly voices");
            assert!(has_young_adult, "Should have young adult voices");
        }

        #[test]
        fn test_openai_voice_ids() {
            let voice_ids = get_openai_voice_ids();
            assert!(voice_ids.contains(&"alloy"));
            assert!(voice_ids.contains(&"echo"));
            assert!(voice_ids.contains(&"fable"));
            assert!(voice_ids.contains(&"onyx"));
            assert!(voice_ids.contains(&"nova"));
            assert!(voice_ids.contains(&"shimmer"));
        }
    }
}
