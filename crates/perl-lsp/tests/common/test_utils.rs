#![allow(dead_code)] // This is a utility module used by other tests

//! Test utilities and helpers for LSP testing
//!
//! This module provides a fluent API for LSP testing while delegating all
//! subprocess IO to the canonical `common` harness. This prevents the
//! "new BufReader per call" footgun and ensures consistent IO behavior.
//!
//! ## Architecture
//!
//! - `TestServer` wraps `common::LspServer` for consistent IO handling
//! - `TestServerBuilder` provides a fluent API for test setup
//! - All protocol IO goes through `common::send_request()` / `common::send_notification()`
//! - Response matching uses `common::read_response_matching()` for deterministic behavior

// Import from the parent's common module (test files must declare `mod common;` before `mod test_utils;`)
use super::LspServer;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, Instant};

// Auto-generate unique IDs for requests (separate counter from common to avoid collision)
static TEST_UTILS_NEXT_ID: AtomicI64 = AtomicI64::new(2000);

fn next_request_id() -> i64 {
    TEST_UTILS_NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Test server builder with fluent interface
pub struct TestServerBuilder {
    initialization_params: Option<Value>,
    timeout: Duration,
    workspace_folders: Vec<String>,
}

impl Default for TestServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TestServerBuilder {
    pub fn new() -> Self {
        Self {
            initialization_params: None,
            timeout: Duration::from_secs(5),
            workspace_folders: Vec::new(),
        }
    }

    pub fn with_workspace(mut self, path: &str) -> Self {
        self.workspace_folders.push(path.to_string());
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_init_params(mut self, params: Value) -> Self {
        self.initialization_params = Some(params);
        self
    }

    pub fn build(self) -> TestServer {
        // Use the canonical common harness for server startup
        let mut server = super::start_lsp_server();

        // Build initialization params
        let mut init_params = self.initialization_params.unwrap_or_else(|| {
            json!({
                "rootUri": null,
                "capabilities": {
                    "textDocument": {
                        "diagnostic": { "dynamicRegistration": true }
                    }
                }
            })
        });

        // Add workspace folders if specified
        if !self.workspace_folders.is_empty() {
            let folders: Vec<Value> = self
                .workspace_folders
                .iter()
                .map(|path| json!({ "uri": format!("file://{}", path), "name": path }))
                .collect();

            if let Some(obj) = init_params.as_object_mut() {
                obj.insert("workspaceFolders".to_string(), folders.into());
            }
        }

        // Send initialize request via common harness
        let init_id = next_request_id();
        let init_response = super::send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": init_id,
                "method": "initialize",
                "params": init_params
            }),
        );

        // Verify initialization succeeded
        if init_response.get("error").is_some() {
            eprintln!("TestServerBuilder: Initialize failed: {init_response:#}");
        }

        // Send initialized notification (required by LSP protocol)
        super::send_notification(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "method": "initialized",
                "params": {}
            }),
        );

        // Wait for index-ready notification to ensure deterministic completion behavior
        // Only wait if workspace folders were specified (semantic tests usually don't need workspace index)
        if !self.workspace_folders.is_empty() {
            super::await_index_ready(&mut server);
        } else {
            // For non-workspace tests, just do a brief quiet drain
            super::drain_until_quiet(
                &mut server,
                std::time::Duration::from_millis(50),
                std::time::Duration::from_millis(200),
            );
        }

        TestServer { server, timeout: self.timeout }
    }
}

/// Test server wrapper with helper methods
///
/// Wraps `common::LspServer` to provide a fluent testing API while
/// delegating all IO to the canonical harness with persistent reader thread.
pub struct TestServer {
    server: LspServer,
    timeout: Duration,
}

impl TestServer {
    /// Send a text document did open notification
    pub fn open_document(&mut self, uri: &str, content: &str) {
        super::send_notification(
            &mut self.server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "perl",
                        "version": 1,
                        "text": content
                    }
                }
            }),
        );
        // Brief delay to let server process the document
        std::thread::sleep(Duration::from_millis(20));
    }

    /// Send a text document did change notification
    pub fn change_document(&mut self, uri: &str, content: &str, version: i32) {
        super::send_notification(
            &mut self.server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "version": version
                    },
                    "contentChanges": [{
                        "text": content
                    }]
                }
            }),
        );
    }

    /// Request diagnostics for a document
    pub fn get_diagnostics(&mut self, uri: &str) -> Value {
        super::send_request(
            &mut self.server,
            json!({
                "jsonrpc": "2.0",
                "id": next_request_id(),
                "method": "textDocument/diagnostic",
                "params": {
                    "textDocument": { "uri": uri }
                }
            }),
        )
    }

    /// Request document symbols
    pub fn get_symbols(&mut self, uri: &str) -> Value {
        super::send_request(
            &mut self.server,
            json!({
                "jsonrpc": "2.0",
                "id": next_request_id(),
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": { "uri": uri }
                }
            }),
        )
    }

    /// Request definition at position
    pub fn get_definition(&mut self, uri: &str, line: u32, character: u32) -> Value {
        super::send_request(
            &mut self.server,
            json!({
                "jsonrpc": "2.0",
                "id": next_request_id(),
                "method": "textDocument/definition",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": character }
                }
            }),
        )
    }

    /// Request references at position
    pub fn get_references(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Value {
        super::send_request(
            &mut self.server,
            json!({
                "jsonrpc": "2.0",
                "id": next_request_id(),
                "method": "textDocument/references",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": character },
                    "context": { "includeDeclaration": include_declaration }
                }
            }),
        )
    }

    /// Request hover information
    pub fn get_hover(&mut self, uri: &str, line: u32, character: u32) -> Value {
        super::send_request(
            &mut self.server,
            json!({
                "jsonrpc": "2.0",
                "id": next_request_id(),
                "method": "textDocument/hover",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": character }
                }
            }),
        )
    }

    /// Request signature help
    pub fn get_signature_help(&mut self, uri: &str, line: u32, character: u32) -> Value {
        super::send_request(
            &mut self.server,
            json!({
                "jsonrpc": "2.0",
                "id": next_request_id(),
                "method": "textDocument/signatureHelp",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": character }
                }
            }),
        )
    }

    /// Shutdown the server gracefully
    pub fn shutdown(mut self) {
        super::shutdown_and_exit(&mut self.server);
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        // LspServer's Drop impl handles cleanup
    }
}

/// Test assertion helpers
pub mod assertions {
    #![allow(clippy::panic)]

    use serde_json::Value;

    /// Assert that the response contains no errors
    pub fn assert_no_error(response: &Value) {
        assert!(
            response.get("error").is_none(),
            "Expected no error, got: {:?}",
            response.get("error")
        );
    }

    /// Assert that diagnostics contain expected error
    pub fn assert_has_diagnostic(response: &Value, expected_message: &str) {
        let items = response["result"]["items"]
            .as_array()
            .unwrap_or_else(|| panic!("Expected diagnostic items array, got: {response:?}"));

        let found = items
            .iter()
            .any(|item| item["message"].as_str().is_some_and(|msg| msg.contains(expected_message)));

        assert!(found, "Expected diagnostic containing '{}', got: {:?}", expected_message, items);
    }

    /// Assert symbol count
    pub fn assert_symbol_count(response: &Value, expected_count: usize) {
        let symbols = response["result"]
            .as_array()
            .unwrap_or_else(|| panic!("Expected symbols array, got: {response:?}"));
        assert_eq!(
            symbols.len(),
            expected_count,
            "Expected {} symbols, got {}",
            expected_count,
            symbols.len()
        );
    }

    /// Assert that location matches expected
    pub fn assert_location(location: &Value, uri: &str, line: u32, character: u32) {
        assert_eq!(location["uri"].as_str(), Some(uri));
        assert_eq!(location["range"]["start"]["line"], line);
        assert_eq!(location["range"]["start"]["character"], character);
    }

    /// Assert that definition response contains a location at the expected position
    pub fn assert_definition_at(response: &Value, uri: &str, line: u32) {
        let (def_uri, def_line, _) =
            super::semantic::first_location(response).unwrap_or_else(|| {
                panic!("Expected definition location in response, got: {response:#}")
            });

        assert_eq!(
            def_uri, uri,
            "Definition URI mismatch.\nExpected: {}\nActual: {}\nFull response: {:#}",
            uri, def_uri, response
        );
        assert_eq!(
            def_line, line,
            "Definition line mismatch.\nExpected: {}\nActual: {}\nFull response: {:#}",
            line, def_line, response
        );
    }

    /// Assert that hover response contains expected content
    pub fn assert_hover_contains(response: &Value, expected_content: &str) {
        let content = super::semantic::hover_content(response)
            .unwrap_or_else(|| panic!("Expected hover content in response, got: {response:#}"));

        assert!(
            content.contains(expected_content),
            "Hover content does not contain expected string.\nExpected to contain: {}\nActual content: {}\nFull response: {:#}",
            expected_content,
            content,
            response
        );
    }

    /// Assert that hover response contains any of the expected strings
    pub fn assert_hover_contains_any(response: &Value, expected_strings: &[&str]) {
        let content = super::semantic::hover_content(response)
            .unwrap_or_else(|| panic!("Expected hover content in response, got: {response:#}"));

        let found = expected_strings.iter().any(|s| content.contains(s));
        assert!(
            found,
            "Hover content does not contain any expected string.\nExpected one of: {:?}\nActual content: {}\nFull response: {:#}",
            expected_strings, content, response
        );
    }
}

/// Semantic analyzer testing helpers
pub mod semantic {
    use serde_json::Value;

    /// Extract the first definition location from an LSP response.
    /// Returns (uri, line, character) for easier assertions.
    ///
    /// # Returns
    /// - `Some((uri, line, character))` if a location is found
    /// - `None` if the result is empty or malformed
    ///
    /// # Examples
    /// ```ignore
    /// let response = server.get_definition(uri, line, character);
    /// let (def_uri, def_line, def_char) = first_location(&response)
    ///     .expect("Expected to find definition");
    /// assert_eq!(def_uri, "file:///test.pl");
    /// ```
    pub fn first_location(resp: &Value) -> Option<(String, u32, u32)> {
        let arr = resp.get("result")?.as_array()?;
        let first = arr.first()?;
        let uri = first.get("uri")?.as_str()?.to_string();
        let range = first.get("range")?;
        let start = &range["start"];
        let line = start.get("line")?.as_u64()? as u32;
        let character = start.get("character")?.as_u64()? as u32;
        Some((uri, line, character))
    }

    /// Extract hover content from an LSP hover response.
    /// Returns the markdown value string for assertions.
    ///
    /// # Returns
    /// - `Some(content)` if hover content is found
    /// - `None` if the result is null or malformed
    ///
    /// # Examples
    /// ```ignore
    /// let response = server.get_hover(uri, line, character);
    /// let content = hover_content(&response)
    ///     .expect("Expected hover content");
    /// assert!(content.contains("Scalar Variable"));
    /// ```
    pub fn hover_content(resp: &Value) -> Option<String> {
        let result = resp.get("result")?;
        if result.is_null() {
            return None;
        }
        let contents = result.get("contents")?;
        let value = contents.get("value")?.as_str()?;
        Some(value.to_string())
    }

    /// Compute (line, character) for a given needle on a specific target line.
    /// This helper is resilient to whitespace changes and provides clear error messages.
    ///
    /// # Arguments
    /// - `code`: The full source code text
    /// - `needle`: The text to search for (e.g., "$x", "foo()")
    /// - `target_line`: Zero-indexed line number to search on
    ///
    /// # Returns
    /// `(line, character)` tuple suitable for LSP position
    ///
    /// # Panics
    /// Panics with a helpful message if:
    /// - The target line doesn't exist
    /// - The needle is not found on the target line
    ///
    /// # Examples
    /// ```ignore
    /// let code = "my $x = 1;\n$x + 2;\n";
    /// let (line, char) = find_pos(code, "$x", 1);  // Find $x on line 1
    /// assert_eq!(line, 1);
    /// assert_eq!(char, 0);
    /// ```
    pub fn find_pos(code: &str, needle: &str, target_line: usize) -> (u32, u32) {
        let lines: Vec<&str> = code.lines().collect();

        assert!(
            target_line < lines.len(),
            "Target line {} does not exist in code (total lines: {}).\nCode:\n{}",
            target_line,
            lines.len(),
            code
        );

        let line = lines[target_line];
        let col = line.find(needle);
        assert!(
            col.is_some(),
            "Could not find '{}' on line {}.\nLine content: '{}'\nFull code:\n{}",
            needle,
            target_line,
            line,
            code
        );
        let col = col.unwrap_or(0);

        (target_line as u32, col as u32)
    }

    /// Find position with flexible matching - searches multiple lines if not found on target.
    /// This is useful for tests that might be affected by whitespace changes.
    ///
    /// # Arguments
    /// - `code`: The full source code text
    /// - `needle`: The text to search for
    /// - `preferred_line`: Line to search first (zero-indexed)
    ///
    /// # Returns
    /// `Some((line, character))` if found, `None` otherwise
    ///
    /// # Examples
    /// ```ignore
    /// let code = "my $x = 1;\n\n$x + 2;\n";  // Extra blank line
    /// let (line, char) = find_pos_flexible(code, "$x", 1)
    ///     .expect("Should find $x somewhere");
    /// ```
    pub fn find_pos_flexible(
        code: &str,
        needle: &str,
        preferred_line: usize,
    ) -> Option<(u32, u32)> {
        let lines: Vec<&str> = code.lines().collect();

        // Try preferred line first
        if preferred_line < lines.len() {
            if let Some(col) = lines[preferred_line].find(needle) {
                return Some((preferred_line as u32, col as u32));
            }
        }

        // Search nearby lines (±2 lines from preferred)
        let start = preferred_line.saturating_sub(2);
        let end = (preferred_line + 3).min(lines.len());

        for (idx, line) in lines.iter().enumerate().take(end).skip(start) {
            if let Some(col) = line.find(needle) {
                return Some((idx as u32, col as u32));
            }
        }

        None
    }

    /// Find the nth occurrence of needle in code.
    /// Useful when the same symbol appears multiple times.
    ///
    /// # Arguments
    /// - `code`: The full source code text
    /// - `needle`: The text to search for
    /// - `occurrence`: Which occurrence to find (0-indexed)
    ///
    /// # Returns
    /// `Some((line, character))` if found, `None` if not enough occurrences
    ///
    /// # Examples
    /// ```ignore
    /// let code = "my $x = 1;\n$x + $x;\n";
    /// let (line, char) = find_nth_occurrence(code, "$x", 2)
    ///     .expect("Should find third $x");
    /// assert_eq!(line, 1);  // Second line, second $x
    /// ```
    pub fn find_nth_occurrence(code: &str, needle: &str, occurrence: usize) -> Option<(u32, u32)> {
        let mut count = 0;

        for (line_idx, line) in code.lines().enumerate() {
            let mut search_start = 0;
            while let Some(col) = line[search_start..].find(needle) {
                if count == occurrence {
                    return Some((line_idx as u32, (search_start + col) as u32));
                }
                count += 1;
                search_start += col + needle.len();
            }
        }

        None
    }
}

/// Performance testing utilities
pub mod performance {
    use super::*;

    /// Measure operation time
    pub fn measure_time<F, R>(operation: F) -> (R, Duration)
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = operation();
        (result, start.elapsed())
    }

    /// Assert operation completes within timeout
    pub fn assert_completes_within<F, R>(operation: F, max_duration: Duration) -> R
    where
        F: FnOnce() -> R,
    {
        let (result, duration) = measure_time(operation);
        assert!(
            duration <= max_duration,
            "Operation took {:?}, expected max {:?}",
            duration,
            max_duration
        );
        result
    }

    /// Run operation multiple times and get average time
    pub fn benchmark<F>(operation: F, iterations: usize) -> Duration
    where
        F: Fn(),
    {
        let mut total = Duration::ZERO;
        for _ in 0..iterations {
            let start = Instant::now();
            operation();
            total += start.elapsed();
        }
        total / iterations as u32
    }
}

/// Test data generators
pub mod generators {
    /// Generate a large Perl file for stress testing
    pub fn generate_large_file(lines: usize) -> String {
        let mut content = String::new();
        content.push_str("#!/usr/bin/perl\n");
        content.push_str("use strict;\n");
        content.push_str("use warnings;\n\n");

        for i in 0..lines {
            content.push_str(&format!("my $var_{} = {};\n", i, i));
            if i % 10 == 0 {
                content.push_str(&format!("sub function_{} {{\n", i));
                content.push_str(&format!("    return $var_{};\n", i));
                content.push_str("}\n\n");
            }
        }

        content
    }

    /// Generate deeply nested code
    pub fn generate_nested_code(depth: usize) -> String {
        let mut content = String::new();
        for i in 0..depth {
            content.push_str(&"  ".repeat(i));
            content.push_str("if ($condition) {\n");
        }
        content.push_str(&"  ".repeat(depth));
        content.push_str("print 'deep';\n");
        for i in (0..depth).rev() {
            content.push_str(&"  ".repeat(i));
            content.push_str("}\n");
        }
        content
    }

    /// Generate code with many symbols
    pub fn generate_symbols(count: usize) -> String {
        let mut content = String::new();
        for i in 0..count {
            content.push_str(&format!("my $scalar_{} = {};\n", i, i));
            content.push_str(&format!("my @array_{} = ({});\n", i, i));
            content.push_str(&format!("my %hash_{} = (key => {});\n", i, i));
            content.push_str(&format!("sub sub_{} {{ return {}; }}\n", i, i));
        }
        content
    }
}
