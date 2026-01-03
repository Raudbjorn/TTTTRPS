//! CDP client for communicating with Claude Desktop.

use std::time::Duration;

use chromiumoxide::{Browser, Page};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, instrument, warn};

use super::config::ClaudeConfig;
use super::error::{ClaudeCdpError, Result};

/// A message sent to or received from Claude.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The role: "user" or "assistant".
    pub role: String,
    /// The message content.
    pub content: String,
    /// Timestamp when the message was created/received.
    pub timestamp: Option<String>,
}

impl Message {
    /// Create a new user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
            timestamp: Some(Self::now()),
        }
    }

    /// Create a new assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            timestamp: Some(Self::now()),
        }
    }

    fn now() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        format!("{}000", duration.as_secs())
    }
}

/// Connection state for the Claude Desktop bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    /// Not connected to Claude Desktop.
    Disconnected,
    /// Currently connecting.
    Connecting,
    /// Successfully connected.
    Connected,
    /// Connection failed.
    Failed,
}

/// The Claude Desktop CDP bridge client.
pub struct ClaudeClient {
    config: ClaudeConfig,
    browser: Option<Browser>,
    page: Option<Page>,
    state: ConnectionState,
}

impl ClaudeClient {
    /// Create a new client with default configuration.
    pub fn new() -> Self {
        Self::with_config(ClaudeConfig::default())
    }

    /// Create a new client with custom configuration.
    pub fn with_config(config: ClaudeConfig) -> Self {
        Self {
            config,
            browser: None,
            page: None,
            state: ConnectionState::Disconnected,
        }
    }

    /// Get the current connection state.
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Check if connected to Claude Desktop.
    pub fn is_connected(&self) -> bool {
        self.state == ConnectionState::Connected && self.page.is_some()
    }

    /// Get the configured port.
    pub fn port(&self) -> u16 {
        self.config.port
    }

    /// Connect to Claude Desktop via CDP.
    #[instrument(skip(self), fields(port = self.config.port))]
    pub async fn connect(&mut self) -> Result<()> {
        if self.is_connected() {
            debug!("already connected to Claude Desktop");
            return Ok(());
        }

        self.state = ConnectionState::Connecting;
        info!(
            port = self.config.port,
            "connecting to Claude Desktop via CDP"
        );

        let ws_url = self.config.ws_url();

        let (browser, mut handler) = Browser::connect(&ws_url).await.map_err(|e| {
            self.state = ConnectionState::Failed;
            ClaudeCdpError::ConnectionFailed {
                url: ws_url.clone(),
                source: Box::new(e),
            }
        })?;

        // Spawn the CDP handler task
        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if let Err(e) = event {
                    error!("CDP event handler error: {}", e);
                    break;
                }
            }
        });

        // Find the Claude conversation page
        let page = self.find_claude_page(&browser).await?;

        self.browser = Some(browser);
        self.page = Some(page);
        self.state = ConnectionState::Connected;

        info!("successfully connected to Claude Desktop");
        Ok(())
    }

    /// Find the Claude conversation page in the browser.
    async fn find_claude_page(&self, browser: &Browser) -> Result<Page> {
        let mut pages = browser.pages().await.map_err(|e| {
            ClaudeCdpError::ProtocolError(format!("failed to list pages: {e}"))
        })?;

        // Look for a page with Claude in the URL or title
        for page in pages.iter_mut() {
            if let Ok(url) = page.url().await {
                let url_str = url.map(|u| u.to_string()).unwrap_or_default();
                debug!(url = %url_str, "checking page");

                // Claude Desktop URLs typically contain "claude" or "anthropic"
                if url_str.contains("claude") || url_str.contains("anthropic") {
                    info!(url = %url_str, "found Claude page");
                    return Ok(page.clone());
                }
            }
        }

        // If no specific Claude page found, check if there are any pages at all
        if pages.is_empty() {
            return Err(ClaudeCdpError::NoClaudePageFound);
        }

        // Log what we found for debugging
        warn!(
            page_count = pages.len(),
            "no explicit Claude page found - this may not be Claude Desktop"
        );

        Err(ClaudeCdpError::NoClaudePageFound)
    }

    /// Disconnect from Claude Desktop.
    pub async fn disconnect(&mut self) {
        if let Some(browser) = self.browser.take() {
            drop(browser);
        }
        self.page = None;
        self.state = ConnectionState::Disconnected;
        info!("disconnected from Claude Desktop");
    }

    /// Send a message to Claude and wait for the response.
    #[instrument(skip(self, message), fields(message_len = message.len()))]
    pub async fn send_message(&self, message: &str) -> Result<String> {
        let page = self.page.as_ref().ok_or(ClaudeCdpError::NotConnected)?;

        info!("sending message to Claude");

        // Get the current message count before sending
        let initial_count = self.get_assistant_message_count(page).await?;
        debug!(initial_count, "current assistant message count");

        // Find and fill the input element
        self.fill_input(page, message).await?;

        // Submit the message
        self.submit_message(page).await?;

        // Wait for Claude's response
        let response = self.wait_for_response(page, initial_count).await?;

        info!(
            response_len = response.len(),
            "received response from Claude"
        );
        Ok(response)
    }

    /// Get the current count of assistant messages on the page.
    async fn get_assistant_message_count(&self, page: &Page) -> Result<usize> {
        let js = r#"
            (() => {
                const messages = document.querySelectorAll('[data-is-streaming], .font-claude-message, [class*="claude"], [class*="assistant"]');
                return messages.length;
            })()
        "#;

        let result = page.evaluate(js).await.map_err(|e| {
            ClaudeCdpError::JsExecutionFailed {
                script_hint: "get message count".to_string(),
                error: e.to_string(),
            }
        })?;

        let count: usize = result.into_value().unwrap_or(0);
        Ok(count)
    }

    /// Fill the message input with the given text.
    async fn fill_input(&self, page: &Page, message: &str) -> Result<()> {
        // Escape the message for JavaScript
        let escaped = message
            .replace('\\', "\\\\")
            .replace('`', "\\`")
            .replace('$', "\\$");

        // Try multiple selectors that Claude Desktop might use
        let js = format!(
            r#"
            (() => {{
                const selectors = [
                    '[contenteditable="true"]',
                    'div[contenteditable]',
                    'textarea',
                    '[data-placeholder]',
                    '.ProseMirror',
                    '[role="textbox"]'
                ];

                for (const selector of selectors) {{
                    const input = document.querySelector(selector);
                    if (input) {{
                        input.focus();

                        if (input.contentEditable === 'true') {{
                            input.innerHTML = '';
                            input.textContent = `{escaped}`;
                            input.dispatchEvent(new InputEvent('input', {{ bubbles: true }}));
                            return {{ success: true, selector }};
                        }}

                        if (input.tagName === 'TEXTAREA') {{
                            input.value = `{escaped}`;
                            input.dispatchEvent(new InputEvent('input', {{ bubbles: true }}));
                            return {{ success: true, selector }};
                        }}
                    }}
                }}

                return {{ success: false, error: 'no input element found' }};
            }})()
            "#
        );

        let result = page.evaluate(js.as_str()).await.map_err(|e| {
            ClaudeCdpError::JsExecutionFailed {
                script_hint: "fill input".to_string(),
                error: e.to_string(),
            }
        })?;

        let value: serde_json::Value = result.into_value().unwrap_or(serde_json::Value::Null);

        if value.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            let selector = value.get("selector").and_then(|v| v.as_str()).unwrap_or("unknown");
            debug!(selector, "filled input element");
            Ok(())
        } else {
            let error = value.get("error").and_then(|v| v.as_str()).unwrap_or("unknown error");
            Err(ClaudeCdpError::InputElementNotFound {
                details: error.to_string(),
            })
        }
    }

    /// Submit the message (press Enter or click send button).
    async fn submit_message(&self, page: &Page) -> Result<()> {
        let js = r#"
            (() => {
                const buttonSelectors = [
                    'button[type="submit"]',
                    'button[aria-label*="send" i]',
                    'button[aria-label*="Send" i]',
                    '[data-testid="send-button"]',
                    'button svg[class*="send"]'
                ];

                for (const selector of buttonSelectors) {
                    const btn = document.querySelector(selector);
                    if (btn) {
                        const button = btn.closest('button') || btn;
                        if (!button.disabled) {
                            button.click();
                            return { success: true, method: 'button', selector };
                        }
                    }
                }

                const input = document.querySelector('[contenteditable="true"], textarea');
                if (input) {
                    const enterEvent = new KeyboardEvent('keydown', {
                        key: 'Enter',
                        code: 'Enter',
                        keyCode: 13,
                        which: 13,
                        bubbles: true
                    });
                    input.dispatchEvent(enterEvent);
                    return { success: true, method: 'enter' };
                }

                return { success: false, error: 'no submit method found' };
            })()
        "#;

        let result = page.evaluate(js).await.map_err(|e| {
            ClaudeCdpError::JsExecutionFailed {
                script_hint: "submit message".to_string(),
                error: e.to_string(),
            }
        })?;

        let value: serde_json::Value = result.into_value().unwrap_or(serde_json::Value::Null);

        if value.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            let method = value.get("method").and_then(|v| v.as_str()).unwrap_or("unknown");
            debug!(method, "submitted message");
            Ok(())
        } else {
            let error = value.get("error").and_then(|v| v.as_str()).unwrap_or("unknown error");
            Err(ClaudeCdpError::SendFailed {
                reason: error.to_string(),
            })
        }
    }

    /// Wait for Claude's response after sending a message.
    async fn wait_for_response(&self, page: &Page, initial_count: usize) -> Result<String> {
        let timeout_duration = Duration::from_secs(self.config.response_timeout_secs);
        let poll_interval = Duration::from_millis(self.config.poll_interval_ms);

        let result = timeout(timeout_duration, async {
            loop {
                let js = format!(
                    r#"
                    (() => {{
                        const streaming = document.querySelector('[data-is-streaming="true"]');
                        if (streaming) {{
                            return {{ done: false, streaming: true }};
                        }}

                        const messages = document.querySelectorAll('.font-claude-message, [class*="assistant-message"], [data-message-author="assistant"]');

                        if (messages.length > {initial_count}) {{
                            const lastMsg = messages[messages.length - 1];
                            const text = lastMsg.textContent || lastMsg.innerText || '';
                            return {{ done: true, text: text.trim() }};
                        }}

                        const allContent = document.querySelectorAll('[class*="prose"], [class*="markdown"]');
                        if (allContent.length > {initial_count}) {{
                            const lastContent = allContent[allContent.length - 1];
                            const text = lastContent.textContent || lastContent.innerText || '';
                            return {{ done: true, text: text.trim() }};
                        }}

                        return {{ done: false, streaming: false }};
                    }})()
                    "#
                );

                let result = page.evaluate(js.as_str()).await.map_err(|e| {
                    ClaudeCdpError::JsExecutionFailed {
                        script_hint: "check response".to_string(),
                        error: e.to_string(),
                    }
                })?;

                let value: serde_json::Value = result.into_value().unwrap_or(serde_json::Value::Null);

                let done = value.get("done").and_then(|v| v.as_bool()).unwrap_or(false);
                let streaming = value.get("streaming").and_then(|v| v.as_bool()).unwrap_or(false);

                if done {
                    let text = value
                        .get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    return Ok(text);
                }

                if streaming {
                    debug!("Claude is still streaming response...");
                }

                sleep(poll_interval).await;
            }
        })
        .await;

        match result {
            Ok(inner) => inner,
            Err(_) => Err(ClaudeCdpError::ResponseTimeout {
                seconds: self.config.response_timeout_secs,
            }),
        }
    }

    /// Start a new conversation in Claude Desktop.
    #[instrument(skip(self))]
    pub async fn new_conversation(&self) -> Result<()> {
        let page = self.page.as_ref().ok_or(ClaudeCdpError::NotConnected)?;

        let js = r#"
            (() => {
                const buttonSelectors = [
                    'button[aria-label*="new" i]',
                    'button[aria-label*="New" i]',
                    '[data-testid="new-chat"]',
                    'a[href="/new"]',
                    'button:has(svg[class*="plus"])'
                ];

                for (const selector of buttonSelectors) {
                    const btn = document.querySelector(selector);
                    if (btn) {
                        btn.click();
                        return { success: true, selector };
                    }
                }

                document.dispatchEvent(new KeyboardEvent('keydown', {
                    key: 'n',
                    code: 'KeyN',
                    metaKey: true,
                    ctrlKey: true,
                    bubbles: true
                }));

                return { success: true, method: 'keyboard' };
            })()
        "#;

        page.evaluate(js).await.map_err(|e| {
            ClaudeCdpError::JsExecutionFailed {
                script_hint: "new conversation".to_string(),
                error: e.to_string(),
            }
        })?;

        // Wait a moment for the UI to update
        sleep(Duration::from_millis(500)).await;

        info!("started new conversation");
        Ok(())
    }

    /// Get the current conversation history from the page.
    #[instrument(skip(self))]
    pub async fn get_conversation(&self) -> Result<Vec<Message>> {
        let page = self.page.as_ref().ok_or(ClaudeCdpError::NotConnected)?;

        let js = r#"
            (() => {
                const messages = [];

                const containers = document.querySelectorAll('[data-message-author], [class*="message-"]');

                containers.forEach(container => {
                    const isAssistant = container.getAttribute('data-message-author') === 'assistant'
                        || container.classList.contains('font-claude-message')
                        || container.querySelector('.font-claude-message');

                    const text = container.textContent || container.innerText || '';

                    if (text.trim()) {
                        messages.push({
                            role: isAssistant ? 'assistant' : 'user',
                            content: text.trim()
                        });
                    }
                });

                return messages;
            })()
        "#;

        let result = page.evaluate(js).await.map_err(|e| {
            ClaudeCdpError::JsExecutionFailed {
                script_hint: "get conversation".to_string(),
                error: e.to_string(),
            }
        })?;

        let messages: Vec<Message> = result.into_value().unwrap_or_default();

        debug!(message_count = messages.len(), "retrieved conversation history");
        Ok(messages)
    }
}

impl Default for ClaudeClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let client = ClaudeClient::new();
        assert_eq!(client.config.port, 9333);
        assert_eq!(client.config.host, "127.0.0.1");
    }

    #[test]
    fn test_message_creation() {
        let user_msg = Message::user("Hello");
        assert_eq!(user_msg.role, "user");
        assert_eq!(user_msg.content, "Hello");

        let assistant_msg = Message::assistant("Hi there!");
        assert_eq!(assistant_msg.role, "assistant");
    }

    #[test]
    fn test_connection_state() {
        let client = ClaudeClient::new();
        assert_eq!(client.state(), ConnectionState::Disconnected);
        assert!(!client.is_connected());
    }
}
