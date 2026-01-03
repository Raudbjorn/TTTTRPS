//! Tauri command bindings for the frontend.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    async fn invoke_inner(cmd: &str, args: JsValue) -> JsValue;
}

/// Connection status from the backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub state: String,
    pub connected: bool,
    pub port: u16,
}

/// Command result wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum CommandResult<T> {
    #[serde(rename = "ok")]
    Ok(T),
    #[serde(rename = "error")]
    Error(String),
}

impl<T> CommandResult<T> {
    pub fn into_result(self) -> Result<T, String> {
        match self {
            CommandResult::Ok(data) => Ok(data),
            CommandResult::Error(e) => Err(e),
        }
    }
}

/// Invoke a Tauri command with arguments.
async fn invoke<T, A>(cmd: &str, args: &A) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
    A: Serialize,
{
    let args_js = serde_wasm_bindgen::to_value(args).map_err(|e| e.to_string())?;
    let result = invoke_inner(cmd, args_js).await;
    serde_wasm_bindgen::from_value(result).map_err(|e| e.to_string())
}

/// Invoke a Tauri command without arguments.
async fn invoke_no_args<T>(cmd: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let args_js = serde_wasm_bindgen::to_value(&serde_json::json!({})).map_err(|e| e.to_string())?;
    let result = invoke_inner(cmd, args_js).await;
    serde_wasm_bindgen::from_value(result).map_err(|e| e.to_string())
}

/// Connect to Claude Desktop.
pub async fn connect() -> Result<ConnectionStatus, String> {
    let result: CommandResult<ConnectionStatus> = invoke_no_args("connect").await?;
    result.into_result()
}

/// Disconnect from Claude Desktop.
pub async fn disconnect() -> Result<(), String> {
    let result: CommandResult<()> = invoke_no_args("disconnect").await?;
    result.into_result()
}

/// Get connection status.
pub async fn get_status() -> Result<ConnectionStatus, String> {
    invoke_no_args("get_status").await
}

/// Send a message to Claude.
pub async fn send_message(message: &str) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        message: &'a str,
    }

    let result: CommandResult<String> = invoke("send_message", &Args { message }).await?;
    result.into_result()
}

/// Start a new conversation.
pub async fn new_conversation() -> Result<(), String> {
    let result: CommandResult<()> = invoke_no_args("new_conversation").await?;
    result.into_result()
}

/// Update CDP configuration.
pub async fn update_config(port: Option<u16>, timeout_secs: Option<u64>) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        port: Option<u16>,
        timeout_secs: Option<u64>,
    }

    let result: CommandResult<()> = invoke("update_config", &Args { port, timeout_secs }).await?;
    result.into_result()
}
