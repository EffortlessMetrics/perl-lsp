//! End-to-end DAP attach smoke test using a loopback TCP debugger.

// Tests use panic! as structured test failure reporters.
#![allow(clippy::panic)]

use perl_content_length_framing::frame;
use perl_dap::{DapMessage, DebugAdapter};
use serde_json::{Value, json};
use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{Duration, Instant};

type TestResult = Result<(), Box<dyn Error>>;

fn smoke_timeout() -> Duration {
    if std::env::var_os("LLVM_PROFILE_FILE").is_some()
        || std::env::var_os("CARGO_LLVM_COV").is_some()
    {
        Duration::from_secs(60)
    } else {
        Duration::from_secs(10)
    }
}

fn wait_for_event(
    rx: &Receiver<DapMessage>,
    event_name: &str,
    timeout: Duration,
) -> Result<DapMessage, String> {
    let deadline = Instant::now() + timeout;
    loop {
        let now = Instant::now();
        if now >= deadline {
            return Err(format!("timeout waiting for event `{event_name}`"));
        }

        let remaining = deadline.saturating_duration_since(now);
        match rx.recv_timeout(remaining) {
            Ok(message) => {
                if let DapMessage::Event { event, .. } = &message
                    && event == event_name
                {
                    return Ok(message);
                }
            }
            Err(_) => return Err(format!("channel timeout waiting for `{event_name}`")),
        }
    }
}

fn response_success(response: DapMessage, command: &str) -> Result<Option<Value>, String> {
    match response {
        DapMessage::Response {
            success,
            command: actual,
            body,
            message,
            ..
        } => {
            if actual != command {
                return Err(format!("expected `{command}` response, got `{actual}`"));
            }
            if !success {
                return Err(format!(
                    "command `{command}` failed: {}",
                    message.unwrap_or_else(|| "<no message>".to_string())
                ));
            }
            Ok(body)
        }
        _ => Err(format!("expected response message for `{command}`")),
    }
}

fn event_body(message: &DapMessage) -> Option<&Value> {
    match message {
        DapMessage::Event { body, .. } => body.as_ref(),
        _ => None,
    }
}

#[test]
fn dap_attach_e2e_tcp_loopback() -> TestResult {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();

    let server_handle = thread::spawn(move || {
        let result = (|| -> Result<(), Box<dyn Error + Send + Sync>> {
            let (mut socket, _) = listener.accept()?;

            let stopped_event = json!({
                "type": "event",
                "seq": 1,
                "event": "stopped",
                "body": {
                    "reason": "breakpoint",
                    "threadId": 7,
                    "allThreadsStopped": true
                }
            })
            .to_string();
            socket.write_all(&frame(stopped_event.as_bytes()))?;
            socket.flush()?;

            let mut buf = [0u8; 512];
            loop {
                match socket.read(&mut buf) {
                    Ok(0) => break,
                    Ok(_) => continue,
                    Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(err) => return Err(Box::new(err)),
                }
            }

            Ok(())
        })();

        if let Err(err) = result {
            panic!("fake TCP debugger server failed: {err}");
        }
    });

    let timeout = smoke_timeout();
    let mut adapter = DebugAdapter::new();
    let (tx, rx) = channel();
    adapter.set_event_sender(tx);

    let init_body = response_success(adapter.handle_request(1, "initialize", None), "initialize")?;
    let capabilities = init_body.ok_or("initialize response missing capability body")?;
    assert!(
        capabilities
            .get("supportsRestartRequest")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    );
    let _initialized = wait_for_event(&rx, "initialized", timeout)?;

    response_success(
        adapter.handle_request(
            2,
            "attach",
            Some(json!({
                "host": "127.0.0.1",
                "port": port,
                "timeout": 2000
            })),
        ),
        "attach",
    )?;

    let threads_body = response_success(adapter.handle_request(3, "threads", None), "threads")?
        .ok_or("threads response missing body")?;
    let threads = threads_body
        .get("threads")
        .and_then(Value::as_array)
        .ok_or("threads response missing thread list")?;
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0]["id"], 1);
    assert_eq!(threads[0]["name"], "TCP Attached Thread");

    let stopped = wait_for_event(&rx, "stopped", timeout)?;
    let stopped_body = event_body(&stopped).ok_or("stopped event missing body")?;
    assert_eq!(
        stopped_body.get("reason").and_then(Value::as_str),
        Some("breakpoint")
    );
    assert_eq!(
        stopped_body.get("threadId").and_then(Value::as_i64),
        Some(7)
    );

    response_success(
        adapter.handle_request(4, "disconnect", Some(json!({}))),
        "disconnect",
    )?;
    let _terminated = wait_for_event(&rx, "terminated", timeout)?;

    server_handle
        .join()
        .map_err(|_| std::io::Error::other("fake TCP debugger server panicked"))?;
    Ok(())
}
