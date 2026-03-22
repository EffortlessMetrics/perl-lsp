#!/usr/bin/env bash
# Test suite for plan-reviewer label enforcement in .claude/hooks/subagent-stop.sh
# TDD: exercises the enforcement block in isolation.
# Issue #2656: plan-reviewers must add builder-ready or already-fixed label.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOK="$REPO_ROOT/.claude/hooks/subagent-stop.sh"
PASS_COUNT=0
FAIL_COUNT=0

pass() { printf 'PASS  %s\n' "$1"; PASS_COUNT=$((PASS_COUNT + 1)); }
fail() { printf 'FAIL  %s\n' "$1"; FAIL_COUNT=$((FAIL_COUNT + 1)); }

# Verify the hook exists
if [[ ! -f "$HOOK" ]]; then
    echo "ERROR: subagent-stop.sh not found at $HOOK"
    exit 1
fi

# ── Helpers ───────────────────────────────────────────────────────────────────

# Build a JSON payload and pipe it into the hook; return exit code
run_hook() {
    local payload="$1"
    local code=0
    echo "$payload" | bash "$HOOK" 2>/dev/null || code=$?
    echo "$code"
}

# Cleanup git worktree created during tests
cleanup_worktree() {
    local wtdir="$1"
    git -C "$REPO_ROOT" worktree remove --force "$wtdir" 2>/dev/null || true
    git -C "$REPO_ROOT" worktree prune 2>/dev/null || true
}

# ── Test 1: Non-plan-reviewer exits 0 (enforcement is a no-op) ───────────────

test_non_plan_reviewer_exits_0() {
    local code
    code="$(run_hook '{"agent_type":"builder","worktree_path":"/tmp/nonexistent"}')"
    if [[ "$code" -eq 0 ]]; then
        pass "non-plan-reviewer exits 0 (enforcement skipped)"
    else
        fail "non-plan-reviewer — expected exit 0, got $code"
    fi
}

# ── Test 2: Plan-reviewer with no WORKTREE_PATH exits 0 (fail-open) ──────────

test_plan_reviewer_no_worktree_fails_open() {
    local code
    code="$(run_hook '{"agent_type":"plan-reviewer"}')"
    if [[ "$code" -eq 0 ]]; then
        pass "plan-reviewer with missing worktree_path fails open (exit 0)"
    else
        fail "plan-reviewer no worktree_path — expected exit 0, got $code"
    fi
}

# ── Test 3: Plan-reviewer with non-existent worktree path exits 0 (fail-open)

test_plan_reviewer_nonexistent_path_fails_open() {
    local code
    code="$(run_hook '{"agent_type":"plan-reviewer","worktree_path":"/tmp/does-not-exist-xyz"}')"
    if [[ "$code" -eq 0 ]]; then
        pass "plan-reviewer with non-existent path fails open (exit 0)"
    else
        fail "plan-reviewer non-existent path — expected exit 0, got $code"
    fi
}

# ── Test 4: Plan-reviewer branch with no parseable issue number exits 0 ──────

test_plan_reviewer_no_issue_number_fails_open() {
    # Create a real worktree on a branch without a number
    local tmpwt
    tmpwt="$(mktemp -d)"
    rm -rf "$tmpwt"
    git -C "$REPO_ROOT" worktree add -q -b "plan-reviewer-custom-name-no-num" "$tmpwt" HEAD 2>/dev/null || {
        git -C "$REPO_ROOT" worktree add -q "$tmpwt" -b "plan-reviewer-custom-name-no-num-$RANDOM" HEAD
    }

    local payload="{\"agent_type\":\"plan-reviewer\",\"worktree_path\":\"${tmpwt}\"}"
    local code
    code="$(run_hook "$payload")"

    cleanup_worktree "$tmpwt"

    if [[ "$code" -eq 0 ]]; then
        pass "plan-reviewer branch with no issue number fails open (exit 0)"
    else
        fail "plan-reviewer no issue number — expected exit 0, got $code"
    fi
}

# ── Test 5: Plan-reviewer where gh returns no labels → exit 2 (block) ────────
# This simulates a real plan-reviewer worktree on branch plan-review-NNNN but
# where the gh issue has no builder-ready or already-fixed label.
# We use a fake issue number (9999999) that won't exist on GitHub.
# gh will fail or return empty labels → the check must block (exit 2).
# Note: if gh fails (network/auth) it returns non-zero; LABELS will be "".
# The guard: only block if LABELS is non-empty and missing required labels.
# So a gh failure → LABELS="" → fail-open (exit 0).
# A real issue with wrong labels → LABELS has content → exit 2.
#
# We test this by using WORKTREE_PATH pointing to a real worktree on a branch
# plan-review-NNNNN, then stubbing gh via PATH manipulation.

test_plan_reviewer_blocks_when_label_missing() {
    # Create a worktree on branch plan-review-99999 (fake issue)
    local tmpwt
    tmpwt="$(mktemp -d)"
    rm -rf "$tmpwt"
    git -C "$REPO_ROOT" worktree add -q -b "plan-review-99999" "$tmpwt" HEAD 2>/dev/null || {
        git -C "$REPO_ROOT" worktree add -q "$tmpwt" -b "plan-review-99999-$RANDOM" HEAD
    }

    # Create a fake gh script that returns a label list WITHOUT builder-ready
    local fake_bin
    fake_bin="$(mktemp -d)"
    cat > "$fake_bin/gh" <<'FAKEGH'
#!/usr/bin/env bash
# Fake gh: returns a label without builder-ready or already-fixed
echo "in-build"
echo "priority:high"
FAKEGH
    chmod +x "$fake_bin/gh"

    local payload="{\"agent_type\":\"plan-reviewer\",\"worktree_path\":\"${tmpwt}\"}"
    local code=0
    echo "$payload" | PATH="$fake_bin:$PATH" bash "$HOOK" 2>/dev/null || code=$?

    cleanup_worktree "$tmpwt"
    rm -rf "$fake_bin"

    if [[ "$code" -eq 2 ]]; then
        pass "plan-reviewer blocked when builder-ready label missing (exit 2)"
    else
        fail "plan-reviewer missing label — expected exit 2, got $code"
    fi
}

# ── Test 6: Plan-reviewer passes when builder-ready is present ────────────────

test_plan_reviewer_passes_with_builder_ready() {
    local tmpwt
    tmpwt="$(mktemp -d)"
    rm -rf "$tmpwt"
    git -C "$REPO_ROOT" worktree add -q -b "plan-review-88888" "$tmpwt" HEAD 2>/dev/null || {
        git -C "$REPO_ROOT" worktree add -q "$tmpwt" -b "plan-review-88888-$RANDOM" HEAD
    }

    local fake_bin
    fake_bin="$(mktemp -d)"
    cat > "$fake_bin/gh" <<'FAKEGH'
#!/usr/bin/env bash
echo "plan-reviewed"
echo "builder-ready"
echo "priority:high"
FAKEGH
    chmod +x "$fake_bin/gh"

    local payload="{\"agent_type\":\"plan-reviewer\",\"worktree_path\":\"${tmpwt}\"}"
    local code=0
    echo "$payload" | PATH="$fake_bin:$PATH" bash "$HOOK" 2>/dev/null || code=$?

    cleanup_worktree "$tmpwt"
    rm -rf "$fake_bin"

    if [[ "$code" -eq 0 ]]; then
        pass "plan-reviewer passes with builder-ready label (exit 0)"
    else
        fail "plan-reviewer builder-ready present — expected exit 0, got $code"
    fi
}

# ── Test 7: Plan-reviewer passes when already-fixed is present ───────────────

test_plan_reviewer_passes_with_already_fixed() {
    local tmpwt
    tmpwt="$(mktemp -d)"
    rm -rf "$tmpwt"
    git -C "$REPO_ROOT" worktree add -q -b "plan-review-77777" "$tmpwt" HEAD 2>/dev/null || {
        git -C "$REPO_ROOT" worktree add -q "$tmpwt" -b "plan-review-77777-$RANDOM" HEAD
    }

    local fake_bin
    fake_bin="$(mktemp -d)"
    cat > "$fake_bin/gh" <<'FAKEGH'
#!/usr/bin/env bash
echo "already-fixed"
echo "plan-reviewed"
FAKEGH
    chmod +x "$fake_bin/gh"

    local payload="{\"agent_type\":\"plan-reviewer\",\"worktree_path\":\"${tmpwt}\"}"
    local code=0
    echo "$payload" | PATH="$fake_bin:$PATH" bash "$HOOK" 2>/dev/null || code=$?

    cleanup_worktree "$tmpwt"
    rm -rf "$fake_bin"

    if [[ "$code" -eq 0 ]]; then
        pass "plan-reviewer passes with already-fixed label (exit 0)"
    else
        fail "plan-reviewer already-fixed present — expected exit 0, got $code"
    fi
}

# ── Test 8: gh failure (network error) → fail-open (exit 0) ─────────────────

test_plan_reviewer_gh_failure_fails_open() {
    local tmpwt
    tmpwt="$(mktemp -d)"
    rm -rf "$tmpwt"
    git -C "$REPO_ROOT" worktree add -q -b "plan-review-66666" "$tmpwt" HEAD 2>/dev/null || {
        git -C "$REPO_ROOT" worktree add -q "$tmpwt" -b "plan-review-66666-$RANDOM" HEAD
    }

    local fake_bin
    fake_bin="$(mktemp -d)"
    cat > "$fake_bin/gh" <<'FAKEGH'
#!/usr/bin/env bash
# Simulate gh failure (network error, auth failure, etc.)
exit 1
FAKEGH
    chmod +x "$fake_bin/gh"

    local payload="{\"agent_type\":\"plan-reviewer\",\"worktree_path\":\"${tmpwt}\"}"
    local code=0
    echo "$payload" | PATH="$fake_bin:$PATH" bash "$HOOK" 2>/dev/null || code=$?

    cleanup_worktree "$tmpwt"
    rm -rf "$fake_bin"

    if [[ "$code" -eq 0 ]]; then
        pass "gh failure fails open (exit 0)"
    else
        fail "gh failure should fail open — expected exit 0, got $code"
    fi
}

# ── Test 9: Error message is informative (mentions issue number and labels) ───

test_error_message_is_informative() {
    local tmpwt
    tmpwt="$(mktemp -d)"
    rm -rf "$tmpwt"
    git -C "$REPO_ROOT" worktree add -q -b "plan-review-55555" "$tmpwt" HEAD 2>/dev/null || {
        git -C "$REPO_ROOT" worktree add -q "$tmpwt" -b "plan-review-55555-$RANDOM" HEAD
    }

    local fake_bin
    fake_bin="$(mktemp -d)"
    cat > "$fake_bin/gh" <<'FAKEGH'
#!/usr/bin/env bash
echo "in-build"
echo "needs-plan-review"
FAKEGH
    chmod +x "$fake_bin/gh"

    local payload="{\"agent_type\":\"plan-reviewer\",\"worktree_path\":\"${tmpwt}\"}"
    local output code=0
    output="$(echo "$payload" | PATH="$fake_bin:$PATH" bash "$HOOK" 2>&1)" || code=$?

    cleanup_worktree "$tmpwt"
    rm -rf "$fake_bin"

    if echo "$output" | grep -qi "builder-ready\|already-fixed"; then
        pass "error message mentions required labels"
    else
        fail "error message should mention builder-ready/already-fixed — got: $output"
    fi
}

# ── Test 10: Metrics are still logged regardless of enforcement result ─────────

test_metrics_logged_for_plan_reviewer() {
    local tmpwt
    tmpwt="$(mktemp -d)"
    rm -rf "$tmpwt"
    git -C "$REPO_ROOT" worktree add -q -b "plan-review-44444" "$tmpwt" HEAD 2>/dev/null || {
        git -C "$REPO_ROOT" worktree add -q "$tmpwt" -b "plan-review-44444-$RANDOM" HEAD
    }

    local fake_bin
    fake_bin="$(mktemp -d)"
    cat > "$fake_bin/gh" <<'FAKEGH'
#!/usr/bin/env bash
echo "builder-ready"
FAKEGH
    chmod +x "$fake_bin/gh"

    # Use a temp OPS_DIR so we don't pollute the real metrics file
    local ops_dir
    ops_dir="$(mktemp -d)"

    local payload="{\"agent_type\":\"plan-reviewer\",\"worktree_path\":\"${tmpwt}\"}"
    local code=0
    echo "$payload" | PATH="$fake_bin:$PATH" OPS_DIR="$ops_dir" bash "$HOOK" 2>/dev/null || code=$?

    cleanup_worktree "$tmpwt"
    rm -rf "$fake_bin"

    local logged=0
    [[ -f "$ops_dir/swarm-metrics.jsonl" ]] && logged=1
    rm -rf "$ops_dir"

    if [[ "$logged" -eq 1 ]]; then
        pass "metrics are still logged when enforcement passes"
    else
        fail "metrics were not logged — expected swarm-metrics.jsonl to be written"
    fi
}

# ── Test 11: Metrics are logged even when enforcement BLOCKS (exit 2) ─────────
# The jq metrics write runs before the enforcement block, so a blocked plan-reviewer
# should still have its stop event recorded for audit purposes.

test_metrics_logged_when_enforcement_blocks() {
    local tmpwt
    tmpwt="$(mktemp -d)"
    rm -rf "$tmpwt"
    git -C "$REPO_ROOT" worktree add -q -b "plan-review-33333" "$tmpwt" HEAD 2>/dev/null || {
        git -C "$REPO_ROOT" worktree add -q "$tmpwt" -b "plan-review-33333-$RANDOM" HEAD
    }

    local fake_bin
    fake_bin="$(mktemp -d)"
    cat > "$fake_bin/gh" <<'FAKEGH'
#!/usr/bin/env bash
# Returns labels that will trigger enforcement (neither builder-ready nor already-fixed)
echo "in-build"
echo "needs-plan-review"
FAKEGH
    chmod +x "$fake_bin/gh"

    local ops_dir
    ops_dir="$(mktemp -d)"

    local payload="{\"agent_type\":\"plan-reviewer\",\"worktree_path\":\"${tmpwt}\"}"
    local code=0
    echo "$payload" | PATH="$fake_bin:$PATH" OPS_DIR="$ops_dir" bash "$HOOK" 2>/dev/null || code=$?

    cleanup_worktree "$tmpwt"
    rm -rf "$fake_bin"

    local logged=0
    [[ -f "$ops_dir/swarm-metrics.jsonl" ]] && logged=1
    rm -rf "$ops_dir"

    # Enforcement must have blocked (exit 2)
    if [[ "$code" -ne 2 ]]; then
        fail "metrics-blocked test precondition: expected exit 2 from enforcement, got $code"
        return
    fi

    if [[ "$logged" -eq 1 ]]; then
        pass "metrics are logged even when enforcement blocks (exit 2)"
    else
        fail "metrics were not logged when enforcement blocked — subagent_stop event lost"
    fi
}

# ── Run all tests ─────────────────────────────────────────────────────────────

echo "=== subagent-stop enforcement test suite ==="
echo ""

test_non_plan_reviewer_exits_0
test_plan_reviewer_no_worktree_fails_open
test_plan_reviewer_nonexistent_path_fails_open
test_plan_reviewer_no_issue_number_fails_open
test_plan_reviewer_blocks_when_label_missing
test_plan_reviewer_passes_with_builder_ready
test_plan_reviewer_passes_with_already_fixed
test_plan_reviewer_gh_failure_fails_open
test_error_message_is_informative
test_metrics_logged_for_plan_reviewer
test_metrics_logged_when_enforcement_blocks

echo ""
echo "=== Results: $PASS_COUNT passed, $FAIL_COUNT failed ==="

if [[ "$FAIL_COUNT" -gt 0 ]]; then
    exit 1
fi
exit 0
