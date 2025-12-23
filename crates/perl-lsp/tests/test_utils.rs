#![allow(dead_code)] // This is a utility module used by other tests

/// Test utilities and helpers for LSP testing
/// Provides common functionality to reduce code duplication
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

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
        let mut server = start_lsp_server();

        // Initialize with custom params
        let init_params = self.initialization_params.unwrap_or_else(|| {
            json!({
                "rootUri": null,
                "capabilities": {
                    "textDocument": {
                        "diagnostic": { "dynamicRegistration": true }
                    }
                }
            })
        });

        if !self.workspace_folders.is_empty() {
            let folders: Vec<Value> = self
                .workspace_folders
                .iter()
                .map(|path| json!({ "uri": format!("file://{}", path), "name": path }))
                .collect();

            let mut params = init_params.as_object().unwrap().clone();
            params.insert("workspaceFolders".to_string(), folders.into());

            send_request(
                &mut server.process,
                json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": params
                }),
            );
        } else {
            send_request(
                &mut server.process,
                json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": init_params
                }),
            );
        }

        let _ = read_response(&mut server.process);
        server
    }
}

/// Test server wrapper with helper methods
pub struct TestServer {
    process: Child,
}

impl TestServer {
    /// Send a text document did open notification
    pub fn open_document(&mut self, uri: &str, content: &str) {
        send_notification(
            &mut self.process,
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
    }

    /// Send a text document did change notification
    pub fn change_document(&mut self, uri: &str, content: &str, version: i32) {
        send_notification(
            &mut self.process,
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
        send_request(
            &mut self.process,
            json!({
                "jsonrpc": "2.0",
                "id": 100,
                "method": "textDocument/diagnostic",
                "params": {
                    "textDocument": { "uri": uri }
                }
            }),
        );
        read_response(&mut self.process)
    }

    /// Request document symbols
    pub fn get_symbols(&mut self, uri: &str) -> Value {
        send_request(
            &mut self.process,
            json!({
                "jsonrpc": "2.0",
                "id": 100,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": { "uri": uri }
                }
            }),
        );
        read_response(&mut self.process)
    }

    /// Request definition at position
    pub fn get_definition(&mut self, uri: &str, line: u32, character: u32) -> Value {
        send_request(
            &mut self.process,
            json!({
                "jsonrpc": "2.0",
                "id": 100,
                "method": "textDocument/definition",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": character }
                }
            }),
        );
        read_response(&mut self.process)
    }

    /// Request references at position
    pub fn get_references(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Value {
        send_request(
            &mut self.process,
            json!({
                "jsonrpc": "2.0",
                "id": 100,
                "method": "textDocument/references",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": character },
                    "context": { "includeDeclaration": include_declaration }
                }
            }),
        );
        read_response(&mut self.process)
    }

    /// Request hover information
    pub fn get_hover(&mut self, uri: &str, line: u32, character: u32) -> Value {
        send_request(
            &mut self.process,
            json!({
                "jsonrpc": "2.0",
                "id": 100,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": character }
                }
            }),
        );
        read_response(&mut self.process)
    }

    /// Request signature help
    pub fn get_signature_help(&mut self, uri: &str, line: u32, character: u32) -> Value {
        send_request(
            &mut self.process,
            json!({
                "jsonrpc": "2.0",
                "id": 100,
                "method": "textDocument/signatureHelp",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": line, "character": character }
                }
            }),
        );
        read_response(&mut self.process)
    }

    /// Shutdown the server
    pub fn shutdown(mut self) {
        send_request(
            &mut self.process,
            json!({
                "jsonrpc": "2.0",
                "id": 999,
                "method": "shutdown"
            }),
        );
        let _ = read_response(&mut self.process);

        send_notification(
            &mut self.process,
            json!({
                "jsonrpc": "2.0",
                "method": "exit"
            }),
        );

        let _ = self.process.wait();
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        // Ensure server is terminated on drop
        let _ = self.process.kill();
    }
}

/// Test assertion helpers
pub mod assertions {
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
        let items =
            response["result"]["items"].as_array().expect("Expected diagnostic items array");

        let found = items.iter().any(|item| {
            item["message"].as_str().map(|msg| msg.contains(expected_message)).unwrap_or(false)
        });

        assert!(found, "Expected diagnostic containing '{}', got: {:?}", expected_message, items);
    }

    /// Assert symbol count
    pub fn assert_symbol_count(response: &Value, expected_count: usize) {
        let symbols = response["result"].as_array().expect("Expected symbols array");
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
                panic!("Expected definition location in response, got: {:#}", response)
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
            .unwrap_or_else(|| panic!("Expected hover content in response, got: {:#}", response));

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
            .unwrap_or_else(|| panic!("Expected hover content in response, got: {:#}", response));

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

        if target_line >= lines.len() {
            panic!(
                "Target line {} does not exist in code (total lines: {}).\nCode:\n{}",
                target_line,
                lines.len(),
                code
            );
        }

        let line = lines[target_line];
        let col = line.find(needle).unwrap_or_else(|| {
            panic!(
                "Could not find '{}' on line {}.\nLine content: '{}'\nFull code:\n{}",
                needle, target_line, line, code
            )
        });

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

// Common functions (to be imported from test files that include this module)

// Helper to start server from Child process
fn start_lsp_server() -> TestServer {
    let process = Command::new("cargo")
        .args(["run", "-p", "perl-parser", "--bin", "perl-lsp", "--", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start LSP server");

    TestServer { process }
}

// Send request to server via JSON-RPC
fn send_request(child: &mut Child, request: Value) {
    let request_str = serde_json::to_string(&request).unwrap();
    let length = request_str.len();

    let stdin = child.stdin.as_mut().unwrap();
    write!(stdin, "Content-Length: {}\r\n\r\n{}", length, request_str).unwrap();
    stdin.flush().unwrap();
}

// Send notification to server
fn send_notification(child: &mut Child, notification: Value) {
    send_request(child, notification); // Notifications use same format
}

// Read response from server
fn read_response(child: &mut Child) -> Value {
    let stdout = child.stdout.as_mut().unwrap();
    let mut reader = BufReader::new(stdout);

    // Read headers
    let mut headers = String::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if line == "\r\n" {
            break;
        }
        headers.push_str(&line);
    }

    // Parse content length
    let content_length: usize = headers
        .lines()
        .find(|line| line.starts_with("Content-Length:"))
        .and_then(|line| line.split(':').nth(1))
        .and_then(|len| len.trim().parse().ok())
        .unwrap_or(0);

    // Read content
    let mut content = vec![0; content_length];
    use std::io::Read;
    reader.read_exact(&mut content).unwrap();

    serde_json::from_slice(&content).unwrap()
}
