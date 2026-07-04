//! UX receipt: Neovim ranged-typing latency on a medium Perl file.
//!
//! Phase-1 of the Neovim live-edit latency lane — **instrumentation only**.
//! This receipt proves the current bottleneck: `textDocument/didChange`
//! synchronously runs a FULL parse plus a parent-map rebuild while holding the
//! documents lock, so completion latency after a keystroke includes full-parse
//! latency. Prior lean-mode receipts only exercised a tiny 5-line source with a
//! full-document replacement; this one drives ~20 **ranged** edits against a
//! realistic ~78 KB file and issues a completion after the final edit.
//!
//! CI asserts SHAPE only (a receipt is emitted, the completion returned, one
//! full parse per ranged edit). It never asserts hard latency budgets — the
//! millisecond timings are informational and hardware-dependent.
//!
//! Requires `--features expose_lsp_test_api` (test-only server entrypoints).

#![cfg(feature = "expose_lsp_test_api")]
// This is a receipt test whose deliverable is a printed JSON receipt; stdout is
// the test-harness capture stream (never the LSP transport), so `println!` is
// intentional here. The workspace otherwise denies `print_stdout`.
#![allow(clippy::print_stdout)]

use perl_lsp::LspServer;
use serde_json::{Value, json};
use std::time::Instant;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const URI: &str = "file:///workspace/lib/Medium/Fixture.pm";

/// Round to 3 decimal places for a stable, readable receipt.
fn round3(value: f64) -> f64 {
    (value * 1_000.0).round() / 1_000.0
}

/// Deterministically generate a medium (>= 50 KB) Perl source file.
///
/// The shape is realistic (package + many small subs) so the parser does real
/// work, and it is fully deterministic so the receipt is reproducible.
fn medium_fixture() -> String {
    let mut src = String::with_capacity(90_000);
    src.push_str("package Medium::Fixture;\nuse strict;\nuse warnings;\n\n");
    for i in 0..600 {
        // ~130 bytes/block * 600 ≈ 78 KB.
        src.push_str(&format!(
            "sub helper_{i:05} {{\n    my ($self, $arg) = @_;\n    my $result = $arg + {i};\n    $result = $result * 2 if $arg > 0;\n    return $result;\n}}\n\n"
        ));
    }
    src.push_str("1;\n");
    src
}

#[test]
fn ux_neovim_ranged_typing_medium_file_receipt() -> TestResult {
    let server = LspServer::new();
    let source = medium_fixture();
    let file_bytes = source.len();

    // Open the medium file (version 1).
    server.test_apply_did_open(URI, &source, 1)?;

    let ranged_edits: usize = 20;

    // Capture the internal PERL_LSP_TIMING spans for the receipt breakdown.
    // This is independent of the env sink, so it does not race on env state.
    server.test_timing_capture_start();

    // Send ~20 RANGED edits (zero-width insertions at the start of the blank
    // line 3). Each is a small ranged change — NOT a full-document replacement.
    let mut handler_times_ms: Vec<f64> = Vec::with_capacity(ranged_edits);
    for n in 0..ranged_edits {
        let version = 2 + i32::try_from(n)?;
        let params = json!({
            "textDocument": { "uri": URI, "version": version },
            "contentChanges": [{
                "range": {
                    "start": {"line": 3, "character": 0},
                    "end": {"line": 3, "character": 0},
                },
                "text": "#"
            }]
        });
        let start = Instant::now();
        server.test_handle_did_change(Some(params))?;
        handler_times_ms.push(start.elapsed().as_secs_f64() * 1_000.0);
    }

    // Issue a completion request after the final edit.
    let completion_params = json!({
        "textDocument": { "uri": URI },
        "position": {"line": 3, "character": 1},
    });
    let completion = server.test_handle_completion(Some(completion_params));
    let latest_completion_returned = completion.is_ok();
    let completion_items = match &completion {
        Ok(Some(Value::Array(items))) => items.len(),
        Ok(Some(value)) => value.get("items").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
        _ => 0,
    };

    // Drain the captured spans.
    let spans = server.test_timing_capture_drain();
    let max_ms = |name: &str| -> f64 {
        spans
            .iter()
            .filter(|(span, _, _)| span == name)
            .map(|(_, ms, _)| *ms)
            .fold(0.0_f64, f64::max)
    };
    let avg_ms = |name: &str| -> f64 {
        let values: Vec<f64> =
            spans.iter().filter(|(span, _, _)| span == name).map(|(_, ms, _)| *ms).collect();
        if values.is_empty() { 0.0 } else { values.iter().sum::<f64>() / values.len() as f64 }
    };
    let count = |name: &str| -> usize { spans.iter().filter(|(span, _, _)| span == name).count() };

    let did_change_handler_max_ms = handler_times_ms.iter().copied().fold(0.0_f64, f64::max);
    let did_change_handler_avg_ms = if handler_times_ms.is_empty() {
        0.0
    } else {
        handler_times_ms.iter().sum::<f64>() / handler_times_ms.len() as f64
    };

    // In the direct (non-scheduler) test path, edits are serial, so no parse is
    // discarded. Stale-job discard is only observable under the concurrent
    // scheduler path (phase 2).
    let parse_jobs_started = count("didChange.full_parse");
    let parse_jobs_discarded: usize = 0;

    let receipt = json!({
        "receipt": "ux_neovim_ranged_typing_medium_file_receipt",
        "file_bytes": file_bytes,
        "ranged_edits": ranged_edits,
        "latest_completion_returned": latest_completion_returned,
        "completion_items": completion_items,
        "did_change_handler_max_ms": round3(did_change_handler_max_ms),
        "did_change_handler_avg_ms": round3(did_change_handler_avg_ms),
        "internal_spans_ms": {
            "total_max": round3(max_ms("didChange.total")),
            "lock_wait_max": round3(max_ms("didChange.lock_wait")),
            "apply_changes_max": round3(max_ms("didChange.apply_changes")),
            "rope_to_string_max": round3(max_ms("didChange.rope_to_string")),
            "full_parse_max": round3(max_ms("didChange.full_parse")),
            "full_parse_avg": round3(avg_ms("didChange.full_parse")),
            "parent_map_max": round3(max_ms("didChange.parent_map")),
            "incremental_doc_update_max": round3(max_ms("didChange.incremental_doc_update")),
            "commit_max": round3(max_ms("didChange.commit")),
        },
        "parse_jobs_started": parse_jobs_started,
        "parse_jobs_discarded": parse_jobs_discarded,
        "notes": concat!(
            "Instrumentation-only receipt (phase 1). Timings are informational and ",
            "hardware-dependent; CI asserts SHAPE only. did_change_handler_* are external ",
            "wall-times around test_handle_did_change; internal_spans_ms come from the ",
            "PERL_LSP_TIMING probes. full_parse_max shows the synchronous full parse that ",
            "runs under the documents lock on every keystroke. parse_jobs_discarded is 0 in ",
            "the serial direct-call path — stale-job discard is observable only under the ",
            "concurrent scheduler (phase 2)."
        ),
    });

    // Emit the receipt to stdout (test harness capture, NOT the LSP transport).
    println!("PERL_LSP_TIMING_RECEIPT {}", serde_json::to_string_pretty(&receipt)?);

    // ---- SHAPE assertions only (never hard latency budgets) ----
    assert!(
        file_bytes >= 50_000,
        "fixture should be a medium file (>= 50 KB), got {file_bytes} bytes"
    );
    assert!(latest_completion_returned, "completion after the final edit must return");
    assert!(!spans.is_empty(), "timing probes must emit spans when capture is enabled");
    assert_eq!(
        count("didChange.total"),
        ranged_edits,
        "exactly one didChange.total span per ranged edit"
    );
    assert_eq!(
        parse_jobs_started, ranged_edits,
        "each ranged edit should trigger exactly one synchronous full parse"
    );
    assert!(
        max_ms("didChange.full_parse") > 0.0,
        "the full_parse span must record a real (non-zero) duration"
    );

    Ok(())
}
