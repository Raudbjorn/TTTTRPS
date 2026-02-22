#[cfg(test)]
mod tests {
    use super::super::process_manager::*;
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};

    // Helper to create a test process manager
    fn create_test_manager() -> Arc<ProcessManager> {
        let config = ProcessConfig {
            max_restart_attempts: 2,
            restart_delay_ms: 100,
            health_check_interval_ms: 100,
            health_check_timeout_ms: 50,
            max_health_check_failures: 2,
            resource_monitor_interval_ms: 100,
            cpu_alert_threshold: 50.0,
            memory_alert_threshold: 100.0,
            auto_restart_on_crash: true,
            graceful_shutdown_timeout_ms: 100,
        };
        Arc::new(ProcessManager::with_config(config))
    }

    #[tokio::test]
    async fn test_process_lifecycle() {
        let manager = create_test_manager();
        
        // Initial state should be stopped
        let stats = manager.get_stats().await;
        assert_eq!(stats.state, ProcessState::Stopped);
        assert_eq!(stats.health, HealthStatus::Unknown);
        assert!(stats.pid.is_none());
        
        // Simulate process start
        manager.on_process_started(1234).await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.state, ProcessState::Running);
        assert_eq!(stats.pid, Some(1234));
        assert!(stats.start_time.is_some());
        
        // Simulate process stop
        manager.on_process_stopped(Some(0)).await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.state, ProcessState::Stopped);
        assert!(stats.pid.is_none());
    }

    #[tokio::test]
    async fn test_crash_detection_and_restart() {
        let manager = create_test_manager();
        
        // Start process
        manager.on_process_started(1234).await;
        
        // Simulate crash (non-zero exit code)
        manager.on_process_stopped(Some(1)).await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.state, ProcessState::Crashed);
        
        // Check that restart was scheduled
        sleep(Duration::from_millis(50)).await;
        
        let should_restart = manager.should_restart().await;
        assert!(should_restart);
    }

    #[tokio::test]
    async fn test_restart_limit() {
        let manager = create_test_manager();
        
        // Simulate multiple crashes
        for i in 0..3 {
            manager.on_process_started(1234 + i).await;
            manager.on_process_stopped(Some(1)).await;
            
            let stats = manager.get_stats().await;
            
            if i < 2 {
                // Should still attempt restart
                assert!(manager.should_restart().await);
            } else {
                // Should not restart after max attempts
                assert!(!manager.should_restart().await);
            }
        }
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.restart_count, 2); // Max attempts reached
    }

    #[tokio::test]
    async fn test_health_check_failures() {
        let manager = create_test_manager();
        
        manager.on_process_started(1234).await;
        
        // Simulate health check failures
        for i in 0..3 {
            manager.on_health_check_result(false, Some(format!("Health check {} failed", i))).await;
            
            let stats = manager.get_stats().await;
            
            if i < 1 {
                assert_eq!(stats.health, HealthStatus::Degraded);
            } else {
                assert_eq!(stats.health, HealthStatus::Unhealthy);
            }
        }
        
        // After max failures, should trigger restart
        assert!(manager.should_restart().await);
    }

    #[tokio::test]
    async fn test_health_recovery() {
        let manager = create_test_manager();
        
        manager.on_process_started(1234).await;
        
        // Fail once
        manager.on_health_check_result(false, Some("Temporary failure".to_string())).await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.health, HealthStatus::Degraded);
        assert_eq!(stats.health_check_failures, 1);
        
        // Then recover
        manager.on_health_check_result(true, None).await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.health, HealthStatus::Healthy);
        assert_eq!(stats.health_check_failures, 0);
    }

    #[tokio::test]
    async fn test_event_tracking() {
        let manager = create_test_manager();
        
        // Generate some events
        manager.on_process_started(1234).await;
        manager.on_health_check_result(true, None).await;
        manager.on_process_stopped(Some(0)).await;
        
        let events = manager.get_recent_events(10).await;
        assert!(events.len() >= 3);
        
        // Check event types
        let has_started = events.iter().any(|e| matches!(e, ProcessEvent::Started { .. }));
        let has_stopped = events.iter().any(|e| matches!(e, ProcessEvent::Stopped { .. }));
        let has_health_passed = events.iter().any(|e| matches!(e, ProcessEvent::HealthCheckPassed { .. }));
        
        assert!(has_started);
        assert!(has_stopped);
        assert!(has_health_passed);
    }

    #[tokio::test]
    async fn test_restart_count_reset() {
        let manager = create_test_manager();
        
        // Simulate a crash
        manager.on_process_started(1234).await;
        manager.on_process_stopped(Some(1)).await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.restart_count, 1);
        
        // Reset counter
        manager.reset_restart_count().await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.restart_count, 0);
    }

    #[tokio::test]
    async fn test_config_update() {
        let manager = create_test_manager();
        
        let new_config = ProcessConfig {
            max_restart_attempts: 5,
            restart_delay_ms: 500,
            health_check_interval_ms: 1000,
            health_check_timeout_ms: 200,
            max_health_check_failures: 5,
            resource_monitor_interval_ms: 500,
            cpu_alert_threshold: 90.0,
            memory_alert_threshold: 1000.0,
            auto_restart_on_crash: false,
            graceful_shutdown_timeout_ms: 2000,
        };
        
        manager.update_config(new_config.clone()).await;
        
        // Config is internal, but we can test its effects
        // For example, after update, auto_restart should be false
        manager.on_process_started(1234).await;
        manager.on_process_stopped(Some(1)).await;
        
        // Should not restart because auto_restart_on_crash is false
        assert!(!manager.should_restart().await);
    }

    #[tokio::test]
    async fn test_graceful_vs_forced_shutdown() {
        let manager = create_test_manager();
        
        // Start process
        manager.on_process_started(1234).await;
        
        // Graceful shutdown (exit code 0)
        manager.on_process_stopped(Some(0)).await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.state, ProcessState::Stopped);
        // Should not be marked as crashed
        assert_ne!(stats.state, ProcessState::Crashed);
        
        // Start again
        manager.on_process_started(1235).await;
        
        // Forced shutdown (non-zero exit code)
        manager.on_process_stopped(Some(137)).await; // SIGKILL exit code
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.state, ProcessState::Crashed);
    }

    #[tokio::test]
    async fn test_event_limit() {
        let manager = create_test_manager();
        
        // Generate many events
        for i in 0..20 {
            manager.on_process_started(1234 + i).await;
            manager.on_process_stopped(Some(0)).await;
        }
        
        // Request limited events
        let events = manager.get_recent_events(5).await;
        assert_eq!(events.len(), 5);
        
        // Events should be the most recent ones
        let last_event = events.last().unwrap();
        assert!(matches!(last_event, ProcessEvent::Stopped { .. }));
    }
}