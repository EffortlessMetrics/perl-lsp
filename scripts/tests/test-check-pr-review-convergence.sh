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

# ── Run all tests ─────────────────────────────────────────────────────────────

echo "=== check-pr-review-convergence test suite ==="
echo ""

test_outdated_unresolved_blocks
test_active_unresolved_blocks
test_all_resolved_converges
test_stale_bot_review_advisory_only
test_outdated_case_emits_block_line
test_missing_fixture_dir_errors

echo ""
echo "=== Results: $PASS_COUNT passed, $FAIL_COUNT failed ==="

if [[ "$FAIL_COUNT" -gt 0 ]]; then
    exit 1
fi
exit 0
