//! Common test utilities for LSP integration tests
//!
//! Env toggles:
//!   LSP_TEST_TIMEOUT_MS   – default per-request timeout (ms), default 5000
//!   LSP_TEST_SHORT_MS     – "short" timeout for optional responses (ms), default 500
//!   LSP_TEST_ECHO_STDERR  – if set, echo perl-lsp stderr lines in tests

#![allow(dead_code)] // Common test utilities - some may not be used by all test files

use serde_json::{Value, json};
use std::collections::VecDeque;

const PENDING_CAP: usize = 512; // Prevent unbounded growth of pending message queue
use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

/// Get completion items from a response, handling both array and object formats
pub fn completion_items(resp: &serde_json::Value) -> &Vec<serde_json::Value> {
    resp["result"]["items"]
        .as_array()
        .or_else(|| resp["result"].as_array())
        .expect("completion result should be array or { items: [] }")
}

pub struct LspServer {
    pub process: Child,
    pub stdin: Option<std::process::ChildStdin>,
    rx: Receiver<Value>,
    // Keep threads alive for the lifetime of the server
    _stdout_thread: std::thread::JoinHandle<()>,
    _stderr_thread: std::thread::JoinHandle<()>,
    pending: VecDeque<Value>,
}

fn resolve_perl_lsp_cmds() -> impl Iterator<Item = Command> {
    // Try CARGO_BIN_EXE_* first, then PATH, then cargo run
    let mut v: Vec<Command> = Vec::new();

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

    // Try perl-lsp from PATH
    {
        let mut c = Command::new("perl-lsp");
        c.arg("--stdio");
        v.push(c);
    }

    // Fallback: use cargo run
    {
        let mut c = Command::new("cargo");
        c.args([
            "run",
            "-q",
            "-p",
            "perl-parser",
            "--bin",
            "perl-lsp",
            "--",
            "--stdio",
        ]);
        v.push(c);
    }

    v.into_iter()
}

pub fn start_lsp_server() -> LspServer {
    // Try candidates in order; fall back cleanly on NotFound
    let mut last_err: Option<io::Error> = None;
    let mut process: Child = {
        let mut spawned: Option<Child> = None;
        for mut cmd in resolve_perl_lsp_cmds() {
            match cmd
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
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
            panic!(
                "Failed to start perl-lsp via CARGO_BIN_EXE, PATH, or cargo run: {:?}",
                last_err
            )
        })
    };

    let stdin = process.stdin.take();

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
    let _stdout_thread = std::thread::Builder::new()
        .name("lsp-stdout-reader".into())
        .spawn(move || {
            let mut r = BufReader::new(stdout);
            loop {
                // Parse headers
                let mut content_len: Option<usize> = None;
                let mut line = String::new();
                loop {
                    line.clear();
                    match r.read_line(&mut line) {
                        Ok(0) => return, // EOF
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
                        Err(_) => return,
                    }
                }
                let len = match content_len {
                    Some(n) => n,
                    None => continue,
                };
                // Read body
                let mut buf = vec![0u8; len];
                if r.read_exact(&mut buf).is_err() {
                    return;
                }
                if let Ok(val) = serde_json::from_slice::<Value>(&buf) {
                    let _ = tx.send(val);
                }
            }
        })
        .unwrap();

    LspServer {
        process,
        stdin,
        rx,
        _stdout_thread,
        _stderr_thread,
        pending: VecDeque::new(),
    }
}

pub fn send_request(server: &mut LspServer, request: Value) -> Value {
    use std::io::Write as _;
    let id = request.get("id").cloned();
    let body = request.to_string();
    let header = format!("Content-Length: {}\r\n\r\n", body.len());

    let w = server.stdin.as_mut().expect("child stdin");
    w.write_all(header.as_bytes()).unwrap();
    w.write_all(body.as_bytes()).unwrap();
    w.flush().unwrap();

    // Match by ID to avoid confusion with interleaved notifications
    match id {
        Some(Value::Number(n)) if n.as_i64().is_some() => {
            read_response_matching_i64(server, n.as_i64().unwrap(), default_timeout())
                .unwrap_or(json!(null))
        }
        Some(v) => read_response_matching(server, &v, default_timeout()).unwrap_or(json!(null)),
        None => read_response(server),
    }
}

pub fn send_notification(server: &mut LspServer, notification: Value) {
    let notification_str = serde_json::to_string(&notification).unwrap();
    let stdin = server.stdin.as_mut().unwrap();

    writeln!(
        stdin,
        "Content-Length: {}\r\n\r\n{}",
        notification_str.len(),
        notification_str
    )
    .unwrap();
    stdin.flush().unwrap();
}

fn default_timeout() -> Duration {
    std::env::var("LSP_TEST_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_secs(5))
}

/// Short timeout for expected non-responses (malformed requests, etc)
pub fn short_timeout() -> Duration {
    std::env::var("LSP_TEST_SHORT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_millis(500))
}

/// Blocking receive with a sane default timeout to avoid hangs.
pub fn read_response(server: &mut LspServer) -> Value {
    read_response_timeout(server, default_timeout()).unwrap_or(json!(null))
}

/// Try to receive a response within `dur`. Returns None on timeout.
pub fn read_response_timeout(server: &mut LspServer, dur: Duration) -> Option<Value> {
    server.rx.recv_timeout(dur).ok()
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
    use std::io::Write as _;
    let w = server.stdin.as_mut().expect("child stdin");
    w.write_all(bytes).unwrap();
    w.flush().unwrap();
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
        match server
            .rx
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
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
        use std::io::Write as _;
        let w = server.stdin.as_mut().expect("child stdin");
        let body = init.to_string();
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        w.write_all(header.as_bytes()).unwrap();
        w.write_all(body.as_bytes()).unwrap();
        w.flush().unwrap();
    }

    // wait specifically for id=1
    let resp =
        read_response_matching_i64(server, 1, default_timeout()).expect("initialize response");

    // send initialized notification
    send_notification(server, json!({"jsonrpc":"2.0","method":"initialized"}));

    resp
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

impl Drop for LspServer {
    fn drop(&mut self) {
        // Best-effort cleanup if shutdown wasn't called.
        if self.process.try_wait().ok().flatten().is_none() {
            let _ = self.process.kill();
            let _ = self.process.wait();
        }
    }
}
