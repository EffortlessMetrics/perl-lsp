//! DAP launch-success scorecard harness.
//!
//! Measures the launch success rate across the five standard fixture debuggees:
//! hello.pl, loops.pl, eval.pl, args.pl, breakpoints_begin_end.pl.
//!
//! For each fixture: initialize → launch (stopOnEntry=true) → wait for
//! `stopped` event.  Records elapsed time from launch request to first
//! `stopped` event for latency percentiles.  Asserts that ≥4/5 (80%) of
//! launches succeed.
//!
//! Emits human-readable scorecard output via `eprintln!` so it surfaces in
//! `cargo test -- --nocapture` and CI logs without asserting on exact timing
//! numbers.
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
use serde_json::json;
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

type TestResult = Result<(), Box<dyn std::error::Error>>;

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

// ─── Low-level event waiter (copied from dap_smoke_e2e.rs) ───────────────────

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

// ─── Per-fixture result ───────────────────────────────────────────────────────

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

// ─── Single fixture launch probe ─────────────────────────────────────────────

/// Launch `script_path` with stopOnEntry=true; wait for `stopped` event.
///
/// Returns elapsed milliseconds from launch request to `stopped` event, or an
/// error string describing what went wrong.
fn probe_launch(script_path: &Path, timeout: Duration) -> Result<u128, String> {
    let script_str =
        script_path.to_str().ok_or("fixture path contains non-UTF-8 characters")?.to_string();

    let mut adapter = DebugAdapter::new();
    let (tx, rx) = channel();
    adapter.set_event_sender(tx);

    // initialize
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

    // launch with stopOnEntry=true — measure from here to first `stopped`
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

    // Clean disconnect
    let _ = adapter.handle_request(3, "disconnect", Some(json!({})));

    Ok(elapsed_ms)
}

// ─── Percentile helper ────────────────────────────────────────────────────────

/// Compute a percentile over a sorted slice of `u128` values.
///
/// `pct` must be in `[0, 100]`.  Returns `None` if `values` is empty.
fn percentile(sorted: &[u128], pct: u8) -> Option<u128> {
    if sorted.is_empty() {
        return None;
    }
    // Nearest-rank method
    let rank = ((pct as f64 / 100.0) * sorted.len() as f64).ceil() as usize;
    let idx = rank.saturating_sub(1).min(sorted.len() - 1);
    Some(sorted[idx])
}

// ─── Scorecard test ───────────────────────────────────────────────────────────

/// DAP launch-success rate across 5 standard fixture debuggees.
///
/// Asserts ≥ 80 % pass rate (≥4/5).  Emits p50/p95 latency to stdout for
/// inclusion in the dap.md metrics table.
#[test]
fn scorecard_launch_success_rate() -> TestResult {
    if !perl_available() {
        eprintln!("scorecard_launch_success_rate: skipping — perl not on PATH");
        return Ok(());
    }

    // Resolve fixture directory relative to this file's Cargo manifest.
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

    // ── Print scorecard table ─────────────────────────────────────────────────
    eprintln!();
    eprintln!("┌─────────────────── DAP Launch Scorecard ────────────────────────────────┐");
    eprintln!("│ Fixture              │ Result    │ Latency (ms) │ Detail                 │");
    eprintln!("├──────────────────────┼───────────┼──────────────┼────────────────────────┤");
    for r in &results {
        let status = if r.passed() { "PASS" } else { "FAIL" };
        let latency = r.elapsed_ms.map(|ms| ms.to_string()).unwrap_or_else(|| "—".to_string());
        let detail = r.error.as_deref().unwrap_or("");
        // Truncate detail to keep the table readable
        let detail_trunc = if detail.len() > 22 { &detail[..22] } else { detail };
        eprintln!("│ {:<20} │ {:<9} │ {:>12} │ {:<22} │", r.name, status, latency, detail_trunc);
    }
    eprintln!("└─────────────────────────────────────────────────────────────────────────┘");

    // ── Latency percentiles ───────────────────────────────────────────────────
    let mut latencies: Vec<u128> = results.iter().filter_map(|r| r.elapsed_ms).collect();
    latencies.sort_unstable();

    if let (Some(p50), Some(p95)) = (percentile(&latencies, 50), percentile(&latencies, 95)) {
        eprintln!();
        eprintln!(
            "  cold_launch_p50 = {}ms   cold_launch_p95 = {}ms   (n={})",
            p50,
            p95,
            latencies.len()
        );
    }
    eprintln!();

    // ── Assertion: ≥ 80 % pass rate ──────────────────────────────────────────
    let passed = results.iter().filter(|r| r.passed()).count();
    let total = results.len();
    let threshold = (total * 4).div_ceil(5); // ceil(80 %) = 4 out of 5

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

    eprintln!("  scorecard_launch_success_rate: {passed}/{total} passed");

    Ok(())
}
