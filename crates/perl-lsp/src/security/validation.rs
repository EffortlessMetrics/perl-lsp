//! Input validation and sanitization utilities for production hardening
//! 
//! This module provides comprehensive input validation functions to ensure
//! all external inputs are properly sanitized before processing.

use std::path::{Path, PathBuf};
use std::ffi::OsStr;
use anyhow::{Result, anyhow};

/// Maximum allowed file size for parsing (10MB)
const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;

/// Maximum allowed path length
const MAX_PATH_LENGTH: usize = 4096;

/// Allowed file extensions for Perl files
const ALLOWED_EXTENSIONS: &[&str] = &["pl", "pm", "t", "pod"];

/// Validates and sanitizes a file path to prevent path traversal attacks
/// 
/// # Arguments
/// * `path` - The input path to validate
/// * `workspace_root` - The allowed workspace root directory
/// 
/// # Returns
/// * `Ok(PathBuf)` - The validated and canonicalized path
/// * `Err(anyhow::Error)` - If the path is invalid or outside workspace
pub fn validate_file_path<P: AsRef<Path>>(path: P, workspace_root: &Path) -> Result<PathBuf> {
    let path = path.as_ref();
    
    // Check path length
    if path.to_string_lossy().len() > MAX_PATH_LENGTH {
        return Err(anyhow!("Path too long: {}", path.display()));
    }
    
    // Convert to absolute path
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    
    // Canonicalize to resolve symlinks and relative components
    let canonical_path = match absolute_path.canonicalize() {
        Ok(path) => path,
        Err(_) => return Err(anyhow!("Cannot canonicalize path: {}", path.display())),
    };
    
    // Ensure the path is within the workspace root
    let canonical_workspace = workspace_root.canonicalize()
        .map_err(|e| anyhow!("Cannot canonicalize workspace root: {}", e))?;
    
    if !canonical_path.starts_with(&canonical_workspace) {
        return Err(anyhow!(
            "Path {} is outside workspace root {}",
            canonical_path.display(),
            canonical_workspace.display()
        ));
    }
    
    // Validate file extension
    if let Some(extension) = canonical_path.extension().and_then(OsStr::to_str) {
        if !ALLOWED_EXTENSIONS.contains(&extension) {
            return Err(anyhow!(
                "File extension '{}' not allowed. Allowed: {:?}",
                extension,
                ALLOWED_EXTENSIONS
            ));
        }
    }
    
    Ok(canonical_path)
}

/// Validates file content before parsing to prevent resource exhaustion
/// 
/// # Arguments
/// * `content` - The file content to validate
/// * `file_path` - The path of the file (for error reporting)
/// 
/// # Returns
/// * `Ok(())` - If the content is valid
/// * `Err(anyhow::Error)` - If the content is invalid
pub fn validate_file_content(content: &str, file_path: &Path) -> Result<()> {
    // Check file size
    if content.len() > MAX_FILE_SIZE {
        return Err(anyhow!(
            "File {} too large: {} bytes (max: {})",
            file_path.display(),
            content.len(),
            MAX_FILE_SIZE
        ));
    }
    
    // Check for null bytes (can cause issues in string processing)
    if content.contains('\0') {
        return Err(anyhow!(
            "File {} contains null bytes",
            file_path.display()
        ));
    }
    
    // Check for extremely long lines (can indicate malformed content)
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.len() > 100_000 {
            return Err(anyhow!(
                "Line {} in file {} is too long: {} characters",
                i + 1,
                file_path.display(),
                line.len()
            ));
        }
    }
    
    // Check for suspicious patterns that might indicate injection attempts
    let suspicious_patterns = [
        "<script",  // HTML/JS injection
        "javascript:",  // JS protocol injection
        "data:text/html",  // Data URI injection
        "<?php",  // PHP injection
        "<%",  // Template injection
    ];
    
    for pattern in &suspicious_patterns {
        if content.to_lowercase().contains(pattern) {
            return Err(anyhow!(
                "File {} contains suspicious pattern: {}",
                file_path.display(),
                pattern
            ));
        }
    }
    
    Ok(())
}

/// Validates LSP request parameters to ensure they're safe
/// 
/// # Arguments
/// * `method` - The LSP method name
/// * `params` - The parameters to validate
/// 
/// # Returns
/// * `Ok(())` - If the parameters are valid
/// * `Err(anyhow::Error)` - If the parameters are invalid
pub fn validate_lsp_request(method: &str, params: &serde_json::Value) -> Result<()> {
    // Check method name for injection
    if method.len() > 100 || !method.chars().all(|c| c.is_alphanumeric() || c == '/' || c == '$') {
        return Err(anyhow!("Invalid LSP method: {}", method));
    }
    
    // Check parameter size to prevent DoS
    let params_str = serde_json::to_string(params)?;
    if params_str.len() > 1_000_000 {
        return Err(anyhow!("LSP parameters too large for method: {}", method));
    }
    
    // Method-specific validations
    match method {
        "textDocument/didOpen" | "textDocument/didChange" | "textDocument/didSave" => {
            validate_text_document_params(params)?;
        }
        "workspace/executeCommand" => {
            validate_execute_command_params(params)?;
        }
        _ => {
            // For unknown methods, do basic validation
            if params_str.contains("javascript:") || params_str.contains("<script") {
                return Err(anyhow!("Suspicious content in parameters for method: {}", method));
            }
        }
    }
    
    Ok(())
}

/// Validates text document parameters for LSP methods
fn validate_text_document_params(params: &serde_json::Value) -> Result<()> {
    if let Some(uri) = params.get("textDocument")
        .and_then(|td| td.get("uri"))
        .and_then(|u| u.as_str()) {
        
        // Validate URI format
        if !uri.starts_with("file://") && !uri.starts_with("untitled:") {
            return Err(anyhow!("Invalid URI scheme: {}", uri));
        }
        
        // Check URI length
        if uri.len() > 4096 {
            return Err(anyhow!("URI too long: {}", uri));
        }
    }
    
    if let Some(text) = params.get("textDocument")
        .and_then(|td| td.get("text"))
        .and_then(|t| t.as_str()) {
        
        // Validate text content
        validate_file_content(text, Path::new("<lsp_input>"))?;
    }
    
    Ok(())
}

/// Validates execute command parameters for LSP methods
fn validate_execute_command_params(params: &serde_json::Value) -> Result<()> {
    if let Some(command) = params.get("command").and_then(|c| c.as_str()) {
        // Whitelist allowed commands
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

/// Sanitizes a string by removing potentially dangerous characters
/// 
/// # Arguments
/// * `input` - The input string to sanitize
/// 
/// # Returns
/// * A sanitized string safe for processing
pub fn sanitize_string(input: &str) -> String {
    input
        .chars()
        .filter(|c| {
            // Allow printable ASCII and common Unicode characters
            *c == '\t' || *c == '\n' || *c == '\r' || // Control chars for formatting
            (*c >= ' ' && *c <= '~') || // Printable ASCII
            *c as u32 > 127 // Unicode characters
        })
        .collect()
}

/// Validates workspace root to ensure it's safe
/// 
/// # Arguments
/// * `workspace_root` - The workspace root path to validate
/// 
/// # Returns
/// * `Ok(())` - If the workspace root is valid
/// * `Err(anyhow::Error)` - If the workspace root is invalid
pub fn validate_workspace_root(workspace_root: &Path) -> Result<()> {
    // Check if path exists and is a directory
    if !workspace_root.exists() {
        return Err(anyhow!("Workspace root does not exist: {}", workspace_root.display()));
    }
    
    if !workspace_root.is_dir() {
        return Err(anyhow!("Workspace root is not a directory: {}", workspace_root.display()));
    }
    
    // Check for suspicious patterns in path
    let path_str = workspace_root.to_string_lossy();
    if path_str.contains("..") || path_str.contains("~") {
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
        let mut content = String::new();
        content.resize(MAX_FILE_SIZE + 1, 'x');
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
}