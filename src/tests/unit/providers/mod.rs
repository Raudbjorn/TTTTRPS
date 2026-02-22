//! LLM Provider Unit Tests
//!
//! Comprehensive unit tests for all LLM provider implementations.
//! Uses wiremock for HTTP mocking to test:
//! - API request formatting
//! - Response parsing (success and error cases)
//! - Model switching
//! - Streaming response handling
//! - Rate limit error handling
//! - Timeout handling
//! - Invalid API key handling

mod claude_tests;
mod openai_tests;
mod google_tests;   // API key-based Google provider
// mod gemini_tests; // OAuth-based Gemini provider (has its own tests in gemini.rs)
mod ollama_tests;
