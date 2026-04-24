//! DAP scorecard harness.
//!
//! Measures real-session debugger scorecard metrics and prints marker-friendly
//! Markdown blocks that can be copied into `docs/project/status/dap.md`.
//!
//! # Running
//!
//! ```text
//! cargo test -p perl-dap --test dap_scorecard_harness -- --nocapture
//! ```
//!
//! Tests skip gracefully when `perl` is not on `PATH`.

mod common;

use common::perl_available;
use perl_dap::{DapMessage, DebugAdapter};
use perl_lsp_rs_core::transport::framing::frame;
use serde_json::{Value, json};
use std::fs::write;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

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
            Err(_) => return Err(format!("channel closed/timeout waiting for `{event_name}`")),
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

#[derive(Clone)]
struct FixtureResult {
    name: &'static str,
    elapsed_ms: Option<u128>,
    error: Option<String>,
}

impl FixtureResult {
    fn passed(&self) -> bool {
        self.error.is_none()
    }
}

fn probe_launch(script_path: &Path, timeout: Duration) -> Result<u128, String> {
    let script_str =
        script_path.to_str().ok_or("fixture path contains non-UTF-8 characters")?.to_string();

    let mut adapter = DebugAdapter::new();
    let (tx, rx) = channel();
    adapter.set_event_sender(tx);

    response_success(adapter.handle_request(1, "initialize", None), "initialize")?;
    wait_for_event(&rx, "initialized", timeout)?;

    let t_launch = Instant::now();
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

    wait_for_event(&rx, "stopped", timeout)?;
    let elapsed_ms = t_launch.elapsed().as_millis();

    let _ = adapter.handle_request(3, "disconnect", Some(json!({})));

    Ok(elapsed_ms)
}

fn percentile(sorted: &[u128], pct: u8) -> Option<u128> {
    if sorted.is_empty() {
        return None;
    }
    let rank = ((pct as f64 / 100.0) * sorted.len() as f64).ceil() as usize;
    let idx = rank.saturating_sub(1).min(sorted.len() - 1);
    Some(sorted[idx])
}

fn measure_attach_success(timeout: Duration, attempts: usize) -> Result<(usize, usize), String> {
    let mut passed = 0usize;

    for _ in 0..attempts {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|e| format!("failed to bind loopback listener: {e}"))?;
        let port =
            listener.local_addr().map_err(|e| format!("failed to read local addr: {e}"))?.port();

        let server_handle = thread::spawn(move || {
            let result = (|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
                return Err(format!("loopback attach server failed: {err}"));
            }
            Ok(())
        });

        let mut adapter = DebugAdapter::new();
        let (tx, rx) = channel();
        adapter.set_event_sender(tx);

        let attempt_ok = (|| -> Result<(), String> {
            response_success(adapter.handle_request(1, "initialize", None), "initialize")?;
            wait_for_event(&rx, "initialized", timeout)?;

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

            wait_for_event(&rx, "stopped", timeout)?;
            response_success(
                adapter.handle_request(3, "disconnect", Some(json!({}))),
                "disconnect",
            )?;
            let _ = wait_for_event(&rx, "terminated", timeout);
            Ok(())
        })()
        .is_ok();

        let server_ok = matches!(server_handle.join(), Ok(Ok(())));
        if attempt_ok && server_ok {
            passed += 1;
        }
    }

    Ok((passed, attempts))
}

fn run_realtime_session_metrics(timeout: Duration) -> Result<(bool, bool, bool), String> {
    let workspace = tempdir().map_err(|e| format!("failed to create tempdir: {e}"))?;
    let script_path = workspace.path().join("scorecard_runtime.pl");
    write(
        &script_path,
        "use strict;\nuse warnings;\n\nour $SCALAR = 42;\nour @BIG = (0..299);\nmy %hash = (foo => 1, bar => 2);\nmy $sum = $SCALAR + $BIG[10];\nprint \"$sum\\n\";\n",
    )
    .map_err(|e| format!("failed to write runtime script: {e}"))?;

    let script_str = script_path.to_str().ok_or("script path not valid UTF-8")?.to_string();

    let mut adapter = DebugAdapter::new();
    let (tx, rx) = channel();
    adapter.set_event_sender(tx);

    response_success(adapter.handle_request(1, "initialize", None), "initialize")?;
    wait_for_event(&rx, "initialized", timeout)?;

    response_success(
        adapter.handle_request(
            2,
            "launch",
            Some(json!({
                "program": script_str,
                "args": [],
                "stopOnEntry": false,
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

    response_success(
        adapter.handle_request(
            3,
            "setBreakpoints",
            Some(json!({
                "source": { "path": script_path },
                "breakpoints": [{ "line": 7 }]
            })),
        ),
        "setBreakpoints",
    )?;

    response_success(adapter.handle_request(4, "configurationDone", None), "configurationDone")?;

    let stopped = wait_for_event(&rx, "stopped", timeout)?;
    let thread_id = match stopped {
        DapMessage::Event { body, .. } => {
            body.and_then(|b| b.get("threadId").and_then(Value::as_i64)).unwrap_or(1)
        }
        _ => 1,
    };

    let stack = response_success(
        adapter.handle_request(
            5,
            "stackTrace",
            Some(json!({"threadId": thread_id, "startFrame": 0, "levels": 1})),
        ),
        "stackTrace",
    )?
    .ok_or("stackTrace response missing body")?;

    let frame_id = stack
        .get("stackFrames")
        .and_then(Value::as_array)
        .and_then(|frames| frames.first())
        .and_then(|frame| frame.get("id"))
        .and_then(Value::as_i64)
        .ok_or("stackTrace missing frame id")?;

    let scopes = response_success(
        adapter.handle_request(6, "scopes", Some(json!({"frameId": frame_id}))),
        "scopes",
    )?
    .ok_or("scopes response missing body")?;

    let scope_array = scopes
        .get("scopes")
        .and_then(Value::as_array)
        .ok_or("scopes response missing scopes array")?;

    let mut refs = Vec::new();
    for scope in scope_array {
        if let Some(reference) = scope.get("variablesReference").and_then(Value::as_i64)
            && reference > 0
        {
            refs.push(reference);
        }
    }
    if refs.is_empty() {
        return Err("no variables references available in scopes response".to_string());
    }

    let mut local_vars: Vec<Value> = Vec::new();
    let mut seq = 7;
    for reference in refs {
        let vars = response_success(
            adapter.handle_request(
                seq,
                "variables",
                Some(json!({"variablesReference": reference})),
            ),
            "variables",
        )?
        .ok_or("variables response missing body")?;
        seq += 1;

        if let Some(arr) = vars.get("variables").and_then(Value::as_array) {
            local_vars.extend(arr.iter().cloned());
        }
    }

    let scalar_ok = local_vars.iter().any(|var| {
        let name = var
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_ascii_uppercase)
            .unwrap_or_default();
        let value = var.get("value").and_then(Value::as_str).unwrap_or("");
        name.contains("SCALAR") && value.contains("42")
    });

    if !scalar_ok {
        let sample_names: Vec<String> = local_vars
            .iter()
            .filter_map(|var| var.get("name").and_then(Value::as_str))
            .take(12)
            .map(ToString::to_string)
            .collect();
        eprintln!("variables sample (first 12): {:?}", sample_names);
    }

    let big_var = local_vars.iter().find(|var| {
        var.get("indexedVariables").and_then(Value::as_i64).unwrap_or(0) >= 300
            && var.get("variablesReference").and_then(Value::as_i64).unwrap_or(0) > 0
    });

    let pagination_ok = if let Some(big) = big_var {
        let big_ref = big.get("variablesReference").and_then(Value::as_i64).unwrap_or(0);
        let big_size = big.get("indexedVariables").and_then(Value::as_i64).unwrap_or(0);
        if big_ref <= 0 || big_size < 300 {
            false
        } else {
            let page_a = response_success(
                adapter.handle_request(
                    seq,
                    "variables",
                    Some(json!({"variablesReference": big_ref, "start": 0, "count": 50})),
                ),
                "variables",
            )
            .ok()
            .flatten();
            seq += 1;
            let page_b = response_success(
                adapter.handle_request(
                    seq,
                    "variables",
                    Some(json!({"variablesReference": big_ref, "start": 250, "count": 100})),
                ),
                "variables",
            )
            .ok()
            .flatten();

            let len_a = page_a
                .as_ref()
                .and_then(|body| body.get("variables"))
                .and_then(Value::as_array)
                .map(|vars| vars.len())
                .unwrap_or(0);
            let len_b = page_b
                .as_ref()
                .and_then(|body| body.get("variables"))
                .and_then(Value::as_array)
                .map(|vars| vars.len())
                .unwrap_or(0);

            let first_a = page_a
                .as_ref()
                .and_then(|body| body.get("variables"))
                .and_then(Value::as_array)
                .and_then(|vars| vars.first())
                .and_then(|var| var.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let first_b = page_b
                .as_ref()
                .and_then(|body| body.get("variables"))
                .and_then(Value::as_array)
                .and_then(|vars| vars.first())
                .and_then(|var| var.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("");

            len_a == 50 && len_b == 50 && first_a.contains("0") && first_b.contains("250")
        }
    } else {
        false
    };

    let evaluate = response_success(
        adapter.handle_request(
            seq,
            "evaluate",
            Some(json!({
                "expression": "$main::SCALAR + 8",
                "frameId": frame_id,
                "context": "watch"
            })),
        ),
        "evaluate",
    )?
    .ok_or("evaluate response missing body")?;
    seq += 1;

    let evaluate_ok = evaluate
        .get("result")
        .and_then(Value::as_str)
        .map(|result| result.contains("50"))
        .unwrap_or(false);

    let _ = adapter.handle_request(seq, "disconnect", Some(json!({})));

    Ok((scalar_ok, evaluate_ok, pagination_ok))
}

fn best_effort_rss_kib() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
        let kib = line.split_whitespace().nth(1).and_then(|value| value.parse::<u64>().ok())?;
        return Some(kib);
    }

    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[test]
fn scorecard_launch_success_rate() -> TestResult {
    if !perl_available() {
        eprintln!("scorecard_launch_success_rate: skipping — perl not on PATH");
        eprintln!("<!-- BEGIN: DAP_RUNTIME_SCORECARD -->");
        eprintln!("| Metric | Value | Target | Status |");
        eprintln!("|---|---|---|---|");
        eprintln!("| Attach success rate | SKIP (perl unavailable) | best-effort >= 80 % | SKIP |");
        eprintln!(
            "| Variables pane correctness (real session) | SKIP (perl unavailable) | PASS required | SKIP |"
        );
        eprintln!(
            "| Evaluate correctness (real session) | SKIP (perl unavailable) | PASS required | SKIP |"
        );
        eprintln!(
            "| Deep truncation/pagination correctness | SKIP (perl unavailable) | PASS required | SKIP |"
        );
        eprintln!(
            "| Memory footprint baseline (best-effort) | SKIP (perl unavailable) | baseline-only | SKIP |"
        );
        eprintln!("<!-- END: DAP_RUNTIME_SCORECARD -->");
        return Ok(());
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let fixture_dir = Path::new(&manifest_dir).join("tests").join("fixtures");

    let fixtures: &[(&str, &str)] = &[
        ("hello", "hello.pl"),
        ("loops", "loops.pl"),
        ("eval", "eval.pl"),
        ("args", "args.pl"),
        ("begin_end", "breakpoints_begin_end.pl"),
    ];

    let timeout = smoke_timeout();
    let mut results: Vec<FixtureResult> = Vec::with_capacity(fixtures.len());

    for (name, filename) in fixtures {
        let path = fixture_dir.join(filename);
        let (elapsed_ms, error) = match probe_launch(&path, timeout) {
            Ok(ms) => (Some(ms), None),
            Err(e) => (None, Some(e)),
        };
        results.push(FixtureResult { name, elapsed_ms, error });
    }

    let mut latencies: Vec<u128> = results.iter().filter_map(|r| r.elapsed_ms).collect();
    latencies.sort_unstable();

    let (attach_passed, attach_total) = measure_attach_success(timeout, 3)?;
    let attach_status = if attach_passed * 5 >= attach_total * 4 { "PASS" } else { "FAIL" };

    let rss_before = best_effort_rss_kib();
    let (variables_ok, evaluate_ok, pagination_ok) = run_realtime_session_metrics(timeout)?;
    let rss_after = best_effort_rss_kib();

    eprintln!();
    eprintln!("┌─────────────────── DAP Launch Scorecard ────────────────────────────────┐");
    eprintln!("│ Fixture              │ Result    │ Latency (ms) │ Detail                 │");
    eprintln!("├──────────────────────┼───────────┼──────────────┼────────────────────────┤");
    for r in &results {
        let status = if r.passed() { "PASS" } else { "FAIL" };
        let latency = r.elapsed_ms.map(|ms| ms.to_string()).unwrap_or_else(|| "—".to_string());
        let detail = r.error.as_deref().unwrap_or("");
        let detail_trunc = if detail.len() > 22 { &detail[..22] } else { detail };
        eprintln!("│ {:<20} │ {:<9} │ {:>12} │ {:<22} │", r.name, status, latency, detail_trunc);
    }
    eprintln!("└─────────────────────────────────────────────────────────────────────────┘");

    if let (Some(p50), Some(p95)) = (percentile(&latencies, 50), percentile(&latencies, 95)) {
        eprintln!();
        eprintln!(
            "  cold_launch_p50 = {}ms   cold_launch_p95 = {}ms   (n={})",
            p50,
            p95,
            latencies.len()
        );
    }

    let memory_value = match (rss_before, rss_after) {
        (Some(before), Some(after)) => format!(
            "{} KiB -> {} KiB (delta {:+} KiB)",
            before,
            after,
            after as i64 - before as i64
        ),
        _ => "SKIP (unsupported platform for /proc/self/status VmRSS)".to_string(),
    };
    let memory_status =
        if rss_before.is_some() && rss_after.is_some() { "BASELINE" } else { "SKIP" };

    eprintln!();
    eprintln!("<!-- BEGIN: DAP_RUNTIME_SCORECARD -->");
    eprintln!("| Metric | Value | Target | Status |");
    eprintln!("|---|---|---|---|");
    eprintln!(
        "| Attach success rate | {}/{} ({:.0} %) | ≥ 80 % | {} |",
        attach_passed,
        attach_total,
        (attach_passed as f64 * 100.0) / attach_total as f64,
        attach_status
    );
    eprintln!(
        "| Variables pane correctness (real session) | {} | PASS required | {} |",
        if variables_ok { "PASS" } else { "FAIL" },
        if variables_ok { "PASS" } else { "FAIL" }
    );
    eprintln!(
        "| Evaluate correctness (real session) | {} | PASS required | {} |",
        if evaluate_ok { "PASS" } else { "FAIL" },
        if evaluate_ok { "PASS" } else { "FAIL" }
    );
    eprintln!(
        "| Deep truncation/pagination correctness | {} | PASS required | {} |",
        if pagination_ok { "PASS" } else { "FAIL" },
        if pagination_ok { "PASS" } else { "FAIL" }
    );
    eprintln!(
        "| Memory footprint baseline (best-effort) | {} | baseline-only | {} |",
        memory_value, memory_status
    );
    eprintln!("<!-- END: DAP_RUNTIME_SCORECARD -->");
    eprintln!();

    let passed = results.iter().filter(|r| r.passed()).count();
    let total = results.len();
    let threshold = (total * 4).div_ceil(5);

    assert!(
        passed >= threshold,
        "DAP launch success rate below threshold: {passed}/{total} passed (need ≥{threshold}). \
         Failed fixtures: {}",
        results
            .iter()
            .filter(|r| !r.passed())
            .map(|r| format!("{} ({})", r.name, r.error.as_deref().unwrap_or("?")))
            .collect::<Vec<_>>()
            .join(", ")
    );

    assert!(
        attach_status == "PASS",
        "attach success rate below threshold: {attach_passed}/{attach_total}"
    );
    // Runtime correctness metrics are emitted as scorecard values (PASS/FAIL)
    // and intentionally do not fail this harness yet; launch and attach remain
    // hard gates while correctness trends are promoted into visible status.
    let _ = (variables_ok, evaluate_ok, pagination_ok);

    Ok(())
}
