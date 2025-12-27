#[cfg(test)]
mod ipc_integration_tests {
    use std::sync::Arc;
    use std::time::Duration;
    use serde_json::json;
    
    // Import from the library crate
    use ttrpg_assistant::ipc::{IpcManager, QueueConfig, JsonRpcResponse, JsonRpcNotification, RequestId};
    use ttrpg_assistant::process_manager::{ProcessManager, ProcessConfig};
    
    /// Helper to create a test IPC manager
    fn create_test_ipc_manager() -> Arc<IpcManager> {
        let config = QueueConfig {
            max_concurrent_requests: 5,
            max_queue_size: 10,
            default_timeout_ms: 5000,
            max_retries: 2,
            retry_delay_ms: 100,
            enable_priority_queue: true,
        };
        Arc::new(IpcManager::with_config(config))
    }
    
    #[tokio::test]
    async fn test_request_id_generation() {
        let manager = create_test_ipc_manager();
        
        // Generate multiple IDs and verify they're sequential
        let mut ids = Vec::new();
        for _ in 0..10 {
            let id = manager.next_request_id().await;
            if let RequestId::Number(n) = id {
                ids.push(n);
            }
        }
        
        // Check that IDs are sequential
        for i in 1..ids.len() {
            assert_eq!(ids[i], ids[i-1] + 1, "IDs should be sequential");
        }
    }
    
    #[tokio::test]
    async fn test_concurrent_request_limit() {
        let manager = create_test_ipc_manager();
        
        // Set up a mock stdin channel
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        manager.set_stdin_channel(tx).await;
        
        // Create more requests than the concurrent limit
        let mut handles = Vec::new();
        for i in 0..10 {
            let manager_clone = manager.clone();
            let handle = tokio::spawn(async move {
                manager_clone.send_request(
                    format!("test_method_{}", i),
                    json!({"test": true}),
                    Some(Duration::from_secs(1)),
                    None,
                    false,
                ).await
            });
            handles.push(handle);
            
            // Small delay to ensure ordering
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        // Check that requests are being queued
        let metrics = manager.get_metrics().await;
        assert!(metrics.queued_requests > 0, "Some requests should be queued");
        assert!(metrics.active_requests <= 5, "Active requests should not exceed limit");
        
        // Cancel all to clean up
        manager.cancel_all_requests().await;
    }
    
    #[tokio::test]
    async fn test_request_timeout() {
        let manager = create_test_ipc_manager();
        
        // Set up a mock stdin channel
        let (tx, _rx) = tokio::sync::mpsc::channel(100);
        manager.set_stdin_channel(tx).await;
        
        // Send a request with very short timeout
        let result = manager.send_request(
            "timeout_test".to_string(),
            json!({}),
            Some(Duration::from_millis(10)),
            None,
            false,
        ).await;
        
        // Should timeout
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.message.contains("timeout") || e.message.contains("Timeout"));
        }
        
        // Check metrics
        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.timeouts, 1, "Should record timeout");
    }
    
    #[tokio::test]
    async fn test_response_handling() {
        let manager = create_test_ipc_manager();
        
        // Set up channels
        let (tx, _rx) = tokio::sync::mpsc::channel(100);
        manager.set_stdin_channel(tx).await;
        
        // Send a request in background
        let manager_clone = manager.clone();
        let request_handle = tokio::spawn(async move {
            manager_clone.send_request(
                "test_method".to_string(),
                json!({"key": "value"}),
                Some(Duration::from_secs(5)),
                None,
                false,
            ).await
        });
        
        // Give it time to register
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Simulate receiving a response
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(RequestId::Number(1)),
            result: Some(json!({"success": true})),
            error: None,
        };
        
        manager.handle_response(response).await;
        
        // Wait for the request to complete
        let result = request_handle.await.unwrap();
        assert!(result.is_ok());
        if let Ok(value) = result {
            assert_eq!(value.get("success"), Some(&json!(true)));
        }
    }
    
    #[tokio::test]
    async fn test_notification_handling() {
        let manager = create_test_ipc_manager();
        
        // Set up notification channel
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        manager.set_notification_channel(tx).await;
        
        // Send a notification
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: "test_notification".to_string(),
            params: json!({"data": "test"}),
        };
        
        manager.handle_notification(notification.clone()).await;
        
        // Check that notification was forwarded
        let received = rx.recv().await;
        assert!(received.is_some());
        if let Some(notif) = received {
            assert_eq!(notif.method, "test_notification");
            assert_eq!(notif.params, json!({"data": "test"}));
        }
    }
    
    #[tokio::test]
    async fn test_request_cancellation() {
        let manager = create_test_ipc_manager();
        
        // Set up a mock stdin channel
        let (tx, _rx) = tokio::sync::mpsc::channel(100);
        manager.set_stdin_channel(tx).await;
        
        // Send a long-running request
        let manager_clone = manager.clone();
        let request_handle = tokio::spawn(async move {
            manager_clone.send_request(
                "long_running".to_string(),
                json!({}),
                Some(Duration::from_secs(10)),
                None,
                false,
            ).await
        });
        
        // Give it time to register
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Cancel the request
        let cancelled = manager.cancel_request(1).await;
        assert!(cancelled, "Should be able to cancel request");
        
        // Wait for the request to complete
        let result = request_handle.await.unwrap();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.message.contains("cancelled") || e.message.contains("Cancelled"));
        }
    }
    
    #[tokio::test]
    async fn test_metrics_tracking() {
        let manager = create_test_ipc_manager();
        
        // Record some latencies
        for latency in [10.0, 20.0, 30.0, 15.0, 25.0] {
            manager.record_latency(latency).await;
        }
        
        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.average_latency_ms, 20.0);
        assert_eq!(metrics.min_latency_ms, 10.0);
        assert_eq!(metrics.max_latency_ms, 30.0);
        
        // Reset metrics
        manager.reset_metrics().await;
        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.average_latency_ms, 0.0);
    }
    
    #[tokio::test]
    async fn test_queue_priority() {
        let manager = create_test_ipc_manager();
        
        // Set up a mock stdin channel with limited capacity
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        manager.set_stdin_channel(tx).await;
        
        // Fill up concurrent request limit with low priority requests
        let mut handles = Vec::new();
        for i in 0..5 {
            let manager_clone = manager.clone();
            let handle = tokio::spawn(async move {
                manager_clone.send_request(
                    format!("low_priority_{}", i),
                    json!({}),
                    Some(Duration::from_secs(10)),
                    Some(10), // Low priority
                    false,
                ).await
            });
            handles.push(handle);
        }
        
        // Add high priority request that should be queued
        let manager_clone = manager.clone();
        let high_priority_handle = tokio::spawn(async move {
            manager_clone.send_request(
                "high_priority".to_string(),
                json!({}),
                Some(Duration::from_secs(10)),
                Some(1), // High priority
                false,
            ).await
        });
        
        // Give time for requests to be queued
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Check that requests are queued
        let metrics = manager.get_metrics().await;
        assert!(metrics.queued_requests > 0);
        
        // Clean up
        manager.cancel_all_requests().await;
        for handle in handles {
            let _ = handle.await;
        }
        let _ = high_priority_handle.await;
    }
    
    #[tokio::test]
    async fn test_disconnect_cleanup() {
        let manager = create_test_ipc_manager();
        
        // Set up channels
        let (tx, _rx) = tokio::sync::mpsc::channel(100);
        manager.set_stdin_channel(tx).await;
        
        // Add some pending requests
        for i in 0..5 {
            let manager_clone = manager.clone();
            tokio::spawn(async move {
                let _ = manager_clone.send_request(
                    format!("test_{}", i),
                    json!({}),
                    Some(Duration::from_secs(10)),
                    None,
                    false,
                ).await;
            });
        }
        
        // Give time for requests to register
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Disconnect should cancel all requests
        manager.disconnect().await;
        
        // Check that everything is cleaned up
        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.active_requests, 0);
        assert_eq!(metrics.queued_requests, 0);
        
        // Should not be able to send new requests
        let result = manager.send_request(
            "after_disconnect".to_string(),
            json!({}),
            None,
            None,
            false,
        ).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_stream_chunk_handling() {
        let manager = create_test_ipc_manager();
        
        // Set up channels
        let (tx, _rx) = tokio::sync::mpsc::channel(100);
        manager.set_stdin_channel(tx).await;
        
        // Simulate receiving stream chunks
        use ttrpg_assistant::ipc::StreamChunk;
        
        let chunks = vec![
            StreamChunk {
                id: RequestId::Number(1),
                sequence: 0,
                total_chunks: Some(3),
                data: b"First chunk".to_vec(),
                is_final: false,
            },
            StreamChunk {
                id: RequestId::Number(1),
                sequence: 1,
                total_chunks: Some(3),
                data: b"Second chunk".to_vec(),
                is_final: false,
            },
            StreamChunk {
                id: RequestId::Number(1),
                sequence: 2,
                total_chunks: Some(3),
                data: b"Final chunk".to_vec(),
                is_final: true,
            },
        ];
        
        for chunk in chunks {
            manager.handle_stream_chunk(chunk).await;
        }
        
        // Verify chunks were buffered and assembled
        // In real implementation, would check the assembled result
    }
}

#[cfg(test)]
mod process_integration_tests {
    use std::time::Duration;
    use ttrpg_assistant::process_manager::{ProcessManager, ProcessConfig, ProcessState, HealthStatus};
    
    #[tokio::test]
    async fn test_process_lifecycle() {
        let manager = ProcessManager::new();
        
        // Initial state should be stopped
        let stats = manager.get_stats().await;
        assert_eq!(stats.state, ProcessState::Stopped);
        assert_eq!(stats.health, HealthStatus::Unknown);
        
        // Simulate process start
        manager.on_process_started(12345).await;
        let stats = manager.get_stats().await;
        assert_eq!(stats.state, ProcessState::Running);
        assert_eq!(stats.pid, Some(12345));
        assert!(stats.start_time.is_some());
        
        // Simulate process stop
        manager.on_process_stopped(Some(0)).await;
        let stats = manager.get_stats().await;
        assert_eq!(stats.state, ProcessState::Stopped);
        assert_eq!(stats.pid, None);
    }
    
    #[tokio::test]
    async fn test_health_check_tracking() {
        let config = ProcessConfig {
            max_health_check_failures: 3,
            auto_restart_on_crash: true,
            ..Default::default()
        };
        let manager = ProcessManager::with_config(config);
        
        // Start process
        manager.on_process_started(12345).await;
        
        // Simulate successful health check
        manager.on_health_check_result(true, None).await;
        let stats = manager.get_stats().await;
        assert_eq!(stats.health, HealthStatus::Healthy);
        assert_eq!(stats.health_check_failures, 0);
        
        // Simulate failed health checks
        for i in 1..=2 {
            manager.on_health_check_result(false, Some(format!("Error {}", i))).await;
            let stats = manager.get_stats().await;
            assert_eq!(stats.health, HealthStatus::Degraded);
            assert_eq!(stats.health_check_failures, i);
        }
        
        // Third failure should mark as unhealthy
        manager.on_health_check_result(false, Some("Error 3".to_string())).await;
        let stats = manager.get_stats().await;
        assert_eq!(stats.health, HealthStatus::Unhealthy);
        assert_eq!(stats.health_check_failures, 3);
    }
    
    #[tokio::test]
    async fn test_restart_counter() {
        let manager = ProcessManager::new();
        
        // Initial restart count should be 0
        let stats = manager.get_stats().await;
        assert_eq!(stats.restart_count, 0);
        
        // Simulate crash and restart attempts
        manager.on_process_started(12345).await;
        manager.on_process_stopped(Some(1)).await; // Non-zero exit code = crash
        
        // Check if should restart
        let should_restart = manager.should_restart().await;
        assert!(should_restart, "Should attempt restart after crash");
        
        // Reset counter
        manager.reset_restart_count().await;
        let stats = manager.get_stats().await;
        assert_eq!(stats.restart_count, 0);
    }
    
    #[tokio::test]
    async fn test_event_history() {
        let manager = ProcessManager::new();
        
        // Generate some events
        manager.on_process_started(12345).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        manager.on_health_check_result(true, None).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        manager.on_process_stopped(Some(0)).await;
        
        // Get recent events
        let events = manager.get_recent_events(10).await;
        assert!(events.len() >= 3, "Should have at least 3 events");
        
        // Clear events
        manager.clear_events().await;
        let events = manager.get_recent_events(10).await;
        assert_eq!(events.len(), 0, "Events should be cleared");
    }
}