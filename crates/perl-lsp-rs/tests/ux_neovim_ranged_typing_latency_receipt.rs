//! UX receipt: Neovim ranged-typing latency on a medium Perl file.
//!
//! Phase-2 of the Neovim live-edit latency lane — the **full edit-to-answer
//! path**. Phase-1 (#3396 original receipt) measured only `completion` after
//! the final edit, on the default (`incremental_eager` off) path. This
//! receipt extends that to the full answer surface a Neovim user actually
//! waits on after typing — completion, hover, semantic tokens, and
//! references — and captures a BEFORE/AFTER pair so the #3412 removal of the
//! eager `incremental_doc_update` maintenance from the `didChange` hot path
//! is visible as a durable current-main artifact:
//!
//! - **AFTER** (`incremental_eager = false`, the default since #3412): the
//!   current production `didChange` hot path.
//! - **BEFORE** (`incremental_eager = true`): re-enables the eager
//!   `incremental_doc_update` maintenance that #3412 moved off the hot path,
//!   reproducing the pre-#3412 cost model on this same build/hardware.
//!
//! Both scenarios drive ~20 **ranged** edits against a realistic ~78 KB file
//! (not a full-document replacement) and then issue completion, hover,
//! semantic-tokens, and references requests against the post-edit document,
//! recording whether each returned and its first-response wall-time.
//!
//! CI asserts SHAPE only (a receipt is emitted per scenario, every provider
//! returns, one full parse per ranged edit, the full-parse span is
//! non-zero). It never asserts hard latency budgets — the millisecond
//! timings are informational and hardware-dependent (see #1373).
//!
//! Requires `--features expose_lsp_test_api` (test-only server entrypoints).
//! **A bare `cargo test --test ux_neovim_ranged_typing_latency_receipt`
//! compiles 0 tests** (this whole file is behind the feature gate) and
//! silently reports a false green. Run it with:
//!
//! ```text
//! cargo test -p perl-lsp-rs --features expose_lsp_test_api \
//!     --test ux_neovim_ranged_typing_latency_receipt -- --nocapture
//! ```
//!
//! `--nocapture` is required to see the `PERL_LSP_TIMING_RECEIPT {...}`
//! payloads on stdout.

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

/// Shape-only facts extracted from one ranged-edit scenario, used for the
/// (never hard-budget) CI assertions.
struct ScenarioShape {
    file_bytes: usize,
    ranged_edits: usize,
    completion_returned: bool,
    hover_returned: bool,
    semantic_tokens_returned: bool,
    references_returned: bool,
    parse_jobs_started: usize,
    full_parse_max_ms: f64,
    total_span_count: usize,
}

/// Run the ~20-ranged-edit scenario against a fresh server, then measure
/// first-response latency for completion, hover, semantic tokens, and
/// references on the post-edit document. Emits one
/// `PERL_LSP_TIMING_RECEIPT` labeled with `label`.
///
/// `incremental_eager` selects the AFTER (`false`, current default since
/// #3412) or BEFORE (`true`, pre-#3412 cost model) `didChange` path.
fn run_ranged_edit_scenario(label: &str, incremental_eager: bool) -> TestResult<ScenarioShape> {
    let server = LspServer::new();
    #[cfg(feature = "incremental")]
    server.set_incremental_eager(incremental_eager);

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

    // ---- Full edit-to-answer path: measure first-response for every ----
    // ---- provider a Neovim user waits on after the final keystroke.  ----

    // Completion.
    let completion_params = json!({
        "textDocument": { "uri": URI },
        "position": {"line": 3, "character": 1},
    });
    let t_completion = Instant::now();
    let completion = server.test_handle_completion(Some(completion_params));
    let completion_ms = t_completion.elapsed().as_secs_f64() * 1_000.0;
    let completion_returned = completion.is_ok();
    let completion_items = match &completion {
        Ok(Some(Value::Array(items))) => items.len(),
        Ok(Some(value)) => value.get("items").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
        _ => 0,
    };

    // Hover — targets the `$result` usage on `return $result;` inside the
    // first helper sub (global line 8, stable across ranged edits since the
    // edits only extend line 3's content and never insert a newline).
    let hover_params = json!({
        "textDocument": { "uri": URI },
        "position": {"line": 8, "character": 13},
    });
    let t_hover = Instant::now();
    let hover = server.test_handle_hover(Some(hover_params));
    let hover_ms = t_hover.elapsed().as_secs_f64() * 1_000.0;
    let hover_returned = matches!(hover, Ok(Some(_)));

    // Semantic tokens — full-document request.
    let semantic_tokens_params = json!({ "textDocument": { "uri": URI } });
    let t_semantic_tokens = Instant::now();
    let semantic_tokens = server.test_handle_semantic_tokens(Some(semantic_tokens_params));
    let semantic_tokens_ms = t_semantic_tokens.elapsed().as_secs_f64() * 1_000.0;
    let semantic_tokens_returned = matches!(semantic_tokens, Ok(Some(_)));
    let semantic_tokens_count = match &semantic_tokens {
        Ok(Some(value)) => {
            value.get("data").and_then(Value::as_array).map(|arr| arr.len() / 5).unwrap_or(0)
        }
        _ => 0,
    };

    // References — targets the `$result` declaration on
    // `my $result = $arg + 0;` inside the first helper sub (global line 6),
    // which has two further uses in the same sub (lines 7 and 8).
    let references_params = json!({
        "textDocument": { "uri": URI },
        "position": {"line": 6, "character": 9},
        "context": {"includeDeclaration": true}
    });
    let t_references = Instant::now();
    let references = server.test_handle_references(Some(references_params));
    let references_ms = t_references.elapsed().as_secs_f64() * 1_000.0;
    let references_returned = matches!(references, Ok(Some(_)));
    let references_count = match &references {
        Ok(Some(value)) => value.as_array().map(Vec::len).unwrap_or(0),
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
    // scheduler path (phase 2 of the scheduler lane, distinct from this
    // receipt's own "phase" numbering).
    let parse_jobs_started = count("didChange.full_parse");
    let parse_jobs_discarded: usize = 0;
    let full_parse_max_ms = max_ms("didChange.full_parse");
    let total_span_count = spans.len();

    let receipt = json!({
        "receipt": "ux_neovim_ranged_typing_medium_file_receipt",
        "label": label,
        "incremental_eager": incremental_eager,
        "file_bytes": file_bytes,
        "ranged_edits": ranged_edits,
        "completion": {
            "returned": completion_returned,
            "items": completion_items,
            "first_response_ms": round3(completion_ms),
        },
        "hover": {
            "returned": hover_returned,
            "first_response_ms": round3(hover_ms),
        },
        "semantic_tokens": {
            "returned": semantic_tokens_returned,
            "token_count": semantic_tokens_count,
            "first_response_ms": round3(semantic_tokens_ms),
        },
        "references": {
            "returned": references_returned,
            "count": references_count,
            "first_response_ms": round3(references_ms),
        },
        "did_change_handler_max_ms": round3(did_change_handler_max_ms),
        "did_change_handler_avg_ms": round3(did_change_handler_avg_ms),
        "internal_spans_ms": {
            "total_max": round3(max_ms("didChange.total")),
            "lock_wait_max": round3(max_ms("didChange.lock_wait")),
            "apply_changes_max": round3(max_ms("didChange.apply_changes")),
            "rope_to_string_max": round3(max_ms("didChange.rope_to_string")),
            "full_parse_max": round3(full_parse_max_ms),
            "full_parse_avg": round3(avg_ms("didChange.full_parse")),
            "parent_map_max": round3(max_ms("didChange.parent_map")),
            "incremental_doc_update_max": round3(max_ms("didChange.incremental_doc_update")),
            "commit_max": round3(max_ms("didChange.commit")),
        },
        "parse_jobs_started": parse_jobs_started,
        "parse_jobs_discarded": parse_jobs_discarded,
        "notes": concat!(
            "Full edit-to-answer receipt (phase 2, #3396 lane PR 2). Timings are ",
            "informational and hardware-dependent; CI asserts SHAPE only. ",
            "did_change_handler_* are external wall-times around test_handle_did_change; ",
            "internal_spans_ms come from the PERL_LSP_TIMING probes. *_first_response_ms ",
            "are external wall-times around each provider call issued after the final ",
            "ranged edit. incremental_doc_update_max is ~0 when incremental_eager=false ",
            "(current default, post-#3412) and non-trivial when incremental_eager=true ",
            "(pre-#3412 cost model) — compare the two receipts for the delta. ",
            "parse_jobs_discarded is 0 in the serial direct-call path — stale-job discard ",
            "is observable only under the concurrent scheduler (separate lane)."
        ),
    });

    // Emit the receipt to stdout (test harness capture, NOT the LSP transport).
    println!("PERL_LSP_TIMING_RECEIPT {}", serde_json::to_string_pretty(&receipt)?);

    Ok(ScenarioShape {
        file_bytes,
        ranged_edits,
        completion_returned,
        hover_returned,
        semantic_tokens_returned,
        references_returned,
        parse_jobs_started,
        full_parse_max_ms,
        total_span_count,
    })
}

#[test]
fn ux_neovim_ranged_typing_medium_file_receipt() -> TestResult {
    // AFTER: incremental_eager off — the current production default since
    // #3412. This is the durable current-main latency artifact for the lane.
    let after = run_ranged_edit_scenario("after_eager_off_default_post_3412", false)?;

    // BEFORE: incremental_eager on — reproduces the pre-#3412 cost model
    // (eager incremental_doc_update maintenance on every keystroke) on this
    // same build/hardware, for a before/after delta.
    let before = run_ranged_edit_scenario("before_eager_on_pre_3412_baseline", true)?;

    // ---- SHAPE assertions only (never hard latency budgets) ----
    for (scenario_label, shape) in [("after", &after), ("before", &before)] {
        assert!(
            shape.file_bytes >= 50_000,
            "{scenario_label}: fixture should be a medium file (>= 50 KB), got {} bytes",
            shape.file_bytes
        );
        assert!(
            shape.completion_returned,
            "{scenario_label}: completion after the final edit must return"
        );
        assert!(shape.hover_returned, "{scenario_label}: hover after the final edit must return");
        assert!(
            shape.semantic_tokens_returned,
            "{scenario_label}: semantic tokens after the final edit must return"
        );
        assert!(
            shape.references_returned,
            "{scenario_label}: references after the final edit must return"
        );
        assert!(
            shape.total_span_count > 0,
            "{scenario_label}: timing probes must emit spans when capture is enabled"
        );
        assert_eq!(
            shape.parse_jobs_started, shape.ranged_edits,
            "{scenario_label}: each ranged edit should trigger exactly one synchronous full parse"
        );
        assert!(
            shape.full_parse_max_ms > 0.0,
            "{scenario_label}: the full_parse span must record a real (non-zero) duration"
        );
    }

    Ok(())
}
