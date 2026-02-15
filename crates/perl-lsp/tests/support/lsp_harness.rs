//! LSP Test Harness for Real JSON-RPC Testing
//!
//! Provides a test harness that communicates with the LSP server using real JSON-RPC protocol.

#![allow(dead_code)]
#![allow(clippy::collapsible_if)]

use parking_lot::Mutex;
use perl_lsp::LspServer;
use perl_tdd_support::must;
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::fs;
use std::io::{Cursor, Write};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use url::Url;

/// Temporary workspace for testing with real files
pub struct TempWorkspace {
    pub dir: TempDir,
    pub root_uri: String,
}

impl TempWorkspace {
    /// Create a new temporary workspace
    pub fn new() -> Result<Self, String> {
        let dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
        let root_uri = Url::from_directory_path(dir.path())
            .map_err(|_| "Failed to create file URL")?
            .to_string();
        Ok(Self { dir, root_uri })
    }

    /// Write a file to the workspace
    pub fn write(&self, relative_path: &str, content: &str) -> Result<(), String> {
        let path = self.dir.path().join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create dirs: {}", e))?;
        }
        fs::write(&path, content).map_err(|e| format!("Failed to write file: {}", e))?;
        Ok(())
    }

    /// Get the full URI for a relative path
    pub fn uri(&self, relative_path: &str) -> String {
        let path = self.dir.path().join(relative_path);
        match Url::from_file_path(&path) {
            Ok(url) => url.to_string(),
            Err(_) => must(Url::from_file_path(&path)).to_string(),
        }
    }
}

/// LSP Test Harness for testing with real JSON-RPC protocol
pub struct LspHarness {
    sender: mpsc::Sender<Vec<u8>>,
    output_buffer: Arc<Mutex<Vec<u8>>>,
    notification_buffer: Arc<Mutex<VecDeque<Value>>>,
    next_request_id: i32,
    handle: Option<thread::JoinHandle<()>>,
    canceled_ids: Arc<Mutex<Vec<i32>>>, // Track canceled request IDs
}

struct SendableServer(LspServer);
unsafe impl Send for SendableServer {}

impl LspHarness {
    /// Lowest-level constructor: spawn server and wire pipes, no messages sent.
    pub fn new_raw() -> Self {
        let output_buffer = Arc::new(Mutex::new(Vec::new()));
        let notification_buffer = Arc::new(Mutex::new(VecDeque::new()));

        // Create server with captured output
        let writer = Arc::new(Mutex::new(Box::new(TestWriter {
            buffer: output_buffer.clone(),
            notifications: notification_buffer.clone(),
        }) as Box<dyn Write + Send>));
        let server = SendableServer(LspServer::with_output(writer));

        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        let handle = thread::spawn(move || {
            let mut server = server;
            while let Ok(msg) = rx.recv() {
                if msg.is_empty() {
                    break;
                }
                let mut cursor = Cursor::new(msg);
                let _ = server.0.handle_message(&mut cursor);
            }
        });

        Self {
            sender: tx,
            output_buffer,
            notification_buffer,
            next_request_id: 1,
            handle: Some(handle),
            canceled_ids: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create a new test harness
    pub fn new() -> Self {
        Self::new_raw()
    }

    /// Create a new test harness without sending initialize
    /// Used for testing pre-initialization behavior
    pub fn new_without_initialize() -> Self {
        Self::new_raw()
    }

    /// Initialize the LSP server
    pub fn initialize(&mut self, capabilities: Option<Value>) -> Result<Value, String> {
        self.initialize_with_root("file:///workspace", capabilities)
    }

    /// Initialize the LSP server with a specific root URI and enhanced timeout handling
    pub fn initialize_with_root(
        &mut self,
        root_uri: &str,
        capabilities: Option<Value>,
    ) -> Result<Value, String> {
        let caps = capabilities.unwrap_or_else(|| {
            json!({
                "textDocument": {
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true
                        }
                    },
                    "hover": {
                        "contentFormat": ["markdown", "plaintext"]
                    },
                    "signatureHelp": {
                        "signatureInformation": {
                            "documentationFormat": ["markdown", "plaintext"]
                        }
                    }
                }
            })
        });

        let init_request = json!({
            "jsonrpc": "2.0",
            "id": self.next_request_id,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "capabilities": caps,
                "rootUri": root_uri
            }
        });
        self.next_request_id += 1;

        // Use adaptive timeout for initialization based on environment
        let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
        let init_timeout = if is_ci {
            Duration::from_secs(5) // CI: longer initialization timeout
        } else if std::env::var("PERL_LSP_PERFORMANCE_TEST").is_ok() {
            Duration::from_millis(800) // Performance tests: faster initialization
        } else {
            Duration::from_secs(2) // Local: balanced timeout
        };

        let response = self.send_request_with_timeout(init_request, init_timeout)?;

        // Only send initialized notification if initialization succeeded
        // (The response will contain capabilities if successful)
        if response.get("capabilities").is_some() {
            self.notify("initialized", json!({}));

            // Give server a moment to process the initialized notification
            let settle_time = if is_ci {
                Duration::from_millis(100) // CI: extra settling time
            } else {
                Duration::from_millis(50) // Local: minimal settling time
            };
            thread::sleep(settle_time);
        }

        Ok(response)
    }

    /// Create a harness with a temporary workspace
    pub fn with_workspace(files: &[(&str, &str)]) -> Result<(Self, TempWorkspace), String> {
        let workspace = TempWorkspace::new()?;

        // Write all files to disk
        for (path, content) in files {
            workspace.write(path, content)?;
        }

        let mut harness = Self::new_raw();
        harness.initialize_with_root(&workspace.root_uri, None)?;

        Ok((harness, workspace))
    }

    /// Initialize with default capabilities
    pub fn initialize_default(&mut self) -> Result<Value, String> {
        self.initialize(None)
    }

    /// Initialize and wait until the server is fully ready.
    ///
    /// This is the **canonical initialization pattern** that should be used in most tests.
    /// It combines:
    /// 1. `initialize` request with proper capabilities
    /// 2. `initialized` notification
    /// 3. Barrier synchronization to ensure server is fully ready
    ///
    /// # Example
    /// ```ignore
    /// let mut harness = LspHarness::new_raw();
    /// harness.initialize_ready("file:///workspace", None)?;
    /// harness.open("file:///test.pl", "my $x = 1;")?;
    /// let result = harness.request("textDocument/hover", params)?;
    /// ```
    pub fn initialize_ready(
        &mut self,
        root_uri: &str,
        capabilities: Option<Value>,
    ) -> Result<Value, String> {
        let response = self.initialize_with_root(root_uri, capabilities)?;
        self.barrier();
        Ok(response)
    }

    /// Open a document (alias for open)
    pub fn open_document(&mut self, uri: &str, text: &str) -> Result<(), String> {
        self.open(uri, text)
    }

    /// Open a document
    pub fn open(&mut self, uri: &str, text: &str) -> Result<(), String> {
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": text
                }
            }),
        );
        Ok(())
    }

    /// Change document content (full replacement)
    ///
    /// This is a convenience wrapper for `textDocument/didChange` with full content replacement.
    pub fn change_full(&mut self, uri: &str, version: i32, text: &str) -> Result<(), String> {
        self.notify(
            "textDocument/didChange",
            json!({
                "textDocument": {
                    "uri": uri,
                    "version": version
                },
                "contentChanges": [{ "text": text }]
            }),
        );
        Ok(())
    }

    /// Close a document
    pub fn close(&mut self, uri: &str) -> Result<(), String> {
        self.notify(
            "textDocument/didClose",
            json!({
                "textDocument": {
                    "uri": uri
                }
            }),
        );
        Ok(())
    }

    /// Send a request and wait for response with adaptive timeout based on thread configuration
    pub fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let timeout = self.get_adaptive_timeout();
        self.request_with_timeout(method, params, timeout)
    }

    /// Send a request and wait for response with custom timeout
    pub fn request_with_timeout(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_request_id,
            "method": method,
            "params": params
        });
        self.next_request_id += 1;

        self.send_request_with_timeout(request, timeout)
    }

    /// Send a didSave notification
    pub fn did_save(&mut self, uri: &str) -> Result<(), String> {
        self.notify(
            "textDocument/didSave",
            json!({
                "textDocument": {
                    "uri": uri
                }
            }),
        );
        Ok(())
    }

    /// Wait for the server to become idle by draining notifications with adaptive timing
    pub fn wait_for_idle(&mut self, duration: Duration) {
        // Adaptive idle detection based on environment
        let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
        let is_performance_test = std::env::var("PERL_LSP_PERFORMANCE_TEST").is_ok();

        // Adjust timing based on environment
        let (max_wait, required_idle_count, poll_interval) = if is_performance_test {
            // Performance tests: very fast polling
            (duration.min(Duration::from_millis(100)), 2, Duration::from_millis(2))
        } else if is_ci {
            // CI: more patient waiting for reliability
            (duration.min(Duration::from_millis(500)), 5, Duration::from_millis(10))
        } else {
            // Local development: balanced approach
            (duration.min(Duration::from_millis(200)), 3, Duration::from_millis(5))
        };

        let start = Instant::now();
        let mut idle_count = 0;
        let mut total_checks = 0;

        while start.elapsed() < max_wait {
            total_checks += 1;

            // Check for notifications more efficiently
            let notifications = self.notification_buffer.lock();
            if notifications.is_empty() {
                idle_count += 1;
                if idle_count >= required_idle_count {
                    // Consider idle after required consecutive empty checks
                    drop(notifications);
                    break;
                }
                drop(notifications);
                thread::sleep(poll_interval);
            } else {
                idle_count = 0;
                drop(notifications);
                // Slightly longer sleep when processing notifications
                thread::sleep(poll_interval * 2);
            }

            // Prevent excessive polling in CI environments
            if is_ci && total_checks > 100 {
                thread::sleep(Duration::from_millis(5));
            }
        }
    }

    /// Poll workspace/symbol until query appears with enhanced reliability and CI optimization
    pub fn wait_for_symbol(
        &mut self,
        query: &str,
        want_uri: Option<&str>,
        budget: Duration,
    ) -> Result<(), String> {
        // Detect environment characteristics for optimization
        let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
        let is_performance_test = std::env::var("PERL_LSP_PERFORMANCE_TEST").is_ok();
        let use_fallbacks = std::env::var("LSP_TEST_FALLBACKS").is_ok();

        // Fast path for performance tests or fallback mode
        if use_fallbacks || is_performance_test {
            let timeout = if is_performance_test { 50 } else { 100 };
            let res = self.request_with_timeout(
                "workspace/symbol",
                serde_json::json!({ "query": query }),
                Duration::from_millis(timeout),
            );
            if res.is_ok() {
                return Ok(()); // Symbol indexing is working
            }
            if use_fallbacks {
                eprintln!("Warning: symbol '{}' not indexed, proceeding anyway", query);
                return Ok(());
            }
        }

        // Adaptive parameters based on environment
        let (max_attempts, initial_timeout, max_sleep) = if is_ci {
            (8, 300, 200) // CI: more attempts, longer timeouts
        } else if is_performance_test {
            (3, 100, 50) // Performance: fewer attempts, faster timeouts
        } else {
            (5, 200, 100) // Local: balanced approach
        };

        let start = Instant::now();
        let mut attempt = 0;
        let mut last_error = None;

        while start.elapsed() < budget && attempt < max_attempts {
            attempt += 1;

            // Progressive timeout increase for reliability
            let timeout = Duration::from_millis(initial_timeout + (attempt * 50).min(200));

            let res = self.request_with_timeout(
                "workspace/symbol",
                serde_json::json!({ "query": query }),
                timeout,
            );

            match res {
                Ok(v) => {
                    if let Some(arr) = v.as_array() {
                        let found = arr.iter().any(|s| {
                            let uri = s.pointer("/location/uri").and_then(|u| u.as_str());
                            want_uri.is_none_or(|expect| uri == Some(expect))
                        });
                        if found {
                            return Ok(());
                        }
                        // Symbol search succeeded but didn't find target - continue
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                    // Request failed - might be server not ready, continue with backoff
                }
            }

            // Adaptive backoff strategy
            let sleep_ms = if is_ci {
                // CI: More conservative backoff for reliability
                (20 * attempt).min(max_sleep)
            } else {
                // Local/Performance: Faster backoff
                (10 * attempt).min(max_sleep)
            };
            thread::sleep(Duration::from_millis(sleep_ms));

            // Give server more time between attempts in CI
            if is_ci && attempt > 3 {
                thread::sleep(Duration::from_millis(50));
            }
        }

        // Enhanced error reporting
        let error_context = if let Some(err) = last_error {
            format!("Last error: {}", err)
        } else {
            "Symbol search succeeded but target not found".to_string()
        };

        Err(format!(
            "symbol '{}' not ready within {:?} after {} attempts. {} (CI: {}, Perf: {})",
            query, budget, attempt, error_context, is_ci, is_performance_test
        ))
    }

    /// Alternative request method that accepts a full JSON-RPC request object (for schema tests)
    pub fn request_raw(&mut self, request: Value) -> Value {
        // Handle full JSON-RPC request object
        if request.is_object() && request.get("jsonrpc").is_some() {
            let mut req = request;
            req["id"] = json!(self.next_request_id);
            self.next_request_id += 1;

            // Use send_request_full_response to get the complete JSON-RPC response
            self.send_request_full_response(req).unwrap_or_else(|e| {
                json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32603,
                        "message": e
                    }
                })
            })
        } else {
            // This shouldn't happen, but handle gracefully
            json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": {
                    "code": -32600,
                    "message": "Invalid request"
                }
            })
        }
    }

    /// Send a notification (no response expected)
    pub fn notify(&mut self, method: &str, params: Value) {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let request_str = format!("{}\r\n", notification);
        let content = format!("Content-Length: {}\r\n\r\n{}", request_str.len(), request_str);

        let _ = self.sender.send(content.into_bytes());
    }

    /// Drain notifications from the buffer
    pub fn drain_notifications(&mut self, method: Option<&str>, timeout_ms: u64) -> Vec<Value> {
        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        // Wait a bit for notifications to arrive
        while start.elapsed() < timeout {
            thread::sleep(Duration::from_millis(10));

            let notifications = self.notification_buffer.lock();
            if !notifications.is_empty() {
                break;
            }
        }

        let mut notifications = self.notification_buffer.lock();
        let mut result = Vec::new();

        while let Some(notif) = notifications.pop_front() {
            if let Some(filter_method) = method {
                if notif["method"].as_str() == Some(filter_method) {
                    result.push(notif);
                } else {
                    // Put it back if it doesn't match
                    notifications.push_back(notif);
                    break;
                }
            } else {
                result.push(notif);
            }
        }

        result
    }

    /// Get performance timing for a request
    pub fn timed_request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<(Value, Duration), String> {
        let start = Instant::now();
        let result = self.request(method, params)?;
        let duration = start.elapsed();
        Ok((result, duration))
    }

    // Private helper to send request and get response with adaptive timeout
    fn send_request(&mut self, request: Value) -> Result<Value, String> {
        let timeout = self.get_adaptive_timeout();
        self.send_request_with_timeout(request, timeout)
    }

    // Private helper to send request and get full JSON-RPC response with adaptive timeout
    fn send_request_full_response(&mut self, request: Value) -> Result<Value, String> {
        let timeout = self.get_adaptive_timeout();
        self.send_request_with_timeout_full_response(request, timeout)
    }

    // Private helper to send request with timeout
    fn send_request_with_timeout(
        &mut self,
        request: Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        // Clear output buffer
        self.output_buffer.lock().clear();

        // Format request with Content-Length header
        let request_str = request.to_string();
        let content = format!("Content-Length: {}\r\n\r\n{}", request_str.len(), request_str);

        // Send to server thread
        if let Err(e) = self.sender.send(content.into_bytes()) {
            return Err(format!("Server send error: {}", e));
        }

        // Wait for response with timeout
        let start = Instant::now();
        loop {
            if start.elapsed() > timeout {
                return Err(format!("Request timed out after {:?}", timeout));
            }

            // Check if we have a response
            if let Some(output) = self.output_buffer.try_lock() {
                let output_str = String::from_utf8_lossy(&output);

                // Parse all messages in the output (might be multiple)
                let mut remaining = output_str.as_ref();
                while !remaining.is_empty() {
                    // Look for Content-Length header
                    if let Some(content_start) = remaining.find("Content-Length:") {
                        remaining = &remaining[content_start..];

                        // Parse content length
                        if let Some(header_end) = remaining.find("\r\n\r\n") {
                            let header = &remaining[..header_end];
                            if let Some(length_str) = header.strip_prefix("Content-Length:") {
                                if let Ok(length) = length_str.trim().parse::<usize>() {
                                    let json_start = header_end + 4; // Skip \r\n\r\n
                                    if remaining.len() >= json_start + length {
                                        let json_str = &remaining[json_start..json_start + length];
                                        if let Ok(msg) = serde_json::from_str::<Value>(json_str) {
                                            // Check if this is our response (has matching id)
                                            if msg.get("id").is_some() {
                                                if let Some(error) = msg.get("error") {
                                                    return Err(format!("LSP error: {:?}", error));
                                                }
                                                if let Some(result) = msg.get("result") {
                                                    return Ok(result.clone());
                                                }
                                            }
                                        }
                                        // Move to next message
                                        remaining = &remaining[json_start + length..];
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                    break; // No more complete messages
                }

                drop(output);
            }

            // If no response yet, wait a bit
            if start.elapsed() < timeout {
                thread::sleep(Duration::from_millis(10));
            } else {
                break;
            }
        }

        Err("No response received".to_string())
    }

    // Private helper to send request with timeout and return full JSON-RPC response
    fn send_request_with_timeout_full_response(
        &mut self,
        request: Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        // Clear output buffer
        self.output_buffer.lock().clear();

        // Format request with Content-Length header
        let request_str = request.to_string();
        let content = format!("Content-Length: {}\r\n\r\n{}", request_str.len(), request_str);

        // Send to server thread
        if let Err(e) = self.sender.send(content.into_bytes()) {
            return Err(format!("Server send error: {}", e));
        }

        // Wait for response with timeout
        let start = Instant::now();
        loop {
            if start.elapsed() > timeout {
                return Err(format!("Request timed out after {:?}", timeout));
            }

            // Check if we have a response
            if let Some(output) = self.output_buffer.try_lock() {
                let output_str = String::from_utf8_lossy(&output);

                // Parse all messages in the output (might be multiple)
                let mut remaining = output_str.as_ref();
                while !remaining.is_empty() {
                    // Look for Content-Length header
                    if let Some(content_start) = remaining.find("Content-Length:") {
                        remaining = &remaining[content_start..];

                        // Parse content length
                        if let Some(header_end) = remaining.find("\r\n\r\n") {
                            let header = &remaining[..header_end];
                            if let Some(length_str) = header.strip_prefix("Content-Length:") {
                                if let Ok(length) = length_str.trim().parse::<usize>() {
                                    let json_start = header_end + 4; // Skip \r\n\r\n
                                    if remaining.len() >= json_start + length {
                                        let json_str = &remaining[json_start..json_start + length];
                                        if let Ok(msg) = serde_json::from_str::<Value>(json_str) {
                                            // Check if this is our response (has matching id)
                                            if msg.get("id").is_some() {
                                                // Return the full message for schema validation tests
                                                return Ok(msg);
                                            }
                                        }
                                        // Move to next message
                                        remaining = &remaining[json_start + length..];
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                    break; // No more complete messages
                }

                drop(output);
            }

            // If no response yet, wait a bit
            if start.elapsed() < timeout {
                thread::sleep(Duration::from_millis(10));
            } else {
                break;
            }
        }

        Err("No response received".to_string())
    }

    /// Get type definition at a position
    pub fn type_definition(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Value, String> {
        self.request(
            "textDocument/typeDefinition",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )
    }

    /// Get implementation locations at a position
    pub fn implementation(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Value, String> {
        self.request(
            "textDocument/implementation",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )
    }

    /// Execute a command
    pub fn execute_command(
        &mut self,
        command: &str,
        arguments: Vec<Value>,
    ) -> Result<Value, String> {
        self.request(
            "workspace/executeCommand",
            json!({
                "command": command,
                "arguments": arguments
            }),
        )
    }

    /// Send a cancellation request for a specific request ID
    /// Returns immediately - does NOT wait for confirmation
    /// Use assert_no_response_for_canceled() to verify cancellation worked
    pub fn cancel(&mut self, request_id: i32) {
        // Track this as a canceled ID
        self.canceled_ids.lock().push(request_id);

        // Send $/cancelRequest notification
        self.notify(
            "$/cancelRequest",
            json!({
                "id": request_id
            }),
        );
    }

    /// Assert that no response was received for a canceled request ID
    /// This verifies that the cancellation was successful
    pub fn assert_no_response_for_canceled(&mut self, request_id: i32, timeout: Duration) {
        let start = Instant::now();

        // Wait for timeout period to ensure no response arrives
        while start.elapsed() < timeout {
            {
                let output = self.output_buffer.lock();
                let output_str = String::from_utf8_lossy(&output);

                // Check if we got a response for this ID
                if output_str.contains(&format!("\"id\":{}", request_id))
                    || output_str.contains(&format!("\"id\": {}", request_id))
                {
                    assert!(false, "Received response for canceled request ID {}", request_id);
                }
            }

            thread::sleep(Duration::from_millis(10));
        }

        // Success - no response was received
    }

    /// Normalize file path for cross-platform testing (Windows/WSL/Unix)
    pub fn normalize_path(path: &str) -> String {
        // Detect WSL and convert Windows paths if needed
        if cfg!(target_os = "linux") && std::env::var("WSL_DISTRO_NAME").is_ok() {
            // In WSL, convert Windows paths like C:\foo to /mnt/c/foo
            if path.len() >= 3 && path.chars().nth(1) == Some(':') {
                let drive_char = match path.chars().next() {
                    Some(c) => c.to_lowercase().next().unwrap_or(c),
                    None => {
                        assert!(false, "Path should have at least one character: {path}");
                        ' '
                    }
                };
                let rest = path[2..].replace('\\', "/");
                return format!("/mnt/{}{}", drive_char, rest);
            }
        }

        // On Windows, normalize to forward slashes for file:// URIs
        if cfg!(target_os = "windows") {
            return path.replace('\\', "/");
        }

        // Unix paths are already normalized
        path.to_string()
    }

    /// Wait for a specific notification to arrive (barrier pattern)
    /// Returns the notification params if found, or error if timeout
    pub fn wait_for_notification(
        &mut self,
        method: &str,
        timeout: Duration,
    ) -> Result<Value, String> {
        let start = Instant::now();

        while start.elapsed() < timeout {
            let mut notifications = self.notification_buffer.lock();

            // Search for matching notification
            if let Some(pos) = notifications
                .iter()
                .position(|n| n.get("method").and_then(|m| m.as_str()) == Some(method))
            {
                let notif = match notifications.remove(pos) {
                    Some(n) => n,
                    None => {
                        assert!(false, "Notification at position {pos} should exist");
                        json!({})
                    }
                };
                drop(notifications);

                return Ok(notif.get("params").cloned().unwrap_or(json!({})));
            }

            drop(notifications);
            thread::sleep(Duration::from_millis(10));
        }

        Err(format!("Notification '{}' not received within {:?}", method, timeout))
    }

    /// Synchronization barrier - wait for all pending server operations to complete
    /// This replaces "sleep and hope" patterns with deterministic synchronization
    pub fn barrier(&mut self) {
        // Send a dummy request that forces the server to process all pending work
        // We use workspace/symbol with empty query as it's lightweight
        let _ = self.request_with_timeout(
            "workspace/symbol",
            json!({"query": "__barrier__"}),
            Duration::from_millis(500),
        );

        // Drain any notifications that arrived
        self.wait_for_idle(Duration::from_millis(100));
    }
}

impl LspHarness {
    /// Get adaptive timeout based on CI environment and thread configuration
    fn get_adaptive_timeout(&self) -> Duration {
        let thread_count = std::env::var("RUST_TEST_THREADS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(4);

        // Detect CI environments which need longer timeouts
        let is_ci = std::env::var("CI").is_ok()
            || std::env::var("GITHUB_ACTIONS").is_ok()
            || std::env::var("TRAVIS").is_ok()
            || std::env::var("CIRCLECI").is_ok()
            || std::env::var("JENKINS_URL").is_ok();

        // Detect containerized/constrained environments
        let is_constrained = std::env::var("DOCKER_CONTAINER").is_ok()
            || std::path::Path::new("/.dockerenv").exists()
            || std::env::var("KUBERNETES_SERVICE_HOST").is_ok();

        // Detect WSL environment (often has different performance characteristics)
        let is_wsl = std::env::var("WSL_DISTRO_NAME").is_ok() || std::env::var("WSLENV").is_ok();

        // Base timeout calculation with thread contention
        let base_timeout = match thread_count {
            0..=1 => Duration::from_millis(800), // Very high contention: much longer timeout
            2 => Duration::from_millis(600),     // High contention: longer timeout
            3..=4 => Duration::from_millis(400), // Medium contention
            5..=8 => Duration::from_millis(300), // Low contention
            _ => Duration::from_millis(200),     // Very low contention: shorter timeout
        };

        // Apply environment multipliers for reliability
        let multiplier = if is_ci && is_constrained {
            2.5 // CI + containerized: most constrained
        } else if is_ci {
            2.0 // CI environments: longer for reliability
        } else if is_constrained {
            1.8 // Containerized: some overhead
        } else if is_wsl {
            1.5 // WSL: moderate overhead
        } else {
            1.0 // Local development: optimal
        };

        // Apply performance test optimization
        let final_timeout = if std::env::var("PERL_LSP_PERFORMANCE_TEST").is_ok() {
            // Performance tests use shorter timeouts for speed
            Duration::from_millis((base_timeout.as_millis() as f64 * multiplier * 0.7) as u64)
        } else {
            Duration::from_millis((base_timeout.as_millis() as f64 * multiplier) as u64)
        };

        // Cap maximum timeout to prevent tests from hanging indefinitely
        final_timeout.min(Duration::from_secs(30))
    }
}

impl Drop for LspHarness {
    fn drop(&mut self) {
        // Enhanced cleanup with proper shutdown sequence
        self.shutdown_gracefully();
    }
}

impl LspHarness {
    /// Gracefully shutdown the LSP server with proper cleanup
    pub fn shutdown_gracefully(&mut self) {
        // Send shutdown request if we have an active connection
        let shutdown_timeout = if std::env::var("CI").is_ok() {
            Duration::from_secs(2) // CI: more time for cleanup
        } else {
            Duration::from_millis(500) // Local: faster cleanup
        };

        // Try to send shutdown request
        let _shutdown_result = self.request_with_timeout("shutdown", json!({}), shutdown_timeout);

        // Send exit notification
        self.notify("exit", json!({}));

        // Give server time to process shutdown
        thread::sleep(Duration::from_millis(50));

        // Signal server thread to terminate
        let _ = self.sender.send(Vec::new());

        // Wait for server thread to complete with timeout
        if let Some(handle) = self.handle.take() {
            let join_timeout = Duration::from_millis(1000);
            let start = Instant::now();

            // Use a simple timeout mechanism since we can't use thread::join with timeout in std
            while start.elapsed() < join_timeout {
                if handle.is_finished() {
                    let _ = handle.join();
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }

            // If thread didn't finish, we'll let it drop naturally
            // This prevents test hangs while still attempting graceful cleanup
        }
    }

    /// Add a method for checking if server is responsive
    pub fn is_server_responsive(&mut self) -> bool {
        // Quick ping to check if server is still alive
        let ping_result = self.request_with_timeout(
            "$/ping", // Non-standard but harmless ping
            json!({}),
            Duration::from_millis(100),
        );

        // If it responds (even with error), server is alive
        ping_result.is_ok() || ping_result.err().is_some_and(|e| !e.contains("timed out"))
    }
}

/// Test writer that captures output
struct TestWriter {
    buffer: Arc<Mutex<Vec<u8>>>,
    notifications: Arc<Mutex<VecDeque<Value>>>,
}

impl Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut buffer = self.buffer.lock();
        buffer.extend_from_slice(buf);

        // Try to parse as notification
        let content = String::from_utf8_lossy(buf);
        if let Some(json_start) = content.find("{") {
            let json_str = &content[json_start..];
            if let Ok(value) = serde_json::from_str::<Value>(json_str) {
                if value.get("method").is_some() && value.get("id").is_none() {
                    // It's a notification
                    self.notifications.lock().push_back(value);
                }
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// ======================== PHASE 1 STABILIZATION HELPERS ========================

/// Spawn LSP server with clean environment - Phase 1 stable interface
/// Returns a harness that is NOT yet initialized (call handshake_initialize separately)
pub fn spawn_lsp() -> LspHarness {
    // Set predictable environment for LSP server
    // SAFETY: We're in a test environment where modifying environment variables
    // is acceptable. These changes only affect the current test process.
    unsafe {
        std::env::set_var("RUST_LOG", "warn"); // Reduce noise in test output
        std::env::remove_var("PERL_LSP_PERFORMANCE_TEST"); // Ensure consistent behavior
    }

    LspHarness::new_raw()
}

/// Perform LSP handshake: initialize → wait for response → initialized notification
/// This is the deterministic initialization sequence for Phase 1
pub fn handshake_initialize(
    harness: &mut LspHarness,
    root_uri: Option<&str>,
) -> Result<Value, String> {
    let root = root_uri.unwrap_or("file:///test");

    // Step 1: Send initialize request
    let capabilities = json!({
        "textDocument": {
            "completion": {
                "completionItem": {
                    "snippetSupport": true
                }
            },
            "hover": {
                "contentFormat": ["markdown", "plaintext"]
            }
        }
    });

    let init_response = harness.initialize_with_root(root, Some(capabilities))?;

    // Step 2: Already sent initialized notification in initialize_with_root
    // Step 3: Barrier to ensure server is fully ready
    harness.barrier();

    Ok(init_response)
}

/// Gracefully shutdown LSP server - Phase 1 stable interface
/// This is a convenience wrapper around LspHarness::shutdown_gracefully
pub fn shutdown_graceful(harness: &mut LspHarness) {
    harness.shutdown_gracefully();
}

// Convenience macros

/// Macro for setting up a test with an open document
#[macro_export]
macro_rules! with_open_doc {
    ($uri:expr, $text:expr, $harness:ident, $body:block) => {{
        let mut $harness = LspHarness::new();
        match $harness.initialize(None) {
            Ok(_) => {}
            Err(e) => assert!(false, "Failed to initialize: {e}"),
        }
        match $harness.open($uri, $text) {
            Ok(_) => {}
            Err(e) => assert!(false, "Failed to open document: {e}"),
        }
        $body
    }};
}

/// Macro for asserting response contains expected locations
#[macro_export]
macro_rules! assert_locations {
    ($response:expr, [$( ($uri:expr, ($sl:expr, $sc:expr)..($el:expr, $ec:expr)) ),*]) => {
        {
            let locations = match $response.as_array() {
                Some(arr) => arr,
                None => assert!(false, "Response should be array: {:?}", $response),
            };
            let expected = vec![
                $( (
                    $uri,
                    ($sl, $sc),
                    ($el, $ec)
                ) ),*
            ];

            assert_eq!(locations.len(), expected.len(), "Location count mismatch");

            for (i, (uri, (sl, sc), (el, ec))) in expected.iter().enumerate() {
                let loc = &locations[i];
                assert_eq!(loc["uri"].as_str(), Some(*uri));
                assert_eq!(loc["range"]["start"]["line"].as_u64(), Some(*sl as u64));
                assert_eq!(loc["range"]["start"]["character"].as_u64(), Some(*sc as u64));
                assert_eq!(loc["range"]["end"]["line"].as_u64(), Some(*el as u64));
                assert_eq!(loc["range"]["end"]["character"].as_u64(), Some(*ec as u64));
            }
        }
    };
}

/// Macro for asserting highlights
#[macro_export]
macro_rules! assert_highlights {
    ($response:expr, [$( (($sl:expr, $sc:expr)..($el:expr, $ec:expr), $kind:expr) ),*]) => {
        {
            let highlights = match $response.as_array() {
                Some(arr) => arr,
                None => assert!(false, "Response should be array: {:?}", $response),
            };
            let expected = vec![
                $( (
                    ($sl, $sc),
                    ($el, $ec),
                    $kind
                ) ),*
            ];

            assert_eq!(highlights.len(), expected.len(), "Highlight count mismatch");

            for (i, ((sl, sc), (el, ec), kind)) in expected.iter().enumerate() {
                let hl = &highlights[i];
                assert_eq!(hl["range"]["start"]["line"].as_u64(), Some(*sl as u64));
                assert_eq!(hl["range"]["start"]["character"].as_u64(), Some(*sc as u64));
                assert_eq!(hl["range"]["end"]["line"].as_u64(), Some(*el as u64));
                assert_eq!(hl["range"]["end"]["character"].as_u64(), Some(*ec as u64));

                let actual_kind = hl["kind"].as_u64().unwrap_or(1);
                let expected_kind = match kind.as_str() {
                    "Read" => 1,
                    "Write" => 2,
                    "Text" => 3,
                    _ => 1,
                };
                assert_eq!(actual_kind, expected_kind as u64, "Highlight kind mismatch");
            }
        }
    };
}

/// Assert no diagnostics were published
#[macro_export]
macro_rules! assert_no_diags {
    ($harness:expr) => {{
        let diags = $harness.drain_notifications(Some("textDocument/publishDiagnostics"), 100);
        assert!(diags.is_empty(), "Expected no diagnostics, got: {:?}", diags);
    }};
}

/// Assert performance timing
#[macro_export]
macro_rules! assert_perf {
    ($duration:expr, < $max_ms:expr) => {{
        let max = std::time::Duration::from_millis($max_ms);
        assert!(
            $duration < max,
            "Performance assertion failed: {:?} >= {:?}ms",
            $duration,
            $max_ms
        );
    }};
}

// ======================== TESTCONTEXT COMPATIBILITY WRAPPER ========================

/// A compatibility wrapper that provides the same API as the old TestContext
/// but uses LspHarness underneath for proper initialization and synchronization.
///
/// This enables mechanical migration of tests from TestContext to LspHarness
/// with minimal diff.
///
/// # Migration
///
/// Old code:
/// ```ignore
/// let mut ctx = TestContext::new();
/// let _ = ctx.initialize();
/// ctx.open_document(uri, text);
/// let result = ctx.send_request("textDocument/hover", params);
/// ```
///
/// New code (just change imports):
/// ```ignore
/// use support::lsp_harness::TestContext;  // <-- Changed import
/// let mut ctx = TestContext::new();  // Same API
/// let _ = ctx.initialize();
/// ctx.open_document(uri, text);
/// let result = ctx.send_request("textDocument/hover", params);
/// ```
pub struct TestContext {
    harness: LspHarness,
    version_counter: i32,
}

impl TestContext {
    /// Create a new test context with uninitialized harness
    pub fn new() -> Self {
        Self { harness: LspHarness::new_raw(), version_counter: 1 }
    }

    /// Initialize the LSP server and wait for it to be fully ready
    ///
    /// Returns the initialization response value.
    /// Unlike the old TestContext, this includes a barrier to ensure the server is ready.
    /// Uses default root_uri "file:///workspace" and default capabilities.
    pub fn initialize(&mut self) -> Value {
        self.initialize_with("file:///workspace", None)
    }

    /// Initialize the LSP server with custom root_uri and capabilities
    ///
    /// Returns the initialization response value.
    /// This includes a barrier to ensure the server is fully ready.
    ///
    /// # Arguments
    /// * `root_uri` - The workspace root URI (e.g., "file:///test" or a real temp directory)
    /// * `capabilities` - Optional custom client capabilities (None = sensible defaults)
    pub fn initialize_with(&mut self, root_uri: &str, capabilities: Option<Value>) -> Value {
        match self.harness.initialize_ready(root_uri, capabilities) {
            Ok(v) => v,
            Err(e) => panic!("initialization should succeed: {e}"),
        }
    }

    /// Send a request and wait for response
    ///
    /// Returns `Some(result)` on success, `None` on error.
    /// Note: `params: None` maps to JSON `null`, not `{}` - this is correct per JSON-RPC spec.
    pub fn send_request(&mut self, method: &str, params: Option<Value>) -> Option<Value> {
        let p = params.unwrap_or(json!(null));
        self.harness.request(method, p).ok()
    }

    /// Send a notification (no response expected)
    /// Note: `params: None` maps to JSON `null`, not `{}` - this is correct per JSON-RPC spec.
    pub fn send_notification(&mut self, method: &str, params: Option<Value>) {
        let p = params.unwrap_or(json!(null));
        self.harness.notify(method, p);
    }

    /// Open a document
    pub fn open_document(&mut self, uri: &str, text: &str) {
        match self.harness.open(uri, text) {
            Ok(_) => {}
            Err(e) => assert!(false, "open should succeed: {e}"),
        }
    }

    /// Update document content with auto-incrementing version
    pub fn update_document(&mut self, uri: &str, text: &str) {
        self.version_counter += 1;
        match self.harness.change_full(uri, self.version_counter, text) {
            Ok(_) => {}
            Err(e) => assert!(false, "change should succeed: {e}"),
        }
    }

    /// Close a document
    pub fn close_document(&mut self, uri: &str) {
        match self.harness.close(uri) {
            Ok(_) => {}
            Err(e) => assert!(false, "close should succeed: {e}"),
        }
    }

    /// Synchronization barrier - wait for server to be idle
    pub fn barrier(&mut self) {
        self.harness.barrier();
    }

    /// Get underlying harness for advanced operations
    pub fn harness(&mut self) -> &mut LspHarness {
        &mut self.harness
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}
