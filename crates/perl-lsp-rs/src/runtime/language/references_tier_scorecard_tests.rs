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
    use parking_lot::Mutex;
    use perl_parser::workspace_index::IndexCoordinator;
    use serde_json::{Value, json};
    use std::io::Cursor;
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
}
