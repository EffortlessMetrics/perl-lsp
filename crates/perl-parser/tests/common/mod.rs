#![allow(dead_code)] // Common test utilities - some may not be used by all test files

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

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
    pub stdout: Option<BufReader<std::process::ChildStdout>>,
    // Keep a handle so the drain thread isn't dropped immediately.
    #[allow(dead_code)]
    stderr_drain: Option<std::thread::JoinHandle<()>>,
}

pub fn start_lsp_server() -> LspServer {
    // Use cargo run but discard stderr entirely to avoid blocking
    let mut process = Command::new("cargo")
        .args([
            "run",
            "-q", // Quiet mode to reduce output
            "-p",
            "perl-parser",
            "--bin",
            "perl-lsp",
            "--",
            "--stdio",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null()) // Discard stderr completely
        .spawn()
        .expect("Failed to start LSP server");

    let stdin = process.stdin.take();
    let stdout = process.stdout.take().map(BufReader::new);

    // Give the server a moment to start up
    std::thread::sleep(std::time::Duration::from_millis(500));

    LspServer {
        process,
        stdin,
        stdout,
        stderr_drain: None,
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

pub fn read_response(server: &mut LspServer) -> Value {
    use std::io::ErrorKind;
    use std::time::{Duration, Instant};

    let stdout = server.stdout.as_mut().unwrap();
    let mut headers = String::new();
    let timeout = Duration::from_secs(5);
    let start = Instant::now();

    // Read headers with timeout check
    loop {
        if start.elapsed() > timeout {
            eprintln!("Timeout reading headers");
            return json!(null);
        }

        let mut line = String::new();
        match stdout.read_line(&mut line) {
            Ok(0) => {
                // EOF reached
                return json!(null);
            }
            Ok(_) => {
                if line == "\r\n" || line == "\n" {
                    break;
                }
                headers.push_str(&line);
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                // Non-blocking read, retry
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(e) => {
                eprintln!("Error reading headers: {}", e);
                return json!(null);
            }
        }
    }

    // Parse content length
    let content_length = headers
        .lines()
        .find(|line| line.starts_with("Content-Length:"))
        .and_then(|line| line.split(':').nth(1))
        .and_then(|len| len.trim().parse::<usize>().ok())
        .unwrap_or(0);

    if content_length == 0 {
        return json!(null);
    }

    // Read content
    let mut content = vec![0u8; content_length];
    use std::io::Read;
    match stdout.read_exact(&mut content) {
        Ok(_) => serde_json::from_slice(&content).unwrap_or(json!(null)),
        Err(e) => {
            eprintln!("Error reading content: {}", e);
            json!(null)
        }
    }
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
