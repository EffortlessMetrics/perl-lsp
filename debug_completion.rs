/// Debug completion test
use serde_json::json;
use std::process::{Command, Stdio};
use std::io::{Write, BufRead, BufReader};

fn main() {
    // Start LSP server
    let mut process = Command::new("cargo")
        .args(&["run", "-p", "perl-parser", "--bin", "perl-lsp", "--", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start LSP server");
    
    let mut stdin = process.stdin.take().unwrap();
    let mut stdout = BufReader::new(process.stdout.take().unwrap());
    
    // Send initialize request
    let init_request = json!({
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
                    }
                }
            }
        }
    });
    
    let request_str = serde_json::to_string(&init_request).unwrap();
    write!(stdin, "Content-Length: {}\r\n\r\n{}", request_str.len(), request_str).unwrap();
    stdin.flush().unwrap();
    
    // Read response
    let mut headers = String::new();
    loop {
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        if line == "\r\n" || line == "\n" {
            break;
        }
        headers.push_str(&line);
    }
    
    let content_length: usize = headers
        .lines()
        .find(|line| line.starts_with("Content-Length:"))
        .and_then(|line| line.split(':').nth(1))
        .and_then(|len| len.trim().parse().ok())
        .unwrap();
    
    let mut content = vec![0u8; content_length];
    use std::io::Read;
    stdout.read_exact(&mut content).unwrap();
    
    let response: serde_json::Value = serde_json::from_slice(&content).unwrap();
    println!("Initialize response: {}", serde_json::to_string_pretty(&response).unwrap());
    
    // Open document
    let open_notification = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///test.pl",
                "languageId": "perl",
                "version": 1,
                "text": "pri"
            }
        }
    });
    
    let notification_str = serde_json::to_string(&open_notification).unwrap();
    write!(stdin, "Content-Length: {}\r\n\r\n{}", notification_str.len(), notification_str).unwrap();
    stdin.flush().unwrap();
    
    // Wait a bit for processing
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Request completion
    let completion_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 3 }
        }
    });
    
    let request_str = serde_json::to_string(&completion_request).unwrap();
    write!(stdin, "Content-Length: {}\r\n\r\n{}", request_str.len(), request_str).unwrap();
    stdin.flush().unwrap();
    
    // Read completion response
    let mut headers = String::new();
    loop {
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        if line == "\r\n" || line == "\n" {
            break;
        }
        headers.push_str(&line);
    }
    
    let content_length: usize = headers
        .lines()
        .find(|line| line.starts_with("Content-Length:"))
        .and_then(|line| line.split(':').nth(1))
        .and_then(|len| len.trim().parse().ok())
        .unwrap();
    
    let mut content = vec![0u8; content_length];
    stdout.read_exact(&mut content).unwrap();
    
    let response: serde_json::Value = serde_json::from_slice(&content).unwrap();
    println!("Completion response: {}", serde_json::to_string_pretty(&response).unwrap());
    
    // Kill server
    let _ = process.kill();
}