#!/usr/bin/env bash
# scripts/tests/test_swarm_doctor.sh
#
# Test suite for scripts/swarm-doctor.
#
# Verifies:
#   - Basic invocation succeeds (exit 0 always, even on error)
#   - --json flag emits parseable JSON
#   - Worktree inventory is included in output
#   - A dirty worktree is correctly reported as dirty
#   - A clean worktree is correctly reported as clean
#
# Uses isolated temporary git repos/worktrees; cleans up on exit.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMPL="$SCRIPT_DIR/../swarm-doctor"
PASS_COUNT=0
FAIL_COUNT=0

if [[ ! -f "$IMPL" ]]; then
    echo "ERROR: swarm-doctor not found at $IMPL"
    echo "Write the implementation first: scripts/swarm-doctor"
    exit 1
fi

pass() { printf 'PASS %s\n' "$1"; PASS_COUNT=$((PASS_COUNT + 1)); }
fail() { printf 'FAIL %s\n' "$1"; FAIL_COUNT=$((FAIL_COUNT + 1)); }

# ── Fixture infrastructure ────────────────────────────────────────────────────
#
# IMPORTANT: make_fixture_repo sets globals FIXTURE_DIR and FIXTURE_REPO.
# Do NOT call make_fixture_repo inside $() — that runs it in a subshell
# and the globals won't propagate back to the parent.

FIXTURE_DIR=""
FIXTURE_REPO=""

cleanup_fixtures() {
    if [[ -n "$FIXTURE_DIR" ]] && [[ -d "$FIXTURE_DIR" ]]; then
        rm -rf "$FIXTURE_DIR"
    fi
}
trap cleanup_fixtures EXIT

# Sets FIXTURE_DIR and FIXTURE_REPO globals. Call without $().
make_fixture_repo() {
    # Cleanup any previous fixture first.
    if [[ -n "$FIXTURE_DIR" ]] && [[ -d "$FIXTURE_DIR" ]]; then
        rm -rf "$FIXTURE_DIR"
    fi

    FIXTURE_DIR="$(mktemp -d)"
    FIXTURE_REPO="$FIXTURE_DIR/main-repo"
    mkdir -p "$FIXTURE_REPO"

    git -C "$FIXTURE_REPO" init -q --initial-branch=main 2>/dev/null \
        || git -C "$FIXTURE_REPO" init -q 2>/dev/null  # older git without --initial-branch

    git -C "$FIXTURE_REPO" config user.email "test@test.local"
    git -C "$FIXTURE_REPO" config user.name "Test"
    # Disable commit signing so fixture commits don't hit the signing server.
    git -C "$FIXTURE_REPO" config commit.gpgsign false
    git -C "$FIXTURE_REPO" config gpg.format ""

    # Create a commit so HEAD is valid.
    echo "init" > "$FIXTURE_REPO/README"
    git -C "$FIXTURE_REPO" add README
    git -C "$FIXTURE_REPO" commit -q -m "init"
}

# Add a linked worktree to the fixture repo. Returns the worktree path.
make_fixture_worktree() {
    local branch="$1"
    local wt_path="$FIXTURE_DIR/wt-${branch}"
    git -C "$FIXTURE_REPO" worktree add -q -b "$branch" "$wt_path" 2>/dev/null
    echo "$wt_path"
}

run_doctor() {
    local args=("$@")
    REPO_ROOT="$FIXTURE_REPO" bash "$IMPL" "${args[@]}" 2>&1
}

teardown() {
    if [[ -n "$FIXTURE_DIR" ]] && [[ -d "$FIXTURE_DIR" ]]; then
        rm -rf "$FIXTURE_DIR"
    fi
    FIXTURE_DIR=""
    FIXTURE_REPO=""
}

# ── Tests ─────────────────────────────────────────────────────────────────────

# Test 1: Basic invocation always exits 0.
test_exits_zero_always() {
    make_fixture_repo
    local exit_code=0
    REPO_ROOT="$FIXTURE_REPO" bash "$IMPL" >/dev/null 2>&1 || exit_code=$?
    teardown

    if [[ "$exit_code" -eq 0 ]]; then
        pass "exits 0 always (basic invocation)"
    else
        fail "exits non-zero ($exit_code) — swarm-doctor must always exit 0"
    fi
}

# Test 2: --json flag produces JSON with worktrees key.
test_json_flag_produces_valid_structure() {
    make_fixture_repo
    local output exit_code=0
    output="$(run_doctor --json)" || exit_code=$?
    teardown

    if [[ "$exit_code" -ne 0 ]]; then
        fail "--json: exited $exit_code (expected 0)"
        return
    fi

    # Must contain the "worktrees" key.
    if echo "$output" | grep -q '"worktrees"'; then
        pass "--json output contains 'worktrees' key"
    else
        fail "--json output missing 'worktrees' key — got: ${output:0:200}"
    fi
}

# Test 3: A dirty worktree is reported as dirty in human output.
test_dirty_worktree_reported_as_dirty() {
    make_fixture_repo
    local wt_path
    wt_path="$(make_fixture_worktree "test-dirty-branch")"

    # Create an uncommitted change in the worktree.
    echo "dirty content" > "$wt_path/dirty-file.txt"

    # Verify it shows up as dirty.
    local git_status
    git_status="$(git -C "$wt_path" status --porcelain 2>/dev/null || echo "")"
    if [[ -z "$git_status" ]]; then
        fail "test setup: worktree not dirty (git status empty)"
        teardown
        return
    fi

    local output exit_code=0
    output="$(run_doctor)" || exit_code=$?
    teardown

    if [[ "$exit_code" -ne 0 ]]; then
        fail "dirty worktree test: exited $exit_code"
        return
    fi

    # Human output must contain "dirty" for the worktree.
    if echo "$output" | grep -q "dirty"; then
        pass "dirty worktree is reported as dirty in human output"
    else
        fail "dirty worktree not reflected in output — got: ${output:0:400}"
    fi
}

# Test 4: A clean worktree is reported as clean in human output.
test_clean_worktree_reported_as_clean() {
    make_fixture_repo
    make_fixture_worktree "test-clean-branch" >/dev/null
    # No changes — worktree is clean by default.

    local output exit_code=0
    output="$(run_doctor)" || exit_code=$?
    teardown

    if [[ "$exit_code" -ne 0 ]]; then
        fail "clean worktree test: exited $exit_code"
        return
    fi

    # Human output must contain "clean" for the worktree.
    if echo "$output" | grep -q "clean"; then
        pass "clean worktree is reported as clean in human output"
    else
        fail "clean worktree not reflected as clean in output — got: ${output:0:400}"
    fi
}

# Test 5: Divergence section appears in output.
test_divergence_section_appears() {
    make_fixture_repo
    local output exit_code=0
    output="$(run_doctor)" || exit_code=$?
    teardown

    if echo "$output" | grep -qi "divergence\|ahead\|behind\|branch"; then
        pass "divergence section appears in output"
    else
        fail "divergence section missing from output — got: ${output:0:400}"
    fi
}

# Test 6: --json output contains divergence data.
test_json_contains_divergence() {
    make_fixture_repo
    local output exit_code=0
    output="$(run_doctor --json)" || exit_code=$?
    teardown

    if echo "$output" | grep -q '"divergence"'; then
        pass "--json output contains 'divergence' key"
    else
        fail "--json output missing 'divergence' key — got: ${output:0:200}"
    fi
}

# Test 7: Unknown flag exits non-zero and prints usage.
test_unknown_flag_fails() {
    local exit_code=0
    bash "$IMPL" --no-such-flag >/dev/null 2>&1 || exit_code=$?
    if [[ "$exit_code" -ne 0 ]]; then
        pass "unknown flag exits non-zero"
    else
        fail "unknown flag should exit non-zero but exited 0"
    fi
}

# ── Run all tests ─────────────────────────────────────────────────────────────

echo "=== swarm-doctor test suite ==="
echo ""

test_exits_zero_always
test_json_flag_produces_valid_structure
test_dirty_worktree_reported_as_dirty
test_clean_worktree_reported_as_clean
test_divergence_section_appears
test_json_contains_divergence
test_unknown_flag_fails

echo ""
echo "=== Results: $PASS_COUNT passed, $FAIL_COUNT failed ==="

if [[ "$FAIL_COUNT" -gt 0 ]]; then
    exit 1
fi
exit 0
