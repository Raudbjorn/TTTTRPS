use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// ============================================================================
// Tauri Invoke
// ============================================================================

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = "invoke", catch)]
    async fn invoke_raw(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

    // Dialog plugin - file picker
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "dialog"], js_name = "open")]
    async fn dialog_open(options: JsValue) -> JsValue;

    // Event listener - for progress events (returns Promise<UnlistenFn>)
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = "listen")]
    fn event_listen(event: &str, handler: &js_sys::Function) -> js_sys::Promise;
}

/// Listen for Tauri events (returns unlisten function wrapped in Promise)
/// Note: In Tauri 2, listen() is async and returns a Promise
pub fn listen_event<F>(event_name: &str, callback: F) -> JsValue
where
    F: Fn(JsValue) + 'static,
{
    let closure = wasm_bindgen::closure::Closure::wrap(Box::new(callback) as Box<dyn Fn(JsValue)>);
    let promise = event_listen(event_name, closure.as_ref().unchecked_ref());
    closure.forget(); // Prevent closure from being dropped

    // The promise resolves to the unlisten function
    // We return the promise as JsValue for compatibility
    promise.into()
}

/// Invoke a Tauri command with typed arguments and response
pub async fn invoke<A: Serialize, R: for<'de> Deserialize<'de>>(
    cmd: &str,
    args: &A,
) -> Result<R, String> {
    let args_js = serde_wasm_bindgen::to_value(args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = invoke_raw(cmd, args_js).await
        .map_err(|e| {
            serde_wasm_bindgen::from_value::<String>(e)
                .unwrap_or_else(|_| "Unknown invoke error".to_string())
        })?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

/// Invoke a Tauri command with no arguments
pub async fn invoke_no_args<R: for<'de> Deserialize<'de>>(cmd: &str) -> Result<R, String> {
    #[derive(Serialize)]
    struct Empty {}
    invoke(cmd, &Empty {}).await
}

/// Invoke a Tauri command that returns void (Result<(), String>)
/// This handles the case where null/undefined is a valid success response
pub async fn invoke_void<A: Serialize>(cmd: &str, args: &A) -> Result<(), String> {
    let args_js = serde_wasm_bindgen::to_value(args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = invoke_raw(cmd, args_js).await
        .map_err(|e| {
            serde_wasm_bindgen::from_value::<String>(e)
                .unwrap_or_else(|_| "Unknown invoke error".to_string())
        })?;

    // For void commands, null/undefined means success
    // Only check for error object with __TAURI_ERROR__ or similar patterns if we needed to,
    // but the catch above handles the rejection case.
    if !result.is_null() && !result.is_undefined() {
        // Double check if it's a success value that looks like an error string (unlikely for void but safe)
        if let Ok(err_str) = serde_wasm_bindgen::from_value::<String>(result.clone()) {
            if !err_str.is_empty() {
                // This path might not be hit if backend rejects on error, but keeping for safety
                return Err(err_str);
            }
        }
    }
    Ok(())
}

/// Invoke a Tauri command with no arguments that returns void
pub async fn invoke_void_no_args(cmd: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct Empty {}
    invoke_void(cmd, &Empty {}).await
}

// ============================================================================
// File Dialog
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct FileFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenDialogOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<FileFilter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple: Option<bool>,
}

/// Open a file picker dialog
/// Returns the selected file path(s) or None if canceled
pub async fn open_file_dialog(options: OpenDialogOptions) -> Option<String> {
    let options_js = serde_wasm_bindgen::to_value(&options).ok()?;
    let result = dialog_open(options_js).await;

    if result.is_null() || result.is_undefined() {
        return None;
    }

    // Result can be a string (single file) or array (multiple files)
    // For single file mode, it returns the path directly
    serde_wasm_bindgen::from_value(result).ok()
}

/// Open a file picker for PDF documents
pub async fn pick_pdf_file() -> Option<String> {
    open_file_dialog(OpenDialogOptions {
        title: Some("Select PDF Document".to_string()),
        filters: Some(vec![
            FileFilter {
                name: "PDF Documents".to_string(),
                extensions: vec!["pdf".to_string()],
            },
            FileFilter {
                name: "All Files".to_string(),
                extensions: vec!["*".to_string()],
            },
        ]),
        default_path: None,
        directory: Some(false),
        multiple: Some(false),
    }).await
}

/// Open a file picker for any supported document type
pub async fn pick_document_file() -> Option<String> {
    open_file_dialog(OpenDialogOptions {
        title: Some("Select Document".to_string()),
        filters: Some(vec![
            FileFilter {
                name: "All Supported".to_string(),
                extensions: vec![
                    "pdf".to_string(),
                    "epub".to_string(),
                    "mobi".to_string(),
                    "azw".to_string(),
                    "azw3".to_string(),
                    "docx".to_string(),
                    "txt".to_string(),
                    "md".to_string(),
                    "markdown".to_string(),
                ],
            },
            FileFilter {
                name: "PDF".to_string(),
                extensions: vec!["pdf".to_string()],
            },
            FileFilter {
                name: "EPUB".to_string(),
                extensions: vec!["epub".to_string()],
            },
            FileFilter {
                name: "MOBI/AZW".to_string(),
                extensions: vec!["mobi".to_string(), "azw".to_string(), "azw3".to_string()],
            },
            FileFilter {
                name: "DOCX".to_string(),
                extensions: vec!["docx".to_string()],
            },
            FileFilter {
                name: "Text/Markdown".to_string(),
                extensions: vec!["txt".to_string(), "md".to_string(), "markdown".to_string()],
            },
            FileFilter {
                name: "All Files".to_string(),
                extensions: vec!["*".to_string()],
            },
        ]),
        default_path: None,
        directory: Some(false),
        multiple: Some(false),
    }).await
}

// ============================================================================
// Utility Commands
// ============================================================================

/// Open a URL in the system's default browser
///
/// Uses Tauri's shell plugin to open URLs properly on all platforms.
pub async fn open_url_in_browser(url: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        url: String,
    }
    invoke_void("open_url_in_browser", &Args { url }).await
}

/// Copy text to system clipboard
pub async fn copy_to_clipboard(text: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        text: String,
    }
    invoke_void("copy_to_clipboard", &Args { text }).await
}
