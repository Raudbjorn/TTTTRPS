#[cfg(test)]
mod comprehensive_resource_manager_tests {
    use crate::resource_manager::*;
    use std::sync::{Arc, atomic::{AtomicBool, AtomicU32, Ordering}};
    use std::time::{Duration, Instant};
    use tokio::time::{sleep, timeout};
    use std::sync::Mutex as StdMutex;

    // Test helper to create resource manager with fast timeouts
    fn create_fast_test_manager() -> ResourceManager {
        let limits = ResourceLimits {
            max_memory_mb: 100,
            max_processes: 3,
            max_connections: 5,
            max_file_handles: 10,
            max_concurrent_tasks: 5,
            cleanup_timeout_ms: 100,
            stale_resource_timeout_secs: 1, // Very short for testing
        };
        ResourceManager::with_limits(limits)
    }

    #[tokio::test]
    async fn test_resource_manager_creation() {
        let manager = ResourceManager::new();
        let stats = manager.get_stats().await;
        
        assert_eq!(stats.total_resources, 0);
        assert_eq!(stats.active_resources, 0);
        assert_eq!(stats.cleaned_resources, 0);
        assert_eq!(stats.failed_cleanups, 0);
    }

    #[tokio::test]
    async fn test_resource_manager_with_custom_limits() {
        let custom_limits = ResourceLimits {
            max_memory_mb: 512,
            max_processes: 20,
            max_connections: 200,
            max_file_handles: 2000,
            max_concurrent_tasks: 100,
            cleanup_timeout_ms: 30000,
            stale_resource_timeout_secs: 7200,
        };
        
        let manager = ResourceManager::with_limits(custom_limits.clone());
        let stats = manager.get_stats().await;
        
        // Verify initial state
        assert_eq!(stats.total_resources, 0);
        assert_eq!(stats.active_resources, 0);
    }

    #[tokio::test]
    async fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        
        assert_eq!(limits.max_memory_mb, 2048);
        assert_eq!(limits.max_processes, 10);
        assert_eq!(limits.max_connections, 100);
        assert_eq!(limits.max_file_handles, 1000);
        assert_eq!(limits.max_concurrent_tasks, 50);
        assert_eq!(limits.cleanup_timeout_ms, 10000);
        assert_eq!(limits.stale_resource_timeout_secs, 3600);
    }

    #[tokio::test]
    async fn test_resource_registration_basic() {
        let manager = create_fast_test_manager();
        
        let cleanup_called = Arc::new(AtomicBool::new(false));
        let cleanup_called_clone = cleanup_called.clone();
        
        let resource_id = manager.register_resource(
            ResourceType::Task,
            "Test task".to_string(),
            Some(1024),
            false,
            Some(move || {
                let cleanup_called = cleanup_called_clone.clone();
                Box::pin(async move {
                    cleanup_called.store(true, Ordering::Relaxed);
                    Ok(())
                })
            }),
        ).await.unwrap();
        
        assert!(!resource_id.is_empty());
        assert!(resource_id.starts_with("task_"));
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_resources, 1);
        assert_eq!(stats.active_resources, 1);
        
        // Unregister should call cleanup
        manager.unregister_resource(&resource_id).await.unwrap();
        assert!(cleanup_called.load(Ordering::Relaxed));
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.active_resources, 0);
        assert_eq!(stats.cleaned_resources, 1);
    }

    #[tokio::test]
    async fn test_resource_registration_without_cleanup() {
        let manager = create_fast_test_manager();
        
        let resource_id = manager.register_resource(
            ResourceType::Memory,
            "Memory block".to_string(),
            Some(2048),
            true,
            None::<fn() -> _>,
        ).await.unwrap();
        
        assert!(resource_id.starts_with("memory_"));
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.active_resources, 1);
        
        // Unregister without cleanup function should still work
        manager.unregister_resource(&resource_id).await.unwrap();
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.active_resources, 0);
        assert_eq!(stats.cleaned_resources, 1);
    }

    #[tokio::test]
    async fn test_resource_type_variations() {
        let manager = create_fast_test_manager();
        
        let resource_types = vec![
            ResourceType::Process,
            ResourceType::NetworkConnection,
            ResourceType::FileHandle,
            ResourceType::Channel,
            ResourceType::Task,
            ResourceType::Stream,
            ResourceType::Timer,
            ResourceType::Memory,
        ];
        
        let mut resource_ids = Vec::new();
        
        for (i, resource_type) in resource_types.into_iter().enumerate() {
            let resource_id = manager.register_resource(
                resource_type,
                format!("Resource {}", i),
                Some(100),
                false,
                None::<fn() -> _>,
            ).await.unwrap();
            
            resource_ids.push(resource_id);
        }
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.active_resources, 8);
        
        // Clean up
        for resource_id in resource_ids {
            manager.unregister_resource(&resource_id).await.unwrap();
        }
        
        let final_stats = manager.get_stats().await;
        assert_eq!(final_stats.active_resources, 0);
    }

    #[tokio::test]
    async fn test_resource_limits_enforcement() {
        let manager = create_fast_test_manager();
        
        // Fill up to process limit
        let mut process_ids = Vec::new();
        for i in 0..3 {
            let id = manager.register_resource(
                ResourceType::Process,
                format!("Process {}", i),
                None,
                false,
                None::<fn() -> _>,
            ).await.unwrap();
            process_ids.push(id);
        }
        
        // Next process registration should fail
        let result = manager.register_resource(
            ResourceType::Process,
            "Extra process".to_string(),
            None,
            false,
            None::<fn() -> _>,
        ).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Process limit reached"));
        
        // After unregistering one, should be able to register again
        manager.unregister_resource(&process_ids[0]).await.unwrap();
        
        let new_process = manager.register_resource(
            ResourceType::Process,
            "New process".to_string(),
            None,
            false,
            None::<fn() -> _>,
        ).await;
        
        assert!(new_process.is_ok());
    }

    #[tokio::test]
    async fn test_memory_limit_enforcement() {
        let manager = create_fast_test_manager();
        
        // Register memory resources up to limit
        let id1 = manager.register_resource(
            ResourceType::Memory,
            "Memory 1".to_string(),
            Some(50 * 1024 * 1024), // 50MB
            false,
            None::<fn() -> _>,
        ).await.unwrap();
        
        let id2 = manager.register_resource(
            ResourceType::Memory,
            "Memory 2".to_string(),
            Some(40 * 1024 * 1024), // 40MB
            false,
            None::<fn() -> _>,
        ).await.unwrap();
        
        // This should exceed the 100MB limit
        let result = manager.register_resource(
            ResourceType::Memory,
            "Memory 3".to_string(),
            Some(20 * 1024 * 1024), // 20MB
            false,
            None::<fn() -> _>,
        ).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Memory limit reached"));
        
        // Clean up
        manager.unregister_resource(&id1).await.unwrap();
        manager.unregister_resource(&id2).await.unwrap();
    }

    #[tokio::test]
    async fn test_cleanup_timeout_handling() {
        let manager = create_fast_test_manager();
        
        // Create resource with slow cleanup
        let resource_id = manager.register_resource(
            ResourceType::Task,
            "Slow cleanup task".to_string(),
            None,
            false,
            Some(|| Box::pin(async {
                // Cleanup takes longer than timeout (100ms)
                sleep(Duration::from_millis(200)).await;
                Ok(())
            })),
        ).await.unwrap();
        
        // Unregister should timeout and fail
        let start = Instant::now();
        let result = manager.unregister_resource(&resource_id).await;
        let elapsed = start.elapsed();
        
        // Should complete quickly due to timeout
        assert!(elapsed < Duration::from_millis(150));
        assert!(result.is_ok()); // Should still succeed, just log the timeout
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.failed_cleanups, 1);
    }

    #[tokio::test]
    async fn test_cleanup_error_handling() {
        let manager = create_fast_test_manager();
        
        // Create resource with failing cleanup
        let resource_id = manager.register_resource(
            ResourceType::Task,
            "Failing cleanup task".to_string(),
            None,
            false,
            Some(|| Box::pin(async {
                Err("Cleanup failed".to_string())
            })),
        ).await.unwrap();
        
        // Unregister should handle the cleanup error
        let result = manager.unregister_resource(&resource_id).await;
        assert!(result.is_ok()); // Should not propagate cleanup error
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.failed_cleanups, 1);
    }

    #[tokio::test]
    async fn test_force_cleanup() {
        let manager = create_fast_test_manager();
        
        // Register critical and non-critical resources
        let critical_count = Arc::new(AtomicU32::new(0));
        let non_critical_count = Arc::new(AtomicU32::new(0));
        
        let mut resource_ids = Vec::new();
        
        // Critical resources (should not be cleaned)
        for i in 0..3 {
            let critical_count_clone = critical_count.clone();
            let resource_id = manager.register_resource(
                ResourceType::Task,
                format!("Critical task {}", i),
                None,
                true, // Critical
                Some(move || {
                    let count = critical_count_clone.clone();
                    Box::pin(async move {
                        count.fetch_add(1, Ordering::Relaxed);
                        Ok(())
                    })
                }),
            ).await.unwrap();
            resource_ids.push(resource_id);
        }
        
        // Non-critical resources (should be cleaned)
        for i in 0..5 {
            let non_critical_count_clone = non_critical_count.clone();
            let resource_id = manager.register_resource(
                ResourceType::Task,
                format!("Non-critical task {}", i),
                None,
                false, // Not critical
                Some(move || {
                    let count = non_critical_count_clone.clone();
                    Box::pin(async move {
                        count.fetch_add(1, Ordering::Relaxed);
                        Ok(())
                    })
                }),
            ).await.unwrap();
            resource_ids.push(resource_id);
        }
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.active_resources, 8);
        
        // Force cleanup should only clean non-critical resources
        let cleaned_count = manager.force_cleanup().await.unwrap();
        assert_eq!(cleaned_count, 5);
        
        // Verify cleanup was called correctly
        assert_eq!(critical_count.load(Ordering::Relaxed), 0); // Not cleaned
        assert_eq!(non_critical_count.load(Ordering::Relaxed), 5); // All cleaned
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.active_resources, 3); // Only critical resources remain
    }

    #[tokio::test]
    async fn test_shutdown_cleanup() {
        let manager = create_fast_test_manager();
        
        let cleanup_count = Arc::new(AtomicU32::new(0));
        let mut resource_ids = Vec::new();
        
        // Register multiple resources
        for i in 0..10 {
            let cleanup_count_clone = cleanup_count.clone();
            let resource_id = manager.register_resource(
                ResourceType::Task,
                format!("Task {}", i),
                None,
                i < 3, // First 3 are critical
                Some(move || {
                    let count = cleanup_count_clone.clone();
                    Box::pin(async move {
                        count.fetch_add(1, Ordering::Relaxed);
                        Ok(())
                    })
                }),
            ).await.unwrap();
            resource_ids.push(resource_id);
        }
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.active_resources, 10);
        
        // Shutdown should clean up all resources
        let result = manager.shutdown().await;
        assert!(result.is_ok());
        
        // All cleanups should have been called
        assert_eq!(cleanup_count.load(Ordering::Relaxed), 10);
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.active_resources, 0);
    }

    #[tokio::test]
    async fn test_shutdown_during_operation() {
        let manager = Arc::new(create_fast_test_manager());
        
        // Start monitoring
        manager.start_monitoring().await;
        
        // Register some resources
        for i in 0..5 {
            manager.register_resource(
                ResourceType::Task,
                format!("Task {}", i),
                None,
                false,
                None::<fn() -> _>,
            ).await.unwrap();
        }
        
        // Shutdown should stop monitoring and clean up
        let result = manager.shutdown().await;
        assert!(result.is_ok());
        
        // Verify shutdown state
        assert!(manager.is_shutting_down.load(Ordering::Relaxed));
        
        // Should not be able to register new resources after shutdown
        let result = manager.register_resource(
            ResourceType::Task,
            "Post-shutdown task".to_string(),
            None,
            false,
            None::<fn() -> _>,
        ).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("shutting down"));
    }

    #[tokio::test]
    async fn test_monitoring_tasks() {
        let manager = Arc::new(create_fast_test_manager());
        
        // Start monitoring
        manager.start_monitoring().await;
        
        // Register some resources
        for i in 0..3 {
            manager.register_resource(
                ResourceType::Memory,
                format!("Memory {}", i),
                Some(10 * 1024 * 1024), // 10MB each
                false,
                None::<fn() -> _>,
            ).await.unwrap();
        }
        
        // Wait for monitoring to update stats
        sleep(Duration::from_millis(150)).await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.active_resources, 3);
        assert!(stats.memory_usage_mb > 0.0);
        
        // Stop monitoring by shutting down
        manager.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_stale_resource_cleanup() {
        let manager = Arc::new(create_fast_test_manager());
        
        // Start monitoring for automatic cleanup
        manager.start_monitoring().await;
        
        let cleanup_count = Arc::new(AtomicU32::new(0));
        
        // Register a non-critical resource
        let cleanup_count_clone = cleanup_count.clone();
        let resource_id = manager.register_resource(
            ResourceType::Task,
            "Stale task".to_string(),
            None,
            false, // Not critical, can be cleaned up
            Some(move || {
                let count = cleanup_count_clone.clone();
                Box::pin(async move {
                    count.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                })
            }),
        ).await.unwrap();
        
        // Wait for stale resource cleanup (timeout is 1 second + cleanup interval)
        sleep(Duration::from_millis(1500)).await;
        
        // Stale resource should have been cleaned up automatically
        let stats = manager.get_stats().await;
        
        // The resource might still be in the map but should be marked as cleaned
        // or removed entirely depending on cleanup implementation
        assert_eq!(cleanup_count.load(Ordering::Relaxed), 1);
        
        manager.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_concurrent_resource_operations() {
        let manager = Arc::new(create_fast_test_manager());
        
        // Start monitoring
        manager.start_monitoring().await;
        
        let success_count = Arc::new(AtomicU32::new(0));
        let error_count = Arc::new(AtomicU32::new(0));
        
        // Start multiple concurrent operations
        let mut handles = Vec::new();
        
        for i in 0..20 {
            let manager_clone = manager.clone();
            let success_count_clone = success_count.clone();
            let error_count_clone = error_count.clone();
            
            let handle = tokio::spawn(async move {
                // Try to register a resource
                let result = manager_clone.register_resource(
                    ResourceType::Task,
                    format!("Concurrent task {}", i),
                    None,
                    false,
                    None::<fn() -> _>,
                ).await;
                
                match result {
                    Ok(resource_id) => {
                        success_count_clone.fetch_add(1, Ordering::Relaxed);
                        
                        // Try to unregister after a short delay
                        sleep(Duration::from_millis(10)).await;
                        let _ = manager_clone.unregister_resource(&resource_id).await;
                    },
                    Err(_) => {
                        error_count_clone.fetch_add(1, Ordering::Relaxed);
                    },
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all operations to complete
        for handle in handles {
            let _ = handle.await;
        }
        
        let total_operations = success_count.load(Ordering::Relaxed) + error_count.load(Ordering::Relaxed);
        assert_eq!(total_operations, 20);
        
        // Some operations should succeed (within task limit)
        assert!(success_count.load(Ordering::Relaxed) > 0);
        
        manager.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_resource_stats_accuracy() {
        let manager = create_fast_test_manager();
        
        // Register resources of different types
        let mut resource_ids = Vec::new();
        
        // Processes
        for i in 0..2 {
            let id = manager.register_resource(
                ResourceType::Process,
                format!("Process {}", i),
                None,
                false,
                None::<fn() -> _>,
            ).await.unwrap();
            resource_ids.push(id);
        }
        
        // Memory
        let mem_id = manager.register_resource(
            ResourceType::Memory,
            "Memory block".to_string(),
            Some(50 * 1024 * 1024), // 50MB
            false,
            None::<fn() -> _>,
        ).await.unwrap();
        resource_ids.push(mem_id);
        
        // Connections
        for i in 0..3 {
            let id = manager.register_resource(
                ResourceType::NetworkConnection,
                format!("Connection {}", i),
                None,
                false,
                None::<fn() -> _>,
            ).await.unwrap();
            resource_ids.push(id);
        }
        
        // Check stats
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_resources, 6);
        assert_eq!(stats.active_resources, 6);
        assert_eq!(stats.process_count, 2);
        assert_eq!(stats.network_connections, 3);
        assert!(stats.memory_usage_mb > 40.0); // Should be around 50MB
        
        // Clean up half the resources
        for i in 0..3 {
            manager.unregister_resource(&resource_ids[i]).await.unwrap();
        }
        
        let updated_stats = manager.get_stats().await;
        assert_eq!(updated_stats.active_resources, 3);
        assert_eq!(updated_stats.cleaned_resources, 3);
        
        // Clean up remaining resources
        for i in 3..6 {
            manager.unregister_resource(&resource_ids[i]).await.unwrap();
        }
        
        let final_stats = manager.get_stats().await;
        assert_eq!(final_stats.active_resources, 0);
        assert_eq!(final_stats.cleaned_resources, 6);
    }

    #[tokio::test]
    async fn test_resource_limits_update() {
        let manager = create_fast_test_manager();
        
        // Register maximum number of processes (3)
        let mut process_ids = Vec::new();
        for i in 0..3 {
            let id = manager.register_resource(
                ResourceType::Process,
                format!("Process {}", i),
                None,
                false,
                None::<fn() -> _>,
            ).await.unwrap();
            process_ids.push(id);
        }
        
        // Should fail to register another
        let result = manager.register_resource(
            ResourceType::Process,
            "Extra process".to_string(),
            None,
            false,
            None::<fn() -> _>,
        ).await;
        assert!(result.is_err());
        
        // Update limits to allow more processes
        let new_limits = ResourceLimits {
            max_processes: 10,
            ..ResourceLimits::default()
        };
        manager.update_limits(new_limits).await.unwrap();
        
        // Should now be able to register more processes
        let result = manager.register_resource(
            ResourceType::Process,
            "New process".to_string(),
            None,
            false,
            None::<fn() -> _>,
        ).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_nonexistent_resource_unregister() {
        let manager = create_fast_test_manager();
        
        let result = manager.unregister_resource("nonexistent_resource_id").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Resource not found: nonexistent_resource_id");
    }

    #[tokio::test]
    async fn test_force_cleanup_in_progress() {
        let manager = Arc::new(create_fast_test_manager());
        
        // Set cleanup in progress
        manager.cleanup_in_progress.store(true, Ordering::Relaxed);
        
        // Force cleanup should fail
        let result = manager.force_cleanup().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Cleanup already in progress");
        
        // Reset and try again
        manager.cleanup_in_progress.store(false, Ordering::Relaxed);
        let result = manager.force_cleanup().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_resource_info_completeness() {
        let manager = create_fast_test_manager();
        
        let before_register = Instant::now();
        let resource_id = manager.register_resource(
            ResourceType::FileHandle,
            "Test file handle".to_string(),
            Some(4096),
            true,
            None::<fn() -> _>,
        ).await.unwrap();
        let after_register = Instant::now();
        
        // Resource ID should follow expected format
        assert!(resource_id.starts_with("file_"));
        assert!(resource_id.contains("_"));
        
        // Verify resource was properly registered
        let stats = manager.get_stats().await;
        assert_eq!(stats.active_resources, 1);
        assert_eq!(stats.file_handles, 1);
        
        // Clean up
        manager.unregister_resource(&resource_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_edge_case_resource_sizes() {
        let manager = create_fast_test_manager();
        
        // Test zero-size resource
        let id1 = manager.register_resource(
            ResourceType::Memory,
            "Zero size".to_string(),
            Some(0),
            false,
            None::<fn() -> _>,
        ).await.unwrap();
        
        // Test very large resource size
        let id2 = manager.register_resource(
            ResourceType::Memory,
            "Large size".to_string(),
            Some(u64::MAX),
            false,
            None::<fn() -> _>,
        ).await.unwrap();
        
        // Test None size
        let id3 = manager.register_resource(
            ResourceType::Memory,
            "No size".to_string(),
            None,
            false,
            None::<fn() -> _>,
        ).await.unwrap();
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.active_resources, 3);
        
        // Clean up
        manager.unregister_resource(&id1).await.unwrap();
        manager.unregister_resource(&id2).await.unwrap();
        manager.unregister_resource(&id3).await.unwrap();
    }

    #[test]
    fn test_resource_type_string_conversion() {
        use super::super::resource_manager::resource_type_to_string;
        
        assert_eq!(resource_type_to_string(&ResourceType::Process), "process");
        assert_eq!(resource_type_to_string(&ResourceType::NetworkConnection), "connection");
        assert_eq!(resource_type_to_string(&ResourceType::FileHandle), "file");
        assert_eq!(resource_type_to_string(&ResourceType::Channel), "channel");
        assert_eq!(resource_type_to_string(&ResourceType::Task), "task");
        assert_eq!(resource_type_to_string(&ResourceType::Stream), "stream");
        assert_eq!(resource_type_to_string(&ResourceType::Timer), "timer");
        assert_eq!(resource_type_to_string(&ResourceType::Memory), "memory");
    }

    #[tokio::test]
    async fn test_shutdown_with_failed_cleanups() {
        let manager = create_fast_test_manager();
        
        // Register resources with mix of successful and failing cleanups
        let success_count = Arc::new(AtomicU32::new(0));
        let failure_count = Arc::new(AtomicU32::new(0));
        
        // Successful cleanup
        let success_count_clone = success_count.clone();
        let _id1 = manager.register_resource(
            ResourceType::Task,
            "Success task".to_string(),
            None,
            false,
            Some(move || {
                let count = success_count_clone.clone();
                Box::pin(async move {
                    count.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                })
            }),
        ).await.unwrap();
        
        // Failing cleanup
        let failure_count_clone = failure_count.clone();
        let _id2 = manager.register_resource(
            ResourceType::Task,
            "Failure task".to_string(),
            None,
            false,
            Some(move || {
                let count = failure_count_clone.clone();
                Box::pin(async move {
                    count.fetch_add(1, Ordering::Relaxed);
                    Err("Cleanup failed".to_string())
                })
            }),
        ).await.unwrap();
        
        // Shutdown should report failure due to failed cleanup
        let result = manager.shutdown().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to cleanup"));
        
        // But both cleanups should have been attempted
        assert_eq!(success_count.load(Ordering::Relaxed), 1);
        assert_eq!(failure_count.load(Ordering::Relaxed), 1);
    }
}
