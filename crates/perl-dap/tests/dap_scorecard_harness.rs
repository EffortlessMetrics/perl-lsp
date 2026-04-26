//! DAP scorecard harness.
//!
//! Measures launch success and session correctness metrics used by
//! `docs/project/status/dap.md`.
//!
//! # Running
//!
//! ```text
//! cargo test -p perl-dap --test dap_scorecard_harness -- --nocapture
//! ```

mod common;

use common::{DapWorkflowSession, perl_available, workflow_timeout};
use perl_dap::{DapMessage, DebugAdapter};
use perl_lsp_rs_core::transport::framing::frame;
use serde::Serialize;
use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const RECEIPT_FILE: &str = "dap_scorecard_receipt.json";

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
struct RateMetric {
    passed: usize,
    total: usize,
    threshold_pct: u8,
    p50_ms: Option<u128>,
    p95_ms: Option<u128>,
    details: Vec<FixtureResult>,
}

#[derive(Debug, Clone, Serialize)]
struct BinaryMetric {
    status: &'static str,
    detail: String,
}

#[derive(Debug, Clone, Serialize)]
struct ScorecardReceipt {
    perl_available: bool,
    launch: RateMetric,
    attach: RateMetric,
    variables: BinaryMetric,
    evaluate: BinaryMetric,
    deep_pagination: BinaryMetric,
    memory: BinaryMetric,
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

fn percentile(sorted: &[u128], pct: u8) -> Option<u128> {
    if sorted.is_empty() {
        return None;
    }
    let rank = ((pct as f64 / 100.0) * sorted.len() as f64).ceil() as usize;
    let idx = rank.saturating_sub(1).min(sorted.len() - 1);
    Some(sorted[idx])
}

fn launch_timeout() -> Duration {
    if std::env::var_os("LLVM_PROFILE_FILE").is_some()
        || std::env::var_os("CARGO_LLVM_COV").is_some()
    {
        Duration::from_secs(60)
    } else {
        Duration::from_secs(10)
    }
}

fn probe_launch(script_path: &Path, timeout: Duration) -> Result<u128, String> {
    let script_str =
        script_path.to_str().ok_or("fixture path contains non-UTF-8 characters")?.to_string();

    let mut adapter = DebugAdapter::new();
    let (tx, rx) = channel();
    adapter.set_event_sender(tx);

    let init_resp = adapter.handle_request(1, "initialize", None);
    response_success(init_resp, "initialize")?;
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
    response_success(launch_resp, "launch")?;
    wait_for_event(&rx, "stopped", timeout)?;
    let elapsed_ms = t_launch.elapsed().as_millis();

    let _ = adapter.handle_request(3, "disconnect", Some(json!({})));
    Ok(elapsed_ms)
}

fn probe_attach(timeout: Duration) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();

    let server_handle = thread::spawn(move || -> Result<(), String> {
        let (mut socket, _) = listener.accept().map_err(|e| e.to_string())?;
        let stopped = json!({
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
        socket.write_all(&frame(stopped.as_bytes())).map_err(|e| e.to_string())?;
        socket.flush().map_err(|e| e.to_string())?;

        let mut buf = [0u8; 512];
        loop {
            match socket.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => continue,
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(err.to_string()),
            }
        }
        Ok(())
    });

    let mut adapter = DebugAdapter::new();
    let (tx, rx) = channel();
    adapter.set_event_sender(tx);

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
    response_success(adapter.handle_request(3, "disconnect", Some(json!({}))), "disconnect")?;
    wait_for_event(&rx, "terminated", timeout)?;

    match server_handle.join() {
        Ok(result) => result,
        Err(_) => Err("fake TCP debugger server panicked".to_string()),
    }
}

fn metric_from_result(result: Result<String, String>) -> BinaryMetric {
    match result {
        Ok(detail) => BinaryMetric { status: "PASS", detail },
        Err(detail) => BinaryMetric { status: "FAIL", detail },
    }
}

fn probe_session_metrics() -> Result<(BinaryMetric, BinaryMetric, BinaryMetric), String> {
    let workspace = tempdir().map_err(|e| e.to_string())?;
    let script_path = workspace.path().join("scorecard_session.pl");
    let script_text = r#"use strict;
use warnings;
our $x = 41;
our @big = (1..500);
our %meta = (name => "dap-scorecard");
my $marker = $x + 1;
print "marker=$marker\n";
"#;
    fs::write(&script_path, script_text).map_err(|e| e.to_string())?;

    let script_str =
        script_path.to_str().ok_or_else(|| "script path is not valid UTF-8".to_string())?;

    let mut session = DapWorkflowSession::new(workflow_timeout())?;
    session.launch(script_str)?;
    session.set_breakpoints(script_str, &[6])?;
    session.configuration_done()?;

    let stop = session.wait_stopped()?;
    let (frame_id, _, _) = session.stack_trace(stop.thread_id)?;
    let globals_ref = session.scopes_globals_ref(frame_id)?;
    let globals = session.variables(globals_ref)?;

    let vars_metric = metric_from_result((|| {
        if globals.is_empty() {
            return Err("globals scope returned no variables".to_string());
        }
        if globals
            .iter()
            .any(|var| var.get("name").and_then(Value::as_str).unwrap_or("").is_empty())
        {
            return Err("globals scope contains variables with empty names".to_string());
        }
        let n = globals.len();
        Ok(format!("globals scope returned {} named {}", n, if n == 1 { "variable" } else { "variables" }))
    })());

    let eval_metric = metric_from_result((|| {
        let eval_response = session.request(
            "evaluate",
            Some(json!({
                "expression": "$x + 1",
                "frameId": frame_id,
                "context": "watch",
                "allowSideEffects": false
            })),
        );
        let eval_body = session.expect_success(&eval_response, "evaluate")?;
        let eval_body = eval_body.ok_or_else(|| "evaluate response missing body".to_string())?;
        let result = eval_body.get("result").and_then(Value::as_str).unwrap_or("");
        if !result.contains("42") {
            return Err(format!("evaluate result does not contain 42: {result:?}"));
        }
        Ok("evaluate($x + 1) returns 42".to_string())
    })());

    let deep_metric = if let Some(expandable) = globals.iter().find(|var| {
        var.get("variablesReference").and_then(Value::as_i64).unwrap_or(0) > 0
            && var.get("indexedVariables").and_then(Value::as_i64).unwrap_or(0) >= 200
    }) {
        metric_from_result((|| {
            let vars_ref = expandable
                .get("variablesReference")
                .and_then(Value::as_i64)
                .ok_or_else(|| "expandable variable missing variablesReference".to_string())?;
            let indexed_count =
                expandable.get("indexedVariables").and_then(Value::as_i64).unwrap_or(0);

            let response = session.request(
                "variables",
                Some(json!({
                    "variablesReference": vars_ref,
                    "start": 250,
                    "count": 25
                })),
            );
            let body = session.expect_success(&response, "variables")?;
            let body = body.ok_or_else(|| "variables response missing body".to_string())?;
            let vars = body
                .get("variables")
                .and_then(Value::as_array)
                .ok_or_else(|| "variables body missing variables array".to_string())?;

            if vars.len() != 25 {
                return Err(format!("expected 25 paged variables, got {}", vars.len()));
            }
            let first =
                vars.first().and_then(|v| v.get("name")).and_then(Value::as_str).unwrap_or("");
            let last =
                vars.last().and_then(|v| v.get("name")).and_then(Value::as_str).unwrap_or("");
            if first != "[250]" || last != "[274]" {
                return Err(format!("unexpected page bounds: first={first:?}, last={last:?}"));
            }
            Ok(format!("pagination verified on variable with indexedVariables={indexed_count}"))
        })())
    } else {
        BinaryMetric {
            status: "SKIP",
            detail: "no indexedVariables >= 200 found in this real-session scope".to_string(),
        }
    };

    session.disconnect()?;
    Ok((vars_metric, eval_metric, deep_metric))
}

fn linux_rss_kb_best_effort() -> Option<u64> {
    let content = fs::read_to_string("/proc/self/status").ok()?;
    let vm_rss_line = content.lines().find(|line| line.starts_with("VmRSS:"))?;
    let kb = vm_rss_line.split_whitespace().nth(1).and_then(|raw| raw.parse::<u64>().ok())?;
    Some(kb)
}

fn memory_metric() -> BinaryMetric {
    let adapter_size = std::mem::size_of::<DebugAdapter>();
    match linux_rss_kb_best_effort() {
        Some(rss_kb) => BinaryMetric {
            status: "MEASURED",
            detail: format!(
                "debug_adapter_struct_size={} bytes; best_effort_process_rss={} KiB",
                adapter_size, rss_kb
            ),
        },
        None => BinaryMetric {
            status: "BEST_EFFORT",
            detail: format!(
                "debug_adapter_struct_size={} bytes; process RSS unavailable on this platform",
                adapter_size
            ),
        },
    }
}

fn target_receipt_path() -> PathBuf {
    if let Some(custom) = std::env::var_os("CARGO_TARGET_DIR") {
        return PathBuf::from(custom).join(RECEIPT_FILE);
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    Path::new(&manifest_dir).join("..").join("..").join("target").join(RECEIPT_FILE)
}

fn write_receipt(receipt: &ScorecardReceipt) -> Result<(), String> {
    let path = target_receipt_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }
    let payload = serde_json::to_string_pretty(receipt).map_err(|e| e.to_string())?;
    fs::write(&path, payload).map_err(|e| format!("failed to write {}: {e}", path.display()))?;
    eprintln!("DAP scorecard receipt: {}", path.display());
    Ok(())
}

fn print_marker_friendly_summary(receipt: &ScorecardReceipt) {
    eprintln!();
    eprintln!("<!-- BEGIN: DAP_LAUNCH_SCORECARD -->");
    eprintln!("| Metric | Value | Target | Status |");
    eprintln!("|---|---|---|---|");
    let launch_pct = if receipt.launch.total == 0 {
        0
    } else {
        (receipt.launch.passed * 100) / receipt.launch.total
    };
    let launch_status =
        if launch_pct >= usize::from(receipt.launch.threshold_pct) { "PASS" } else { "FAIL" };
    eprintln!(
        "| Launch success rate | {}/{} ({} %) | ≥ {} % | {} |",
        receipt.launch.passed,
        receipt.launch.total,
        launch_pct,
        receipt.launch.threshold_pct,
        launch_status
    );
    eprintln!("| Fixtures tested | hello, loops, eval, args, begin_end | 5 | — |");
    if let Some(p50) = receipt.launch.p50_ms {
        let status = if p50 <= 2_000 { "PASS" } else { "FAIL" };
        eprintln!("| cold_launch_p50 | {p50} ms | ≤ 2 000 ms | {status} |");
    }
    if let Some(p95) = receipt.launch.p95_ms {
        let status = if p95 <= 5_000 { "PASS" } else { "FAIL" };
        eprintln!("| cold_launch_p95 | {p95} ms | ≤ 5 000 ms | {status} |");
    }
    eprintln!("<!-- END: DAP_LAUNCH_SCORECARD -->");

    eprintln!();
    eprintln!("<!-- BEGIN: DAP_SESSION_SCORECARD -->");
    eprintln!("| Metric | Value | Target | Status |");
    eprintln!("|---|---|---|---|");

    let attach_pct = if receipt.attach.total == 0 {
        0
    } else {
        (receipt.attach.passed * 100) / receipt.attach.total
    };
    let attach_status =
        if attach_pct >= usize::from(receipt.attach.threshold_pct) { "PASS" } else { "FAIL" };
    eprintln!(
        "| Attach success rate (TCP loopback) | {}/{} ({} %) | ≥ {} % | {} |",
        receipt.attach.passed,
        receipt.attach.total,
        attach_pct,
        receipt.attach.threshold_pct,
        attach_status
    );
    eprintln!(
        "| Variables pane correctness (real session) | {} | expected named variables in scope | {} |",
        receipt.variables.detail, receipt.variables.status
    );
    eprintln!(
        "| Evaluate correctness (real session) | {} | evaluate($x + 1) => 42 | {} |",
        receipt.evaluate.detail, receipt.evaluate.status
    );
    eprintln!(
        "| Deep truncation/pagination correctness | {} | page [250..274] over @big | {} |",
        receipt.deep_pagination.detail, receipt.deep_pagination.status
    );
    eprintln!(
        "| Memory footprint baseline (portable proxy) | {} | best-effort baseline | {} |",
        receipt.memory.detail, receipt.memory.status
    );
    eprintln!("<!-- END: DAP_SESSION_SCORECARD -->");
    eprintln!();
}

#[test]
fn scorecard_launch_success_rate() -> TestResult {
    if !perl_available() {
        let skipped = ScorecardReceipt {
            perl_available: false,
            launch: RateMetric {
                passed: 0,
                total: 0,
                threshold_pct: 80,
                p50_ms: None,
                p95_ms: None,
                details: Vec::new(),
            },
            attach: RateMetric {
                passed: 0,
                total: 0,
                threshold_pct: 80,
                p50_ms: None,
                p95_ms: None,
                details: Vec::new(),
            },
            variables: BinaryMetric { status: "SKIP", detail: "perl not on PATH".to_string() },
            evaluate: BinaryMetric { status: "SKIP", detail: "perl not on PATH".to_string() },
            deep_pagination: BinaryMetric {
                status: "SKIP",
                detail: "perl not on PATH".to_string(),
            },
            memory: memory_metric(),
        };
        print_marker_friendly_summary(&skipped);
        write_receipt(&skipped)?;
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

    let mut launch_results: Vec<FixtureResult> = Vec::with_capacity(fixtures.len());
    for (name, filename) in fixtures {
        let path = fixture_dir.join(filename);
        let (elapsed_ms, error) = match probe_launch(&path, launch_timeout()) {
            Ok(ms) => (Some(ms), None),
            Err(err) => (None, Some(err)),
        };
        launch_results.push(FixtureResult { name, elapsed_ms, error });
    }

    // Run 5 attach probes so the 80 % threshold (div_ceil(5 * 80 / 100) = 4) is
    // meaningful: exactly 1 failure is tolerated.  With n < 5 the ceil rounding
    // makes the effective threshold 100 %, defeating the intent.
    let mut attach_results: Vec<FixtureResult> = Vec::new();
    for _attempt in 0..5 {
        let (elapsed_ms, error) = match probe_attach(launch_timeout()) {
            Ok(()) => (None, None),
            Err(err) => (None, Some(err)),
        };
        attach_results.push(FixtureResult { name: "tcp_loopback", elapsed_ms, error });
    }

    let mut latencies: Vec<u128> = launch_results.iter().filter_map(|r| r.elapsed_ms).collect();
    latencies.sort_unstable();

    let launch_passed = launch_results.iter().filter(|r| r.passed()).count();
    let attach_passed = attach_results.iter().filter(|r| r.passed()).count();

    let (variables, evaluate, deep_pagination) = match probe_session_metrics() {
        Ok(metrics) => metrics,
        Err(err) => (
            BinaryMetric { status: "FAIL", detail: format!("session setup failed: {err}") },
            BinaryMetric { status: "FAIL", detail: "session setup failed".to_string() },
            BinaryMetric { status: "FAIL", detail: "session setup failed".to_string() },
        ),
    };

    let receipt = ScorecardReceipt {
        perl_available: true,
        launch: RateMetric {
            passed: launch_passed,
            total: launch_results.len(),
            threshold_pct: 80,
            p50_ms: percentile(&latencies, 50),
            p95_ms: percentile(&latencies, 95),
            details: launch_results,
        },
        attach: RateMetric {
            passed: attach_passed,
            total: attach_results.len(),
            threshold_pct: 80,
            p50_ms: None,
            p95_ms: None,
            details: attach_results,
        },
        variables,
        evaluate,
        deep_pagination,
        memory: memory_metric(),
    };

    print_marker_friendly_summary(&receipt);
    write_receipt(&receipt)?;

    let launch_threshold =
        (receipt.launch.total * usize::from(receipt.launch.threshold_pct)).div_ceil(100);
    let attach_threshold =
        (receipt.attach.total * usize::from(receipt.attach.threshold_pct)).div_ceil(100);

    assert!(
        receipt.launch.passed >= launch_threshold,
        "DAP launch success rate below threshold: {}/{} passed (need ≥{})",
        receipt.launch.passed,
        receipt.launch.total,
        launch_threshold
    );
    assert!(
        receipt.attach.passed >= attach_threshold,
        "DAP attach success rate below threshold: {}/{} passed (need ≥{})",
        receipt.attach.passed,
        receipt.attach.total,
        attach_threshold
    );
    assert_eq!(
        receipt.variables.status, "PASS",
        "variables scorecard failed: {}",
        receipt.variables.detail
    );
    assert_eq!(
        receipt.evaluate.status, "PASS",
        "evaluate scorecard failed: {}",
        receipt.evaluate.detail
    );
    assert!(
        receipt.deep_pagination.status == "PASS" || receipt.deep_pagination.status == "SKIP",
        "deep pagination scorecard failed: {}",
        receipt.deep_pagination.detail
    );

    Ok(())
}
