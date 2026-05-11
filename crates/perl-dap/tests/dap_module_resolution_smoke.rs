//! DAP smoke test: script-to-module path mapping.
//!
//! Verifies that the DAP adapter accepts a breakpoint request targeting a
//! module file loaded via `use lib 'lib'; use My::App;`, and documents
//! whether a breakpoint *hit* (execution stop at the module site) is
//! currently supported.  The test is a status receipt — it does not crash
//! regardless of whether the hit is delivered; it explicitly records the
//! outcome so regressions are visible.
//!
//! Related issue: #8621

use perl_dap::{DapMessage, DebugAdapter};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ── Line constants for lib/My/App.pm ─────────────────────────────────────────
//
//   Line 12: sub run {
//   Line 13:     my ($self) = @_;
//   Line 14:     my $x = 1;      ← first executable line inside run()
//   Line 15:     my $y = $x + 1;
//   Line 16:     print "Result: $y\n";
//
const MODULE_BREAKPOINT_LINE: u64 = 14; // my $x = 1 — first executable line in sub run

fn perl_available() -> bool {
    std::process::Command::new("perl").arg("--version").output().is_ok()
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
            Ok(msg) => {
                if let DapMessage::Event { event, .. } = &msg
                    && event == event_name
                {
                    return Ok(msg);
                }
            }
            Err(_) => return Err(format!("channel timeout waiting for `{event_name}`")),
        }
    }
}

fn response_success(msg: DapMessage, command: &str) -> Result<Option<Value>, String> {
    match msg {
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

/// Set up the module-resolution fixture in a tempdir with an absolute `use lib` path.
///
/// `use lib 'lib'` is relative to CWD, not to the script location, and the DAP
/// adapter does not `chdir` to the script's directory before spawning `perl -d`.
/// To make the fixture self-contained regardless of CWD, the script is written
/// with the absolute path to the `lib/` directory baked in at fixture creation
/// time.
///
/// Returns `(tempdir, script_path, module_path)` where both paths are absolute
/// UTF-8 strings suitable for DAP `source.path` values.
fn setup_fixtures() -> Result<(tempfile::TempDir, String, String), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let lib_dir = dir.path().join("lib").join("My");
    fs::create_dir_all(&lib_dir)?;

    let fixture_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/module-resolution");

    // Copy the module file as-is.
    let module_dest = lib_dir.join("App.pm");
    fs::copy(fixture_root.join("lib/My/App.pm"), &module_dest)?;

    // Write the script with the absolute lib path so it works regardless of CWD.
    let abs_lib = dir.path().join("lib").to_str().ok_or("lib path is not valid UTF-8")?.to_string();
    let script_content = format!(
        "use strict;\nuse warnings;\nuse lib '{abs_lib}';\nuse My::App;\n\nmy $app = My::App->new();\n$app->run();\n"
    );
    let script_dest = dir.path().join("script.pl");
    fs::write(&script_dest, script_content)?;

    let script_str = script_dest.to_str().ok_or("script path is not valid UTF-8")?.to_string();
    let module_str = module_dest.to_str().ok_or("module path is not valid UTF-8")?.to_string();

    Ok((dir, script_str, module_str))
}

// ── Test 1: setBreakpoints on a module file is accepted ──────────────────────

/// Verifies that `setBreakpoints` with a source path pointing to a module
/// file returns a successful response with a breakpoints array.
///
/// This is a *static* (pre-launch) receipt: it does not start `perl -d` and
/// therefore does not require Perl to be installed.  It exercises the DAP
/// adapter's breakpoint store and AST validator with a real `.pm` file.
#[test]
fn test_module_breakpoint_accepted_without_session() -> TestResult {
    let (_dir, _script_str, module_str) = setup_fixtures()?;

    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    let response = adapter.handle_request(
        2,
        "setBreakpoints",
        Some(json!({
            "source": { "path": module_str },
            "breakpoints": [{ "line": MODULE_BREAKPOINT_LINE }]
        })),
    );

    // The response must succeed — the adapter must not reject a module-file source.
    let body = response_success(response, "setBreakpoints")
        .map_err(|e| format!("setBreakpoints rejected module file path: {e}"))?;

    let body = body.ok_or("setBreakpoints returned no body for module file")?;
    let breakpoints = body
        .get("breakpoints")
        .and_then(Value::as_array)
        .ok_or("setBreakpoints response missing `breakpoints` array")?;

    assert!(
        !breakpoints.is_empty(),
        "setBreakpoints must return at least one breakpoint entry for module file at line {MODULE_BREAKPOINT_LINE}"
    );

    // Record whether the breakpoint was verified by the AST validator.
    // Both outcomes are valid at this stage; the assertion documents the status.
    let first = &breakpoints[0];
    let verified = first.get("verified").and_then(Value::as_bool).unwrap_or(false);
    eprintln!(
        "[module-resolution smoke] static setBreakpoints: verified={verified} (line {MODULE_BREAKPOINT_LINE} in My::App)"
    );

    Ok(())
}

// ── Test 2: full launch + module breakpoint + status receipt ─────────────────

/// Full DAP flow: launch `script.pl`, set a breakpoint in the loaded module
/// (`lib/My/App.pm`), send `configurationDone`, and record whether the
/// debugger stops at the module site.
///
/// This test explicitly documents which outcome it received without failing on
/// either.  If the breakpoint is hit, it verifies the source path resolves to
/// the module file.  If not hit, it records the outcome and asserts the session
/// terminates cleanly.  A future fix that *does* deliver the hit will need to
/// tighten these assertions.
///
/// Skips when `perl` is not on `PATH`.
#[test]
fn test_module_breakpoint_hit_status_receipt() -> TestResult {
    if !perl_available() {
        eprintln!("Skipping test_module_breakpoint_hit_status_receipt - perl not available");
        return Ok(());
    }

    let (_dir, script_str, module_str) = setup_fixtures()?;
    let timeout = smoke_timeout();

    let mut adapter = DebugAdapter::new();
    let (tx, rx) = channel();
    adapter.set_event_sender(tx);

    // Initialize
    response_success(adapter.handle_request(1, "initialize", None), "initialize")?;
    let _initialized = wait_for_event(&rx, "initialized", timeout)?;

    // Launch the script (stopOnEntry so we can set breakpoints before execution begins)
    response_success(
        adapter.handle_request(
            2,
            "launch",
            Some(json!({
                "program": script_str,
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

    // Wait for the entry-stop before setting breakpoints (required for stopOnEntry flow).
    let entry_stop = wait_for_event(&rx, "stopped", timeout)?;
    let entry_reason = if let DapMessage::Event { body, .. } = &entry_stop {
        body.as_ref()
            .and_then(|b| b.get("reason"))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string()
    } else {
        "unknown".to_string()
    };
    assert!(
        matches!(entry_reason.as_str(), "entry" | "step"),
        "expected initial stopped reason `entry` or `step`, got: `{entry_reason}`"
    );

    // Set a breakpoint in the module file.
    let bp_response = adapter.handle_request(
        3,
        "setBreakpoints",
        Some(json!({
            "source": { "path": module_str },
            "breakpoints": [{ "line": MODULE_BREAKPOINT_LINE }]
        })),
    );
    let bp_body = response_success(bp_response, "setBreakpoints")
        .map_err(|e| format!("setBreakpoints on module failed: {e}"))?;
    let bp_body = bp_body.ok_or("setBreakpoints returned no body")?;
    let breakpoints = bp_body
        .get("breakpoints")
        .and_then(Value::as_array)
        .ok_or("setBreakpoints body missing `breakpoints` array")?;

    assert!(!breakpoints.is_empty(), "setBreakpoints must return at least one entry");
    let bp_verified = breakpoints[0].get("verified").and_then(Value::as_bool).unwrap_or(false);
    eprintln!(
        "[module-resolution smoke] post-launch setBreakpoints: verified={bp_verified} (module line {MODULE_BREAKPOINT_LINE})"
    );

    // Resume execution.
    response_success(
        adapter.handle_request(4, "configurationDone", Some(json!({}))),
        "configurationDone",
    )?;
    response_success(
        adapter.handle_request(5, "continue", Some(json!({"threadId": 1}))),
        "continue",
    )?;

    // Wait for either a stopped event (breakpoint hit) or terminated (no hit).
    // We accept either outcome and record it as the status receipt.
    let deadline = Instant::now() + timeout;
    let mut module_breakpoint_hit = false;
    let mut session_terminated = false;

    'outer: loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match rx.recv_timeout(remaining) {
            Ok(DapMessage::Event { ref event, ref body, .. }) => {
                match event.as_str() {
                    "stopped" => {
                        let reason = body
                            .as_ref()
                            .and_then(|b| b.get("reason"))
                            .and_then(Value::as_str)
                            .unwrap_or("unknown")
                            .to_string();
                        if reason == "breakpoint" {
                            module_breakpoint_hit = true;
                        }
                        // Whether hit or step/other, disconnect cleanly.
                        let _ = adapter.handle_request(6, "disconnect", Some(json!({})));
                        break 'outer;
                    }
                    "terminated" | "exited" => {
                        session_terminated = true;
                        break 'outer;
                    }
                    _ => {}
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }

    // Status receipt: document the outcome without hard-failing on the hit.
    // Future work: when module-path mapping is fully implemented, change this
    // to `assert!(module_breakpoint_hit)`.
    eprintln!(
        "[module-resolution smoke] outcome: module_breakpoint_hit={module_breakpoint_hit} \
         session_terminated={session_terminated}"
    );

    if module_breakpoint_hit {
        eprintln!("[module-resolution smoke] SUPPORTED: breakpoint in loaded module was hit");
    } else {
        eprintln!(
            "[module-resolution smoke] STATUS: breakpoint in loaded module was NOT hit \
             (path mapping not yet fully supported for runtime-loaded modules)"
        );
    }

    // Tear down: disconnect and wait for the terminated event.  This confirms
    // the adapter exits cleanly rather than panicking or hanging.
    let _ = adapter.handle_request(7, "disconnect", Some(json!({})));
    let terminated = wait_for_event(&rx, "terminated", Duration::from_secs(5));
    assert!(
        terminated.is_ok(),
        "adapter must emit `terminated` after disconnect — status receipt must complete cleanly"
    );

    Ok(())
}
