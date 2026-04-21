#![warn(missing_docs)]
//! Input validation and sanitization utilities for production hardening.

use anyhow::{Result, anyhow};
use perl_parser_core::path_security::validate_workspace_path;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

// NOTE(G2-API-fix): After absorption, input_validation no longer depends on the
// external perl-lsp-limits crate. It instead accesses the sibling limits module
// through the super::limits path within perl-lsp-rs-core::runtime.
use crate::runtime::limits::max_file_size_bytes as limits_max_file_size_bytes;

/// Maximum allowed path length.
const MAX_PATH_LENGTH: usize = 4096;

/// Allowed file extensions for Perl files.
const ALLOWED_EXTENSIONS: &[&str] = &["pl", "pm", "t", "pod"];

/// Validates and sanitizes a file path to prevent path traversal attacks.
pub fn validate_file_path<P: AsRef<Path>>(path: P, workspace_root: &Path) -> Result<PathBuf> {
    let path = path.as_ref();

    if path.to_string_lossy().len() > MAX_PATH_LENGTH {
        return Err(anyhow!("Path too long: {}", path.display()));
    }

    let validated = validate_workspace_path(path, workspace_root)
        .map_err(|error| anyhow!("Invalid workspace path {}: {error}", path.display()))?;

    if let Some(extension) = validated.extension().and_then(OsStr::to_str)
        && !ALLOWED_EXTENSIONS.contains(&extension)
    {
        return Err(anyhow!(
            "File extension '{}' not allowed. Allowed: {:?}",
            extension,
            ALLOWED_EXTENSIONS
        ));
    }

    Ok(validated)
}

/// Validates file content before parsing to prevent resource exhaustion.
///
/// The file size limit is sourced from [`crate::runtime::limits::max_file_size_bytes`],
/// which defaults to 1MB and is configurable via `perl.limits.maxFileSizeBytes`.
pub fn validate_file_content(content: &str, file_path: &Path) -> Result<()> {
    let max_file_size = limits_max_file_size_bytes();
    if content.len() > max_file_size {
        return Err(anyhow!(
            "File {} too large: {} bytes (max: {} bytes) — adjust perl.limits.maxFileSizeBytes to increase",
            file_path.display(),
            content.len(),
            max_file_size
        ));
    }

    if content.contains('\0') {
        return Err(anyhow!("File {} contains null bytes", file_path.display()));
    }

    for (index, line) in content.lines().enumerate() {
        if line.len() > 100_000 {
            return Err(anyhow!(
                "Line {} in file {} is too long: {} characters",
                index + 1,
                file_path.display(),
                line.len()
            ));
        }
    }

    let suspicious_patterns = ["<script", "javascript:", "data:text/html", "<?php", "<%"];
    let lowercase = content.to_lowercase();
    for pattern in suspicious_patterns {
        if lowercase.contains(pattern) {
            return Err(anyhow!(
                "File {} contains suspicious pattern: {}",
                file_path.display(),
                pattern
            ));
        }
    }

    Ok(())
}

/// Validates LSP request parameters to ensure they're safe.
pub fn validate_lsp_request(method: &str, params: &serde_json::Value) -> Result<()> {
    if method.len() > 100 || !method.chars().all(|c| c.is_alphanumeric() || c == '/' || c == '$') {
        return Err(anyhow!("Invalid LSP method: {}", method));
    }

    let params_str = serde_json::to_string(params)?;
    if params_str.len() > 1_000_000 {
        return Err(anyhow!("LSP parameters too large for method: {}", method));
    }

    match method {
        "textDocument/didOpen" | "textDocument/didChange" | "textDocument/didSave" => {
            validate_text_document_params(params)?;
        }
        "workspace/executeCommand" => {
            validate_execute_command_params(params)?;
        }
        _ => {
            if params_str.contains("javascript:") || params_str.contains("<script") {
                return Err(anyhow!("Suspicious content in parameters for method: {}", method));
            }
        }
    }

    Ok(())
}

fn validate_text_document_params(params: &serde_json::Value) -> Result<()> {
    if let Some(uri) = params
        .get("textDocument")
        .and_then(|text_document| text_document.get("uri"))
        .and_then(serde_json::Value::as_str)
    {
        if !uri.starts_with("file://") && !uri.starts_with("untitled:") {
            return Err(anyhow!("Invalid URI scheme: {}", uri));
        }

        if uri.len() > 4096 {
            return Err(anyhow!("URI too long: {}", uri));
        }
    }

    if let Some(text) = params
        .get("textDocument")
        .and_then(|text_document| text_document.get("text"))
        .and_then(serde_json::Value::as_str)
    {
        validate_file_content(text, Path::new("<lsp_input>"))?;
    }

    Ok(())
}

fn validate_execute_command_params(params: &serde_json::Value) -> Result<()> {
    if let Some(command) = params.get("command").and_then(serde_json::Value::as_str) {
        let allowed_commands = [
            "perl.runCritic",
            "perl.formatDocument",
            "perl.extractVariable",
            "perl.extractSubroutine",
            "perl.optimizeImports",
        ];

        if !allowed_commands.contains(&command) {
            return Err(anyhow!("Command not allowed: {}", command));
        }
    }

    Ok(())
}

/// Sanitizes a string by removing potentially dangerous characters.
pub fn sanitize_string(input: &str) -> String {
    input
        .chars()
        .filter(|character| {
            *character == '\t'
                || *character == '\n'
                || *character == '\r'
                || (*character >= ' ' && *character <= '~')
                || *character as u32 > 127
        })
        .collect()
}

/// Validates workspace root to ensure it's safe.
pub fn validate_workspace_root(workspace_root: &Path) -> Result<()> {
    if !workspace_root.exists() {
        return Err(anyhow!("Workspace root does not exist: {}", workspace_root.display()));
    }

    if !workspace_root.is_dir() {
        return Err(anyhow!("Workspace root is not a directory: {}", workspace_root.display()));
    }

    let path_str = workspace_root.to_string_lossy();
    if path_str.contains("..") || path_str.contains('~') {
        return Err(anyhow!("Suspicious workspace root path: {}", workspace_root.display()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
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
    fn test_validate_lsp_request_invalid_method() {
        let method = "invalid<script>alert('xss')</script>";
        let params = serde_json::json!({});

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
    fn test_file_size_limit_sourced_from_lsp_limits() {
        // The file size limit must come from perl-lsp-limits, not a local constant.
        // This test documents the expected single source of truth.
        let expected_limit = crate::runtime::limits::max_file_size_bytes();
        // Default is 1MB per LspLimits::default()
        assert_eq!(
            expected_limit,
            1_024 * 1_024,
            "default file size limit must be 1MB from perl-lsp-limits"
        );
    }
}
