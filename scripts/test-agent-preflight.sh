#!/usr/bin/env bash
# Test suite for scripts/agent-preflight.sh
# TDD: exercises each check independently using temporary git environments

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREFLIGHT="$SCRIPT_DIR/agent-preflight.sh"
PASS_COUNT=0
FAIL_COUNT=0

pass() { printf 'PASS %s\n' "$1"; PASS_COUNT=$((PASS_COUNT + 1)); }
fail() { printf 'FAIL %s\n' "$1"; FAIL_COUNT=$((FAIL_COUNT + 1)); }

# Verify the preflight script exists (test the test can run at all)
if [[ ! -f "$PREFLIGHT" ]]; then
    echo "ERROR: agent-preflight.sh not found at $PREFLIGHT"
    echo "Write the implementation first: scripts/agent-preflight.sh"
    exit 1
fi

# ── Helpers ──────────────────────────────────────────────────────────────────

# Create a minimal git repo in a temp dir
make_git_repo() {
    local tmpdir
    tmpdir="$(mktemp -d)"
    git -C "$tmpdir" init -q
    git -C "$tmpdir" config user.email "test@test.com"
    git -C "$tmpdir" config user.name "Test"
    # Need at least one commit so branches work
    echo "init" > "$tmpdir/README"
    git -C "$tmpdir" add README
    git -C "$tmpdir" commit -q -m "init"
    echo "$tmpdir"
}

# Create a worktree from a repo
make_worktree() {
    local repo="$1"
    local branch="${2:-agent-test-branch}"
    local wtdir
    wtdir="$(mktemp -d)"
    rm -rf "$wtdir"  # worktree add needs the dir to not exist
    git -C "$repo" worktree add -q -b "$branch" "$wtdir"
    echo "$wtdir"
}

cleanup() {
    # Remove temp dirs created during tests
    local dir
    for dir in "$@"; do
        [[ -d "$dir" ]] || continue
        rm -rf "$dir"
    done
}

# ── Test 1: Fails on master branch ───────────────────────────────────────────

test_fails_on_master() {
    local repo
    repo="$(make_git_repo)"
    # Rename to master
    git -C "$repo" branch -m master 2>/dev/null || git -C "$repo" checkout -q -b master 2>/dev/null || true

    local code
    code=0
    (cd "$repo" && bash "$PREFLIGHT" >/dev/null 2>&1) || code=$?

    cleanup "$repo"

    if [[ "$code" -eq 1 ]]; then
        pass "fails on master branch (exit 1)"
    else
        fail "fails on master branch — expected exit 1, got $code"
    fi
}

# ── Test 2: Fails on main branch ─────────────────────────────────────────────

test_fails_on_main() {
    local repo
    repo="$(make_git_repo)"
    # git init defaults may use 'main'
    git -C "$repo" branch -m main 2>/dev/null || true

    local code
    code=0
    (cd "$repo" && bash "$PREFLIGHT" >/dev/null 2>&1) || code=$?

    cleanup "$repo"

    if [[ "$code" -eq 1 ]]; then
        pass "fails on main branch (exit 1)"
    else
        fail "fails on main branch — expected exit 1, got $code"
    fi
}

# ── Test 3: Fails in non-worktree checkout ────────────────────────────────────

test_fails_in_non_worktree() {
    local repo
    repo="$(make_git_repo)"
    # Create a feature branch so we're not on master/main
    git -C "$repo" checkout -q -b feature-test

    local code
    code=0
    (cd "$repo" && bash "$PREFLIGHT" >/dev/null 2>&1) || code=$?

    cleanup "$repo"

    # Should fail with exit code 2 (not a worktree)
    if [[ "$code" -eq 2 ]]; then
        pass "fails in non-worktree checkout (exit 2)"
    else
        fail "fails in non-worktree checkout — expected exit 2, got $code"
    fi
}

# ── Test 4: Passes in a proper worktree ──────────────────────────────────────

test_passes_in_worktree() {
    local repo
    repo="$(make_git_repo)"
    local wt
    wt="$(make_worktree "$repo" "agent-test-ok")"

    local code
    code=0
    (cd "$wt" && bash "$PREFLIGHT" >/dev/null 2>&1) || code=$?

    # Cleanup worktree then repo
    git -C "$repo" worktree remove --force "$wt" 2>/dev/null || true
    git -C "$repo" worktree prune 2>/dev/null || true
    rm -rf "$repo"

    if [[ "$code" -eq 0 ]]; then
        pass "passes in a proper worktree (exit 0)"
    else
        fail "passes in a proper worktree — expected exit 0, got $code"
    fi
}

# ── Test 5: Fails with unresolved merge conflicts ────────────────────────────

test_fails_with_conflicts() {
    local repo
    repo="$(make_git_repo)"
    local wt
    wt="$(make_worktree "$repo" "agent-conflict-test")"

    # Create a conflict marker file manually to simulate unresolved conflicts
    printf '<<<<<<< HEAD\nfoo\n=======\nbar\n>>>>>>> other\n' > "$wt/conflict.txt"

    local code
    code=0
    (cd "$wt" && bash "$PREFLIGHT" >/dev/null 2>&1) || code=$?

    git -C "$repo" worktree remove --force "$wt" 2>/dev/null || true
    git -C "$repo" worktree prune 2>/dev/null || true
    rm -rf "$repo"

    if [[ "$code" -eq 3 ]]; then
        pass "fails with unresolved merge conflicts (exit 3)"
    else
        fail "fails with unresolved merge conflicts — expected exit 3, got $code"
    fi
}

# ── Test 6: Detached HEAD fails ───────────────────────────────────────────────

test_fails_in_detached_head() {
    local repo
    repo="$(make_git_repo)"
    # Detach HEAD
    local sha
    sha="$(git -C "$repo" rev-parse HEAD)"
    git -C "$repo" checkout -q --detach "$sha"

    local code
    code=0
    (cd "$repo" && bash "$PREFLIGHT" >/dev/null 2>&1) || code=$?

    cleanup "$repo"

    if [[ "$code" -eq 1 ]]; then
        pass "fails in detached HEAD state (exit 1)"
    else
        fail "fails in detached HEAD state — expected exit 1, got $code"
    fi
}

# ── Test 7: Error messages are informative ────────────────────────────────────

test_error_messages_on_master() {
    local repo
    repo="$(make_git_repo)"
    git -C "$repo" branch -m master 2>/dev/null || true

    local output
    output="$(cd "$repo" && bash "$PREFLIGHT" 2>&1)" || true

    cleanup "$repo"

    if echo "$output" | grep -qi "master\|main"; then
        pass "error message mentions branch name"
    else
        fail "error message does not mention branch name — got: $output"
    fi
}

# ── Test 8: Runs without errors in THIS worktree ─────────────────────────────

test_current_worktree_passes() {
    local code
    code=0
    (cd "$SCRIPT_DIR/.." && bash "$PREFLIGHT" >/dev/null 2>&1) || code=$?

    if [[ "$code" -eq 0 ]]; then
        pass "current worktree passes preflight (exit 0)"
    else
        fail "current worktree should pass preflight — expected exit 0, got $code"
    fi
}

# ── Run all tests ─────────────────────────────────────────────────────────────

echo "=== agent-preflight test suite ==="
echo ""

test_fails_on_master
test_fails_on_main
test_fails_in_non_worktree
test_passes_in_worktree
test_fails_with_conflicts
test_fails_in_detached_head
test_error_messages_on_master
test_current_worktree_passes

echo ""
echo "=== Results: $PASS_COUNT passed, $FAIL_COUNT failed ==="

if [[ "$FAIL_COUNT" -gt 0 ]]; then
    exit 1
fi
exit 0
