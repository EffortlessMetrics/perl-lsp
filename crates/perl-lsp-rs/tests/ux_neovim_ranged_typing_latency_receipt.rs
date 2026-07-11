//! UX receipt: Neovim ranged-typing latency on a medium Perl file.
//!
//! Phase-3 of the Neovim live-edit latency lane (#3396) — the **off-lock
//! async parse worker**. Phase-2 (#3396 lane PR 2) measured the full
//! edit-to-answer path on the *synchronous* `didChange` hot path (one full
//! parse per ranged edit, inline, before the handler returned). This
//! receipt now installs the real off-lock parse worker for the AFTER
//! scenario and proves the headline Phase-3 claim on the actual production
//! wiring, not a synthetic stand-in:
//!
//! - **AFTER** (`incremental_eager = false`, the default since #3412, now
//!   WITH the async parse worker installed): `didChange` performs NO parse
//!   and NO parent-map build before returning -- it applies the text edit,
//!   bumps the generation, and enqueues a coalescing parse job. The worker
//!   parses off-lock and publishes only the final, freshness-current
//!   generation.
//! - **BEFORE** (`incremental_eager = true`): unchanged from Phase-2 --
//!   reproduces the pre-#3412 cost model (eager `incremental_doc_update`
//!   maintenance), which still requires the parse to run synchronously
//!   under the mutation lock (see `LspServer::incremental_eager_enabled`'s
//!   doc comment in `runtime/text_sync.rs`) and so is NOT eligible for the
//!   async worker path regardless of whether one is installed.
//!
//! Both scenarios drive ~20 **ranged** edits against a realistic ~78 KB file
//! (not a full-document replacement) and then issue completion, hover,
//! semantic-tokens, and references requests against the post-edit document,
//! recording whether each returned and its first-response wall-time.
//!
//! CI asserts SHAPE only (a receipt is emitted per scenario, every provider
//! returns). It never asserts hard latency budgets — the millisecond
//! timings are informational and hardware-dependent (see #1373). For the
//! AFTER (async) scenario, shape assertions cover the Phase-3 closure
//! claim: `didChange.full_parse`/`didChange.parent_map` are never emitted on
//! the mutation path, the worker starts at most one job per edit, exactly
//! one (the final) generation publishes, and at least one job was
//! discarded-or-coalesced from the burst. For the BEFORE (sync) scenario,
//! shape assertions preserve the Phase-2 invariant: one synchronous full
//! parse per ranged edit.
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
use std::sync::Arc;
use std::time::{Duration, Instant};

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
    total_span_count: usize,
    /// Whether this scenario ran on the off-lock async parse worker path
    /// (AFTER) or the synchronous fallback (BEFORE, `incremental_eager`).
    is_async: bool,
    /// Count of `didChange.full_parse` spans -- must be 0 on the async
    /// path (that work no longer happens in the mutation handler), and
    /// equal to `ranged_edits` on the synchronous path.
    did_change_full_parse_count: usize,
    /// Count of `didChange.parent_map` spans -- same shape as above.
    did_change_parent_map_count: usize,
    full_parse_max_ms: f64,
    /// Worker metrics, present only for the async scenario.
    worker_jobs_started: Option<u64>,
    worker_jobs_published: Option<u64>,
    worker_jobs_discarded_or_coalesced: Option<u64>,
}

/// Run the ~20-ranged-edit scenario against a fresh server, then measure
/// first-response latency for completion, hover, semantic tokens, and
/// references on the post-edit document. Emits one
/// `PERL_LSP_TIMING_RECEIPT` labeled with `label`.
///
/// `incremental_eager` selects the AFTER (`false`, current default since
/// #3412) or BEFORE (`true`, pre-#3412 cost model) `didChange` path. The
/// off-lock async parse worker (#3396 Phase 3) is installed whenever
/// `!incremental_eager`, matching production
/// (`LspServer::install_default_parse_worker`'s eligibility rule in
/// `runtime/text_sync.rs`) -- the BEFORE scenario intentionally stays on
/// the synchronous fallback since eager incremental maintenance requires
/// its own parse under the same lock as the text-state update.
fn run_ranged_edit_scenario(label: &str, incremental_eager: bool) -> TestResult<ScenarioShape> {
    let server = Arc::new(LspServer::new());
    #[cfg(feature = "incremental")]
    server.set_incremental_eager(incremental_eager);

    let is_async = !incremental_eager;
    if is_async {
        server.test_install_parse_worker();
        assert!(
            server.test_parse_worker_installed(),
            "{label}: parse worker must be installed for the async scenario"
        );
    }

    let source = medium_fixture();
    let file_bytes = source.len();

    // Open the medium file (version 1). didOpen is unaffected by Phase 3
    // (always synchronous), so this always completes with a fresh AST.
    server.test_apply_did_open(URI, &source, 1)?;

    let ranged_edits: usize = 20;

    // Capture the internal PERL_LSP_TIMING spans for the receipt breakdown.
    // This is independent of the env sink, so it does not race on env state.
    // The capture buffer is process-global, so spans emitted from the parse
    // worker's pool threads (a different OS thread than this test) are
    // captured too -- see `runtime::timing::capture`.
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

    // On the async path, `didChange` returns after enqueueing -- wait for
    // the worker to settle (publish the final generation + run its side
    // effects) before querying providers, otherwise completion/hover/etc.
    // could observe the mid-burst pending-parse gap and answer degraded
    // (see `pending_parse_provider_freshness_tests.rs`), which would make
    // this receipt's "every provider returns" shape assertions meaningless.
    if is_async {
        let settled = server.test_wait_for_parse_worker_settled(URI, Duration::from_secs(30));
        assert!(settled, "{label}: parse worker must settle within the timeout after the burst");
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

    // Drain the captured spans. By this point the async worker has already
    // settled (waited for above), so every span its pool threads emitted
    // for this burst is already in the buffer.
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

    let did_change_full_parse_count = count("didChange.full_parse");
    let did_change_parent_map_count = count("didChange.parent_map");
    let full_parse_max_ms = max_ms("didChange.full_parse");
    let total_span_count = spans.len();

    let worker_metrics = server.test_parse_worker_metrics();
    let worker_jobs_started = worker_metrics.map(|m| m.jobs_started);
    let worker_jobs_published = worker_metrics.map(|m| m.jobs_published);
    let worker_jobs_discarded_or_coalesced =
        worker_metrics.map(|m| m.jobs_coalesced + m.jobs_rejected_stale);

    let receipt = json!({
        "receipt": "ux_neovim_ranged_typing_medium_file_receipt",
        "label": label,
        "incremental_eager": incremental_eager,
        "async_parse_worker": is_async,
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
        "did_change_full_parse_count": did_change_full_parse_count,
        "did_change_parent_map_count": did_change_parent_map_count,
        "worker_jobs_started": worker_jobs_started,
        "worker_jobs_published": worker_jobs_published,
        "worker_jobs_discarded_or_coalesced": worker_jobs_discarded_or_coalesced,
        "notes": concat!(
            "Phase-3 receipt (#3396): the AFTER scenario installs the real off-lock ",
            "async parse worker and proves didChange no longer parses inline. Timings are ",
            "informational and hardware-dependent; CI asserts SHAPE only. ",
            "did_change_handler_* are external wall-times around test_handle_did_change -- ",
            "on the async scenario these collapse toward the text-apply-only cost, since no ",
            "parse happens before the handler returns. internal_spans_ms come from the ",
            "PERL_LSP_TIMING probes. *_first_response_ms are external wall-times around each ",
            "provider call issued after the burst has settled. incremental_doc_update_max is ",
            "~0 on the async (eager-off) scenario and non-trivial on the sync (eager-on) ",
            "scenario -- compare the two receipts for the delta. worker_jobs_* fields are only ",
            "populated for the async scenario."
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
        total_span_count,
        is_async,
        did_change_full_parse_count,
        did_change_parent_map_count,
        full_parse_max_ms,
        worker_jobs_started,
        worker_jobs_published,
        worker_jobs_discarded_or_coalesced,
    })
}

#[test]
fn ux_neovim_ranged_typing_medium_file_receipt() -> TestResult {
    // AFTER: incremental_eager off — the current production default since
    // #3412, now with the real off-lock async parse worker installed. This
    // is the durable current-main latency artifact for the lane, and the
    // scenario the Phase-3 closure claim ("didChange no longer parses
    // inline") is proven against.
    let after = run_ranged_edit_scenario("after_async_parse_worker_phase3", false)?;

    // BEFORE: incremental_eager on — reproduces the pre-#3412 cost model
    // (eager incremental_doc_update maintenance on every keystroke), which
    // still requires the synchronous fallback path regardless of the
    // worker being installed.
    let before = run_ranged_edit_scenario("before_eager_on_pre_3412_baseline", true)?;

    // ---- SHAPE assertions common to both scenarios ----
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
    }

    // ---- AFTER (async): the Phase-3 closure claim ----
    assert!(after.is_async);
    assert_eq!(
        after.did_change_full_parse_count, 0,
        "after: didChange must perform NO full parse before returning on the async path"
    );
    assert_eq!(
        after.did_change_parent_map_count, 0,
        "after: didChange must perform NO parent-map build before returning on the async path"
    );
    let worker_started = after.worker_jobs_started.ok_or("after: worker_jobs_started missing")?;
    let worker_published =
        after.worker_jobs_published.ok_or("after: worker_jobs_published missing")?;
    let worker_discarded_or_coalesced = after
        .worker_jobs_discarded_or_coalesced
        .ok_or("after: worker_jobs_discarded_or_coalesced missing")?;
    assert!(
        worker_started <= after.ranged_edits as u64,
        "after: coalescing must start no more jobs than edits enqueued; started={worker_started}"
    );
    assert_eq!(
        worker_published, 1,
        "after: exactly one (the final) generation from the burst must publish"
    );
    assert!(
        worker_discarded_or_coalesced > 0,
        "after: at least one job from the 20-edit burst must be discarded or coalesced"
    );

    // ---- BEFORE (sync fallback): the Phase-2 invariant, unchanged ----
    assert!(!before.is_async);
    assert_eq!(
        before.did_change_full_parse_count, before.ranged_edits,
        "before: each ranged edit should trigger exactly one synchronous full parse"
    );
    assert!(
        before.full_parse_max_ms > 0.0,
        "before: the full_parse span must record a real (non-zero) duration"
    );

    Ok(())
}
