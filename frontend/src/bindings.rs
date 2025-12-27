//! Tauri IPC Bindings
//!
//! Wrapper functions for calling Tauri commands from the frontend.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// ============================================================================
// Tauri Invoke
// ============================================================================

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = "invoke")]
    async fn invoke_raw(cmd: &str, args: JsValue) -> JsValue;
}

/// Invoke a Tauri command with typed arguments and response
pub async fn invoke<A: Serialize, R: for<'de> Deserialize<'de>>(
    cmd: &str,
    args: &A,
) -> Result<R, String> {
    let args_js = serde_wasm_bindgen::to_value(args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = invoke_raw(cmd, args_js).await;

    // Check if result is an error
    if result.is_undefined() || result.is_null() {
        return Err("Command returned null/undefined".to_string());
    }

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

/// Invoke a Tauri command with no arguments
pub async fn invoke_no_args<R: for<'de> Deserialize<'de>>(cmd: &str) -> Result<R, String> {
    #[derive(Serialize)]
    struct Empty {}
    invoke(cmd, &Empty {}).await
}

// ============================================================================
// Request/Response Types (match backend commands.rs)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequestPayload {
    pub message: String,
    pub system_prompt: Option<String>,
    pub context: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponsePayload {
    pub content: String,
    pub model: String,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMSettings {
    pub provider: String,
    pub api_key: Option<String>,
    pub host: Option<String>,
    pub model: String,
    pub embedding_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub provider: String,
    pub healthy: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    pub page_count: usize,
    pub character_count: usize,
    pub source_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub version: String,
}

// ============================================================================
// LLM Commands
// ============================================================================

pub async fn configure_llm(settings: LLMSettings) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        settings: LLMSettings,
    }
    invoke("configure_llm", &Args { settings }).await
}

pub async fn chat(payload: ChatRequestPayload) -> Result<ChatResponsePayload, String> {
    #[derive(Serialize)]
    struct Args {
        payload: ChatRequestPayload,
    }
    invoke("chat", &Args { payload }).await
}

pub async fn check_llm_health() -> Result<HealthStatus, String> {
    invoke_no_args("check_llm_health").await
}

pub async fn get_llm_config() -> Result<Option<LLMSettings>, String> {
    invoke_no_args("get_llm_config").await
}

// ============================================================================
// Credential Commands
// ============================================================================

pub async fn save_api_key(provider: String, api_key: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        provider: String,
        api_key: String,
    }
    invoke("save_api_key", &Args { provider, api_key }).await
}

pub async fn get_api_key(provider: String) -> Result<Option<String>, String> {
    #[derive(Serialize)]
    struct Args {
        provider: String,
    }
    invoke("get_api_key", &Args { provider }).await
}

pub async fn list_stored_providers() -> Result<Vec<String>, String> {
    invoke_no_args("list_stored_providers").await
}

// ============================================================================
// Document Commands
// ============================================================================

pub async fn ingest_pdf(path: String) -> Result<IngestResult, String> {
    #[derive(Serialize)]
    struct Args {
        path: String,
    }
    invoke("ingest_pdf", &Args { path }).await
}

// ============================================================================
// Utility Commands
// ============================================================================

pub async fn get_app_version() -> Result<String, String> {
    invoke_no_args("get_app_version").await
}

pub async fn get_system_info() -> Result<SystemInfo, String> {
    invoke_no_args("get_system_info").await
}
