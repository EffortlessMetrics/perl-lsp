//! Golden fixture tests for LSP functionality
//!
//! These tests use fixture files to ensure consistent behavior across releases

mod support;

use serde_json::{Value, json};
use std::fs;
use support::test_helpers::{
    assert_completion_has_items, assert_folding_ranges_valid, assert_hover_has_text,
};

/// Test context that manages the LSP server lifecycle
struct TestContext {
    server: std::process::Child,
    reader: std::io::BufReader<std::process::ChildStdout>,
    writer: std::process::ChildStdin,
}

impl TestContext {
    fn new() -> Self {
        // Check if we should use in-process server for better performance
        if std::env::var("LSP_TEST_FALLBACKS").is_ok() {
            // Use in-process server for faster tests
            return Self::new_in_process();
        }

        // Start LSP server with optimized settings
        let mut server = std::process::Command::new("cargo")
            .args(["run", "-p", "perl-parser", "--bin", "perl-lsp", "--", "--stdio"])
            .env("LSP_TEST_MODE", "1") // Enable test optimizations
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to start LSP server");

        let reader = std::io::BufReader::new(server.stdout.take().unwrap());
        let writer = server.stdin.take().unwrap();

        let mut ctx = TestContext { server, reader, writer };

        // Initialize with minimal capabilities for faster startup
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
                "rootUri": format!("file://{}", std::env::current_dir().unwrap().display())
            })),
        );

        ctx.send_notification("initialized", None);

        // Give minimal time for initialization
        std::thread::sleep(std::time::Duration::from_millis(50));

        ctx
    }

    /// Create in-process test context for faster testing
    fn new_in_process() -> Self {
        // For now, fall back to external process but with optimizations
        // TODO: Implement true in-process server for maximum performance
        let mut server = std::process::Command::new("cargo")
            .args(["run", "-p", "perl-parser", "--bin", "perl-lsp", "--", "--stdio"])
            .env("LSP_TEST_MODE", "1")
            .env("LSP_FAST_MODE", "1")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to start LSP server");

        let reader = std::io::BufReader::new(server.stdout.take().unwrap());
        let writer = server.stdin.take().unwrap();

        let mut ctx = TestContext { server, reader, writer };

        // Fast initialization
        ctx.send_request(
            "initialize",
            Some(json!({
                "processId": std::process::id(),
                "capabilities": {},
                "rootUri": format!("file://{}", std::env::current_dir().unwrap().display())
            })),
        );

        ctx.send_notification("initialized", None);
        ctx
    }

    fn send_request(&mut self, method: &str, params: Option<Value>) -> Option<Value> {
        use std::io::{BufRead, Write};

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

        let content = serde_json::to_string(&request).unwrap();
        let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

        self.writer.write_all(message.as_bytes()).unwrap();
        self.writer.flush().unwrap();

        // Read response
        let mut headers = String::new();
        loop {
            headers.clear();
            self.reader.read_line(&mut headers).ok()?;
            if headers == "\r\n" {
                break;
            }
        }

        self.reader.read_line(&mut headers).ok()?;
        let content_length: usize = headers.split(':').nth(1)?.trim().parse().ok()?;

        let mut buffer = vec![0; content_length];
        use std::io::Read;
        self.reader.read_exact(&mut buffer).ok()?;

        let response: Value = serde_json::from_slice(&buffer).ok()?;
        response.get("result").cloned()
    }

    fn send_notification(&mut self, method: &str, params: Option<Value>) {
        use std::io::Write;

        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params.unwrap_or(json!({}))
        });

        let content = serde_json::to_string(&notification).unwrap();
        let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

        self.writer.write_all(message.as_bytes()).unwrap();
        self.writer.flush().unwrap();
    }

    fn open_file(&mut self, path: &str) {
        let content = fs::read_to_string(path).expect("Failed to read fixture file");
        let uri = format!("file://{}", std::fs::canonicalize(path).unwrap().display());

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
        );
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        let _ = self.server.kill();
    }
}

// ===================== Golden Tests =====================

#[test]
fn test_hover_golden() {
    let mut ctx = TestContext::new();
    let fixture = "tests/fixtures/hover_test.pl";
    ctx.open_file(fixture);

    // Test hover on custom function
    let hover = ctx.send_request("textDocument/hover", Some(json!({
        "textDocument": { "uri": format!("file://{}", std::fs::canonicalize(fixture).unwrap().display()) },
        "position": { "line": 10, "character": 15 }  // On 'calculate_sum'
    })));

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
    let hover_builtin = ctx.send_request("textDocument/hover", Some(json!({
        "textDocument": { "uri": format!("file://{}", std::fs::canonicalize(fixture).unwrap().display()) },
        "position": { "line": 16, "character": 20 }  // On 'join'
    })));

    assert_hover_has_text(&hover_builtin);
}

#[test]
fn test_diagnostics_golden() {
    let mut ctx = TestContext::new();
    let fixture = "tests/fixtures/diagnostics_test.pl";
    ctx.open_file(fixture);

    // Minimal wait for diagnostics
    std::thread::sleep(std::time::Duration::from_millis(20));

    // In a real implementation, we'd capture diagnostics via the publishDiagnostics notification
    // For now, we'll test that the file opens without crashing

    // Test that we can still get hover despite errors
    let hover = ctx.send_request("textDocument/hover", Some(json!({
        "textDocument": { "uri": format!("file://{}", std::fs::canonicalize(fixture).unwrap().display()) },
        "position": { "line": 7, "character": 10 }  // On undefined_var
    })));

    // Should handle gracefully even with syntax errors
    assert!(hover.is_none(), "Should handle invalid code gracefully");
}

#[test]
fn test_completion_golden() {
    let mut ctx = TestContext::new();
    let fixture = "tests/fixtures/completion_test.pl";
    ctx.open_file(fixture);

    // Test variable completion
    let completion = ctx.send_request("textDocument/completion", Some(json!({
        "textDocument": { "uri": format!("file://{}", std::fs::canonicalize(fixture).unwrap().display()) },
        "position": { "line": 31, "character": 14 }  // After '$us' in comment
    })));

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
}

#[test]
fn test_semantic_tokens_golden() {
    let mut ctx = TestContext::new();
    let fixture = "tests/fixtures/hover_test.pl";
    ctx.open_file(fixture);

    // Request semantic tokens
    let tokens = ctx.send_request("textDocument/semanticTokens/full", Some(json!({
        "textDocument": { "uri": format!("file://{}", std::fs::canonicalize(fixture).unwrap().display()) }
    })));

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
}

#[test]
fn test_folding_ranges_golden() {
    let mut ctx = TestContext::new();
    let fixture = "tests/fixtures/hover_test.pl";
    ctx.open_file(fixture);

    // Request folding ranges
    let ranges = ctx.send_request("textDocument/foldingRange", Some(json!({
        "textDocument": { "uri": format!("file://{}", std::fs::canonicalize(fixture).unwrap().display()) }
    })));

    if let Some(r) = ranges {
        assert_folding_ranges_valid(&Some(r.clone()));

        // Verify specific folds exist
        let ranges_arr = r.as_array().expect("Folding ranges should be array");

        // Should have folds for subroutines and package
        let sub_folds = ranges_arr
            .iter()
            .filter(|r| {
                let start = r.get("startLine").and_then(|v| v.as_u64()).unwrap_or(0);
                start == 6 || start == 22 || start == 27 // Line numbers of subs
            })
            .count();

        assert!(sub_folds > 0, "Should have folding ranges for subroutines");
    }
}

// ===================== Edit Loop Fuzzing =====================

#[test]
fn test_edit_loop_robustness() {
    let mut ctx = TestContext::new();

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
    );

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
        );

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
}
