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

// Test reliability and timeout utilities
pub mod test_reliability;
pub mod timeout_scaler;

// Error codes aligned with crates/perl-parser/src/lsp/protocol/errors.rs
/// JSON-RPC reserved: Server error range is -32000 to -32099
const ERR_TEST_TIMEOUT: i64 = -32000;
/// Connection closed - BrokenPipe or similar transport termination
const ERR_CONNECTION_CLOSED: i64 = -32050;
/// Transport error - general I/O failures that aren't connection closures
const ERR_TRANSPORT_ERROR: i64 = -32051;

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
    match resp["result"]["items"].as_array() {
        Some(arr) => arr,
        None => match resp["result"].as_array() {
            Some(arr) => arr,
            None => must(Err::<(), _>(format!(
                "completion result should be array or {{ items: [] }}, got: {resp:?}"
            ))),
        },
    }
}

pub struct LspServer {
    pub process: Child,
    writer: BufWriter<ChildStdin>, // keep stdin pinned and flushed
    rx: Receiver<Value>,
    // Keep threads alive for the lifetime of the server
    _stdout_thread: std::thread::JoinHandle<()>,
    _stderr_thread: std::thread::JoinHandle<()>,
    pending: VecDeque<Value>,
    /// Flag to track if shutdown has been initiated (prevents double-shutdown)
    shutdown_initiated: std::sync::atomic::AtomicBool,
}

impl LspServer {
    /// Check if the server process is still running
    pub fn is_alive(&mut self) -> bool {
        match self.process.try_wait() {
            Ok(status) => status.is_none(),
            Err(_) => false, // If we can't check status, assume not alive
        }
    }

    /// Get mutable access to the stdin writer
    pub fn stdin_writer(&mut self) -> &mut BufWriter<ChildStdin> {
        &mut self.writer
    }
}

/// Compile-time path to the perl-lsp binary, set by Cargo when building integration tests.
/// This is the most reliable way to get the correct binary path.
const CARGO_BIN_EXE: Option<&str> = option_env!("CARGO_BIN_EXE_perl-lsp");

fn resolve_perl_lsp_cmds() -> impl Iterator<Item = Command> {
    // Resolution order (fixed for test reliability):
    // 1. PERL_LSP_BIN env var (explicit override, useful for custom target dirs)
    // 2. Compile-time CARGO_BIN_EXE (guaranteed correct during `cargo test -p perl-lsp`)
    // 3. Runtime CARGO_BIN_EXE_* (fallback for edge cases)
    // 4. Workspace target directory binaries (DEBUG first, then release)
    // 5. PATH lookup
    // 6. cargo run fallback (slow but always works)
    //
    // IMPORTANT: Debug binary is checked BEFORE release to avoid stale release binaries
    // causing test failures. When you run `cargo test -p perl-lsp`, cargo builds debug.
    let mut v: Vec<Command> = Vec::new();

    // 1. Explicit override via PERL_LSP_BIN
    if let Ok(p) = std::env::var("PERL_LSP_BIN") {
        let mut c = Command::new(p);
        c.arg("--stdio");
        v.push(c);
    }

    // 2. Compile-time CARGO_BIN_EXE (most reliable for `cargo test`)
    // This is set at compile time by Cargo and points to the exact binary that was built
    if let Some(p) = CARGO_BIN_EXE {
        let mut c = Command::new(p);
        c.arg("--stdio");
        v.push(c);
    }

    // 3. Runtime CARGO_BIN_EXE_* (fallback, in case compile-time wasn't set)
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

    // 4. Try workspace target directory binaries (using absolute paths)
    // IMPORTANT: Debug BEFORE release to avoid stale release binary issues
    // CARGO_MANIFEST_DIR points to the crate directory, we need the workspace root
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let crate_dir = std::path::Path::new(&manifest_dir);
        // Walk up to find workspace root (contains Cargo.toml with [workspace])
        let workspace_root =
            crate_dir.ancestors().find(|p| p.join("Cargo.lock").exists()).unwrap_or(crate_dir);

        // Try DEBUG binary first (this is what `cargo test` builds by default)
        let debug_binary = workspace_root.join("target/debug/perl-lsp");
        if debug_binary.exists() {
            let mut c = Command::new(&debug_binary);
            c.arg("--stdio");
            v.push(c);
        }

        // Then try release binary (only if debug doesn't exist)
        let release_binary = workspace_root.join("target/release/perl-lsp");
        if release_binary.exists() {
            let mut c = Command::new(&release_binary);
            c.arg("--stdio");
            v.push(c);
        }
    }

    // 5. Try perl-lsp from PATH
    {
        let mut c = Command::new("perl-lsp");
        c.arg("--stdio");
        v.push(c);
    }

    // 6. Fallback: use cargo run with debug profile (matches what tests build)
    // This is SLOW because it may need to compile, but always works
    {
        let mut c = Command::new("cargo");
        c.args(["run", "-q", "-p", "perl-lsp", "--", "--stdio"]);
        v.push(c);
    }

    v.into_iter()
}

pub fn start_lsp_server() -> LspServer {
    // Serialize LSP server creation to prevent resource conflicts during concurrent testing
    let _guard = match LSP_SERVER_MUTEX.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(poisoned) => {
            eprintln!("Warning: LSP_SERVER_MUTEX was poisoned, recovering...");
            poisoned.into_inner()
        }
    };

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
            eprintln!("╔════════════════════════════════════════════════════════════════════╗");
            eprintln!("║ ERROR: Failed to start perl-lsp server                             ║");
            eprintln!("╠════════════════════════════════════════════════════════════════════╣");
            eprintln!("║ Resolution order tried:                                            ║");
            eprintln!("║  1. PERL_LSP_BIN env var: {:?}", std::env::var("PERL_LSP_BIN").ok());
            eprintln!("║  2. Compile-time CARGO_BIN_EXE: {:?}", CARGO_BIN_EXE);
            eprintln!(
                "║  3. Runtime CARGO_BIN_EXE_perl-lsp: {:?}",
                std::env::var("CARGO_BIN_EXE_perl-lsp").ok()
            );
            eprintln!(
                "║  4. Runtime CARGO_BIN_EXE_perl_lsp: {:?}",
                std::env::var("CARGO_BIN_EXE_perl_lsp").ok()
            );
            if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
                let crate_dir = std::path::Path::new(&manifest_dir);
                let workspace_root = crate_dir
                    .ancestors()
                    .find(|p| p.join("Cargo.lock").exists())
                    .unwrap_or(crate_dir);
                let debug_binary = workspace_root.join("target/debug/perl-lsp");
                let release_binary = workspace_root.join("target/release/perl-lsp");
                eprintln!(
                    "║  5. Debug binary exists: {} ({})",
                    debug_binary.exists(),
                    debug_binary.display()
                );
                eprintln!(
                    "║  6. Release binary exists: {} ({})",
                    release_binary.exists(),
                    release_binary.display()
                );
            }
            eprintln!("║  7. perl-lsp in PATH: {:?}", which::which("perl-lsp").ok());
            eprintln!("║  8. cargo run fallback");
            eprintln!("╠════════════════════════════════════════════════════════════════════╣");
            eprintln!("║ Last error: {:?}", last_err);
            eprintln!("╠════════════════════════════════════════════════════════════════════╣");
            eprintln!("║ HINTS:                                                             ║");
            eprintln!("║  • Run: cargo build -p perl-lsp   (builds debug binary)            ║");
            eprintln!("║  • Or:  cargo test -p perl-lsp    (builds + tests automatically)   ║");
            eprintln!("║  • Set PERL_LSP_BIN=/path/to/perl-lsp for custom binary            ║");
            eprintln!("╚════════════════════════════════════════════════════════════════════╝");
            must(Err::<(), _>(format!(
                "Failed to start perl-lsp via any available method: {:?}",
                last_err
            )))
        })
    };

    let stdin = match process.stdin.take() {
        Some(s) => s,
        None => must(Err::<(), _>(format!("child stdin should be available after spawn"))),
    };

    // -------- stderr drain thread (prevents child from blocking on logs) --------
    let stderr = match process.stderr.take() {
        Some(s) => s,
        None => must(Err::<(), _>(format!("stderr should be piped after spawn"))),
    };
    let echo = std::env::var_os("LSP_TEST_ECHO_STDERR").is_some();
    let _stderr_thread =
        match std::thread::Builder::new().name("lsp-stderr-drain".into()).spawn(move || {
            let mut r = BufReader::new(stderr);
            let mut line = String::new();
            while r.read_line(&mut line).unwrap_or(0) > 0 {
                if echo {
                    eprintln!("[perl-lsp] {}", line.trim_end());
                }
                line.clear();
            }
        }) {
            Ok(handle) => handle,
            Err(e) => must(Err::<(), _>(format!("Failed to spawn stderr drain thread: {e}"))),
        };

    // -------- stdout LSP reader thread --------
    let stdout = match process.stdout.take() {
        Some(s) => s,
        None => must(Err::<(), _>(format!("stdout should be piped after spawn"))),
    };
    let (tx, rx) = mpsc::channel::<Value>();
    let debug_reader = std::env::var_os("LSP_TEST_DEBUG_READER").is_some();
    let _stdout_thread =
        match std::thread::Builder::new().name("lsp-stdout-reader".into()).spawn(move || {
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
        }) {
            Ok(handle) => handle,
            Err(e) => must(Err::<(), _>(format!("Failed to spawn stdout reader thread: {e}"))),
        };

    let server = LspServer {
        process,
        writer: BufWriter::new(stdin),
        rx,
        _stdout_thread,
        _stderr_thread,
        pending: VecDeque::new(),
        shutdown_initiated: std::sync::atomic::AtomicBool::new(false),
    };

    // Brief delay to allow server to fully initialize before returning
    std::thread::sleep(Duration::from_millis(100));

    server
}

pub fn send_request(server: &mut LspServer, mut request: Value) -> Value {
    // IMPORTANT: Extract/assign ID FIRST, before any early returns.
    // This ensures error responses can include the proper request ID.
    let id = match request.get("id") {
        Some(v) => v.clone(),
        None => {
            let nid = next_id();
            request["id"] = json!(nid);
            json!(nid)
        }
    };

    let body = request.to_string();
    if let Err(e) = send_message_inner(&mut server.writer, &body) {
        // Handle write errors gracefully with proper JSON-RPC envelope
        // BrokenPipe during teardown is expected; other errors are transport failures
        return map_send_error(Some(id), e, "send_request");
    }

    // Match by ID to avoid confusion with interleaved notifications
    match &id {
        Value::Number(n) if n.as_i64().is_some() => {
            // Safe unwrap: we just checked is_some() in the match guard
            let id_num = match n.as_i64() {
                Some(num) => num,
                None => must(Err::<(), _>(format!("ID number should be i64: {n:?}"))),
            };
            match read_response_matching_i64(server, id_num, default_timeout()) {
                Some(resp) => resp,
                None => error_response_for_request(
                    Some(id.clone()),
                    ERR_TEST_TIMEOUT,
                    "test harness timeout",
                ),
            }
        }
        v => match read_response_matching(server, v, default_timeout()) {
            Some(resp) => resp,
            None => error_response_for_request(
                Some(id.clone()),
                ERR_TEST_TIMEOUT,
                "test harness timeout",
            ),
        },
    }
}

/// Maps I/O send errors to proper JSON-RPC error responses.
///
/// BrokenPipe → CONNECTION_CLOSED (-32050)
/// Other I/O errors → TRANSPORT_ERROR (-32051)
fn map_send_error(id: Option<Value>, e: io::Error, context: &str) -> Value {
    if e.kind() == io::ErrorKind::BrokenPipe {
        connection_closed_error_for_request(id)
    } else {
        transport_error_for_request(id, &format!("{}: {}", context, e))
    }
}

pub fn send_notification(server: &mut LspServer, notification: Value) {
    let body = notification.to_string();
    // Ignore write errors during notification sends - BrokenPipe during teardown is expected
    let _ = send_message_inner(&mut server.writer, &body);
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

/// Helper function to send a JSON-RPC message over the wire.
/// Returns io::Result to allow graceful error handling.
fn send_message_inner(writer: &mut impl Write, body: &str) -> io::Result<()> {
    write!(writer, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    writer.flush()
}

/// Creates a JSON-RPC 2.0 error response with proper envelope.
///
/// All error responses MUST include `jsonrpc` and `id` fields per JSON-RPC 2.0 spec.
/// The `id` should be extracted from the original request before any error handling.
fn error_response_for_request(id: Option<Value>, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "error": {
            "code": code,
            "message": message
        }
    })
}

/// Creates an error response for connection-closed scenarios (BrokenPipe).
fn connection_closed_error_for_request(id: Option<Value>) -> Value {
    error_response_for_request(id, ERR_CONNECTION_CLOSED, "Connection closed")
}

/// Creates an error response for internal transport errors.
fn transport_error_for_request(id: Option<Value>, msg: &str) -> Value {
    error_response_for_request(id, ERR_TRANSPORT_ERROR, msg)
}

// Legacy functions for backward compatibility with code that doesn't have request context
// These return responses with null id (valid JSON-RPC but less informative)

/// Creates an error response for connection-closed scenarios (legacy, no request context).
fn connection_closed_error() -> Value {
    connection_closed_error_for_request(None)
}

/// Creates an error response for internal transport errors (legacy, no request context).
fn internal_transport_error(msg: &str) -> Value {
    transport_error_for_request(None, msg)
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
    // Ignore write errors - BrokenPipe during teardown is expected
    let _ = server.writer.write_all(bytes).and_then(|_| server.writer.flush());
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
        if let Err(e) = send_message_inner(&mut server.writer, &body) {
            // Handle write errors gracefully with proper JSON-RPC envelope (id=1)
            return map_send_error(Some(json!(1)), e, "initialize");
        }
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
                    must(Err::<(), _>(format!(
                        "initialize response timeout - server may have crashed or is not responding"
                    )))
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
    use std::sync::atomic::Ordering;

    // Mark shutdown as initiated to prevent duplicate shutdown in Drop
    if server.shutdown_initiated.swap(true, Ordering::SeqCst) {
        // Already initiated, just wait for exit
        for _ in 0..20 {
            if server.process.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        return;
    }

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
    // Ignore write errors - BrokenPipe during teardown is expected
    let _ = send_message_inner(&mut server.writer, content);
}

/// Send request without waiting for response
pub fn send_request_no_wait(server: &mut LspServer, req: Value) {
    let body = req.to_string();
    // Ignore write errors - BrokenPipe during teardown is expected
    let _ = send_message_inner(&mut server.writer, &body);
}

impl Drop for LspServer {
    fn drop(&mut self) {
        use std::sync::atomic::Ordering;

        // Check if already exited
        if self.process.try_wait().ok().flatten().is_some() {
            return;
        }

        // Check if shutdown was already initiated by explicit call
        if self.shutdown_initiated.swap(true, Ordering::SeqCst) {
            // Already initiated, just wait for exit then force-kill if needed
            for _ in 0..50 {
                if self.process.try_wait().ok().flatten().is_some() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let _ = self.process.kill();
            let _ = self.process.wait();
            return;
        }

        // Best-effort graceful shutdown (never panic in Drop)
        // 1. Try to send shutdown request
        let shutdown_body = r#"{"jsonrpc":"2.0","id":999999,"method":"shutdown","params":{}}"#;
        let _ = send_message_inner(&mut self.writer, shutdown_body);

        // 2. Try to send exit notification
        let exit_body = r#"{"jsonrpc":"2.0","method":"exit"}"#;
        let _ = send_message_inner(&mut self.writer, exit_body);

        // 3. Wait briefly for graceful exit (max 500ms)
        for _ in 0..50 {
            if self.process.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        // 4. Fall back to hard kill if graceful shutdown didn't work
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}
