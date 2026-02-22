//! Input Validator Module
//!
//! Provides security-focused input validation for Tauri commands.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Input exceeds maximum length of {max} characters")]
    TooLong { max: usize },

    #[error("Input is empty")]
    Empty,

    #[error("Invalid characters detected: {0}")]
    InvalidCharacters(String),

    #[error("Potential XSS detected")]
    XssDetected,

    #[error("Potential path traversal detected")]
    PathTraversal,

    #[error("Potential SQL injection detected")]
    SqlInjection,

    #[error("Potential command injection detected")]
    CommandInjection,

    #[error("Invalid file extension: {0}")]
    InvalidExtension(String),

    #[error("Input contains null bytes")]
    NullByte,

    #[error("Invalid UTF-8 encoding")]
    InvalidUtf8,
}

pub type Result<T> = std::result::Result<T, ValidationError>;

// ============================================================================
// Validation Result
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub sanitized: String,
    pub warnings: Vec<String>,
}

// ============================================================================
// Input Validator
// ============================================================================

/// Security-focused input validation
pub struct InputValidator {
    /// Maximum allowed string length
    max_string_length: usize,
    /// Maximum allowed path depth
    max_path_depth: usize,
    /// Allowed file extensions
    allowed_extensions: Vec<String>,
}

impl InputValidator {
    pub fn new() -> Self {
        Self {
            max_string_length: 100_000, // 100KB
            max_path_depth: 10,
            allowed_extensions: vec![
                "pdf".to_string(),
                "epub".to_string(),
                "mobi".to_string(),
                "txt".to_string(),
                "md".to_string(),
                "json".to_string(),
            ],
        }
    }

    /// Validate and sanitize text input
    pub fn validate_text(&self, input: &str) -> Result<ValidationResult> {
        let mut warnings = Vec::new();

        // Check for null bytes
        if input.contains('\0') {
            return Err(ValidationError::NullByte);
        }

        // Check length
        if input.len() > self.max_string_length {
            return Err(ValidationError::TooLong {
                max: self.max_string_length,
            });
        }

        // Detect potential XSS
        if self.detect_xss(input) {
            return Err(ValidationError::XssDetected);
        }

        // Sanitize the input
        let sanitized = self.sanitize_html(input);

        if sanitized != input {
            warnings.push("Input was sanitized to remove potentially dangerous content".to_string());
        }

        Ok(ValidationResult {
            is_valid: true,
            sanitized,
            warnings,
        })
    }

    /// Validate a file path
    pub fn validate_path(&self, path: &str) -> Result<ValidationResult> {
        let mut warnings = Vec::new();

        // Check for null bytes
        if path.contains('\0') {
            return Err(ValidationError::NullByte);
        }

        // Check for path traversal
        if self.detect_path_traversal(path) {
            return Err(ValidationError::PathTraversal);
        }

        // Check path depth
        let depth = path.matches('/').count() + path.matches('\\').count();
        if depth > self.max_path_depth {
            warnings.push(format!("Path depth ({}) is unusually deep", depth));
        }

        // Normalize the path
        let path_obj = Path::new(path);
        let sanitized = path_obj
            .to_string_lossy()
            .replace("..", "")
            .replace("~", "");

        Ok(ValidationResult {
            is_valid: true,
            sanitized,
            warnings,
        })
    }

    /// Validate a file path with extension check
    pub fn validate_file_path(&self, path: &str) -> Result<ValidationResult> {
        let base_result = self.validate_path(path)?;

        // Check extension
        let path_obj = Path::new(path);
        if let Some(ext) = path_obj.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if !self.allowed_extensions.contains(&ext_str) {
                return Err(ValidationError::InvalidExtension(ext_str));
            }
        }

        Ok(base_result)
    }

    /// Validate a database query parameter
    pub fn validate_query_param(&self, input: &str) -> Result<ValidationResult> {
        // Check for SQL injection patterns
        if self.detect_sql_injection(input) {
            return Err(ValidationError::SqlInjection);
        }

        self.validate_text(input)
    }

    /// Validate a shell command parameter
    pub fn validate_command_param(&self, input: &str) -> Result<ValidationResult> {
        // Check for command injection patterns
        if self.detect_command_injection(input) {
            return Err(ValidationError::CommandInjection);
        }

        self.validate_text(input)
    }

    /// Detect potential XSS patterns
    fn detect_xss(&self, input: &str) -> bool {
        let lower = input.to_lowercase();

        // Common XSS patterns
        let patterns = [
            "<script",
            "javascript:",
            "onerror=",
            "onload=",
            "onclick=",
            "onmouseover=",
            "onfocus=",
            "onblur=",
            "onsubmit=",
            "eval(",
            "document.cookie",
            "document.write",
            "innerHTML",
            "outerHTML",
            "fromCharCode",
            "String.fromCharCode",
            "<iframe",
            "<object",
            "<embed",
            "<form",
            "data:text/html",
            "vbscript:",
        ];

        for pattern in patterns {
            if lower.contains(pattern) {
                return true;
            }
        }

        // Check for encoded variants
        let decoded = self.url_decode(&lower);
        if decoded != lower {
            for pattern in patterns {
                if decoded.contains(pattern) {
                    return true;
                }
            }
        }

        false
    }

    /// Detect potential path traversal
    fn detect_path_traversal(&self, input: &str) -> bool {
        let patterns = [
            "..",
            "..\\",
            "../",
            "..%2f",
            "..%5c",
            "%2e%2e",
            "....//",
            "....\\\\",
        ];

        let lower = input.to_lowercase();
        for pattern in patterns {
            if lower.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// Detect potential SQL injection
    fn detect_sql_injection(&self, input: &str) -> bool {
        let lower = input.to_lowercase();

        let patterns = [
            "' or ",
            "\" or ",
            "' and ",
            "\" and ",
            "'; drop",
            "\"; drop",
            "'; delete",
            "\"; delete",
            "'; insert",
            "\"; insert",
            "'; update",
            "\"; update",
            "'; select",
            "\"; select",
            "union select",
            "union all select",
            "1=1",
            "1 = 1",
            "' --",
            "\" --",
            "/*",
            "*/",
            "xp_",
            "sp_",
            "@@",
            "char(",
            "nchar(",
            "varchar(",
            "exec(",
            "execute(",
        ];

        for pattern in patterns {
            if lower.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// Detect potential command injection
    fn detect_command_injection(&self, input: &str) -> bool {
        let patterns = [
            ";",
            "&&",
            "||",
            "|",
            "`",
            "$(",
            "$()",
            "${",
            "\n",
            "\r",
            ">",
            "<",
            ">>",
            "<<",
        ];

        for pattern in patterns {
            if input.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// Simple URL decoding
    fn url_decode(&self, input: &str) -> String {
        let mut result = input.to_string();
        result = result.replace("%3c", "<");
        result = result.replace("%3e", ">");
        result = result.replace("%22", "\"");
        result = result.replace("%27", "'");
        result = result.replace("%2f", "/");
        result = result.replace("%5c", "\\");
        result = result.replace("%2e", ".");
        result
    }

    /// Sanitize HTML by escaping special characters
    fn sanitize_html(&self, input: &str) -> String {
        input
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }

    /// Validate an API key format
    pub fn validate_api_key(&self, provider: &str, key: &str) -> Result<ValidationResult> {
        if key.is_empty() {
            return Err(ValidationError::Empty);
        }

        // Basic format validation per provider
        let is_valid = match provider.to_lowercase().as_str() {
            "openai" => key.starts_with("sk-") && key.len() > 20,
            "claude" | "anthropic" => key.starts_with("sk-ant-") && key.len() > 20,
            "gemini" | "google" => key.starts_with("AIza") && key.len() > 20,
            "elevenlabs" => key.len() > 20,
            _ => key.len() > 10,
        };

        if !is_valid {
            return Err(ValidationError::InvalidCharacters(format!(
                "API key format doesn't match expected {} format",
                provider
            )));
        }

        Ok(ValidationResult {
            is_valid: true,
            sanitized: key.to_string(),
            warnings: Vec::new(),
        })
    }
}

impl Default for InputValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xss_detection() {
        let validator = InputValidator::new();

        assert!(validator.validate_text("<script>alert('xss')</script>").is_err());
        assert!(validator.validate_text("javascript:alert(1)").is_err());
        assert!(validator.validate_text("Normal text").is_ok());
    }

    #[test]
    fn test_path_traversal() {
        let validator = InputValidator::new();

        assert!(validator.validate_path("../../../etc/passwd").is_err());
        assert!(validator.validate_path("..\\..\\windows").is_err());
        assert!(validator.validate_path("/home/user/file.txt").is_ok());
    }

    #[test]
    fn test_sql_injection() {
        let validator = InputValidator::new();

        assert!(validator.validate_query_param("'; DROP TABLE users; --").is_err());
        assert!(validator.validate_query_param("1' OR '1'='1").is_err());
        assert!(validator.validate_query_param("normal query").is_ok());
    }

    #[test]
    fn test_command_injection() {
        let validator = InputValidator::new();

        assert!(validator.validate_command_param("file; rm -rf /").is_err());
        assert!(validator.validate_command_param("$(whoami)").is_err());
        assert!(validator.validate_command_param("normal_filename").is_ok());
    }

    #[test]
    fn test_api_key_validation() {
        let validator = InputValidator::new();

        assert!(validator.validate_api_key("openai", "sk-test12345678901234567890").is_ok());
        assert!(validator.validate_api_key("claude", "sk-ant-test12345678901234567890").is_ok());
        assert!(validator.validate_api_key("openai", "invalid").is_err());
    }
}
