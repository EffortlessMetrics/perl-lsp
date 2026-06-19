use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

struct ChildGuard {
    child: Child,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn reserve_local_port() -> Result<u16, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

fn connect_with_deadline(port: u16) -> Result<TcpStream, Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut last_error = None;

    while Instant::now() < deadline {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(stream) => return Ok(stream),
            Err(error) => {
                last_error = Some(error);
                thread::sleep(Duration::from_millis(50));
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        format!("timed out waiting for perl-lsp socket on 127.0.0.1:{port}: {last_error:?}"),
    )
    .into())
}

/// Integration test for TCP socket mode.
/// Spawns the LSP server in socket mode, connects, and verifies the initialize handshake.
#[test]
fn test_socket_connection() -> Result<(), Box<dyn std::error::Error>> {
    let bin_path = env!("CARGO_BIN_EXE_perl-lsp");
    let port = reserve_local_port()?;

    let child = Command::new(bin_path)
        .arg("--socket")
        .arg("--port")
        .arg(port.to_string())
        .env("PERL_LSP_QUIET", "1")
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .spawn()?;
    let child = ChildGuard::new(child);

    // Connect to the server with timeout
    let stream = connect_with_deadline(port)?;
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
    drop(child);

    Ok(())
}
