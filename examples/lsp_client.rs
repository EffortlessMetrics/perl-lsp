//! Example: Simple LSP client demonstration

use std::io::{self, BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use serde_json::json;

fn main() -> io::Result<()> {
    println!("Starting Perl Language Server...");
    
    // Start the LSP server
    let mut child = Command::new("perl-lsp")
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    
    // Send initialize request
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": std::process::id(),
            "rootUri": "file:///tmp/perl-project",
            "capabilities": {
                "textDocument": {
                    "publishDiagnostics": {
                        "relatedInformation": true
                    }
                }
            }
        }
    });
    
    send_message(&mut stdin, &initialize_request)?;
    
    // Read response
    if let Some(response) = read_message(&mut reader)? {
        println!("Initialize response: {}", serde_json::to_string_pretty(&response)?);
    }
    
    // Send initialized notification
    let initialized = json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    
    send_message(&mut stdin, &initialized)?;
    
    // Open a document
    let did_open = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///tmp/test.pl",
                "languageId": "perl",
                "version": 1,
                "text": "#!/usr/bin/perl\nuse strict;\nuse warnings;\n\nmy $x = 42;\nprint \"Hello, $x\\n\";\n"
            }
        }
    });
    
    send_message(&mut stdin, &did_open)?;
    
    // Wait for diagnostics
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Request document symbols
    let doc_symbols = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/documentSymbol",
        "params": {
            "textDocument": {
                "uri": "file:///tmp/test.pl"
            }
        }
    });
    
    send_message(&mut stdin, &doc_symbols)?;
    
    // Read response
    if let Some(response) = read_message(&mut reader)? {
        println!("Document symbols: {}", serde_json::to_string_pretty(&response)?);
    }
    
    // Shutdown
    let shutdown = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "shutdown",
        "params": null
    });
    
    send_message(&mut stdin, &shutdown)?;
    
    // Exit notification
    let exit = json!({
        "jsonrpc": "2.0",
        "method": "exit",
        "params": null
    });
    
    send_message(&mut stdin, &exit)?;
    
    child.wait()?;
    println!("LSP server shut down successfully");
    
    Ok(())
}

fn send_message(writer: &mut impl Write, message: &serde_json::Value) -> io::Result<()> {
    let content = message.to_string();
    let header = format!("Content-Length: {}\r\n\r\n", content.len());
    writer.write_all(header.as_bytes())?;
    writer.write_all(content.as_bytes())?;
    writer.flush()?;
    Ok(())
}

fn read_message(reader: &mut impl BufRead) -> io::Result<Option<serde_json::Value>> {
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line == "\r\n" {
            break;
        }
        headers.push(line);
    }
    
    // Parse Content-Length
    let content_length = headers.iter()
        .find_map(|h| {
            if h.starts_with("Content-Length: ") {
                h.trim_start_matches("Content-Length: ")
                    .trim()
                    .parse::<usize>()
                    .ok()
            } else {
                None
            }
        })?;
    
    // Read content
    let mut content = vec![0; content_length];
    reader.read_exact(&mut content)?;
    
    let value = serde_json::from_slice(&content).ok()?;
    Some(value)
}