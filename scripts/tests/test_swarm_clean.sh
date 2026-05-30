#!/usr/bin/env bash
# scripts/tests/test_swarm_clean.sh
#
# Test suite for scripts/swarm-clean.
#
# Key invariants under test (per spec):
#   (a) A worktree with uncommitted changes is classified 'dirty' and is
#       NEVER in the --apply delete set.
#   (b) A clean merged worktree is 'clean-finished' and appears in dry-run output.
#
# Additional tests:
#   - Active (locked + live pid) worktrees are never touched.
#   - Ambiguous worktrees are reported only, never deleted.
#   - Dry-run default: nothing is deleted without --apply.
#   - --apply only removes clean-finished, not dirty or ambiguous.
#
# All tests build fixtures with temporary git repos/worktrees in a mktemp dir
# and clean them up via EXIT trap.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMPL="$SCRIPT_DIR/../swarm-clean"
PASS_COUNT=0
FAIL_COUNT=0

if [[ ! -f "$IMPL" ]]; then
    echo "ERROR: swarm-clean not found at $IMPL"
    echo "Write the implementation first: scripts/swarm-clean"
    exit 1
fi

pass() { printf 'PASS %s\n' "$1"; PASS_COUNT=$((PASS_COUNT + 1)); }
fail() { printf 'FAIL %s\n' "$1"; FAIL_COUNT=$((FAIL_COUNT + 1)); }

# ── Fixture infrastructure ───────────────────────────────────────────────────────────────────────────────
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
        || git -C "$FIXTURE_REPO" init -q 2>/dev/null

    git -C "$FIXTURE_REPO" config user.email "test@test.local"
    git -C "$FIXTURE_REPO" config user.name "Test"
    # Disable commit signing so fixture commits don't hit the signing server.
    git -C "$FIXTURE_REPO" config commit.gpgsign false
    git -C "$FIXTURE_REPO" config gpg.format ""

    # Initial commit so HEAD is valid and we have a 'main' branch.
    echo "init" > "$FIXTURE_REPO/README"
    git -C "$FIXTURE_REPO" add README
    git -C "$FIXTURE_REPO" commit -q -m "init"
}

# Add a linked worktree to the fixture repo. Returns the worktree path.
# Relies on FIXTURE_DIR and FIXTURE_REPO being set by make_fixture_repo.
make_fixture_worktree() {
    local branch="$1"
    local wt_path="$FIXTURE_DIR/wt-${branch}"
    git -C "$FIXTURE_REPO" worktree add -q -b "$branch" "$wt_path" 2>/dev/null
    echo "$wt_path"
}

# Merge a branch into main (simulates merged state).
merge_branch_into_main() {
    local branch="$1"
    git -C "$FIXTURE_REPO" checkout -q main 2>/dev/null
    git -C "$FIXTURE_REPO" merge -q --no-ff -m "merge $branch" "$branch" 2>/dev/null || true
}

# Create a stub `gh` on a dedicated dir and echo that dir. The stub reports NO
# open PR (empty output, exit 0) so clean-finished classification is exercised
# deterministically regardless of whether real gh is installed in the test env.
make_gh_stub_nopr() {
    local stub_dir="$FIXTURE_DIR/ghstub-nopr"
    mkdir -p "$stub_dir"
    cat > "$stub_dir/gh" <<'STUB'
#!/usr/bin/env bash
# Stub gh: always reports no open PR. `gh pr list ... --jq '.[0].number'`
# yields empty string on an empty array, which is what real gh does.
exit 0
STUB
    chmod +x "$stub_dir/gh"
    echo "$stub_dir"
}

# Run dry-run with a gh stub that affirmatively reports "no PR" (pr_status=none).
run_clean_dryrun() {
    local stub_dir; stub_dir="$(make_gh_stub_nopr)"
    PATH="$stub_dir:$PATH" REPO_ROOT="$FIXTURE_REPO" bash "$IMPL" 2>&1
}

# Run --apply with a gh stub that affirmatively reports "no PR" (pr_status=none).
run_clean_apply() {
    local stub_dir; stub_dir="$(make_gh_stub_nopr)"
    PATH="$stub_dir:$PATH" REPO_ROOT="$FIXTURE_REPO" bash "$IMPL" --apply 2>&1
}

# Create a stub `gh` that FAILS (exit non-zero), simulating gh being unable to
# determine PR status (not authenticated, no network, rate limited). This drives
# pr_status to "unknown" — the same safety-critical outcome as gh being absent.
make_gh_stub_failing() {
    local stub_dir="$FIXTURE_DIR/ghstub-fail"
    mkdir -p "$stub_dir"
    cat > "$stub_dir/gh" <<'STUB'
#!/usr/bin/env bash
echo "gh: simulated failure (no auth/network)" >&2
exit 1
STUB
    chmod +x "$stub_dir/gh"
    echo "$stub_dir"
}

# Run --apply where gh cannot determine PR status (pr_status=unknown) — the
# gh-unavailable safety case. A merged, clean worktree must NOT be deleted here.
run_clean_apply_gh_unknown() {
    local stub_dir; stub_dir="$(make_gh_stub_failing)"
    PATH="$stub_dir:$PATH" REPO_ROOT="$FIXTURE_REPO" bash "$IMPL" --apply 2>&1
}

teardown() {
    if [[ -n "$FIXTURE_DIR" ]] && [[ -d "$FIXTURE_DIR" ]]; then
        rm -rf "$FIXTURE_DIR"
    fi
    FIXTURE_DIR=""
    FIXTURE_REPO=""
}

# ── Spec invariant (a): dirty worktree is classified 'dirty', never deleted ──

# Test 1: Dirty worktree classified as dirty (dry-run).
test_dirty_classified_as_dirty_in_dryrun() {
    make_fixture_repo
    local wt_path
    wt_path="$(make_fixture_worktree "dirty-branch-dr")"

    # Introduce an uncommitted change.
    echo "uncommitted" > "$wt_path/new-file.txt"

    local output exit_code=0
    output="$(run_clean_dryrun)" || exit_code=$?

    teardown

    if echo "$output" | grep -q "dirty"; then
        pass "dirty worktree classified as 'dirty' in dry-run output"
    else
        fail "dirty worktree not classified as dirty — output: ${output:0:400}"
    fi
}

# Test 2 (spec invariant a): dirty worktree NEVER in --apply delete set.
test_dirty_worktree_never_deleted_under_apply() {
    make_fixture_repo
    local wt_path
    wt_path="$(make_fixture_worktree "dirty-branch-apply")"

    # Introduce an uncommitted change.
    echo "dirty data" > "$wt_path/dirty.txt"

    local output exit_code=0
    output="$(run_clean_apply)" || exit_code=$?

    # The worktree directory must still exist.
    local still_exists=0
    [[ -d "$wt_path" ]] && still_exists=1

    teardown

    if [[ "$still_exists" -eq 1 ]]; then
        pass "(spec a) dirty worktree NOT deleted under --apply"
    else
        fail "(spec a) SAFETY VIOLATION: dirty worktree was deleted under --apply"
    fi
}

# Test 3: Output for dirty worktree does not say REMOVE.
test_dirty_worktree_action_is_not_remove() {
    make_fixture_repo
    local wt_path
    wt_path="$(make_fixture_worktree "dirty-no-remove")"
    echo "uncommitted" > "$wt_path/dirty2.txt"

    local output
    output="$(run_clean_apply 2>&1)" || true

    teardown

    # The dirty branch line must not have REMOVE.
    if echo "$output" | grep "dirty-no-remove" | grep -q "REMOVE"; then
        fail "dirty worktree action shows REMOVE — should be REPORT-ONLY"
    else
        pass "dirty worktree action is not REMOVE"
    fi
}

# ── Spec invariant (b): clean merged worktree is 'clean-finished' ────────────────────────

# Test 4 (spec invariant b): clean merged worktree classified as clean-finished.
test_clean_merged_classified_as_clean_finished() {
    make_fixture_repo
    make_fixture_worktree "clean-merged-branch" >/dev/null
    # No uncommitted changes.

    # Merge the branch into main.
    merge_branch_into_main "clean-merged-branch"

    local output exit_code=0
    output="$(run_clean_dryrun)" || exit_code=$?

    teardown

    if echo "$output" | grep -q "clean-finished"; then
        pass "(spec b) clean merged worktree classified as 'clean-finished'"
    else
        fail "(spec b) clean merged worktree not classified as clean-finished — output: ${output:0:400}"
    fi
}

# Test 5 (spec invariant b): clean-finished appears in dry-run output.
test_clean_finished_appears_in_dryrun_output() {
    make_fixture_repo
    make_fixture_worktree "clean-dryrun-branch" >/dev/null
    merge_branch_into_main "clean-dryrun-branch"

    local output exit_code=0
    output="$(run_clean_dryrun)" || exit_code=$?

    teardown

    # dry-run output must mention "would remove" for clean-finished.
    if echo "$output" | grep -qi "would remove\|dry-run.*remove\|clean-finished"; then
        pass "(spec b) clean-finished appears in dry-run output"
    else
        fail "(spec b) clean-finished not in dry-run output — output: ${output:0:400}"
    fi
}

# Test 6: clean-finished IS removed under --apply.
test_clean_finished_removed_under_apply() {
    make_fixture_repo
    local wt_path
    wt_path="$(make_fixture_worktree "clean-apply-branch")"
    merge_branch_into_main "clean-apply-branch"

    local output exit_code=0
    output="$(run_clean_apply)" || exit_code=$?

    # The worktree directory should be gone.
    local removed=0
    [[ ! -d "$wt_path" ]] && removed=1

    teardown

    if [[ "$removed" -eq 1 ]]; then
        pass "clean-finished worktree removed under --apply"
    else
        fail "clean-finished worktree NOT removed under --apply — output: ${output:0:400}"
    fi
}

# ── Dry-run safety: nothing deleted without --apply ────────────────────────────────────────

# Test 7: Dry-run (no --apply) never deletes any worktree.
test_dryrun_never_deletes() {
    make_fixture_repo
    local wt_path
    wt_path="$(make_fixture_worktree "safe-dryrun-branch")"
    merge_branch_into_main "safe-dryrun-branch"

    local output exit_code=0
    output="$(run_clean_dryrun)" || exit_code=$?

    local still_exists=0
    [[ -d "$wt_path" ]] && still_exists=1

    teardown

    if [[ "$still_exists" -eq 1 ]]; then
        pass "dry-run (no --apply) does NOT delete any worktree"
    else
        fail "SAFETY VIOLATION: dry-run deleted a worktree"
    fi
}

# ── Unmerged worktree stays ambiguous ────────────────────────────────────────────────────────────────────

# Test 8: Unmerged clean worktree classified as ambiguous.
test_unmerged_clean_classified_as_ambiguous() {
    make_fixture_repo
    local wt_path
    wt_path="$(make_fixture_worktree "unmerged-clean-branch")"
    # Add a commit on the branch so it diverges from main — making it clearly
    # NOT merged (git branch --merged counts a branch as merged if its tip is
    # reachable from main; a newly-created branch at the same commit IS "merged").
    git -C "$wt_path" config commit.gpgsign false
    git -C "$wt_path" config gpg.format ""
    echo "extra work" > "$wt_path/extra.txt"
    git -C "$wt_path" add extra.txt
    git -C "$wt_path" commit -q -m "extra work on branch"

    # Verify branch is indeed not merged.
    local output exit_code=0
    output="$(run_clean_dryrun)" || exit_code=$?

    teardown

    if echo "$output" | grep "unmerged-clean-branch" | grep -q "ambiguous"; then
        pass "unmerged clean worktree classified as 'ambiguous'"
    else
        fail "unmerged clean worktree should be 'ambiguous' — output: ${output:0:400}"
    fi
}

# Test 9: Ambiguous worktree never deleted under --apply.
test_ambiguous_worktree_never_deleted() {
    make_fixture_repo
    local wt_path
    wt_path="$(make_fixture_worktree "ambiguous-nodel-branch")"
    # Add a commit on the branch so it diverges from main — making it unambiguously
    # NOT merged and NOT dirty (committed work not yet merged = ambiguous).
    git -C "$wt_path" config commit.gpgsign false
    git -C "$wt_path" config gpg.format ""
    echo "committed work" > "$wt_path/work.txt"
    git -C "$wt_path" add work.txt
    git -C "$wt_path" commit -q -m "committed work, not yet merged"

    local output exit_code=0
    output="$(run_clean_apply)" || exit_code=$?

    local still_exists=0
    [[ -d "$wt_path" ]] && still_exists=1

    teardown

    if [[ "$still_exists" -eq 1 ]]; then
        pass "ambiguous worktree NOT deleted under --apply"
    else
        fail "SAFETY VIOLATION: ambiguous worktree was deleted under --apply"
    fi
}

# ── Summary section ────────────────────────────────────────────────────────────────────────────────────

# Test 10: Summary section appears in output.
test_summary_section_appears() {
    make_fixture_repo

    local output exit_code=0
    output="$(run_clean_dryrun)" || exit_code=$?

    teardown

    if echo "$output" | grep -q "summary"; then
        pass "summary section appears in output"
    else
        fail "summary section missing from output — output: ${output:0:400}"
    fi
}

# ── AT-2: locked worktree with dead PID → ambiguous, never deleted ───────────────────────

# Test 11 (spec AT-2): locked worktree with a non-existent pid is ambiguous, never deleted.
# This is the "stale lock after crash" scenario: harness crashed, lock file survived,
# but the pid in the lock reason no longer exists in /proc.
test_locked_dead_pid_classified_as_ambiguous() {
    make_fixture_repo
    local wt_path
    wt_path="$(make_fixture_worktree "locked-dead-pid-branch")"

    # Lock the worktree with a pid that is guaranteed to not exist (PID 0 is the
    # idle/swapper process on Linux and is never a user process; it will not be found
    # in /proc under normal circumstances).
    git -C "$FIXTURE_REPO" worktree lock \
        --reason "claude agent locked-dead-pid-branch (pid 0 start 0)" \
        "$wt_path" 2>/dev/null

    local output exit_code=0
    output="$(run_clean_apply)" || exit_code=$?

    local still_exists=0
    [[ -d "$wt_path" ]] && still_exists=1

    teardown

    if [[ "$still_exists" -eq 1 ]]; then
        pass "(AT-2) locked-with-dead-pid worktree NOT deleted under --apply"
    else
        fail "(AT-2) SAFETY VIOLATION: locked-with-dead-pid worktree was deleted under --apply"
    fi
}

# Test 12 (spec AT-2 classification): locked+dead-pid output contains 'ambiguous'.
test_locked_dead_pid_output_is_ambiguous() {
    make_fixture_repo
    local wt_path
    wt_path="$(make_fixture_worktree "locked-dead-pid-class-branch")"

    git -C "$FIXTURE_REPO" worktree lock \
        --reason "claude agent locked-dead-pid-class-branch (pid 0 start 0)" \
        "$wt_path" 2>/dev/null

    local output exit_code=0
    output="$(run_clean_dryrun)" || exit_code=$?

    teardown

    if echo "$output" | grep "locked-dead-pid-class-branch" | grep -q "ambiguous"; then
        pass "(AT-2) locked+dead-pid worktree classified as 'ambiguous' in output"
    else
        fail "(AT-2) locked+dead-pid worktree not classified as ambiguous — output: ${output:0:400}"
    fi
}

# ── P0 regression: gh unavailable → PR status unknown → never deleted ────────────────────
#
# Guards the factory-droid P0 (#925): a merged, clean, unlocked worktree that
# MIGHT still have an open PR must NOT be deleted when gh cannot determine PR
# status (absent / unauthenticated / offline). "unknown" must fall to ambiguous,
# never clean-finished.

# Test 13: merged+clean worktree with gh failing → NOT deleted under --apply.
test_gh_unknown_merged_clean_not_deleted() {
    make_fixture_repo
    local wt_path
    wt_path="$(make_fixture_worktree "gh-unknown-apply-branch")"
    merge_branch_into_main "gh-unknown-apply-branch"

    local output exit_code=0
    output="$(run_clean_apply_gh_unknown)" || exit_code=$?

    local still_exists=0
    [[ -d "$wt_path" ]] && still_exists=1

    teardown

    if [[ "$still_exists" -eq 1 ]]; then
        pass "(P0) merged+clean worktree NOT deleted when gh PR status is unknown"
    else
        fail "(P0) SAFETY VIOLATION: merged+clean worktree deleted under --apply with gh unavailable"
    fi
}

# Test 14: merged+clean worktree with gh failing → classified 'ambiguous', not clean-finished.
test_gh_unknown_classified_ambiguous() {
    make_fixture_repo
    local wt_path
    wt_path="$(make_fixture_worktree "gh-unknown-class-branch")"
    merge_branch_into_main "gh-unknown-class-branch"

    local output exit_code=0
    output="$(run_clean_apply_gh_unknown)" || exit_code=$?

    teardown

    local line
    line="$(echo "$output" | grep "gh-unknown-class-branch" || true)"
    if echo "$line" | grep -q "ambiguous" && ! echo "$line" | grep -q "clean-finished"; then
        pass "(P0) gh-unknown merged worktree classified 'ambiguous', not clean-finished"
    else
        fail "(P0) gh-unknown merged worktree misclassified — line: ${line:-<none>}"
    fi
}

# ── Run all tests ───────────────────────────────────────────────────────────────────────────────────

echo "=== swarm-clean test suite ==="
echo ""

test_dirty_classified_as_dirty_in_dryrun
test_dirty_worktree_never_deleted_under_apply
test_dirty_worktree_action_is_not_remove
test_clean_merged_classified_as_clean_finished
test_clean_finished_appears_in_dryrun_output
test_clean_finished_removed_under_apply
test_dryrun_never_deletes
test_unmerged_clean_classified_as_ambiguous
test_ambiguous_worktree_never_deleted
test_summary_section_appears
test_locked_dead_pid_classified_as_ambiguous
test_locked_dead_pid_output_is_ambiguous
test_gh_unknown_merged_clean_not_deleted
test_gh_unknown_classified_ambiguous

echo ""
echo "=== Results: $PASS_COUNT passed, $FAIL_COUNT failed ==="

if [[ "$FAIL_COUNT" -gt 0 ]]; then
    exit 1
fi
exit 0
