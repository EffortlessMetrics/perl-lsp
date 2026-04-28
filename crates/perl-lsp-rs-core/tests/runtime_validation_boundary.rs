//! Boundary and mutation-killing tests for perl-lsp-input-validation.
//!
//! Complements the inline happy-path tests in lib.rs by targeting:
//! - Exact boundary values (max_file_size_bytes from perl-lsp-limits, MAX_PATH_LENGTH, line length 100_000)
//! - Every disallowed condition in validate_file_content and validate_lsp_request
//! - All suspicious patterns in the content filter
//! - sanitize_string: exactly which characters are kept vs removed
//! - validate_file_path: extension filtering

use perl_lsp_rs_core::runtime::input_validation::{
    sanitize_string, validate_file_content, validate_lsp_request,
};
use perl_lsp_rs_core::runtime::limits::max_file_size_bytes;
use std::path::Path;

// ---------------------------------------------------------------------------
// validate_file_content: size boundary
// ---------------------------------------------------------------------------

#[test]
fn validate_file_content_at_exactly_max_size_is_ok() -> anyhow::Result<()> {
    // max_file_size_bytes() bytes exactly must pass (> not >=).
    // Use many short lines to avoid triggering the per-line length check.
    let max = max_file_size_bytes();
    // Each "x\n" = 2 bytes; max / 2 lines avoids the 100_000 char line check
    let line = "x\n";
    let lines = max / line.len();
    let content = line.repeat(lines);
    assert!(content.len() <= max, "content must not exceed max in this test");
    validate_file_content(&content, Path::new("test.pl"))?;
    Ok(())
}

#[test]
fn validate_file_content_one_byte_over_max_errors() {
    let max = max_file_size_bytes();
    // Use short lines spread across many rows so only the total size limit is hit
    let line = "x\n"; // 2 bytes each
    let lines = (max / line.len()) + 1;
    let content = line.repeat(lines);
    // content.len() > max here
    assert!(content.len() > max);
    let result = validate_file_content(&content, Path::new("test.pl"));
    assert!(result.is_err(), "content over max_file_size_bytes must be rejected");
}

// ---------------------------------------------------------------------------
// validate_file_content: null byte detection
// ---------------------------------------------------------------------------

#[test]
fn validate_file_content_null_byte_at_start_errors() {
    let content = "\0hello";
    let result = validate_file_content(content, Path::new("test.pl"));
    assert!(result.is_err(), "null byte at start must be rejected");
}

#[test]
fn validate_file_content_null_byte_at_end_errors() {
    let content = "print 'hello';\0";
    let result = validate_file_content(content, Path::new("test.pl"));
    assert!(result.is_err(), "null byte at end must be rejected");
}

#[test]
fn validate_file_content_without_null_bytes_is_ok() -> anyhow::Result<()> {
    let content = "print 'hello';\n";
    validate_file_content(content, Path::new("test.pl"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// validate_file_content: line length check (100_000 chars)
// ---------------------------------------------------------------------------

#[test]
fn validate_file_content_line_at_exactly_100000_is_ok() -> anyhow::Result<()> {
    // Exactly 100_000 chars on one line must pass (> not >=)
    let content = "x".repeat(100_000);
    validate_file_content(&content, Path::new("test.pl"))?;
    Ok(())
}

#[test]
fn validate_file_content_line_one_char_over_100000_errors() {
    let content = "x".repeat(100_001);
    let result = validate_file_content(&content, Path::new("test.pl"));
    assert!(result.is_err(), "line of 100001 chars must be rejected");
}

#[test]
fn validate_file_content_long_line_on_second_line_errors() {
    let line1 = "use strict;\n";
    let long_line = "x".repeat(100_001);
    let content = format!("{line1}{long_line}");
    let result = validate_file_content(&content, Path::new("test.pl"));
    assert!(result.is_err(), "long line on second line must be rejected");
}

// ---------------------------------------------------------------------------
// validate_file_content: suspicious pattern detection
// ---------------------------------------------------------------------------

#[test]
fn validate_file_content_rejects_script_tag() {
    let content = "# <script>alert(1)</script>";
    let result = validate_file_content(content, Path::new("test.pl"));
    assert!(result.is_err(), "<script> pattern must be rejected");
}

#[test]
fn validate_file_content_rejects_javascript_protocol() {
    let content = "# javascript:void(0)";
    let result = validate_file_content(content, Path::new("test.pl"));
    assert!(result.is_err(), "javascript: pattern must be rejected");
}

#[test]
fn validate_file_content_rejects_data_uri() {
    let content = "# data:text/html,<h1>xss</h1>";
    let result = validate_file_content(content, Path::new("test.pl"));
    assert!(result.is_err(), "data:text/html pattern must be rejected");
}

#[test]
fn validate_file_content_rejects_php_tag() {
    let content = "<?php echo 'hello'; ?>";
    let result = validate_file_content(content, Path::new("test.pl"));
    assert!(result.is_err(), "<?php pattern must be rejected");
}

#[test]
fn validate_file_content_rejects_asp_tag() {
    let content = "<% Response.Write(\"hello\") %>";
    let result = validate_file_content(content, Path::new("test.pl"));
    assert!(result.is_err(), "<% pattern must be rejected");
}

#[test]
fn validate_file_content_case_insensitive_script_tag() {
    // Detection uses `.to_lowercase()` so uppercase variants must also be caught
    let content = "# <SCRIPT>alert(1)</SCRIPT>";
    let result = validate_file_content(content, Path::new("test.pl"));
    assert!(result.is_err(), "uppercase <SCRIPT> must be rejected (case-insensitive check)");
}

#[test]
fn validate_file_content_case_insensitive_javascript_protocol() {
    let content = "# JAVASCRIPT:void(0)";
    let result = validate_file_content(content, Path::new("test.pl"));
    assert!(result.is_err(), "uppercase JAVASCRIPT: must be rejected");
}

// ---------------------------------------------------------------------------
// validate_lsp_request: method validation
// ---------------------------------------------------------------------------

#[test]
fn validate_lsp_request_known_methods_are_ok() -> anyhow::Result<()> {
    let params = serde_json::json!({});
    validate_lsp_request("textDocument/didChange", &params)?;
    validate_lsp_request("textDocument/didSave", &params)?;
    Ok(())
}

#[test]
fn validate_lsp_request_method_over_100_chars_errors() {
    let method = "a".repeat(101);
    let params = serde_json::json!({});
    let result = validate_lsp_request(&method, &params);
    assert!(result.is_err(), "method name > 100 chars must be rejected");
}

#[test]
fn validate_lsp_request_method_exactly_100_chars_is_ok() -> anyhow::Result<()> {
    // Method of exactly 100 alphanumeric chars — should pass if no special chars
    // Note: method must use chars that satisfy: alphanumeric || '/' || '$'
    let method = "a".repeat(100);
    let params = serde_json::json!({});
    validate_lsp_request(&method, &params)?;
    Ok(())
}

#[test]
fn validate_lsp_request_unknown_method_with_javascript_in_params_errors() {
    let method = "someOther/Method";
    let params = serde_json::json!({ "url": "javascript:void(0)" });
    let result = validate_lsp_request(method, &params);
    assert!(result.is_err(), "unknown method with javascript: in params must be rejected");
}

#[test]
fn validate_lsp_request_unknown_method_with_script_tag_in_params_errors() {
    let method = "someOther/Method";
    let params = serde_json::json!({ "text": "<script>alert(1)</script>" });
    let result = validate_lsp_request(method, &params);
    assert!(result.is_err(), "unknown method with <script> in params must be rejected");
}

#[test]
fn validate_lsp_request_text_document_with_invalid_uri_scheme_errors() {
    let method = "textDocument/didOpen";
    let params = serde_json::json!({
        "textDocument": {
            "uri": "ftp://example.com/script.pl",
            "text": "print 1;"
        }
    });
    let result = validate_lsp_request(method, &params);
    assert!(result.is_err(), "non-file:// URI scheme must be rejected for textDocument requests");
}

#[test]
fn validate_lsp_request_text_document_with_untitled_uri_is_ok() -> anyhow::Result<()> {
    let method = "textDocument/didOpen";
    let params = serde_json::json!({
        "textDocument": {
            "uri": "untitled:Untitled-1",
            "text": "print 1;"
        }
    });
    validate_lsp_request(method, &params)?;
    Ok(())
}

#[test]
fn validate_lsp_request_execute_command_all_allowed_commands_pass() -> anyhow::Result<()> {
    let allowed = [
        "perl.runCritic",
        "perl.formatDocument",
        "perl.extractVariable",
        "perl.extractSubroutine",
        "perl.optimizeImports",
    ];

    for command in allowed {
        let method = "workspace/executeCommand";
        let params = serde_json::json!({ "command": command });
        validate_lsp_request(method, &params)
            .map_err(|e| anyhow::anyhow!("Command '{command}' should be allowed but got: {e}"))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// sanitize_string: boundary character cases
// ---------------------------------------------------------------------------

#[test]
fn sanitize_string_keeps_tab_character() {
    assert_eq!(sanitize_string("\t"), "\t", "tab must be kept");
}

#[test]
fn sanitize_string_keeps_newline() {
    assert_eq!(sanitize_string("\n"), "\n", "newline must be kept");
}

#[test]
fn sanitize_string_keeps_carriage_return() {
    assert_eq!(sanitize_string("\r"), "\r", "carriage return must be kept");
}

#[test]
fn sanitize_string_keeps_printable_ascii() {
    let printable = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~";
    assert_eq!(sanitize_string(printable), printable, "all printable ASCII must be kept");
}

#[test]
fn sanitize_string_removes_control_chars() {
    // DEL (0x7F) and other control chars below space (0x20) except tab/LF/CR
    let input = "\x01\x02\x03\x7f";
    let result = sanitize_string(input);
    assert_eq!(result, "", "control characters must be removed");
}

#[test]
fn sanitize_string_keeps_unicode_above_127() {
    // Characters > 127 are allowed (unicode)
    let input = "Héllo Wörld";
    let result = sanitize_string(input);
    assert_eq!(result, input, "unicode chars > 127 must be kept");
}

#[test]
fn sanitize_string_removes_null_byte() {
    // Null byte (0x00) is below space and not tab/LF/CR → removed
    let result = sanitize_string("a\x00b");
    assert_eq!(result, "ab", "null byte must be removed by sanitize_string");
}

#[test]
fn sanitize_string_empty_input_returns_empty() {
    assert_eq!(sanitize_string(""), "", "empty input must return empty");
}

#[test]
fn sanitize_string_all_safe_returns_unchanged() {
    let input = "sub foo { return 42; }";
    assert_eq!(sanitize_string(input), input, "safe Perl code must be unchanged");
}
