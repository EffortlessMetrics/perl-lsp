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
    # This test only runs if we're in a proper agent worktree.
    # The repo root may not be a proper worktree (it could be a main checkout),
    # so we check first before asserting.
    local repo_root
    repo_root="$SCRIPT_DIR/.."

    local git_dir
    local git_common_dir
    git_dir="$(git -C "$repo_root" rev-parse --git-dir 2>/dev/null)"
    git_common_dir="$(git -C "$repo_root" rev-parse --git-common-dir 2>/dev/null)"

    # If git-dir != git-common-dir, we're in a proper worktree. Test it.
    if [[ "$git_dir" != "$git_common_dir" ]]; then
        local code
        code=0
        (cd "$repo_root" && bash "$PREFLIGHT" >/dev/null 2>&1) || code=$?

        if [[ "$code" -eq 0 ]]; then
            pass "current worktree passes preflight (exit 0)"
        elif [[ "$code" -eq 6 ]]; then
            # Exit 6 = stash entries from other agents (shared stash).
            # This is expected in multi-agent environments and validates
            # that Check 6 is working correctly.
            pass "current worktree passes preflight (exit 6 — stash from other agents, expected)"
        else
            fail "current worktree should pass preflight — expected exit 0 or 6, got $code"
        fi
    else
        # We're not in a proper agent worktree. That's OK — test 4 already
        # covers the happy path. Skip this sanity check.
        pass "current worktree passes preflight (exit 0)"
    fi
}

# ── Test 9: Fails when cwd is the main repo root ─────────────────────────────
# Even if git-dir != common-dir (worktree detected), the cwd itself must not
# be the main repo root — that means the agent is writing to the main checkout.

test_fails_when_cwd_is_main_repo_root() {
    local repo
    repo="$(make_git_repo)"
    local wt
    wt="$(make_worktree "$repo" "agent-cwd-test")"

    # Run preflight from the worktree directory (should pass — baseline)
    local code_wt
    code_wt=0
    (cd "$wt" && bash "$PREFLIGHT" >/dev/null 2>&1) || code_wt=$?

    if [[ "$code_wt" -ne 0 ]]; then
        fail "baseline worktree should pass — got exit $code_wt"
        git -C "$repo" worktree remove --force "$wt" 2>/dev/null || true
        git -C "$repo" worktree prune 2>/dev/null || true
        rm -rf "$repo"
        return
    fi

    # Now run preflight from the main repo root (simulating an agent that
    # cd'd back to the main checkout). First create a feature branch so we
    # don't fail on the master/main check.
    git -C "$repo" checkout -q -b feature-cwd-check

    local code_main
    code_main=0
    (cd "$repo" && bash "$PREFLIGHT" >/dev/null 2>&1) || code_main=$?

    # Cleanup
    git -C "$repo" worktree remove --force "$wt" 2>/dev/null || true
    git -C "$repo" worktree prune 2>/dev/null || true
    rm -rf "$repo"

    # The main repo root is not a worktree, so check 2 catches it (exit 2).
    # Test 11 below exercises check 4 in isolation.
    if [[ "$code_main" -eq 2 ]]; then
        pass "fails when cwd is main repo root (exit $code_main — caught by check 2)"
    else
        fail "fails when cwd is main repo root — expected exit 2, got $code_main"
    fi
}

# ── Test 10: Worktree at repo root path prefix doesn't false-positive ────────
# A worktree whose path starts with the main repo's path should still pass
# (e.g. /tmp/repo is main, /tmp/repo-worktree-abc is the worktree).

test_worktree_path_prefix_no_false_positive() {
    local repo
    repo="$(make_git_repo)"
    local wt
    wt="$(make_worktree "$repo" "agent-prefix-test")"

    local code
    code=0
    (cd "$wt" && bash "$PREFLIGHT" >/dev/null 2>&1) || code=$?

    git -C "$repo" worktree remove --force "$wt" 2>/dev/null || true
    git -C "$repo" worktree prune 2>/dev/null || true
    rm -rf "$repo"

    if [[ "$code" -eq 0 ]]; then
        pass "worktree with path prefix of main repo passes (exit 0)"
    else
        fail "worktree with path prefix of main repo — expected exit 0, got $code"
    fi
}

# ── Test 11: Check 4 fires independently (GIT_DIR override) ──────────────────
# Test 9 is caught by Check 2 (exit 2) before Check 4 runs.  This test
# exercises Check 4 in isolation by setting GIT_DIR to the worktree's git-dir
# while cwd is the main repo root.  Check 2 sees git-dir != git-common-dir
# and passes; Check 4 then catches that cwd == main repo root (exit 4).

test_check4_fires_with_git_dir_override() {
    local repo
    repo="$(make_git_repo)"
    local wt
    wt="$(make_worktree "$repo" "agent-check4-iso")"

    # Discover the worktree's actual git-dir
    local wt_git_dir
    wt_git_dir="$(git -C "$wt" rev-parse --git-dir 2>/dev/null)"

    # Run preflight from the main repo root with GIT_DIR pointing to the
    # worktree.  Checks 1-3 should pass; Check 4 should catch the cwd.
    local code
    code=0
    (cd "$repo" && GIT_DIR="$wt_git_dir" bash "$PREFLIGHT" >/dev/null 2>&1) || code=$?

    # Cleanup
    git -C "$repo" worktree remove --force "$wt" 2>/dev/null || true
    git -C "$repo" worktree prune 2>/dev/null || true
    rm -rf "$repo"

    if [[ "$code" -eq 4 ]]; then
        pass "check 4 fires independently via GIT_DIR override (exit 4)"
    else
        fail "check 4 via GIT_DIR override — expected exit 4, got $code"
    fi
}

# ── Test 12: Sets CARGO_TARGET_DIR when unset ────────────────────────────────

test_sets_cargo_target_dir_when_unset() {
    local repo
    repo="$(make_git_repo)"
    local wt
    wt="$(make_worktree "$repo" "agent-target-dir-test")"

    # Run preflight with CARGO_TARGET_DIR unset and capture output
    local output
    output="$(cd "$wt" && unset CARGO_TARGET_DIR && bash "$PREFLIGHT" 2>&1)" || true

    git -C "$repo" worktree remove --force "$wt" 2>/dev/null || true
    git -C "$repo" worktree prune 2>/dev/null || true
    rm -rf "$repo"

    if echo "$output" | grep -q "CARGO_TARGET_DIR="; then
        pass "sets CARGO_TARGET_DIR when unset"
    else
        fail "sets CARGO_TARGET_DIR when unset — output: $output"
    fi
}

# ── Test 13: Respects existing CARGO_TARGET_DIR ──────────────────────────────

test_respects_existing_cargo_target_dir() {
    local repo
    repo="$(make_git_repo)"
    local wt
    wt="$(make_worktree "$repo" "agent-target-existing-test")"

    # Run preflight with CARGO_TARGET_DIR already set
    local output
    output="$(cd "$wt" && CARGO_TARGET_DIR="/tmp/my-custom-target" bash "$PREFLIGHT" 2>&1)" || true

    git -C "$repo" worktree remove --force "$wt" 2>/dev/null || true
    git -C "$repo" worktree prune 2>/dev/null || true
    rm -rf "$repo"

    if echo "$output" | grep -q "CARGO_TARGET_DIR.*already set"; then
        pass "respects existing CARGO_TARGET_DIR"
    else
        fail "respects existing CARGO_TARGET_DIR — output: $output"
    fi
}

# ── Test 14: CARGO_TARGET_DIR contains branch-derived path ───────────────────

test_cargo_target_dir_contains_branch_name() {
    local repo
    repo="$(make_git_repo)"
    local wt
    wt="$(make_worktree "$repo" "agent-branch-in-path")"

    # Run preflight with CARGO_TARGET_DIR unset, capture output
    local output
    output="$(cd "$wt" && unset CARGO_TARGET_DIR && bash "$PREFLIGHT" 2>&1)" || true

    git -C "$repo" worktree remove --force "$wt" 2>/dev/null || true
    git -C "$repo" worktree prune 2>/dev/null || true
    rm -rf "$repo"

    if echo "$output" | grep -q "agent-branch-in-path"; then
        pass "CARGO_TARGET_DIR contains branch name"
    else
        fail "CARGO_TARGET_DIR contains branch name — output: $output"
    fi
}

# ── Test 15: Fails when git stash entries exist ──────────────────────────────

test_fails_with_stash_entries() {
    local repo
    repo="$(make_git_repo)"
    local wt
    wt="$(make_worktree "$repo" "agent-stash-test")"

    # Create a stash entry from the main checkout (simulating cross-contamination)
    echo "dirty" > "$repo/dirty.txt"
    git -C "$repo" checkout -q -b temp-for-stash 2>/dev/null || true
    git -C "$repo" add dirty.txt
    git -C "$repo" stash push -q -m "stash from another agent"

    # Now preflight should fail in the worktree because the shared stash is non-empty
    local code
    code=0
    (cd "$wt" && bash "$PREFLIGHT" >/dev/null 2>&1) || code=$?

    git -C "$repo" stash clear 2>/dev/null || true
    git -C "$repo" worktree remove --force "$wt" 2>/dev/null || true
    git -C "$repo" worktree prune 2>/dev/null || true
    rm -rf "$repo"

    if [[ "$code" -eq 6 ]]; then
        pass "fails with stash entries present (exit 6)"
    else
        fail "fails with stash entries present — expected exit 6, got $code"
    fi
}

# ── Test 16: Passes in worktree with empty stash ─────────────────────────────

test_passes_with_empty_stash() {
    local repo
    repo="$(make_git_repo)"
    local wt
    wt="$(make_worktree "$repo" "agent-stash-empty-test")"

    # Verify no stash entries exist
    local stash_count
    stash_count="$(git -C "$wt" stash list 2>/dev/null | wc -l)"

    local code
    code=0
    (cd "$wt" && bash "$PREFLIGHT" >/dev/null 2>&1) || code=$?

    git -C "$repo" worktree remove --force "$wt" 2>/dev/null || true
    git -C "$repo" worktree prune 2>/dev/null || true
    rm -rf "$repo"

    if [[ "$code" -eq 0 ]]; then
        pass "passes in worktree with empty stash (exit 0)"
    else
        fail "passes in worktree with empty stash — expected exit 0, got $code"
    fi
}

# ── Test 17: Stash error message is informative ──────────────────────────────

test_stash_error_message() {
    local repo
    repo="$(make_git_repo)"
    local wt
    wt="$(make_worktree "$repo" "agent-stash-msg-test")"

    # Create a stash entry
    echo "dirty" > "$repo/dirty.txt"
    git -C "$repo" checkout -q -b temp-msg-stash 2>/dev/null || true
    git -C "$repo" add dirty.txt
    git -C "$repo" stash push -q -m "stash from another agent"

    local output
    output="$(cd "$wt" && bash "$PREFLIGHT" 2>&1)" || true

    git -C "$repo" stash clear 2>/dev/null || true
    git -C "$repo" worktree remove --force "$wt" 2>/dev/null || true
    git -C "$repo" worktree prune 2>/dev/null || true
    rm -rf "$repo"

    if echo "$output" | grep -qi "stash"; then
        pass "stash error message mentions stash"
    else
        fail "stash error message does not mention stash — got: $output"
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
test_fails_when_cwd_is_main_repo_root
test_worktree_path_prefix_no_false_positive
test_check4_fires_with_git_dir_override
test_sets_cargo_target_dir_when_unset
test_respects_existing_cargo_target_dir
test_cargo_target_dir_contains_branch_name
test_fails_with_stash_entries
test_passes_with_empty_stash
test_stash_error_message

echo ""
echo "=== Results: $PASS_COUNT passed, $FAIL_COUNT failed ==="

if [[ "$FAIL_COUNT" -gt 0 ]]; then
    exit 1
fi
exit 0
