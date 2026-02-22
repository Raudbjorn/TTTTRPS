//! Property-based tests for the Input Validator module
//!
//! Tests invariants:
//! - Never accepts script tags
//! - Never accepts SQL keywords in dangerous positions
//! - Never accepts path traversal sequences
//! - Accepts all alphanumeric input
//! - Consistent results for same input

use proptest::prelude::*;

use crate::core::input_validator::{InputValidator, ValidationError};

// ============================================================================
// Strategies for generating test inputs
// ============================================================================

/// Generate alphanumeric strings (should always be safe)
fn arb_alphanumeric() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9]{1,100}"
}

/// Generate safe text that doesn't contain dangerous patterns
fn arb_safe_text() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 .,!?-]{1,1000}"
}

/// Generate various XSS attack patterns
fn arb_xss_pattern() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("<script>alert('xss')</script>".to_string()),
        Just("<SCRIPT>alert('xss')</SCRIPT>".to_string()),
        Just("<script src=\"evil.js\">".to_string()),
        Just("javascript:alert(1)".to_string()),
        Just("JAVASCRIPT:alert(1)".to_string()),
        Just("<img onerror=\"alert(1)\">".to_string()),
        Just("<img onload=\"alert(1)\">".to_string()),
        Just("<div onclick=\"alert(1)\">".to_string()),
        Just("<body onmouseover=\"alert(1)\">".to_string()),
        Just("eval(String.fromCharCode(97,108,101,114,116,40,49,41))".to_string()),
        Just("document.cookie".to_string()),
        Just("document.write('<script>')".to_string()),
        Just("innerHTML = '<script>'".to_string()),
        Just("<iframe src=\"evil.com\">".to_string()),
        Just("<object data=\"evil.swf\">".to_string()),
        Just("<embed src=\"evil.swf\">".to_string()),
        Just("<form action=\"evil.com\">".to_string()),
        Just("data:text/html,<script>alert(1)</script>".to_string()),
        Just("vbscript:msgbox(1)".to_string()),
        // URL-encoded variants
        Just("%3cscript%3ealert(1)%3c/script%3e".to_string()),
        Just("%3Cscript%3Ealert(1)%3C/script%3E".to_string()),
    ]
}

/// Generate SQL injection patterns
fn arb_sql_injection() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("' OR '1'='1".to_string()),
        Just("\" OR \"1\"=\"1".to_string()),
        Just("' AND '1'='1".to_string()),
        Just("'; DROP TABLE users; --".to_string()),
        Just("\"; DROP TABLE users; --".to_string()),
        Just("'; DELETE FROM users; --".to_string()),
        Just("'; INSERT INTO users VALUES('hacked'); --".to_string()),
        Just("'; UPDATE users SET password='hacked'; --".to_string()),
        Just("'; SELECT * FROM users; --".to_string()),
        Just("UNION SELECT * FROM users".to_string()),
        Just("UNION ALL SELECT * FROM passwords".to_string()),
        Just("1=1".to_string()),
        Just("1 = 1".to_string()),
        Just("admin' --".to_string()),
        Just("/* comment */".to_string()),
        Just("xp_cmdshell".to_string()),
        Just("sp_executesql".to_string()),
        Just("@@version".to_string()),
        Just("char(65)".to_string()),
        Just("nchar(65)".to_string()),
        Just("varchar(100)".to_string()),
        Just("exec(command)".to_string()),
        Just("execute(command)".to_string()),
    ]
}

/// Generate path traversal patterns
fn arb_path_traversal() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("../../../etc/passwd".to_string()),
        Just("..\\..\\..\\windows\\system32".to_string()),
        Just("....//....//etc/passwd".to_string()),
        Just("....\\\\....\\\\windows".to_string()),
        Just("..%2f..%2f..%2fetc%2fpasswd".to_string()),
        Just("..%5c..%5c..%5cwindows".to_string()),
        Just("%2e%2e%2f%2e%2e%2fetc".to_string()),
        Just("../".to_string()),
        Just("..\\".to_string()),
        Just("..".to_string()),
    ]
}

/// Generate command injection patterns
fn arb_command_injection() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("file; rm -rf /".to_string()),
        Just("file && cat /etc/passwd".to_string()),
        Just("file || echo hacked".to_string()),
        Just("file | cat /etc/passwd".to_string()),
        Just("`whoami`".to_string()),
        Just("$(whoami)".to_string()),
        Just("$()".to_string()),
        Just("${PATH}".to_string()),
        Just("command\nmalicious".to_string()),
        Just("command\rmalicious".to_string()),
        Just("file > /tmp/hacked".to_string()),
        Just("file < /etc/passwd".to_string()),
        Just("file >> /tmp/log".to_string()),
        Just("file << EOF".to_string()),
    ]
}

/// Generate inputs with null bytes
fn arb_with_null_byte() -> impl Strategy<Value = String> {
    arb_safe_text().prop_map(|s| format!("{}\0malicious", s))
}

// ============================================================================
// Property Tests
// ============================================================================

proptest! {
    /// Property: Never accepts script tags
    #[test]
    fn prop_never_accepts_script_tags(
        xss in arb_xss_pattern()
    ) {
        let validator = InputValidator::new();
        let result = validator.validate_text(&xss);

        prop_assert!(
            result.is_err(),
            "XSS pattern '{}' should be rejected",
            xss
        );
    }

    /// Property: Never accepts SQL injection patterns in query validation
    #[test]
    fn prop_never_accepts_sql_injection(
        sql_pattern in arb_sql_injection()
    ) {
        let validator = InputValidator::new();
        let result = validator.validate_query_param(&sql_pattern);

        prop_assert!(
            result.is_err(),
            "SQL injection pattern '{}' should be rejected",
            sql_pattern
        );
    }

    /// Property: Never accepts path traversal sequences
    #[test]
    fn prop_never_accepts_path_traversal(
        path_pattern in arb_path_traversal()
    ) {
        let validator = InputValidator::new();
        let result = validator.validate_path(&path_pattern);

        prop_assert!(
            result.is_err(),
            "Path traversal pattern '{}' should be rejected",
            path_pattern
        );
    }

    /// Property: Never accepts command injection patterns
    #[test]
    fn prop_never_accepts_command_injection(
        cmd_pattern in arb_command_injection()
    ) {
        let validator = InputValidator::new();
        let result = validator.validate_command_param(&cmd_pattern);

        prop_assert!(
            result.is_err(),
            "Command injection pattern '{}' should be rejected",
            cmd_pattern
        );
    }

    /// Property: Always accepts pure alphanumeric input
    #[test]
    fn prop_accepts_alphanumeric(
        input in arb_alphanumeric()
    ) {
        let validator = InputValidator::new();
        let result = validator.validate_text(&input);

        prop_assert!(
            result.is_ok(),
            "Alphanumeric input '{}' should be accepted",
            input
        );
    }

    /// Property: Always accepts safe text for general validation
    #[test]
    fn prop_accepts_safe_text(
        input in arb_safe_text()
    ) {
        let validator = InputValidator::new();
        let result = validator.validate_text(&input);

        prop_assert!(
            result.is_ok(),
            "Safe text '{}' should be accepted",
            input
        );
    }

    /// Property: Consistent results for same input (idempotency)
    #[test]
    fn prop_consistent_results_for_same_input(
        input in ".*"
    ) {
        let validator = InputValidator::new();

        let result1 = validator.validate_text(&input);
        let result2 = validator.validate_text(&input);

        // Both should have the same Ok/Err status
        prop_assert_eq!(
            result1.is_ok(), result2.is_ok(),
            "Validation should be consistent for input"
        );

        // If both Ok, sanitized output should match
        if let (Ok(r1), Ok(r2)) = (result1, result2) {
            prop_assert_eq!(
                r1.sanitized, r2.sanitized,
                "Sanitized output should be consistent"
            );
        }
    }

    /// Property: Never accepts inputs with null bytes
    #[test]
    fn prop_rejects_null_bytes(
        input in arb_with_null_byte()
    ) {
        let validator = InputValidator::new();
        let result = validator.validate_text(&input);

        prop_assert!(
            result.is_err(),
            "Input with null byte should be rejected"
        );

        // Verify it's specifically a NullByte error
        if let Err(e) = result {
            match e {
                ValidationError::NullByte => prop_assert!(true),
                other => prop_assert!(false, "Expected NullByte error, got {:?}", other),
            }
        }
    }

    /// Property: Path validation rejects null bytes
    #[test]
    fn prop_path_rejects_null_bytes(
        base_path in "/[a-z]{1,10}(/[a-z]{1,10}){0,3}"
    ) {
        let validator = InputValidator::new();
        let path_with_null = format!("{}\0malicious", base_path);
        let result = validator.validate_path(&path_with_null);

        prop_assert!(
            result.is_err(),
            "Path with null byte should be rejected"
        );
    }

    /// Property: Empty input is handled correctly
    #[test]
    fn prop_empty_input_handling(
        _seed in any::<u64>() // Just to make it a property test
    ) {
        let validator = InputValidator::new();

        // Empty string for text validation should be OK
        let result = validator.validate_text("");
        prop_assert!(result.is_ok(), "Empty text should be accepted");

        // Empty API key should fail
        let result = validator.validate_api_key("openai", "");
        prop_assert!(result.is_err(), "Empty API key should be rejected");
    }

    /// Property: Very long inputs are rejected
    #[test]
    fn prop_long_inputs_rejected(
        repeat in 100001usize..200000
    ) {
        let validator = InputValidator::new();
        let long_input = "a".repeat(repeat);
        let result = validator.validate_text(&long_input);

        prop_assert!(
            result.is_err(),
            "Input of length {} should be rejected (max 100000)",
            repeat
        );
    }

    /// Property: Inputs within length limit are accepted
    #[test]
    fn prop_reasonable_length_accepted(
        len in 1usize..1000
    ) {
        let validator = InputValidator::new();
        let input = "a".repeat(len);
        let result = validator.validate_text(&input);

        prop_assert!(
            result.is_ok(),
            "Input of length {} should be accepted",
            len
        );
    }

    /// Property: Valid file extensions are accepted
    #[test]
    fn prop_valid_extensions_accepted(
        extension in prop_oneof![
            Just("pdf"),
            Just("epub"),
            Just("mobi"),
            Just("txt"),
            Just("md"),
            Just("json"),
        ]
    ) {
        let validator = InputValidator::new();
        let path = format!("/home/user/file.{}", extension);
        let result = validator.validate_file_path(&path);

        prop_assert!(
            result.is_ok(),
            "Path with .{} extension should be accepted",
            extension
        );
    }

    /// Property: Invalid file extensions are rejected
    #[test]
    fn prop_invalid_extensions_rejected(
        extension in prop_oneof![
            Just("exe"),
            Just("bat"),
            Just("sh"),
            Just("dll"),
            Just("so"),
            Just("php"),
            Just("py"),
            Just("js"),
        ]
    ) {
        let validator = InputValidator::new();
        let path = format!("/home/user/file.{}", extension);
        let result = validator.validate_file_path(&path);

        prop_assert!(
            result.is_err(),
            "Path with .{} extension should be rejected",
            extension
        );
    }

    /// Property: OpenAI API key format validation
    #[test]
    fn prop_openai_api_key_format(
        suffix in "[a-zA-Z0-9]{30,50}"
    ) {
        let validator = InputValidator::new();

        // Valid format (starts with sk-)
        let valid_key = format!("sk-{}", suffix);
        let result = validator.validate_api_key("openai", &valid_key);
        prop_assert!(
            result.is_ok(),
            "Valid OpenAI key format should be accepted"
        );

        // Invalid format (doesn't start with sk-)
        let invalid_key = format!("invalid-{}", suffix);
        let result = validator.validate_api_key("openai", &invalid_key);
        prop_assert!(
            result.is_err(),
            "Invalid OpenAI key format should be rejected"
        );
    }

    /// Property: Claude API key format validation
    #[test]
    fn prop_claude_api_key_format(
        suffix in "[a-zA-Z0-9]{30,50}"
    ) {
        let validator = InputValidator::new();

        // Valid format (starts with sk-ant-)
        let valid_key = format!("sk-ant-{}", suffix);
        let result = validator.validate_api_key("claude", &valid_key);
        prop_assert!(
            result.is_ok(),
            "Valid Claude key format should be accepted"
        );

        // Invalid format
        let invalid_key = format!("sk-{}", suffix); // OpenAI format, not Claude
        let result = validator.validate_api_key("claude", &invalid_key);
        prop_assert!(
            result.is_err(),
            "Invalid Claude key format should be rejected"
        );
    }

    /// Property: Sanitization produces valid output
    #[test]
    fn prop_sanitization_produces_valid_output(
        input in "[a-zA-Z0-9<>&\"']{1,100}"
    ) {
        let validator = InputValidator::new();

        // If validation succeeds, sanitized output should be valid UTF-8
        if let Ok(result) = validator.validate_text(&input) {
            // Verify the sanitized string is valid UTF-8
            let bytes = result.sanitized.as_bytes();
            let revalidated = std::str::from_utf8(bytes);
            prop_assert!(revalidated.is_ok(), "Sanitized output should be valid UTF-8");

            // Verify dangerous characters are escaped
            prop_assert!(
                !result.sanitized.contains('<') || result.sanitized.contains("&lt;"),
                "< should be escaped in output"
            );
            prop_assert!(
                !result.sanitized.contains('>') || result.sanitized.contains("&gt;"),
                "> should be escaped in output"
            );
        }
    }

    /// Property: Case-insensitive XSS detection
    #[test]
    fn prop_xss_detection_case_insensitive(
        prefix in "[a-zA-Z0-9 ]{0,10}",
        suffix in "[a-zA-Z0-9 ]{0,10}"
    ) {
        let validator = InputValidator::new();

        // Various case combinations of "script"
        let patterns = vec![
            format!("{}<script>alert(1)</script>{}", prefix, suffix),
            format!("{}<SCRIPT>alert(1)</SCRIPT>{}", prefix, suffix),
            format!("{}<ScRiPt>alert(1)</ScRiPt>{}", prefix, suffix),
            format!("{}<sCrIpT>alert(1)</sCrIpT>{}", prefix, suffix),
        ];

        for pattern in patterns {
            let result = validator.validate_text(&pattern);
            prop_assert!(
                result.is_err(),
                "Case-insensitive script tag '{}' should be detected",
                pattern
            );
        }
    }

    /// Property: URL-encoded XSS detection
    #[test]
    fn prop_url_encoded_xss_detection(
        _seed in any::<u64>()
    ) {
        let validator = InputValidator::new();

        // URL-encoded script tag variants
        let patterns = vec![
            "%3cscript%3ealert(1)%3c/script%3e",
            "%3Cscript%3Ealert(1)%3C/script%3E",
        ];

        for pattern in patterns {
            let result = validator.validate_text(pattern);
            prop_assert!(
                result.is_err(),
                "URL-encoded XSS pattern '{}' should be detected",
                pattern
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Basic sanity test that the validator exists and works
    #[test]
    fn test_validator_exists() {
        let validator = InputValidator::new();
        let result = validator.validate_text("hello world");
        assert!(result.is_ok());
    }

    /// Test that XSS is detected
    #[test]
    fn test_xss_detected() {
        let validator = InputValidator::new();
        let result = validator.validate_text("<script>alert('xss')</script>");
        assert!(result.is_err());
    }

    /// Test that SQL injection is detected
    #[test]
    fn test_sql_injection_detected() {
        let validator = InputValidator::new();
        let result = validator.validate_query_param("' OR '1'='1");
        assert!(result.is_err());
    }
}
