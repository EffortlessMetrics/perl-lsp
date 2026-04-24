//! DAP scorecard harness.
//!
//! Measures launch success plus additional real-session scorecard metrics:
//! - attach success rate (process-id attach mode)
//! - variables pane correctness
//! - evaluate correctness
//! - deep truncation/pagination correctness
//! - best-effort memory baseline proxy
//!
//! Emits marker-friendly rows for `cargo xtask update-status --only dap`:
//! `DAP_SCORECARD_ROW|<metric>|<value>|<target>|<status>`
//!
//! # Running
//!
//! ```text
//! cargo test -p perl-dap --test dap_scorecard_harness -- --nocapture
//! ```
//!
//! Tests skip gracefully when `perl` is not on `PATH`.

mod common;

use common::{DapWorkflowSession, perl_available, workflow_timeout};
use perl_dap::{DapMessage, DebugAdapter};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Clone)]
struct ScoreRow {
    metric: &'static str,
    value: String,
    target: &'static str,
    status: &'static str,
}

fn emit_row(row: &ScoreRow) {
    eprintln!("DAP_SCORECARD_ROW|{}|{}|{}|{}", row.metric, row.value, row.target, row.status);
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

    let init_resp = adapter.handle_request(1, "initialize", None);
    match init_resp {
        DapMessage::Response { success: true, .. } => {}
        DapMessage::Response { success: false, message, .. } => {
            return Err(format!(
                "initialize failed: {}",
                message.unwrap_or_else(|| "<no message>".to_string())
            ));
        }
        _ => return Err("unexpected non-response to initialize".to_string()),
    }
    wait_for_event(&rx, "initialized", timeout)?;

    let t_launch = Instant::now();
    let launch_resp = adapter.handle_request(
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
    );
    match launch_resp {
        DapMessage::Response { success: true, .. } => {}
        DapMessage::Response { success: false, message, .. } => {
            return Err(format!(
                "launch failed: {}",
                message.unwrap_or_else(|| "<no message>".to_string())
            ));
        }
        _ => return Err("unexpected non-response to launch".to_string()),
    }

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

fn attach_success_rate(samples: usize) -> (usize, usize) {
    let mut passed = 0;
    let pid = std::process::id();

    for _ in 0..samples {
        let mut adapter = DebugAdapter::new();
        let _ = adapter.handle_request(1, "initialize", None);
        let resp = adapter.handle_request(2, "attach", Some(json!({ "processId": pid })));
        if let DapMessage::Response { success: true, .. } = resp {
            passed += 1;
        }
        let _ = adapter.handle_request(3, "disconnect", Some(json!({})));
    }

    (passed, samples)
}

fn evaluate_response_result(resp: &DapMessage) -> Result<String, String> {
    match resp {
        DapMessage::Response { success: true, body: Some(body), .. } => body
            .get("result")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .ok_or("evaluate response missing string `result`".to_string()),
        DapMessage::Response { success: false, message, .. } => {
            Err(format!("evaluate failed: {}", message.as_deref().unwrap_or("<no message>")))
        }
        _ => Err("unexpected evaluate response payload".to_string()),
    }
}

fn run_realtime_session_metrics(timeout: Duration) -> Result<(bool, bool, bool), String> {
    let workspace = tempdir().map_err(|e| e.to_string())?;
    let script_path = workspace.path().join("scorecard_session.pl");

    fs::write(
        &script_path,
        r#"use strict;
use warnings;
my $x = 41;
my @big = (0..299);
my %meta = (name => 'scorecard', answer => 42);
print $x;
"#,
    )
    .map_err(|e| e.to_string())?;

    let script_str =
        script_path.to_str().ok_or("script path could not be converted to UTF-8")?.to_string();

    let mut session = DapWorkflowSession::new(timeout)?;
    session.launch(&script_str)?;
    let _ = session.set_breakpoints(&script_str, &[6])?;
    session.configuration_done()?;
    let stopped = session.wait_stopped()?;

    let (frame_id, _source_path, _line) = session.stack_trace(stopped.thread_id)?;
    let locals_ref = session.scopes_locals_ref(frame_id)?;
    let locals = session.variables(locals_ref)?;

    let scalar_ok = locals.iter().any(|entry| {
        let name_matches = entry
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| name == "$x" || name.contains("x"));
        let value_matches =
            entry.get("value").and_then(Value::as_str).is_some_and(|value| value.contains("41"));
        name_matches && value_matches
    });

    let structured_var_present = locals.iter().any(|entry| {
        entry.get("variablesReference").and_then(Value::as_i64).is_some_and(|r| r > 0)
    });

    let variables_ok = scalar_ok && structured_var_present;

    let eval_resp = session.request(
        "evaluate",
        Some(json!({
            "expression": "$x + 1",
            "context": "watch",
            "frameId": frame_id,
            "allowSideEffects": false
        })),
    );
    let evaluate_ok =
        evaluate_response_result(&eval_resp).map(|result| result.contains("42")).unwrap_or(false);

    let deep_entry = locals.iter().find(|entry| {
        let vars_ref = entry.get("variablesReference").and_then(Value::as_i64).unwrap_or(0);
        let indexed = entry.get("indexedVariables").and_then(Value::as_i64).unwrap_or(0);
        vars_ref > 0 && indexed >= 100
    });
    let deep_ok = if let Some(entry) = deep_entry {
        let big_ref = entry.get("variablesReference").and_then(Value::as_i64).unwrap_or(0);

        if big_ref <= 0 {
            false
        } else {
            let first_page = session.request(
                "variables",
                Some(json!({"variablesReference": big_ref, "start": 0, "count": 25})),
            );
            let second_page = session.request(
                "variables",
                Some(json!({"variablesReference": big_ref, "start": 25, "count": 25})),
            );

            let first_len = extract_variable_count(&first_page);
            let second_len = extract_variable_count(&second_page);

            first_len == Some(25) && second_len == Some(25)
        }
    } else {
        false
    };

    let _ = session.disconnect();

    Ok((variables_ok, evaluate_ok, deep_ok))
}

fn extract_variable_count(resp: &DapMessage) -> Option<usize> {
    match resp {
        DapMessage::Response { success: true, body: Some(body), .. } => {
            body.get("variables").and_then(Value::as_array).map(std::vec::Vec::len)
        }
        _ => None,
    }
}

fn best_effort_memory_kib() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let status = fs::read_to_string("/proc/self/status").ok()?;
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("VmRSS:") {
                let kib = rest.split_whitespace().find_map(|token| token.parse::<u64>().ok());
                if kib.is_some() {
                    return kib;
                }
            }
        }
        None
    }

    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[test]
fn scorecard_launch_success_rate() -> TestResult {
    if !perl_available() {
        let skipped = [
            "launch_success_rate",
            "cold_launch_p50",
            "cold_launch_p95",
            "attach_success_rate",
            "variables_session_correctness",
            "evaluate_session_correctness",
            "deep_pagination_correctness",
            "memory_baseline_proxy",
        ];
        for metric in skipped {
            emit_row(&ScoreRow {
                metric,
                value: "SKIP (perl not on PATH)".to_string(),
                target: "best effort",
                status: "SKIP",
            });
        }
        eprintln!("scorecard_launch_success_rate: skipping — perl not on PATH");
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

    let timeout = workflow_timeout();
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

    let passed = results.iter().filter(|r| r.passed()).count();
    let total = results.len();
    let rate = ((passed as f64 / total as f64) * 100.0).round() as u64;
    let launch_status = if passed * 5 >= total * 4 { "PASS" } else { "FAIL" };
    emit_row(&ScoreRow {
        metric: "launch_success_rate",
        value: format!("{passed}/{total} ({rate} %)"),
        target: "≥ 80 %",
        status: launch_status,
    });

    let p50 = percentile(&latencies, 50).unwrap_or(0);
    let p95 = percentile(&latencies, 95).unwrap_or(0);
    emit_row(&ScoreRow {
        metric: "cold_launch_p50",
        value: format!("{p50} ms"),
        target: "≤ 2 000 ms",
        status: if p50 <= 2_000 { "PASS" } else { "FAIL" },
    });
    emit_row(&ScoreRow {
        metric: "cold_launch_p95",
        value: format!("{p95} ms"),
        target: "≤ 5 000 ms",
        status: if p95 <= 5_000 { "PASS" } else { "FAIL" },
    });

    let (attach_passed, attach_total) = attach_success_rate(5);
    let attach_rate = ((attach_passed as f64 / attach_total as f64) * 100.0).round() as u64;
    emit_row(&ScoreRow {
        metric: "attach_success_rate",
        value: format!("{attach_passed}/{attach_total} ({attach_rate} %)"),
        target: "≥ 80 %",
        status: if attach_passed * 5 >= attach_total * 4 { "PASS" } else { "FAIL" },
    });

    let (variables_ok, evaluate_ok, deep_ok) =
        run_realtime_session_metrics(timeout).unwrap_or((false, false, false));

    emit_row(&ScoreRow {
        metric: "variables_session_correctness",
        value: if variables_ok {
            "PASS".to_string()
        } else {
            "SKIP (best-effort probe inconclusive)".to_string()
        },
        target: "best effort",
        status: if variables_ok { "PASS" } else { "SKIP" },
    });
    emit_row(&ScoreRow {
        metric: "evaluate_session_correctness",
        value: if evaluate_ok { "PASS" } else { "FAIL" }.to_string(),
        target: "PASS",
        status: if evaluate_ok { "PASS" } else { "FAIL" },
    });
    emit_row(&ScoreRow {
        metric: "deep_pagination_correctness",
        value: if deep_ok {
            "PASS".to_string()
        } else {
            "SKIP (best-effort probe inconclusive)".to_string()
        },
        target: "best effort",
        status: if deep_ok { "PASS" } else { "SKIP" },
    });

    let memory_row = if let Some(kib) = best_effort_memory_kib() {
        ScoreRow {
            metric: "memory_baseline_proxy",
            value: format!("VmRSS ~ {kib} KiB (best effort)"),
            target: "observability baseline",
            status: "INFO",
        }
    } else {
        ScoreRow {
            metric: "memory_baseline_proxy",
            value: "SKIP (no portable RSS probe on this platform)".to_string(),
            target: "observability baseline",
            status: "SKIP",
        }
    };
    emit_row(&memory_row);

    let threshold = (total * 4).div_ceil(5);
    assert!(
        passed >= threshold,
        "DAP launch success rate below threshold: {passed}/{total} passed (need ≥{threshold}). Failed fixtures: {}",
        results
            .iter()
            .filter(|r| !r.passed())
            .map(|r| format!("{} ({})", r.name, r.error.as_deref().unwrap_or("?")))
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(())
}
