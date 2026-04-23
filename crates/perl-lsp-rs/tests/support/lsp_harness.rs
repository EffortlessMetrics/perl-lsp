//! LSP Test Harness for Real JSON-RPC Testing
//!
//! Provides a test harness that communicates with the LSP server using real JSON-RPC protocol.
//!
//! This module acts as a facade: the implementation is split across focused
//! submodules (`test_workspace`, `message_framing`, `notification_queue`) while
//! this file re-exports every public symbol so that downstream test files
//! continue to compile without import changes.

#![allow(dead_code)]
#![allow(clippy::assertions_on_constants)]
#![allow(clippy::collapsible_if)]

use parking_lot::{Condvar, Mutex};
use perl_lsp_rs_core::transport::framing::{ContentLengthFramer, frame};
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::io::{Cursor, Write};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

// Re-export types from focused submodules so downstream imports are unchanged.
pub use super::notification_queue::TestContext;
pub use super::test_workspace::TempWorkspace;

// Import internal-only types from submodules.
use super::message_framing::{SendableServer, TestWriter};

/// LSP Test Harness for testing with real JSON-RPC protocol
pub struct LspHarness {
    sender: mpsc::Sender<Vec<u8>>,
    output_buffer: Arc<Mutex<Vec<u8>>>,
    output_framer: ContentLengthFramer,
    output_signal: Arc<Condvar>,
    notification_buffer: Arc<Mutex<VecDeque<Value>>>,
    server_requests: Arc<Mutex<VecDeque<Value>>>,
    next_request_id: i32,
    handle: Option<thread::JoinHandle<()>>,
    canceled_ids: Arc<Mutex<Vec<i32>>>, // Track canceled request IDs
}

fn uri_matches(expected: &str, actual: &str) -> bool {
    if expected == actual {
        return true;
    }

    if cfg!(windows) {
        return expected.eq_ignore_ascii_case(actual);
    }

    false
}

impl LspHarness {
    fn effective_request_timeout(&self, timeout: Duration) -> Duration {
        if timeout < Duration::from_secs(2) || std::env::var("PERL_LSP_PERFORMANCE_TEST").is_ok() {
            return timeout;
        }

        let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();

        if is_ci {
            timeout.max(Duration::from_secs(12))
        } else if cfg!(windows) {
            timeout.max(Duration::from_secs(10))
        } else {
            timeout
        }
    }

    fn is_coverage_instrumented() -> bool {
        std::env::var_os("LLVM_PROFILE_FILE").is_some()
            || std::env::var_os("CARGO_LLVM_COV").is_some()
            || std::env::var_os("CARGO_LLVM_COV_TARGET_DIR").is_some()
    }

    /// Lowest-level constructor: spawn server and wire pipes, no messages sent.
    pub fn new_raw() -> Self {
        let output_buffer = Arc::new(Mutex::new(Vec::new()));
        let output_signal = Arc::new(Condvar::new());
        let notification_buffer = Arc::new(Mutex::new(VecDeque::new()));
        let server_requests = Arc::new(Mutex::new(VecDeque::new()));

        // Create server with captured output
        let writer = Arc::new(Mutex::new(Box::new(TestWriter {
            buffer: output_buffer.clone(),
            signal: output_signal.clone(),
            notifications: notification_buffer.clone(),
            server_requests: server_requests.clone(),
        }) as Box<dyn Write + Send>));
        let server = SendableServer(perl_lsp::LspServer::with_output(writer));

        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        let handle = thread::spawn(move || {
            let server = server;
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
            output_framer: ContentLengthFramer::new(),
            output_signal,
            notification_buffer,
            server_requests,
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
        let is_windows = cfg!(windows);
        let init_timeout = if is_ci {
            Duration::from_secs(5) // CI: longer initialization timeout
        } else if std::env::var("PERL_LSP_PERFORMANCE_TEST").is_ok() {
            Duration::from_millis(800) // Performance tests: faster initialization
        } else if is_windows {
            Duration::from_secs(5) // Windows local runs need more room for process startup + discovery
        } else {
            Duration::from_secs(2) // Local: balanced timeout
        };
        let init_timeout = if Self::is_coverage_instrumented() {
            init_timeout.max(Duration::from_secs(6))
        } else {
            init_timeout
        };

        let response = self.send_request_with_timeout(init_request, init_timeout)?;

        // Only send initialized notification if initialization succeeded
        // (The response will contain capabilities if successful)
        if response.get("capabilities").is_some() {
            self.notify("initialized", json!({}));

            // Give server a moment to process the initialized notification
            let settle_time = if is_ci || is_windows {
                Duration::from_millis(100) // CI: extra settling time
            } else {
                Duration::from_millis(50) // Local: minimal settling time
            };
            thread::sleep(settle_time);
        }

        Ok(response)
    }

    /// Initialize the LSP server with explicit `initializationOptions`.
    ///
    /// Unlike `initialize()` which only sets `params.capabilities`, this method
    /// also injects `initializationOptions` into the initialize params, enabling
    /// tests of per-feature disable via `disabledFeatures`.
    pub fn initialize_with_init_options(
        &mut self,
        capabilities: Option<Value>,
        initialization_options: Value,
    ) -> Result<Value, String> {
        let caps = capabilities.unwrap_or_else(|| {
            json!({
                "textDocument": {
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true
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
                "rootUri": "file:///workspace",
                "initializationOptions": initialization_options
            }
        });
        self.next_request_id += 1;

        let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
        let is_windows = cfg!(windows);
        let init_timeout =
            if is_ci || is_windows { Duration::from_secs(5) } else { Duration::from_secs(2) };
        let init_timeout = if Self::is_coverage_instrumented() {
            init_timeout.max(Duration::from_secs(6))
        } else {
            init_timeout
        };

        let response = self.send_request_with_timeout(init_request, init_timeout)?;

        if response.get("capabilities").is_some() {
            self.notify("initialized", json!({}));
            let settle_time = if is_ci || is_windows {
                Duration::from_millis(100)
            } else {
                Duration::from_millis(50)
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
        harness.initialize_ready(&workspace.root_uri, None)?;

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

    /// Request `textDocument/completion` at the provided LSP position.
    pub fn completion_at(&mut self, uri: &str, line: u32, character: u32) -> Result<Value, String> {
        self.request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )
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
        let is_windows = cfg!(windows);
        let (max_attempts, initial_timeout, max_sleep) = if is_ci {
            (8, 300, 200) // CI: more attempts, longer timeouts
        } else if is_windows {
            (8, 300, 150) // Windows local runs are slower than Linux/macOS for temp workspace indexing
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
                            want_uri.is_none_or(|expect| {
                                uri.is_some_and(|actual| uri_matches(expect, actual))
                            })
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
        let timeout = self.get_adaptive_timeout();
        self.request_raw_with_timeout(request, timeout)
    }

    /// Alternative request method that accepts a full JSON-RPC request object with a custom timeout.
    pub fn request_raw_with_timeout(&mut self, request: Value, timeout: Duration) -> Value {
        // Handle full JSON-RPC request object
        if request.is_object() && request.get("jsonrpc").is_some() {
            let mut req = request;
            req["id"] = json!(self.next_request_id);
            self.next_request_id += 1;

            // Use send_request_full_response to get the complete JSON-RPC response
            self.send_request_with_timeout_full_response(req, timeout).unwrap_or_else(|e| {
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

        let request_str = notification.to_string();
        let _ = self.sender.send(frame(request_str.as_bytes()));
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

        if let Some(filter_method) = method {
            // Drain the entire deque, collecting matches and keeping non-matches in order
            let mut remaining = VecDeque::with_capacity(notifications.len());
            while let Some(notif) = notifications.pop_front() {
                if notif["method"].as_str() == Some(filter_method) {
                    result.push(notif);
                } else {
                    remaining.push_back(notif);
                }
            }
            *notifications = remaining;
        } else {
            // No filter: drain all
            while let Some(notif) = notifications.pop_front() {
                result.push(notif);
            }
        }

        result
    }

    /// Wait for a `$/progress` notification whose token and kind match.
    ///
    /// Actively polls the notification buffer until a `$/progress` message
    /// matching `token` and `kind` ("begin" | "report" | "end") is found, or
    /// the timeout expires.  Non-matching messages are left in the buffer.
    ///
    /// Returns the full notification value on success, or an error message.
    pub fn wait_for_progress_kind(
        &mut self,
        token: &str,
        kind: &str,
        timeout: Duration,
    ) -> Result<Value, String> {
        let start = Instant::now();

        loop {
            // Pump any pending output into the framer and stash parsed messages.
            {
                let mut guard = self.output_buffer.lock();
                if !guard.is_empty() {
                    let chunk = std::mem::take(&mut *guard);
                    self.output_framer.push(&chunk);
                }
                drop(guard);
            }
            while let Some(msg_bytes) = self.try_take_one_framed_message() {
                if let Ok(msg) = serde_json::from_slice::<Value>(&msg_bytes) {
                    self.stash_non_matching_message(msg);
                }
            }

            {
                let mut notifications = self.notification_buffer.lock();
                if let Some(pos) = notifications.iter().position(|n| {
                    n.get("method").and_then(|m| m.as_str()) == Some("$/progress")
                        && n.pointer("/params/token").and_then(|v| v.as_str()) == Some(token)
                        && n.pointer("/params/value/kind").and_then(|v| v.as_str()) == Some(kind)
                }) {
                    let found = notifications.remove(pos).unwrap_or(json!(null));
                    return Ok(found);
                }
            }

            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                return Err(format!(
                    "$/progress {kind} for token '{token}' not received within {timeout:?}"
                ));
            }

            // Wait for the TestWriter to signal new data.
            let mut guard = self.output_buffer.lock();
            self.output_signal.wait_for(&mut guard, remaining.min(Duration::from_millis(50)));
        }
    }

    /// Drain server-initiated requests from the buffer.
    ///
    /// Server-to-client requests (e.g., `window/workDoneProgress/create`) are
    /// buffered in `server_requests` by `stash_non_matching_message`.  This
    /// method waits up to `timeout_ms` for at least one request to arrive, then
    /// drains and returns all of them.
    pub fn drain_server_requests(&mut self, timeout_ms: u64) -> Vec<Value> {
        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        // Pump the output buffer so any queued bytes are parsed first.
        while start.elapsed() < timeout {
            {
                let mut guard = self.output_buffer.lock();
                if !guard.is_empty() {
                    let chunk = std::mem::take(&mut *guard);
                    self.output_framer.push(&chunk);
                }
                drop(guard);
            }

            while let Some(msg_bytes) = self.try_take_one_framed_message() {
                if let Ok(msg) = serde_json::from_slice::<Value>(&msg_bytes) {
                    self.stash_non_matching_message(msg);
                }
            }

            {
                let reqs = self.server_requests.lock();
                if !reqs.is_empty() {
                    break;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }

        let mut reqs = self.server_requests.lock();
        let mut result = Vec::new();
        while let Some(req) = reqs.pop_front() {
            result.push(req);
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

    // Stash a non-matching message into the appropriate buffer by type.
    // Called from response drain loops to avoid discarding server-initiated messages.
    fn stash_non_matching_message(&self, msg: Value) {
        let has_method = msg.get("method").is_some();
        let has_id = msg.get("id").is_some();
        if has_method && !has_id {
            // Server notification
            self.notification_buffer.lock().push_back(msg);
        } else if has_method && has_id {
            // Server-initiated request
            self.server_requests.lock().push_back(msg);
        }
        // Responses with non-matching ids are intentionally dropped
        // (they belong to canceled or timed-out requests)
    }

    fn try_take_one_framed_message(&mut self) -> Option<Vec<u8>> {
        loop {
            match self.output_framer.try_next() {
                Ok(Some(body)) => return Some(body),
                Ok(None) => return None,
                Err(error) => {
                    eprintln!("LSP harness framing error: {error}");
                }
            }
        }
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
        let timeout = self.effective_request_timeout(timeout);
        let expect_id = request.get("id").and_then(|v| v.as_i64());

        // Format request with Content-Length framing
        let request_str = request.to_string();
        let content = frame(request_str.as_bytes());

        // Send to server thread
        if let Err(e) = self.sender.send(content) {
            return Err(format!("Server send error: {}", e));
        }

        // Wait for response with timeout using condvar signaling.
        // Lock the buffer once, then drain complete messages.
        let start = Instant::now();
        let mut guard = self.output_buffer.lock();
        loop {
            if start.elapsed() > timeout {
                return Err(format!("Request timed out after {:?}", timeout));
            }

            if !guard.is_empty() {
                let chunk = std::mem::take(&mut *guard);
                self.output_framer.push(&chunk);
            }

            drop(guard);

            while let Some(msg_bytes) = self.try_take_one_framed_message() {
                if let Ok(msg) = serde_json::from_slice::<Value>(&msg_bytes) {
                    if msg.get("id").and_then(|v| v.as_i64()) == expect_id {
                        if let Some(error) = msg.get("error") {
                            return Err(format!("LSP error: {:?}", error));
                        }
                        if let Some(result) = msg.get("result") {
                            return Ok(result.clone());
                        }
                    } else {
                        // Non-matching message: stash by type instead of discarding
                        self.stash_non_matching_message(msg);
                    }
                }
            }

            guard = self.output_buffer.lock();

            // Wait for signal from TestWriter with bounded timeout
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                break;
            }
            self.output_signal.wait_for(&mut guard, remaining.min(Duration::from_millis(100)));
        }

        Err("No response received".to_string())
    }

    // Private helper to send request with timeout and return full JSON-RPC response
    fn send_request_with_timeout_full_response(
        &mut self,
        request: Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        let timeout = self.effective_request_timeout(timeout);
        let expect_id = request.get("id").and_then(|v| v.as_i64());

        // Format request with Content-Length framing
        let request_str = request.to_string();
        let content = frame(request_str.as_bytes());

        // Send to server thread
        if let Err(e) = self.sender.send(content) {
            return Err(format!("Server send error: {}", e));
        }

        // Wait for response with timeout using condvar signaling.
        // Lock the buffer once, then drain complete messages.
        let start = Instant::now();
        let mut guard = self.output_buffer.lock();
        loop {
            if start.elapsed() > timeout {
                return Err(format!("Request timed out after {:?}", timeout));
            }

            if !guard.is_empty() {
                let chunk = std::mem::take(&mut *guard);
                self.output_framer.push(&chunk);
            }

            drop(guard);

            while let Some(msg_bytes) = self.try_take_one_framed_message() {
                if let Ok(msg) = serde_json::from_slice::<Value>(&msg_bytes) {
                    if msg.get("id").and_then(|v| v.as_i64()) == expect_id {
                        // Return the full message for schema validation tests
                        return Ok(msg);
                    } else {
                        // Non-matching message: stash by type instead of discarding
                        self.stash_non_matching_message(msg);
                    }
                }
            }

            guard = self.output_buffer.lock();

            // Wait for signal from TestWriter with bounded timeout
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                break;
            }
            self.output_signal.wait_for(&mut guard, remaining.min(Duration::from_millis(100)));
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

        loop {
            {
                let mut notifications = self.notification_buffer.lock();

                // Search for matching notification
                if let Some(pos) = notifications
                    .iter()
                    .position(|n| n.get("method").and_then(|m| m.as_str()) == Some(method))
                {
                    let notif = match notifications.remove(pos) {
                        Some(n) => n,
                        None => return Err(format!("Notification at position {pos} vanished")),
                    };
                    drop(notifications);

                    return Ok(notif.get("params").cloned().unwrap_or(json!({})));
                }
            }

            // Wait for signal from TestWriter with bounded timeout
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                break;
            }
            let mut guard = self.output_buffer.lock();
            self.output_signal.wait_for(&mut guard, remaining.min(Duration::from_millis(100)));
        }

        Err(format!("Notification '{}' not received within {:?}", method, timeout))
    }

    /// Synchronization barrier - wait for all pending server operations to complete
    /// This replaces "sleep and hope" patterns with deterministic synchronization
    pub fn barrier(&mut self) {
        // Send a dummy request that forces the server to process all pending work
        // We use workspace/symbol with empty query as it's lightweight
        let timeout =
            if cfg!(windows) { Duration::from_millis(1500) } else { Duration::from_millis(500) };
        let _ =
            self.request_with_timeout("workspace/symbol", json!({"query": "__barrier__"}), timeout);

        // Drain any notifications that arrived
        let idle_budget =
            if cfg!(windows) { Duration::from_millis(200) } else { Duration::from_millis(100) };
        self.wait_for_idle(idle_budget);
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
        let is_windows = cfg!(windows);

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

        let final_timeout = if is_windows && std::env::var("PERL_LSP_PERFORMANCE_TEST").is_err() {
            final_timeout.max(Duration::from_millis(1200))
        } else {
            final_timeout
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

        // Signal server thread to terminate via empty message.
        // Do NOT send "exit" notification — the server's handle_exit_dispatch calls
        // std::process::exit() which would kill the entire test process.
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

/// Perform LSP handshake: initialize -> wait for response -> initialized notification
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
