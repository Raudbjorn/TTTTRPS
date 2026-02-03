//! Shared streaming chat utilities
//!
//! Common logic for handling streaming chat responses across components.
//! Used by Chat, NpcConversation, and SessionChatPanel.

use crate::bindings::{listen_chat_chunks_async, ChatChunk};
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// Trait for messages that support streaming updates
pub trait StreamingMessage {
    fn stream_id(&self) -> Option<&String>;
    fn is_streaming(&self) -> bool;
    fn append_content(&mut self, content: &str);
    fn finalize(&mut self, finish_reason: Option<&str>);
}

/// Callback type for when a stream completes
pub type OnStreamComplete = Box<dyn Fn(String, String) + 'static>;

/// Setup a streaming chunk listener that processes chunks into a messages signal.
///
/// # Important: Unlisten Handle Behavior
///
/// The returned `_unlisten` JsValue is intentionally not stored because:
///
/// 1. **Tauri's event system keeps the callback alive**: The callback passed to
///    `listen_event` is captured by a JavaScript closure that persists in the
///    Tauri event system. Dropping the unlisten handle does NOT unregister the
///    callback - it only removes your ability to explicitly unregister it.
///
/// 2. **JsValue is !Send**: Cannot be stored in Leptos signals (which require
///    Send+Sync) or used with `on_cleanup` closures.
///
/// 3. **Stream ID filtering prevents interference**: Each stream has a unique ID,
///    so listeners from different component instances won't process each other's
///    chunks.
///
/// 4. **Automatic cleanup**: Tauri cleans up all event listeners when the
///    webview closes.
///
/// 5. **Graceful degradation**: `try_update`/`try_set` return `None` when
///    signals are disposed, preventing crashes if chunks arrive after unmount.
///
/// # Arguments
///
/// * `messages` - RwSignal containing the message list
/// * `find_and_update` - Closure that finds and updates a streaming message
/// * `on_complete` - Optional callback when stream completes (receives stream_id, final_content)
pub fn setup_stream_listener<T, F>(messages: RwSignal<Vec<T>>, find_and_update: F)
where
    T: Clone + Send + Sync + 'static,
    F: Fn(&mut Vec<T>, &ChatChunk) -> Option<String> + Clone + 'static,
{
    spawn_local(async move {
        // Note: _unlisten is intentionally not stored - see doc comment above
        let _unlisten = listen_chat_chunks_async(move |chunk: ChatChunk| {
            let find_and_update = find_and_update.clone();

            // Try to update messages - returns None if signal is disposed
            let _result = messages.try_update(|msgs| find_and_update(msgs, &chunk));
        })
        .await;
    });
}

/// Common chunk processing logic for streaming messages.
/// Returns Some(final_content) if the stream completed, None otherwise.
pub fn process_chat_chunk<T>(
    msgs: &mut Vec<T>,
    chunk: &ChatChunk,
    get_stream_id: impl Fn(&T) -> Option<&String>,
    is_streaming: impl Fn(&T) -> bool,
    append_content: impl Fn(&mut T, &str),
    finalize: impl Fn(&mut T, Option<&str>),
    get_content: impl Fn(&T) -> &str,
) -> Option<String> {
    // Find the message matching this stream
    if let Some(msg) = msgs
        .iter_mut()
        .find(|m| get_stream_id(m) == Some(&chunk.stream_id) && is_streaming(m))
    {
        // Append content
        if !chunk.content.is_empty() {
            append_content(msg, &chunk.content);
        }

        // Handle stream completion
        if chunk.is_final {
            finalize(msg, chunk.finish_reason.as_deref());
            return Some(get_content(msg).to_string());
        }
    }
    None
}
