#!/usr/bin/env bash
# Test suite for scripts/ci/check-pr-review-convergence
#
# Regression coverage for issue #3679: the script previously treated
# unresolved-but-OUTDATED review threads as ADVISORY (non-blocking), but
# this repo's live 'main' branch-protection ruleset enforces
# required_conversation_resolution, which blocks on EVERY unresolved
# thread regardless of isOutdated. PR #3621 proved this directly (sat
# BLOCKED with 0 active threads but 9 outdated-unresolved threads; merge
# fired immediately once those 9 were resolved).
#
# This suite exercises the script fully offline via the
# CONVERGENCE_TEST_FIXTURE_DIR test seam (see the script's own header
# comment for the fixture file contract: pr_view.json, latestReviews.json,
# reviewThreads.json). No `gh` network call is made.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPT="$SCRIPT_DIR/../ci/check-pr-review-convergence"
FIXTURES_ROOT="$SCRIPT_DIR/../ci/fixtures/convergence"
PASS_COUNT=0
FAIL_COUNT=0

pass() { printf 'PASS %s\n' "$1"; PASS_COUNT=$((PASS_COUNT + 1)); }
fail() { printf 'FAIL %s\n' "$1"; FAIL_COUNT=$((FAIL_COUNT + 1)); }

if [[ ! -f "$SCRIPT" ]]; then
    echo "ERROR: check-pr-review-convergence not found at $SCRIPT"
    exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
    echo "ERROR: jq not found on PATH — required to run this test suite"
    exit 1
fi

# ── Helper: run the script against a named fixture case ────────────────────
# Prints "<exit_code>\n<stdout>" via globals so callers can inspect both.
RUN_EXIT=0
RUN_STDOUT=""
run_case() {
    local case_name="$1"
    local fixture_dir="$FIXTURES_ROOT/$case_name"

    if [[ ! -d "$fixture_dir" ]]; then
        echo "ERROR: fixture dir missing: $fixture_dir" >&2
        exit 1
    fi

    RUN_EXIT=0
    RUN_STDOUT="$(CONVERGENCE_TEST_FIXTURE_DIR="$fixture_dir" bash "$SCRIPT" 9999 "test-owner/test-repo" 2>/dev/null)" || RUN_EXIT=$?
}

# Same as run_case but with REVIEW_PROTOCOL_ENFORCE=1 — promotes the R1
# protocol axes (#3693) from advisory to hard BLOCK. The advisory-vs-enforce
# split is itself under test: run_case (advisory default) proves the new axes
# do NOT block; run_case_enforce proves they DO block once the flag flips.
run_case_enforce() {
    local case_name="$1"
    local fixture_dir="$FIXTURES_ROOT/$case_name"

    if [[ ! -d "$fixture_dir" ]]; then
        echo "ERROR: fixture dir missing: $fixture_dir" >&2
        exit 1
    fi

    RUN_EXIT=0
    RUN_STDOUT="$(CONVERGENCE_TEST_FIXTURE_DIR="$fixture_dir" REVIEW_PROTOCOL_ENFORCE=1 bash "$SCRIPT" 9999 "test-owner/test-repo" 2>/dev/null)" || RUN_EXIT=$?
}

json_field() {
    # $1 = stdout blob, $2 = jq filter. The script emits `jq -n` output
    # (pretty-printed, multi-line) as the tail of stdout — extract
    # everything from the first line starting with '{' onward so this
    # works regardless of jq's default formatting.
    printf '%s' "$1" | sed -n '/^{/,$p' | jq -r "$2"
}

# ── Test 1: outdated-unresolved thread BLOCKS (THE regression) ─────────────
# 1 unresolved+OUTDATED thread, 0 active, no pending reviewers, no stale
# human reviews. Before the fix this incorrectly reported ADVISORY/exit 0.

test_outdated_unresolved_blocks() {
    run_case "outdated-unresolved-blocks"

    local converged unresolved_total unresolved_outdated unresolved_active
    converged="$(json_field "$RUN_STDOUT" '.converged')"
    unresolved_total="$(json_field "$RUN_STDOUT" '.unresolved_total')"
    unresolved_outdated="$(json_field "$RUN_STDOUT" '.unresolved_outdated')"
    unresolved_active="$(json_field "$RUN_STDOUT" '.unresolved_active')"

    if [[ "$RUN_EXIT" -eq 1 && "$converged" == "false" && \
          "$unresolved_total" -eq 1 && "$unresolved_outdated" -eq 1 && \
          "$unresolved_active" -eq 0 ]]; then
        pass "outdated-unresolved thread blocks convergence (exit 1, converged:false, unresolved_total=1, unresolved_outdated=1)"
    else
        fail "outdated-unresolved thread should block — got exit=$RUN_EXIT converged=$converged unresolved_total=$unresolved_total unresolved_outdated=$unresolved_outdated unresolved_active=$unresolved_active"
    fi
}

# ── Test 2: active-unresolved thread BLOCKS (guard the other direction) ────
# 1 active thread, 0 outdated. Confirms the pre-existing active-thread
# blocking behavior was not broken by the fix.

test_active_unresolved_blocks() {
    run_case "active-unresolved-blocks"

    local converged unresolved_total unresolved_active unresolved_outdated
    converged="$(json_field "$RUN_STDOUT" '.converged')"
    unresolved_total="$(json_field "$RUN_STDOUT" '.unresolved_total')"
    unresolved_active="$(json_field "$RUN_STDOUT" '.unresolved_active')"
    unresolved_outdated="$(json_field "$RUN_STDOUT" '.unresolved_outdated')"

    if [[ "$RUN_EXIT" -eq 1 && "$converged" == "false" && \
          "$unresolved_total" -eq 1 && "$unresolved_active" -eq 1 && \
          "$unresolved_outdated" -eq 0 ]]; then
        pass "active-unresolved thread blocks convergence (exit 1, converged:false, unresolved_total=1, unresolved_active=1)"
    else
        fail "active-unresolved thread should block — got exit=$RUN_EXIT converged=$converged unresolved_total=$unresolved_total unresolved_active=$unresolved_active unresolved_outdated=$unresolved_outdated"
    fi
}

# ── Test 3: all threads resolved converges ──────────────────────────────────
# 0 unresolved threads (mix of resolved-active-shape and
# resolved-outdated-shape), no pending reviewers, no stale human reviews.

test_all_resolved_converges() {
    run_case "all-resolved-converges"

    local converged unresolved_total resolved_threads
    converged="$(json_field "$RUN_STDOUT" '.converged')"
    unresolved_total="$(json_field "$RUN_STDOUT" '.unresolved_total')"
    resolved_threads="$(json_field "$RUN_STDOUT" '.resolved_threads')"

    if [[ "$RUN_EXIT" -eq 0 && "$converged" == "true" && \
          "$unresolved_total" -eq 0 && "$resolved_threads" -eq 2 ]]; then
        pass "all threads resolved converges (exit 0, converged:true, unresolved_total=0, resolved_threads=2)"
    else
        fail "all resolved should converge — got exit=$RUN_EXIT converged=$converged unresolved_total=$unresolved_total resolved_threads=$resolved_threads"
    fi
}

# ── Test 4: stale bot review stays ADVISORY, does not block ────────────────
# Regression guard for the #3621 bot-staleness fix — this PR's change must
# NOT touch that exclusion. A stale bot review + fresh human review + zero
# threads should still converge.

test_stale_bot_review_advisory_only() {
    run_case "stale-bot-advisory-only"

    local converged stale_bot_count
    converged="$(json_field "$RUN_STDOUT" '.converged')"
    stale_bot_count="$(json_field "$RUN_STDOUT" '.stale_bot_reviews | length')"

    if [[ "$RUN_EXIT" -eq 0 && "$converged" == "true" && "$stale_bot_count" -eq 1 ]]; then
        pass "stale bot review stays advisory, does not block (exit 0, converged:true, stale_bot_reviews has 1 entry)"
    else
        fail "stale bot review should stay advisory — got exit=$RUN_EXIT converged=$converged stale_bot_count=$stale_bot_count"
    fi
}

# ── Test 5: BLOCK line emitted on stderr for the outdated case ─────────────
# The verdict alone isn't enough — a human/agent reading stderr must see a
# BLOCK (not ADVISORY) line for the outdated thread.

test_outdated_case_emits_block_line() {
    local fixture_dir="$FIXTURES_ROOT/outdated-unresolved-blocks"
    local stderr_output
    stderr_output="$(CONVERGENCE_TEST_FIXTURE_DIR="$fixture_dir" bash "$SCRIPT" 9999 "test-owner/test-repo" 2>&1 1>/dev/null)" || true

    if echo "$stderr_output" | grep -q "^BLOCK.*outdated"; then
        pass "outdated-unresolved thread emits a BLOCK line (not ADVISORY)"
    else
        fail "expected a BLOCK line mentioning 'outdated' — got: $stderr_output"
    fi
}

# ── Test 6: missing fixture directory fails with usage/fetch error (exit 2) ─

test_missing_fixture_dir_errors() {
    local code
    code=0
    CONVERGENCE_TEST_FIXTURE_DIR="$FIXTURES_ROOT/does-not-exist" bash "$SCRIPT" 9999 "test-owner/test-repo" >/dev/null 2>&1 || code=$?

    if [[ "$code" -eq 2 ]]; then
        pass "missing fixture file errors with exit 2 (usage/fetch error)"
    else
        fail "missing fixture file — expected exit 2, got $code"
    fi
}

# ── Test 7: resolved thread with NO reply BLOCKS (#3693, resolved-to-clear) ─
# A resolved thread whose comments connection has totalCount == 1 (only the
# original review comment, no reply from anyone) is the mechanical
# signature of the #3647 incident: 15 threads resolved with zero evidence.

test_resolved_without_disposition_blocks() {
    run_case "resolved-without-disposition-blocks"

    local converged resolved_without_disposition
    converged="$(json_field "$RUN_STDOUT" '.converged')"
    resolved_without_disposition="$(json_field "$RUN_STDOUT" '.resolved_without_disposition')"

    if [[ "$RUN_EXIT" -eq 1 && "$converged" == "false" && \
          "$resolved_without_disposition" -ge 1 ]]; then
        pass "resolved thread with no reply blocks convergence (exit 1, converged:false, resolved_without_disposition>=1)"
    else
        fail "resolved-without-disposition thread should block — got exit=$RUN_EXIT converged=$converged resolved_without_disposition=$resolved_without_disposition"
    fi
}

# ── Test 8: resolved thread WITH a disposition reply does NOT trip the block

test_resolved_with_disposition_does_not_block() {
    run_case "resolved-with-disposition-ok"

    local converged resolved_without_disposition
    converged="$(json_field "$RUN_STDOUT" '.converged')"
    resolved_without_disposition="$(json_field "$RUN_STDOUT" '.resolved_without_disposition')"

    if [[ "$RUN_EXIT" -eq 0 && "$converged" == "true" && \
          "$resolved_without_disposition" -eq 0 ]]; then
        pass "resolved thread with a disposition reply does not block (exit 0, converged:true, resolved_without_disposition=0)"
    else
        fail "resolved-with-disposition thread should NOT block — got exit=$RUN_EXIT converged=$converged resolved_without_disposition=$resolved_without_disposition"
    fi
}

# ── Test 9: 'needs-deep-review' label BLOCKS regardless of thread state ────
# Makes an in-flight independent review mechanically visible — the #3647
# hole was that the review existed only in an orchestrator's task list.

test_pending_independent_review_blocks() {
    run_case "pending-independent-review-blocks"

    local converged independent_review_pending
    converged="$(json_field "$RUN_STDOUT" '.converged')"
    independent_review_pending="$(json_field "$RUN_STDOUT" '.independent_review_pending')"

    if [[ "$RUN_EXIT" -eq 1 && "$converged" == "false" && \
          "$independent_review_pending" == "true" ]]; then
        pass "'needs-deep-review' label blocks convergence (exit 1, converged:false, independent_review_pending:true)"
    else
        fail "needs-deep-review label should block — got exit=$RUN_EXIT converged=$converged independent_review_pending=$independent_review_pending"
    fi
}

# ══ R1 protocol-axis tests (#3693) ═════════════════════════════════════════
# Design: each new axis is ADVISORY by default and BLOCK under
# REVIEW_PROTOCOL_ENFORCE=1. So each axis has (at least) an enforce test
# proving it blocks, and the advisory-first contract itself is proven by
# tests 11 + 20 (same fixture converges under the default, WARNs not BLOCKs).

# ── Test 10: prose-only reply (no marker) BLOCKS under enforce ─────────────
test_prose_reply_blocks_under_enforce() {
    run_case_enforce "resolved-prose-reply-only-blocks"
    local converged missing
    converged="$(json_field "$RUN_STDOUT" '.converged')"
    missing="$(json_field "$RUN_STDOUT" '.dispositions_missing_marker')"
    if [[ "$RUN_EXIT" -eq 1 && "$converged" == "false" && "$missing" -ge 1 ]]; then
        pass "prose-only reply (no disposition marker) blocks under enforce (exit 1, dispositions_missing_marker>=1)"
    else
        fail "prose-only reply should block under enforce — got exit=$RUN_EXIT converged=$converged dispositions_missing_marker=$missing"
    fi
}

# ── Test 11: SAME fixture is ADVISORY (exit 0) under the default ───────────
# Proves the advisory-first rollout: the new marker-content axis reports but
# does NOT block until REVIEW_PROTOCOL_ENFORCE=1.
test_prose_reply_advisory_by_default() {
    run_case "resolved-prose-reply-only-blocks"
    local converged missing enforce
    converged="$(json_field "$RUN_STDOUT" '.converged')"
    missing="$(json_field "$RUN_STDOUT" '.dispositions_missing_marker')"
    enforce="$(json_field "$RUN_STDOUT" '.review_protocol_enforce')"
    if [[ "$RUN_EXIT" -eq 0 && "$converged" == "true" && "$missing" -ge 1 && "$enforce" == "false" ]]; then
        pass "prose-only reply is advisory-only by default (exit 0, converged:true, dispositions_missing_marker>=1, review_protocol_enforce:false)"
    else
        fail "prose-only reply should be advisory by default — got exit=$RUN_EXIT converged=$converged dispositions_missing_marker=$missing enforce=$enforce"
    fi
}

# ── Test 12: fix-commit unreachable from head BLOCKS under enforce ─────────
test_unreachable_fix_commit_blocks_under_enforce() {
    run_case_enforce "fixed-disposition-commit-not-reachable-from-head-blocks"
    local converged unreach
    converged="$(json_field "$RUN_STDOUT" '.converged')"
    unreach="$(json_field "$RUN_STDOUT" '.unreachable_fix_commits')"
    if [[ "$RUN_EXIT" -eq 1 && "$converged" == "false" && "$unreach" -ge 1 ]]; then
        pass "fixed disposition citing an unreachable commit blocks under enforce (exit 1, unreachable_fix_commits>=1)"
    else
        fail "unreachable fix commit should block under enforce — got exit=$RUN_EXIT converged=$converged unreachable_fix_commits=$unreach"
    fi
}

# ── Test 13: fixed-without-independent-verifier BLOCKS under enforce ───────
test_fixed_without_verifier_blocks_under_enforce() {
    run_case_enforce "fixed-without-required-verifier-blocks"
    local converged vmatch
    converged="$(json_field "$RUN_STDOUT" '.converged')"
    vmatch="$(json_field "$RUN_STDOUT" '.verification_receipt_head_match')"
    if [[ "$RUN_EXIT" -eq 1 && "$converged" == "false" && "$vmatch" == "false" ]]; then
        pass "fixed disposition without an independent verifier at head blocks under enforce (exit 1, verification_receipt_head_match:false)"
    else
        fail "fixed-without-verifier should block under enforce — got exit=$RUN_EXIT converged=$converged verification_receipt_head_match=$vmatch"
    fi
}

# ── Test 14: refuted-without-independent-adjudication BLOCKS under enforce ─
test_refuted_without_adjudication_blocks_under_enforce() {
    run_case_enforce "refuted-substantive-without-independent-adjudication-blocks"
    local converged vmatch
    converged="$(json_field "$RUN_STDOUT" '.converged')"
    vmatch="$(json_field "$RUN_STDOUT" '.verification_receipt_head_match')"
    if [[ "$RUN_EXIT" -eq 1 && "$converged" == "false" && "$vmatch" == "false" ]]; then
        pass "substantive refutation without independent adjudication at head blocks under enforce (exit 1, verification_receipt_head_match:false)"
    else
        fail "refuted-without-adjudication should block under enforce — got exit=$RUN_EXIT converged=$converged verification_receipt_head_match=$vmatch"
    fi
}

# ── Test 15: follow-up-without-issue-number BLOCKS under enforce ───────────
test_followup_without_issue_blocks_under_enforce() {
    run_case_enforce "follow-up-without-issue-number-blocks"
    local converged fu
    converged="$(json_field "$RUN_STDOUT" '.converged')"
    fu="$(json_field "$RUN_STDOUT" '.followups_without_issue')"
    if [[ "$RUN_EXIT" -eq 1 && "$converged" == "false" && "$fu" -ge 1 ]]; then
        pass "follow-up disposition without an issue number blocks under enforce (exit 1, followups_without_issue>=1)"
    else
        fail "follow-up-without-issue should block under enforce — got exit=$RUN_EXIT converged=$converged followups_without_issue=$fu"
    fi
}

# ── Test 16: review-run receipt still running BLOCKS under enforce ─────────
test_review_run_running_blocks_under_enforce() {
    run_case_enforce "review-run-receipt-still-running-blocks"
    local converged rr
    converged="$(json_field "$RUN_STDOUT" '.converged')"
    rr="$(json_field "$RUN_STDOUT" '.review_runs_in_flight')"
    if [[ "$RUN_EXIT" -eq 1 && "$converged" == "false" && "$rr" -ge 1 ]]; then
        pass "in-flight (running) review-run receipt blocks under enforce (exit 1, review_runs_in_flight>=1)"
    else
        fail "running review-run should block under enforce — got exit=$RUN_EXIT converged=$converged review_runs_in_flight=$rr"
    fi
}

# ── Test 17: deep review receipt bound to an older head BLOCKS under enforce
test_receipt_older_head_blocks_under_enforce() {
    run_case_enforce "receipt-bound-to-older-head-blocks"
    local converged dmatch
    converged="$(json_field "$RUN_STDOUT" '.converged')"
    dmatch="$(json_field "$RUN_STDOUT" '.deep_review_receipt_head_match')"
    if [[ "$RUN_EXIT" -eq 1 && "$converged" == "false" && "$dmatch" == "false" ]]; then
        pass "deep review receipt bound to an older head blocks under enforce (exit 1, deep_review_receipt_head_match:false)"
    else
        fail "older-head receipt should block under enforce — got exit=$RUN_EXIT converged=$converged deep_review_receipt_head_match=$dmatch"
    fi
}

# ── Test 18: the full valid fixed→verify→disposition chain CONVERGES ───────
# The positive: converges even under enforce (0 new-axis violations).
test_valid_fixed_proof_converges_under_enforce() {
    run_case_enforce "valid-fixed-proof-verification-disposition-passes"
    local converged missing unreach vmatch dmatch
    converged="$(json_field "$RUN_STDOUT" '.converged')"
    missing="$(json_field "$RUN_STDOUT" '.dispositions_missing_marker')"
    unreach="$(json_field "$RUN_STDOUT" '.unreachable_fix_commits')"
    vmatch="$(json_field "$RUN_STDOUT" '.verification_receipt_head_match')"
    dmatch="$(json_field "$RUN_STDOUT" '.deep_review_receipt_head_match')"
    if [[ "$RUN_EXIT" -eq 0 && "$converged" == "true" && "$missing" -eq 0 && \
          "$unreach" -eq 0 && "$vmatch" == "true" && "$dmatch" == "true" ]]; then
        pass "valid fixed→verify→disposition chain converges under enforce (exit 0, all new axes clean)"
    else
        fail "valid chain should converge under enforce — got exit=$RUN_EXIT converged=$converged missing=$missing unreach=$unreach vmatch=$vmatch dmatch=$dmatch"
    fi
}

# ── Test 19: resolved_to_clear_3647 — 15 threads blocked (executable memory)
# The #3647 incident as a fixture: 15 threads resolved with no reply. Blocks
# in BOTH modes (the no-reply subset is #3732's hard block), and the R1
# marker axis independently counts all 15.
test_resolved_to_clear_3647_blocks_all() {
    run_case "resolved_to_clear_3647"
    local converged rwd missing
    converged="$(json_field "$RUN_STDOUT" '.converged')"
    rwd="$(json_field "$RUN_STDOUT" '.resolved_without_disposition')"
    missing="$(json_field "$RUN_STDOUT" '.dispositions_missing_marker')"
    if [[ "$RUN_EXIT" -eq 1 && "$converged" == "false" && "$rwd" -eq 15 && "$missing" -eq 15 ]]; then
        pass "resolved_to_clear_3647: 15 no-reply resolved threads block (exit 1, resolved_without_disposition=15, dispositions_missing_marker=15)"
    else
        fail "resolved_to_clear_3647 should block all 15 — got exit=$RUN_EXIT converged=$converged resolved_without_disposition=$rwd dispositions_missing_marker=$missing"
    fi
}

# ── Test 20: advisory mode emits WARN (not BLOCK) for a new-axis finding ───
# Guards the advisory-first stderr contract: the finding is visible as WARN
# and does NOT appear as a BLOCK line when REVIEW_PROTOCOL_ENFORCE is unset.
test_advisory_emits_warn_not_block() {
    local fixture_dir="$FIXTURES_ROOT/resolved-prose-reply-only-blocks"
    local stderr_output
    stderr_output="$(CONVERGENCE_TEST_FIXTURE_DIR="$fixture_dir" bash "$SCRIPT" 9999 "test-owner/test-repo" 2>&1 1>/dev/null)" || true
    if echo "$stderr_output" | grep -q "^WARN.*disposition:v1" && \
       ! echo "$stderr_output" | grep -q "^BLOCK.*disposition:v1"; then
        pass "advisory mode emits a WARN line (not BLOCK) for the marker-missing finding"
    else
        fail "expected a WARN (not BLOCK) line for the advisory marker finding — got: $stderr_output"
    fi
}

# ── Run all tests ─────────────────────────────────────────────────────────────

echo "=== check-pr-review-convergence test suite ==="
echo ""

test_outdated_unresolved_blocks
test_active_unresolved_blocks
test_all_resolved_converges
test_stale_bot_review_advisory_only
test_outdated_case_emits_block_line
test_missing_fixture_dir_errors
test_resolved_without_disposition_blocks
test_resolved_with_disposition_does_not_block
test_pending_independent_review_blocks
test_prose_reply_blocks_under_enforce
test_prose_reply_advisory_by_default
test_unreachable_fix_commit_blocks_under_enforce
test_fixed_without_verifier_blocks_under_enforce
test_refuted_without_adjudication_blocks_under_enforce
test_followup_without_issue_blocks_under_enforce
test_review_run_running_blocks_under_enforce
test_receipt_older_head_blocks_under_enforce
test_valid_fixed_proof_converges_under_enforce
test_resolved_to_clear_3647_blocks_all
test_advisory_emits_warn_not_block

echo ""
echo "=== Results: $PASS_COUNT passed, $FAIL_COUNT failed ==="

if [[ "$FAIL_COUNT" -gt 0 ]]; then
    exit 1
fi
exit 0
