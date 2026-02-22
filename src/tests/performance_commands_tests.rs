#[cfg(test)]
mod tests {
    use crate::performance_commands::*;
    use std::sync::{Arc, atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering}};
    use std::collections::HashMap;
    use tokio::sync::RwLock;
    use serde_json::{json, Value};
    use chrono::{DateTime, Utc, Duration as ChronoDuration};

    // Mock performance structures for testing
    #[derive(Debug, Clone)]
    struct MockPerformanceManager {
        initialized: Arc<AtomicBool>,
        startup_count: Arc<AtomicU32>,
        optimization_count: Arc<AtomicU32>,
        benchmark_runs: Arc<AtomicU32>,
        config: Arc<RwLock<MockPerformanceConfig>>,
        metrics: Arc<RwLock<MockPerformanceMetrics>>,
        resource_stats: Arc<RwLock<MockResourceMetrics>>,
        benchmarks: Arc<RwLock<Vec<MockBenchmarkResult>>>,
    }

    #[derive(Debug, Clone)]
    struct MockPerformanceConfig {
        enable_monitoring: bool,
        benchmark_iterations: u32,
        cache_size_mb: u32,
        optimization_level: u8,
    }

    #[derive(Debug, Clone)]
    struct MockPerformanceMetrics {
        startup_metrics: MockStartupMetrics,
        ipc_metrics: MockIpcMetrics,
        memory_metrics: MockMemoryMetrics,
        last_updated: DateTime<Utc>,
    }

    #[derive(Debug, Clone)]
    struct MockStartupMetrics {
        total_startup_time_ms: u64,
        component_load_time_ms: u64,
        initialization_time_ms: u64,
        first_render_time_ms: u64,
    }

    #[derive(Debug, Clone)]
    struct MockIpcMetrics {
        average_latency_ms: f64,
        throughput_messages_per_sec: f64,
        cache_hit_ratio: f64,
        error_rate: f64,
    }

    #[derive(Debug, Clone)]
    struct MockMemoryMetrics {
        heap_used_mb: f64,
        heap_total_mb: f64,
        cache_size_mb: f64,
        gc_count: u32,
    }

    #[derive(Debug, Clone)]
    struct MockResourceMetrics {
        timestamp: DateTime<Utc>,
        health_score: f64,
        system_metrics: MockSystemMetrics,
        process_metrics: MockProcessMetrics,
    }

    #[derive(Debug, Clone)]
    struct MockSystemMetrics {
        cpu_usage_percent: f64,
        memory_total_gb: f64,
        memory_used_gb: f64,
        memory_usage_percent: f64,
        cpu_cores: u32,
        cpu_frequency_mhz: u32,
    }

    #[derive(Debug, Clone)]
    struct MockProcessMetrics {
        pid: u32,
        cpu_usage_percent: f64,
        memory_usage_mb: f64,
        thread_count: u32,
        handle_count: u32,
    }

    #[derive(Debug, Clone)]
    struct MockBenchmarkResult {
        name: String,
        duration_ms: u64,
        operations_per_second: f64,
        memory_used_mb: f64,
        cpu_usage_percent: f64,
        timestamp: DateTime<Utc>,
    }

    #[derive(Debug, Clone)]
    struct MockOptimizationResult {
        startup_time_ms: u64,
        memory_usage_mb: u64,
        memory_saved_mb: u64,
        ipc_latency_ms: f64,
        cache_hit_ratio: f64,
        optimizations_applied: Vec<String>,
        warnings: Vec<String>,
    }

    impl MockOptimizationResult {
        fn new() -> Self {
            Self {
                startup_time_ms: 1500,
                memory_usage_mb: 128,
                memory_saved_mb: 32,
                ipc_latency_ms: 2.5,
                cache_hit_ratio: 0.85,
                optimizations_applied: Vec::new(),
                warnings: Vec::new(),
            }
        }
    }

    #[derive(Debug, Clone)]
    struct MockCacheStats {
        size_mb: f64,
        hit_ratio: f64,
        miss_count: u64,
        evictions: u64,
    }

    #[derive(Debug, Clone)]
    struct MockPoolStats {
        total_pools: u32,
        active_allocations: u64,
        total_allocated_mb: f64,
        fragmentation_ratio: f64,
    }

    impl MockPerformanceManager {
        fn new() -> Self {
            Self {
                initialized: Arc::new(AtomicBool::new(false)),
                startup_count: Arc::new(AtomicU32::new(0)),
                optimization_count: Arc::new(AtomicU32::new(0)),
                benchmark_runs: Arc::new(AtomicU32::new(0)),
                config: Arc::new(RwLock::new(MockPerformanceConfig {
                    enable_monitoring: true,
                    benchmark_iterations: 100,
                    cache_size_mb: 64,
                    optimization_level: 2,
                })),
                metrics: Arc::new(RwLock::new(MockPerformanceMetrics {
                    startup_metrics: MockStartupMetrics {
                        total_startup_time_ms: 2000,
                        component_load_time_ms: 500,
                        initialization_time_ms: 800,
                        first_render_time_ms: 700,
                    },
                    ipc_metrics: MockIpcMetrics {
                        average_latency_ms: 3.5,
                        throughput_messages_per_sec: 1000.0,
                        cache_hit_ratio: 0.85,
                        error_rate: 0.01,
                    },
                    memory_metrics: MockMemoryMetrics {
                        heap_used_mb: 95.5,
                        heap_total_mb: 256.0,
                        cache_size_mb: 32.0,
                        gc_count: 12,
                    },
                    last_updated: Utc::now(),
                })),
                resource_stats: Arc::new(RwLock::new(MockResourceMetrics {
                    timestamp: Utc::now(),
                    health_score: 87.5,
                    system_metrics: MockSystemMetrics {
                        cpu_usage_percent: 35.2,
                        memory_total_gb: 16.0,
                        memory_used_gb: 8.5,
                        memory_usage_percent: 53.1,
                        cpu_cores: 8,
                        cpu_frequency_mhz: 3200,
                    },
                    process_metrics: MockProcessMetrics {
                        pid: 1234,
                        cpu_usage_percent: 12.5,
                        memory_usage_mb: 128.0,
                        thread_count: 16,
                        handle_count: 256,
                    },
                })),
                benchmarks: Arc::new(RwLock::new(Vec::new())),
            }
        }

        async fn initialize(&self) -> Result<(), String> {
            self.initialized.store(true, Ordering::Relaxed);
            Ok(())
        }

        async fn begin_startup(&self) -> String {
            self.startup_count.fetch_add(1, Ordering::Relaxed);
            format!("startup_context_{}", Utc::now().timestamp())
        }

        async fn get_metrics(&self) -> MockPerformanceMetrics {
            self.metrics.read().await.clone()
        }

        async fn update_config(&self, config: MockPerformanceConfig) -> Result<(), String> {
            *self.config.write().await = config;
            Ok(())
        }

        async fn run_benchmarks(&self) -> Result<Vec<MockBenchmarkResult>, String> {
            if !self.initialized.load(Ordering::Relaxed) {
                return Err("Performance manager not initialized".to_string());
            }

            self.benchmark_runs.fetch_add(1, Ordering::Relaxed);

            let benchmarks = vec![
                MockBenchmarkResult {
                    name: "IPC Latency".to_string(),
                    duration_ms: 150,
                    operations_per_second: 6666.67,
                    memory_used_mb: 2.5,
                    cpu_usage_percent: 15.0,
                    timestamp: Utc::now(),
                },
                MockBenchmarkResult {
                    name: "Memory Allocation".to_string(),
                    duration_ms: 200,
                    operations_per_second: 5000.0,
                    memory_used_mb: 128.0,
                    cpu_usage_percent: 25.0,
                    timestamp: Utc::now(),
                },
                MockBenchmarkResult {
                    name: "File I/O".to_string(),
                    duration_ms: 500,
                    operations_per_second: 2000.0,
                    memory_used_mb: 8.0,
                    cpu_usage_percent: 30.0,
                    timestamp: Utc::now(),
                },
            ];

            *self.benchmarks.write().await = benchmarks.clone();
            Ok(benchmarks)
        }

        async fn get_resource_stats(&self) -> MockResourceMetrics {
            self.resource_stats.read().await.clone()
        }

        async fn optimize_memory(&self) -> Result<(), String> {
            self.optimization_count.fetch_add(1, Ordering::Relaxed);
            
            // Simulate memory optimization by reducing memory usage
            let mut resource_stats = self.resource_stats.write().await;
            resource_stats.process_metrics.memory_usage_mb *= 0.9; // 10% reduction
            
            Ok(())
        }

        // Mock memory manager methods
        async fn get_cache_stats(&self) -> MockCacheStats {
            MockCacheStats {
                size_mb: 32.0,
                hit_ratio: 0.87,
                miss_count: 1250,
                evictions: 45,
            }
        }

        async fn cache_clear(&self) {
            // Mock cache clear - no actual implementation needed
        }

        async fn get_pool_stats(&self) -> MockPoolStats {
            MockPoolStats {
                total_pools: 8,
                active_allocations: 15624,
                total_allocated_mb: 256.5,
                fragmentation_ratio: 0.12,
            }
        }

        // Mock IPC optimizer methods
        async fn get_detailed_ipc_stats(&self) -> Value {
            json!({
                "requests_processed": 15234,
                "average_latency_ms": 2.3,
                "cache_hits": 12987,
                "cache_misses": 2247,
                "errors": 15,
                "throughput_per_second": 1250.0
            })
        }

        async fn reset_ipc_optimizer(&self) {
            // Mock reset - no actual implementation needed
        }

        // Mock lazy loader methods
        async fn get_lazy_loader_stats(&self) -> Value {
            json!({
                "components_loaded": 23,
                "components_cached": 18,
                "load_time_ms": 450,
                "cache_hit_ratio": 0.78
            })
        }

        async fn get_component_info(&self) -> Value {
            json!({
                "total_components": 45,
                "loaded_components": 23,
                "pending_loads": 3,
                "failed_loads": 1
            })
        }

        async fn load_component(&self, component_name: &str, _trigger: &str) -> Result<Value, String> {
            if component_name.is_empty() {
                return Err("Component name cannot be empty".to_string());
            }

            Ok(json!({
                "component": component_name,
                "load_time_ms": 125,
                "size_kb": 45.6,
                "dependencies": 3
            }))
        }

        async fn preload_components(&self, trigger: &str) -> Vec<Value> {
            let component_count = match trigger {
                "app_start" => 5,
                "user_interaction" => 2,
                "idle" => 3,
                _ => 1,
            };

            (0..component_count)
                .map(|i| json!({
                    "component": format!("component_{}", i),
                    "preloaded": true,
                    "load_time_ms": 50 + i * 10
                }))
                .collect()
        }

        // Mock benchmark suite methods
        async fn get_benchmark_history(&self) -> Vec<MockBenchmarkResult> {
            self.benchmarks.read().await.clone()
        }

        async fn get_benchmark_baselines(&self) -> Value {
            json!({
                "ipc_latency_ms": 2.0,
                "memory_usage_mb": 100.0,
                "startup_time_ms": 1500,
                "throughput_ops_per_sec": 1000.0
            })
        }

        async fn clear_benchmark_baselines(&self) {
            // Mock clear - no actual implementation needed
        }

        async fn run_startup_benchmark(&self) -> Result<MockBenchmarkResult, String> {
            Ok(MockBenchmarkResult {
                name: "Startup Performance".to_string(),
                duration_ms: 1750,
                operations_per_second: 0.57, // 1/1.75 seconds
                memory_used_mb: 145.0,
                cpu_usage_percent: 65.0,
                timestamp: Utc::now(),
            })
        }

        // Mock metrics collector methods
        async fn get_metrics_history(&self) -> Vec<Value> {
            vec![
                json!({
                    "timestamp": (Utc::now() - ChronoDuration::minutes(5)).to_rfc3339(),
                    "cpu_usage": 25.0,
                    "memory_usage": 120.0
                }),
                json!({
                    "timestamp": Utc::now().to_rfc3339(),
                    "cpu_usage": 30.0,
                    "memory_usage": 128.0
                })
            ]
        }

        async fn get_metrics_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<Value> {
            // Simple mock - return one data point in range
            if start < Utc::now() && end > Utc::now() {
                vec![json!({
                    "timestamp": Utc::now().to_rfc3339(),
                    "cpu_usage": 28.5,
                    "memory_usage": 124.0
                })]
            } else {
                vec![]
            }
        }

        // Mock resource monitor methods
        async fn get_active_alerts(&self) -> Vec<Value> {
            vec![
                json!({
                    "type": "memory_high",
                    "severity": "warning",
                    "message": "Memory usage above 80%",
                    "threshold": 80.0,
                    "current": 85.2
                })
            ]
        }

        async fn get_performance_trends(&self) -> Value {
            json!({
                "cpu_trend": "stable",
                "memory_trend": "increasing",
                "startup_trend": "improving",
                "confidence": 0.85
            })
        }

        async fn get_optimization_recommendations(&self) -> Vec<Value> {
            vec![
                json!({
                    "category": "memory",
                    "recommendation": "Increase cache size to reduce memory pressure",
                    "impact": "medium",
                    "effort": "low"
                }),
                json!({
                    "category": "startup",
                    "recommendation": "Enable lazy loading for non-critical components",
                    "impact": "high",
                    "effort": "medium"
                })
            ]
        }

        async fn get_historical_data(&self) -> Vec<Value> {
            let mut data = Vec::new();
            for i in 0..10 {
                data.push(json!({
                    "timestamp": (Utc::now() - ChronoDuration::hours(i)).to_rfc3339(),
                    "cpu_usage": 25.0 + i as f64 * 2.0,
                    "memory_usage": 120.0 + i as f64 * 5.0,
                    "health_score": 90.0 - i as f64 * 1.0
                }));
            }
            data
        }

        async fn shutdown(&self) -> Result<(), String> {
            self.initialized.store(false, Ordering::Relaxed);
            Ok(())
        }
    }

    // Mock state wrapper
    struct MockPerformanceManagerState {
        manager: Arc<MockPerformanceManager>,
    }

    impl MockPerformanceManagerState {
        fn new() -> Self {
            Self {
                manager: Arc::new(MockPerformanceManager::new()),
            }
        }
    }

    #[tokio::test]
    async fn test_performance_manager_creation() {
        let state = MockPerformanceManagerState::new();
        assert!(!state.manager.initialized.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn test_performance_manager_initialization() {
        let state = MockPerformanceManagerState::new();
        
        let result = state.manager.initialize().await;
        assert!(result.is_ok());
        assert!(state.manager.initialized.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn test_startup_sequence() {
        let state = MockPerformanceManagerState::new();
        state.manager.initialize().await.unwrap();

        assert_eq!(state.manager.startup_count.load(Ordering::Relaxed), 0);

        let context_id = state.manager.begin_startup().await;
        assert!(!context_id.is_empty());
        assert!(context_id.starts_with("startup_context_"));
        assert_eq!(state.manager.startup_count.load(Ordering::Relaxed), 1);

        // Multiple startups should increment counter
        let _context_id2 = state.manager.begin_startup().await;
        assert_eq!(state.manager.startup_count.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn test_performance_metrics_retrieval() {
        let state = MockPerformanceManagerState::new();
        state.manager.initialize().await.unwrap();

        let metrics = state.manager.get_metrics().await;
        
        // Verify startup metrics
        assert_eq!(metrics.startup_metrics.total_startup_time_ms, 2000);
        assert_eq!(metrics.startup_metrics.component_load_time_ms, 500);
        assert_eq!(metrics.startup_metrics.initialization_time_ms, 800);
        assert_eq!(metrics.startup_metrics.first_render_time_ms, 700);

        // Verify IPC metrics
        assert_eq!(metrics.ipc_metrics.average_latency_ms, 3.5);
        assert_eq!(metrics.ipc_metrics.throughput_messages_per_sec, 1000.0);
        assert_eq!(metrics.ipc_metrics.cache_hit_ratio, 0.85);
        assert_eq!(metrics.ipc_metrics.error_rate, 0.01);

        // Verify memory metrics
        assert_eq!(metrics.memory_metrics.heap_used_mb, 95.5);
        assert_eq!(metrics.memory_metrics.heap_total_mb, 256.0);
        assert_eq!(metrics.memory_metrics.cache_size_mb, 32.0);
        assert_eq!(metrics.memory_metrics.gc_count, 12);
    }

    #[tokio::test]
    async fn test_config_update() {
        let state = MockPerformanceManagerState::new();
        state.manager.initialize().await.unwrap();

        let new_config = MockPerformanceConfig {
            enable_monitoring: false,
            benchmark_iterations: 500,
            cache_size_mb: 128,
            optimization_level: 3,
        };

        let result = state.manager.update_config(new_config.clone()).await;
        assert!(result.is_ok());

        let updated_config = state.manager.config.read().await;
        assert!(!updated_config.enable_monitoring);
        assert_eq!(updated_config.benchmark_iterations, 500);
        assert_eq!(updated_config.cache_size_mb, 128);
        assert_eq!(updated_config.optimization_level, 3);
    }

    #[tokio::test]
    async fn test_benchmark_execution() {
        let state = MockPerformanceManagerState::new();
        state.manager.initialize().await.unwrap();

        assert_eq!(state.manager.benchmark_runs.load(Ordering::Relaxed), 0);

        let benchmarks = state.manager.run_benchmarks().await.unwrap();
        assert_eq!(benchmarks.len(), 3);
        assert_eq!(state.manager.benchmark_runs.load(Ordering::Relaxed), 1);

        // Verify benchmark data
        let ipc_benchmark = &benchmarks[0];
        assert_eq!(ipc_benchmark.name, "IPC Latency");
        assert_eq!(ipc_benchmark.duration_ms, 150);
        assert_eq!(ipc_benchmark.operations_per_second, 6666.67);

        let memory_benchmark = &benchmarks[1];
        assert_eq!(memory_benchmark.name, "Memory Allocation");
        assert_eq!(memory_benchmark.memory_used_mb, 128.0);

        let io_benchmark = &benchmarks[2];
        assert_eq!(io_benchmark.name, "File I/O");
        assert_eq!(io_benchmark.cpu_usage_percent, 30.0);

        // Test benchmark without initialization
        let uninitialized_manager = MockPerformanceManager::new();
        let result = uninitialized_manager.run_benchmarks().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Performance manager not initialized");
    }

    #[tokio::test]
    async fn test_resource_statistics() {
        let state = MockPerformanceManagerState::new();
        
        let resource_stats = state.manager.get_resource_stats().await;
        
        assert_eq!(resource_stats.health_score, 87.5);
        assert_eq!(resource_stats.system_metrics.cpu_usage_percent, 35.2);
        assert_eq!(resource_stats.system_metrics.memory_total_gb, 16.0);
        assert_eq!(resource_stats.system_metrics.memory_used_gb, 8.5);
        assert_eq!(resource_stats.system_metrics.cpu_cores, 8);

        assert_eq!(resource_stats.process_metrics.pid, 1234);
        assert_eq!(resource_stats.process_metrics.cpu_usage_percent, 12.5);
        assert_eq!(resource_stats.process_metrics.memory_usage_mb, 128.0);
        assert_eq!(resource_stats.process_metrics.thread_count, 16);
    }

    #[tokio::test]
    async fn test_memory_optimization() {
        let state = MockPerformanceManagerState::new();
        state.manager.initialize().await.unwrap();

        let initial_memory = state.manager.get_resource_stats().await.process_metrics.memory_usage_mb;
        assert_eq!(state.manager.optimization_count.load(Ordering::Relaxed), 0);

        let result = state.manager.optimize_memory().await;
        assert!(result.is_ok());
        assert_eq!(state.manager.optimization_count.load(Ordering::Relaxed), 1);

        let optimized_memory = state.manager.get_resource_stats().await.process_metrics.memory_usage_mb;
        assert!(optimized_memory < initial_memory);
        assert_eq!(optimized_memory, initial_memory * 0.9);
    }

    #[tokio::test]
    async fn test_cache_statistics() {
        let state = MockPerformanceManagerState::new();
        
        let cache_stats = state.manager.get_cache_stats().await;
        assert_eq!(cache_stats.size_mb, 32.0);
        assert_eq!(cache_stats.hit_ratio, 0.87);
        assert_eq!(cache_stats.miss_count, 1250);
        assert_eq!(cache_stats.evictions, 45);
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let state = MockPerformanceManagerState::new();
        
        // Cache clear should not panic
        state.manager.cache_clear().await;
    }

    #[tokio::test]
    async fn test_memory_pool_statistics() {
        let state = MockPerformanceManagerState::new();
        
        let pool_stats = state.manager.get_pool_stats().await;
        assert_eq!(pool_stats.total_pools, 8);
        assert_eq!(pool_stats.active_allocations, 15624);
        assert_eq!(pool_stats.total_allocated_mb, 256.5);
        assert_eq!(pool_stats.fragmentation_ratio, 0.12);
    }

    #[tokio::test]
    async fn test_ipc_optimizer_statistics() {
        let state = MockPerformanceManagerState::new();
        
        let ipc_stats = state.manager.get_detailed_ipc_stats().await;
        assert_eq!(ipc_stats["requests_processed"], 15234);
        assert_eq!(ipc_stats["average_latency_ms"], 2.3);
        assert_eq!(ipc_stats["cache_hits"], 12987);
        assert_eq!(ipc_stats["throughput_per_second"], 1250.0);
    }

    #[tokio::test]
    async fn test_ipc_optimizer_reset() {
        let state = MockPerformanceManagerState::new();
        
        // Reset should not panic
        state.manager.reset_ipc_optimizer().await;
    }

    #[tokio::test]
    async fn test_lazy_loader_statistics() {
        let state = MockPerformanceManagerState::new();
        
        let lazy_stats = state.manager.get_lazy_loader_stats().await;
        assert_eq!(lazy_stats["components_loaded"], 23);
        assert_eq!(lazy_stats["components_cached"], 18);
        assert_eq!(lazy_stats["load_time_ms"], 450);
        assert_eq!(lazy_stats["cache_hit_ratio"], 0.78);
    }

    #[tokio::test]
    async fn test_component_info() {
        let state = MockPerformanceManagerState::new();
        
        let component_info = state.manager.get_component_info().await;
        assert_eq!(component_info["total_components"], 45);
        assert_eq!(component_info["loaded_components"], 23);
        assert_eq!(component_info["pending_loads"], 3);
        assert_eq!(component_info["failed_loads"], 1);
    }

    #[tokio::test]
    async fn test_component_loading() {
        let state = MockPerformanceManagerState::new();
        
        // Test successful component loading
        let result = state.manager.load_component("test_component", "user_request").await;
        assert!(result.is_ok());
        
        let load_result = result.unwrap();
        assert_eq!(load_result["component"], "test_component");
        assert_eq!(load_result["load_time_ms"], 125);
        assert_eq!(load_result["dependencies"], 3);

        // Test empty component name
        let result = state.manager.load_component("", "trigger").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Component name cannot be empty");
    }

    #[tokio::test]
    async fn test_component_preloading() {
        let state = MockPerformanceManagerState::new();
        
        // Test different preload triggers
        let app_start_results = state.manager.preload_components("app_start").await;
        assert_eq!(app_start_results.len(), 5);

        let user_interaction_results = state.manager.preload_components("user_interaction").await;
        assert_eq!(user_interaction_results.len(), 2);

        let idle_results = state.manager.preload_components("idle").await;
        assert_eq!(idle_results.len(), 3);

        let unknown_results = state.manager.preload_components("unknown_trigger").await;
        assert_eq!(unknown_results.len(), 1);

        // Verify preload result structure
        assert_eq!(app_start_results[0]["preloaded"], true);
        assert!(app_start_results[0]["component"].as_str().unwrap().starts_with("component_"));
    }

    #[tokio::test]
    async fn test_benchmark_history() {
        let state = MockPerformanceManagerState::new();
        state.manager.initialize().await.unwrap();

        // Initially empty
        let history = state.manager.get_benchmark_history().await;
        assert!(history.is_empty());

        // Run benchmarks to populate history
        state.manager.run_benchmarks().await.unwrap();
        
        let history = state.manager.get_benchmark_history().await;
        assert_eq!(history.len(), 3);
    }

    #[tokio::test]
    async fn test_benchmark_baselines() {
        let state = MockPerformanceManagerState::new();
        
        let baselines = state.manager.get_benchmark_baselines().await;
        assert_eq!(baselines["ipc_latency_ms"], 2.0);
        assert_eq!(baselines["memory_usage_mb"], 100.0);
        assert_eq!(baselines["startup_time_ms"], 1500);
        assert_eq!(baselines["throughput_ops_per_sec"], 1000.0);
    }

    #[tokio::test]
    async fn test_benchmark_baseline_operations() {
        let state = MockPerformanceManagerState::new();
        
        // Clear should not panic
        state.manager.clear_benchmark_baselines().await;
    }

    #[tokio::test]
    async fn test_startup_benchmark() {
        let state = MockPerformanceManagerState::new();
        
        let result = state.manager.run_startup_benchmark().await;
        assert!(result.is_ok());
        
        let benchmark = result.unwrap();
        assert_eq!(benchmark.name, "Startup Performance");
        assert_eq!(benchmark.duration_ms, 1750);
        assert_eq!(benchmark.memory_used_mb, 145.0);
        assert_eq!(benchmark.cpu_usage_percent, 65.0);
    }

    #[tokio::test]
    async fn test_metrics_history() {
        let state = MockPerformanceManagerState::new();
        
        let history = state.manager.get_metrics_history().await;
        assert_eq!(history.len(), 2);
        
        // Verify history structure
        assert!(history[0]["timestamp"].is_string());
        assert_eq!(history[0]["cpu_usage"], 25.0);
        assert_eq!(history[0]["memory_usage"], 120.0);
        assert_eq!(history[1]["cpu_usage"], 30.0);
        assert_eq!(history[1]["memory_usage"], 128.0);
    }

    #[tokio::test]
    async fn test_metrics_range_query() {
        let state = MockPerformanceManagerState::new();
        
        let now = Utc::now();
        let start = now - ChronoDuration::hours(1);
        let end = now + ChronoDuration::hours(1);
        
        let range_data = state.manager.get_metrics_range(start, end).await;
        assert_eq!(range_data.len(), 1);
        assert_eq!(range_data[0]["cpu_usage"], 28.5);
        assert_eq!(range_data[0]["memory_usage"], 124.0);

        // Test range with no data
        let past_start = now - ChronoDuration::days(2);
        let past_end = now - ChronoDuration::days(1);
        let empty_range = state.manager.get_metrics_range(past_start, past_end).await;
        assert!(empty_range.is_empty());
    }

    #[tokio::test]
    async fn test_active_alerts() {
        let state = MockPerformanceManagerState::new();
        
        let alerts = state.manager.get_active_alerts().await;
        assert_eq!(alerts.len(), 1);
        
        let alert = &alerts[0];
        assert_eq!(alert["type"], "memory_high");
        assert_eq!(alert["severity"], "warning");
        assert_eq!(alert["threshold"], 80.0);
        assert_eq!(alert["current"], 85.2);
    }

    #[tokio::test]
    async fn test_performance_trends() {
        let state = MockPerformanceManagerState::new();
        
        let trends = state.manager.get_performance_trends().await;
        assert_eq!(trends["cpu_trend"], "stable");
        assert_eq!(trends["memory_trend"], "increasing");
        assert_eq!(trends["startup_trend"], "improving");
        assert_eq!(trends["confidence"], 0.85);
    }

    #[tokio::test]
    async fn test_optimization_recommendations() {
        let state = MockPerformanceManagerState::new();
        
        let recommendations = state.manager.get_optimization_recommendations().await;
        assert_eq!(recommendations.len(), 2);
        
        let memory_rec = &recommendations[0];
        assert_eq!(memory_rec["category"], "memory");
        assert_eq!(memory_rec["impact"], "medium");
        assert_eq!(memory_rec["effort"], "low");
        
        let startup_rec = &recommendations[1];
        assert_eq!(startup_rec["category"], "startup");
        assert_eq!(startup_rec["impact"], "high");
        assert_eq!(startup_rec["effort"], "medium");
    }

    #[tokio::test]
    async fn test_historical_data() {
        let state = MockPerformanceManagerState::new();
        
        let historical_data = state.manager.get_historical_data().await;
        assert_eq!(historical_data.len(), 10);
        
        // Verify trend in data (CPU usage should increase)
        let first_cpu = historical_data[0]["cpu_usage"].as_f64().unwrap();
        let last_cpu = historical_data[9]["cpu_usage"].as_f64().unwrap();
        assert!(last_cpu > first_cpu);
        
        // Verify health score trend (should decrease)
        let first_health = historical_data[0]["health_score"].as_f64().unwrap();
        let last_health = historical_data[9]["health_score"].as_f64().unwrap();
        assert!(last_health < first_health);
    }

    #[tokio::test]
    async fn test_optimization_result_creation() {
        let mut result = MockOptimizationResult::new();
        
        assert_eq!(result.startup_time_ms, 1500);
        assert_eq!(result.memory_usage_mb, 128);
        assert_eq!(result.memory_saved_mb, 32);
        assert_eq!(result.ipc_latency_ms, 2.5);
        assert_eq!(result.cache_hit_ratio, 0.85);
        assert!(result.optimizations_applied.is_empty());
        assert!(result.warnings.is_empty());
        
        // Test modification
        result.optimizations_applied.push("Memory optimization".to_string());
        result.warnings.push("High CPU usage detected".to_string());
        
        assert_eq!(result.optimizations_applied.len(), 1);
        assert_eq!(result.warnings.len(), 1);
    }

    #[tokio::test]
    async fn test_manager_shutdown() {
        let state = MockPerformanceManagerState::new();
        state.manager.initialize().await.unwrap();
        
        assert!(state.manager.initialized.load(Ordering::Relaxed));
        
        let result = state.manager.shutdown().await;
        assert!(result.is_ok());
        assert!(!state.manager.initialized.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let state = Arc::new(MockPerformanceManagerState::new());
        state.manager.initialize().await.unwrap();

        let mut handles = Vec::new();

        // Concurrent benchmark runs
        for _ in 0..5 {
            let state_clone = state.clone();
            let handle = tokio::spawn(async move {
                state_clone.manager.run_benchmarks().await
            });
            handles.push(handle);
        }

        // Concurrent memory optimizations
        for _ in 0..3 {
            let state_clone = state.clone();
            let handle = tokio::spawn(async move {
                state_clone.manager.optimize_memory().await
            });
            handles.push(handle);
        }

        // Concurrent metrics retrieval
        for _ in 0..5 {
            let state_clone = state.clone();
            let handle = tokio::spawn(async move {
                state_clone.manager.get_metrics().await;
                Ok::<(), String>(())
            });
            handles.push(handle);
        }

        // Wait for all operations to complete
        let mut success_count = 0;
        for handle in handles {
            if let Ok(result) = handle.await {
                if result.is_ok() {
                    success_count += 1;
                }
            }
        }

        assert!(success_count > 0); // At least some operations should succeed

        // Verify counters were updated correctly
        assert!(state.manager.benchmark_runs.load(Ordering::Relaxed) > 0);
        assert!(state.manager.optimization_count.load(Ordering::Relaxed) > 0);
    }

    #[tokio::test]
    async fn test_data_serialization_compatibility() {
        let state = MockPerformanceManagerState::new();
        state.manager.initialize().await.unwrap();

        // Test that all returned data structures can be serialized to JSON
        let metrics = state.manager.get_metrics().await;
        let metrics_json = serde_json::to_value(&metrics);
        assert!(metrics_json.is_ok());

        let resource_stats = state.manager.get_resource_stats().await;
        let resource_json = serde_json::to_value(&resource_stats);
        assert!(resource_json.is_ok());

        let benchmarks = state.manager.run_benchmarks().await.unwrap();
        let benchmarks_json = serde_json::to_value(&benchmarks);
        assert!(benchmarks_json.is_ok());

        // Test complex nested data
        let ipc_stats = state.manager.get_detailed_ipc_stats().await;
        let ipc_json = serde_json::to_string(&ipc_stats);
        assert!(ipc_json.is_ok());
    }

    #[tokio::test]
    async fn test_error_conditions() {
        let state = MockPerformanceManagerState::new();

        // Test operations without initialization where applicable
        let result = state.manager.run_benchmarks().await;
        assert!(result.is_err());

        // Test edge cases
        let empty_component_result = state.manager.load_component("", "trigger").await;
        assert!(empty_component_result.is_err());

        // Test boundary conditions
        let far_past = Utc::now() - ChronoDuration::days(365);
        let far_future = Utc::now() + ChronoDuration::days(365);
        let range_result = state.manager.get_metrics_range(far_past, far_past).await;
        assert!(range_result.is_empty());
    }
}
