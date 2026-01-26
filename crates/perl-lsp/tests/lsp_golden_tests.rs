//! Golden fixture tests for LSP functionality
//!
//! These tests use fixture files to ensure consistent behavior across releases

mod support;

use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use support::test_helpers::{
    assert_completion_has_items, assert_folding_ranges_valid, assert_hover_has_text,
};
use url::Url;

/// Convert a path to a file:// URI string, cross-platform safe
fn path_to_uri(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Url::from_file_path(path)
        .map_err(|_| format!("file path to URI failed: {}", path.display()))?
        .to_string())
}

/// Test context that manages the LSP server lifecycle
struct TestContext {
    server: std::process::Child,
    reader: std::io::BufReader<std::process::ChildStdout>,
    writer: std::process::ChildStdin,
}

/// Compile-time path to the perl-lsp binary, set by Cargo when building integration tests.
const CARGO_BIN_EXE: Option<&str> = option_env!("CARGO_BIN_EXE_perl-lsp");

impl TestContext {
    /// Find the perl-lsp binary using multiple resolution strategies
    fn find_perl_lsp_binary() -> std::process::Command {
        // Resolution order:
        // 1. Compile-time CARGO_BIN_EXE (most reliable for `cargo test`)
        // 2. Runtime CARGO_BIN_EXE_perl-lsp env var
        // 3. Workspace target/debug/perl-lsp
        // 4. Fallback to cargo run (slow but always works)

        if let Some(bin_path) = CARGO_BIN_EXE {
            if std::path::Path::new(bin_path).exists() {
                let mut cmd = std::process::Command::new(bin_path);
                cmd.arg("--stdio");
                return cmd;
            }
        }

        if let Ok(bin_path) = std::env::var("CARGO_BIN_EXE_perl-lsp") {
            if std::path::Path::new(&bin_path).exists() {
                let mut cmd = std::process::Command::new(bin_path);
                cmd.arg("--stdio");
                return cmd;
            }
        }

        // Try workspace target directory
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let crate_dir = std::path::Path::new(&manifest_dir);
            if let Some(workspace_root) =
                crate_dir.ancestors().find(|p| p.join("Cargo.lock").exists())
            {
                let debug_binary = workspace_root.join("target/debug/perl-lsp");
                if debug_binary.exists() {
                    let mut cmd = std::process::Command::new(&debug_binary);
                    cmd.arg("--stdio");
                    return cmd;
                }
            }
        }

        // Fallback to cargo run (slow)
        let mut cmd = std::process::Command::new("cargo");
        cmd.args(["run", "-q", "-p", "perl-lsp", "--", "--stdio"]);
        cmd
    }

    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Start LSP server using optimized binary resolution
        let mut cmd = Self::find_perl_lsp_binary();
        let mut server = cmd
            .env("LSP_TEST_MODE", "1") // Enable test optimizations
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start LSP server: {}", e))?;

        let reader =
            std::io::BufReader::new(server.stdout.take().ok_or("Failed to capture server stdout")?);
        let writer = server.stdin.take().ok_or("Failed to capture server stdin")?;

        let mut ctx = TestContext { server, reader, writer };

        // Initialize with minimal capabilities for faster startup
        let current_dir = std::env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {}", e))?;
        let root_uri = path_to_uri(&current_dir)?;

        ctx.send_request(
            "initialize",
            Some(json!({
                "processId": std::process::id(),
                "capabilities": {
                    "textDocument": {
                        "hover": { "contentFormat": ["plaintext"] },
                        "completion": {}
                    }
                },
                "rootUri": root_uri
            })),
        )?;

        ctx.send_notification("initialized", None)?;

        // Give minimal time for initialization
        std::thread::sleep(std::time::Duration::from_millis(50));

        Ok(ctx)
    }

    fn send_request(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Option<Value>, Box<dyn std::error::Error>> {
        use std::io::{BufRead, Read, Write};

        static mut REQUEST_ID: i32 = 1;
        let id = unsafe {
            let current = REQUEST_ID;
            REQUEST_ID += 1;
            current
        };

        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params.unwrap_or(json!({}))
        });

        let content = serde_json::to_string(&request)
            .map_err(|e| format!("Failed to serialize request: {}", e))?;
        let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

        self.writer
            .write_all(message.as_bytes())
            .map_err(|e| format!("Failed to write request: {}", e))?;
        self.writer.flush().map_err(|e| format!("Failed to flush writer: {}", e))?;

        // Read response using proper LSP framing:
        // 1. Parse headers line-by-line until blank line
        // 2. Extract Content-Length from headers
        // 3. Read exactly Content-Length bytes for body
        // 4. Do NOT call read_line after the blank line!
        let mut content_length: Option<usize> = None;
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = self
                .reader
                .read_line(&mut line)
                .map_err(|e| format!("Failed to read response header: {}", e))?;
            if bytes_read == 0 {
                return Ok(None); // EOF
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                // Blank line = end of headers
                break;
            }

            // Parse Content-Length header (case-insensitive)
            let lower = trimmed.to_ascii_lowercase();
            if let Some(rest) = lower.strip_prefix("content-length") {
                let value_part = rest.trim_start_matches(':').trim();
                content_length = value_part.parse().ok();
            }
        }

        // Now read exactly Content-Length bytes
        let len = content_length.ok_or("Missing Content-Length header")?;
        let mut buffer = vec![0u8; len];
        self.reader
            .read_exact(&mut buffer)
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        let response: Value = serde_json::from_slice(&buffer)
            .map_err(|e| format!("Failed to parse response JSON: {}", e))?;
        Ok(response.get("result").cloned())
    }

    fn send_notification(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::io::Write;

        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params.unwrap_or(json!({}))
        });

        let content = serde_json::to_string(&notification)
            .map_err(|e| format!("Failed to serialize notification: {}", e))?;
        let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

        self.writer
            .write_all(message.as_bytes())
            .map_err(|e| format!("Failed to write notification: {}", e))?;
        self.writer.flush().map_err(|e| format!("Failed to flush writer: {}", e))?;
        Ok(())
    }

    fn open_file(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read fixture file {}: {}", path, e))?;
        let canonical_path = std::fs::canonicalize(path)
            .map_err(|e| format!("Failed to canonicalize path {}: {}", path, e))?;
        let uri = path_to_uri(&canonical_path)?;

        self.send_notification(
            "textDocument/didOpen",
            Some(json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            })),
        )?;
        Ok(())
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        let _ = self.server.kill();
    }
}

// ===================== Golden Tests =====================

#[test]
fn test_hover_golden() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = TestContext::new()?;
    let fixture = "tests/fixtures/hover_test.pl";
    ctx.open_file(fixture)?;

    // Test hover on custom function
    let canonical_fixture = std::fs::canonicalize(fixture)
        .map_err(|e| format!("Failed to canonicalize fixture path: {}", e))?;
    let uri = path_to_uri(&canonical_fixture)?;

    let hover = ctx.send_request(
        "textDocument/hover",
        Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 10, "character": 15 }  // On 'calculate_sum'
        })),
    )?;

    assert_hover_has_text(&hover);

    // Snapshot the hover content
    if let Some(h) = hover {
        let content = h
            .get("contents")
            .and_then(|c| c.get("value"))
            .and_then(|v| v.as_str())
            .or_else(|| h.get("contents").and_then(|c| c.as_str()));

        if let Some(text) = content {
            // In a real implementation, compare with stored snapshot
            assert!(
                text.contains("calculate_sum") || text.contains("sub"),
                "Hover should mention the function name or type"
            );
        }
    }

    // Test hover on built-in function
    let hover_builtin = ctx.send_request(
        "textDocument/hover",
        Some(json!({
            "textDocument": { "uri": path_to_uri(&canonical_fixture)? },
            "position": { "line": 16, "character": 20 }  // On 'join'
        })),
    )?;

    assert_hover_has_text(&hover_builtin);
    Ok(())
}

#[test]
fn test_diagnostics_golden() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = TestContext::new()?;
    let fixture = "tests/fixtures/diagnostics_test.pl";
    ctx.open_file(fixture)?;

    // Minimal wait for diagnostics
    std::thread::sleep(std::time::Duration::from_millis(20));

    // In a real implementation, we'd capture diagnostics via the publishDiagnostics notification
    // For now, we'll test that the file opens without crashing

    // Test that we can still get hover despite errors
    let canonical_fixture = std::fs::canonicalize(fixture)
        .map_err(|e| format!("Failed to canonicalize fixture path: {}", e))?;
    let uri = path_to_uri(&canonical_fixture)?;

    let hover = ctx.send_request(
        "textDocument/hover",
        Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 7, "character": 10 }  // On undefined_var
        })),
    )?;

    // Should handle gracefully even with syntax errors
    assert!(hover.is_none(), "Should handle invalid code gracefully");
    Ok(())
}

#[test]
fn test_completion_golden() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = TestContext::new()?;
    let fixture = "tests/fixtures/completion_test.pl";
    ctx.open_file(fixture)?;

    // Test variable completion
    let canonical_fixture = std::fs::canonicalize(fixture)
        .map_err(|e| format!("Failed to canonicalize fixture path: {}", e))?;
    let uri = path_to_uri(&canonical_fixture)?;

    let completion = ctx.send_request(
        "textDocument/completion",
        Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 31, "character": 14 }  // After '$us' in comment
        })),
    )?;

    assert_completion_has_items(&completion);

    // Verify specific completions are present
    if let Some(comp) = completion {
        let items = if let Some(arr) = comp.as_array() {
            arr.clone()
        } else if let Some(obj) = comp.as_object() {
            obj.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default()
        } else {
            vec![]
        };

        // Check for expected completions
        let labels: Vec<String> = items
            .iter()
            .filter_map(|item| item.get("label").and_then(|l| l.as_str()))
            .map(|s| s.to_string())
            .collect();

        // In a real snapshot test, we'd compare the exact list
        assert!(labels.iter().any(|l| l.contains("user_name")), "Should complete $user_name");
        assert!(labels.iter().any(|l| l.contains("user_age")), "Should complete $user_age");
    }
    Ok(())
}

#[test]
fn test_semantic_tokens_golden() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = TestContext::new()?;
    let fixture = "tests/fixtures/semantic_test.pl";
    ctx.open_file(fixture)?;

    // Request semantic tokens
    let canonical_fixture = std::fs::canonicalize(fixture)
        .map_err(|e| format!("Failed to canonicalize fixture path: {}", e))?;
    let uri = path_to_uri(&canonical_fixture)?;

    let tokens = ctx.send_request(
        "textDocument/semanticTokens/full",
        Some(json!({
            "textDocument": { "uri": uri }
        })),
    )?;

    if let Some(t) = tokens {
        let data = t.get("data").and_then(|d| d.as_array());
        assert!(data.is_some(), "Semantic tokens should have data array");

        // In a real snapshot test, we'd verify the exact token positions and types
        if let Some(token_data) = data {
            // Tokens come in groups of 5: [deltaLine, deltaStartChar, length, tokenType, tokenModifiers]
            assert!(token_data.len() % 5 == 0, "Token data should be multiple of 5");
            assert!(token_data.len() >= 5, "Should have at least one token");
        }
    }
    Ok(())
}

#[test]
fn test_folding_ranges_golden() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = TestContext::new()?;
    let fixture = "tests/fixtures/folding_test.pl";
    ctx.open_file(fixture)?;

    // Request folding ranges
    let canonical_fixture = std::fs::canonicalize(fixture)
        .map_err(|e| format!("Failed to canonicalize fixture path: {}", e))?;
    let uri = path_to_uri(&canonical_fixture)?;

    let ranges = ctx.send_request(
        "textDocument/foldingRange",
        Some(json!({
            "textDocument": { "uri": uri }
        })),
    )?;

    if let Some(r) = ranges {
        assert_folding_ranges_valid(&Some(r.clone()));

        // Verify specific folds exist
        let ranges_arr = r.as_array().ok_or("Folding ranges should be array")?;

        // Should have folds for subroutines (lines 11, 18) and control structures (line 28)
        let sub_folds = ranges_arr
            .iter()
            .filter(|r| {
                let start = r.get("startLine").and_then(|v| v.as_u64()).unwrap_or(0);
                start == 10 || start == 17 || start == 27 // 0-indexed line numbers of foldable regions
            })
            .count();

        assert!(sub_folds > 0, "Should have folding ranges for subroutines");
    }
    Ok(())
}

// ===================== Edit Loop Fuzzing =====================

#[test]
fn test_edit_loop_robustness() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = TestContext::new()?;

    // Create a test document with various Perl constructs
    let test_content = r#"#!/usr/bin/perl
my $x = "test";
my @arr = (1, 2, 3);
sub test { print "hello" }
"#;

    ctx.send_notification(
        "textDocument/didOpen",
        Some(json!({
            "textDocument": {
                "uri": "file:///tmp/fuzz_test.pl",
                "languageId": "perl",
                "version": 1,
                "text": test_content
            }
        })),
    )?;

    // Simulate rapid edits around quotes and braces
    let edits = [
        // Insert quote in string
        (1, 10, "\""),
        // Delete closing brace
        (3, 25, ""),
        // Add random characters
        (2, 15, "xyz"),
        // Insert newline in string
        (1, 12, "\n"),
        // Delete opening paren
        (2, 14, ""),
    ];

    for (version, (line, char, text)) in edits.iter().enumerate() {
        ctx.send_notification(
            "textDocument/didChange",
            Some(json!({
                "textDocument": {
                    "uri": "file:///tmp/fuzz_test.pl",
                    "version": version + 2
                },
                "contentChanges": [{
                    "range": {
                        "start": { "line": line, "character": char },
                        "end": { "line": line, "character": char }
                    },
                    "text": text
                }]
            })),
        )?;

        // Try to trigger operations on potentially invalid code
        let _ = ctx.send_request(
            "textDocument/hover",
            Some(json!({
                "textDocument": { "uri": "file:///tmp/fuzz_test.pl" },
                "position": { "line": line, "character": char }
            })),
        );

        // Server should not crash or hang - minimal delay
        std::thread::sleep(std::time::Duration::from_millis(2));
    }

    // If we got here without hanging, the test passes
    // Server survived rapid edit fuzzing
    Ok(())
}
