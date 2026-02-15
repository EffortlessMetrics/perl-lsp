#![allow(dead_code)] // This is a utility module used by other tests

use perl_tdd_support::{must, must_some};
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

            let obj_opt = init_params.as_object();
            assert!(
                obj_opt.is_some(),
                "Initialization parameters must be a JSON object, got: {:?}",
                init_params
            );
            if let Some(obj) = obj_opt {
                let mut params = obj.clone();
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
            }
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
        let items_opt = response["result"]["items"].as_array();
        assert!(
            items_opt.is_some(),
            "Expected diagnostic items array in response, got: {:?}",
            response["result"]
        );
        let items = items_opt.unwrap_or_else(|| unreachable!());

        let found = items.iter().any(|item| {
            item["message"].as_str().map(|msg| msg.contains(expected_message)).unwrap_or(false)
        });

        assert!(found, "Expected diagnostic containing '{}', got: {:?}", expected_message, items);
    }

    /// Assert symbol count
    pub fn assert_symbol_count(response: &Value, expected_count: usize) {
        let symbols_opt = response["result"].as_array();
        assert!(
            symbols_opt.is_some(),
            "Expected symbols array in response, got: {:?}",
            response["result"]
        );
        let symbols = symbols_opt.unwrap_or_else(|| unreachable!());
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
    use perl_tdd_support::must;
    let process = match Command::new("cargo")
        .args(["run", "-p", "perl-parser", "--bin", "perl-lsp", "--", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(proc) => proc,
        Err(e) => {
            must(Err::<(), _>(format!("Failed to start LSP server: {}", e)));
            unreachable!()
        }
    };

    TestServer { process }
}

// Send request to server via JSON-RPC
fn send_request(child: &mut Child, request: Value) {
    use perl_tdd_support::{must, must_some};
    let request_str = must(serde_json::to_string(&request));
    let length = request_str.len();

    let stdin = must_some(child.stdin.as_mut());

    must(write!(stdin, "Content-Length: {}\r\n\r\n{}", length, request_str));
    must(stdin.flush());
}

// Send notification to server
fn send_notification(child: &mut Child, notification: Value) {
    send_request(child, notification); // Notifications use same format
}

// Read response from server
fn read_response(child: &mut Child) -> Value {
    use perl_tdd_support::{must, must_some};
    let stdout = must_some(child.stdout.as_mut());
    let mut reader = BufReader::new(stdout);

    // Read headers
    let mut headers = String::new();
    loop {
        let mut line = String::new();
        must(reader.read_line(&mut line));
        if line == "\r\n" {
            break;
        }
        headers.push_str(&line);
    }

    // Parse content length
    let content_length: usize = must_some(
        headers
            .lines()
            .find(|line| line.starts_with("Content-Length:"))
            .and_then(|line| line.split(':').nth(1))
            .and_then(|len| len.trim().parse().ok()),
    );

    // Read content
    let mut content = vec![0; content_length];
    use std::io::Read;
    must(reader.read_exact(&mut content));

    must(serde_json::from_slice(&content))
}
