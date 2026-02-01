//! Test Validators
//!
//! Provides InputValidator and ApiKeyValidator implementations for security testing.
//! These are extracted from the security tests to be reusable across test modules.

use std::path::Path;

// =============================================================================
// Validation Types
// =============================================================================

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub sanitized: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    TooLong { max: usize },
    Empty,
    InvalidCharacters(String),
    XssDetected,
    PathTraversal,
    SqlInjection,
    CommandInjection,
    InvalidExtension(String),
    NullByte,
    InvalidUtf8,
}

// =============================================================================
// Input Validator
// =============================================================================

/// Comprehensive input validator for security testing.
/// Validates against XSS, SQL injection, path traversal, and command injection.
pub struct InputValidator {
    pub max_string_length: usize,
    pub max_path_depth: usize,
    pub allowed_extensions: Vec<String>,
    pub max_file_size: u64,
}

impl InputValidator {
    pub fn new() -> Self {
        Self {
            max_string_length: 100_000,
            max_path_depth: 10,
            allowed_extensions: vec![
                "pdf".to_string(),
                "epub".to_string(),
                "txt".to_string(),
                "md".to_string(),
                "json".to_string(),
            ],
            max_file_size: 50 * 1024 * 1024, // 50 MB
        }
    }

    pub fn with_max_length(mut self, max: usize) -> Self {
        self.max_string_length = max;
        self
    }

    pub fn with_max_file_size(mut self, size: u64) -> Self {
        self.max_file_size = size;
        self
    }

    // -------------------------------------------------------------------------
    // Text Validation
    // -------------------------------------------------------------------------

    pub fn validate_text(&self, input: &str) -> Result<ValidationResult, ValidationError> {
        if input.contains('\0') {
            return Err(ValidationError::NullByte);
        }

        if input.len() > self.max_string_length {
            return Err(ValidationError::TooLong {
                max: self.max_string_length,
            });
        }

        if self.detect_xss(input) {
            return Err(ValidationError::XssDetected);
        }

        let sanitized = self.sanitize_html(input);
        let mut warnings = Vec::new();

        if sanitized != input {
            warnings.push("Input was sanitized".to_string());
        }

        Ok(ValidationResult {
            is_valid: true,
            sanitized,
            warnings,
        })
    }

    // -------------------------------------------------------------------------
    // Path Validation
    // -------------------------------------------------------------------------

    pub fn validate_path(&self, path: &str) -> Result<ValidationResult, ValidationError> {
        if path.contains('\0') {
            return Err(ValidationError::NullByte);
        }

        if self.detect_path_traversal(path) {
            return Err(ValidationError::PathTraversal);
        }

        let mut warnings = Vec::new();
        let depth = path.matches('/').count() + path.matches('\\').count();
        if depth > self.max_path_depth {
            warnings.push(format!("Path depth ({}) is unusually deep", depth));
        }

        let sanitized = path.replace("..", "").replace('~', "");

        Ok(ValidationResult {
            is_valid: true,
            sanitized,
            warnings,
        })
    }

    pub fn validate_file_path(&self, path: &str) -> Result<ValidationResult, ValidationError> {
        let base_result = self.validate_path(path)?;

        let path_obj = Path::new(path);
        if let Some(ext) = path_obj.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if !self.allowed_extensions.contains(&ext_str) {
                return Err(ValidationError::InvalidExtension(ext_str));
            }
        }

        Ok(base_result)
    }

    pub fn validate_file_size(&self, size: u64) -> Result<(), ValidationError> {
        if size > self.max_file_size {
            Err(ValidationError::TooLong {
                max: self.max_file_size as usize,
            })
        } else {
            Ok(())
        }
    }

    // -------------------------------------------------------------------------
    // Query/Command Validation
    // -------------------------------------------------------------------------

    pub fn validate_query_param(&self, input: &str) -> Result<ValidationResult, ValidationError> {
        if self.detect_sql_injection(input) {
            return Err(ValidationError::SqlInjection);
        }
        self.validate_text(input)
    }

    pub fn validate_command_param(&self, input: &str) -> Result<ValidationResult, ValidationError> {
        if self.detect_command_injection(input) {
            return Err(ValidationError::CommandInjection);
        }
        self.validate_text(input)
    }

    // -------------------------------------------------------------------------
    // XSS Detection
    // -------------------------------------------------------------------------

    pub fn detect_xss(&self, input: &str) -> bool {
        let lower = input.to_lowercase();

        // Script tags
        let script_patterns = ["<script", "</script>", "<script>"];
        for pattern in script_patterns {
            if lower.contains(pattern) {
                return true;
            }
        }

        // Event handlers
        let event_handlers = [
            "onerror=",
            "onload=",
            "onclick=",
            "onmouseover=",
            "onfocus=",
            "onblur=",
            "onsubmit=",
            "onchange=",
            "onkeyup=",
            "onkeydown=",
            "onmouseout=",
            "ondblclick=",
            "oncontextmenu=",
            "oninput=",
        ];
        for handler in event_handlers {
            if lower.contains(handler) {
                return true;
            }
        }

        // JavaScript URLs
        let js_patterns = ["javascript:", "vbscript:", "data:text/html"];
        for pattern in js_patterns {
            if lower.contains(pattern) {
                return true;
            }
        }

        // Dangerous functions
        let dangerous_funcs = [
            "eval(",
            "document.cookie",
            "document.write",
            "innerhtml",
            "outerhtml",
            "fromcharcode",
            "string.fromcharcode",
        ];
        for func in dangerous_funcs {
            if lower.contains(func) {
                return true;
            }
        }

        // Dangerous elements
        let dangerous_elements = ["<iframe", "<object", "<embed", "<form", "<svg", "<math"];
        for elem in dangerous_elements {
            if lower.contains(elem) {
                return true;
            }
        }

        // Check URL-encoded variants
        let url_decoded = self.url_decode(&lower);
        if url_decoded != lower {
            for pattern in script_patterns {
                if url_decoded.contains(pattern) {
                    return true;
                }
            }
            for handler in event_handlers {
                if url_decoded.contains(handler) {
                    return true;
                }
            }
            for pattern in js_patterns {
                if url_decoded.contains(pattern) {
                    return true;
                }
            }
        }

        // Check HTML entity encoded variants
        let html_decoded = self.html_entity_decode(&lower);
        if html_decoded != lower {
            for pattern in script_patterns {
                if html_decoded.contains(pattern) {
                    return true;
                }
            }
            for handler in event_handlers {
                if html_decoded.contains(handler) {
                    return true;
                }
            }
            for pattern in js_patterns {
                if html_decoded.contains(pattern) {
                    return true;
                }
            }
        }

        false
    }

    // -------------------------------------------------------------------------
    // Path Traversal Detection
    // -------------------------------------------------------------------------

    pub fn detect_path_traversal(&self, input: &str) -> bool {
        let patterns = [
            "..", "..\\", "../", "..%2f", "..%5c", "%2e%2e", "....//", "....\\\\", ".../", "...\\",
        ];

        let lower = input.to_lowercase();
        for pattern in patterns {
            if lower.contains(pattern) {
                return true;
            }
        }

        let sensitive_paths = [
            "/etc/",
            "/root/",
            "/var/",
            "/usr/",
            "c:\\windows",
            "c:\\system32",
        ];
        for path in sensitive_paths {
            if lower.starts_with(path) {
                return true;
            }
        }

        false
    }

    // -------------------------------------------------------------------------
    // SQL Injection Detection
    // -------------------------------------------------------------------------

    pub fn detect_sql_injection(&self, input: &str) -> bool {
        let lower = input.to_lowercase();

        // Single/double quote attacks
        let quote_patterns = [
            "' or ", "\" or ", "' and ", "\" and ", "'='", "\"=\"", "' --", "\" --", "';", "\";",
        ];
        for pattern in quote_patterns {
            if lower.contains(pattern) {
                return true;
            }
        }

        // UNION attacks
        let union_patterns = ["union select", "union all select", "union distinct select"];
        for pattern in union_patterns {
            if lower.contains(pattern) {
                return true;
            }
        }

        // Comment attacks
        let comment_patterns = ["/*", "*/", "-- "];
        for pattern in comment_patterns {
            if lower.contains(pattern) {
                return true;
            }
        }

        // MySQL comment
        if lower.contains(" # ")
            || lower.ends_with(" #")
            || lower.contains("\n#")
            || lower.starts_with('#')
            || lower.contains("'#")
            || lower.contains("\"#")
        {
            return true;
        }

        // Stacked queries
        let stacked_patterns = [
            "; drop",
            "; delete",
            "; insert",
            "; update",
            "; select",
            "; create",
            "; alter",
            "; truncate",
        ];
        for pattern in stacked_patterns {
            if lower.contains(pattern) {
                return true;
            }
        }

        // Other dangerous patterns
        let other_patterns = [
            "1=1",
            "1 = 1",
            "xp_",
            "sp_",
            "@@",
            "char(",
            "nchar(",
            "varchar(",
            "exec(",
            "execute(",
            "waitfor",
            "benchmark(",
            "sleep(",
        ];
        for pattern in other_patterns {
            if lower.contains(pattern) {
                return true;
            }
        }

        false
    }

    // -------------------------------------------------------------------------
    // Command Injection Detection
    // -------------------------------------------------------------------------

    pub fn detect_command_injection(&self, input: &str) -> bool {
        let patterns = [
            ";", "&&", "||", "|", "`", "$(", "$()", "${", "\n", "\r", ">", "<", ">>", "<<",
        ];

        for pattern in patterns {
            if input.contains(pattern) {
                return true;
            }
        }

        false
    }

    // -------------------------------------------------------------------------
    // Encoding/Decoding Helpers
    // -------------------------------------------------------------------------

    pub fn url_decode(&self, input: &str) -> String {
        let mut result = input.to_string();

        let url_replacements = [
            ("%3c", "<"),
            ("%3e", ">"),
            ("%22", "\""),
            ("%27", "'"),
            ("%2f", "/"),
            ("%5c", "\\"),
            ("%2e", "."),
            ("%00", "\0"),
            ("%20", " "),
            ("%3d", "="),
            ("%26", "&"),
            ("%3b", ";"),
            ("%3a", ":"),
            ("%28", "("),
            ("%29", ")"),
        ];

        for (encoded, decoded) in url_replacements {
            result = result.replace(encoded, decoded);
        }

        result
    }

    pub fn html_entity_decode(&self, input: &str) -> String {
        let mut result = input.to_string();

        // Named entities
        let named_entities = [
            ("&lt;", "<"),
            ("&gt;", ">"),
            ("&quot;", "\""),
            ("&apos;", "'"),
            ("&amp;", "&"),
        ];
        for (entity, decoded) in named_entities {
            result = result.replace(entity, decoded);
        }

        // Decimal numeric entities
        let decimal_entities = [
            ("&#60;", "<"),
            ("&#62;", ">"),
            ("&#34;", "\""),
            ("&#39;", "'"),
            ("&#38;", "&"),
            ("&#58;", ":"),
            ("&#40;", "("),
            ("&#41;", ")"),
            ("&#47;", "/"),
            ("&#92;", "\\"),
        ];
        for (entity, decoded) in decimal_entities {
            result = result.replace(entity, decoded);
        }

        // Hex numeric entities
        let hex_entities = [
            ("&#x3c;", "<"),
            ("&#x3C;", "<"),
            ("&#x3e;", ">"),
            ("&#x3E;", ">"),
            ("&#x22;", "\""),
            ("&#x27;", "'"),
            ("&#x26;", "&"),
            ("&#x3a;", ":"),
            ("&#x3A;", ":"),
        ];
        for (entity, decoded) in hex_entities {
            result = result.replace(entity, decoded);
        }

        result
    }

    pub fn sanitize_html(&self, input: &str) -> String {
        input
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }
}

impl Default for InputValidator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// API Key Validator
// =============================================================================

/// Validator for API key formats by provider.
pub struct ApiKeyValidator;

impl ApiKeyValidator {
    pub fn validate(provider: &str, key: &str) -> Result<(), String> {
        if key.is_empty() {
            return Err("API key cannot be empty".to_string());
        }

        match provider.to_lowercase().as_str() {
            "openai" => {
                if !key.starts_with("sk-") || key.len() < 20 {
                    return Err("Invalid OpenAI API key format".to_string());
                }
            }
            "claude" | "anthropic" => {
                if !key.starts_with("sk-ant-") || key.len() < 20 {
                    return Err("Invalid Anthropic API key format".to_string());
                }
            }
            "gemini" | "google" => {
                if !key.starts_with("AIza") || key.len() < 20 {
                    return Err("Invalid Google API key format".to_string());
                }
            }
            "elevenlabs" => {
                if key.len() < 20 {
                    return Err("Invalid ElevenLabs API key format".to_string());
                }
            }
            _ => {
                if key.len() < 10 {
                    return Err("API key too short".to_string());
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_validator_creation() {
        let validator = InputValidator::new();
        assert_eq!(validator.max_string_length, 100_000);
    }

    #[test]
    fn test_api_key_validator_openai() {
        assert!(ApiKeyValidator::validate("openai", "sk-test12345678901234567890").is_ok());
        assert!(ApiKeyValidator::validate("openai", "invalid").is_err());
    }
}
