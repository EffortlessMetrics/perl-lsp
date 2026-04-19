//! End-to-end DAP smoke test using the native debug adapter and real `perl -d`.

use perl_dap::{DapMessage, DebugAdapter};
use serde_json::{Value, json};
use std::fs::write;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn perl_available() -> bool {
    std::process::Command::new("perl")
        .arg("--version")
        .output()
        .is_ok()
}

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

fn stopped_reason(message: &DapMessage) -> Option<String> {
    match message {
        DapMessage::Event { event, body, .. } if event == "stopped" => body
            .as_ref()
            .and_then(|payload| payload.get("reason"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
        _ => None,
    }
}

#[test]
fn dap_smoke_e2e() -> TestResult {
    if !perl_available() {
        eprintln!("Skipping dap_smoke_e2e - perl executable is not available");
        return Ok(());
    }

    let workspace = tempdir()?;
    let script_path = workspace.path().join("smoke.pl");
    write(
        &script_path,
        r#"use strict;
use warnings;
my $x = 1;
$x++;
print "$x\n";
"#,
    )?;

    let script_path_str = script_path
        .to_str()
        .ok_or("script path could not be converted to UTF-8 string")?
        .to_string();
    let timeout = smoke_timeout();

    let mut adapter = DebugAdapter::new();
    let (tx, rx) = channel();
    adapter.set_event_sender(tx);

    let init_body = response_success(adapter.handle_request(1, "initialize", None), "initialize")?;
    let capabilities = init_body.ok_or("initialize response missing capability body")?;
    assert!(
        capabilities
            .get("supportsConfigurationDoneRequest")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    );
    assert!(
        capabilities
            .get("supportsInlineValues")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    );
    let _initialized = wait_for_event(&rx, "initialized", timeout)?;

    response_success(
        adapter.handle_request(
            2,
            "launch",
            Some(json!({
                "program": script_path_str,
                "args": [],
                "stopOnEntry": true,
                "env": {
                    "PERL_PERTURB_KEYS": "0",
                    "PERL_HASH_SEED": "0",
                    "LC_ALL": "C",
                    "TZ": "UTC"
                }
            })),
        ),
        "launch",
    )?;
    let entry_stop = wait_for_event(&rx, "stopped", timeout)?;
    let entry_reason = stopped_reason(&entry_stop);
    assert!(
        matches!(entry_reason.as_deref(), Some("entry" | "step")),
        "expected initial stopped reason `entry` or `step`, got: {entry_stop:#?}"
    );

    let request_seq = 3;

    response_success(
        adapter.handle_request(request_seq, "disconnect", Some(json!({}))),
        "disconnect",
    )?;
    let _terminated = wait_for_event(&rx, "terminated", timeout)?;

    Ok(())
}
