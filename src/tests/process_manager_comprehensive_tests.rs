#[cfg(test)]
mod comprehensive_process_manager_tests {
    use crate::process_manager::*;
    use std::sync::Arc;
    use std::time::{Duration, SystemTime};
    use tokio::time::{sleep, timeout};
    use serde_json::json;

    // Test helper to create process manager with short timeouts for fast testing
    fn create_fast_test_manager() -> Arc<ProcessManager> {
        let config = ProcessConfig {
            max_restart_attempts: 3,
            restart_delay_ms: 50,
            health_check_interval_ms: 100,
            health_check_timeout_ms: 50,
            max_health_check_failures: 2,
            resource_monitor_interval_ms: 100,
            cpu_alert_threshold: 80.0,
            memory_alert_threshold: 100.0,
            auto_restart_on_crash: true,
            graceful_shutdown_timeout_ms: 100,
        };
        Arc::new(ProcessManager::with_config(config))
    }

    #[tokio::test]
    async fn test_process_config_default() {
        let config = ProcessConfig::default();
        assert_eq!(config.max_restart_attempts, 3);
        assert_eq!(config.restart_delay_ms, 2000);
        assert_eq!(config.health_check_interval_ms, 30000);
        assert_eq!(config.health_check_timeout_ms, 5000);
        assert_eq!(config.max_health_check_failures, 3);
        assert_eq!(config.resource_monitor_interval_ms, 10000);
        assert_eq!(config.cpu_alert_threshold, 80.0);
        assert_eq!(config.memory_alert_threshold, 500.0);
        assert!(config.auto_restart_on_crash);
        assert_eq!(config.graceful_shutdown_timeout_ms, 5000);
    }

    #[tokio::test]
    async fn test_process_stats_default() {
        let stats = ProcessStats::default();
        assert_eq!(stats.state, ProcessState::Stopped);
        assert_eq!(stats.health, HealthStatus::Unknown);
        assert!(stats.pid.is_none());
        assert!(stats.start_time.is_none());
        assert_eq!(stats.restart_count, 0);
        assert_eq!(stats.health_check_failures, 0);
        assert!(stats.last_health_check.is_none());
        assert!(stats.resource_usage.is_none());
        assert!(stats.events.is_empty());
    }

    #[tokio::test]
    async fn test_process_manager_creation() {
        let manager = ProcessManager::new();
        let stats = manager.get_stats().await;
        assert_eq!(stats.state, ProcessState::Stopped);
        assert_eq!(stats.health, HealthStatus::Unknown);
        assert_eq!(stats.restart_count, 0);
    }

    #[tokio::test]
    async fn test_process_manager_with_custom_config() {
        let config = ProcessConfig {
            max_restart_attempts: 5,
            restart_delay_ms: 1000,
            auto_restart_on_crash: false,
            ..Default::default()
        };
        let manager = ProcessManager::with_config(config);
        
        // Start and crash a process
        manager.on_process_started(1234).await;
        manager.on_process_stopped(Some(1)).await; // Crash
        
        // Should not restart because auto_restart_on_crash is false
        assert!(!manager.should_restart().await);
    }

    #[tokio::test]
    async fn test_process_lifecycle_complete() {
        let manager = create_fast_test_manager();
        
        // Initial state
        let stats = manager.get_stats().await;
        assert_eq!(stats.state, ProcessState::Stopped);
        assert_eq!(stats.health, HealthStatus::Unknown);
        assert!(stats.pid.is_none());
        assert!(stats.start_time.is_none());
        
        // Start process
        let start_time_before = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        manager.on_process_started(1234).await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.state, ProcessState::Running);
        assert_eq!(stats.health, HealthStatus::Unknown); // Initially unknown
        assert_eq!(stats.pid, Some(1234));
        assert!(stats.start_time.is_some());
        assert!(stats.start_time.unwrap() >= start_time_before);
        assert_eq!(stats.health_check_failures, 0);
        
        // Verify event was recorded
        let events = manager.get_recent_events(10).await;
        assert_eq!(events.len(), 1);
        match &events[0] {
            ProcessEvent::Started { pid, timestamp: _ } => {
                assert_eq!(*pid, 1234);
            },
            _ => panic!("Expected Started event"),
        }
        
        // Stop process gracefully
        manager.on_process_stopped(Some(0)).await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.state, ProcessState::Stopped);
        assert_eq!(stats.health, HealthStatus::Unknown);
        assert!(stats.pid.is_none());
        
        // Should have both Started and Stopped events
        let events = manager.get_recent_events(10).await;
        assert_eq!(events.len(), 2);
        match &events[1] {
            ProcessEvent::Stopped { exit_code, timestamp: _ } => {
                assert_eq!(*exit_code, Some(0));
            },
            _ => panic!("Expected Stopped event"),
        }
    }

    #[tokio::test]
    async fn test_process_crash_detection() {
        let manager = create_fast_test_manager();
        
        // Start process
        manager.on_process_started(1234).await;
        
        // Crash with various exit codes
        for exit_code in [1, -1, 139, 255] {
            // Reset for each test
            manager.on_process_started(1234 + exit_code as u32).await;
            manager.on_process_stopped(Some(exit_code)).await;
            
            let stats = manager.get_stats().await;
            assert_eq!(stats.state, ProcessState::Crashed);
            
            // Should trigger restart logic
            assert!(manager.should_restart().await);
            
            // Reset for next iteration
            manager.reset_restart_count().await;
        }
    }

    #[tokio::test]
    async fn test_restart_attempt_limiting() {
        let manager = create_fast_test_manager();
        
        // Simulate multiple crashes
        for attempt in 1..=5 {
            manager.on_process_started(1000 + attempt as u32).await;
            manager.on_process_stopped(Some(1)).await; // Crash
            
            let stats = manager.get_stats().await;
            assert_eq!(stats.state, ProcessState::Crashed);
            assert_eq!(stats.restart_count, attempt as u32);
            
            if attempt < 3 {
                // Should still attempt restart within limit
                assert!(manager.should_restart().await);
                
                // Simulate restart delay
                sleep(Duration::from_millis(60)).await;
                
                // Check that state changed to restarting
                let stats = manager.get_stats().await;
                assert_eq!(stats.state, ProcessState::Restarting);
            } else {
                // Should not restart after max attempts
                assert!(!manager.should_restart().await);
            }
        }
        
        let final_stats = manager.get_stats().await;
        assert_eq!(final_stats.restart_count, 3); // Stopped at max attempts
    }

    #[tokio::test]
    async fn test_health_check_progression() {
        let manager = create_fast_test_manager();
        
        // Start process
        manager.on_process_started(1234).await;
        
        // Initial health check success
        manager.on_health_check_result(true, None).await;
        let stats = manager.get_stats().await;
        assert_eq!(stats.health, HealthStatus::Healthy);
        assert_eq!(stats.health_check_failures, 0);
        assert!(stats.last_health_check.is_some());
        
        // First failure - should be degraded
        manager.on_health_check_result(false, Some("Minor issue".to_string())).await;
        let stats = manager.get_stats().await;
        assert_eq!(stats.health, HealthStatus::Degraded);
        assert_eq!(stats.health_check_failures, 1);
        
        // Second failure - still degraded but failure count increases
        manager.on_health_check_result(false, Some("Another issue".to_string())).await;
        let stats = manager.get_stats().await;
        assert_eq!(stats.health, HealthStatus::Unhealthy); // Reaches max failures
        assert_eq!(stats.health_check_failures, 2);
        
        // Should trigger restart due to health failures
        assert!(manager.should_restart().await);
        
        // Recovery should reset failure count
        manager.on_health_check_result(true, None).await;
        let stats = manager.get_stats().await;
        assert_eq!(stats.health, HealthStatus::Healthy);
        assert_eq!(stats.health_check_failures, 0);
    }

    #[tokio::test]
    async fn test_event_history_management() {
        let manager = create_fast_test_manager();
        
        // Generate many events
        for i in 0..20 {
            manager.on_process_started(1000 + i).await;
            manager.on_health_check_result(i % 2 == 0, None).await;
            manager.on_process_stopped(Some(if i % 3 == 0 { 0 } else { 1 })).await;
        }
        
        // Test event retrieval with limits
        let all_events = manager.get_recent_events(100).await;
        assert_eq!(all_events.len(), 60); // 20 * 3 events each
        
        let limited_events = manager.get_recent_events(10).await;
        assert_eq!(limited_events.len(), 10);
        
        // Events should be in chronological order (most recent last)
        let limited_events = manager.get_recent_events(5).await;
        assert_eq!(limited_events.len(), 5);
        
        // Clear events
        manager.clear_events().await;
        let events_after_clear = manager.get_recent_events(100).await;
        assert_eq!(events_after_clear.len(), 0);
    }

    #[tokio::test]
    async fn test_config_update() {
        let manager = ProcessManager::new();
        
        let new_config = ProcessConfig {
            max_restart_attempts: 10,
            restart_delay_ms: 5000,
            health_check_interval_ms: 60000,
            auto_restart_on_crash: false,
            cpu_alert_threshold: 95.0,
            memory_alert_threshold: 1000.0,
            ..Default::default()
        };
        
        manager.update_config(new_config.clone()).await;
        
        // Verify the config affects behavior
        manager.on_process_started(1234).await;
        manager.on_process_stopped(Some(1)).await; // Crash
        
        // Should not restart because auto_restart_on_crash is false
        assert!(!manager.should_restart().await);
    }

    #[tokio::test]
    async fn test_restart_count_reset() {
        let manager = create_fast_test_manager();
        
        // Cause some restarts
        for i in 0..2 {
            manager.on_process_started(1000 + i).await;
            manager.on_process_stopped(Some(1)).await;
        }
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.restart_count, 2);
        
        // Reset count
        manager.reset_restart_count().await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.restart_count, 0);
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let manager = Arc::new(create_fast_test_manager());
        
        // Start multiple concurrent operations
        let manager1 = manager.clone();
        let manager2 = manager.clone();
        let manager3 = manager.clone();
        
        let handle1 = tokio::spawn(async move {
            for i in 0..10 {
                manager1.on_process_started(1000 + i).await;
                sleep(Duration::from_millis(10)).await;
                manager1.on_process_stopped(Some(0)).await;
                sleep(Duration::from_millis(10)).await;
            }
        });
        
        let handle2 = tokio::spawn(async move {
            for i in 0..10 {
                manager2.on_health_check_result(i % 2 == 0, None).await;
                sleep(Duration::from_millis(15)).await;
            }
        });
        
        let handle3 = tokio::spawn(async move {
            for _ in 0..5 {
                let _ = manager3.get_stats().await;
                let _ = manager3.get_recent_events(10).await;
                sleep(Duration::from_millis(20)).await;
            }
        });
        
        // Wait for all operations to complete
        let results = tokio::try_join!(handle1, handle2, handle3);
        assert!(results.is_ok());
        
        // Verify final state is consistent
        let final_stats = manager.get_stats().await;
        assert!(final_stats.events.len() > 0);
    }

    #[tokio::test]
    async fn test_edge_cases() {
        let manager = create_fast_test_manager();
        
        // Test multiple starts without stops
        manager.on_process_started(1234).await;
        manager.on_process_started(1235).await; // Should update PID
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.pid, Some(1235));
        
        // Test stop without start
        let manager2 = ProcessManager::new();
        manager2.on_process_stopped(Some(0)).await;
        let stats2 = manager2.get_stats().await;
        assert_eq!(stats2.state, ProcessState::Stopped);
        
        // Test health check without running process
        let manager3 = ProcessManager::new();
        manager3.on_health_check_result(false, Some("Error".to_string())).await;
        // Should not crash
        
        // Test very high restart attempts
        let high_config = ProcessConfig {
            max_restart_attempts: u32::MAX,
            ..Default::default()
        };
        let manager4 = ProcessManager::with_config(high_config);
        
        for _ in 0..100 {
            manager4.on_process_started(1234).await;
            manager4.on_process_stopped(Some(1)).await;
        }
        
        let stats4 = manager4.get_stats().await;
        assert_eq!(stats4.restart_count, 100);
        assert!(manager4.should_restart().await); // Should still allow restart
    }

    #[tokio::test]
    async fn test_process_events_serialization() {
        let started_event = ProcessEvent::Started {
            pid: 1234,
            timestamp: 1640000000,
        };
        
        let stopped_event = ProcessEvent::Stopped {
            exit_code: Some(0),
            timestamp: 1640000001,
        };
        
        let crashed_event = ProcessEvent::Crashed {
            error: "Process crashed".to_string(),
            timestamp: 1640000002,
        };
        
        let restart_event = ProcessEvent::Restarting {
            attempt: 2,
            max_attempts: 3,
            timestamp: 1640000003,
        };
        
        let health_failed_event = ProcessEvent::HealthCheckFailed {
            reason: "Health check timeout".to_string(),
            timestamp: 1640000004,
        };
        
        let health_passed_event = ProcessEvent::HealthCheckPassed {
            timestamp: 1640000005,
        };
        
        let resource_alert_event = ProcessEvent::ResourceAlert {
            alert_type: "cpu_high".to_string(),
            value: 95.5,
            threshold: 80.0,
            timestamp: 1640000006,
        };
        
        // Test serialization/deserialization
        let events = vec![
            started_event, stopped_event, crashed_event, restart_event,
            health_failed_event, health_passed_event, resource_alert_event,
        ];
        
        for event in events {
            let serialized = serde_json::to_string(&event).unwrap();
            let deserialized: ProcessEvent = serde_json::from_str(&serialized).unwrap();
            
            // Compare based on discriminant since we can't derive PartialEq easily
            match (&event, &deserialized) {
                (ProcessEvent::Started { pid: p1, .. }, ProcessEvent::Started { pid: p2, .. }) => {
                    assert_eq!(p1, p2);
                },
                (ProcessEvent::Stopped { exit_code: e1, .. }, ProcessEvent::Stopped { exit_code: e2, .. }) => {
                    assert_eq!(e1, e2);
                },
                (ProcessEvent::Crashed { error: e1, .. }, ProcessEvent::Crashed { error: e2, .. }) => {
                    assert_eq!(e1, e2);
                },
                _ => {}, // Other variants would need similar matching
            }
        }
    }

    #[tokio::test]
    async fn test_resource_usage_tracking() {
        let resource_usage = ResourceUsage {
            cpu_percent: 45.7,
            memory_mb: 128.5,
            uptime_seconds: 3600,
            timestamp: 1640000000,
        };
        
        // Test serialization
        let serialized = serde_json::to_string(&resource_usage).unwrap();
        let deserialized: ResourceUsage = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.cpu_percent, 45.7);
        assert_eq!(deserialized.memory_mb, 128.5);
        assert_eq!(deserialized.uptime_seconds, 3600);
        assert_eq!(deserialized.timestamp, 1640000000);
    }

    #[tokio::test]
    async fn test_process_state_transitions() {
        let manager = create_fast_test_manager();
        
        // Test all possible state transitions
        assert_eq!(manager.get_stats().await.state, ProcessState::Stopped);
        
        // Stopped -> Running
        manager.on_process_started(1234).await;
        assert_eq!(manager.get_stats().await.state, ProcessState::Running);
        
        // Running -> Stopped (graceful)
        manager.on_process_stopped(Some(0)).await;
        assert_eq!(manager.get_stats().await.state, ProcessState::Stopped);
        
        // Running -> Crashed
        manager.on_process_started(1235).await;
        manager.on_process_stopped(Some(1)).await;
        assert_eq!(manager.get_stats().await.state, ProcessState::Crashed);
        
        // Crashed -> Restarting (automatic)
        sleep(Duration::from_millis(60)).await; // Allow restart logic to run
        let stats = manager.get_stats().await;
        assert_eq!(stats.state, ProcessState::Restarting);
    }

    #[tokio::test]
    async fn test_health_status_transitions() {
        let manager = create_fast_test_manager();
        
        // Start with unknown health
        manager.on_process_started(1234).await;
        assert_eq!(manager.get_stats().await.health, HealthStatus::Unknown);
        
        // Unknown -> Healthy
        manager.on_health_check_result(true, None).await;
        assert_eq!(manager.get_stats().await.health, HealthStatus::Healthy);
        
        // Healthy -> Degraded (first failure)
        manager.on_health_check_result(false, Some("Issue".to_string())).await;
        assert_eq!(manager.get_stats().await.health, HealthStatus::Degraded);
        
        // Degraded -> Unhealthy (max failures reached)
        manager.on_health_check_result(false, Some("Issue".to_string())).await;
        assert_eq!(manager.get_stats().await.health, HealthStatus::Unhealthy);
        
        // Unhealthy -> Healthy (recovery)
        manager.on_health_check_result(true, None).await;
        assert_eq!(manager.get_stats().await.health, HealthStatus::Healthy);
        
        // Process stop -> Unknown
        manager.on_process_stopped(Some(0)).await;
        assert_eq!(manager.get_stats().await.health, HealthStatus::Unknown);
    }

    #[tokio::test]
    async fn test_timestamp_consistency() {
        let manager = create_fast_test_manager();
        
        let before_start = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        manager.on_process_started(1234).await;
        
        let after_start = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let stats = manager.get_stats().await;
        let start_time = stats.start_time.unwrap();
        
        assert!(start_time >= before_start);
        assert!(start_time <= after_start);
        
        // Test health check timestamp
        let before_health = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        manager.on_health_check_result(true, None).await;
        
        let after_health = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let stats = manager.get_stats().await;
        let health_time = stats.last_health_check.unwrap();
        
        assert!(health_time >= before_health);
        assert!(health_time <= after_health);
    }
}
