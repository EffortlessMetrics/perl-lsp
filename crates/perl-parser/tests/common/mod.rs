#![allow(dead_code)] // Common test utilities - some may not be used by all test files

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

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
}

fn resolve_perl_lsp_bin() -> Command {
    // Prefer Cargo-provided binary paths when running integration tests
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_perl-lsp") {
        let mut cmd = Command::new(p);
        cmd.arg("--stdio");
        return cmd;
    }
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_perl_lsp") {
        let mut cmd = Command::new(p);
        cmd.arg("--stdio");
        return cmd;
    }
    // Fallback: use cargo run
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "-q",
        "-p",
        "perl-parser",
        "--bin",
        "perl-lsp",
        "--",
        "--stdio",
    ]);
    cmd
}

pub fn start_lsp_server() -> LspServer {
    // Spawn the language server
    let mut cmd = resolve_perl_lsp_bin();
    #[allow(clippy::zombie_processes)] // Process is owned by LspServer and cleaned up in Drop
    let mut process = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start perl-lsp");

    let stdin = process.stdin.take();

    // -------- stderr drain thread (prevents child from blocking on logs) --------
    let stderr = process.stderr.take().expect("stderr piped");
    let _stderr_thread = std::thread::spawn(move || {
        let mut r = BufReader::new(stderr);
        let mut line = String::new();
        while r.read_line(&mut line).unwrap_or(0) > 0 {
            // discard or mirror to eprintln! if needed
            line.clear();
        }
    });

    // -------- stdout LSP reader thread --------
    let stdout = process.stdout.take().expect("stdout piped");
    let (tx, rx) = mpsc::channel::<Value>();
    let _stdout_thread = std::thread::spawn(move || {
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
                        if let Some(rest) = l.strip_prefix("Content-Length:") {
                            content_len = rest.trim().parse::<usize>().ok();
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
    });

    LspServer {
        process,
        stdin,
        rx,
        _stdout_thread,
        _stderr_thread,
    }
}

pub fn send_request(server: &mut LspServer, request: Value) -> Value {
    let request_str = serde_json::to_string(&request).unwrap();
    let stdin = server.stdin.as_mut().unwrap();

    // Extract the request ID for matching the response
    let request_id = request["id"].clone();

    writeln!(
        stdin,
        "Content-Length: {}\r\n\r\n{}",
        request_str.len(),
        request_str
    )
    .unwrap();
    stdin.flush().unwrap();

    // Read responses until we find the one matching our request ID
    loop {
        let response = read_response(server);

        // Check if this is a response to our request (has matching ID)
        if let Some(id) = response.get("id") {
            if id == &request_id {
                return response;
            }
        }

        // If it's a notification or different response, continue reading
        // But only continue if we got valid JSON (not null)
        if response.is_null() {
            // No more messages available, return null
            return response;
        }
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

/// Blocking receive with a sane default timeout to avoid hangs.
pub fn read_response(server: &mut LspServer) -> Value {
    read_response_timeout(server, Duration::from_secs(5)).unwrap_or(json!(null))
}

/// Try to receive a response within `dur`. Returns None on timeout.
pub fn read_response_timeout(server: &mut LspServer, dur: Duration) -> Option<Value> {
    server.rx.recv_timeout(dur).ok()
}

pub fn initialize_lsp(server: &mut LspServer) -> Value {
    send_request(
        server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": null,
                "rootUri": "file:///test",
                "capabilities": {
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
                }
            }
        }),
    )
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
        // Send exit notification
        if let Some(stdin) = &mut self.stdin {
            let exit = json!({
                "jsonrpc": "2.0",
                "method": "exit",
                "params": null
            });
            let exit_str = serde_json::to_string(&exit).unwrap();
            let _ = writeln!(
                stdin,
                "Content-Length: {}\r\n\r\n{}",
                exit_str.len(),
                exit_str
            );
        }

        // Kill process if still running
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}
