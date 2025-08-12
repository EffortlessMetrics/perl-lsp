use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

pub struct LspServer {
    pub process: Child,
    pub stdin: Option<std::process::ChildStdin>,
    pub stdout: Option<BufReader<std::process::ChildStdout>>,
}

pub fn start_lsp_server() -> LspServer {
    let mut process = Command::new("cargo")
        .args([
            "run",
            "-p",
            "perl-parser",
            "--bin",
            "perl-lsp",
            "--",
            "--stdio",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start LSP server");

    let stdin = process.stdin.take();
    let stdout = process.stdout.take().map(BufReader::new);

    LspServer {
        process,
        stdin,
        stdout,
    }
}

pub fn send_request(server: &mut LspServer, request: Value) -> Value {
    let request_str = serde_json::to_string(&request).unwrap();
    let stdin = server.stdin.as_mut().unwrap();

    writeln!(
        stdin,
        "Content-Length: {}\r\n\r\n{}",
        request_str.len(),
        request_str
    )
    .unwrap();
    stdin.flush().unwrap();

    read_response(server)
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
    let stdout = server.stdout.as_mut().unwrap();
    let mut headers = String::new();

    // Read headers
    loop {
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();

        if line == "\r\n" || line == "\n" {
            break;
        }
        headers.push_str(&line);
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
    stdout.read_exact(&mut content).unwrap();

    serde_json::from_slice(&content).unwrap_or(json!(null))
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
