//! Common test utilities for LSP integration tests
//!
//! ## Test Harness Contracts
//!
//! - **Deterministic IO**: Background reader thread with bounded queue prevents blocking
//! - **Request IDs**: Auto-generated when omitted from test requests (avoids collisions)
//! - **Response Matching**: Match by ID for request/response pairing
//! - **Timeouts**: Configurable via env vars, with sensible defaults
//! - **Quiet Drain**: Wait for server to settle after changes before assertions
//! - **Portable Spawn**: PERL_LSP_BIN → CARGO_BIN_EXE_* → PATH → cargo run fallback
//!
//! ## Environment Variables
//!
//! - `PERL_LSP_BIN`: Explicit path to perl-lsp binary (useful for custom CARGO_TARGET_DIR)
//! - `LSP_TEST_TIMEOUT_MS`: Default per-request timeout (ms), default 5000
//! - `LSP_TEST_SHORT_MS`: "Short" timeout for optional responses (ms), default 500
//! - `LSP_TEST_ECHO_STDERR`: If set, echo perl-lsp stderr lines in tests
//!
//! ## Key Functions
//!
//! - `send_request()`: Sends request and returns matched response (auto-generates ID if missing)
//! - `drain_until_quiet()`: Waits for server to stop sending messages
//! - `read_notification_method()`: Reads specific notification by method name
//! - `read_response_matching()`: Reads response matching specific ID

#![allow(dead_code)] // Common test utilities - some may not be used by all test files

// Re-export test_utils for semantic tests
pub mod test_utils;

// Error codes
const ERR_TEST_TIMEOUT: i64 = -32000;

use serde_json::{Value, json};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};

const PENDING_CAP: usize = 512; // Prevent unbounded growth of pending message queue
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

// Auto-generate unique IDs for requests
static NEXT_ID: AtomicI64 = AtomicI64::new(1000);

// Global mutex to serialize LSP server creation to prevent resource conflicts
static LSP_SERVER_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

fn next_id() -> i64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Get completion items from a response, handling both array and object formats
pub fn completion_items(resp: &serde_json::Value) -> &Vec<serde_json::Value> {
    resp["result"]["items"]
        .as_array()
        .or_else(|| resp["result"].as_array())
        .expect("completion result should be array or { items: [] }")
}

pub struct LspServer {
    pub process: Child,
    writer: BufWriter<ChildStdin>, // keep stdin pinned and flushed
    rx: Receiver<Value>,
    // Keep threads alive for the lifetime of the server
    _stdout_thread: std::thread::JoinHandle<()>,
    _stderr_thread: std::thread::JoinHandle<()>,
    pending: VecDeque<Value>,
}

impl LspServer {
    /// Check if the server process is still running
    pub fn is_alive(&mut self) -> bool {
        self.process.try_wait().unwrap().is_none()
    }

    /// Get mutable access to the stdin writer
    pub fn stdin_writer(&mut self) -> &mut BufWriter<ChildStdin> {
        &mut self.writer
    }
}

fn resolve_perl_lsp_cmds() -> impl Iterator<Item = Command> {
    // Resolution order:
    // 1. PERL_LSP_BIN env var (explicit override, useful for custom target dirs)
    // 2. CARGO_BIN_EXE_* (Cargo-provided path during test execution)
    // 3. Workspace target directory binaries (absolute paths)
    // 4. PATH lookup
    // 5. cargo run fallback (slow but always works)
    let mut v: Vec<Command> = Vec::new();

    // 1. Explicit override via PERL_LSP_BIN
    if let Ok(p) = std::env::var("PERL_LSP_BIN") {
        let mut c = Command::new(p);
        c.arg("--stdio");
        v.push(c);
    }

    // 2. Cargo-provided binary path (set during `cargo test`)
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_perl-lsp") {
        let mut c = Command::new(p);
        c.arg("--stdio");
        v.push(c);
    }
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_perl_lsp") {
        let mut c = Command::new(p);
        c.arg("--stdio");
        v.push(c);
    }

    // 3. Try workspace target directory binaries (using absolute paths)
    // CARGO_MANIFEST_DIR points to the crate directory, we need the workspace root
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let crate_dir = std::path::Path::new(&manifest_dir);
        // Walk up to find workspace root (contains Cargo.toml with [workspace])
        let workspace_root =
            crate_dir.ancestors().find(|p| p.join("Cargo.lock").exists()).unwrap_or(crate_dir);

        // Try release binary first (faster)
        let release_binary = workspace_root.join("target/release/perl-lsp");
        if release_binary.exists() {
            let mut c = Command::new(&release_binary);
            c.arg("--stdio");
            v.push(c);
        }

        // Then try debug binary
        let debug_binary = workspace_root.join("target/debug/perl-lsp");
        if debug_binary.exists() {
            let mut c = Command::new(&debug_binary);
            c.arg("--stdio");
            v.push(c);
        }
    }

    // 4. Try perl-lsp from PATH
    {
        let mut c = Command::new("perl-lsp");
        c.arg("--stdio");
        v.push(c);
    }

    // 5. Fallback: use cargo run with release profile (avoid debug linking issues)
    // This is SLOW because it may need to compile, but always works
    {
        let mut c = Command::new("cargo");
        c.args(["run", "-q", "-p", "perl-lsp", "--release", "--", "--stdio"]);
        v.push(c);
    }

    v.into_iter()
}

pub fn start_lsp_server() -> LspServer {
    // Serialize LSP server creation to prevent resource conflicts during concurrent testing
    let _guard = LSP_SERVER_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap();

    // Try candidates in order; fall back cleanly on NotFound
    let mut last_err: Option<io::Error> = None;
    let mut process: Child = {
        let mut spawned: Option<Child> = None;
        for mut cmd in resolve_perl_lsp_cmds() {
            match cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
                Ok(child) => {
                    spawned = Some(child);
                    break;
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    last_err = Some(e);
                    continue;
                }
                Err(e) => {
                    last_err = Some(e);
                    continue;
                }
            }
        }
        spawned.unwrap_or_else(|| {
            eprintln!("Failed to start perl-lsp server - tried all methods:");
            eprintln!("  1. PERL_LSP_BIN env var (explicit override)");
            eprintln!("  2. CARGO_BIN_EXE_perl-lsp env var");
            eprintln!("  3. CARGO_BIN_EXE_perl_lsp env var");
            eprintln!("  4. Workspace target/release/perl-lsp");
            eprintln!("  5. Workspace target/debug/perl-lsp");
            eprintln!("  6. perl-lsp from PATH");
            eprintln!("  7. cargo run --release -p perl-lsp");
            eprintln!("Last error: {:?}", last_err);
            eprintln!("Hint: Set PERL_LSP_BIN=/path/to/perl-lsp to use a custom binary");
            eprintln!("Hint: Run 'cargo build -p perl-lsp --release' first for faster tests");
            panic!("Failed to start perl-lsp via any available method: {:?}", last_err)
        })
    };

    let stdin = process.stdin.take().expect("child stdin should be available");

    // -------- stderr drain thread (prevents child from blocking on logs) --------
    let stderr = process.stderr.take().expect("stderr piped");
    let echo = std::env::var_os("LSP_TEST_ECHO_STDERR").is_some();
    let _stderr_thread = std::thread::Builder::new()
        .name("lsp-stderr-drain".into())
        .spawn(move || {
            let mut r = BufReader::new(stderr);
            let mut line = String::new();
            while r.read_line(&mut line).unwrap_or(0) > 0 {
                if echo {
                    eprintln!("[perl-lsp] {}", line.trim_end());
                }
                line.clear();
            }
        })
        .unwrap();

    // -------- stdout LSP reader thread --------
    let stdout = process.stdout.take().expect("stdout piped");
    let (tx, rx) = mpsc::channel::<Value>();
    let debug_reader = std::env::var_os("LSP_TEST_DEBUG_READER").is_some();
    let _stdout_thread = std::thread::Builder::new()
        .name("lsp-stdout-reader".into())
        .spawn(move || {
            let mut r = BufReader::new(stdout);
            if debug_reader {
                eprintln!("[reader] Thread started");
            }
            loop {
                // Parse headers
                let mut content_len: Option<usize> = None;
                let mut line = String::new();
                loop {
                    line.clear();
                    match r.read_line(&mut line) {
                        Ok(0) => {
                            if debug_reader {
                                eprintln!("[reader] EOF on stdout");
                            }
                            return; // EOF
                        }
                        Ok(_) => {
                            let l = line.trim_end();
                            if l.is_empty() {
                                break;
                            }
                            // Case-insensitive header matching with flexible colon handling
                            let lower = l.to_ascii_lowercase();
                            if let Some(rest) = lower.strip_prefix("content-length") {
                                let rest = rest.trim_start_matches(':').trim();
                                content_len = rest.parse::<usize>().ok();
                            }
                        }
                        Err(e) => {
                            if debug_reader {
                                eprintln!("[reader] Error reading line: {e}");
                            }
                            return;
                        }
                    }
                }
                let len = match content_len {
                    Some(n) => n,
                    None => continue,
                };
                // Read body
                let mut buf = vec![0u8; len];
                if r.read_exact(&mut buf).is_err() {
                    if debug_reader {
                        eprintln!("[reader] Error reading body");
                    }
                    return;
                }
                if let Ok(val) = serde_json::from_slice::<Value>(&buf) {
                    if debug_reader {
                        let id = val.get("id").map(|v| v.to_string()).unwrap_or_default();
                        let method = val.get("method").and_then(|v| v.as_str()).unwrap_or("");
                        eprintln!("[reader] Received message id={id} method={method}");
                    }
                    let _ = tx.send(val);
                }
            }
        })
        .unwrap();

    let server = LspServer {
        process,
        writer: BufWriter::new(stdin),
        rx,
        _stdout_thread,
        _stderr_thread,
        pending: VecDeque::new(),
    };

    // Brief delay to allow server to fully initialize before returning
    std::thread::sleep(Duration::from_millis(100));

    server
}

pub fn send_request(server: &mut LspServer, mut request: Value) -> Value {
    // Ensure every request has an id so we can match the response deterministically
    let id = match request.get("id") {
        Some(v) => Some(v.clone()),
        None => {
            let nid = next_id();
            request["id"] = json!(nid);
            Some(json!(nid))
        }
    };

    let body = request.to_string();
    write!(server.writer, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
    server.writer.flush().unwrap();

    // Match by ID to avoid confusion with interleaved notifications
    match id {
        Some(Value::Number(n)) if n.as_i64().is_some() => {
            read_response_matching_i64(server, n.as_i64().unwrap(), default_timeout())
                .unwrap_or_else(
                    || json!({"error":{"code":ERR_TEST_TIMEOUT,"message":"test harness timeout"}}),
                )
        }
        Some(v) => read_response_matching(server, &v, default_timeout()).unwrap_or_else(
            || json!({"error":{"code":ERR_TEST_TIMEOUT,"message":"test harness timeout"}}),
        ),
        None => unreachable!("we always assign an id"),
    }
}

pub fn send_notification(server: &mut LspServer, notification: Value) {
    let body = notification.to_string();
    write!(server.writer, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
    server.writer.flush().unwrap();
}

fn default_timeout() -> Duration {
    std::env::var("LSP_TEST_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or_else(|| {
            // More nuanced adaptive timeout with exponential backoff
            let _base_timeout = Duration::from_secs(5); // Use underscore to suppress unused var warning
            let thread_count = max_concurrent_threads();

            match thread_count {
                0..=2 => Duration::from_secs(8), // Heavily constrained: reduced from 15s to 8s for faster execution
                3..=4 => Duration::from_secs(6), // Moderately constrained: reduced from 10s to 6s
                5..=8 => Duration::from_secs(4), // Lightly constrained: reduced from 7s to 4s
                _ => Duration::from_secs(3),     // Unconstrained: reduced from 5s to 3s
            }
        })
}

/// Short timeout for expected non-responses (malformed requests, etc)
pub fn short_timeout() -> Duration {
    std::env::var("LSP_TEST_SHORT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or_else(|| {
            // Adaptive short timeout based on thread constraints
            let thread_count = max_concurrent_threads();
            match thread_count {
                0..=2 => Duration::from_millis(500), // Heavily constrained: reduced from 1000ms
                3..=4 => Duration::from_millis(400), // Moderately constrained: reduced from 750ms
                5..=8 => Duration::from_millis(300), // Lightly constrained: reduced from 500ms
                _ => Duration::from_millis(200),     // Unconstrained: reduced from 300ms
            }
        })
}

/// Get the maximum number of concurrent threads to use in tests
/// Respects RUST_TEST_THREADS environment variable and scales down thread counts appropriately
pub fn max_concurrent_threads() -> usize {
    std::env::var("RUST_TEST_THREADS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or_else(|| {
            // Try to detect system thread count, default to 8
            std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8)
        })
        .max(1) // Ensure at least 1 thread
}

/// Get adaptive timeout based on thread constraints
/// More comprehensive handling of timeout scaling with logarithmic backoff
pub fn adaptive_timeout() -> Duration {
    let base_timeout = default_timeout();
    let thread_count = max_concurrent_threads();

    // Reduced multipliers for faster test execution
    match thread_count {
        0..=2 => base_timeout, // Heavily constrained: reduced from 3x to 1x
        3..=4 => base_timeout, // Moderately constrained: reduced from 2x to 1x
        5..=8 => base_timeout, // Lightly constrained: reduced from 1.5x to 1x
        _ => base_timeout,     // Unconstrained: standard timeout
    }
}

/// Adaptive sleep duration based on thread constraints
/// More sophisticated sleep scaling with exponential strategy
pub fn adaptive_sleep_ms(base_ms: u64) -> Duration {
    let thread_count = max_concurrent_threads();
    let multiplier = match thread_count {
        0..=2 => 1, // Extremely constrained: reduced from 4x to 1x sleep
        3..=4 => 1, // Heavily constrained: reduced from 3x to 1x sleep
        5..=8 => 1, // Moderately constrained: reduced from 2x to 1x sleep
        _ => 1,     // Unconstrained: standard sleep
    };
    Duration::from_millis(base_ms * multiplier)
}

/// Blocking receive with a sane default timeout to avoid hangs.
pub fn read_response(server: &mut LspServer) -> Value {
    read_response_timeout(server, default_timeout()).unwrap_or_else(
        || json!({"error":{"code":ERR_TEST_TIMEOUT,"message":"test harness timeout"}}),
    )
}

/// Try to receive a response within `dur`. Returns None on timeout.
pub fn read_response_timeout(server: &mut LspServer, dur: Duration) -> Option<Value> {
    server.rx.recv_timeout(dur).ok()
}

/// Try to receive a notification (message without id) within `dur`. Returns None on timeout or if a response is received.
pub fn read_notification_timeout(server: &mut LspServer, dur: Duration) -> Option<Value> {
    match server.rx.recv_timeout(dur) {
        Ok(val) if val.get("id").is_none() => Some(val),
        Ok(_) => None,  // Got a response, not a notification
        Err(_) => None, // Timeout or disconnected
    }
}

/// Receive the response matching `id` (number or string), buffering other traffic.
pub fn read_response_matching(server: &mut LspServer, id: &Value, dur: Duration) -> Option<Value> {
    // scan buffered
    for _ in 0..server.pending.len() {
        if let Some(msg) = server.pending.pop_front() {
            if msg.get("id") == Some(id) {
                return Some(msg);
            }
            if server.pending.len() >= PENDING_CAP {
                server.pending.pop_front();
            }
            server.pending.push_back(msg);
        }
    }
    // then poll
    let deadline = Instant::now() + dur;
    loop {
        let now = Instant::now();
        if now >= deadline {
            return None;
        }
        match server.rx.recv_timeout(deadline - now) {
            Ok(msg) => {
                if msg.get("id") == Some(id) {
                    return Some(msg);
                }
                if server.pending.len() >= PENDING_CAP {
                    server.pending.pop_front();
                }
                server.pending.push_back(msg);
            }
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => return None,
        }
    }
}

/// Convenience for numeric ids.
pub fn read_response_matching_i64(server: &mut LspServer, id: i64, dur: Duration) -> Option<Value> {
    read_response_matching(server, &json!(id), dur)
}

/// Write raw bytes (for malformed/binary frame tests).
pub fn send_raw(server: &mut LspServer, bytes: &[u8]) {
    server.writer.write_all(bytes).unwrap();
    server.writer.flush().unwrap();
}

/// Read a notification matching the given method name
pub fn read_notification_method(
    server: &mut LspServer,
    method: &str,
    dur: Duration,
) -> Option<Value> {
    let deadline = Instant::now() + dur;

    // scan buffered first
    for _ in 0..server.pending.len() {
        if let Some(msg) = server.pending.pop_front() {
            if msg.get("id").is_none() && msg.get("method") == Some(&json!(method)) {
                return Some(msg);
            }
            server.pending.push_back(msg);
        }
    }

    // then poll
    while Instant::now() < deadline {
        match server.rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(msg) => {
                let is_match = msg.get("id").is_none() && msg.get("method") == Some(&json!(method));
                if is_match {
                    return Some(msg);
                }
                if server.pending.len() >= PENDING_CAP {
                    server.pending.pop_front();
                }
                server.pending.push_back(msg);
            }
            Err(_) => break,
        }
    }
    None
}

/// Drain messages until no traffic for a quiet period (stabilizes CI)
pub fn drain_until_quiet(server: &mut LspServer, quiet: Duration, ceiling: Duration) {
    let start = Instant::now();
    let mut last = Instant::now();
    while start.elapsed() < ceiling {
        match server.rx.recv_timeout(quiet.saturating_sub(last.elapsed())) {
            Ok(msg) => {
                if server.pending.len() >= PENDING_CAP {
                    server.pending.pop_front();
                }
                server.pending.push_back(msg);
                last = Instant::now();
            }
            Err(_) => break, // quiet period achieved
        }
    }
}

pub fn initialize_lsp(server: &mut LspServer) -> Value {
    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "capabilities": {},
            "clientInfo": {"name":"perl-parser-tests","version":"0"},
            "rootUri": null,
            "workspaceFolders": null
        }
    });

    // write without reading
    {
        let body = init.to_string();
        write!(server.writer, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
        server.writer.flush().unwrap();
    }

    // wait specifically for id=1 - use extended timeout for initialization
    // Enhanced timeout for LSP cancellation tests with environment-aware scaling
    let base_multiplier = 3; // Increased base multiplier for cancellation infrastructure tests (increased from 2x to 3x)
    let thread_count = max_concurrent_threads();
    let env_multiplier = if thread_count <= 2 { 3 } else { 2 }; // Extra time for constrained environments with cancellation infrastructure (increased from 2x to 3x)

    // Additional CI environment detection for graceful degradation
    let ci_multiplier = if std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("CONTINUOUS_INTEGRATION").is_ok()
    {
        2 // Extra time for CI environments with limited resources
    } else {
        1
    };

    let init_timeout = adaptive_timeout() * base_multiplier * env_multiplier * ci_multiplier;

    // Enhanced retry logic for cancellation infrastructure tests
    let mut retry_count = 0;
    let max_retries = 2; // Allow 2 retries for infrastructure tests

    let resp = loop {
        match read_response_matching_i64(server, 1, init_timeout) {
            Some(response) => break response,
            None => {
                retry_count += 1;
                if retry_count > max_retries {
                    eprintln!(
                        "LSP server failed to respond to initialize request within {:?} after {} retries",
                        init_timeout, max_retries
                    );
                    eprintln!(
                        "Check if server started properly and is responding to JSON-RPC requests"
                    );
                    eprintln!("Server process alive: {}", server.is_alive());
                    panic!(
                        "initialize response timeout - server may have crashed or is not responding"
                    )
                } else {
                    eprintln!(
                        "Initialize timeout attempt {}/{}, retrying with fresh request...",
                        retry_count,
                        max_retries + 1
                    );
                    // Brief delay before retry
                    std::thread::sleep(Duration::from_millis(200));
                    // Send another initialize request with a new ID
                    let retry_id = next_id();
                    send_request(
                        server,
                        json!({"id":retry_id,"method":"initialize","params":{"capabilities":{}}}),
                    );
                    // Try reading the retry response
                    if let Some(retry_resp) =
                        read_response_matching_i64(server, retry_id, init_timeout)
                    {
                        break retry_resp;
                    }
                }
            }
        }
    };

    // Send initialized notification with a brief delay
    std::thread::sleep(Duration::from_millis(50));
    send_notification(server, json!({"jsonrpc":"2.0","method":"initialized"}));

    // Wait for index-ready notification to ensure deterministic completion behavior
    await_index_ready(server);

    resp
}

/// Wait for the index-ready notification from the server
pub fn await_index_ready(server: &mut LspServer) {
    // Wait for perl-lsp/index-ready notification with a reasonable timeout
    if let Some(_notification) =
        read_notification_method(server, "perl-lsp/index-ready", Duration::from_millis(500))
    {
        eprintln!("Index ready notification received");
    } else {
        eprintln!("No index-ready notification received within timeout (proceeding anyway)");
    }
}

/// Gracefully shut the server down (best-effort) without hanging tests.
pub fn shutdown_and_exit(server: &mut LspServer) {
    // Try a graceful shutdown first; if the server ignores, we'll still exit the test.
    let _ = send_request(
        server,
        json!({"jsonrpc":"2.0","id": 999_001,"method":"shutdown","params":{}}),
    );
    send_notification(server, json!({"jsonrpc":"2.0","method":"exit"}));

    // Give it a moment, then force-kill if needed.
    for _ in 0..20 {
        if server.process.try_wait().ok().flatten().is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = server.process.kill();
}

/// Send raw message to server (for testing malformed input)
pub fn send_raw_message(server: &mut LspServer, content: &str) {
    write!(server.writer, "Content-Length: {}\r\n\r\n{}", content.len(), content).unwrap();
    server.writer.flush().unwrap();
}

/// Send request without waiting for response
pub fn send_request_no_wait(server: &mut LspServer, req: Value) {
    let body = req.to_string();
    write!(server.writer, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
    server.writer.flush().unwrap();
}

impl Drop for LspServer {
    fn drop(&mut self) {
        // Best-effort cleanup if shutdown wasn't called.
        if self.process.try_wait().ok().flatten().is_none() {
            let _ = self.process.kill();
            let _ = self.process.wait();
        }
    }
}
