use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Integration test for TCP socket mode.
/// Spawns the LSP server in socket mode, connects, and verifies the initialize handshake.
#[test]
fn test_socket_connection() -> Result<(), Box<dyn std::error::Error>> {
    let bin_path = env!("CARGO_BIN_EXE_perl-lsp");

    // Start the server in socket mode on a random port (port 0)
    let mut child = Command::new(bin_path)
        .arg("--socket")
        .arg("--port")
        .arg("0")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
    let reader = BufReader::new(stderr);

    // Read stderr to find the port
    let mut lines = reader.lines();
    let mut port = 0;
    for line_res in lines.by_ref() {
        let line = line_res?;
        println!("Server startup: {}", line); // Debug output
        if line.contains("Perl LSP listening on") {
            // Parse port from "Perl LSP listening on 127.0.0.1:12345"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(addr) = parts.last() {
                if let Some(port_str) = addr.split(':').next_back() {
                    port = port_str.parse()?;
                    break;
                }
            }
        }
    }

    assert_ne!(port, 0, "Failed to determine server port");

    // Spawn thread to continue reading stderr
    thread::spawn(move || {
        for l in lines.map_while(Result::ok) {
            println!("SERVER LOG: {}", l);
        }
    });

    // Connect to the server with timeout
    let stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    // Clone stream for reading/writing - BufReader will own the read half
    let mut write_stream = stream.try_clone()?;

    // Send initialize request
    let request = r#"{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"processId": null, "rootUri": null, "capabilities": {}}}"#;
    let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
    write_stream.write_all(message.as_bytes())?;
    write_stream.flush()?;

    // Read response
    let mut reader = BufReader::new(stream);

    // Read headers
    let mut content_length = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line == "\r\n" {
            break;
        }
        if line.starts_with("Content-Length: ") {
            content_length = line.trim()["Content-Length: ".len()..].parse()?;
        }
    }

    assert!(content_length > 0, "Content-Length should be positive");

    // Read body
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body)?;
    let response_str = String::from_utf8(body)?;

    // Validate response
    assert!(response_str.contains("\"result\""), "Response should contain result");
    assert!(response_str.contains("\"capabilities\""), "Response should contain capabilities");

    // Send shutdown request
    let shutdown_request = r#"{"jsonrpc": "2.0", "id": 2, "method": "shutdown"}"#;
    let message = format!("Content-Length: {}\r\n\r\n{}", shutdown_request.len(), shutdown_request);
    write_stream.write_all(message.as_bytes())?;
    write_stream.flush()?;

    // Send exit notification for graceful shutdown
    let exit_notification = r#"{"jsonrpc": "2.0", "method": "exit"}"#;
    let exit_message =
        format!("Content-Length: {}\r\n\r\n{}", exit_notification.len(), exit_notification);
    let _ = write_stream.write_all(exit_message.as_bytes());
    let _ = write_stream.flush();

    // Give server time to exit gracefully before force-killing
    std::thread::sleep(std::time::Duration::from_millis(100));
    let _ = child.kill();

    Ok(())
}
