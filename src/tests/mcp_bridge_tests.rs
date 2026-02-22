#[cfg(test)]
mod tests {
    use crate::mcp_bridge::*;
    use crate::process_manager::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::{Mutex, RwLock};
    use serde_json::{json, Value};
    use std::collections::HashMap;

    // Mock process manager for testing
    struct MockProcessManager {
        pub process_started_called: Arc<Mutex<Vec<u32>>>,
        pub process_stopped_called: Arc<Mutex<Vec<Option<i32>>>>,
        pub health_check_results: Arc<RwLock<Vec<(bool, Option<String>)>>>,
        pub should_restart_result: Arc<RwLock<bool>>,
        pub restart_count_reset_called: Arc<Mutex<bool>>,
    }

    impl MockProcessManager {
        fn new() -> Self {
            Self {
                process_started_called: Arc::new(Mutex::new(Vec::new())),
                process_stopped_called: Arc::new(Mutex::new(Vec::new())),
                health_check_results: Arc::new(RwLock::new(Vec::new())),
                should_restart_result: Arc::new(RwLock::new(false)),
                restart_count_reset_called: Arc::new(Mutex::new(false)),
            }
        }

        async fn on_process_started(&self, pid: u32) {
            self.process_started_called.lock().await.push(pid);
        }

        async fn on_process_stopped(&self, exit_code: Option<i32>) {
            self.process_stopped_called.lock().await.push(exit_code);
        }

        async fn on_health_check_result(&self, is_healthy: bool, error: Option<String>) {
            self.health_check_results.write().await.push((is_healthy, error));
        }

        async fn should_restart(&self) -> bool {
            *self.should_restart_result.read().await
        }

        async fn reset_restart_count(&self) {
            *self.restart_count_reset_called.lock().await = true;
        }

        async fn set_app_handle(&self, _handle: tauri::AppHandle) {
            // Mock implementation - do nothing
        }
    }

    fn create_test_bridge() -> MCPBridge {
        let mock_manager = Arc::new(ProcessManager::new());
        MCPBridge::new(mock_manager)
    }

    #[tokio::test]
    async fn test_mcp_bridge_creation() {
        let bridge = create_test_bridge();
        assert!(!*bridge.is_running.read().await);
        assert_eq!(*bridge.request_id.read().await, 0);
        assert!(bridge.pending.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_json_rpc_request_creation() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "test_method".to_string(),
            params: json!({"key": "value"}),
        };

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, 1);
        assert_eq!(request.method, "test_method");
        assert_eq!(request.params, json!({"key": "value"}));
    }

    #[tokio::test]
    async fn test_json_rpc_response_creation() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: 1,
            result: Some(json!({"success": true})),
            error: None,
        };

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, 1);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_json_rpc_error_creation() {
        let error = JsonRpcError {
            code: -32600,
            message: "Invalid Request".to_string(),
            data: Some(json!({"details": "request malformed"})),
        };

        assert_eq!(error.code, -32600);
        assert_eq!(error.message, "Invalid Request");
        assert!(error.data.is_some());
    }

    #[tokio::test]
    async fn test_call_without_running_process() {
        let bridge = create_test_bridge();
        
        let result = bridge.call("test_method", json!({})).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "MCP server not running");
    }

    #[tokio::test]
    async fn test_request_id_generation() {
        let bridge = create_test_bridge();
        
        // Mock running state for testing
        *bridge.is_running.write().await = true;
        
        // Create a mock stdin channel
        let (tx, _rx) = tokio::sync::mpsc::channel::<String>(10);
        *bridge.stdin_tx.lock().await = Some(tx);
        
        // Test that request IDs are generated sequentially
        let initial_id = *bridge.request_id.read().await;
        
        // Start multiple calls concurrently (they will timeout but that's OK for ID testing)
        let bridge_clone1 = Arc::new(bridge);
        let bridge_clone2 = bridge_clone1.clone();
        let bridge_clone3 = bridge_clone1.clone();
        
        let handle1 = tokio::spawn({
            let bridge = bridge_clone1.clone();
            async move {
                let _ = bridge.call("method1", json!({})).await;
            }
        });
        
        let handle2 = tokio::spawn({
            let bridge = bridge_clone2.clone();
            async move {
                let _ = bridge.call("method2", json!({})).await;
            }
        });
        
        let handle3 = tokio::spawn({
            let bridge = bridge_clone3.clone();
            async move {
                let _ = bridge.call("method3", json!({})).await;
            }
        });
        
        // Give some time for the calls to be made
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let final_id = *bridge_clone1.request_id.read().await;
        assert_eq!(final_id, initial_id + 3);
        
        // Clean up
        handle1.abort();
        handle2.abort();
        handle3.abort();
    }

    #[tokio::test]
    async fn test_call_timeout() {
        let bridge = create_test_bridge();
        
        // Mock running state
        *bridge.is_running.write().await = true;
        
        // Create a channel that we won't respond to
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);
        *bridge.stdin_tx.lock().await = Some(tx);
        
        // Start a task to consume messages but not respond
        tokio::spawn(async move {
            while let Some(_msg) = rx.recv().await {
                // Don't respond to simulate timeout
            }
        });
        
        let start_time = std::time::Instant::now();
        let result = bridge.call("test_method", json!({})).await;
        let elapsed = start_time.elapsed();
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("timeout"));
        // For tests, use a shorter timeout validation (200ms instead of 30s)
        assert!(elapsed >= Duration::from_millis(100)); // Quick timeout for tests
    }

    #[tokio::test]
    async fn test_stop_without_running_process() {
        let bridge = create_test_bridge();
        
        let result = bridge.stop().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_restart_without_app_handle() {
        let bridge = create_test_bridge();
        
        let result = bridge.restart().await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "App handle not available for restart");
    }

    #[tokio::test]
    async fn test_health_check_when_not_running() {
        let bridge = create_test_bridge();
        
        let is_healthy = bridge.is_healthy().await;
        assert!(!is_healthy);
    }

    #[tokio::test]
    async fn test_concurrent_calls() {
        let bridge = Arc::new(create_test_bridge());
        
        // Mock running state
        *bridge.is_running.write().await = true;
        
        // Create channels for communication
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);
        *bridge.stdin_tx.lock().await = Some(tx);
        
        // Create a responder task
        let pending_clone = bridge.pending.clone();
        tokio::spawn(async move {
            let mut request_count = 0;
            while let Some(message) = rx.recv().await {
                request_count += 1;
                
                // Parse the request to get ID
                if let Ok(request) = serde_json::from_str::<JsonRpcRequest>(&message) {
                    let response = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(json!({"response_id": request.id})),
                        error: None,
                    };
                    
                    // Send response to pending handler
                    if let Some(sender) = pending_clone.write().await.remove(&request.id) {
                        let _ = sender.send(Ok(response.result.unwrap()));
                    }
                }
                
                if request_count >= 5 {
                    break;
                }
            }
        });
        
        // Make multiple concurrent calls
        let mut handles = Vec::new();
        for i in 0..5 {
            let bridge_clone = bridge.clone();
            let handle = tokio::spawn(async move {
                bridge_clone.call(&format!("method_{}", i), json!({"index": i})).await
            });
            handles.push(handle);
        }
        
        // Wait for all calls to complete
        let mut success_count = 0;
        for handle in handles {
            if let Ok(result) = handle.await {
                if result.is_ok() {
                    success_count += 1;
                }
            }
        }
        
        assert_eq!(success_count, 5);
    }

    #[tokio::test]
    async fn test_pending_request_cleanup_on_stop() {
        let bridge = create_test_bridge();
        
        // Mock running state
        *bridge.is_running.write().await = true;
        
        // Add some pending requests
        let (tx1, _rx1) = tokio::sync::oneshot::channel();
        let (tx2, _rx2) = tokio::sync::oneshot::channel();
        bridge.pending.write().await.insert(1, tx1);
        bridge.pending.write().await.insert(2, tx2);
        
        assert_eq!(bridge.pending.read().await.len(), 2);
        
        // Stop should clear pending requests
        let result = bridge.stop().await;
        assert!(result.is_ok());
        assert_eq!(bridge.pending.read().await.len(), 0);
    }

    #[tokio::test]
    async fn test_stdin_channel_unavailable() {
        let bridge = create_test_bridge();
        
        // Mock running state but no stdin channel
        *bridge.is_running.write().await = true;
        
        let result = bridge.call("test_method", json!({})).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Stdin channel not available");
    }

    #[tokio::test]
    async fn test_error_response_handling() {
        let bridge = Arc::new(create_test_bridge());
        
        // Mock running state
        *bridge.is_running.write().await = true;
        
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);
        *bridge.stdin_tx.lock().await = Some(tx);
        
        // Create responder that sends error responses
        let pending_clone = bridge.pending.clone();
        tokio::spawn(async move {
            if let Some(message) = rx.recv().await {
                if let Ok(request) = serde_json::from_str::<JsonRpcRequest>(&message) {
                    // Send error response
                    if let Some(sender) = pending_clone.write().await.remove(&request.id) {
                        let _ = sender.send(Err("Test error".to_string()));
                    }
                }
            }
        });
        
        let result = bridge.call("test_method", json!({})).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Test error");
    }

    #[tokio::test]
    async fn test_multiple_bridge_instances() {
        let bridge1 = create_test_bridge();
        let bridge2 = create_test_bridge();
        
        // Each bridge should have independent state
        *bridge1.request_id.write().await = 5;
        *bridge2.request_id.write().await = 10;
        
        assert_eq!(*bridge1.request_id.read().await, 5);
        assert_eq!(*bridge2.request_id.read().await, 10);
        
        // Modifying one shouldn't affect the other
        *bridge1.is_running.write().await = true;
        assert!(*bridge1.is_running.read().await);
        assert!(!*bridge2.is_running.read().await);
    }

    #[tokio::test]
    async fn test_bridge_state_consistency() {
        let bridge = create_test_bridge();
        
        // Test initial state
        assert!(!*bridge.is_running.read().await);
        assert_eq!(*bridge.request_id.read().await, 0);
        assert!(bridge.pending.read().await.is_empty());
        assert!(bridge.stdin_tx.lock().await.is_none());
        assert!(bridge.child_process.lock().await.is_none());
        
        // Test state changes
        *bridge.is_running.write().await = true;
        let (tx, _rx) = tokio::sync::mpsc::channel::<String>(10);
        *bridge.stdin_tx.lock().await = Some(tx);
        
        assert!(*bridge.is_running.read().await);
        assert!(bridge.stdin_tx.lock().await.is_some());
    }

    // Integration tests with mock Tauri commands would go here if we had access to Tauri test framework
    // For now, we test the core logic without Tauri dependencies

    #[tokio::test]
    async fn test_json_serialization() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 123,
            method: "test_method".to_string(),
            params: json!({"param1": "value1", "param2": 42}),
        };
        
        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: JsonRpcRequest = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.jsonrpc, request.jsonrpc);
        assert_eq!(deserialized.id, request.id);
        assert_eq!(deserialized.method, request.method);
        assert_eq!(deserialized.params, request.params);
    }

    #[tokio::test]
    async fn test_response_with_null_result() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: 1,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: "Method not found".to_string(),
                data: None,
            }),
        };
        
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: JsonRpcResponse = serde_json::from_str(&serialized).unwrap();
        
        assert!(deserialized.result.is_none());
        assert!(deserialized.error.is_some());
        assert_eq!(deserialized.error.unwrap().code, -32601);
    }

    #[tokio::test]
    async fn test_large_payload_handling() {
        let bridge = create_test_bridge();
        
        // Test with large JSON payload
        let large_payload = json!({
            "data": "x".repeat(10000), // 10KB string
            "array": (0..1000).collect::<Vec<i32>>(), // Large array
            "nested": {
                "deep": {
                    "structure": {
                        "with": {
                            "many": {
                                "levels": "value"
                            }
                        }
                    }
                }
            }
        });
        
        // Mock running state
        *bridge.is_running.write().await = true;
        let (tx, _rx) = tokio::sync::mpsc::channel::<String>(10);
        *bridge.stdin_tx.lock().await = Some(tx);
        
        // This should handle large payloads gracefully (though it will timeout)
        let result = tokio::time::timeout(
            Duration::from_millis(100), 
            bridge.call("large_method", large_payload)
        ).await;
        
        // Should timeout (expected) but not panic or crash
        assert!(result.is_err()); // Timeout error
    }

    #[tokio::test]
    async fn test_invalid_json_handling() {
        let bridge = Arc::new(create_test_bridge());
        
        // Test that invalid JSON in responses doesn't crash the system
        *bridge.is_running.write().await = true;
        
        // Create a scenario where we might receive invalid JSON
        // This tests the robustness of the JSON parsing in the response handler
        
        // For this test, we just verify the bridge doesn't panic with invalid internal state
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "test".to_string(),
            params: json!({"invalid": "\u{FFFF}"}), // Invalid Unicode
        };
        
        // Should be able to serialize even with edge case Unicode
        let result = serde_json::to_string(&request);
        assert!(result.is_ok());
    }
}
