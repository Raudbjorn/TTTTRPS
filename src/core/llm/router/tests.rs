//! LLM Router Tests
//!
//! Comprehensive test suite for the LLM router including mock providers,
//! routing strategies, failover, cost tracking, and streaming.

use super::*;
use crate::core::llm::cost::{CostTracker, ProviderPricing, TokenUsage};
use crate::core::llm::health::{CircuitBreaker, CircuitBreakerConfig, CircuitState, HealthTracker};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

// ========================================================================
// Mock Provider Implementation
// ========================================================================

/// Mock LLM provider for testing with configurable behavior
#[derive(Debug)]
struct MockProvider {
    id: String,
    name: String,
    model: String,
    healthy: Arc<RwLock<bool>>,
    should_succeed: Arc<RwLock<bool>>,
    error_type: Arc<RwLock<MockErrorType>>,
    response_content: Arc<RwLock<String>>,
    latency_ms: Arc<RwLock<u64>>,
    token_usage: Arc<RwLock<Option<TokenUsage>>>,
    call_count: Arc<AtomicU32>,
    supports_streaming_flag: bool,
    pricing: Option<ProviderPricing>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Variants reserved for comprehensive error testing
enum MockErrorType {
    None,
    ApiError { status: u16, message: String },
    RateLimited { retry_after: u64 },
    AuthError(String),
    Timeout,
}

impl MockProvider {
    fn new(id: &str, model: &str) -> Self {
        Self {
            id: id.to_string(),
            name: format!("Mock {}", id),
            model: model.to_string(),
            healthy: Arc::new(RwLock::new(true)),
            should_succeed: Arc::new(RwLock::new(true)),
            error_type: Arc::new(RwLock::new(MockErrorType::None)),
            response_content: Arc::new(RwLock::new("Mock response".to_string())),
            latency_ms: Arc::new(RwLock::new(10)),
            token_usage: Arc::new(RwLock::new(Some(TokenUsage::new(100, 50)))),
            call_count: Arc::new(AtomicU32::new(0)),
            supports_streaming_flag: true,
            pricing: None,
        }
    }

    fn with_streaming(mut self, supports: bool) -> Self {
        self.supports_streaming_flag = supports;
        self
    }

    async fn set_healthy(&self, healthy: bool) {
        *self.healthy.write().await = healthy;
    }

    async fn set_should_succeed(&self, succeed: bool) {
        *self.should_succeed.write().await = succeed;
    }

    async fn set_error_type(&self, error_type: MockErrorType) {
        *self.error_type.write().await = error_type;
    }

    async fn set_response(&self, content: &str) {
        *self.response_content.write().await = content.to_string();
    }

    async fn set_latency(&self, ms: u64) {
        *self.latency_ms.write().await = ms;
    }

    fn call_count(&self) -> u32 {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl LLMProvider for MockProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn health_check(&self) -> bool {
        *self.healthy.read().await
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        self.pricing.clone()
    }

    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse> {
        self.call_count.fetch_add(1, Ordering::SeqCst);

        let latency = *self.latency_ms.read().await;
        if latency > 0 {
            tokio::time::sleep(Duration::from_millis(latency)).await;
        }

        let should_succeed = *self.should_succeed.read().await;
        if !should_succeed {
            let error_type = self.error_type.read().await.clone();
            return Err(match error_type {
                MockErrorType::None => LLMError::ApiError {
                    status: 500,
                    message: "Mock error".to_string(),
                },
                MockErrorType::ApiError { status, message } => {
                    LLMError::ApiError { status, message }
                }
                MockErrorType::RateLimited { retry_after } => LLMError::RateLimited {
                    retry_after_secs: retry_after,
                },
                MockErrorType::AuthError(msg) => LLMError::AuthError(msg),
                MockErrorType::Timeout => LLMError::Timeout,
            });
        }

        let content = self.response_content.read().await.clone();
        let usage = self.token_usage.read().await.clone();

        Ok(ChatResponse {
            content,
            model: self.model.clone(),
            provider: self.id.clone(),
            usage,
            finish_reason: Some("stop".to_string()),
            latency_ms: latency,
            cost_usd: None,
            tool_calls: None,
        })
    }

    async fn stream_chat(&self, _request: ChatRequest) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        self.call_count.fetch_add(1, Ordering::SeqCst);

        if !self.supports_streaming_flag {
            return Err(LLMError::StreamingNotSupported(self.id.clone()));
        }

        let should_succeed = *self.should_succeed.read().await;
        if !should_succeed {
            return Err(LLMError::ApiError {
                status: 500,
                message: "Mock streaming error".to_string(),
            });
        }

        let (tx, rx) = mpsc::channel(10);
        let content = self.response_content.read().await.clone();
        let usage = self.token_usage.read().await.clone();
        let provider = self.id.clone();
        let model = self.model.clone();
        let latency = *self.latency_ms.read().await;

        tokio::spawn(async move {
            let stream_id = uuid::Uuid::new_v4().to_string();
            let words: Vec<&str> = content.split_whitespace().collect();

            for (i, word) in words.iter().enumerate() {
                if latency > 0 {
                    tokio::time::sleep(Duration::from_millis(latency / 10)).await;
                }

                let chunk = ChatChunk {
                    stream_id: stream_id.clone(),
                    content: format!("{} ", word),
                    provider: provider.clone(),
                    model: model.clone(),
                    is_final: false,
                    finish_reason: None,
                    usage: None,
                    index: i as u32,
                };
                if tx.send(Ok(chunk)).await.is_err() {
                    break;
                }
            }

            let final_chunk = ChatChunk {
                stream_id: stream_id.clone(),
                content: String::new(),
                provider: provider.clone(),
                model: model.clone(),
                is_final: true,
                finish_reason: Some("stop".to_string()),
                usage,
                index: words.len() as u32,
            };
            let _ = tx.send(Ok(final_chunk)).await;
        });

        Ok(rx)
    }

    fn supports_streaming(&self) -> bool {
        self.supports_streaming_flag
    }
}

// ========================================================================
// Helper Functions
// ========================================================================

fn create_test_request() -> ChatRequest {
    ChatRequest::new(vec![ChatMessage::user("Hello, world!")])
}

fn create_mock_provider(id: &str) -> Arc<MockProvider> {
    Arc::new(MockProvider::new(id, &format!("{}-model", id)))
}

fn create_mock_provider_with_model(id: &str, model: &str) -> Arc<MockProvider> {
    Arc::new(MockProvider::new(id, model))
}

// ========================================================================
// Basic Unit Tests (existing)
// ========================================================================

#[test]
fn test_chat_message_builders() {
    let system = ChatMessage::system("You are helpful");
    assert_eq!(system.role, MessageRole::System);

    let user = ChatMessage::user("Hello");
    assert_eq!(user.role, MessageRole::User);

    let assistant = ChatMessage::assistant("Hi there");
    assert_eq!(assistant.role, MessageRole::Assistant);
}

#[test]
fn test_chat_request_builder() {
    let request = ChatRequest::new(vec![ChatMessage::user("Hi")])
        .with_system("Be helpful")
        .with_temperature(0.7)
        .with_max_tokens(1000)
        .with_provider("openai");

    assert_eq!(request.system_prompt, Some("Be helpful".to_string()));
    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.max_tokens, Some(1000));
    assert_eq!(request.provider, Some("openai".to_string()));
}

#[test]
fn test_provider_stats() {
    let mut stats = ProviderStats::default();

    stats.record_success(100, Some(&TokenUsage::new(100, 50)), 0.01);
    assert_eq!(stats.successful_requests, 1);
    assert_eq!(stats.total_latency_ms, 100);
    assert_eq!(stats.total_input_tokens, 100);
    assert_eq!(stats.total_output_tokens, 50);

    stats.record_failure();
    assert_eq!(stats.total_requests, 2);
    assert_eq!(stats.failed_requests, 1);

    assert_eq!(stats.success_rate(), 0.5);
    assert_eq!(stats.avg_latency_ms(), 100);
}

#[test]
fn test_routing_strategy() {
    assert_eq!(RoutingStrategy::default(), RoutingStrategy::Priority);
    assert_ne!(RoutingStrategy::Priority, RoutingStrategy::CostOptimized);
}

#[test]
fn test_router_config_defaults() {
    let config = RouterConfig::default();
    assert_eq!(config.request_timeout, Duration::from_secs(120));
    assert!(config.enable_fallback);
    assert_eq!(config.routing_strategy, RoutingStrategy::Priority);
}

#[test]
fn test_message_role_display() {
    assert_eq!(MessageRole::System.to_string(), "system");
    assert_eq!(MessageRole::User.to_string(), "user");
    assert_eq!(MessageRole::Assistant.to_string(), "assistant");
}

#[test]
fn test_chat_message_constructors() {
    let user = ChatMessage::user("Hello");
    assert_eq!(user.role, MessageRole::User);
    assert_eq!(user.content, "Hello");

    let assistant = ChatMessage::assistant("Hi!");
    assert_eq!(assistant.role, MessageRole::Assistant);
    assert_eq!(assistant.content, "Hi!");

    let system = ChatMessage::system("You are helpful.");
    assert_eq!(system.role, MessageRole::System);
    assert_eq!(system.content, "You are helpful.");
}

#[test]
fn test_chat_request_new() {
    let messages = vec![ChatMessage::user("Test")];
    let request = ChatRequest::new(messages.clone());
    assert_eq!(request.messages.len(), 1);
    assert!(request.system_prompt.is_none());
    assert!(request.max_tokens.is_none());
}

#[test]
fn test_chat_request_with_system() {
    let messages = vec![ChatMessage::user("Test")];
    let request = ChatRequest::new(messages).with_system("Be helpful");
    assert_eq!(request.system_prompt, Some("Be helpful".to_string()));
}

// ========================================================================
// Provider Selection Tests - Single Provider
// ========================================================================

#[tokio::test]
async fn test_single_provider_selection() {
    let mut router = LLMRouter::new(RouterConfig::default());
    let provider = create_mock_provider("test");

    router.add_provider(provider.clone()).await;

    let request = create_test_request();
    let result = router.chat(request).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.provider, "test");
    assert_eq!(provider.call_count(), 1);
}

#[tokio::test]
async fn test_single_provider_unhealthy() {
    let mut router = LLMRouter::new(RouterConfig::default());
    let provider = create_mock_provider("test");
    provider.set_healthy(false).await;

    router.add_provider(provider.clone()).await;

    // Record failures to trigger unhealthy state
    for _ in 0..3 {
        router
            .health_tracker
            .write()
            .await
            .record_failure("test", "test failure");
    }

    let request = create_test_request();
    let result = router.chat(request).await;

    assert!(result.is_err());
}

// ========================================================================
// Provider Selection Tests - Multiple Providers (Cost-Based)
// ========================================================================

#[tokio::test]
async fn test_cost_optimized_routing_selects_cheapest() {
    let config = RouterConfig {
        routing_strategy: RoutingStrategy::CostOptimized,
        ..Default::default()
    };
    let mut router = LLMRouter::new(config);

    let expensive = create_mock_provider("expensive");
    router.add_provider(expensive.clone()).await;

    let cheap = create_mock_provider("cheap");
    router.add_provider(cheap.clone()).await;

    // Record usage with different costs
    {
        let mut stats = router.stats.write().await;
        stats.get_mut("expensive").unwrap().record_success(
            100,
            Some(&TokenUsage::new(1000, 500)),
            1.0,
        );
        stats.get_mut("cheap").unwrap().record_success(
            100,
            Some(&TokenUsage::new(1000, 500)),
            0.1,
        );
    }

    // Test that get_next_provider selects the cheapest based on recorded stats
    // Note: The chat() method iterates providers in priority order for failover,
    // but get_next_provider applies the routing strategy for initial selection
    let request = create_test_request();
    let selected = router.get_next_provider(&request).await;

    assert!(selected.is_some());
    assert_eq!(selected.unwrap().id(), "cheap");
}

#[tokio::test]
async fn test_latency_optimized_routing_selects_fastest() {
    let config = RouterConfig {
        routing_strategy: RoutingStrategy::LatencyOptimized,
        ..Default::default()
    };
    let mut router = LLMRouter::new(config);

    let slow = create_mock_provider("slow");
    router.add_provider(slow.clone()).await;

    let fast = create_mock_provider("fast");
    router.add_provider(fast.clone()).await;

    // Record stats with different latencies
    {
        let mut stats = router.stats.write().await;
        stats.get_mut("slow").unwrap().record_success(1000, None, 0.0);
        stats.get_mut("fast").unwrap().record_success(50, None, 0.0);
    }

    // Test that get_next_provider selects the fastest based on recorded stats
    // Note: The chat() method iterates providers in priority order for failover,
    // but get_next_provider applies the routing strategy for initial selection
    let request = create_test_request();
    let selected = router.get_next_provider(&request).await;

    assert!(selected.is_some());
    assert_eq!(selected.unwrap().id(), "fast");
}

#[tokio::test]
async fn test_round_robin_routing() {
    let config = RouterConfig {
        routing_strategy: RoutingStrategy::RoundRobin,
        ..Default::default()
    };
    let mut router = LLMRouter::new(config);

    let provider1 = create_mock_provider("provider1");
    let provider2 = create_mock_provider("provider2");
    let provider3 = create_mock_provider("provider3");

    router.add_provider(provider1.clone()).await;
    router.add_provider(provider2.clone()).await;
    router.add_provider(provider3.clone()).await;

    let mut providers_used = Vec::new();
    for _ in 0..6 {
        let request = create_test_request();
        let result = router.chat(request).await;
        assert!(result.is_ok());
        providers_used.push(result.unwrap().provider);
    }

    // Should cycle through providers
    assert_eq!(providers_used[0], providers_used[3]);
    assert_eq!(providers_used[1], providers_used[4]);
    assert_eq!(providers_used[2], providers_used[5]);
}

#[tokio::test]
async fn test_priority_routing_uses_first_available() {
    let config = RouterConfig {
        routing_strategy: RoutingStrategy::Priority,
        ..Default::default()
    };
    let mut router = LLMRouter::new(config);

    let primary = create_mock_provider("primary");
    let secondary = create_mock_provider("secondary");

    router.add_provider(primary.clone()).await;
    router.add_provider(secondary.clone()).await;

    let request = create_test_request();
    let result = router.chat(request).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().provider, "primary");
    assert_eq!(primary.call_count(), 1);
    assert_eq!(secondary.call_count(), 0);
}

// ========================================================================
// Provider Selection Tests - Capability Based
// ========================================================================

#[tokio::test]
async fn test_specific_provider_request() {
    let mut router = LLMRouter::new(RouterConfig::default());

    let provider1 = create_mock_provider("provider1");
    let provider2 = create_mock_provider("provider2");

    router.add_provider(provider1.clone()).await;
    router.add_provider(provider2.clone()).await;

    let request = create_test_request().with_provider("provider2");
    let result = router.chat(request).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().provider, "provider2");
    assert_eq!(provider2.call_count(), 1);
    assert_eq!(provider1.call_count(), 0);
}

#[tokio::test]
async fn test_streaming_provider_selection() {
    let mut router = LLMRouter::new(RouterConfig::default());

    // Add streaming provider first so it's selected by priority routing
    let streaming = Arc::new(MockProvider::new("streaming", "model").with_streaming(true));
    router.add_provider(streaming.clone()).await;

    let non_streaming = Arc::new(MockProvider::new("non_streaming", "model").with_streaming(false));
    router.add_provider(non_streaming.clone()).await;

    let request = create_test_request();
    let result = router.stream_chat(request).await;

    // stream_chat uses get_next_provider which returns the first available provider
    // (priority routing), so streaming provider must be added first
    assert!(result.is_ok());
}

// ========================================================================
// Failover Tests
// ========================================================================

#[tokio::test]
async fn test_failover_when_primary_fails() {
    let config = RouterConfig {
        enable_fallback: true,
        ..Default::default()
    };
    let mut router = LLMRouter::new(config);

    let primary = create_mock_provider("primary");
    primary.set_should_succeed(false).await;

    let secondary = create_mock_provider("secondary");

    router.add_provider(primary.clone()).await;
    router.add_provider(secondary.clone()).await;

    let request = create_test_request();
    let result = router.chat(request).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().provider, "secondary");
    assert_eq!(primary.call_count(), 1);
    assert_eq!(secondary.call_count(), 1);
}

#[tokio::test]
async fn test_failover_disabled() {
    let config = RouterConfig {
        enable_fallback: false,
        ..Default::default()
    };
    let mut router = LLMRouter::new(config);

    let primary = create_mock_provider("primary");
    primary.set_should_succeed(false).await;

    let secondary = create_mock_provider("secondary");

    router.add_provider(primary.clone()).await;
    router.add_provider(secondary.clone()).await;

    let request = create_test_request();
    let result = router.chat(request).await;

    assert!(result.is_err());
    assert_eq!(primary.call_count(), 1);
    assert_eq!(secondary.call_count(), 0);
}

#[tokio::test]
async fn test_failover_chain_exhaustion() {
    let config = RouterConfig {
        enable_fallback: true,
        ..Default::default()
    };
    let mut router = LLMRouter::new(config);

    let provider1 = create_mock_provider("provider1");
    provider1.set_should_succeed(false).await;
    let provider2 = create_mock_provider("provider2");
    provider2.set_should_succeed(false).await;
    let provider3 = create_mock_provider("provider3");
    provider3.set_should_succeed(false).await;

    router.add_provider(provider1.clone()).await;
    router.add_provider(provider2.clone()).await;
    router.add_provider(provider3.clone()).await;

    let request = create_test_request();
    let result = router.chat(request).await;

    assert!(result.is_err());
    // When all providers fail, the router returns the last error encountered.
    // The MockProvider returns ApiError with status 500 when should_succeed is false.
    // The 503 "All providers failed" message is only returned when no providers were tried.
    match result.unwrap_err() {
        LLMError::ApiError { status, .. } => {
            assert_eq!(status, 500);
        }
        _ => panic!("Expected ApiError with status 500"),
    }

    // Verify all providers were tried
    assert_eq!(provider1.call_count(), 1);
    assert_eq!(provider2.call_count(), 1);
    assert_eq!(provider3.call_count(), 1);
}

#[tokio::test]
async fn test_failover_skips_unhealthy_providers() {
    let config = RouterConfig {
        enable_fallback: true,
        ..Default::default()
    };
    let mut router = LLMRouter::new(config);

    let provider1 = create_mock_provider("provider1");
    let provider2 = create_mock_provider("provider2");
    let provider3 = create_mock_provider("provider3");

    router.add_provider(provider1.clone()).await;
    router.add_provider(provider2.clone()).await;
    router.add_provider(provider3.clone()).await;

    // Make provider1 unhealthy
    for _ in 0..3 {
        router
            .health_tracker
            .write()
            .await
            .record_failure("provider1", "test failure");
    }

    // Make provider2 fail
    provider2.set_should_succeed(false).await;

    let request = create_test_request();
    let result = router.chat(request).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().provider, "provider3");
    assert_eq!(provider1.call_count(), 0); // Skipped
    assert_eq!(provider2.call_count(), 1); // Tried but failed
    assert_eq!(provider3.call_count(), 1); // Succeeded
}

// ========================================================================
// Cost Calculation Tests
// ========================================================================

#[test]
fn test_cost_calculation_claude_sonnet() {
    let usage = TokenUsage::new(1000, 500);
    let pricing = ProviderPricing::for_model("claude", "claude-3-5-sonnet").unwrap();
    let cost = pricing.calculate_cost(&usage);
    // (1000/1M * 3.0) + (500/1M * 15.0) = 0.003 + 0.0075 = 0.0105
    assert!((cost - 0.0105).abs() < 0.0001);
}

#[test]
fn test_cost_calculation_openai_gpt4o() {
    let usage = TokenUsage::new(1000, 500);
    let pricing = ProviderPricing::for_model("openai", "gpt-4o").unwrap();
    let cost = pricing.calculate_cost(&usage);
    // (1000/1M * 2.5) + (500/1M * 10.0) = 0.0025 + 0.005 = 0.0075
    assert!((cost - 0.0075).abs() < 0.0001);
}

#[test]
fn test_cost_calculation_ollama_free() {
    let usage = TokenUsage::new(100000, 50000);
    let pricing = ProviderPricing::for_model("ollama", "llama3").unwrap();
    let cost = pricing.calculate_cost(&usage);
    assert_eq!(cost, 0.0);
    assert!(pricing.is_free);
}

#[test]
fn test_cost_tracker_accumulation() {
    let mut tracker = CostTracker::new();

    let usage1 = TokenUsage::new(1000, 500);
    let cost1 = tracker.record_usage("claude", "claude-3-5-sonnet", &usage1);

    let usage2 = TokenUsage::new(2000, 1000);
    let cost2 = tracker.record_usage("claude", "claude-3-5-sonnet", &usage2);

    let provider_costs = tracker.costs.get("claude").unwrap();
    assert_eq!(provider_costs.request_count, 2);
    assert_eq!(provider_costs.input_tokens, 3000);
    assert_eq!(provider_costs.output_tokens, 1500);
    assert!((provider_costs.total_cost_usd - (cost1 + cost2)).abs() < 0.0001);
}

// ========================================================================
// Token Counting Tests
// ========================================================================

#[test]
fn test_token_usage_total() {
    let usage = TokenUsage::new(100, 50);
    assert_eq!(usage.total(), 150);
}

#[test]
fn test_token_usage_add() {
    let mut usage1 = TokenUsage::new(100, 50);
    let usage2 = TokenUsage::new(200, 100);
    usage1.add(&usage2);

    assert_eq!(usage1.input_tokens, 300);
    assert_eq!(usage1.output_tokens, 150);
    assert_eq!(usage1.total(), 450);
}

#[test]
fn test_provider_stats_token_tracking() {
    let mut stats = ProviderStats::default();

    stats.record_success(100, Some(&TokenUsage::new(1000, 500)), 0.01);
    assert_eq!(stats.total_input_tokens, 1000);
    assert_eq!(stats.total_output_tokens, 500);
    assert_eq!(stats.total_tokens(), 1500);

    stats.record_success(200, Some(&TokenUsage::new(2000, 1000)), 0.02);
    assert_eq!(stats.total_input_tokens, 3000);
    assert_eq!(stats.total_output_tokens, 1500);
    assert_eq!(stats.total_tokens(), 4500);
    assert_eq!(stats.avg_tokens_per_request(), 2250);
}

// ========================================================================
// Rate Limit Detection Tests
// ========================================================================

#[tokio::test]
async fn test_rate_limit_error_detection() {
    let config = RouterConfig {
        enable_fallback: true,
        ..Default::default()
    };
    let mut router = LLMRouter::new(config);

    let rate_limited = create_mock_provider("rate_limited");
    rate_limited.set_should_succeed(false).await;
    rate_limited
        .set_error_type(MockErrorType::RateLimited { retry_after: 60 })
        .await;

    let fallback = create_mock_provider("fallback");

    router.add_provider(rate_limited.clone()).await;
    router.add_provider(fallback.clone()).await;

    let request = create_test_request();
    let result = router.chat(request).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().provider, "fallback");
}

#[tokio::test]
async fn test_rate_limit_returns_proper_error() {
    let config = RouterConfig {
        enable_fallback: false,
        ..Default::default()
    };
    let mut router = LLMRouter::new(config);

    let provider = create_mock_provider("test");
    provider.set_should_succeed(false).await;
    provider
        .set_error_type(MockErrorType::RateLimited { retry_after: 30 })
        .await;

    router.add_provider(provider.clone()).await;

    let request = create_test_request();
    let result = router.chat(request).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        LLMError::RateLimited { retry_after_secs } => {
            assert_eq!(retry_after_secs, 30);
        }
        _ => panic!("Expected RateLimited error"),
    }
}

// ========================================================================
// Provider Health Check Tests
// ========================================================================

#[tokio::test]
async fn test_health_check_success() {
    let mut router = LLMRouter::new(RouterConfig::default());

    let provider = create_mock_provider("test");
    provider.set_healthy(true).await;

    router.add_provider(provider.clone()).await;

    let results = router.health_check_all().await;
    assert!(results.get("test").copied().unwrap_or(false));
}

#[tokio::test]
async fn test_health_check_failure() {
    let mut router = LLMRouter::new(RouterConfig::default());

    let provider = create_mock_provider("test");
    provider.set_healthy(false).await;

    router.add_provider(provider.clone()).await;

    let results = router.health_check_all().await;
    assert!(!results.get("test").copied().unwrap_or(true));
}

#[tokio::test]
async fn test_health_tracker_consecutive_failures() {
    let mut tracker = HealthTracker::default();
    tracker.add_provider("test");

    assert!(tracker.is_healthy("test"));

    tracker.record_failure("test", "error 1");
    tracker.record_failure("test", "error 2");
    assert!(tracker.is_healthy("test"));

    tracker.record_failure("test", "error 3");
    assert!(!tracker.is_healthy("test"));
}

#[tokio::test]
async fn test_health_recovery_on_success() {
    let mut tracker = HealthTracker::default();
    tracker.add_provider("test");

    tracker.record_failure("test", "error 1");
    tracker.record_failure("test", "error 2");
    tracker.record_failure("test", "error 3");
    assert!(!tracker.is_healthy("test"));

    tracker.reset_circuit("test");
    assert!(tracker.is_healthy("test"));
}

// ========================================================================
// Circuit Breaker Tests
// ========================================================================

#[test]
fn test_circuit_breaker_opens_on_failures() {
    let mut cb = CircuitBreaker::default();

    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.can_execute());

    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Closed);

    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
    assert!(!cb.can_execute());
}

#[test]
fn test_circuit_breaker_resets_on_success() {
    let mut cb = CircuitBreaker::default();

    cb.record_failure();
    cb.record_failure();
    cb.record_success();

    assert_eq!(cb.state(), CircuitState::Closed);
    assert_eq!(cb.failure_count(), 0);
}

#[test]
fn test_circuit_breaker_half_open_transitions() {
    let config = CircuitBreakerConfig {
        failure_threshold: 2,
        success_threshold: 2,
        timeout_duration: Duration::from_millis(10),
    };
    let mut cb = CircuitBreaker::with_config(config);

    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);

    std::thread::sleep(Duration::from_millis(15));

    assert!(cb.can_execute());
    assert_eq!(cb.state(), CircuitState::HalfOpen);

    cb.record_success();
    assert_eq!(cb.state(), CircuitState::HalfOpen);

    cb.record_success();
    assert_eq!(cb.state(), CircuitState::Closed);
}

#[test]
fn test_circuit_breaker_manual_reset() {
    let mut cb = CircuitBreaker::default();

    cb.record_failure();
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);

    cb.reset();
    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(cb.can_execute());
}

// ========================================================================
// Streaming Response Tests
// ========================================================================

#[tokio::test]
async fn test_streaming_response_assembly() {
    let mut router = LLMRouter::new(RouterConfig::default());

    let provider = create_mock_provider("test");
    provider.set_response("Hello world from streaming").await;

    router.add_provider(provider.clone()).await;

    let request = create_test_request();
    let result = router.stream_chat(request).await;

    assert!(result.is_ok());
    let mut rx = result.unwrap();

    let mut assembled_content = String::new();
    let mut chunk_count = 0;
    let mut received_final = false;

    while let Some(chunk_result) = rx.recv().await {
        let chunk = chunk_result.unwrap();
        assembled_content.push_str(&chunk.content);
        chunk_count += 1;

        if chunk.is_final {
            received_final = true;
            assert!(chunk.finish_reason.is_some());
        }
    }

    assert!(received_final);
    assert!(chunk_count > 1);
    assert!(assembled_content.contains("Hello"));
    assert!(assembled_content.contains("world"));
}

#[tokio::test]
async fn test_streaming_cancellation() {
    let mut router = LLMRouter::new(RouterConfig::default());

    let provider = create_mock_provider("test");
    // Use longer latency and more chunks to ensure stream stays active
    provider.set_latency(1000).await;
    provider
        .set_response("word1 word2 word3 word4 word5 word6 word7 word8 word9 word10 word11 word12 word13 word14 word15 word16 word17 word18 word19 word20")
        .await;

    router.add_provider(provider.clone()).await;

    let request = create_test_request();
    let result = router.stream_chat(request).await;

    assert!(result.is_ok());
    let mut rx = result.unwrap();

    // Wait for stream to actually start by polling for active streams
    let mut attempts = 0;
    let stream_id = loop {
        let stream_ids = router.active_stream_ids().await;
        if !stream_ids.is_empty() {
            break stream_ids[0].clone();
        }
        attempts += 1;
        if attempts > 50 {
            panic!("Stream never became active");
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };

    // Optionally receive a chunk to confirm streaming is in progress
    let first_chunk = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await;
    assert!(first_chunk.is_ok(), "Should receive at least one chunk before cancellation");

    // Now cancel the stream
    let canceled = router.cancel_stream(&stream_id).await;
    assert!(canceled);

    // Verify stream was interrupted: receiver should close without receiving all chunks
    // (the response has 20 words, so we should NOT get all 20+ chunks if canceled properly)
    let mut chunk_count = 1; // We already received one
    while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
        chunk_count += 1;
        if chunk_count > 25 {
            break; // Safety limit
        }
    }

    // We should have received fewer chunks than the full response would produce
    // (20 words = 20 content chunks + 1 final chunk = 21 total)
    assert!(chunk_count < 21, "Stream should have been interrupted before completion, got {} chunks", chunk_count);
}

#[tokio::test]
async fn test_streaming_not_supported_error() {
    let mut router = LLMRouter::new(RouterConfig::default());

    let provider = Arc::new(MockProvider::new("test", "model").with_streaming(false));
    router.add_provider(provider.clone()).await;

    let request = create_test_request();
    let result = router.stream_chat(request).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        LLMError::StreamingNotSupported(id) => {
            assert_eq!(id, "test");
        }
        _ => panic!("Expected StreamingNotSupported error"),
    }
}

// ========================================================================
// Model Compatibility Matrix Tests
// ========================================================================

#[test]
fn test_model_pricing_lookup_claude() {
    assert!(ProviderPricing::for_model("claude", "claude-3-5-sonnet").is_some());
    assert!(ProviderPricing::for_model("claude", "claude-3.5-sonnet").is_some());
    assert!(ProviderPricing::for_model("claude", "claude-3-5-haiku").is_some());
    assert!(ProviderPricing::for_model("claude", "claude-opus-4").is_some());
}

#[test]
fn test_model_pricing_lookup_openai() {
    assert!(ProviderPricing::for_model("openai", "gpt-4o").is_some());
    assert!(ProviderPricing::for_model("openai", "gpt-4o-mini").is_some());
    assert!(ProviderPricing::for_model("openai", "gpt-4-turbo").is_some());
    assert!(ProviderPricing::for_model("openai", "gpt-3.5-turbo").is_some());
}

#[test]
fn test_model_pricing_lookup_gemini() {
    assert!(ProviderPricing::for_model("gemini", "gemini-2.0-flash").is_some());
    assert!(ProviderPricing::for_model("gemini", "gemini-1.5-pro").is_some());
    assert!(ProviderPricing::for_model("gemini", "gemini-1.5-flash").is_some());
}

#[test]
fn test_model_context_window() {
    let claude = ProviderPricing::for_model("claude", "claude-3-5-sonnet").unwrap();
    assert_eq!(claude.context_window, Some(200_000));

    let gpt4o = ProviderPricing::for_model("openai", "gpt-4o").unwrap();
    assert_eq!(gpt4o.context_window, Some(128_000));

    let gemini = ProviderPricing::for_model("gemini", "gemini-1.5-pro").unwrap();
    assert_eq!(gemini.context_window, Some(2_000_000));
}

// ========================================================================
// Unknown Model Handling Tests
// ========================================================================

#[test]
fn test_unknown_provider_returns_none() {
    assert!(ProviderPricing::for_model("unknown_provider", "model").is_none());
}

#[test]
fn test_unknown_model_returns_none() {
    assert!(ProviderPricing::for_model("openai", "totally-unknown-model").is_none());
    assert!(ProviderPricing::for_model("claude", "nonexistent-model").is_none());
}

#[test]
fn test_cost_tracker_handles_unknown_model() {
    let tracker = CostTracker::new();
    let pricing = tracker.get_pricing("unknown", "unknown");
    assert!(pricing.is_none());

    let estimate = tracker.estimate_cost("unknown", "unknown", 1000, 500);
    assert_eq!(estimate, 0.0);
}

#[tokio::test]
async fn test_router_handles_provider_without_pricing() {
    let mut router = LLMRouter::new(RouterConfig::default());

    let provider = create_mock_provider("custom");
    router.add_provider(provider.clone()).await;

    let request = create_test_request();
    let result = router.chat(request).await;

    assert!(result.is_ok());
}

// ========================================================================
// Budget Enforcement Tests
// ========================================================================

#[tokio::test]
async fn test_budget_exceeded_blocks_requests() {
    let config = RouterConfig {
        monthly_budget: Some(0.001),
        ..Default::default()
    };
    let mut router = LLMRouter::new(config);

    let provider = create_mock_provider_with_model("openai", "gpt-4o");
    router.add_provider(provider.clone()).await;

    let request = create_test_request();
    let result = router.chat(request).await;
    assert!(result.is_ok());

    // Set cost to exceed budget
    router.cost_tracker.write().await.monthly_cost = 0.01;

    let request = create_test_request();
    let result = router.chat(request).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        LLMError::BudgetExceeded(msg) => {
            assert!(msg.contains("budget"));
        }
        _ => panic!("Expected BudgetExceeded error"),
    }
}

#[test]
fn test_cost_tracker_budget_tracking() {
    let mut tracker = CostTracker::new();
    tracker.monthly_budget = Some(10.0);
    tracker.daily_budget = Some(1.0);

    assert!(tracker.is_within_budget());
    assert_eq!(tracker.remaining_monthly_budget(), Some(10.0));
    assert_eq!(tracker.remaining_daily_budget(), Some(1.0));

    let usage = TokenUsage::new(1000000, 500000);
    tracker.record_usage("openai", "gpt-4o", &usage);

    assert!(tracker.monthly_cost > 0.0);
    assert!(tracker.daily_cost > 0.0);

    tracker.monthly_cost = 15.0;
    assert!(!tracker.is_within_monthly_budget());
    assert_eq!(tracker.remaining_monthly_budget(), Some(0.0));
}

// ========================================================================
// Provider Stats Tests
// ========================================================================

#[test]
fn test_provider_stats_success_rate() {
    let mut stats = ProviderStats::default();

    stats.record_success(100, None, 0.0);
    stats.record_success(100, None, 0.0);
    stats.record_failure();

    assert_eq!(stats.total_requests, 3);
    assert_eq!(stats.successful_requests, 2);
    assert_eq!(stats.failed_requests, 1);
    assert!((stats.success_rate() - 0.666666).abs() < 0.01);
}

#[test]
fn test_provider_stats_average_latency() {
    let mut stats = ProviderStats::default();

    stats.record_success(100, None, 0.0);
    stats.record_success(200, None, 0.0);
    stats.record_success(300, None, 0.0);

    assert_eq!(stats.avg_latency_ms(), 200);
}

#[test]
fn test_provider_stats_empty() {
    let stats = ProviderStats::default();

    assert_eq!(stats.success_rate(), 1.0);
    assert_eq!(stats.avg_latency_ms(), 0);
    assert_eq!(stats.avg_tokens_per_request(), 0);
}

// ========================================================================
// Router Builder Tests
// ========================================================================

#[tokio::test]
async fn test_router_builder() {
    let provider = create_mock_provider("test");

    let router = LLMRouterBuilder::new()
        .add_provider(provider.clone())
        .with_timeout(Duration::from_secs(30))
        .with_fallback(true)
        .with_routing_strategy(RoutingStrategy::CostOptimized)
        .with_monthly_budget(100.0)
        .build()
        .await;

    assert_eq!(router.routing_strategy(), RoutingStrategy::CostOptimized);
    assert_eq!(router.config().request_timeout, Duration::from_secs(30));
    assert!(router.config().enable_fallback);
    assert_eq!(router.config().monthly_budget, Some(100.0));

    let ids = router.provider_ids();
    assert!(ids.contains(&"test".to_string()));
}

// ========================================================================
// Timeout Tests
// ========================================================================

#[tokio::test]
async fn test_request_timeout() {
    let config = RouterConfig {
        request_timeout: Duration::from_millis(50),
        enable_fallback: false,
        ..Default::default()
    };
    let mut router = LLMRouter::new(config);

    let provider = create_mock_provider("slow");
    provider.set_latency(500).await;

    router.add_provider(provider.clone()).await;

    let request = create_test_request();
    let result = router.chat(request).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        LLMError::Timeout => {}
        e => panic!("Expected Timeout error, got {:?}", e),
    }
}

// ========================================================================
// Auth Error Tests
// ========================================================================

#[tokio::test]
async fn test_auth_error_handling() {
    let config = RouterConfig {
        enable_fallback: true,
        ..Default::default()
    };
    let mut router = LLMRouter::new(config);

    let bad_auth = create_mock_provider("bad_auth");
    bad_auth.set_should_succeed(false).await;
    bad_auth
        .set_error_type(MockErrorType::AuthError("Invalid API key".to_string()))
        .await;

    let working = create_mock_provider("working");

    router.add_provider(bad_auth.clone()).await;
    router.add_provider(working.clone()).await;

    let request = create_test_request();
    let result = router.chat(request).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().provider, "working");
}

// ========================================================================
// Router State Management Tests
// ========================================================================

#[tokio::test]
async fn test_add_remove_provider() {
    let mut router = LLMRouter::new(RouterConfig::default());

    let provider1 = create_mock_provider("provider1");
    let provider2 = create_mock_provider("provider2");

    router.add_provider(provider1).await;
    router.add_provider(provider2).await;

    let ids = router.provider_ids();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"provider1".to_string()));
    assert!(ids.contains(&"provider2".to_string()));

    router.remove_provider("provider1").await;

    let ids = router.provider_ids();
    assert_eq!(ids.len(), 1);
    assert!(!ids.contains(&"provider1".to_string()));
    assert!(ids.contains(&"provider2".to_string()));
}

#[tokio::test]
async fn test_get_provider() {
    let mut router = LLMRouter::new(RouterConfig::default());

    let provider = create_mock_provider("test");
    router.add_provider(provider).await;

    let retrieved = router.get_provider("test");
    assert!(retrieved.is_some());

    let not_found = router.get_provider("nonexistent");
    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_healthy_providers_list() {
    let mut router = LLMRouter::new(RouterConfig::default());

    let healthy1 = create_mock_provider("healthy1");
    let healthy2 = create_mock_provider("healthy2");
    let unhealthy = create_mock_provider("unhealthy");

    router.add_provider(healthy1).await;
    router.add_provider(healthy2).await;
    router.add_provider(unhealthy).await;

    for _ in 0..3 {
        router
            .health_tracker
            .write()
            .await
            .record_failure("unhealthy", "error");
    }

    let healthy_list = router.healthy_providers().await;
    assert_eq!(healthy_list.len(), 2);
    assert!(healthy_list.contains(&"healthy1".to_string()));
    assert!(healthy_list.contains(&"healthy2".to_string()));
    assert!(!healthy_list.contains(&"unhealthy".to_string()));
}

// ========================================================================
// Cost Summary Tests
// ========================================================================

#[tokio::test]
async fn test_cost_summary() {
    let mut router = LLMRouter::new(RouterConfig::default());

    let provider = create_mock_provider_with_model("claude", "claude-3-5-sonnet");
    router.add_provider(provider.clone()).await;

    let request = create_test_request();
    let _ = router.chat(request).await;

    let summary = router.get_cost_summary().await;
    assert!(summary.total_cost_usd >= 0.0);
    assert!(summary.monthly_cost >= 0.0);
}

#[tokio::test]
async fn test_estimate_cost() {
    let router = LLMRouter::new(RouterConfig::default());

    let estimate = router
        .estimate_cost("claude", "claude-3-5-sonnet", 1000, 500)
        .await;
    assert!((estimate - 0.0105).abs() < 0.0001);

    let estimate = router.estimate_cost("openai", "gpt-4o", 1000, 500).await;
    assert!((estimate - 0.0075).abs() < 0.0001);
}

// ========================================================================
// Health Summary Tests
// ========================================================================

#[tokio::test]
async fn test_health_summary() {
    let mut router = LLMRouter::new(RouterConfig::default());

    let provider1 = create_mock_provider("provider1");
    let provider2 = create_mock_provider("provider2");
    let provider3 = create_mock_provider("provider3");

    router.add_provider(provider1).await;
    router.add_provider(provider2).await;
    router.add_provider(provider3).await;

    for _ in 0..3 {
        router
            .health_tracker
            .write()
            .await
            .record_failure("provider3", "error");
    }

    let summary = router.get_health_summary().await;
    assert_eq!(summary.total_providers, 3);
    assert_eq!(summary.healthy_providers, 2);
    assert_eq!(summary.unhealthy_providers, 1);
}

// ========================================================================
// Circuit Reset Tests
// ========================================================================

#[tokio::test]
async fn test_reset_circuit() {
    let mut router = LLMRouter::new(RouterConfig::default());

    let provider = create_mock_provider("test");
    router.add_provider(provider).await;

    for _ in 0..3 {
        router
            .health_tracker
            .write()
            .await
            .record_failure("test", "error");
    }

    let state = router.get_circuit_state("test").await;
    assert_eq!(state, Some(CircuitState::Open));

    router.reset_circuit("test").await;

    let state = router.get_circuit_state("test").await;
    assert_eq!(state, Some(CircuitState::Closed));
}

// ========================================================================
// No Providers Available Tests
// ========================================================================

#[tokio::test]
async fn test_no_providers_error() {
    let router = LLMRouter::new(RouterConfig::default());

    let request = create_test_request();
    let result = router.chat(request).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        LLMError::NoProvidersAvailable => {}
        e => panic!("Expected NoProvidersAvailable, got {:?}", e),
    }
}

#[tokio::test]
async fn test_all_providers_unavailable() {
    let mut router = LLMRouter::new(RouterConfig::default());

    let provider1 = create_mock_provider("provider1");
    let provider2 = create_mock_provider("provider2");

    router.add_provider(provider1).await;
    router.add_provider(provider2).await;

    for _ in 0..3 {
        router
            .health_tracker
            .write()
            .await
            .record_failure("provider1", "error");
        router
            .health_tracker
            .write()
            .await
            .record_failure("provider2", "error");
    }

    let request = create_test_request();
    let result = router.chat(request).await;

    assert!(result.is_err());
}
