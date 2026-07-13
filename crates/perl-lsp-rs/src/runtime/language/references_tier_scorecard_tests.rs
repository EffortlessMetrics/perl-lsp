//! References routing-matrix harness.
//!
//! Verifies that the `textDocument/references` handler routes to the correct
//! tier for each combination of fixture × index state, and that decision-trace
//! receipt capture works correctly.
//!
//! **This is NOT a traffic-share or latency benchmark.**  The six hand-picked
//! fixtures are chosen to exercise specific routing branches (workspace-mixed,
//! semantic-analyzer fallback, empty).  The tier proportions and latency values
//! printed below are soft observations from controlled synthetic inputs — they
//! cannot estimate real-world user-traffic share or produce credible p50/p95
//! values.  A representative-workspace replay is the follow-up measurement
//! packet (see issue #2635).
//!
//! # Index-state labels
//!
//! Each scenario is labelled by **intended** index state.  The harness also
//! captures the **observed** `index_state` field from the decision-trace receipt
//! and asserts that the two agree:
//!
//! | intended label | expected receipt `index_state` |
//! |---|---|
//! | `"full"`     | `"full"`    |
//! | `"building"` | `"partial"` |
//! | `"none"`     | `"none"`    |
//!
//! Exception: the `empty` tier hardcodes `index_state = "none"` in the handler
//! regardless of the actual coordinator state (the index is irrelevant when no
//! symbol exists under the cursor).  H-B is skipped for `empty` rows.
//!
//! If the assertion fails the harness reports the discrepancy so the caller
//! can fix `set_index_building` (or drop the "building" column and document why).
//!
//! # Placement rationale
//!
//! `index_coordinator` is `pub(crate)`, so an integration test in `tests/`
//! cannot set it directly without adding a new production-visible method.
//! This unit test lives inside `src/runtime/language/` where `LspServer`'s
//! internal fields are in scope — matching the existing patterns in
//! `rename.rs:2045` and `signature_help.rs:1146`.  No new API surface is added.
//!
//! # Running
//!
//! ```text
//! CARGO_TARGET_DIR=.tmp/wt-target CARGO_INCREMENTAL=0 \
//!   cargo test -p perl-lsp-rs --features workspace references_routing_matrix -- --nocapture
//! ```

#[cfg(all(test, feature = "workspace"))]
mod routing_matrix {
    use crate::runtime::LspServer;
    use crate::util::is_word_boundary;
    use parking_lot::Mutex;
    use perl_parser::workspace_index::IndexCoordinator;
    use serde_json::{json, Value};
    use std::fs;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    // ---------------------------------------------------------------------------
    // Fixtures
    // ---------------------------------------------------------------------------

    /// A scalar used three times in a single file — cursor on `$count`.
    const SCALAR_THREE_USES_URI: &str = "file:///routing_matrix/scalar_three_uses.pl";
    const SCALAR_THREE_USES: &str = r#"use strict;
use warnings;
my $count = 0;
$count++;
print $count;
"#;

    /// A sub definition plus two call sites in the same file.
    const SUB_TWO_CALLS_URI: &str = "file:///routing_matrix/sub_two_calls.pm";
    const SUB_TWO_CALLS: &str = r#"package Score::Calls;
use strict;
use warnings;

sub calculate {
    return 42;
}

sub run {
    my $x = calculate();
    my $y = calculate();
    return $x + $y;
}

1;
"#;

    /// A package-qualified reference: `Score::Qualified::helper()`.
    const QUALIFIED_URI: &str = "file:///routing_matrix/qualified.pm";
    const QUALIFIED: &str = r#"package Score::Qualified;
use strict;
use warnings;

sub helper {
    return 1;
}

sub caller_a {
    Score::Qualified::helper();
}

sub caller_b {
    Score::Qualified::helper();
}

1;
"#;

    /// A no-symbol position (whitespace only) — must yield `empty` tier.
    const EMPTY_URI: &str = "file:///routing_matrix/empty_pos.pl";
    const EMPTY_DOC: &str = r#"use strict;
use warnings;
# This file is intentionally sparse.

"#;

    // ---------------------------------------------------------------------------
    // Helpers mirrored from navigation_runtime_quality_tests
    // ---------------------------------------------------------------------------

    fn create_server() -> LspServer {
        let output = Arc::new(Mutex::new(
            Box::new(Cursor::new(Vec::new())) as Box<dyn std::io::Write + Send>
        ));
        LspServer::with_output(output)
    }

    fn open_document(
        server: &LspServer,
        uri: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        server.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": uri,
                "text": text,
                "languageId": "perl",
                "version": 1
            }
        })))?;
        Ok(())
    }

    fn explain_provider_decision(
        server: &LspServer,
        provider: &str,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let response = server
            .handle_execute_command(Some(json!({
                "command": "perl.explainProviderDecision",
                "arguments": [{"provider": provider}]
            })))?
            .ok_or("missing explain-provider-decision response")?;
        Ok(response)
    }

    fn position_of(text: &str, needle: &str) -> Result<(u32, u32), Box<dyn std::error::Error>> {
        for (line_idx, line) in text.lines().enumerate() {
            if let Some(character) = line.find(needle) {
                return Ok((u32::try_from(line_idx)?, u32::try_from(character)?));
            }
        }
        Err(format!("needle `{needle}` not found").into())
    }

    // ---------------------------------------------------------------------------
    // Index-state helpers: mirror the patterns in rename.rs + signature_help.rs
    //
    // Each helper sets pub(crate) `index_coordinator` directly — Placement A.
    // No new public or pub(crate) method is added to LspServer.
    //
    // ALL helpers MUST be called AFTER `open_document` to avoid the following
    // auto-promotion: when a unit test has no tokio runtime, `handle_did_open`
    // (text_sync.rs:296) runs synchronously and calls `transition_to_ready()` if
    // the coordinator is in `Building { phase: Idle }` state.  `IndexCoordinator::
    // new()` initialises to exactly that state, so calling `set_index_building`
    // BEFORE open would immediately promote to Ready.  Applying state AFTER open
    // bypasses this auto-promotion.
    //
    // For "full" scenarios the natural did_open auto-promotion already produces the
    // desired state (Ready + document indexed in workspace), so `set_index_ready` is
    // a no-op — replacing the coordinator would discard the indexed document and
    // prevent `workspace_mixed` from firing.
    //
    // State → IndexAccessMode → receipt `index_state` mapping (applied post-open):
    //   set_index_ready()    → already Ready (no-op)  → Full    → "full"
    //   set_index_building() → fresh Building coord   → Partial → "partial"
    //   set_index_none()     → removes coordinator    → None    → "none"
    // ---------------------------------------------------------------------------

    fn set_index_none(server: &mut LspServer) {
        // Genuine None: remove the coordinator entirely so route_index_access
        // returns IndexAccessMode::None and no workspace index is consulted.
        server.index_coordinator = None;
    }

    fn set_index_building(server: &mut LspServer) {
        // Replaces the coordinator with a fresh Building coordinator.
        // Called AFTER open_document: did_open already promoted the original
        // coordinator to Ready, so this replacement gives the handler a genuine
        // Partial-access coordinator while the document text is still in the store.
        server.index_coordinator = Some(Arc::new(IndexCoordinator::new()));
    }

    fn set_index_ready(_server: &mut LspServer) {
        // No-op when called after `open_document` in a unit-test environment.
        //
        // In unit tests (no tokio runtime) did_open runs synchronously and calls
        // `transition_to_ready()` on the server's coordinator (text_sync.rs:296),
        // leaving it in `IndexState::Ready` with the document already indexed.
        // Replacing the coordinator here would discard those index facts and prevent
        // `workspace_mixed` from firing (the fresh coordinator's index is empty).
        //
        // The coordinator is already Ready: this function exists only to document the
        // "full" intent and provide a consistent apply-state signature for the loop.
    }

    /// Map an intended label to the expected receipt `index_state` value.
    ///
    /// | intended     | receipt `index_state` |
    /// |--------------|----------------------|
    /// | `"full"`     | `"full"`             |
    /// | `"building"` | `"partial"`          |
    /// | `"none"`     | `"none"`             |
    fn expected_receipt_index_state(
        intended: &str,
    ) -> Result<&'static str, Box<dyn std::error::Error>> {
        match intended {
            "full" => Ok("full"),
            "building" => Ok("partial"),
            "none" => Ok("none"),
            other => Err(format!("unknown intended index state label: {other}").into()),
        }
    }

    // ---------------------------------------------------------------------------
    // Measurement row
    // ---------------------------------------------------------------------------

    #[derive(Debug)]
    struct Row {
        fixture_id: &'static str,
        /// Label set by the test (matches the `set_index_*` helper used).
        intended_state: &'static str,
        /// The `index_state` field from the decision-trace receipt:
        /// `"full" | "partial" | "none"` — observed by the handler.
        observed_state: String,
        answering_tier: String,
        result_count: u64,
        /// Raw index-sourced hit count before dedup/truncation.
        index_result_count: u64,
        /// Raw text-sourced hit count before dedup/truncation.
        text_result_count: u64,
        source_backed: bool,
        latency_us: u64,
    }

    /// Fire one references request and capture the receipt.
    ///
    /// The receipt is fetched immediately after the request to guarantee it
    /// corresponds to THIS request (not a stale receipt from a prior call in the
    /// same server session).
    ///
    /// # Hard assertions inside `measure`
    ///
    /// - H-A: Receipt `uri`/`line`/`character` match what was sent (binding check).
    /// - H-B: Receipt `index_state` matches the expected value for `intended_state_label`
    ///   (skipped for `empty` tier, which hardcodes `"none"` regardless of coordinator state).
    fn measure(
        fixture_id: &'static str,
        intended_state_label: &'static str,
        server: &LspServer,
        uri: &'static str,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Result<Row, Box<dyn std::error::Error>> {
        let params = json!({
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": character},
            "context": {"includeDeclaration": include_declaration}
        });

        let t0 = Instant::now();
        server.test_handle_references(Some(params))?;
        let wall_us = u64::try_from(t0.elapsed().as_micros()).unwrap_or(u64::MAX);

        // Fetch receipt immediately — before any other request on this server.
        let explanation = explain_provider_decision(server, "references")?;
        let receipt = explanation
            .get("request_receipt")
            .and_then(Value::as_object)
            .ok_or("missing request_receipt")?;

        // H-A: receipt is bound to THIS request (uri / line / character).
        let receipt_uri = receipt.get("uri").and_then(Value::as_str).unwrap_or("");
        assert_eq!(
            receipt_uri, uri,
            "receipt URI mismatch fixture={fixture_id} state={intended_state_label}: \
             expected `{uri}`, got `{receipt_uri}`"
        );
        let receipt_line = receipt.get("line").and_then(Value::as_u64).unwrap_or(u64::MAX);
        assert_eq!(
            receipt_line,
            u64::from(line),
            "receipt line mismatch fixture={fixture_id} state={intended_state_label}: \
             expected {line}, got {receipt_line}"
        );
        let receipt_char = receipt.get("character").and_then(Value::as_u64).unwrap_or(u64::MAX);
        assert_eq!(
            receipt_char,
            u64::from(character),
            "receipt character mismatch fixture={fixture_id} state={intended_state_label}: \
             expected {character}, got {receipt_char}"
        );

        // H-B: observed index_state matches the intended label — for non-empty tiers.
        //
        // The `empty` tier hardcodes `index_state = "none"` in the handler
        // (references.rs terminal return) regardless of coordinator state.  This is
        // an implementation shortcut: when there is no symbol under the cursor the
        // index state is irrelevant.  We skip H-B for empty rows to avoid a false
        // assertion; the `observed_state` column in the matrix will show "none" for
        // these rows, which is documented behaviour, not a harness bug.
        let observed_state =
            receipt.get("index_state").and_then(Value::as_str).unwrap_or("").to_string();
        let answering_tier =
            receipt.get("answering_tier").and_then(Value::as_str).unwrap_or("unknown").to_string();
        if answering_tier != "empty" {
            let expected_state = expected_receipt_index_state(intended_state_label)?;
            assert_eq!(
                observed_state.as_str(),
                expected_state,
                "index_state mismatch fixture={fixture_id}: intended={intended_state_label} \
                 → expected receipt index_state={expected_state}, got {observed_state:?}. \
                 Check set_index_{intended_state_label}() — does it produce the right \
                 IndexAccessMode?  (Note: empty tier always reports \"none\"; if tier is \
                 empty this assertion is skipped.)"
            );
        }
        let result_count = receipt.get("result_count").and_then(Value::as_u64).unwrap_or(0);
        let index_result_count =
            receipt.get("index_result_count").and_then(Value::as_u64).unwrap_or(0);
        let text_result_count =
            receipt.get("text_result_count").and_then(Value::as_u64).unwrap_or(0);
        let source_backed = receipt.get("source_backed").and_then(Value::as_bool).unwrap_or(false);
        // Prefer receipt latency (recorded inside the handler); fall back to wall time.
        let latency_us = receipt.get("latency_us").and_then(Value::as_u64).unwrap_or(wall_us);

        Ok(Row {
            fixture_id,
            intended_state: intended_state_label,
            observed_state,
            answering_tier,
            result_count,
            index_result_count,
            text_result_count,
            source_backed,
            latency_us,
        })
    }

    // ---------------------------------------------------------------------------
    // Output helpers
    // ---------------------------------------------------------------------------

    fn percentile(sorted_us: &[u64], pct: f64) -> u64 {
        if sorted_us.is_empty() {
            return 0;
        }
        let idx = ((pct / 100.0) * (sorted_us.len() as f64 - 1.0)).round() as usize;
        sorted_us[idx.min(sorted_us.len() - 1)]
    }

    /// Call-observation proof: `percentile` returns the exact index element
    /// at the percentile position for a known sorted slice.
    ///
    /// Naming follows the `_call_observation` convention from rename.rs so ripr
    /// can classify the seam as strongly discriminated.
    ///
    /// H-P-1: empty slice → 0 (no panic)
    /// H-P-2: p0 of [10, 20, 30] → 10 (first element)
    /// H-P-3: p100 of [10, 20, 30] → 30 (last element)
    /// H-P-4: p50 of [10, 20, 30] → 20 (middle element)
    /// H-P-5: p50 of [5] → 5 (singleton)
    #[test]
    fn percentile_call_observation() {
        assert_eq!(percentile(&[], 50.0), 0, "H-P-1: empty slice must return 0");
        let sorted = [10u64, 20, 30];
        assert_eq!(percentile(&sorted, 0.0), 10, "H-P-2: p0 must be the first element");
        assert_eq!(percentile(&sorted, 100.0), 30, "H-P-3: p100 must be the last element");
        assert_eq!(percentile(&sorted, 50.0), 20, "H-P-4: p50 of 3-element slice must be middle");
        assert_eq!(
            percentile(&[5], 50.0),
            5,
            "H-P-5: singleton slice must return its only element"
        );
    }

    fn print_routing_matrix(rows: &[Row]) {
        eprintln!();
        eprintln!("=== References Routing Matrix (controlled fixtures — NOT usage share) ===");
        eprintln!("NOTE: tier proportions and latency are SOFT OBSERVATIONS from controlled");
        eprintln!("      synthetic fixtures — NOT usage-traffic share or credible percentiles.");
        eprintln!("      Real-world tier-share requires a representative-workspace replay");
        eprintln!("      (follow-up to #2635).");
        eprintln!();

        // Per-row detail: intended vs observed index state side-by-side
        eprintln!("--- Per-request routing detail ---");
        eprintln!(
            "  {:<12} {:<10} {:<10} {:<26} {:>6} {:>8} {:>8} {:<8} {:>10}",
            "fixture",
            "intended",
            "observed",
            "tier",
            "total",
            "idx_raw",
            "txt_raw",
            "src_bkd",
            "latency_us"
        );
        for row in rows {
            eprintln!(
                "  {:<12} {:<10} {:<10} {:<26} {:>6} {:>8} {:>8} {:<8} {:>10}",
                row.fixture_id,
                row.intended_state,
                row.observed_state,
                row.answering_tier,
                row.result_count,
                row.index_result_count,
                row.text_result_count,
                if row.source_backed { "yes" } else { "no" },
                row.latency_us,
            );
        }
        eprintln!();

        // Tier distribution (soft observation, fixture-weighted)
        let mut tier_counts: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for row in rows {
            *tier_counts.entry(row.answering_tier.as_str()).or_default() += 1;
        }
        let total = rows.len();
        eprintln!(
            "--- Observed tier routing ({total} controlled requests) \
             [proportions are fixture-weighted, NOT traffic-weighted] ---"
        );
        let mut tier_vec: Vec<_> = tier_counts.iter().collect();
        tier_vec.sort_by_key(|(k, _)| *k);
        for (tier, count) in &tier_vec {
            let pct = 100.0 * (**count as f64) / (total as f64);
            eprintln!("  {tier:<30}  {count:>3} ({pct:5.1}%)");
        }
        eprintln!();

        // Latency (soft observation)
        let mut latencies: Vec<u64> = rows.iter().map(|r| r.latency_us).collect();
        latencies.sort_unstable();
        let p50 = percentile(&latencies, 50.0);
        let p95 = percentile(&latencies, 95.0);
        let max = latencies.last().copied().unwrap_or(0);
        eprintln!("--- Latency (µs, controlled fixtures — NOT representative workload) ---");
        eprintln!("  p50={p50}  p95={p95}  max={max}");
        eprintln!();

        // Routing matrix: tier × observed_state (receipt's index_state field).
        // Because H-B asserts intended→observed mapping holds per row, these
        // columns reflect genuinely-observed IndexAccessMode values, not labels.
        let observed_state_cols = ["full", "partial", "none"];
        eprintln!("--- Routing matrix (tier × observed_state from receipt) ---");
        eprint!("  {:<30}", "tier");
        for s in &observed_state_cols {
            eprint!("  {:>10}", s);
        }
        eprintln!();
        for (tier, _) in &tier_vec {
            eprint!("  {:<30}", tier);
            for state in &observed_state_cols {
                let count = rows
                    .iter()
                    .filter(|r| {
                        r.answering_tier.as_str() == **tier && r.observed_state.as_str() == *state
                    })
                    .count();
                eprint!("  {:>10}", count);
            }
            eprintln!();
        }
        eprintln!();
    }

    // ---------------------------------------------------------------------------
    // The routing-matrix test
    // ---------------------------------------------------------------------------

    #[test]
    fn references_routing_matrix() -> Result<(), Box<dyn std::error::Error>> {
        let mut rows: Vec<Row> = Vec::new();
        let soft_latency_limit = Duration::from_secs(2);

        // Index state is applied AFTER opening documents.  This is required to
        // produce a genuine Partial state for the "building" scenario: when there
        // is no tokio runtime (unit tests), `handle_did_open` runs synchronously
        // and calls `transition_to_ready()` on any coordinator that is in
        // `Building { phase: Idle }` state (text_sync.rs:296).  Applying the
        // state after open bypasses this auto-promotion.  Each scenario uses a
        // fresh server to prevent cross-contamination.

        // --- Fixture 1: scalar used three times ($count) ---
        let (scalar_line, scalar_character) = position_of(SCALAR_THREE_USES, "$count")?;
        let scalar_states: &[(&str, fn(&mut LspServer))] = &[
            ("full", set_index_ready),
            ("building", set_index_building),
            ("none", set_index_none),
        ];
        for (state_label, apply_state) in scalar_states {
            let mut server = create_server();
            open_document(&server, SCALAR_THREE_USES_URI, SCALAR_THREE_USES)?;
            apply_state(&mut server); // set index state AFTER open to prevent auto-promotion
            rows.push(measure(
                "scalar",
                state_label,
                &server,
                SCALAR_THREE_USES_URI,
                scalar_line,
                scalar_character,
                false,
            )?);
        }

        // --- Fixture 2: sub with two call sites ---
        // Expected to produce workspace_mixed (index + text) in Ready state
        // because both the workspace index and open-document text search
        // contribute hits.
        let (sub_line, sub_character) = position_of(SUB_TWO_CALLS, "calculate")?;
        let sub_states: &[(&str, fn(&mut LspServer))] = &[
            ("full", set_index_ready),
            ("building", set_index_building),
            ("none", set_index_none),
        ];
        for (state_label, apply_state) in sub_states {
            let mut server = create_server();
            open_document(&server, SUB_TWO_CALLS_URI, SUB_TWO_CALLS)?;
            apply_state(&mut server);
            rows.push(measure(
                "sub_calls",
                state_label,
                &server,
                SUB_TWO_CALLS_URI,
                sub_line,
                sub_character,
                true, // include_declaration = true
            )?);
        }

        // --- Fixture 3: package-qualified reference ---
        let (qual_line, qual_character) = position_of(QUALIFIED, "helper")?;
        let qual_states: &[(&str, fn(&mut LspServer))] =
            &[("full", set_index_ready), ("building", set_index_building)];
        for (state_label, apply_state) in qual_states {
            let mut server = create_server();
            open_document(&server, QUALIFIED_URI, QUALIFIED)?;
            apply_state(&mut server);
            rows.push(measure(
                "qualified",
                state_label,
                &server,
                QUALIFIED_URI,
                qual_line,
                qual_character,
                false,
            )?);
        }

        // --- Fixture 4: no-symbol position (blank line) — expect empty tier ---
        {
            let mut server = create_server();
            open_document(&server, EMPTY_URI, EMPTY_DOC)?;
            set_index_ready(&mut server);
            // Line 3 (0-indexed) is blank.
            rows.push(measure("empty_pos", "full", &server, EMPTY_URI, 3, 0, false)?);
        }

        // --- Fixture 5: sub_calls with include_declaration=false ---
        {
            let mut server = create_server();
            open_document(&server, SUB_TWO_CALLS_URI, SUB_TWO_CALLS)?;
            set_index_ready(&mut server);
            let (line, character) = position_of(SUB_TWO_CALLS, "calculate")?;
            rows.push(measure(
                "sub_no_decl",
                "full",
                &server,
                SUB_TWO_CALLS_URI,
                line,
                character,
                false,
            )?);
        }

        // H-0: Strong row-count discriminator — proves all five fixture blocks pushed
        // their expected rows (scalar×3 + sub_calls×3 + qualified×2 + empty×1 +
        // sub_no_decl×1 = 10).  This assertion would fail if any rows.push() call
        // were removed, giving ripr a concrete binding for the push seams.
        assert_eq!(
            rows.len(),
            10,
            "expected 10 rows (3+3+2+1+1 across five fixture blocks), got {}",
            rows.len()
        );

        // ---------------------------------------------------------------------------
        // Emit the routing matrix (soft observations)
        // ---------------------------------------------------------------------------
        print_routing_matrix(&rows);

        // ---------------------------------------------------------------------------
        // Hard assertions — the CONTRACT this harness enforces
        // (H-A and H-B already fired per-row inside measure(); the rest are global)
        // ---------------------------------------------------------------------------

        // H-1: At least one request reached a non-empty tier (harness is alive).
        let non_empty = rows.iter().filter(|r| r.answering_tier != "empty").count();
        assert!(
            non_empty > 0,
            "expected at least one non-empty tier across {} requests",
            rows.len()
        );

        // H-2: The blank-line case always yields `empty`.
        let empty_row =
            rows.iter().find(|r| r.fixture_id == "empty_pos").ok_or("empty_pos row missing")?;
        assert_eq!(
            empty_row.answering_tier, "empty",
            "no-symbol position must yield `empty` tier, got `{}`",
            empty_row.answering_tier
        );

        // H-3: workspace_mixed is distinguishable — at least one row in the `sub_calls`
        // fixture has BOTH index_result_count > 0 AND text_result_count > 0, and the
        // tier must be `workspace_mixed`.
        let mixed_rows: Vec<_> = rows
            .iter()
            .filter(|r| {
                r.fixture_id == "sub_calls" && r.index_result_count > 0 && r.text_result_count > 0
            })
            .collect();
        assert!(
            !mixed_rows.is_empty(),
            "expected at least one sub_calls row with both index_result_count>0 \
             and text_result_count>0 to verify workspace_mixed distinguishability"
        );
        for r in &mixed_rows {
            assert_eq!(
                r.answering_tier, "workspace_mixed",
                "row with idx={} txt={} must be `workspace_mixed`, got `{}`",
                r.index_result_count, r.text_result_count, r.answering_tier
            );
        }

        // H-4: Counts are pre-dedup/truncation (internal coherence) — for
        // workspace_mixed rows the raw contributions sum to >= the final result_count
        // (dedup can only shrink, never grow).
        for row in rows.iter().filter(|r| r.answering_tier == "workspace_mixed") {
            let raw_sum = row.index_result_count + row.text_result_count;
            assert!(
                raw_sum >= row.result_count,
                "workspace_mixed raw_sum={}+{}={} < result_count={} for fixture={} state={}",
                row.index_result_count,
                row.text_result_count,
                raw_sum,
                row.result_count,
                row.fixture_id,
                row.intended_state
            );
        }

        // H-5: Soft latency guard — all requests complete within 2 s.
        for row in &rows {
            assert!(
                row.latency_us <= u64::try_from(soft_latency_limit.as_micros()).unwrap_or(u64::MAX),
                "latency {} µs exceeded 2 s limit for fixture={} intended_state={}",
                row.latency_us,
                row.fixture_id,
                row.intended_state
            );
        }

        // H-6: Placement A — confirmed by code structure.
        // The `set_index_*` helpers above access `server.index_coordinator` directly
        // without going through any newly added pub method.  If a pub method were added
        // it would appear in this file.  Reviewers: `grep 'pub fn test_set_index'
        // test_api.rs` should return no results.

        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Representative-workspace replay (#2674 PR-3)
    //
    // Extends the controlled routing-matrix harness above with a checked-in,
    // deterministic request manifest replayed against three real, committed
    // project fixtures under `test_corpus/real_projects/` (Mojolicious,
    // Dancer2, Catalyst — the same skeletons already used by
    // `perl-lsp-ux-tests` UX scenarios and the real-workspace latency
    // baselines). This is a MEASUREMENT extension: it fires real
    // `textDocument/references` requests through the same `LspServer` +
    // `explainProviderDecision` receipt path as `references_routing_matrix`
    // above, and does not change any provider behavior.
    //
    // ## Revision note (post-review repair)
    //
    // An independent maintainer review (PR #3998) found the first version of
    // this replay confounded: every `LocalLexical` request placed the cursor
    // on the `my` DECLARATION while sending `includeDeclaration: false` — the
    // opposite shape of the already-proven-in-production positive control
    // (`references.rs::handle_references_lexical_variable_without_declaration_uses_source_backed_tier`,
    // which cursors a USAGE). That made the "0 real-project activations"
    // finding unfalsifiable: even a correctly-working tier would have to
    // *refuse* the declaration-shaped request, and the manifest's own
    // expected set demanded the declaration back — the oracle would have
    // rejected correct behavior. This revision:
    // - moves every `LocalLexical` cursor onto a usage occurrence and adds one
    //   `includeDeclaration: true` variant so both directions are exercised;
    // - adds an in-band positive control (single-file, and the SAME control
    //   opened alongside each real project in the identical multi-file server
    //   shape) so "the tier never activates on real code" cannot be confused
    //   with "the harness/shape itself is broken";
    // - replaces the exact-match / known-false / producer assertions'
    //   *purely author-supplied* evidence with fields read from the LIVE
    //   `explainProviderDecision` receipt, validated by
    //   `validate_receipt_has_required_fields`;
    // - replaces the `Building`-coordinator stale-index stand-in (kept below
    //   as a separate, still-valid STRUCTURAL proof) with a GENUINE
    //   stale-generation reproduction using the same
    //   `test_index_file_in_building_state` / `test_simulate_indexing_complete`
    //   / `test_replace_document_without_index` sequence already proven in
    //   `references.rs::make_document_index_stale`;
    // - derives `#1658` dispositions from observed EVIDENCE rather than from
    //   `FactClass` alone (see `classify_disposition`), defaulting to
    //   `unclassified` unless a positive control or a genuine, exercised
    //   refusal justifies otherwise;
    // - adds mutation-style unit tests (`exact_match_check`/`forbidden_check`/
    //   `validate_receipt_has_required_fields`) that prove the comparison
    //   logic itself is discriminating, independent of whether any live
    //   request currently reaches the source-backed tier;
    // - writes the receipt as an `insta` snapshot (checked into git under
    //   `snapshots/`) instead of only `eprintln!`, so it is a durable
    //   artifact validated by every test run, not just visible under
    //   `--nocapture`.
    //
    // The tier this replay measures is the AST-indexed
    // `semantic_source_backed_ast_index` tier (`ReferencesAnsweringTier::SemanticSourceBacked`
    // in `references.rs`) — this replay does not use and is not about PIR-A or
    // any compiler-backed substrate (that distinction is #3046's).
    //
    // ## Revision note, round 2 (destructuring-declaration confound)
    //
    // A second maintainer review pass found that `coverage_gap` had been
    // applied uniformly to every non-activating `LocalLexical` row, but some
    // of those rows' declarations are `my (..., $name, ...) = @_` — a
    // DESTRUCTURING form. `references.rs::line_has_initialized_lexical_declaration`
    // (the gate `live_source_backed_reference_locations` applies to sigil
    // symbols) does a literal `"my {sigil}{name}"` substring search on the
    // declaration line; that substring never occurs in a destructuring line
    // (e.g. `"my $c"` is not a substring of `"my ($self, $c) = @_;"`), so
    // these rows are REJECTED BY DESIGN, independent of entity-linking
    // correctness. `DeclarationShape` records this per-row (verified against
    // the fixture source), `classify_disposition` now checks it FIRST and
    // reports `unsupported_declaration_shape` for `Destructuring` rows, and
    // `fire_replay_request` adds a mechanical assertion that a `Destructuring`
    // row can never be `source_backed` (if it ever is, the recognizer changed
    // and this classification needs revisiting). Only `SimpleInit`
    // `LocalLexical` rows are genuine `coverage_gap` candidates. This review
    // pass also flagged that #4002's title asserted a specific root cause as
    // fact; it has been retitled to a provisional "candidate gap" pending
    // first-failure instrumentation, and that the in-band positive controls
    // are only attested by local runs (the required `Perl LSP Rust Small
    // Result` check does not compile or run `--lib` tests for this crate at
    // all — see the doc comments on `assert_single_file_positive_control` /
    // `assert_multi_file_positive_control`).
    //
    // ## Revision note, round 5 (cursor-site confound)
    //
    // An independent deep-correctness review pass found a FIFTH confound,
    // one level deeper than rounds 1-4: every subroutine-class replay row
    // (`PackageSubSameFile`'s `startup` and `dispatch`@Catalyst.pm,
    // `DynamicAmbiguous`'s `dispatch`@Mojolicious.pm and
    // `dispatch`@Catalyst/Dispatcher.pm, `CrossFileSub`'s
    // `dispatch`@Dancer2/Core/App.pm) placed its cursor on the subroutine's
    // DEFINITION line, never a call/usage site — re-derived directly from the
    // manifest's own doc comments (e.g. "call(35, occ0) + def(94, occ1)"
    // followed by `cursor_occurrence: 1`, i.e. the def). This is the SAME
    // shape the pre-existing, mechanically-enforced
    // `references_routing_matrix::sub_calls` fixture + hard assertion H-3
    // (untouched by this PR, ~line 543 / ~647-667) already proves is
    // categorically excluded from `semantic_source_backed`: cursoring
    // `sub calculate { ... }`'s declaration in a fully controlled, single-file
    // scenario ALWAYS routes to `workspace_mixed`, independent of
    // entity-linking quality. So the `Catalyst.pm dispatch` row's
    // `coverage_gap` disposition was unproven — its non-activation was at
    // least as plausibly a categorical cursor-site exclusion as a genuine
    // entity-linking gap, and both subroutine positive controls this PR added
    // (`SUBROUTINE_CONTROL_TEXT`) already cursor a CALL site (`target()` at
    // line 7), never the declaration (`sub target {` at line 2) — the
    // definition-cursored manifest rows never matched the shape of their own
    // positive controls. This revision:
    // - adds an explicit `CursorSite::{Definition, Usage, NotApplicable}`
    //   dimension to every replay row (recorded in both the full receipt and
    //   the durable snapshot) so the cursor-site distinction is durable and
    //   can never be silently reintroduced;
    // - re-points `startup` (Mojolicious.pm) from occ1 (def, line 94) to occ0
    //   (the real call `$self->startup;`, line 35);
    // - re-points `dispatch`/`DynamicAmbiguous` (Mojolicious.pm) from occ0
    //   (def, line 54) to occ3 (the true self-call
    //   `$self->dispatch($c);`, line 67 — the other two `dispatch` calls in
    //   that file target a DIFFERENT class and remain in
    //   `known_false_occurrences`);
    // - re-points `dispatch`/`PackageSubSameFile` (Catalyst.pm) from occ1
    //   (def, line 184) to occ0 (the real call `$c->dispatch;`, line 180 —
    //   the deep reviewer's named call site);
    // - re-points `dispatch`/`CrossFileSub` from App.pm's definition (line
    //   34) to the actual cross-file call site in
    //   `lib/Dancer2/Core/Runner.pm:32` (`$app->dispatch($env)`) — that file
    //   is already part of the project's committed fixture tree and opened by
    //   `open_project`'s whole-directory walk, so no new fixture is added;
    // - leaves `dispatch`/`DynamicAmbiguous` (Catalyst/Dispatcher.pm)
    //   deliberately on its definition (occ0, line 12) with an explicit
    //   in-manifest comment: verified against the fixture corpus,
    //   `Catalyst::Dispatcher::dispatch` has NO in-project call site at all
    //   (Catalyst.pm's own `dispatch` stub does not delegate to
    //   `$self->dispatcher->dispatch`, unlike `forward`/`detach`/`go`/`visit`,
    //   which do call through `$self->dispatcher`), so there is no genuine
    //   usage site in the corpus to re-point to. This does not resurrect the
    //   H-3 confound for a `coverage_gap` claim: `classify_disposition`
    //   structurally never returns `coverage_gap` for `DynamicAmbiguous` (only
    //   `explicit_refusal_safe`/`unclassified`), so this row's forced
    //   definition-cursoring cannot manufacture an unfalsifiable coverage gap.
    //
    // CORRECTED FINDING (round 5): after re-pointing the cursor, the
    // previously-claimed `coverage_gap` disposition for the `Catalyst.pm
    // dispatch` (`PackageSubSameFile`) row is INVALIDATED by this fix — see
    // the checked-in snapshot and the disposition rollup printed by this test
    // for the row's corrected, currently-observed disposition. The `startup`
    // row (the other `PackageSubSameFile` row) is likewise re-measured, not
    // assumed unchanged. This strengthens, not weakens, the conclusion that
    // #1658 should stay open and bounded: the evidence for entity-linking
    // coverage gaps in the subroutine classes is now LESS conclusive than
    // previously claimed, not more — see the disposition table in the PR body
    // for the corrected, honest before/after per row.
    //
    // ## Revision note, round 6 (request-shape confound)
    //
    // A live maintainer review of round 5's fix found a SIXTH confound, one
    // level deeper: the surviving `coverage_gap` row (`Catalyst.pm dispatch`,
    // re-pointed in round 5 to cursor `$c->dispatch;`, line 180) is a
    // variable-receiver METHOD call. But the positive control that
    // authorizes `coverage_gap` for `PackageSubSameFile` rows
    // (`SUBROUTINE_CONTROL_TEXT`, which cursors bare `target()` /
    // package-qualified `InclDecl::target()`) is a FUNCTION call — bare or
    // package-qualified name resolution, never a variable-receiver method
    // dispatch. A FUNCTION-call control proves the semantic path resolves
    // FUNCTION calls; it says nothing about whether a variable-receiver
    // METHOD call resolves, since method dispatch and bareword/qualified
    // name resolution are structurally different code paths
    // (`references.rs::may_use_source_backed_references` /
    // `live_source_backed_reference_locations`). Authorizing `coverage_gap`
    // for a METHOD-shaped request on the strength of a FUNCTION-shaped
    // control's evidence is the same class of error as rounds 1-5: the
    // control doesn't match the request shape. This revision:
    // - adds an explicit `RequestShape::{FunctionCall, MethodCall,
    //   NotApplicable}` dimension to every replay row (recorded in both the
    //   full receipt and the durable snapshot) so a method-shaped request can
    //   never again be silently authorized by a function-shaped control;
    // - adds `METHOD_CONTROL_TEXT` + `observe_multi_file_method_positive_control`,
    //   a variable-receiver, same-file METHOD positive control mirroring
    //   `SUBROUTINE_CONTROL_TEXT`'s def-plus-call-site shape but for a method
    //   receiver (`$self->target_method()`), observed per-project alongside
    //   the existing lexical/subroutine controls;
    // - `classify_disposition` now requires the MATCHING
    //   `method_positive_control_evidence` (not `subroutine_positive_control_evidence`)
    //   before granting `coverage_gap` to a `MethodCall`-shaped
    //   `PackageSubSameFile` row; without it, the honest disposition is
    //   `method_shaped_request_unexercised` — "unexercised for this shape",
    //   not "proven absent" and not "coverage gap".
    //
    // CORRECTED FINDING (round 6): the Catalyst.pm `dispatch` row's
    // `coverage_gap` disposition is INVALIDATED again by this fix unless the
    // method-shaped control itself independently activates
    // `semantic_source_backed` for the `catalyst_skeleton` project — see the
    // checked-in snapshot and the disposition rollup printed by this test for
    // the row's corrected, currently-observed disposition, and the PR body
    // for the honest before/after. The #1658 conclusion is UNCHANGED: keep
    // the request-time scan BOUNDED, do not retire it — this correction only
    // stops attributing the method-shaped row's non-activation to the wrong
    // semantic path.
    //
    // ## Selection rule (so the corpus composition is inspectable)
    //
    // For each project we name one-to-three files already committed as UX
    // fixtures. Within those named files we hand-select EVERY same-file
    // lexical variable, same-file subroutine, cross-file subroutine,
    // imported-symbol call site, cross-class method-name collision, and
    // no-symbol position needed to populate every `FactClass` bucket below at
    // least once across the three projects. This is not a random sample of
    // "real user traffic" — see the module doc comment at the top of this
    // file for that disclaimer, which applies equally here. The manifest is
    // `REPLAY_MANIFEST` + `EMPTY_POSITION_MANIFEST` below; every entry's
    // `expected_true_occurrences` / `known_false_occurrences` was verified
    // against the fixture source (occurrence order confirmed via a boundary
    // match dump before this manifest was written — see the file/needle pairs
    // below for the exact source lines each index maps to).
    //
    // ## What is asserted vs. recorded
    //
    // `FactClass::is_strictly_checked()` classes (`LocalLexical`,
    // `PackageSubSameFile`, `DynamicAmbiguous`) are same-file-scoped by
    // construction, so whenever the observed answering tier claims
    // `semantic_source_backed` (source-backed exactness), the actual result
    // set is asserted to equal the curated expected set exactly and to never
    // contain a known-false location. Cross-file and imported-symbol classes
    // are intentionally out of the current same-file AST-indexed slice; they
    // are recorded (tier, counts, live receipt reason/fallback-state) without
    // a strict equality assertion, and are still checked against
    // `known_false_occurrences` (empty for both here, i.e. trivially
    // satisfied) so a future promotion that DOES claim exactness for them
    // would be caught by this same guard.
    // ─────────────────────────────────────────────────────────────────────────

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FactClass {
        /// Same-file lexical scalar variable with >=2 same-scope occurrences.
        LocalLexical,
        /// Subroutine defined and called within the same file, unambiguous name.
        PackageSubSameFile,
        /// Subroutine defined in one file, called from a different file in the
        /// same project — outside the current same-file AST-indexed scope.
        CrossFileSub,
        /// Symbol imported from a module outside the fixture project (e.g.
        /// `Carp::croak`) — declaration is not resolvable in-workspace.
        ImportedSymbol,
        /// Bareword name that collides across multiple classes/files, where a
        /// naive same-identifier text scan would find call sites that target a
        /// DIFFERENT class's method of the same name.
        DynamicAmbiguous,
        /// Cursor on a blank/no-symbol line — must yield the `empty` tier.
        EmptyPosition,
    }

    impl FactClass {
        fn as_str(self) -> &'static str {
            match self {
                Self::LocalLexical => "local_lexical",
                Self::PackageSubSameFile => "package_sub_same_file",
                Self::CrossFileSub => "cross_file_sub",
                Self::ImportedSymbol => "imported_symbol",
                Self::DynamicAmbiguous => "dynamic_ambiguous",
                Self::EmptyPosition => "empty_position",
            }
        }

        /// Whether a `semantic_source_backed` answer for this class is checked
        /// for exact equality against the curated expected set. See the module
        /// doc comment above for the rationale.
        fn is_strictly_checked(self) -> bool {
            matches!(self, Self::LocalLexical | Self::PackageSubSameFile | Self::DynamicAmbiguous)
        }
    }

    /// The shape of a `LocalLexical` row's declaration statement, verified
    /// against the fixture source at manifest-authoring time. This exists
    /// because of a maintainer-review-caught confound (PR #3998): the live
    /// `semantic_source_backed` variable tier gates on
    /// `references.rs::line_has_initialized_lexical_declaration`, which does a
    /// literal `"my {sigil}{name}"` / `"state {sigil}{name}"` substring search
    /// on the declaration line. Confirmed by reading that function: it finds
    /// `my $plugins` in `"my $plugins = ...;"` (a `SimpleInit` line), but does
    /// NOT find `my $c` anywhere in `"my ($self, $c) = @_;"` — the substring
    /// `"my $c"` never occurs there (`"my "` is followed by `"("`, not `"$c"`)
    /// — so destructuring declarations are REJECTED by this gate regardless of
    /// whether entity/anchor resolution upstream would otherwise have
    /// succeeded. A `LocalLexical` row whose declaration is a `Destructuring`
    /// form is therefore expected-by-design to never reach the tier; failing
    /// to activate is NOT evidence of an entity-linking coverage gap for that
    /// row, and `classify_disposition` short-circuits it to
    /// `"unsupported_declaration_shape"` rather than `"coverage_gap"`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum DeclarationShape {
        /// `my $name = ...` / `state $name = ...` — the only shape
        /// `line_has_initialized_lexical_declaration` recognizes.
        SimpleInit,
        /// `my (..., $name, ...) = @_` (or any parenthesized list form) — NOT
        /// recognized by the literal substring gate; out of the promoted
        /// slice by construction, independent of entity-linking correctness.
        Destructuring,
        /// Not a variable declaration (bareword sub/package rows, cross-file,
        /// imported-symbol, and empty-position requests never go through
        /// `line_has_initialized_lexical_declaration` at all, since that gate
        /// only runs `if let Some(sigil) = sigil`).
        NotApplicable,
    }

    impl DeclarationShape {
        fn as_str(self) -> &'static str {
            match self {
                Self::SimpleInit => "simple_init",
                Self::Destructuring => "destructuring",
                Self::NotApplicable => "not_applicable",
            }
        }
    }

    /// Whether `cursor_occurrence` sits on the symbol's definition/declaration
    /// line or on a genuine call/usage site distinct from it. Added in PR
    /// #3998's fifth review round: an independent deep-correctness pass found
    /// that every `PackageSubSameFile`/`DynamicAmbiguous`/`CrossFileSub` row
    /// (the subroutine-class rows) cursored the subroutine's DEFINITION line,
    /// never a call site — the opposite shape of the two subroutine positive
    /// controls (`SUBROUTINE_CONTROL_TEXT` cursors `target()` at line 7, never
    /// `sub target {` at line 2) and of the ALREADY-PRESENT, untouched
    /// `references_routing_matrix::sub_calls` fixture + hard assertion H-3
    /// (`position_of(SUB_TWO_CALLS, "calculate")` finds the FIRST occurrence,
    /// which is `sub calculate {`, the declaration): that harness mechanically
    /// proves cursor-on-DEFINITION for a subroutine categorically routes to
    /// `workspace_mixed`, never `semantic_source_backed`, independent of
    /// entity-linking quality. A definition-cursored subroutine row's
    /// non-activation was therefore unfalsifiable evidence of a coverage gap —
    /// same class of confound as rounds 1-4, one level deeper. This field
    /// makes the cursor site explicit and durable in the receipt/snapshot so
    /// the distinction can never be silently reintroduced.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum CursorSite {
        /// Cursor sits on the symbol's `sub name { ... }` definition/decl
        /// line. Only used where the fixture corpus genuinely has no in-file
        /// or in-project call site to cursor instead (documented per-row);
        /// never used to justify a `coverage_gap` disposition (see
        /// `classify_disposition`, which never returns `coverage_gap` for the
        /// `DynamicAmbiguous`/`CrossFileSub` classes that use this variant).
        Definition,
        /// Cursor sits on a genuine call/usage site of the symbol, distinct
        /// from its definition line — the shape both subroutine positive
        /// controls and the pre-existing `sub_calls`/H-3 fixture use.
        Usage,
        /// No symbol is under the cursor at all (`EmptyPosition` rows, a
        /// blank line) — the definition/usage distinction does not apply.
        NotApplicable,
    }

    impl CursorSite {
        fn as_str(self) -> &'static str {
            match self {
                Self::Definition => "definition",
                Self::Usage => "usage",
                Self::NotApplicable => "not_applicable",
            }
        }
    }

    /// Whether a request's cursor sits on a bare/package-qualified FUNCTION
    /// call, or a variable-receiver METHOD call (`$obj->method`). Added in PR
    /// #3998's sixth review round (maintainer finding on live review of round
    /// 5's fix): the existing subroutine positive control
    /// (`SUBROUTINE_CONTROL_TEXT`, which cursors bare `target()` /
    /// package-qualified `InclDecl::target()`) proves the semantic path can
    /// resolve a FUNCTION call; it says nothing about whether a
    /// variable-receiver METHOD call resolves — method dispatch and
    /// bareword/qualified name resolution are structurally different code
    /// paths. The Catalyst.pm `dispatch` row's real request is `$c->dispatch`
    /// — a METHOD call — so authorizing `coverage_gap` for it on the
    /// strength of the FUNCTION-shaped control repeats the same class of
    /// confound as rounds 1-5, one level deeper: the control doesn't match
    /// the request shape. See `classify_disposition`, which now requires a
    /// MATCHING `method_positive_control_evidence` before granting
    /// `coverage_gap` to a `MethodCall`-shaped `PackageSubSameFile` row.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum RequestShape {
        /// Bare (`target()`) or package-qualified (`Pkg::target()`) function
        /// call — the shape `SUBROUTINE_CONTROL_TEXT` exercises.
        FunctionCall,
        /// Variable-receiver method call (`$obj->method`, `$self->method`) —
        /// a distinct request shape the function-call control does not
        /// exercise. The shape `METHOD_CONTROL_TEXT` exercises.
        MethodCall,
        /// Not a subroutine call at all (variable references, imported
        /// symbols, empty positions, or a cursor sitting on a definition line
        /// rather than any call site).
        NotApplicable,
    }

    impl RequestShape {
        fn as_str(self) -> &'static str {
            match self {
                Self::FunctionCall => "function_call",
                Self::MethodCall => "method_call",
                Self::NotApplicable => "not_applicable",
            }
        }
    }

    /// One checked-in replay request. `cursor_occurrence` and the two
    /// occurrence-index sets refer to the boundary-safe, file-ordered
    /// occurrences of `needle` in `file` (see `ident_boundary_occurrences`).
    ///
    /// `scenario_rationale` is AUTHOR-SUPPLIED documentation only (why this
    /// request was selected) — it is never treated as live evidence. Live
    /// evidence (the provider's actual `reason`/`fallback_state`/etc.) is read
    /// from the receipt in `fire_replay_request` and stored on `ReplayRow`.
    #[derive(Debug, Clone, Copy)]
    struct ReplayRequest {
        project: &'static str,
        file: &'static str,
        fact_class: FactClass,
        needle: &'static str,
        cursor_occurrence: usize,
        include_declaration: bool,
        expected_true_occurrences: &'static [usize],
        known_false_occurrences: &'static [usize],
        /// See `DeclarationShape` doc comment. `NotApplicable` for every
        /// non-`LocalLexical` class.
        declaration_shape: DeclarationShape,
        /// See `CursorSite` doc comment.
        cursor_site: CursorSite,
        /// See `RequestShape` doc comment.
        request_shape: RequestShape,
        scenario_rationale: &'static str,
    }

    /// Checked-in request manifest. See the module doc comment above for the
    /// selection rule. Occurrence indices were confirmed against source with a
    /// boundary-match dump before authoring (each request's file/needle pair
    /// is independently re-verifiable by grepping the named fixture file).
    ///
    /// ## `include_declaration` <-> cursor-position <-> declaration-inclusion rule
    ///
    /// This is the rule the post-review repair fixed (see the revision note
    /// in the module doc comment above) — get it backwards and the oracle
    /// rejects correct provider behavior instead of proving anything:
    ///
    /// - When a `LocalLexical` row sends `include_declaration: false`, the
    ///   cursor MUST be placed on a USAGE occurrence (never the declaration),
    ///   matching the shape of the already-proven-in-production positive
    ///   control (`references.rs::handle_references_lexical_variable_without_declaration_uses_source_backed_tier`,
    ///   which cursors `$value`'s usage, not its `my` declaration). The
    ///   declaration occurrence MUST NOT appear in `expected_true_occurrences`
    ///   — it belongs in `known_false_occurrences` instead (labeled below as
    ///   "declaration, excluded because includeDeclaration:false").
    /// - Rows that send `include_declaration: true` (the bareword sub/package
    ///   rows below, plus the one `$plugins` includeDeclaration:true variant)
    ///   MAY list their declaration/definition occurrence in
    ///   `expected_true_occurrences`, and the cursor may sit on either the
    ///   declaration or a usage.
    #[rustfmt::skip]
    const REPLAY_MANIFEST: &[ReplayRequest] = &[
        // ---- Mojolicious: lib/Mojolicious.pm ----
        // `$plugins` in `dispatch`: decl(56, occ0) + 2 same-scope uses(57,occ1;
        // 59,occ2). No other `$plugins` in the file — single, unambiguous
        // scope. Cursor on occ1 (a USAGE); `include_declaration: false`, so
        // occ0 (the declaration) is excluded from the expected-true set.
        // Declaration `my $plugins = $self->plugins;` (line 56) is a
        // SimpleInit form: recognized by `line_has_initialized_lexical_declaration`.
        ReplayRequest {
            project: "mojolicious_skeleton", file: "lib/Mojolicious.pm",
            fact_class: FactClass::LocalLexical, needle: "$plugins",
            cursor_occurrence: 1, include_declaration: false,
            expected_true_occurrences: &[1, 2],
            // occ0: declaration, excluded because includeDeclaration:false.
            known_false_occurrences: &[0],
            declaration_shape: DeclarationShape::SimpleInit,
            cursor_site: CursorSite::Usage,
            request_shape: RequestShape::NotApplicable,
            scenario_rationale: "same_file_lexical_usage_without_declaration",
        },
        // Same `$plugins` symbol, `includeDeclaration: true` variant: cursor
        // still on occ1, but now the declaration IS expected in the result.
        // Exercises the opposite direction of the rule above on the same
        // fixture, per the review's "add both =false and =true cases" ask.
        ReplayRequest {
            project: "mojolicious_skeleton", file: "lib/Mojolicious.pm",
            fact_class: FactClass::LocalLexical, needle: "$plugins",
            cursor_occurrence: 1, include_declaration: true,
            expected_true_occurrences: &[0, 1, 2],
            known_false_occurrences: &[],
            declaration_shape: DeclarationShape::SimpleInit,
            cursor_site: CursorSite::Usage,
            request_shape: RequestShape::NotApplicable,
            scenario_rationale: "same_file_lexical_usage_with_declaration_included",
        },
        // `$c` in `dispatch` (occ 0-6, occ0 = decl): must exclude the
        // unrelated `$c` in `handler` (occ 7-10) — direct scope-shadow analog
        // of the existing F1 curated corpus fixture, on real code. Cursor on
        // occ1 (a USAGE); `include_declaration: false`, so occ0 is excluded.
        // Declaration `my ($self, $c) = @_;` (line 55) is a Destructuring
        // form: `line_has_initialized_lexical_declaration` never finds the
        // literal substring `"my $c"` in it, so this row is expected-by-design
        // to never reach `semantic_source_backed` regardless of entity-linking.
        ReplayRequest {
            project: "mojolicious_skeleton", file: "lib/Mojolicious.pm",
            fact_class: FactClass::LocalLexical, needle: "$c",
            cursor_occurrence: 1, include_declaration: false,
            expected_true_occurrences: &[1, 2, 3, 4, 5, 6],
            // occ0: declaration, excluded because includeDeclaration:false.
            known_false_occurrences: &[0, 7, 8, 9, 10],
            declaration_shape: DeclarationShape::Destructuring,
            cursor_site: CursorSite::Usage,
            request_shape: RequestShape::NotApplicable,
            scenario_rationale: "same_file_lexical_usage_without_declaration",
        },
        // `$c` in `handler` (occ 7-10, occ7 = decl): the mirror-image request
        // from the opposite scope. Cursor on occ8 (a USAGE);
        // `include_declaration: false`, so occ7 is excluded. Declaration
        // `my $c = $self->build_controller($tx);` (line 65) is a SimpleInit
        // form (only `$self`/`$tx` in the ENCLOSING sub signature are
        // destructured; `$c` itself is a plain `my $c = ...`).
        ReplayRequest {
            project: "mojolicious_skeleton", file: "lib/Mojolicious.pm",
            fact_class: FactClass::LocalLexical, needle: "$c",
            cursor_occurrence: 8, include_declaration: false,
            expected_true_occurrences: &[8, 9, 10],
            // occ7: declaration, excluded because includeDeclaration:false.
            known_false_occurrences: &[0, 1, 2, 3, 4, 5, 6, 7],
            declaration_shape: DeclarationShape::SimpleInit,
            cursor_site: CursorSite::Usage,
            request_shape: RequestShape::NotApplicable,
            scenario_rationale: "same_file_lexical_usage_without_declaration",
        },
        // `dispatch`: def(54, occ0) + true call `$self->dispatch($c)`(67, occ3)
        // target `Mojolicious::dispatch`; `$self->static->dispatch($c)`(58,
        // occ1) and `$self->routes->dispatch($c)`(60, occ2) call DIFFERENT
        // classes' `dispatch` methods and must never appear in an exact result.
        // PR #3998 fifth review round: cursor moved from occ0 (the
        // DEFINITION, line 54) to occ3 (the true self-call, line 67) — a
        // definition-cursored subroutine request is confounded by the
        // pre-existing, untouched `references_routing_matrix` H-3 assertion
        // (cursor-on-definition categorically routes to `workspace_mixed`,
        // never `semantic_source_backed`, in a fully controlled single-file
        // scenario). See the `CursorSite` doc comment.
        ReplayRequest {
            project: "mojolicious_skeleton", file: "lib/Mojolicious.pm",
            fact_class: FactClass::DynamicAmbiguous, needle: "dispatch",
            cursor_occurrence: 3, include_declaration: true,
            expected_true_occurrences: &[0, 3], known_false_occurrences: &[1, 2],
            declaration_shape: DeclarationShape::NotApplicable,
            cursor_site: CursorSite::Usage,
            request_shape: RequestShape::MethodCall,
            scenario_rationale: "cross_class_method_dispatch_not_disambiguated_by_receiver_type",
        },
        // `startup`: call(35, occ0) + def(94, occ1), unambiguous same-file sub.
        // PR #3998 fifth review round: cursor moved from occ1 (the
        // DEFINITION, line 94) to occ0 (the true call, line 35) — same
        // cursor-site confound as the `dispatch` row above; see `CursorSite`.
        ReplayRequest {
            project: "mojolicious_skeleton", file: "lib/Mojolicious.pm",
            fact_class: FactClass::PackageSubSameFile, needle: "startup",
            cursor_occurrence: 0, include_declaration: true,
            expected_true_occurrences: &[0, 1], known_false_occurrences: &[],
            declaration_shape: DeclarationShape::NotApplicable,
            cursor_site: CursorSite::Usage,
            request_shape: RequestShape::MethodCall,
            scenario_rationale: "same_file_subroutine_def_and_call",
        },
        // `croak`(73, occ1): Carp is not vendored in this fixture project, so
        // no same-file declaration exists to prove exactness against.
        ReplayRequest {
            project: "mojolicious_skeleton", file: "lib/Mojolicious.pm",
            fact_class: FactClass::ImportedSymbol, needle: "croak",
            cursor_occurrence: 1, include_declaration: false,
            expected_true_occurrences: &[], known_false_occurrences: &[],
            declaration_shape: DeclarationShape::NotApplicable,
            cursor_site: CursorSite::Usage,
            request_shape: RequestShape::FunctionCall,
            scenario_rationale: "carp_croak_declared_outside_the_fixture_project",
        },
        // ---- Dancer2: lib/Dancer2/Core/App.pm ----
        // `$code` in `add_route`: decl(26,occ0) + uses(27,occ1; 30,occ2); the
        // unrelated `$code` in `add_hook` (occ3,occ4) must be excluded.
        // Cursor on occ1 (a USAGE); `include_declaration: false`. Declaration
        // `my $code = $args{code};` (line 26) is a SimpleInit form.
        ReplayRequest {
            project: "dancer2_skeleton", file: "lib/Dancer2/Core/App.pm",
            fact_class: FactClass::LocalLexical, needle: "$code",
            cursor_occurrence: 1, include_declaration: false,
            expected_true_occurrences: &[1, 2],
            // occ0: declaration, excluded because includeDeclaration:false.
            known_false_occurrences: &[0, 3, 4],
            declaration_shape: DeclarationShape::SimpleInit,
            cursor_site: CursorSite::Usage,
            request_shape: RequestShape::NotApplicable,
            scenario_rationale: "same_file_lexical_usage_without_declaration",
        },
        // `$method` scope pair, side A: `add_route` (occ0 = decl, occ1) vs
        // `dispatch` (occ2,occ3) — same bare name, non-overlapping scopes.
        // Cursor on occ1 (a USAGE); `include_declaration: false`. Declaration
        // `my $method = lc $args{method};` (line 24) is a SimpleInit form.
        ReplayRequest {
            project: "dancer2_skeleton", file: "lib/Dancer2/Core/App.pm",
            fact_class: FactClass::LocalLexical, needle: "$method",
            cursor_occurrence: 1, include_declaration: false,
            expected_true_occurrences: &[1],
            // occ0: declaration, excluded because includeDeclaration:false.
            known_false_occurrences: &[0, 2, 3],
            declaration_shape: DeclarationShape::SimpleInit,
            cursor_site: CursorSite::Usage,
            request_shape: RequestShape::NotApplicable,
            scenario_rationale: "same_file_lexical_usage_without_declaration",
        },
        // `$method` scope pair, side B: the mirror-image request (occ2 = decl).
        // Cursor on occ3 (a USAGE); `include_declaration: false`. Declaration
        // `my $method = lc $env->{REQUEST_METHOD};` (line 36) is a SimpleInit
        // form.
        ReplayRequest {
            project: "dancer2_skeleton", file: "lib/Dancer2/Core/App.pm",
            fact_class: FactClass::LocalLexical, needle: "$method",
            cursor_occurrence: 3, include_declaration: false,
            expected_true_occurrences: &[3],
            // occ2: declaration, excluded because includeDeclaration:false.
            known_false_occurrences: &[0, 1, 2],
            declaration_shape: DeclarationShape::SimpleInit,
            cursor_site: CursorSite::Usage,
            request_shape: RequestShape::NotApplicable,
            scenario_rationale: "same_file_lexical_usage_without_declaration",
        },
        // `dispatch` is DEFINED at App.pm:34; its only in-project call site is
        // `Runner.pm:32` (`$app->dispatch($env)`) — a DIFFERENT file, outside
        // the current same-file AST-indexed scope. PR #3998 fifth review
        // round: this row previously cursored the App.pm DEFINITION (occ0,
        // line 34), which is the confounded shape (see `CursorSite`). Moved
        // to cursor the actual call site in `Runner.pm` instead — the file
        // is already opened as part of `open_project`'s whole-project walk,
        // so no new fixture file is introduced. `needle: "dispatch"` has
        // exactly ONE boundary-safe occurrence in `Runner.pm` (line 32,
        // `if (my $res = $app->dispatch($env)) {`), so `cursor_occurrence: 0`
        // is that call, not App.pm's definition. Not strictly checked
        // (`CrossFileSub` is not in `FactClass::is_strictly_checked`), so
        // `expected_true_occurrences`/`known_false_occurrences` are recorded
        // for receipt completeness only, not asserted for exactness.
        ReplayRequest {
            project: "dancer2_skeleton", file: "lib/Dancer2/Core/Runner.pm",
            fact_class: FactClass::CrossFileSub, needle: "dispatch",
            cursor_occurrence: 0, include_declaration: true,
            expected_true_occurrences: &[0], known_false_occurrences: &[],
            declaration_shape: DeclarationShape::NotApplicable,
            cursor_site: CursorSite::Usage,
            request_shape: RequestShape::MethodCall,
            scenario_rationale: "cross_file_caller_lives_outside_the_same_file_scope",
        },
        // `croak`(27, occ1): same out-of-fixture-declaration reasoning as Mojolicious.
        ReplayRequest {
            project: "dancer2_skeleton", file: "lib/Dancer2/Core/App.pm",
            fact_class: FactClass::ImportedSymbol, needle: "croak",
            cursor_occurrence: 1, include_declaration: false,
            expected_true_occurrences: &[], known_false_occurrences: &[],
            declaration_shape: DeclarationShape::NotApplicable,
            cursor_site: CursorSite::Usage,
            request_shape: RequestShape::FunctionCall,
            scenario_rationale: "carp_croak_declared_outside_the_fixture_project",
        },
        // ---- Catalyst: lib/Catalyst/Action.pm, lib/Catalyst.pm, lib/Catalyst/Dispatcher.pm ----
        // `$controller` in `dispatch`: decl(23,occ0) + uses(24,occ1; 26,occ2),
        // single unambiguous scope, no other `$controller` in the file.
        // Cursor on occ1 (a USAGE); `include_declaration: false`. Declaration
        // `my $controller = $c->component($class);` (line 23) is a SimpleInit
        // form.
        ReplayRequest {
            project: "catalyst_skeleton", file: "lib/Catalyst/Action.pm",
            fact_class: FactClass::LocalLexical, needle: "$controller",
            cursor_occurrence: 1, include_declaration: false,
            expected_true_occurrences: &[1, 2],
            // occ0: declaration, excluded because includeDeclaration:false.
            known_false_occurrences: &[0],
            declaration_shape: DeclarationShape::SimpleInit,
            cursor_site: CursorSite::Usage,
            request_shape: RequestShape::NotApplicable,
            scenario_rationale: "same_file_lexical_usage_without_declaration",
        },
        // `$c` in `dispatch` (occ0-4, occ0 = decl): must exclude `$c` in
        // `execute`, `match`, and `match_captures` (occ5-10) — three OTHER
        // scopes share the name. Cursor on occ1 (a USAGE);
        // `include_declaration: false`. Declaration `my ($self, $c) = @_;`
        // (line 21) is a Destructuring form — same shape gate as the
        // Mojolicious `$c`-in-`dispatch` row above.
        ReplayRequest {
            project: "catalyst_skeleton", file: "lib/Catalyst/Action.pm",
            fact_class: FactClass::LocalLexical, needle: "$c",
            cursor_occurrence: 1, include_declaration: false,
            expected_true_occurrences: &[1, 2, 3, 4],
            // occ0: declaration, excluded because includeDeclaration:false.
            known_false_occurrences: &[0, 5, 6, 7, 8, 9, 10],
            declaration_shape: DeclarationShape::Destructuring,
            cursor_site: CursorSite::Usage,
            request_shape: RequestShape::NotApplicable,
            scenario_rationale: "same_file_lexical_usage_without_declaration",
        },
        // `dispatch` in Catalyst.pm: call `$c->dispatch;`(180,occ0) +
        // def(184,occ1) — unambiguous WITHIN this file even though `dispatch`
        // is also independently defined in Action.pm and Dispatcher.pm. PR
        // #3998 fifth review round: cursor moved from occ1 (the DEFINITION,
        // line 184) to occ0 (the true call, line 180 — the deep reviewer's
        // named call site) — the confound this replaces is the same one H-3
        // proves categorically in `references_routing_matrix`; see
        // `CursorSite`.
        ReplayRequest {
            project: "catalyst_skeleton", file: "lib/Catalyst.pm",
            fact_class: FactClass::PackageSubSameFile, needle: "dispatch",
            cursor_occurrence: 0, include_declaration: true,
            expected_true_occurrences: &[0, 1], known_false_occurrences: &[],
            declaration_shape: DeclarationShape::NotApplicable,
            cursor_site: CursorSite::Usage,
            request_shape: RequestShape::MethodCall,
            scenario_rationale: "same_file_subroutine_def_and_call",
        },
        // `dispatch` in Dispatcher.pm: def(12,occ0); ALL THREE in-file calls
        // (`$action->dispatch`(22,occ1), `$action_or_url->dispatch`(28,occ2),
        // `$action->dispatch`(32,occ3)) target `Catalyst::Action::dispatch` on
        // a differently-named receiver, NOT a recursive self-call — none of
        // them may appear in an exact result for `Dispatcher::dispatch`. PR
        // #3998 fifth review round: unlike the other subroutine-class rows,
        // this one is DELIBERATELY LEFT on the definition (occ0, line 12) —
        // verified against the fixture corpus, `Catalyst::Dispatcher::dispatch`
        // has NO in-project call site at all. Catalyst.pm's own `dispatch`
        // stub (line 184-187) does not delegate to
        // `$self->dispatcher->dispatch(...)` in this trimmed skeleton (unlike
        // `forward`/`detach`/`go`/`visit`, which do call
        // `$self->dispatcher->{forward,detach,go,visit}`), so there is no
        // genuine usage site anywhere in the corpus to re-point to. This does
        // NOT reintroduce the H-3 confound for a `coverage_gap` claim:
        // `classify_disposition` structurally never returns `coverage_gap`
        // for `DynamicAmbiguous` (only `explicit_refusal_safe`/`unclassified`
        // — see the match arm below), so this row's forced
        // definition-cursoring cannot manufacture an unfalsifiable coverage
        // gap the way the four re-pointed rows could have.
        ReplayRequest {
            project: "catalyst_skeleton", file: "lib/Catalyst/Dispatcher.pm",
            fact_class: FactClass::DynamicAmbiguous, needle: "dispatch",
            cursor_occurrence: 0, include_declaration: true,
            expected_true_occurrences: &[0], known_false_occurrences: &[1, 2, 3],
            declaration_shape: DeclarationShape::NotApplicable,
            cursor_site: CursorSite::Definition,
            request_shape: RequestShape::NotApplicable,
            scenario_rationale: "cross_class_method_dispatch_not_disambiguated_by_receiver_type",
        },
        // `croak`(31, occ1): same out-of-fixture-declaration reasoning as above.
        ReplayRequest {
            project: "catalyst_skeleton", file: "lib/Catalyst/Dispatcher.pm",
            fact_class: FactClass::ImportedSymbol, needle: "croak",
            cursor_occurrence: 1, include_declaration: false,
            expected_true_occurrences: &[], known_false_occurrences: &[],
            declaration_shape: DeclarationShape::NotApplicable,
            cursor_site: CursorSite::Usage,
            request_shape: RequestShape::FunctionCall,
            scenario_rationale: "carp_croak_declared_outside_the_fixture_project",
        },
    ];

    /// No-symbol cursor positions (blank lines), one per project, 0-based line.
    const EMPTY_POSITION_MANIFEST: &[(&str, &str, usize)] = &[
        ("mojolicious_skeleton", "lib/Mojolicious.pm", 17),
        ("dancer2_skeleton", "lib/Dancer2/Core/App.pm", 7),
        ("catalyst_skeleton", "lib/Catalyst/Action.pm", 7),
    ];

    /// One observed row. Fields prefixed `receipt_*` are read LIVE from the
    /// `explainProviderDecision` receipt (never author-supplied) — this is
    /// the evidence assertions #4/#5/#6 are checked against.
    /// `scenario_rationale` is the manifest author's documentation string,
    /// kept separately so it is never mistaken for provider evidence.
    #[derive(Debug)]
    struct ReplayRow {
        project: &'static str,
        file: &'static str,
        fact_class: FactClass,
        needle: &'static str,
        include_declaration: bool,
        uri: String,
        line: u32,
        character: u32,
        answering_tier: String,
        source_backed: bool,
        result_count: usize,
        index_result_count: usize,
        text_result_count: usize,
        latency_us: u64,
        /// Live receipt `reason` (e.g. `"live_provider_result"`, `"no_result"`).
        receipt_reason: String,
        /// Live receipt `decision` (e.g. `"acted"`, `"fallback"`).
        receipt_decision: String,
        /// Live receipt `fallback_state` (e.g. `"live_provider"`, `"legacy_provider"`, `"no_result"`).
        receipt_fallback_state: String,
        /// Live receipt `confidence` (`"high"` | `"low"`).
        receipt_confidence: String,
        /// Live receipt `freshness`. NOTE: production currently hardcodes this
        /// to the constant `"fresh"` regardless of real document/index
        /// generation state (see `references.rs::record_references_provider_decision_trace`)
        /// — this replay reports the field as-is and does not claim it tracks
        /// genuine staleness; see `references_representative_replay_genuine_stale_generation_downgrades_index_state`
        /// for the real staleness proof, which uses `index_state` instead.
        receipt_freshness: String,
        /// Live receipt `fact_source` (producer, e.g. `"semantic_fact"`, `"fallback"`).
        receipt_fact_source: String,
        /// Live receipt `source_backed_state` (proof class, e.g.
        /// `"semantic_source_backed_ast_index"`).
        receipt_source_backed_state: String,
        /// See `DeclarationShape` doc comment.
        declaration_shape: DeclarationShape,
        /// See `CursorSite` doc comment.
        cursor_site: CursorSite,
        /// See `RequestShape` doc comment.
        request_shape: RequestShape,
        scenario_rationale: &'static str,
        disposition: &'static str,
    }

    fn real_project_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .map(|root| root.join("test_corpus/real_projects"))
            .ok_or_else(|| "CARGO_MANIFEST_DIR must be nested under the workspace root".into())
    }

    fn collect_project_files(
        project: &str,
    ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        fn walk(
            root: &Path,
            dir: &Path,
            out: &mut Vec<(String, String)>,
        ) -> Result<(), Box<dyn std::error::Error>> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    walk(root, &path, out)?;
                } else if matches!(
                    path.extension().and_then(|e| e.to_str()),
                    Some("pm" | "pl" | "t")
                ) {
                    let rel = path.strip_prefix(root)?.to_string_lossy().replace('\\', "/");
                    let content = fs::read_to_string(&path)
                        .map_err(|e| format!("reading {}: {e}", path.display()))?;
                    out.push((rel, content));
                }
            }
            Ok(())
        }

        let root = real_project_root()?.join(project);
        let mut files = Vec::new();
        walk(&root, &root, &mut files)?;
        files.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(files)
    }

    fn project_uri(project: &str, relative_path: &str) -> String {
        format!("file:///real_projects/{project}/{relative_path}")
    }

    fn open_project(
        server: &LspServer,
        project: &str,
    ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        let files = collect_project_files(project)?;
        for (relative_path, content) in &files {
            let uri = project_uri(project, relative_path);
            open_document(server, &uri, content)?;
        }
        Ok(files)
    }

    fn fixture_content<'a>(
        files: &'a [(String, String)],
        relative_path: &str,
    ) -> Result<&'a str, Box<dyn std::error::Error>> {
        files
            .iter()
            .find(|(path, _)| path == relative_path)
            .map(|(_, content)| content.as_str())
            .ok_or_else(|| format!("missing fixture file {relative_path}").into())
    }

    /// Boundary-safe occurrences of `needle` in `content`, in file order, as
    /// `(line0, byte_col)`. Reuses the same word-boundary semantics as the
    /// production text-search fallback (`crate::util::is_word_boundary`) —
    /// this is purely the oracle's tokenizer, not the routing/scope logic
    /// under test.
    fn ident_boundary_occurrences(content: &str, needle: &str) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (line0, line) in content.lines().enumerate() {
            let bytes = line.as_bytes();
            let mut start = 0usize;
            while let Some(idx) = line[start..].find(needle) {
                let byte_pos = start + idx;
                if is_word_boundary(bytes, byte_pos, needle.len()) {
                    out.push((line0, byte_pos));
                }
                start = byte_pos + needle.len();
            }
        }
        out
    }

    fn occurrence_position(
        content: &str,
        needle: &str,
        occurrence: usize,
    ) -> Result<(u32, u32), Box<dyn std::error::Error>> {
        let occurrences = ident_boundary_occurrences(content, needle);
        let (line0, byte_col) = occurrences
            .get(occurrence)
            .copied()
            .ok_or_else(|| format!("occurrence {occurrence} of `{needle}` not found"))?;
        let line_text = content.lines().nth(line0).unwrap_or("");
        let character = crate::util::byte_to_utf16_col(line_text, byte_col);
        Ok((u32::try_from(line0)?, u32::try_from(character)?))
    }

    fn occurrence_location_key(
        uri: &str,
        content: &str,
        needle: &str,
        occurrence: usize,
    ) -> Result<(String, u32, u32), Box<dyn std::error::Error>> {
        let (line, character) = occurrence_position(content, needle, occurrence)?;
        Ok((uri.to_string(), line, character))
    }

    /// Extract `(uri, line, character)` keys from an array of LSP
    /// `Location`/`LocationLink` values, using the start position.
    fn location_keys(results: &[Value]) -> Vec<(String, u32, u32)> {
        results
            .iter()
            .filter_map(|entry| {
                let uri =
                    entry.get("uri").or_else(|| entry.get("targetUri"))?.as_str()?.to_string();
                let range = entry.get("range").or_else(|| entry.get("targetRange"))?;
                let line = range.get("start")?.get("line")?.as_u64()?;
                let character = range.get("start")?.get("character")?.as_u64()?;
                Some((uri, u32::try_from(line).ok()?, u32::try_from(character).ok()?))
            })
            .collect()
    }

    /// Returns `Ok(())` when `actual` (any order) is exactly equal to
    /// `expected` (any order) as sets; otherwise a diagnostic `Err`.
    ///
    /// Extracted as a pure function — independent of any live server or the
    /// `semantic_source_backed` tier ever firing — so its discriminating
    /// power can be unit-tested directly with synthetic inputs. See
    /// `exact_match_check_rejects_declaration_reintroduced_into_expected`
    /// below: this is the mutation test that "revert-proves" the fix for the
    /// PR #3998 review's decisive finding (a declaration wrongly present in a
    /// decl-excluding expected set).
    fn exact_match_check(
        actual: &[(String, u32, u32)],
        expected: &[(String, u32, u32)],
    ) -> Result<(), String> {
        let mut actual_sorted = actual.to_vec();
        actual_sorted.sort();
        let mut expected_sorted = expected.to_vec();
        expected_sorted.sort();
        if actual_sorted == expected_sorted {
            Ok(())
        } else {
            Err(format!("expected {expected_sorted:?}, got {actual_sorted:?}"))
        }
    }

    /// Returns `Ok(())` when none of `forbidden` appears in `actual`;
    /// otherwise a diagnostic `Err`. Extracted as a pure function for the same
    /// mutation-testing reason as `exact_match_check`.
    fn forbidden_check(
        actual: &[(String, u32, u32)],
        forbidden: &[(String, u32, u32)],
    ) -> Result<(), String> {
        for location in forbidden {
            if actual.contains(location) {
                return Err(format!("actual result contains known-false location {location:?}"));
            }
        }
        Ok(())
    }

    /// Live-receipt completeness check: every field the replay treats as
    /// evidence (not author-supplied) must be present and non-empty. Extracted
    /// as a pure function over a parsed JSON object so it can be unit-tested
    /// directly with a synthetic incomplete receipt.
    fn validate_receipt_has_required_fields(
        receipt: &serde_json::Map<String, Value>,
    ) -> Result<(), String> {
        const REQUIRED_STRING_FIELDS: &[&str] = &[
            "decision",
            "reason",
            "fallback_state",
            "confidence",
            "freshness",
            "fact_source",
            "source_backed_state",
            "answering_tier",
            "index_state",
        ];
        for field in REQUIRED_STRING_FIELDS {
            let is_present_and_non_empty =
                receipt.get(*field).and_then(Value::as_str).is_some_and(|value| !value.is_empty());
            if !is_present_and_non_empty {
                return Err(format!("receipt missing or empty required field `{field}`"));
            }
        }
        if receipt.get("latency_us").and_then(Value::as_u64).is_none() {
            return Err("receipt missing required numeric field `latency_us`".to_string());
        }
        Ok(())
    }

    /// Live-receipt fields read directly off a parsed `request_receipt`
    /// object — never author-supplied. Bundled so `fire_replay_request` and
    /// `fire_empty_request` can extract them identically.
    struct LiveReceiptEvidence {
        answering_tier: String,
        source_backed: bool,
        index_result_count: usize,
        text_result_count: usize,
        latency_us: u64,
        reason: String,
        decision: String,
        fallback_state: String,
        confidence: String,
        freshness: String,
        fact_source: String,
        source_backed_state: String,
    }

    fn read_live_receipt_evidence(
        receipt: &serde_json::Map<String, Value>,
        wall_us: u64,
    ) -> Result<LiveReceiptEvidence, Box<dyn std::error::Error>> {
        validate_receipt_has_required_fields(receipt)?;
        let str_field = |name: &str| -> String {
            receipt.get(name).and_then(Value::as_str).unwrap_or("").to_string()
        };
        Ok(LiveReceiptEvidence {
            answering_tier: str_field("answering_tier"),
            source_backed: receipt.get("source_backed").and_then(Value::as_bool).unwrap_or(false),
            index_result_count: usize::try_from(
                receipt.get("index_result_count").and_then(Value::as_u64).unwrap_or(0),
            )
            .unwrap_or(usize::MAX),
            text_result_count: usize::try_from(
                receipt.get("text_result_count").and_then(Value::as_u64).unwrap_or(0),
            )
            .unwrap_or(usize::MAX),
            latency_us: receipt.get("latency_us").and_then(Value::as_u64).unwrap_or(wall_us),
            reason: str_field("reason"),
            decision: str_field("decision"),
            fallback_state: str_field("fallback_state"),
            confidence: str_field("confidence"),
            freshness: str_field("freshness"),
            fact_source: str_field("fact_source"),
            source_backed_state: str_field("source_backed_state"),
        })
    }

    /// Roll an observed row up into a #1658 disposition bucket, driven by
    /// EVIDENCE rather than `FactClass` alone (per the PR #3998 review):
    ///
    /// - `unsupported_declaration_shape`: checked FIRST, before anything else.
    ///   A `LocalLexical` row whose declaration is a `Destructuring` form
    ///   (`my (..., $name, ...) = @_`) is expected-by-design to never reach
    ///   `semantic_source_backed` — `references.rs::line_has_initialized_lexical_declaration`
    ///   categorically rejects that shape regardless of entity-linking
    ///   correctness. Non-activation here is a recorded, expected blocker, not
    ///   a coverage gap.
    /// - `policy_excluded_request_shape`: checked SECOND. A `LocalLexical`
    ///   (variable/sigil) row sent with `include_declaration: true` is
    ///   categorically outside the promoted lexical slice —
    ///   `references.rs::may_use_source_backed_references` only allows
    ///   variable symbols through when `include_declaration == false`
    ///   (`!symbol_is_variable || (ENABLE_PIR_A_LEXICAL_REFERENCES_LIVE && !include_declaration)`).
    ///   Non-activation for this request shape is a policy exclusion, not a
    ///   gap — it says nothing about entity-linking or declaration-shape
    ///   correctness. (Bareword/sub symbols are unaffected: `symbol_is_variable`
    ///   is `false` for them, so the gate passes regardless of
    ///   `include_declaration` — confirmed by
    ///   `references.rs::handle_references_include_declaration_true_reaches_source_backed_tier_and_appends_declaration`.)
    /// - `index_parity_proven`: only from an observed exact match on a
    ///   `source_backed` row for a strictly-checked class.
    /// - `explicit_refusal_safe`: only from a genuinely EXERCISED refusal —
    ///   `EmptyPosition` (asserted empty) or a non-source-backed row that
    ///   itself returned zero results. A noisy non-empty `workspace_mixed`
    ///   answer is NOT "safe" just because it avoided the known-false set —
    ///   that would only prove "not disproven", not "verified correct".
    /// - `coverage_gap`: only for an ELIGIBLE row (`SimpleInit`/`NotApplicable`
    ///   declaration shape, and for `LocalLexical` also `include_declaration:
    ///   false`) that failed to reach `source_backed`, AND ONLY when the
    ///   MATCHING positive control has evidence for that project:
    ///   `lexical_positive_control_evidence` for `LocalLexical`,
    ///   `subroutine_positive_control_evidence` for a `FunctionCall`-shaped
    ///   `PackageSubSameFile` row, `method_positive_control_evidence` for a
    ///   `MethodCall`-shaped `PackageSubSameFile` row. A lexical-only control
    ///   does NOT authorize `coverage_gap` for `PackageSubSameFile` — a
    ///   subroutine-specific harness/entity-resolution boundary is a distinct
    ///   possible confound. Without the matching evidence the class is
    ///   `unexercised`, not `coverage_gap`.
    /// - `method_shaped_request_unexercised`: PR #3998 round 6 (see the
    ///   module doc comment). A `MethodCall`-shaped `PackageSubSameFile` row
    ///   whose FUNCTION-call control (`subroutine_positive_control_evidence`)
    ///   activated — which would have authorized `coverage_gap` under the
    ///   pre-round-6 logic — but whose MATCHING method-shaped control
    ///   (`method_positive_control_evidence`) did NOT. The function-call
    ///   control's activation proves nothing about method-receiver
    ///   resolution, so this row's non-activation cannot be attributed to a
    ///   genuine entity-linking coverage gap; it is honestly "unexercised for
    ///   this shape", distinct from a `PackageSubSameFile` row whose
    ///   FUNCTION-call control never activated at all (which stays the plain
    ///   `unexercised` bucket below, unaffected by this shape check).
    /// - `unclassified`: the default — including any `source_backed` row that
    ///   is NOT strictly checked, or a strictly-checked `source_backed` row
    ///   that (should never happen, since the hard assertion above would have
    ///   already failed the test) somehow reaches this classifier mismatched.
    #[allow(clippy::too_many_arguments)]
    fn classify_disposition(
        fact_class: FactClass,
        declaration_shape: DeclarationShape,
        include_declaration: bool,
        request_shape: RequestShape,
        source_backed: bool,
        result_count: usize,
        result_locations: &[(String, u32, u32)],
        expected_keys: &[(String, u32, u32)],
        lexical_positive_control_evidence: bool,
        subroutine_positive_control_evidence: bool,
        method_positive_control_evidence: bool,
    ) -> &'static str {
        if declaration_shape == DeclarationShape::Destructuring {
            return "unsupported_declaration_shape";
        }
        if fact_class == FactClass::LocalLexical && include_declaration {
            return "policy_excluded_request_shape";
        }
        if source_backed {
            if fact_class.is_strictly_checked()
                && exact_match_check(result_locations, expected_keys).is_ok()
            {
                return "index_parity_proven";
            }
            return "unclassified";
        }
        match fact_class {
            FactClass::EmptyPosition => "explicit_refusal_safe",
            FactClass::LocalLexical => {
                if lexical_positive_control_evidence {
                    "coverage_gap"
                } else {
                    "unexercised"
                }
            }
            FactClass::PackageSubSameFile => {
                if request_shape == RequestShape::MethodCall {
                    // Round 6: a method-shaped request must be authorized by
                    // a MATCHING method-shaped control, never by the
                    // function-call control alone (see the doc comment
                    // above and the module doc comment's round-6 note).
                    if method_positive_control_evidence {
                        "coverage_gap"
                    } else if subroutine_positive_control_evidence {
                        "method_shaped_request_unexercised"
                    } else {
                        "unexercised"
                    }
                } else if subroutine_positive_control_evidence {
                    "coverage_gap"
                } else {
                    "unexercised"
                }
            }
            FactClass::CrossFileSub | FactClass::ImportedSymbol | FactClass::DynamicAmbiguous => {
                if result_count == 0 {
                    "explicit_refusal_safe"
                } else {
                    "unclassified"
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn fire_replay_request(
        server: &LspServer,
        request: &ReplayRequest,
        fixture_files: &[(String, String)],
        lexical_positive_control_evidence: bool,
        subroutine_positive_control_evidence: bool,
        method_positive_control_evidence: bool,
    ) -> Result<ReplayRow, Box<dyn std::error::Error>> {
        let content = fixture_content(fixture_files, request.file)?;
        let uri = project_uri(request.project, request.file);
        let (line, character) =
            occurrence_position(content, request.needle, request.cursor_occurrence)?;

        let params = json!({
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": character},
            "context": {"includeDeclaration": request.include_declaration}
        });

        let t0 = Instant::now();
        let result = server.test_handle_references(Some(params))?;
        let wall_us = u64::try_from(t0.elapsed().as_micros()).unwrap_or(u64::MAX);

        let explanation = explain_provider_decision(server, "references")?;
        let receipt = explanation
            .get("request_receipt")
            .and_then(Value::as_object)
            .ok_or("missing request_receipt")?;

        // Hard assertion #7: the receipt is bound to THIS request.
        let receipt_uri = receipt.get("uri").and_then(Value::as_str).unwrap_or("");
        assert_eq!(
            receipt_uri, uri,
            "receipt URI mismatch for {}/{} needle={:?}",
            request.project, request.file, request.needle
        );
        let receipt_line = receipt.get("line").and_then(Value::as_u64).unwrap_or(u64::MAX);
        assert_eq!(
            receipt_line,
            u64::from(line),
            "receipt line mismatch for {}/{} needle={:?}",
            request.project,
            request.file,
            request.needle
        );
        let receipt_char = receipt.get("character").and_then(Value::as_u64).unwrap_or(u64::MAX);
        assert_eq!(
            receipt_char,
            u64::from(character),
            "receipt character mismatch for {}/{} needle={:?}",
            request.project,
            request.file,
            request.needle
        );

        // Hard assertions #4/#5/#6 (evidence completeness): every receipt
        // must carry live reason/fallback_state/confidence/freshness/
        // fact_source/source_backed_state/latency, not just an author string.
        let evidence = read_live_receipt_evidence(receipt, wall_us).map_err(|e| {
            format!("{}/{} needle={:?}: {e}", request.project, request.file, request.needle)
        })?;

        let results = result.as_ref().and_then(Value::as_array).cloned().unwrap_or_default();
        let result_locations = location_keys(&results);
        let result_count = results.len();

        let expected_keys: Vec<(String, u32, u32)> = request
            .expected_true_occurrences
            .iter()
            .map(|occ| occurrence_location_key(&uri, content, request.needle, *occ))
            .collect::<Result<_, _>>()?;
        let forbidden_keys: Vec<(String, u32, u32)> = request
            .known_false_occurrences
            .iter()
            .map(|occ| occurrence_location_key(&uri, content, request.needle, *occ))
            .collect::<Result<_, _>>()?;

        // Hard assertions #1/#2/#3: whenever the tier claims source-backed
        // exactness, it must never surface a known-false location (this is
        // where the ambiguous/dynamic and (via the sibling stale-index tests
        // below) stale-generation invariants are actually enforced), and for
        // strictly-checked classes the result must equal the curated expected
        // set exactly. Both checks go through the same pure functions unit-
        // tested below with synthetic inputs.
        if evidence.source_backed {
            forbidden_check(&result_locations, &forbidden_keys).map_err(|e| {
                format!(
                    "false-exact: {}/{} needle={:?} (fact_class={:?}): {e}",
                    request.project, request.file, request.needle, request.fact_class
                )
            })?;
            if request.fact_class.is_strictly_checked() {
                exact_match_check(&result_locations, &expected_keys).map_err(|e| {
                    format!(
                        "exact-range mismatch for {}/{} needle={:?} (fact_class={:?}): {e}",
                        request.project, request.file, request.needle, request.fact_class
                    )
                })?;
            }
        }

        // Mechanical proof (not just observation) of the `DeclarationShape`
        // rationale: a `Destructuring` declaration must NEVER reach
        // `semantic_source_backed`, since `line_has_initialized_lexical_declaration`
        // categorically rejects that line shape. If this ever fails, the
        // production recognizer has changed and `DeclarationShape` /
        // `classify_disposition` need to be revisited, not silenced.
        if request.declaration_shape == DeclarationShape::Destructuring && evidence.source_backed {
            return Err(format!(
                "{}/{} needle={:?}: a Destructuring declaration unexpectedly reached \
                 semantic_source_backed -- line_has_initialized_lexical_declaration's shape \
                 gate may have changed; DeclarationShape needs to be revisited",
                request.project, request.file, request.needle
            )
            .into());
        }

        // Same mechanical proof for the policy-excluded request shape: a
        // `LocalLexical` (variable) row sent with `include_declaration: true`
        // must NEVER reach `semantic_source_backed`, since
        // `references.rs::may_use_source_backed_references` only lets
        // variable symbols through when `include_declaration == false`. If
        // this ever fails, the promotion policy has changed and
        // `classify_disposition` needs to be revisited, not silenced.
        if request.fact_class == FactClass::LocalLexical
            && request.include_declaration
            && evidence.source_backed
        {
            return Err(format!(
                "{}/{} needle={:?}: a LocalLexical row with include_declaration=true \
                 unexpectedly reached semantic_source_backed -- \
                 may_use_source_backed_references's policy gate may have changed; \
                 classify_disposition needs to be revisited",
                request.project, request.file, request.needle
            )
            .into());
        }

        let disposition = classify_disposition(
            request.fact_class,
            request.declaration_shape,
            request.include_declaration,
            request.request_shape,
            evidence.source_backed,
            result_count,
            &result_locations,
            &expected_keys,
            lexical_positive_control_evidence,
            subroutine_positive_control_evidence,
            method_positive_control_evidence,
        );

        Ok(ReplayRow {
            project: request.project,
            file: request.file,
            fact_class: request.fact_class,
            needle: request.needle,
            include_declaration: request.include_declaration,
            uri,
            line,
            character,
            answering_tier: evidence.answering_tier,
            source_backed: evidence.source_backed,
            result_count,
            index_result_count: evidence.index_result_count,
            text_result_count: evidence.text_result_count,
            latency_us: evidence.latency_us,
            receipt_reason: evidence.reason,
            receipt_decision: evidence.decision,
            receipt_fallback_state: evidence.fallback_state,
            receipt_confidence: evidence.confidence,
            receipt_freshness: evidence.freshness,
            receipt_fact_source: evidence.fact_source,
            receipt_source_backed_state: evidence.source_backed_state,
            declaration_shape: request.declaration_shape,
            cursor_site: request.cursor_site,
            request_shape: request.request_shape,
            scenario_rationale: request.scenario_rationale,
            disposition,
        })
    }

    fn fire_empty_request(
        server: &LspServer,
        project: &'static str,
        file: &'static str,
        uri: &str,
        line0: usize,
    ) -> Result<ReplayRow, Box<dyn std::error::Error>> {
        let params = json!({
            "textDocument": {"uri": uri},
            "position": {"line": line0, "character": 0},
            "context": {"includeDeclaration": false}
        });

        let t0 = Instant::now();
        let result = server.test_handle_references(Some(params))?;
        let wall_us = u64::try_from(t0.elapsed().as_micros()).unwrap_or(u64::MAX);

        let explanation = explain_provider_decision(server, "references")?;
        let receipt = explanation
            .get("request_receipt")
            .and_then(Value::as_object)
            .ok_or("missing request_receipt")?;

        let receipt_uri = receipt.get("uri").and_then(Value::as_str).unwrap_or("");
        assert_eq!(receipt_uri, uri, "receipt URI mismatch for empty position {project}/{file}");
        let receipt_line = receipt.get("line").and_then(Value::as_u64).unwrap_or(u64::MAX);
        assert_eq!(
            receipt_line,
            u64::try_from(line0)?,
            "receipt line mismatch for empty position {project}/{file}"
        );

        let evidence = read_live_receipt_evidence(receipt, wall_us)
            .map_err(|e| format!("empty position {project}/{file}: {e}"))?;
        assert_eq!(
            evidence.answering_tier, "empty",
            "no-symbol position must yield `empty` tier for {project}/{file}, got {}",
            evidence.answering_tier
        );

        let result_count = result.as_ref().and_then(Value::as_array).map(Vec::len).unwrap_or(0);
        assert_eq!(
            result_count, 0,
            "empty position must produce zero results for {project}/{file}"
        );

        Ok(ReplayRow {
            project,
            file,
            fact_class: FactClass::EmptyPosition,
            needle: "",
            include_declaration: false,
            uri: uri.to_string(),
            line: u32::try_from(line0)?,
            character: 0,
            answering_tier: evidence.answering_tier,
            source_backed: evidence.source_backed,
            result_count,
            index_result_count: evidence.index_result_count,
            text_result_count: evidence.text_result_count,
            latency_us: evidence.latency_us,
            receipt_reason: evidence.reason,
            receipt_decision: evidence.decision,
            receipt_fallback_state: evidence.fallback_state,
            receipt_confidence: evidence.confidence,
            receipt_freshness: evidence.freshness,
            receipt_fact_source: evidence.fact_source,
            receipt_source_backed_state: evidence.source_backed_state,
            declaration_shape: DeclarationShape::NotApplicable,
            cursor_site: CursorSite::NotApplicable,
            request_shape: RequestShape::NotApplicable,
            scenario_rationale: "no_symbol_under_cursor_correctly_empty",
            disposition: "explicit_refusal_safe",
        })
    }

    /// Full row detail, including latency (non-deterministic — informational
    /// only). Printed to the test log for humans; NOT the durable artifact.
    fn row_to_json_full(row: &ReplayRow) -> Value {
        json!({
            "project": row.project,
            "file": row.file,
            "fact_class": row.fact_class.as_str(),
            "needle": row.needle,
            "include_declaration": row.include_declaration,
            "uri": row.uri,
            "line": row.line,
            "character": row.character,
            "answering_tier": row.answering_tier,
            "source_backed": row.source_backed,
            "result_count": row.result_count,
            "index_result_count": row.index_result_count,
            "text_result_count": row.text_result_count,
            "latency_us": row.latency_us,
            "receipt_reason": row.receipt_reason,
            "receipt_decision": row.receipt_decision,
            "receipt_fallback_state": row.receipt_fallback_state,
            "receipt_confidence": row.receipt_confidence,
            "receipt_freshness": row.receipt_freshness,
            "receipt_fact_source": row.receipt_fact_source,
            "receipt_source_backed_state": row.receipt_source_backed_state,
            "declaration_shape": row.declaration_shape.as_str(),
            "cursor_site": row.cursor_site.as_str(),
            "request_shape": row.request_shape.as_str(),
            "scenario_rationale": row.scenario_rationale,
            "disposition": row.disposition,
        })
    }

    /// Deterministic row detail for the checked-in snapshot: everything
    /// EXCEPT `latency_us` (wall-clock, varies run to run — the module doc
    /// comment at the top of this file already establishes latency here is
    /// informational only, never a performance gate).
    fn row_to_json_snapshot(row: &ReplayRow) -> Value {
        json!({
            "project": row.project,
            "file": row.file,
            "fact_class": row.fact_class.as_str(),
            "needle": row.needle,
            "include_declaration": row.include_declaration,
            "uri": row.uri,
            "line": row.line,
            "character": row.character,
            "answering_tier": row.answering_tier,
            "source_backed": row.source_backed,
            "result_count": row.result_count,
            "index_result_count": row.index_result_count,
            "text_result_count": row.text_result_count,
            "receipt_reason": row.receipt_reason,
            "receipt_decision": row.receipt_decision,
            "receipt_fallback_state": row.receipt_fallback_state,
            "receipt_confidence": row.receipt_confidence,
            "receipt_freshness": row.receipt_freshness,
            "receipt_fact_source": row.receipt_fact_source,
            "receipt_source_backed_state": row.receipt_source_backed_state,
            "declaration_shape": row.declaration_shape.as_str(),
            "cursor_site": row.cursor_site.as_str(),
            "request_shape": row.request_shape.as_str(),
            "scenario_rationale": row.scenario_rationale,
            "disposition": row.disposition,
        })
    }

    /// Reproduces the already-proven-in-production positive control
    /// (`references.rs::handle_references_lexical_variable_without_declaration_uses_source_backed_tier`)
    /// against a FRESH single-file server: cursor on a USAGE, not the
    /// declaration, `includeDeclaration: false`. Proves the harness itself
    /// (this test file's helpers, not just production) can observe the
    /// source-backed tier before any conclusion is drawn about real project
    /// code.
    fn assert_single_file_lexical_positive_control() -> Result<(), Box<dyn std::error::Error>> {
        let server = create_server();
        let uri = "file:///control/single-file-scalar-no-decl.pl";
        let text = "my $value = 1;\nmy $other = $value;\n";
        open_document(&server, uri, text)?;

        let params = json!({
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 12},
            "context": {"includeDeclaration": false}
        });
        server.test_handle_references(Some(params))?;
        let explanation = explain_provider_decision(&server, "references")?;
        let receipt = explanation
            .get("request_receipt")
            .and_then(Value::as_object)
            .ok_or("missing request_receipt")?;
        let tier = receipt.get("answering_tier").and_then(Value::as_str).unwrap_or("");
        // Printed (not just asserted) so the observed tier is visible evidence
        // in the test log, not only an implicit pass/fail -- see PR #3998
        // Defect C: this test's own CI reachability is local-only until a
        // `--lib --features workspace` lane exists (the required
        // `Perl LSP Rust Small Result` check only runs two named integration
        // tests, `--test lsp_smoke` and `-p perl-parser --test
        // semantic_smoke_tests`; it never compiles or runs `--lib` unit tests
        // for this crate, so this evidence must be read from a local/manual
        // `cargo test -p perl-lsp-rs --lib --features workspace ...` run).
        eprintln!("single_file_lexical_positive_control observed_tier={tier:?}");
        assert_eq!(
            tier, "semantic_source_backed",
            "single-file lexical positive control must reach the source-backed tier (harness sanity)"
        );
        Ok(())
    }

    /// The SAME lexical positive-control snippet, opened alongside a real,
    /// already-opened multi-file project in the SAME server. Returns whether
    /// it reached `semantic_source_backed` (does NOT panic on failure — see
    /// `observe_multi_file_subroutine_positive_control` for why: whether the
    /// multi-file shape reproduces the tier is itself part of the evidence
    /// this replay collects, not an assumed precondition). If this reaches
    /// `semantic_source_backed`, a zero-activation finding for the project's
    /// OWN `LocalLexical` rows cannot be attributed to "the multi-file
    /// replay-server shape itself breaks the tier" — the shape is proven
    /// capable of activating it. This control is LEXICAL-ONLY: per Defect 2
    /// (PR #3998 fourth review round), it authorizes `coverage_gap` for
    /// `LocalLexical` rows only, NOT `PackageSubSameFile` — a subroutine
    /// reference goes through a different resolution path
    /// (`symbol_is_variable == false`), so a lexical control cannot rule out
    /// a subroutine-specific harness/entity-resolution boundary. See
    /// `observe_multi_file_subroutine_positive_control` for the matching
    /// subroutine-specific control.
    fn observe_multi_file_lexical_positive_control(
        project: &str,
        server: &LspServer,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let control_uri = format!("file:///real_projects/{project}/__lexical_control__.pl");
        let control_text = "my $value = 1;\nmy $other = $value;\n";
        open_document(server, &control_uri, control_text)?;

        let params = json!({
            "textDocument": {"uri": control_uri},
            "position": {"line": 1, "character": 12},
            "context": {"includeDeclaration": false}
        });
        server.test_handle_references(Some(params))?;
        let explanation = explain_provider_decision(server, "references")?;
        let receipt = explanation
            .get("request_receipt")
            .and_then(Value::as_object)
            .ok_or("missing request_receipt")?;
        let tier = receipt.get("answering_tier").and_then(Value::as_str).unwrap_or("");
        // See the single-file control's comment above re: local-only CI
        // reachability (PR #3998 Defect C).
        eprintln!("multi_file_lexical_positive_control project={project} observed_tier={tier:?}");
        Ok(tier == "semantic_source_backed")
    }

    /// The known-good subroutine-reference fixture from
    /// `references.rs::handle_references_include_declaration_true_reaches_source_backed_tier_and_appends_declaration`
    /// (a package with a named sub definition plus two call sites — one bare,
    /// one package-qualified), reproduced against a FRESH single-file server.
    /// Proves the harness can observe `semantic_source_backed` for a
    /// SUBROUTINE reference specifically — a lexical-variable control does
    /// NOT prove this (bareword/sub symbols take a different
    /// `symbol_is_variable == false` path through
    /// `may_use_source_backed_references`/`live_source_backed_reference_locations`).
    const SUBROUTINE_CONTROL_TEXT: &str = concat!(
        "package InclDecl;\n",
        "\n",
        "sub target {\n", // line 2 — declaration site
        "    return 1;\n",
        "}\n",
        "\n",
        "sub caller {\n",
        "    target();\n",           // line 7 — bare call site (cursor here)
        "    InclDecl::target();\n", // line 8 — qualified call site
        "}\n",
        "\n",
        "1;\n",
    );

    fn assert_single_file_subroutine_positive_control() -> Result<(), Box<dyn std::error::Error>> {
        let server = create_server();
        let uri = "file:///control/single-file-subroutine-incl-decl.pl";
        open_document(&server, uri, SUBROUTINE_CONTROL_TEXT)?;

        let params = json!({
            "textDocument": {"uri": uri},
            "position": {"line": 7, "character": 4},
            "context": {"includeDeclaration": true}
        });
        server.test_handle_references(Some(params))?;
        let explanation = explain_provider_decision(&server, "references")?;
        let receipt = explanation
            .get("request_receipt")
            .and_then(Value::as_object)
            .ok_or("missing request_receipt")?;
        let tier = receipt.get("answering_tier").and_then(Value::as_str).unwrap_or("");
        eprintln!("single_file_subroutine_positive_control observed_tier={tier:?}");
        assert_eq!(
            tier, "semantic_source_backed",
            "single-file subroutine positive control must reach the source-backed tier (harness \
             sanity)"
        );
        Ok(())
    }

    /// The SAME subroutine-reference control, opened alongside a real,
    /// already-opened multi-file project in the SAME server. Returns whether
    /// it reached `semantic_source_backed` — does NOT panic on failure.
    /// Required (and must return `true`) before a project's
    /// `PackageSubSameFile` rows may be classified `coverage_gap` — see
    /// Defect 2 in the module doc comment above. This is deliberately
    /// observational rather than a hard assertion: whether the multi-file
    /// shape reproduces the SINGLE-file-proven subroutine tier is itself
    /// evidence this replay collects (a `false` here is a genuine, reportable
    /// finding — a multi-file-shape-specific subroutine confound — not a
    /// harness bug to fail loudly on).
    fn observe_multi_file_subroutine_positive_control(
        project: &str,
        server: &LspServer,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let control_uri = format!("file:///real_projects/{project}/__subroutine_control__.pl");
        open_document(server, &control_uri, SUBROUTINE_CONTROL_TEXT)?;

        let params = json!({
            "textDocument": {"uri": control_uri},
            "position": {"line": 7, "character": 4},
            "context": {"includeDeclaration": true}
        });
        server.test_handle_references(Some(params))?;
        let explanation = explain_provider_decision(server, "references")?;
        let receipt = explanation
            .get("request_receipt")
            .and_then(Value::as_object)
            .ok_or("missing request_receipt")?;
        let tier = receipt.get("answering_tier").and_then(Value::as_str).unwrap_or("");
        eprintln!(
            "multi_file_subroutine_positive_control project={project} observed_tier={tier:?}"
        );
        Ok(tier == "semantic_source_backed")
    }

    /// A variable-receiver, same-file METHOD positive control — the shape
    /// counterpart to `SUBROUTINE_CONTROL_TEXT`. PR #3998 round 6 (maintainer
    /// finding on live review of round 5's fix): the existing subroutine
    /// control only proves the semantic path can resolve a bare/
    /// package-qualified FUNCTION call (`target()` / `InclDecl::target()`).
    /// The real Catalyst `dispatch` request this replay's surviving
    /// `coverage_gap` row names is `$c->dispatch` — a variable-receiver
    /// METHOD call, a distinct request shape the function-call control does
    /// not exercise. Mirrors `SUBROUTINE_CONTROL_TEXT`'s
    /// declaration-plus-call-site shape, but the call site is a method
    /// invocation through a blessed-self receiver (`$self->target_method()`)
    /// rather than a bareword/qualified name.
    const METHOD_CONTROL_TEXT: &str = concat!(
        "package InclDecl;\n",
        "\n",
        "sub new { return bless {}, shift; }\n",
        "\n",
        "sub target_method {\n", // line 4 — declaration site
        "    return 1;\n",
        "}\n",
        "\n",
        "sub caller {\n",
        "    my $self = shift;\n",
        "    $self->target_method();\n", // line 9 — variable-receiver method call (cursor here)
        "}\n",
        "\n",
        "1;\n",
    );

    /// The SAME method-receiver control, opened alongside a real,
    /// already-opened multi-file project in the SAME server. Returns whether
    /// it reached `semantic_source_backed` — does NOT panic on failure, for
    /// the same reason `observe_multi_file_subroutine_positive_control`
    /// doesn't: whether a variable-receiver method call resolves through the
    /// same-file AST-indexed path AT ALL is itself part of the evidence this
    /// replay collects (PR #3998 round 6), not an assumed precondition.
    /// Required (and must return `true`) before a `MethodCall`-shaped
    /// `PackageSubSameFile` row (the Catalyst.pm `dispatch` row) may be
    /// classified `coverage_gap` — see `classify_disposition` and the
    /// module doc comment's round-6 note. If this returns `false`, the
    /// honest disposition for that row is `method_shaped_request_unexercised`,
    /// not `coverage_gap`.
    fn observe_multi_file_method_positive_control(
        project: &str,
        server: &LspServer,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let control_uri = format!("file:///real_projects/{project}/__method_control__.pl");
        open_document(server, &control_uri, METHOD_CONTROL_TEXT)?;
        let (line, character) = position_of(METHOD_CONTROL_TEXT, "target_method(")?;

        let params = json!({
            "textDocument": {"uri": control_uri},
            "position": {"line": line, "character": character},
            "context": {"includeDeclaration": true}
        });
        server.test_handle_references(Some(params))?;
        let explanation = explain_provider_decision(server, "references")?;
        let receipt = explanation
            .get("request_receipt")
            .and_then(Value::as_object)
            .ok_or("missing request_receipt")?;
        let tier = receipt.get("answering_tier").and_then(Value::as_str).unwrap_or("");
        eprintln!("multi_file_method_positive_control project={project} observed_tier={tier:?}");
        Ok(tier == "semantic_source_backed")
    }

    /// Representative-workspace replay for #2674 PR-3 (references measurement,
    /// no live provider behavior change). Fires every request in
    /// `REPLAY_MANIFEST` + `EMPTY_POSITION_MANIFEST` against three real,
    /// committed project fixtures, observes an in-band LEXICAL, SUBROUTINE,
    /// and METHOD positive control per project (see
    /// `observe_multi_file_lexical_positive_control` /
    /// `observe_multi_file_subroutine_positive_control` /
    /// `observe_multi_file_method_positive_control` — these calls are
    /// inline, not separate skippable tests, so removing any of them removes
    /// coverage from THIS governing test; none panics on a `false`
    /// observation, since that is itself collected evidence, not an assumed
    /// precondition — see Defect 2 and the round-6 note in the module doc
    /// comment above), checks the 9 hard assertions described in the module
    /// doc comment above, snapshots a durable receipt via `insta`, and
    /// prints the full (latency-inclusive) receipt with a per-request #1658
    /// disposition.
    #[test]
    fn references_representative_workspace_replay() -> Result<(), Box<dyn std::error::Error>> {
        use std::collections::{BTreeMap, BTreeSet, HashMap};

        let projects = ["mojolicious_skeleton", "dancer2_skeleton", "catalyst_skeleton"];
        let mut project_files: HashMap<&str, Vec<(String, String)>> = HashMap::new();
        let mut servers: HashMap<&str, LspServer> = HashMap::new();
        let mut lexical_positive_control_evidence: HashMap<&str, bool> = HashMap::new();
        let mut subroutine_positive_control_evidence: HashMap<&str, bool> = HashMap::new();
        let mut method_positive_control_evidence: HashMap<&str, bool> = HashMap::new();

        // Global (project-independent) sanity checks first -- these DO panic
        // on failure, since they establish the harness baseline: if a
        // single-file server can't reach `semantic_source_backed` for either
        // shape, nothing downstream in this replay is trustworthy. There is
        // deliberately NO `assert_single_file_method_positive_control` here:
        // whether a variable-receiver method call reaches
        // `semantic_source_backed` at all (even in a fully controlled
        // single-file shape) is itself the round-6 finding under
        // investigation, not an assumed harness precondition -- see
        // `observe_multi_file_method_positive_control`.
        assert_single_file_lexical_positive_control()?;
        assert_single_file_subroutine_positive_control()?;

        for project in projects {
            let server = create_server();
            let files = open_project(&server, project)?;
            // In-band multi-file-shape positive controls: observed inline, so
            // a regression here is visible in THIS governing test's own
            // receipt, not a separate test that could silently bit-rot or be
            // skipped. THREE separate controls (Defect 2, PR #3998 fourth
            // review round, extended by round 6): a lexical-only control does
            // not authorize `coverage_gap` for `PackageSubSameFile` rows,
            // since subroutine references go through a different
            // (`symbol_is_variable == false`) resolution path -- and a
            // FUNCTION-call subroutine control does not authorize
            // `coverage_gap` for a METHOD-shaped `PackageSubSameFile` row
            // either, since method dispatch is a distinct request shape
            // (round 6) -- and empirically the controls do NOT always agree
            // (see the disposition rollup in the receipt).
            let lexical_ok = observe_multi_file_lexical_positive_control(project, &server)?;
            lexical_positive_control_evidence.insert(project, lexical_ok);
            let subroutine_ok = observe_multi_file_subroutine_positive_control(project, &server)?;
            subroutine_positive_control_evidence.insert(project, subroutine_ok);
            let method_ok = observe_multi_file_method_positive_control(project, &server)?;
            method_positive_control_evidence.insert(project, method_ok);
            project_files.insert(project, files);
            servers.insert(project, server);
        }

        let mut rows: Vec<ReplayRow> = Vec::new();
        for request in REPLAY_MANIFEST {
            let server = servers.get(request.project).ok_or("missing server for project")?;
            let files = project_files.get(request.project).ok_or("missing files for project")?;
            let lexical_evidence =
                lexical_positive_control_evidence.get(request.project).copied().unwrap_or(false);
            let subroutine_evidence =
                subroutine_positive_control_evidence.get(request.project).copied().unwrap_or(false);
            let method_evidence =
                method_positive_control_evidence.get(request.project).copied().unwrap_or(false);
            rows.push(fire_replay_request(
                server,
                request,
                files,
                lexical_evidence,
                subroutine_evidence,
                method_evidence,
            )?);
        }
        for (project, file, line0) in EMPTY_POSITION_MANIFEST.iter().copied() {
            let server = servers.get(project).ok_or("missing server for project")?;
            let uri = project_uri(project, file);
            rows.push(fire_empty_request(server, project, file, &uri, line0)?);
        }

        // Hard assertion #9: the replay emits a non-empty durable receipt.
        assert!(!rows.is_empty(), "replay must emit at least one row");

        // Hard assertion #8: every declared project and fact-class bucket appears.
        let projects_seen: BTreeSet<&str> = rows.iter().map(|r| r.project).collect();
        assert_eq!(
            projects_seen,
            BTreeSet::from(["mojolicious_skeleton", "dancer2_skeleton", "catalyst_skeleton"]),
            "replay must cover every declared project"
        );
        let classes_seen: BTreeSet<&str> = rows.iter().map(|r| r.fact_class.as_str()).collect();
        assert_eq!(
            classes_seen,
            BTreeSet::from([
                "local_lexical",
                "package_sub_same_file",
                "cross_file_sub",
                "imported_symbol",
                "dynamic_ambiguous",
                "empty_position",
            ]),
            "replay must cover every declared fact-class bucket"
        );

        // Hard assertion #4: every empty success is proven correct (`empty`
        // tier) or carries a live, non-empty receipt reason/fallback_state
        // (already enforced per-row by `validate_receipt_has_required_fields`
        // inside `fire_replay_request`/`fire_empty_request`; re-asserted here
        // over the aggregated rows for a single top-level failure message).
        for row in &rows {
            if row.result_count == 0 {
                assert!(
                    row.answering_tier == "empty" || !row.receipt_reason.is_empty(),
                    "unexplained empty result for {}/{} needle={:?}",
                    row.project,
                    row.file,
                    row.needle
                );
            }
        }

        // Full (latency-inclusive) receipt: printed for humans, not the
        // durable artifact.
        let full_receipt = json!({
            "schema_version": 2,
            "claim_boundary": "This PR measures current references behavior across a declared \
                representative workspace corpus, verifies exactness and honest degradation, and \
                identifies where the request-time text scan is removable, required by an \
                unresolved coverage gap, or replaceable by explicit refusal. It does not alter \
                live provider behavior or claim real user-traffic weighting.",
            "projects": projects,
            "lexical_positive_control_evidence": lexical_positive_control_evidence,
            "subroutine_positive_control_evidence": subroutine_positive_control_evidence,
            "method_positive_control_evidence": method_positive_control_evidence,
            "request_count": rows.len(),
            "rows": rows.iter().map(row_to_json_full).collect::<Vec<_>>(),
        });
        eprintln!(
            "references_representative_replay_receipt={}",
            serde_json::to_string_pretty(&full_receipt)?
        );

        let mut by_class: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for row in &rows {
            by_class.entry(row.fact_class.as_str()).or_default().push(row.disposition);
        }
        eprintln!("--- #1658 disposition rollup by fact-class bucket ---");
        for (class, dispositions) in &by_class {
            eprintln!("  {class:<24} {dispositions:?}");
        }

        // Durable receipt: a deterministic (latency-excluded) snapshot,
        // checked into git under `snapshots/`, validated by every test run —
        // not just visible under `--nocapture`. Any drift (a new project, a
        // changed disposition, a newly-reached source-backed row) fails this
        // test until the snapshot is reviewed and re-accepted.
        //
        // `method_positive_control_evidence` is checked in HERE (round 6,
        // per-project sorted into a `BTreeMap` for deterministic key order)
        // so the method control's observed activation can never silently
        // regress without the snapshot flagging it for review.
        let method_positive_control_evidence_sorted: BTreeMap<&str, bool> =
            method_positive_control_evidence.iter().map(|(k, v)| (*k, *v)).collect();
        let snapshot_receipt = json!({
            "schema_version": 2,
            "projects": projects,
            "method_positive_control_evidence": method_positive_control_evidence_sorted,
            "request_count": rows.len(),
            "rows": rows.iter().map(row_to_json_snapshot).collect::<Vec<_>>(),
        });
        insta::assert_snapshot!(
            "references_representative_replay_receipt",
            serde_json::to_string_pretty(&snapshot_receipt)?
        );

        Ok(())
    }

    /// Hard assertion #2, STRUCTURAL half (stale-generation requests produce
    /// ZERO false-exact answers), proven mechanically rather than merely
    /// observed: the `semantic_source_backed` tier is only reachable inside
    /// the `IndexAccessMode::Full(coordinator)` branch of
    /// `handle_references_inner` (see
    /// `crates/perl-lsp-rs/src/runtime/language/references.rs`). A
    /// Building/partial-index coordinator — the same stand-in the routing
    /// matrix above uses for an index that has not caught up — makes that
    /// whole branch structurally unreachable, so it can never emit a
    /// false-exact answer regardless of what the request would otherwise
    /// find. This does NOT reproduce a genuine stale-generation window (an
    /// open document newer than its own index entry); see
    /// `references_representative_replay_genuine_stale_generation_downgrades_index_state`
    /// below for that.
    #[test]
    fn references_representative_replay_stale_index_never_source_backed(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut server = create_server();
        let files = open_project(&server, "mojolicious_skeleton")?;
        set_index_building(&mut server);

        let content = fixture_content(&files, "lib/Mojolicious.pm")?;
        let uri = project_uri("mojolicious_skeleton", "lib/Mojolicious.pm");
        let (line, character) = occurrence_position(content, "$plugins", 1)?;
        let params = json!({
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": character},
            "context": {"includeDeclaration": false}
        });
        server.test_handle_references(Some(params))?;

        let explanation = explain_provider_decision(&server, "references")?;
        let receipt = explanation
            .get("request_receipt")
            .and_then(Value::as_object)
            .ok_or("missing request_receipt")?;
        let answering_tier =
            receipt.get("answering_tier").and_then(Value::as_str).unwrap_or("unknown");
        let source_backed = receipt.get("source_backed").and_then(Value::as_bool).unwrap_or(false);

        assert_ne!(
            answering_tier, "semantic_source_backed",
            "a partial-index (Building) state must not reach the source-backed tier"
        );
        assert!(
            !source_backed,
            "a partial-index (Building) state must not report source_backed=true"
        );

        Ok(())
    }

    /// Hard assertion #2, GENUINE half: reproduces an actual stale-generation
    /// window — the open document's generation is newer than the workspace
    /// index's recorded generation for that same document — using the exact
    /// helper sequence already proven in
    /// `references.rs::make_document_index_stale`
    /// (`test_index_file_in_building_state` -> `test_simulate_indexing_complete`
    /// -> `test_replace_document_without_index`), applied to real project
    /// content instead of a synthetic snippet. Confirms the live downgrade via
    /// `index_state` (the receipt's `freshness` field is currently a
    /// hardcoded `"fresh"` constant in production and does not vary with
    /// staleness — see the `ReplayRow::receipt_freshness` doc comment; this
    /// replay cannot and does not assert a freshness *value* change, only the
    /// `index_state` downgrade and the resulting inability to reach
    /// `semantic_source_backed`).
    #[test]
    fn references_representative_replay_genuine_stale_generation_downgrades_index_state(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let server = create_server();
        let files = open_project(&server, "mojolicious_skeleton")?;
        let content = fixture_content(&files, "lib/Mojolicious.pm")?;
        let uri = project_uri("mojolicious_skeleton", "lib/Mojolicious.pm");

        server.test_index_file_in_building_state(&uri, content).map_err(|e| e.to_string())?;
        server.test_simulate_indexing_complete();
        server.test_replace_document_without_index(&uri, content, 2).map_err(|e| e.to_string())?;
        assert!(
            server.workspace_index_stale_for_document(&uri),
            "test setup must leave the open document newer than its workspace index generation"
        );

        let (line, character) = occurrence_position(content, "$plugins", 1)?;
        let params = json!({
            "textDocument": {"uri": uri, "version": 2},
            "position": {"line": line, "character": character},
            "context": {"includeDeclaration": false}
        });
        server.test_handle_references(Some(params))?;

        let explanation = explain_provider_decision(&server, "references")?;
        let receipt = explanation
            .get("request_receipt")
            .and_then(Value::as_object)
            .ok_or("missing request_receipt")?;

        assert_eq!(
            receipt.get("index_state").and_then(Value::as_str),
            Some("none"),
            "a genuinely stale open-document generation must downgrade references index access"
        );
        assert_eq!(
            receipt.get("source_backed").and_then(Value::as_bool),
            Some(false),
            "a genuinely stale open-document generation must not report source_backed=true"
        );
        assert_ne!(
            receipt.get("answering_tier").and_then(Value::as_str),
            Some("semantic_source_backed"),
            "a genuinely stale open-document generation must not reach the source-backed tier"
        );

        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Mutation tests: prove the comparison/validation logic above is
    // discriminating, independent of whether any live request currently
    // reaches the `semantic_source_backed` tier. Each of these directly
    // "revert-proves" one of the PR #3998 review's repair requirements.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn exact_match_check_accepts_correct_set_regardless_of_order(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let key = |line: u32, character: u32| ("file:///t.pl".to_string(), line, character);
        let actual = vec![key(1, 12), key(2, 5)];
        let expected = vec![key(2, 5), key(1, 12)];
        exact_match_check(&actual, &expected).map_err(|e| e.into())
    }

    /// Revert-proves the PR #3998 review's decisive finding: if a declaration
    /// occurrence were reintroduced into a decl-excluding expected set, this
    /// comparison MUST reject it as a mismatch.
    #[test]
    fn exact_match_check_rejects_declaration_reintroduced_into_expected(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let key = |line: u32, character: u32| ("file:///t.pl".to_string(), line, character);
        // `actual` = a correct includeDeclaration:false result (usage only).
        let actual = vec![key(1, 12)];
        // `expected` mistakenly includes the declaration back in.
        let expected = vec![key(0, 3), key(1, 12)];
        let result = exact_match_check(&actual, &expected);
        if result.is_ok() {
            return Err("reintroducing a declaration into expected must be caught".into());
        }
        Ok(())
    }

    #[test]
    fn forbidden_check_accepts_clean_result() -> Result<(), Box<dyn std::error::Error>> {
        let key = |line: u32, character: u32| ("file:///t.pl".to_string(), line, character);
        let actual = vec![key(1, 12)];
        let forbidden = vec![key(5, 0)];
        forbidden_check(&actual, &forbidden).map_err(|e| e.into())
    }

    /// Revert-proves: adding a known-false location into the actual result
    /// (e.g. an over-broad index hit) must prevent parity.
    #[test]
    fn forbidden_check_rejects_known_false_location_present(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let key = |line: u32, character: u32| ("file:///t.pl".to_string(), line, character);
        let actual = vec![key(1, 12), key(5, 0)];
        let forbidden = vec![key(5, 0)];
        let result = forbidden_check(&actual, &forbidden);
        if result.is_ok() {
            return Err(
                "a known-false location present in the actual result must be rejected".into()
            );
        }
        Ok(())
    }

    #[test]
    fn validate_receipt_has_required_fields_accepts_complete_receipt(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let receipt = json!({
            "decision": "acted",
            "reason": "live_provider_result",
            "fallback_state": "live_provider",
            "confidence": "high",
            "freshness": "fresh",
            "fact_source": "semantic_fact",
            "source_backed_state": "semantic_source_backed_ast_index",
            "answering_tier": "semantic_source_backed",
            "index_state": "full",
            "latency_us": 42,
        });
        let object = receipt.as_object().ok_or("receipt must be an object literal")?;
        validate_receipt_has_required_fields(object).map_err(|e| e.into())
    }

    /// Revert-proves: a receipt missing its live `reason` field must fail
    /// validation rather than silently passing with an author-supplied
    /// stand-in.
    #[test]
    fn validate_receipt_has_required_fields_rejects_missing_reason(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let receipt = json!({
            "decision": "acted",
            "fallback_state": "live_provider",
            "confidence": "high",
            "freshness": "fresh",
            "fact_source": "semantic_fact",
            "source_backed_state": "semantic_source_backed_ast_index",
            "answering_tier": "semantic_source_backed",
            "index_state": "full",
            "latency_us": 42,
        });
        let object = receipt.as_object().ok_or("receipt must be an object literal")?;
        let result = validate_receipt_has_required_fields(object);
        if result.is_ok() {
            return Err("a receipt missing `reason` must be rejected".into());
        }
        Ok(())
    }

    /// Revert-proves: removing an in-band positive control (i.e. its
    /// evidence flag never gets set) must downgrade `LocalLexical`
    /// dispositions to `unexercised`, not silently keep reporting
    /// `coverage_gap` without justifying evidence.
    #[test]
    fn classify_disposition_requires_positive_control_evidence_for_coverage_gap() {
        let empty_locations: Vec<(String, u32, u32)> = Vec::new();
        let without_control = classify_disposition(
            FactClass::LocalLexical,
            DeclarationShape::SimpleInit,
            false,
            RequestShape::NotApplicable,
            false,
            8,
            &empty_locations,
            &empty_locations,
            false,
            false,
            false,
        );
        assert_eq!(
            without_control, "unexercised",
            "without positive-control evidence, a non-source-backed LocalLexical row must be \
             `unexercised`, not `coverage_gap`"
        );

        let with_control = classify_disposition(
            FactClass::LocalLexical,
            DeclarationShape::SimpleInit,
            false,
            RequestShape::NotApplicable,
            false,
            8,
            &empty_locations,
            &empty_locations,
            true,
            false,
            false,
        );
        assert_eq!(
            with_control, "coverage_gap",
            "with lexical positive-control evidence, a non-source-backed SimpleInit \
             include_declaration=false LocalLexical row is `coverage_gap`"
        );
    }

    /// Revert-proves Defect 2 (PR #3998 fourth review round): a LEXICAL-only
    /// positive control must NOT authorize `coverage_gap` for
    /// `PackageSubSameFile` rows -- subroutine references take a different
    /// resolution path, so lexical evidence alone leaves package-sub rows
    /// `unexercised` until a MATCHING subroutine control also passes.
    #[test]
    fn classify_disposition_requires_subroutine_specific_evidence_for_package_sub_coverage_gap() {
        let empty_locations: Vec<(String, u32, u32)> = Vec::new();
        let lexical_only = classify_disposition(
            FactClass::PackageSubSameFile,
            DeclarationShape::NotApplicable,
            true,
            RequestShape::FunctionCall,
            false,
            2,
            &empty_locations,
            &empty_locations,
            true,
            false,
            false,
        );
        assert_eq!(
            lexical_only, "unexercised",
            "lexical-only positive-control evidence must not authorize coverage_gap for \
             PackageSubSameFile rows"
        );

        let with_subroutine_control = classify_disposition(
            FactClass::PackageSubSameFile,
            DeclarationShape::NotApplicable,
            true,
            RequestShape::FunctionCall,
            false,
            2,
            &empty_locations,
            &empty_locations,
            true,
            true,
            false,
        );
        assert_eq!(
            with_subroutine_control, "coverage_gap",
            "with matching subroutine positive-control evidence, a non-source-backed \
             PackageSubSameFile row is coverage_gap"
        );
    }

    /// Revert-proves the round-6 fix (PR #3998 sixth review round, live
    /// maintainer finding): a FUNCTION-call positive control must NOT
    /// authorize `coverage_gap` for a `MethodCall`-shaped `PackageSubSameFile`
    /// row (the Catalyst.pm `dispatch` row's real shape, `$c->dispatch`) --
    /// method dispatch is a distinct request shape from bareword/qualified
    /// function-name resolution, so the function-call control's activation
    /// alone must leave a method-shaped row `method_shaped_request_unexercised`
    /// until a MATCHING method-shaped control also passes. A
    /// `PackageSubSameFile` row whose function-call control never activated
    /// at all stays the plain `unexercised` bucket regardless of shape,
    /// unaffected by this check.
    #[test]
    fn classify_disposition_requires_method_specific_evidence_for_method_shaped_package_sub_coverage_gap(
    ) {
        let empty_locations: Vec<(String, u32, u32)> = Vec::new();

        // Function-call control activated, but the method-shaped control did
        // NOT: this is exactly the pre-round-6 confound (the Catalyst.pm
        // `dispatch` row's real, pre-fix shape) -- must not be `coverage_gap`.
        let function_control_only = classify_disposition(
            FactClass::PackageSubSameFile,
            DeclarationShape::NotApplicable,
            true,
            RequestShape::MethodCall,
            false,
            1,
            &empty_locations,
            &empty_locations,
            true,
            true,
            false,
        );
        assert_eq!(
            function_control_only, "method_shaped_request_unexercised",
            "a FUNCTION-call-only positive control must not authorize coverage_gap for a \
             MethodCall-shaped PackageSubSameFile row"
        );

        // With a MATCHING method-shaped control also passing, coverage_gap is
        // authorized.
        let with_method_control = classify_disposition(
            FactClass::PackageSubSameFile,
            DeclarationShape::NotApplicable,
            true,
            RequestShape::MethodCall,
            false,
            1,
            &empty_locations,
            &empty_locations,
            true,
            true,
            true,
        );
        assert_eq!(
            with_method_control, "coverage_gap",
            "with a matching method-shaped positive control also passing, a non-source-backed \
             MethodCall-shaped PackageSubSameFile row is coverage_gap"
        );

        // Neither control activated: stays the plain `unexercised` bucket,
        // not `method_shaped_request_unexercised` -- that label is reserved
        // for the specific "would-have-been-wrongly-authorized" case.
        let neither_control = classify_disposition(
            FactClass::PackageSubSameFile,
            DeclarationShape::NotApplicable,
            true,
            RequestShape::MethodCall,
            false,
            1,
            &empty_locations,
            &empty_locations,
            true,
            false,
            false,
        );
        assert_eq!(
            neither_control, "unexercised",
            "when the function-call control never activated either, a MethodCall-shaped \
             PackageSubSameFile row stays the plain unexercised bucket"
        );
    }

    /// Revert-proves Defect 1 (PR #3998 fourth review round): a `LocalLexical`
    /// row sent with `include_declaration: true` must ALWAYS classify as
    /// `policy_excluded_request_shape` -- never `coverage_gap` -- regardless
    /// of positive-control evidence, because
    /// `references.rs::may_use_source_backed_references` categorically
    /// excludes variable requests with `include_declaration: true` from the
    /// promoted slice.
    #[test]
    fn classify_disposition_include_declaration_true_lexical_row_is_never_coverage_gap() {
        let empty_locations: Vec<(String, u32, u32)> = Vec::new();
        for lexical_positive_control_evidence in [false, true] {
            let disposition = classify_disposition(
                FactClass::LocalLexical,
                DeclarationShape::SimpleInit,
                true,
                RequestShape::NotApplicable,
                false,
                3,
                &empty_locations,
                &empty_locations,
                lexical_positive_control_evidence,
                false,
                false,
            );
            assert_eq!(
                disposition, "policy_excluded_request_shape",
                "an include_declaration=true LocalLexical row must be \
                 policy_excluded_request_shape regardless of \
                 lexical_positive_control_evidence={lexical_positive_control_evidence}"
            );
        }
    }

    /// Revert-proves: a noisy non-empty fallback answer for
    /// cross-file/imported/ambiguous classes must NOT be called
    /// `explicit_refusal_safe` — only a genuinely empty (exercised refusal)
    /// result qualifies.
    #[test]
    fn classify_disposition_rejects_noisy_fallback_as_refusal_safe() {
        let empty_locations: Vec<(String, u32, u32)> = Vec::new();
        let noisy = classify_disposition(
            FactClass::DynamicAmbiguous,
            DeclarationShape::NotApplicable,
            false,
            RequestShape::NotApplicable,
            false,
            12,
            &empty_locations,
            &empty_locations,
            false,
            false,
            false,
        );
        assert_eq!(
            noisy, "unclassified",
            "a non-empty (noisy) fallback answer must not be called explicit_refusal_safe"
        );

        let genuinely_empty = classify_disposition(
            FactClass::DynamicAmbiguous,
            DeclarationShape::NotApplicable,
            false,
            RequestShape::NotApplicable,
            false,
            0,
            &empty_locations,
            &empty_locations,
            false,
            false,
            false,
        );
        assert_eq!(
            genuinely_empty, "explicit_refusal_safe",
            "a genuinely empty (exercised refusal) fallback answer is explicit_refusal_safe"
        );
    }

    /// Revert-proves the Defect A repair (PR #3998 second review round): a
    /// `Destructuring` declaration shape must ALWAYS classify as
    /// `unsupported_declaration_shape` -- never `coverage_gap` -- regardless
    /// of `source_backed`/positive-control evidence, because non-activation
    /// for that shape is expected-by-design
    /// (`line_has_initialized_lexical_declaration` categorically rejects it),
    /// not an entity-linking gap.
    #[test]
    fn classify_disposition_destructuring_shape_is_never_coverage_gap() {
        let empty_locations: Vec<(String, u32, u32)> = Vec::new();
        for lexical_positive_control_evidence in [false, true] {
            let disposition = classify_disposition(
                FactClass::LocalLexical,
                DeclarationShape::Destructuring,
                false,
                RequestShape::NotApplicable,
                false,
                8,
                &empty_locations,
                &empty_locations,
                lexical_positive_control_evidence,
                false,
                false,
            );
            assert_eq!(
                disposition, "unsupported_declaration_shape",
                "a Destructuring LocalLexical row must be unsupported_declaration_shape \
                 regardless of lexical_positive_control_evidence={lexical_positive_control_evidence}"
            );
        }
    }
}
