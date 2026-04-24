//! DAP scorecard harness.
//!
//! Tracks launch reliability plus real-session debugger quality probes for:
//! attach workflows, variables pane correctness, evaluate correctness,
//! deep variable pagination, and a best-effort memory baseline proxy.
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
use serde::Serialize;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const RECEIPT_PATH: &str = "target/receipts/dap-scorecard.json";

// ─── Timeout helpers ──────────────────────────────────────────────────────────

/// Test timeout, inflated under coverage/profiling to avoid false failures.
fn smoke_timeout() -> Duration {
    if std::env::var_os("LLVM_PROFILE_FILE").is_some()
        || std::env::var_os("CARGO_LLVM_COV").is_some()
    {
        Duration::from_secs(60)
    } else {
        Duration::from_secs(10)
    }
}

// ─── Scorecard data model ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
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
struct MetricRow {
    metric: String,
    value: String,
    target: String,
    status: String,
    detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ScorecardReceipt {
    schema_version: u32,
    scorecard: String,
    generated_at_utc: String,
    perl_available: bool,
    launch_rows: Vec<MetricRow>,
    session_rows: Vec<MetricRow>,
}

// ─── Low-level event waiter ───────────────────────────────────────────────────

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

// ─── Launch probes ────────────────────────────────────────────────────────────

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

fn probe_launch_scorecard() -> (Vec<FixtureResult>, Vec<MetricRow>, usize, usize) {
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

    let passed = results.iter().filter(|r| r.passed()).count();
    let total = results.len();

    let mut latencies: Vec<u128> = results.iter().filter_map(|r| r.elapsed_ms).collect();
    latencies.sort_unstable();
    let p50 = percentile(&latencies, 50);
    let p95 = percentile(&latencies, 95);

    let mut rows = vec![MetricRow {
        metric: "Launch success rate".to_string(),
        value: format!("{passed}/{total} ({} %)", (passed * 100) / total.max(1)),
        target: "≥ 80 %".to_string(),
        status: if passed * 5 >= total * 4 { "PASS" } else { "FAIL" }.to_string(),
        detail: None,
    }];

    rows.push(MetricRow {
        metric: "Fixtures tested".to_string(),
        value: fixtures.iter().map(|(name, _)| *name).collect::<Vec<_>>().join(", "),
        target: "5".to_string(),
        status: "—".to_string(),
        detail: None,
    });

    rows.push(MetricRow {
        metric: "cold_launch_p50".to_string(),
        value: p50.map_or_else(|| "—".to_string(), |v| format!("{v} ms")),
        target: "≤ 2 000 ms".to_string(),
        status: p50.map_or("SKIP", |v| if v <= 2000 { "PASS" } else { "FAIL" }).to_string(),
        detail: None,
    });

    rows.push(MetricRow {
        metric: "cold_launch_p95".to_string(),
        value: p95.map_or_else(|| "—".to_string(), |v| format!("{v} ms")),
        target: "≤ 5 000 ms".to_string(),
        status: p95.map_or("SKIP", |v| if v <= 5000 { "PASS" } else { "FAIL" }).to_string(),
        detail: None,
    });

    (results, rows, passed, total)
}

// ─── Session probes ───────────────────────────────────────────────────────────

fn launch_with_breakpoint(
    script_path: &Path,
    break_line: u64,
) -> Result<DapWorkflowSession, String> {
    let timeout = workflow_timeout();
    let mut session = DapWorkflowSession::new(timeout)?;

    let script_str = script_path.to_str().ok_or("script path is not valid UTF-8")?.to_string();

    session.launch(&script_str)?;
    session.set_breakpoints(&script_str, &[break_line])?;
    session.configuration_done()?;

    Ok(session)
}

fn probe_attach_success_rate() -> MetricRow {
    let timeout = workflow_timeout();
    let mut passed = 0usize;
    let mut total = 0usize;
    let mut failures = Vec::new();

    for stop_on_entry in [false, true] {
        total += 1;
        let result = (|| -> Result<(), String> {
            let mut session = DapWorkflowSession::new(timeout)?;
            session.attach(std::process::id(), stop_on_entry)?;

            let first = session.wait_stopped()?;
            if first.reason != "attach" {
                return Err(format!("first stop reason `{}`", first.reason));
            }

            if stop_on_entry {
                let second = session.wait_stopped()?;
                if second.reason != "entry" {
                    return Err(format!("second stop reason `{}`", second.reason));
                }
            }

            session.disconnect()?;
            Ok(())
        })();

        if result.is_ok() {
            passed += 1;
        } else if let Err(error) = result {
            failures.push(format!("stopOnEntry={stop_on_entry}: {error}"));
        }
    }

    MetricRow {
        metric: "Attach success rate".to_string(),
        value: format!("{passed}/{total} ({} %)", (passed * 100) / total.max(1)),
        target: "100 % (2/2)".to_string(),
        status: if passed == total { "PASS" } else { "FAIL" }.to_string(),
        detail: (!failures.is_empty()).then(|| failures.join("; ")),
    }
}

fn probe_variables_correctness() -> MetricRow {
    let result = (|| -> Result<String, String> {
        let workspace = tempdir().map_err(|e| e.to_string())?;
        let script = workspace.path().join("variables_scorecard.pl");
        fs::write(
            &script,
            "use strict;\nuse warnings;\n\nmy $x = 10;\nmy $y = 20;\nmy $sum = $x + $y;\nmy $product = $x * $y;\nprint \"$sum/$product\\n\";\n",
        )
        .map_err(|e| e.to_string())?;

        let mut session = launch_with_breakpoint(&script, 8)?;
        let stopped = session.wait_stopped()?;
        let (frame_id, _, _) = session.stack_trace(stopped.thread_id)?;
        let locals_ref = session.scopes_locals_ref(frame_id)?;
        let vars = session.variables(locals_ref)?;
        let mut names = vars
            .iter()
            .filter_map(|v| v.get("name").and_then(Value::as_str))
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        names.sort_unstable();

        let has_sum = names.iter().any(|name| name == "$sum");
        let has_product = names.iter().any(|name| name == "$product");
        session.disconnect()?;

        if has_sum && has_product {
            Ok(format!("locals include $sum/$product ({} vars)", names.len()))
        } else {
            Err(format!("locals missing expected vars; got [{}]", names.join(", ")))
        }
    })();

    match result {
        Ok(detail) => MetricRow {
            metric: "Variables pane correctness (session)".to_string(),
            value: "PASS".to_string(),
            target: "Locals include computed vars".to_string(),
            status: "PASS".to_string(),
            detail: Some(detail),
        },
        Err(error) => MetricRow {
            metric: "Variables pane correctness (session)".to_string(),
            value: "FAIL".to_string(),
            target: "Locals include computed vars".to_string(),
            status: "FAIL".to_string(),
            detail: Some(error),
        },
    }
}

fn probe_evaluate_correctness() -> MetricRow {
    let result = (|| -> Result<String, String> {
        let workspace = tempdir().map_err(|e| e.to_string())?;
        let script = workspace.path().join("evaluate_scorecard.pl");
        fs::write(
            &script,
            "use strict;\nuse warnings;\n\nmy $x = 10;\nmy $y = 20;\nmy $sum = $x + $y;\nprint \"$sum\\n\";\n",
        )
        .map_err(|e| e.to_string())?;

        let mut session = launch_with_breakpoint(&script, 7)?;
        let stop = session.wait_stopped()?;

        let response = session.request(
            "evaluate",
            Some(json!({
                "expression": "$sum",
                "frameId": 1,
                "context": "watch"
            })),
        );

        let body = session.expect_success(&response, "evaluate")?;
        session.disconnect()?;

        let body = body.ok_or("evaluate response missing body")?;
        let result_text = body
            .get("result")
            .and_then(Value::as_str)
            .ok_or("evaluate result missing `result`")?
            .to_string();

        if result_text.contains("30") {
            Ok(format!("evaluate($sum) => {result_text}; thread={}", stop.thread_id))
        } else {
            Err(format!("evaluate($sum) expected 30, got `{result_text}`"))
        }
    })();

    match result {
        Ok(detail) => MetricRow {
            metric: "Evaluate correctness (session)".to_string(),
            value: "PASS".to_string(),
            target: "evaluate($sum) includes 30".to_string(),
            status: "PASS".to_string(),
            detail: Some(detail),
        },
        Err(error) => MetricRow {
            metric: "Evaluate correctness (session)".to_string(),
            value: "FAIL".to_string(),
            target: "evaluate($sum) includes 30".to_string(),
            status: "FAIL".to_string(),
            detail: Some(error),
        },
    }
}

fn probe_deep_pagination() -> MetricRow {
    let result = (|| -> Result<String, String> {
        let workspace = tempdir().map_err(|e| e.to_string())?;
        let script = workspace.path().join("pagination_scorecard.pl");
        fs::write(
            &script,
            "use strict;\nuse warnings;\n\nmy @items = (1..300);\nmy $count = scalar @items;\nprint \"$count\\n\";\n",
        )
        .map_err(|e| e.to_string())?;

        let mut session = launch_with_breakpoint(&script, 6)?;
        let stop = session.wait_stopped()?;
        let (frame_id, _, _) = session.stack_trace(stop.thread_id)?;
        let locals_ref = session.scopes_locals_ref(frame_id)?;
        let locals = session.variables(locals_ref)?;

        let array_var = locals
            .iter()
            .find(|var| var.get("name").and_then(Value::as_str) == Some("@items"))
            .ok_or("@items not found in locals")?;

        let children_ref = array_var
            .get("variablesReference")
            .and_then(Value::as_i64)
            .ok_or("@items missing variablesReference")?;
        if children_ref <= 0 {
            return Err("@items is not expandable".to_string());
        }

        let page_0_resp = session.request(
            "variables",
            Some(json!({"variablesReference": children_ref, "start": 0, "count": 5})),
        );
        let page_200_resp = session.request(
            "variables",
            Some(json!({"variablesReference": children_ref, "start": 200, "count": 5})),
        );

        let page_0_body =
            session.expect_success(&page_0_resp, "variables")?.ok_or("page 0 missing body")?;
        let page_200_body =
            session.expect_success(&page_200_resp, "variables")?.ok_or("page 200 missing body")?;

        let page_0 = page_0_body
            .get("variables")
            .and_then(Value::as_array)
            .ok_or("page 0 missing variables")?;
        let page_200 = page_200_body
            .get("variables")
            .and_then(Value::as_array)
            .ok_or("page 200 missing variables")?;

        let first_0 = page_0
            .first()
            .and_then(|v| v.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_string();
        let first_200 = page_200
            .first()
            .and_then(|v| v.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("?")
            .to_string();

        session.disconnect()?;

        if page_0.is_empty() || page_200.is_empty() {
            return Err("pagination returned empty page(s)".to_string());
        }
        if page_0.len() > 5 || page_200.len() > 5 {
            return Err(format!(
                "pagination ignored count=5 (sizes: {}, {})",
                page_0.len(),
                page_200.len()
            ));
        }
        if first_0 == first_200 {
            return Err(format!("page 0 and page 200 overlap at `{first_0}`"));
        }

        Ok(format!("page0_first={first_0}, page200_first={first_200}"))
    })();

    match result {
        Ok(detail) => MetricRow {
            metric: "Deep truncation/pagination correctness".to_string(),
            value: "PASS".to_string(),
            target: "Distinct pages (start=0 vs 200, count=5)".to_string(),
            status: "PASS".to_string(),
            detail: Some(detail),
        },
        Err(error) => MetricRow {
            metric: "Deep truncation/pagination correctness".to_string(),
            value: "FAIL".to_string(),
            target: "Distinct pages (start=0 vs 200, count=5)".to_string(),
            status: "FAIL".to_string(),
            detail: Some(error),
        },
    }
}

#[cfg(target_os = "linux")]
fn probe_memory_baseline() -> MetricRow {
    fn rss_kib() -> Option<u64> {
        let statm = fs::read_to_string("/proc/self/statm").ok()?;
        let fields = statm.split_whitespace().collect::<Vec<_>>();
        let resident_pages = fields.get(1)?.parse::<u64>().ok()?;
        let page_size = 4096u64;
        Some((resident_pages * page_size) / 1024)
    }

    let before = rss_kib();
    let launch_result = (|| -> Result<(), String> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let script = Path::new(&manifest_dir).join("tests").join("fixtures").join("hello.pl");
        let _ = probe_launch(&script, smoke_timeout())?;
        Ok(())
    })();
    let after = rss_kib();

    match (before, after, launch_result) {
        (Some(start), Some(end), Ok(())) => MetricRow {
            metric: "Memory footprint baseline (best effort)".to_string(),
            value: format!(
                "rss_before={} KiB, rss_after={} KiB, delta={} KiB",
                start,
                end,
                end.saturating_sub(start)
            ),
            target: "Informational baseline".to_string(),
            status: "MEASURED".to_string(),
            detail: Some("Linux /proc/self/statm proxy".to_string()),
        },
        (_, _, Err(error)) => MetricRow {
            metric: "Memory footprint baseline (best effort)".to_string(),
            value: "SKIP".to_string(),
            target: "Informational baseline".to_string(),
            status: "SKIP".to_string(),
            detail: Some(error),
        },
        _ => MetricRow {
            metric: "Memory footprint baseline (best effort)".to_string(),
            value: "SKIP".to_string(),
            target: "Informational baseline".to_string(),
            status: "SKIP".to_string(),
            detail: Some("rss snapshot unavailable".to_string()),
        },
    }
}

#[cfg(not(target_os = "linux"))]
fn probe_memory_baseline() -> MetricRow {
    MetricRow {
        metric: "Memory footprint baseline (best effort)".to_string(),
        value: "SKIP".to_string(),
        target: "Informational baseline".to_string(),
        status: "SKIP".to_string(),
        detail: Some("portable RSS baseline currently implemented only on Linux".to_string()),
    }
}

fn receipt_path() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    Path::new(&manifest_dir).join("..").join("..").join(RECEIPT_PATH)
}

fn generated_at_utc_iso8601() -> String {
    let now = std::time::SystemTime::now();
    match now.duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => format!("unix:{}", duration.as_secs()),
        Err(_) => "unix:0".to_string(),
    }
}

fn write_receipt(receipt: &ScorecardReceipt) -> Result<(), String> {
    let path = receipt_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(receipt).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    eprintln!("  wrote DAP scorecard receipt: {}", path.display());
    Ok(())
}

fn print_table(title: &str, rows: &[MetricRow]) {
    eprintln!();
    eprintln!("{title}");
    eprintln!("| Metric | Value | Target | Status |");
    eprintln!("|---|---|---|---|");
    for row in rows {
        eprintln!("| {} | {} | {} | {} |", row.metric, row.value, row.target, row.status);
    }
}

fn print_details(rows: &[MetricRow]) {
    for row in rows {
        if let Some(detail) = &row.detail {
            eprintln!("  - {}: {}", row.metric, detail);
        }
    }
}

#[test]
fn scorecard_launch_and_session_quality() -> TestResult {
    if !perl_available() {
        eprintln!("scorecard_launch_and_session_quality: skipping — perl not on PATH");
        let skip_row = MetricRow {
            metric: "Perl runtime availability".to_string(),
            value: "SKIP".to_string(),
            target: "perl on PATH".to_string(),
            status: "SKIP".to_string(),
            detail: Some("perl not on PATH".to_string()),
        };
        let receipt = ScorecardReceipt {
            schema_version: 1,
            scorecard: "dap".to_string(),
            generated_at_utc: generated_at_utc_iso8601(),
            perl_available: false,
            launch_rows: vec![skip_row.clone()],
            session_rows: vec![skip_row],
        };
        write_receipt(&receipt)?;
        return Ok(());
    }

    let (launch_fixture_results, launch_rows, launch_passed, launch_total) =
        probe_launch_scorecard();

    let session_rows = vec![
        probe_attach_success_rate(),
        probe_variables_correctness(),
        probe_evaluate_correctness(),
        probe_deep_pagination(),
        probe_memory_baseline(),
    ];

    print_table("DAP Launch Scorecard", &launch_rows);
    for result in &launch_fixture_results {
        let status = if result.passed() { "PASS" } else { "FAIL" };
        let latency = result.elapsed_ms.map_or_else(|| "—".to_string(), |v| format!("{v} ms"));
        let detail = result.error.clone().unwrap_or_default();
        eprintln!("  - fixture={} status={} latency={} {}", result.name, status, latency, detail);
    }

    print_table("DAP Session Quality Scorecard", &session_rows);
    print_details(&session_rows);

    let receipt = ScorecardReceipt {
        schema_version: 1,
        scorecard: "dap".to_string(),
        generated_at_utc: generated_at_utc_iso8601(),
        perl_available: true,
        launch_rows,
        session_rows,
    };
    write_receipt(&receipt)?;

    let threshold = (launch_total * 4).div_ceil(5);
    assert!(
        launch_passed >= threshold,
        "DAP launch success rate below threshold: {launch_passed}/{launch_total} passed (need ≥{threshold}). \
         Failed fixtures: {}",
        launch_fixture_results
            .iter()
            .filter(|r| !r.passed())
            .map(|r| format!("{} ({})", r.name, r.error.as_deref().unwrap_or("?")))
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(())
}
