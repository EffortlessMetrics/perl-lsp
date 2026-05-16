#![warn(missing_docs)]
//! Input validation and sanitization utilities for production hardening.

mod constants;
mod file_validation;
mod lsp_validation;
mod sanitize;
mod workspace_validation;

pub use file_validation::{validate_file_content, validate_file_path};
pub use lsp_validation::validate_lsp_request;
pub use sanitize::sanitize_string;
pub use workspace_validation::validate_workspace_root;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn test_validate_file_path_valid() {
        use perl_tdd_support::must;
        let temp_dir = must(TempDir::new());
        let workspace_root = temp_dir.path();
        let file_path = workspace_root.join("test.pl");
        must(fs::write(&file_path, "print 'Hello';"));

        let result = validate_file_path(&file_path, workspace_root);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_path_traversal() {
        use perl_tdd_support::must;
        let temp_dir = must(TempDir::new());
        let workspace_root = temp_dir.path();
        let malicious_path = Path::new("../../etc/passwd");

        let result = validate_file_path(malicious_path, workspace_root);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_file_path_rejects_disallowed_extension() {
        use perl_tdd_support::must;
        let temp_dir = must(TempDir::new());
        let workspace_root = temp_dir.path();
        let file_path = workspace_root.join("notes.txt");
        must(fs::write(&file_path, "not perl"));

        let result = validate_file_path(&file_path, workspace_root);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_file_content_valid() {
        let content = "print 'Hello, World!';";
        let file_path = Path::new("test.pl");

        let result = validate_file_content(content, file_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_content_too_large() {
        let max = crate::runtime::limits::max_file_size_bytes();
        let mut content = String::new();
        content.reserve(max + 1);
        content.extend(std::iter::repeat_n('x', max + 1));
        let file_path = Path::new("large.pl");

        let result = validate_file_content(&content, file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_file_content_null_bytes() {
        let content = "print 'Hello';\0";
        let file_path = Path::new("null.pl");

        let result = validate_file_content(content, file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_file_content_line_too_long() {
        let long_line = "x".repeat(100_001);
        let file_path = Path::new("long_line.pl");

        let result = validate_file_content(&long_line, file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_file_content_suspicious_patterns_case_insensitive() {
        let content = "print q{<ScRiPt>alert('xss')</script>};";
        let file_path = Path::new("suspicious.pl");

        let result = validate_file_content(content, file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_file_content_exact_size_limit_allowed() {
        let max = crate::runtime::limits::max_file_size_bytes();
        let segment = "x".repeat(100_000);
        let repeats = max / 100_001;
        let remainder = max % 100_001;
        let mut content = (0..repeats).map(|_| format!("{segment}\n")).collect::<String>();
        if remainder > 0 {
            content.push_str(&"x".repeat(remainder));
        }

        let result = validate_file_content(&content, Path::new("exact_limit.pl"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_content_exact_line_limit_allowed() {
        let line = "x".repeat(100_000);
        let result = validate_file_content(&line, Path::new("line_limit.pl"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_sanitize_string() {
        let input = "Hello\x00World<script>alert('xss')</script>";
        let expected = "HelloWorld<script>alert('xss')</script>";

        let result = sanitize_string(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_validate_lsp_request_valid() {
        let method = "textDocument/didOpen";
        let params = serde_json::json!({
            "textDocument": {
                "uri": "file:///test.pl",
                "text": "print 'Hello';"
            }
        });

        let result = validate_lsp_request(method, &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_lsp_request_valid_opencode_uri() {
        let method = "textDocument/didOpen";
        let params = serde_json::json!({
            "textDocument": {
                "uri": "opencode:/workspace/lib/My/Module.pm",
                "text": "package My::Module;\n1;\n"
            }
        });

        let result = validate_lsp_request(method, &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_lsp_request_invalid_uri_scheme() {
        let method = "textDocument/didOpen";
        let params = serde_json::json!({
            "textDocument": {
                "uri": "https://example.com/test.pl",
                "text": "print 'Hello';"
            }
        });

        let result = validate_lsp_request(method, &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_lsp_request_invalid_method() {
        let method = "invalid<script>alert('xss')</script>";
        let params = serde_json::json!({});

        let result = validate_lsp_request(method, &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_lsp_request_invalid_text_document_uri_scheme() {
        let method = "textDocument/didOpen";
        let params = serde_json::json!({
            "textDocument": {
                "uri": "http://example.com/test.pl",
                "text": "print 'Hello';"
            }
        });

        let result = validate_lsp_request(method, &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_lsp_request_rejects_script_in_unknown_method_params() {
        let method = "workspace/symbol";
        let params = serde_json::json!({
            "query": "<script>alert('xss')</script>"
        });

        let result = validate_lsp_request(method, &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_execute_command_allowed() {
        let method = "workspace/executeCommand";
        let params = serde_json::json!({
            "command": "perl.runCritic",
            "arguments": []
        });

        let result = validate_lsp_request(method, &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_execute_command_allows_preview_safe_delete() {
        let method = "workspace/executeCommand";
        let params = serde_json::json!({
            "command": "perl.previewSafeDelete",
            "arguments": [{
                "textDocument": {"uri": "file:///workspace/lib/My.pm"},
                "position": {"line": 0, "character": 0}
            }]
        });

        let result = validate_lsp_request(method, &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_execute_command_allows_safe_delete_symbol() {
        let method = "workspace/executeCommand";
        let params = serde_json::json!({
            "command": "perl.safeDeleteSymbol",
            "arguments": [{
                "textDocument": {"uri": "file:///workspace/lib/My.pm"},
                "position": {"line": 0, "character": 0}
            }]
        });

        let result = validate_lsp_request(method, &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_execute_command_allows_preview_package_rename() {
        let method = "workspace/executeCommand";
        let params = serde_json::json!({
            "command": "perl.previewPackageRename",
            "arguments": [{
                "textDocument": {"uri": "file:///workspace/lib/My.pm"},
                "position": {"line": 0, "character": 0},
                "newName": "Renamed::Package"
            }]
        });

        let result = validate_lsp_request(method, &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_execute_command_blocked() {
        let method = "workspace/executeCommand";
        let params = serde_json::json!({
            "command": "rm -rf /",
            "arguments": []
        });

        let result = validate_lsp_request(method, &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_lsp_request_does_not_require_command_field() {
        let method = "workspace/executeCommand";
        let params = serde_json::json!({ "arguments": [] });

        let result = validate_lsp_request(method, &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_lsp_request_rejects_long_uri() {
        let method = "textDocument/didOpen";
        let uri = format!("file:///{}", "a".repeat(5000));
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "text": "print 'Hello';"
            }
        });

        let result = validate_lsp_request(method, &params);
        assert!(result.is_err());
    }

    /// Regression guard: the `untitled:` scheme must still be accepted.
    #[test]
    fn test_validate_lsp_request_valid_untitled_uri_still_accepted() {
        let method = "textDocument/didOpen";
        let params = serde_json::json!({
            "textDocument": {
                "uri": "untitled:Untitled-1",
                "text": "package Scratch;\n1;\n"
            }
        });

        let result = validate_lsp_request(method, &params);
        assert!(result.is_ok(), "untitled: URI must be accepted after scheme allowlist refactor");
    }

    /// Regression guard: the original `file://` scheme must still be accepted.
    #[test]
    fn test_validate_lsp_request_valid_file_uri_still_accepted() {
        let method = "textDocument/didChange";
        let params = serde_json::json!({
            "textDocument": {
                "uri": "file:///home/user/project/lib/My.pm",
                "text": "package My;\n1;\n"
            }
        });

        let result = validate_lsp_request(method, &params);
        assert!(result.is_ok(), "file:// URI must be accepted after scheme allowlist refactor");
    }

    #[test]
    fn test_file_size_limit_sourced_from_lsp_limits() {
        let expected_limit = crate::runtime::limits::max_file_size_bytes();
        assert_eq!(expected_limit, 1_024 * 1_024);
    }
}
