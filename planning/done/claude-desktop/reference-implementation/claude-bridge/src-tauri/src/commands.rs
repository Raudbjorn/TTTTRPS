//! Tauri commands for the Claude CDP bridge.

use claude_cdp::{ClaudeClient, ClaudeConfig, ConnectionState, Message};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use tracing::{error, info, instrument};

/// Shared state for the Claude client.
pub struct AppState {
    pub client: Arc<Mutex<ClaudeClient>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Mutex::new(ClaudeClient::new())),
        }
    }

    pub fn with_config(config: ClaudeConfig) -> Self {
        Self {
            client: Arc::new(Mutex::new(ClaudeClient::with_config(config))),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Response type for commands that can fail.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum CommandResult<T> {
    #[serde(rename = "ok")]
    Ok(T),
    #[serde(rename = "error")]
    Error(String),
}

impl<T> From<Result<T, claude_cdp::ClaudeCdpError>> for CommandResult<T> {
    fn from(result: Result<T, claude_cdp::ClaudeCdpError>) -> Self {
        match result {
            Ok(data) => CommandResult::Ok(data),
            Err(e) => CommandResult::Error(e.to_string()),
        }
    }
}

/// Connection status response.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub state: String,
    pub connected: bool,
    pub port: u16,
}

/// Connect to Claude Desktop.
#[tauri::command]
#[instrument(skip(state))]
pub async fn connect(state: State<'_, AppState>) -> Result<CommandResult<ConnectionStatus>, ()> {
    info!("connecting to Claude Desktop");
    
    let mut client = state.client.lock().await;
    let result = client.connect().await;
    
    let status = ConnectionStatus {
        state: format!("{:?}", client.state()),
        connected: client.is_connected(),
        port: client.config().port,
    };
    
    match result {
        Ok(_) => {
            info!("successfully connected");
            Ok(CommandResult::Ok(status))
        }
        Err(e) => {
            error!(error = %e, "connection failed");
            Ok(CommandResult::Error(e.to_string()))
        }
    }
}

/// Disconnect from Claude Desktop.
#[tauri::command]
#[instrument(skip(state))]
pub async fn disconnect(state: State<'_, AppState>) -> Result<CommandResult<()>, ()> {
    info!("disconnecting from Claude Desktop");
    
    let mut client = state.client.lock().await;
    client.disconnect().await;
    
    Ok(CommandResult::Ok(()))
}

/// Get the current connection status.
#[tauri::command]
#[instrument(skip(state))]
pub async fn get_status(state: State<'_, AppState>) -> Result<ConnectionStatus, ()> {
    let client = state.client.lock().await;
    
    Ok(ConnectionStatus {
        state: format!("{:?}", client.state()),
        connected: client.is_connected(),
        port: client.config().port,
    })
}

/// Send a message to Claude and wait for response.
#[tauri::command]
#[instrument(skip(state, message), fields(message_len = message.len()))]
pub async fn send_message(
    state: State<'_, AppState>,
    message: String,
) -> Result<CommandResult<String>, ()> {
    info!("sending message to Claude");
    
    let client = state.client.lock().await;
    
    if !client.is_connected() {
        return Ok(CommandResult::Error("not connected to Claude Desktop".to_string()));
    }
    
    let result = client.send_message(&message).await;
    
    match result {
        Ok(response) => {
            info!(response_len = response.len(), "received response");
            Ok(CommandResult::Ok(response))
        }
        Err(e) => {
            error!(error = %e, "failed to send message");
            Ok(CommandResult::Error(e.to_string()))
        }
    }
}

/// Start a new conversation.
#[tauri::command]
#[instrument(skip(state))]
pub async fn new_conversation(state: State<'_, AppState>) -> Result<CommandResult<()>, ()> {
    info!("starting new conversation");
    
    let client = state.client.lock().await;
    
    if !client.is_connected() {
        return Ok(CommandResult::Error("not connected to Claude Desktop".to_string()));
    }
    
    let result = client.new_conversation().await;
    Ok(result.into())
}

/// Get the current conversation history.
#[tauri::command]
#[instrument(skip(state))]
pub async fn get_conversation(state: State<'_, AppState>) -> Result<CommandResult<Vec<Message>>, ()> {
    let client = state.client.lock().await;
    
    if !client.is_connected() {
        return Ok(CommandResult::Error("not connected to Claude Desktop".to_string()));
    }
    
    let result = client.get_conversation().await;
    Ok(result.into())
}

/// Update the CDP configuration.
#[tauri::command]
#[instrument(skip(state))]
pub async fn update_config(
    state: State<'_, AppState>,
    port: Option<u16>,
    timeout_secs: Option<u64>,
) -> Result<CommandResult<()>, ()> {
    info!(port = ?port, timeout = ?timeout_secs, "updating configuration");
    
    // For now, we need to recreate the client with new config
    // A more sophisticated implementation would allow runtime config changes
    let mut config = ClaudeConfig::default();
    
    if let Some(p) = port {
        config = config.with_port(p);
    }
    
    if let Some(t) = timeout_secs {
        config = config.with_timeout(t);
    }
    
    let mut client = state.client.lock().await;
    *client = ClaudeClient::with_config(config);
    
    Ok(CommandResult::Ok(()))
}
