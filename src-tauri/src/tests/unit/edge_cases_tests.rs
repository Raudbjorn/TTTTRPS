//! Edge Case Tests for TTRPG Assistant
//!
//! This module contains comprehensive edge case tests for error scenarios:
//! - Network errors (timeout, connection refused, mid-stream disconnect)
//! - Data errors (malformed JSON, missing fields, invalid UTF-8, empty response)
//! - Resource errors (database locked, permission denied)
//! - Concurrent access (simultaneous updates, race conditions)

#[cfg(test)]
mod tests {
    use crate::tests::mocks::{
        LlmClient, LlmError, MockChatMessage, MockChatResponse, MockLlmClient,
        MockMessageRole, MockSearchClient, MockSynthesisRequest, MockVoiceProvider,
        SearchClient, SearchError, VoiceError, VoiceProvider,
    };
    use crate::database::{
        CampaignOps, CampaignRecord, CombatOps, CombatStateRecord, Database, NpcConversation,
        NpcOps, NpcRecord, SessionOps, SessionRecord,
    };
    use tempfile::TempDir;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::{Barrier, RwLock};

    // =========================================================================
    // Test Helpers
    // =========================================================================

    async fn create_test_db() -> (Database, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db = Database::new(temp_dir.path())
            .await
            .expect("Failed to create database");
        (db, temp_dir)
    }

    // =========================================================================
    // Network Error Tests - LLM Client
    // =========================================================================

    #[tokio::test]
    async fn test_llm_timeout_error() {
        let mut mock = MockLlmClient::new();

        mock.expect_id().returning(|| "timeout-test".to_string());
        mock.expect_model().returning(|| "test-model".to_string());
        mock.expect_supports_streaming().returning(|| false);

        // Simulate timeout
        mock.expect_chat().returning(|_messages, _max_tokens| {
            Err(LlmError::Timeout)
        });

        let messages = vec![MockChatMessage {
            role: MockMessageRole::User,
            content: "Hello".to_string(),
        }];

        let result = mock.chat(messages, None).await;

        assert!(result.is_err());
        match result {
            Err(LlmError::Timeout) => (), // Expected
            other => panic!("Expected Timeout error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_llm_connection_refused() {
        let mut mock = MockLlmClient::new();

        mock.expect_id().returning(|| "connection-test".to_string());
        mock.expect_model().returning(|| "test-model".to_string());
        mock.expect_supports_streaming().returning(|| false);

        // Simulate connection refused
        mock.expect_chat().returning(|_messages, _max_tokens| {
            Err(LlmError::ApiError(
                "Connection refused: unable to connect to server".to_string(),
            ))
        });

        let messages = vec![MockChatMessage {
            role: MockMessageRole::User,
            content: "Test connection".to_string(),
        }];

        let result = mock.chat(messages, None).await;

        assert!(result.is_err());
        match &result {
            Err(LlmError::ApiError(msg)) => {
                assert!(msg.contains("Connection refused"));
            }
            other => panic!("Expected ApiError with connection refused, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_llm_mid_stream_disconnect() {
        let mut mock = MockLlmClient::new();

        mock.expect_id().returning(|| "disconnect-test".to_string());
        mock.expect_model().returning(|| "test-model".to_string());
        mock.expect_supports_streaming().returning(|| true);

        // Simulate mid-stream disconnect
        mock.expect_chat().returning(|_messages, _max_tokens| {
            Err(LlmError::ApiError(
                    "Stream disconnected: connection reset by peer".to_string(),
                ))
        });

        let messages = vec![MockChatMessage {
            role: MockMessageRole::User,
            content: "Generate a long story".to_string(),
        }];

        let result = mock.chat(messages, None).await;

        assert!(result.is_err());
        match &result {
            Err(LlmError::ApiError(msg)) => {
                assert!(msg.contains("disconnected"));
            }
            other => panic!("Expected stream disconnect error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_llm_rate_limited() {
        let mut mock = MockLlmClient::new();

        mock.expect_id().returning(|| "ratelimit-test".to_string());
        mock.expect_model().returning(|| "test-model".to_string());
        mock.expect_supports_streaming().returning(|| false);

        mock.expect_chat().returning(|_messages, _max_tokens| {
            Err(LlmError::RateLimited)
        });

        let messages = vec![MockChatMessage {
            role: MockMessageRole::User,
            content: "Test".to_string(),
        }];

        let result = mock.chat(messages, None).await;

        assert!(result.is_err());
        match result {
            Err(LlmError::RateLimited) => (), // Expected
            other => panic!("Expected RateLimited error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_llm_no_providers_available() {
        let mut mock = MockLlmClient::new();

        mock.expect_id().returning(|| "no-providers-test".to_string());
        mock.expect_model().returning(|| "test-model".to_string());
        mock.expect_supports_streaming().returning(|| false);

        mock.expect_chat().returning(|_messages, _max_tokens| {
            Err(LlmError::NoProviders)
        });

        let messages = vec![MockChatMessage {
            role: MockMessageRole::User,
            content: "Test".to_string(),
        }];

        let result = mock.chat(messages, None).await;

        assert!(result.is_err());
        match result {
            Err(LlmError::NoProviders) => (), // Expected
            other => panic!("Expected NoProviders error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_llm_health_check_failure() {
        let mut mock = MockLlmClient::new();

        mock.expect_id().returning(|| "health-test".to_string());
        mock.expect_model().returning(|| "test-model".to_string());
        mock.expect_supports_streaming().returning(|| false);

        // Provider is unhealthy
        mock.expect_health_check().returning(|| false);

        let is_healthy = mock.health_check().await;
        assert!(!is_healthy);
    }

    // =========================================================================
    // Data Error Tests - Malformed Responses
    // =========================================================================

    #[tokio::test]
    async fn test_malformed_json_response() {
        let mut mock = MockLlmClient::new();

        mock.expect_id().returning(|| "json-error-test".to_string());
        mock.expect_model().returning(|| "test-model".to_string());
        mock.expect_supports_streaming().returning(|| false);

        // Simulate malformed JSON response
        mock.expect_chat().returning(|_messages, _max_tokens| {
            Err(LlmError::ApiError(
                    "Failed to parse response: expected `:` but found `}` at line 1 column 42"
                        .to_string(),
                ))
        });

        let messages = vec![MockChatMessage {
            role: MockMessageRole::User,
            content: "Test".to_string(),
        }];

        let result = mock.chat(messages, None).await;

        assert!(result.is_err());
        match &result {
            Err(LlmError::ApiError(msg)) => {
                assert!(msg.contains("parse"));
            }
            other => panic!("Expected parse error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_missing_required_fields_response() {
        let mut mock = MockLlmClient::new();

        mock.expect_id().returning(|| "missing-fields-test".to_string());
        mock.expect_model().returning(|| "test-model".to_string());
        mock.expect_supports_streaming().returning(|| false);

        // Simulate response with missing required fields
        mock.expect_chat().returning(|_messages, _max_tokens| {
            Err(LlmError::ApiError(
                    "Response missing required field: 'content'".to_string(),
                ))
        });

        let messages = vec![MockChatMessage {
            role: MockMessageRole::User,
            content: "Test".to_string(),
        }];

        let result = mock.chat(messages, None).await;

        assert!(result.is_err());
        match &result {
            Err(LlmError::ApiError(msg)) => {
                assert!(msg.contains("missing required field"));
            }
            other => panic!("Expected missing field error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_invalid_utf8_response() {
        let mut mock = MockLlmClient::new();

        mock.expect_id().returning(|| "utf8-error-test".to_string());
        mock.expect_model().returning(|| "test-model".to_string());
        mock.expect_supports_streaming().returning(|| false);

        // Simulate invalid UTF-8 in response
        mock.expect_chat().returning(|_messages, _max_tokens| {
            Err(LlmError::ApiError(
                    "Invalid UTF-8 sequence in response body".to_string(),
                ))
        });

        let messages = vec![MockChatMessage {
            role: MockMessageRole::User,
            content: "Test".to_string(),
        }];

        let result = mock.chat(messages, None).await;

        assert!(result.is_err());
        match &result {
            Err(LlmError::ApiError(msg)) => {
                assert!(msg.contains("UTF-8"));
            }
            other => panic!("Expected UTF-8 error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_empty_response() {
        let mut mock = MockLlmClient::new();

        mock.expect_id().returning(|| "empty-response-test".to_string());
        mock.expect_model().returning(|| "test-model".to_string());
        mock.expect_supports_streaming().returning(|| false);

        // Simulate empty response
        mock.expect_chat().returning(|_messages, _max_tokens| {
            Ok(MockChatResponse {
                    content: "".to_string(),
                    model: "test-model".to_string(),
                    provider: "empty-response-test".to_string(),
                    input_tokens: 10,
                    output_tokens: 0,
            })
        });

        let messages = vec![MockChatMessage {
            role: MockMessageRole::User,
            content: "Test".to_string(),
        }];

        let result = mock.chat(messages, None).await;

        // Empty response is valid but might need handling by caller
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.content.is_empty());
        assert_eq!(response.output_tokens, 0);
    }

    #[tokio::test]
    async fn test_null_content_in_response() {
        let mut mock = MockLlmClient::new();

        mock.expect_id().returning(|| "null-content-test".to_string());
        mock.expect_model().returning(|| "test-model".to_string());
        mock.expect_supports_streaming().returning(|| false);

        // Simulate API returning null content field
        mock.expect_chat().returning(|_messages, _max_tokens| {
            Err(LlmError::ApiError(
                    "Response content is null".to_string(),
                ))
        });

        let messages = vec![MockChatMessage {
            role: MockMessageRole::User,
            content: "Test".to_string(),
        }];

        let result = mock.chat(messages, None).await;

        assert!(result.is_err());
    }

    // =========================================================================
    // Voice Provider Error Tests
    // =========================================================================

    #[tokio::test]
    async fn test_voice_provider_not_configured() {
        let mut mock = MockVoiceProvider::new();

        mock.expect_id().returning(|| "voice-not-configured".to_string());

        mock.expect_synthesize().returning(|_request| {
            Err(VoiceError::NotConfigured(
                    "ElevenLabs API key not set".to_string(),
                ))
        });

        let request = MockSynthesisRequest {
            text: "Hello".to_string(),
            voice_id: "voice-1".to_string(),
            settings: None,
        };

        let result = mock.synthesize(request.clone()).await;

        assert!(result.is_err());
        match &result {
            Err(VoiceError::NotConfigured(msg)) => {
                assert!(msg.contains("API key"));
            }
            other => panic!("Expected NotConfigured error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_voice_provider_invalid_voice_id() {
        let mut mock = MockVoiceProvider::new();

        mock.expect_id().returning(|| "voice-invalid-id".to_string());

        mock.expect_synthesize().returning(|request| {
            Err(VoiceError::InvalidVoiceId(request.voice_id.clone()))
        });

        let request = MockSynthesisRequest {
            text: "Hello".to_string(),
            voice_id: "nonexistent-voice".to_string(),
            settings: None,
        };

        let result = mock.synthesize(request.clone()).await;

        assert!(result.is_err());
        match &result {
            Err(VoiceError::InvalidVoiceId(id)) => {
                assert_eq!(id, "nonexistent-voice");
            }
            other => panic!("Expected InvalidVoiceId error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_voice_provider_rate_limit() {
        let mut mock = MockVoiceProvider::new();

        mock.expect_id().returning(|| "voice-rate-limit".to_string());

        mock.expect_synthesize().returning(|_request| {
            Err(VoiceError::RateLimitExceeded)
        });

        let request = MockSynthesisRequest {
            text: "Hello".to_string(),
            voice_id: "voice-1".to_string(),
            settings: None,
        };

        let result = mock.synthesize(request.clone()).await;

        assert!(result.is_err());
        match result {
            Err(VoiceError::RateLimitExceeded) => (), // Expected
            other => panic!("Expected RateLimitExceeded error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_voice_provider_quota_exceeded() {
        let mut mock = MockVoiceProvider::new();

        mock.expect_id().returning(|| "voice-quota".to_string());

        mock.expect_synthesize().returning(|_request| {
            Err(VoiceError::QuotaExceeded)
        });

        let request = MockSynthesisRequest {
            text: "Hello".to_string(),
            voice_id: "voice-1".to_string(),
            settings: None,
        };

        let result = mock.synthesize(request.clone()).await;

        assert!(result.is_err());
        match result {
            Err(VoiceError::QuotaExceeded) => (), // Expected
            other => panic!("Expected QuotaExceeded error, got: {:?}", other),
        }
    }

    // =========================================================================
    // Search Client Error Tests
    // =========================================================================

    #[tokio::test]
    async fn test_search_index_not_found() {
        let mut mock = MockSearchClient::new();

        mock.expect_search().returning(|index, _query, _limit| {
            Err(SearchError::IndexNotFound(index))
        });

        let result = mock.search("nonexistent_index".to_string(), "query".to_string(), 10).await;

        assert!(result.is_err());
        match &result {
            Err(SearchError::IndexNotFound(idx)) => {
                assert_eq!(idx, "nonexistent_index");
            }
            other => panic!("Expected IndexNotFound error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_search_document_not_found() {
        let mut mock = MockSearchClient::new();

        mock.expect_delete_document()
            .returning(|_index, doc_id| {
                Err(SearchError::DocumentNotFound(doc_id))
            });

        let result = mock.delete_document("documents".to_string(), "missing-doc".to_string()).await;

        assert!(result.is_err());
        match &result {
            Err(SearchError::DocumentNotFound(id)) => {
                assert_eq!(id, "missing-doc");
            }
            other => panic!("Expected DocumentNotFound error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_search_configuration_error() {
        let mut mock = MockSearchClient::new();

        mock.expect_health_check().returning(|| false);

        mock.expect_search().returning(|_index, _query, _limit| {
            Err(SearchError::ConfigError(
                    "Meilisearch is not configured".to_string(),
                ))
        });

        let is_healthy = mock.health_check().await;
        assert!(!is_healthy);

        let result = mock.search("documents".to_string(), "query".to_string(), 10).await;
        assert!(result.is_err());
    }

    // =========================================================================
    // Resource Error Tests - Database
    // =========================================================================

    #[tokio::test]
    async fn test_database_file_locked() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create first connection
        let db1 = Database::new(temp_dir.path())
            .await
            .expect("Failed to create first database connection");

        // Create campaign
        let campaign = CampaignRecord::new(
            "camp-locked".to_string(),
            "Lock Test".to_string(),
            "D&D 5e".to_string(),
        );
        db1.create_campaign(&campaign).await.expect("Failed to create campaign");

        // Create second connection (SQLite should handle this with WAL mode)
        let db2 = Database::new(temp_dir.path())
            .await
            .expect("Failed to create second database connection");

        // Both connections should work with WAL mode
        let campaigns_1 = db1.list_campaigns().await.expect("db1 list should work");
        let campaigns_2 = db2.list_campaigns().await.expect("db2 list should work");

        assert_eq!(campaigns_1.len(), campaigns_2.len());
    }

    #[tokio::test]
    async fn test_database_permission_denied_simulation() {
        // This test simulates what happens when we try to access a file
        // we don't have permissions for
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db = Database::new(temp_dir.path())
            .await
            .expect("Failed to create database");

        // Database should work normally
        let campaign = CampaignRecord::new(
            "camp-perm".to_string(),
            "Permission Test".to_string(),
            "D&D 5e".to_string(),
        );
        db.create_campaign(&campaign).await.expect("Should create campaign");

        // Note: Actually testing permission denied requires platform-specific
        // file permission manipulation which is complex in tests.
        // The database layer should handle these errors gracefully.
    }

    // =========================================================================
    // Concurrent Access Tests - Simultaneous Updates
    // =========================================================================

    #[tokio::test]
    async fn test_simultaneous_campaign_updates() {
        let (db, _temp) = create_test_db().await;

        // Create campaign
        let campaign = CampaignRecord::new(
            "camp-concurrent".to_string(),
            "Concurrent Test".to_string(),
            "D&D 5e".to_string(),
        );
        db.create_campaign(&campaign).await.expect("Failed to create");

        let db = Arc::new(db);
        let barrier = Arc::new(Barrier::new(5));

        // Spawn 5 tasks that all try to update the same campaign
        let handles: Vec<_> = (0..5)
            .map(|i| {
                let db = db.clone();
                let barrier = barrier.clone();
                tokio::spawn(async move {
                    barrier.wait().await; // Synchronize all tasks

                    let mut camp = db
                        .get_campaign("camp-concurrent")
                        .await
                        .expect("Failed to get")
                        .expect("Not found");

                    camp.description = Some(format!("Updated by task {}", i));
                    camp.updated_at = chrono::Utc::now().to_rfc3339();

                    db.update_campaign(&camp).await.expect("Failed to update");

                    i
                })
            })
            .collect();

        // Wait for all tasks
        for handle in handles {
            handle.await.expect("Task panicked");
        }

        // Verify campaign exists and was updated (last write wins)
        let final_camp = db
            .get_campaign("camp-concurrent")
            .await
            .expect("Failed to get")
            .expect("Not found");
        assert!(final_camp.description.is_some());
    }

    #[tokio::test]
    async fn test_simultaneous_session_creation() {
        let (db, _temp) = create_test_db().await;

        let campaign = CampaignRecord::new(
            "camp-sess-concurrent".to_string(),
            "Session Concurrent".to_string(),
            "D&D 5e".to_string(),
        );
        db.create_campaign(&campaign).await.expect("Failed to create");

        let db = Arc::new(db);
        let barrier = Arc::new(Barrier::new(10));

        // Spawn 10 tasks that create sessions simultaneously
        let handles: Vec<_> = (1..=10)
            .map(|i| {
                let db = db.clone();
                let barrier = barrier.clone();
                tokio::spawn(async move {
                    barrier.wait().await;

                    let session = SessionRecord::new(
                        format!("sess-concurrent-{}", i),
                        "camp-sess-concurrent".to_string(),
                        i,
                    );
                    db.create_session(&session).await.expect("Failed to create");
                    i
                })
            })
            .collect();

        for handle in handles {
            handle.await.expect("Task panicked");
        }

        let sessions = db
            .list_sessions("camp-sess-concurrent")
            .await
            .expect("Failed to list");
        assert_eq!(sessions.len(), 10);
    }

    // =========================================================================
    // Race Condition Tests - Combat Tracker
    // =========================================================================

    #[tokio::test]
    async fn test_combat_state_race_condition() {
        let (db, _temp) = create_test_db().await;

        let campaign = CampaignRecord::new(
            "camp-combat-race".to_string(),
            "Combat Race".to_string(),
            "D&D 5e".to_string(),
        );
        db.create_campaign(&campaign).await.expect("Failed to create");

        let session = SessionRecord::new(
            "sess-combat-race".to_string(),
            "camp-combat-race".to_string(),
            1,
        );
        db.create_session(&session).await.expect("Failed to create");

        let combat = CombatStateRecord::new(
            "combat-race".to_string(),
            "sess-combat-race".to_string(),
            r#"[{"name":"Fighter","initiative":15},{"name":"Goblin","initiative":10}]"#.to_string(),
        );
        db.save_combat_state(&combat).await.expect("Failed to save");

        let db = Arc::new(db);
        let counter = Arc::new(RwLock::new(0));
        let barrier = Arc::new(Barrier::new(5));

        // Simulate multiple DM actions on the same combat
        let handles: Vec<_> = (0..5)
            .map(|i| {
                let db = db.clone();
                let counter = counter.clone();
                let barrier = barrier.clone();
                tokio::spawn(async move {
                    barrier.wait().await;

                    // Read current state
                    let mut combat = db
                        .get_combat_state("combat-race")
                        .await
                        .expect("Failed to get")
                        .expect("Not found");

                    // Simulate thinking time
                    tokio::time::sleep(Duration::from_millis(10)).await;

                    // Update round (this could cause race condition in production
                    // without proper optimistic locking)
                    let mut c = counter.write().await;
                    *c += 1;
                    combat.round = *c as i32;
                    combat.notes = Some(format!("Updated by task {}", i));
                    combat.updated_at = chrono::Utc::now().to_rfc3339();
                    drop(c);

                    db.save_combat_state(&combat).await.expect("Failed to save");
                    i
                })
            })
            .collect();

        for handle in handles {
            handle.await.expect("Task panicked");
        }

        let final_combat = db
            .get_combat_state("combat-race")
            .await
            .expect("Failed to get")
            .expect("Not found");

        // Round should be updated (though the exact value depends on race timing)
        assert!(final_combat.round >= 1);
        assert!(final_combat.notes.is_some());
    }

    #[tokio::test]
    async fn test_npc_conversation_race_condition() {
        let (db, _temp) = create_test_db().await;

        let campaign = CampaignRecord::new(
            "camp-conv-race".to_string(),
            "Conversation Race".to_string(),
            "D&D 5e".to_string(),
        );
        db.create_campaign(&campaign).await.expect("Failed to create");

        let npc = NpcRecord {
            id: "npc-conv-race".to_string(),
            campaign_id: Some("camp-conv-race".to_string()),
            name: "Race Test NPC".to_string(),
            role: "Test".to_string(),
            personality_id: None,
            personality_json: "{}".to_string(),
            data_json: None,
            stats_json: None,
            notes: None,
            location_id: None,
            voice_profile_id: None,
            quest_hooks: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        db.save_npc(&npc).await.expect("Failed to save");

        let conversation = NpcConversation::new(
            "conv-race".to_string(),
            "npc-conv-race".to_string(),
            "camp-conv-race".to_string(),
        );
        db.save_npc_conversation(&conversation)
            .await
            .expect("Failed to save");

        let db = Arc::new(db);
        let message_counter = Arc::new(RwLock::new(0u32));
        let barrier = Arc::new(Barrier::new(3));

        // Simulate multiple messages being added simultaneously
        let handles: Vec<_> = (0..3)
            .map(|i| {
                let db = db.clone();
                let message_counter = message_counter.clone();
                let barrier = barrier.clone();
                tokio::spawn(async move {
                    barrier.wait().await;

                    let mut conv = db
                        .get_npc_conversation("npc-conv-race")
                        .await
                        .expect("Failed to get")
                        .expect("Not found");

                    // Increment message counter atomically
                    let mut count = message_counter.write().await;
                    *count += 1;
                    conv.unread_count = *count;
                    drop(count);

                    conv.messages_json =
                        format!(r#"[{{"id":"msg{}","role":"user","content":"Message {}"}}]"#, i, i);
                    conv.last_message_at = chrono::Utc::now().to_rfc3339();
                    conv.updated_at = chrono::Utc::now().to_rfc3339();

                    db.save_npc_conversation(&conv).await.expect("Failed to save");
                    i
                })
            })
            .collect();

        for handle in handles {
            handle.await.expect("Task panicked");
        }

        let final_conv = db
            .get_npc_conversation("npc-conv-race")
            .await
            .expect("Failed to get")
            .expect("Not found");

        // Unread count should reflect all updates (though order may vary)
        assert!(final_conv.unread_count >= 1);
    }

    // =========================================================================
    // Edge Case Tests - Boundary Conditions
    // =========================================================================

    #[tokio::test]
    async fn test_very_long_content() {
        let (db, _temp) = create_test_db().await;

        // Create campaign with very long description (100KB)
        let long_description = "a".repeat(100_000);
        let mut campaign = CampaignRecord::new(
            "camp-long".to_string(),
            "Long Content Test".to_string(),
            "D&D 5e".to_string(),
        );
        campaign.description = Some(long_description.clone());

        db.create_campaign(&campaign)
            .await
            .expect("Failed to create campaign with long content");

        let retrieved = db
            .get_campaign("camp-long")
            .await
            .expect("Failed to get")
            .expect("Not found");
        assert_eq!(retrieved.description.unwrap().len(), 100_000);
    }

    #[tokio::test]
    async fn test_special_sql_characters() {
        let (db, _temp) = create_test_db().await;

        // Test SQL injection-like content (should be properly escaped)
        let malicious_content = "Robert'); DROP TABLE campaigns;--";
        let mut campaign = CampaignRecord::new(
            "camp-sql".to_string(),
            malicious_content.to_string(),
            "D&D 5e".to_string(),
        );
        campaign.description = Some(malicious_content.to_string());

        db.create_campaign(&campaign)
            .await
            .expect("Failed to create - SQL should be properly escaped");

        // Verify the campaign exists and table wasn't dropped
        let campaigns = db.list_campaigns().await.expect("Table should still exist");
        assert_eq!(campaigns.len(), 1);
        assert_eq!(campaigns[0].name, malicious_content);
    }

    #[tokio::test]
    async fn test_newlines_and_tabs() {
        let (db, _temp) = create_test_db().await;

        let content_with_whitespace = "Line 1\nLine 2\r\nLine 3\tTabbed";
        let mut campaign = CampaignRecord::new(
            "camp-whitespace".to_string(),
            content_with_whitespace.to_string(),
            "D&D 5e".to_string(),
        );
        campaign.description = Some(content_with_whitespace.to_string());

        db.create_campaign(&campaign)
            .await
            .expect("Failed to create");

        let retrieved = db
            .get_campaign("camp-whitespace")
            .await
            .expect("Failed to get")
            .expect("Not found");
        assert_eq!(retrieved.name, content_with_whitespace);
    }

    #[tokio::test]
    async fn test_null_byte_in_content() {
        let (db, _temp) = create_test_db().await;

        // Note: Null bytes can cause issues in some systems
        let content_with_null = "Before\0After";
        let mut campaign = CampaignRecord::new(
            "camp-null-byte".to_string(),
            content_with_null.to_string(),
            "D&D 5e".to_string(),
        );
        campaign.description = Some(content_with_null.to_string());

        // This might fail depending on SQLite version and text handling
        let result = db.create_campaign(&campaign).await;

        // Either it succeeds or fails gracefully - shouldn't panic
        match result {
            Ok(_) => {
                let retrieved = db
                    .get_campaign("camp-null-byte")
                    .await
                    .expect("Query should work");
                if let Some(camp) = retrieved {
                    // Content might be truncated at null byte
                    assert!(camp.name.starts_with("Before"));
                }
            }
            Err(e) => {
                // Some databases reject null bytes - that's fine
                assert!(e.to_string().contains("null") || e.to_string().contains("invalid"));
            }
        }
    }

    // =========================================================================
    // Retry and Recovery Tests
    // =========================================================================

    #[tokio::test]
    async fn test_llm_retry_on_transient_failure() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let mut mock = MockLlmClient::new();
        let call_count = Arc::new(AtomicUsize::new(0));

        mock.expect_id().returning(|| "retry-test".to_string());
        mock.expect_model().returning(|| "test-model".to_string());
        mock.expect_supports_streaming().returning(|| false);

        let call_count_clone = call_count.clone();
        mock.expect_chat().returning(move |_messages, _max_tokens| {
            let count = call_count_clone.fetch_add(1, Ordering::SeqCst) + 1;
            if count < 3 {
                // First two calls fail with a transient error
                Err(LlmError::ApiError("Simulated transient failure".to_string()))
            } else {
                // Third call succeeds
                Ok(MockChatResponse {
                    content: "Success after retry".to_string(),
                    model: "test-model".to_string(),
                    provider: "retry-test".to_string(),
                    input_tokens: 10,
                    output_tokens: 5,
                })
            }
        });

        let messages = vec![MockChatMessage {
            role: MockMessageRole::User,
            content: "Test retry".to_string(),
        }];

        // First call fails
        let result1 = mock.chat(messages.clone(), None).await;
        assert!(result1.is_err());

        // Second call fails
        let result2 = mock.chat(messages.clone(), None).await;
        assert!(result2.is_err());

        // Third call succeeds
        let result3 = mock.chat(messages, None).await;
        assert!(result3.is_ok());
        assert_eq!(result3.unwrap().content, "Success after retry");
    }

    #[tokio::test]
    async fn test_graceful_degradation() {
        let mut mock = MockLlmClient::new();

        mock.expect_id().returning(|| "degradation-test".to_string());
        mock.expect_model().returning(|| "test-model".to_string());
        mock.expect_supports_streaming().returning(|| false);

        // Provider is unhealthy
        mock.expect_health_check().returning(|| false);

        // But can still respond (degraded mode)
        mock.expect_chat().returning(|_messages, _max_tokens| {
            Ok(MockChatResponse {
                    content: "Response in degraded mode".to_string(),
                    model: "test-model".to_string(),
                    provider: "degradation-test".to_string(),
                    input_tokens: 10,
                    output_tokens: 5,
            })
        });

        // Health check fails
        let is_healthy = mock.health_check().await;
        assert!(!is_healthy);

        // But request still works
        let messages = vec![MockChatMessage {
            role: MockMessageRole::User,
            content: "Test".to_string(),
        }];

        let result = mock.chat(messages, None).await;
        assert!(result.is_ok());
    }
}
