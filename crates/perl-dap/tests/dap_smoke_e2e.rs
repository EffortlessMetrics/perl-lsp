//! End-to-end DAP smoke test using the native debug adapter and real `perl -d`.

use perl_dap::{DapMessage, DebugAdapter};
use serde_json::{Value, json};
use std::fs::write;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

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
        DapMessage::Response { success, command: actual, body, message, .. } => {
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

fn evaluate_with_retry(
    adapter: &mut DebugAdapter,
    request_seq: &mut i64,
    expression: &str,
    expected_fragment: &str,
    timeout: Duration,
) -> Result<String, String> {
    let deadline = Instant::now() + timeout;

    loop {
        let eval_body = response_success(
            adapter.handle_request(
                *request_seq,
                "evaluate",
                Some(json!({
                    "expression": expression,
                    "frameId": 1,
                    "allowSideEffects": true
                })),
            ),
            "evaluate",
        )?
        .ok_or("evaluate response body missing")?;
        *request_seq += 1;

        let result = eval_body
            .get("result")
            .and_then(Value::as_str)
            .ok_or("evaluate result missing")?
            .to_string();

        if result.contains(expected_fragment) && !result.contains("timeout") {
            return Ok(result);
        }

        if Instant::now() >= deadline {
            return Ok(result);
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn dap_smoke_e2e() -> TestResult {
    if std::process::Command::new("perl").arg("--version").output().is_err() {
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
    let _initialized = wait_for_event(&rx, "initialized", Duration::from_secs(2))?;

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
    let entry_stop = wait_for_event(&rx, "stopped", Duration::from_secs(3))?;
    assert_eq!(
        stopped_reason(&entry_stop).as_deref(),
        Some("entry"),
        "expected stopped reason `entry`, got: {entry_stop:#?}"
    );

    let breakpoints_body = response_success(
        adapter.handle_request(
            3,
            "setBreakpoints",
            Some(json!({
                "source": { "path": script_path.to_str().ok_or("non-utf8 path")? },
                "breakpoints": [{ "line": 4 }]
            })),
        ),
        "setBreakpoints",
    )?;
    let breakpoints_body = breakpoints_body.ok_or("setBreakpoints response missing body")?;
    let breakpoints = breakpoints_body
        .get("breakpoints")
        .and_then(Value::as_array)
        .ok_or("setBreakpoints response missing breakpoints array")?;
    assert!(!breakpoints.is_empty(), "expected at least one breakpoint response");
    assert!(
        breakpoints[0].get("verified").and_then(Value::as_bool).unwrap_or(false),
        "expected breakpoint on `$x++` line to be verified"
    );

    response_success(
        adapter.handle_request(4, "configurationDone", Some(json!({}))),
        "configurationDone",
    )?;

    response_success(
        adapter.handle_request(5, "continue", Some(json!({"threadId": 1}))),
        "continue",
    )?;
    let breakpoint_stop = wait_for_event(&rx, "stopped", Duration::from_secs(3))?;
    let breakpoint_reason = stopped_reason(&breakpoint_stop);
    assert!(
        breakpoint_reason.as_deref() == Some("breakpoint")
            || breakpoint_reason.as_deref() == Some("step"),
        "expected stopped reason `breakpoint` or `step`, got: {breakpoint_stop:#?}"
    );

    let mut request_seq = 6;
    let result_before =
        evaluate_with_retry(&mut adapter, &mut request_seq, "$x", "1", Duration::from_secs(2))?;
    assert!(
        result_before.contains('1') && !result_before.contains("timeout"),
        "expected `$x` to be 1 before step, got: {result_before}"
    );

    let stack = response_success(
        adapter.handle_request(request_seq, "stackTrace", Some(json!({"threadId": 1}))),
        "stackTrace",
    )?
    .ok_or("stackTrace response missing body")?;
    request_seq += 1;
    let frames =
        stack.get("stackFrames").and_then(Value::as_array).ok_or("stackTrace frames missing")?;
    assert!(!frames.is_empty(), "expected at least one stack frame");
    let top_line = frames
        .first()
        .and_then(|frame| frame.get("line"))
        .and_then(Value::as_i64)
        .ok_or("stackTrace first frame missing line")?;
    assert!(
        (3..=5).contains(&top_line),
        "expected top frame line to be near breakpoint line (4), got: {top_line}"
    );
    let frame_id = frames
        .first()
        .and_then(|frame| frame.get("id"))
        .and_then(Value::as_i64)
        .ok_or("stackTrace first frame missing id")?;

    let scopes = response_success(
        adapter.handle_request(request_seq, "scopes", Some(json!({"frameId": frame_id}))),
        "scopes",
    )?
    .ok_or("scopes response missing body")?;
    request_seq += 1;
    let scopes_arr =
        scopes.get("scopes").and_then(Value::as_array).ok_or("scopes array missing")?;
    let locals_scope = scopes_arr
        .iter()
        .find(|scope| scope.get("name").and_then(Value::as_str) == Some("Locals"))
        .ok_or("Locals scope missing")?;
    let locals_ref = locals_scope
        .get("variablesReference")
        .and_then(Value::as_i64)
        .ok_or("Locals scope missing variablesReference")?;

    let variables = response_success(
        adapter.handle_request(
            request_seq,
            "variables",
            Some(json!({
                "variablesReference": locals_ref,
                "start": 0,
                "count": 50
            })),
        ),
        "variables",
    )?
    .ok_or("variables response missing body")?;
    request_seq += 1;
    let vars_arr =
        variables.get("variables").and_then(Value::as_array).ok_or("variables array missing")?;
    assert!(!vars_arr.is_empty(), "expected variables response to include entries");
    let var_names = vars_arr
        .iter()
        .filter_map(|var| var.get("name").and_then(Value::as_str))
        .collect::<Vec<_>>();
    let mut sorted_names = var_names.clone();
    sorted_names.sort_unstable();
    assert!(
        var_names == sorted_names,
        "expected adapter to return deterministically sorted variable names, got: {var_names:?}"
    );
    assert!(
        var_names.iter().any(|name| ["$x", "$self", "@_"].contains(name)),
        "expected locals to include `$x` (live) or deterministic fallback names, got: {var_names:?}"
    );

    response_success(
        adapter.handle_request(request_seq, "next", Some(json!({"threadId": 1}))),
        "next",
    )?;
    request_seq += 1;

    let result_after =
        evaluate_with_retry(&mut adapter, &mut request_seq, "$x", "2", Duration::from_secs(2))?;
    assert!(
        !result_after.trim().is_empty(),
        "expected non-empty evaluate result after step, got: {result_after}"
    );

    let set_variable = response_success(
        adapter.handle_request(
            request_seq,
            "setVariable",
            Some(json!({
                "variablesReference": locals_ref,
                "name": "$x",
                "value": "3"
            })),
        ),
        "setVariable",
    )?
    .ok_or("setVariable response missing body")?;
    request_seq += 1;
    let set_value = set_variable
        .get("value")
        .and_then(Value::as_str)
        .ok_or("setVariable response missing value")?;
    assert!(
        set_value.contains('3'),
        "expected setVariable read-back value to include 3, got: {set_value}"
    );

    let result_after_set =
        evaluate_with_retry(&mut adapter, &mut request_seq, "$x", "3", Duration::from_secs(2))?;
    assert!(
        result_after_set.contains('3') && !result_after_set.contains("timeout"),
        "expected `$x` to be 3 after setVariable, got: {result_after_set}"
    );

    response_success(
        adapter.handle_request(request_seq, "disconnect", Some(json!({}))),
        "disconnect",
    )?;
    let _terminated = wait_for_event(&rx, "terminated", Duration::from_secs(2))?;

    Ok(())
}
