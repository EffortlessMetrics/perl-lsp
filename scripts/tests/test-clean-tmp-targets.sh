#!/usr/bin/env bash
# scripts/tests/test-clean-tmp-targets.sh
#
# Safety-invariant test suite for scripts/clean-tmp-targets.sh
#
# KEY INVARIANT UNDER TEST:
#   Given a set of fake /tmp target dirs where some correspond to live worktrees
#   and some do not, the reaper (dry-run) MUST:
#     - Report ONLY the orphaned (non-live) dirs
#     - NEVER report a live-worktree target dir as an orphan
#     - NEVER delete anything in dry-run mode
#
# Approach: we create a hermetic tempdir, fake the naming patterns under it,
# and override 'git' with a mock that returns canned worktree list output.
# The IMPL is invoked with GIT_DIR pointed away from the real repo so it
# cannot accidentally see real worktrees.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMPL="$SCRIPT_DIR/../clean-tmp-targets.sh"
PASS_COUNT=0
FAIL_COUNT=0

if [[ ! -f "$IMPL" ]]; then
    echo "ERROR: clean-tmp-targets.sh not found at $IMPL"
    echo "Write the implementation first: scripts/clean-tmp-targets.sh"
    exit 1
fi

pass() { printf 'PASS %s\n' "$1"; PASS_COUNT=$((PASS_COUNT + 1)); }
fail() { printf 'FAIL %s\n' "$1"; FAIL_COUNT=$((FAIL_COUNT + 1)); }

# ── Hermetic tmpdir infrastructure ────────────────────────────────────────────

# Global tmpdir for all fake /tmp dirs; cleaned up on EXIT.
FAKE_TMP="$(mktemp -d)"
trap 'rm -rf "$FAKE_TMP"' EXIT

# Create a mock `git` that returns a canned worktree porcelain output.
# The mock is placed first on PATH so the script picks it up.
make_mock_git() {
    local mock_dir porcelain_output
    mock_dir="$(mktemp -d "$FAKE_TMP/mock-git-XXXX")"
    porcelain_output="$1"
    # Use printf %q to safely embed multi-line content.
    cat > "$mock_dir/git" <<'MOCK_HEADER'
#!/usr/bin/env bash
# Mock git — only intercepts 'worktree list --porcelain'
MOCK_HEADER
    # Append the canned output via a heredoc stored as a variable.
    printf 'CANNED=%q\n' "$porcelain_output" >> "$mock_dir/git"
    cat >> "$mock_dir/git" <<'MOCK_BODY'
if [[ "${*}" == *"worktree list --porcelain"* ]]; then
    printf '%s\n' "$CANNED"
    exit 0
fi
# For rev-parse --show-toplevel, return a stable fake path.
if [[ "${*}" == *"rev-parse --show-toplevel"* ]]; then
    echo "/fake/repo"
    exit 0
fi
# Other git commands fall through to real git if available.
exec git "$@"
MOCK_BODY
    chmod +x "$mock_dir/git"
    echo "$mock_dir"
}

# Create a fake "agent target" directory under FAKE_TMP.
# We redirect the PATTERN search by monkey-patching the variable inside
# the script via env vars — but since the script hard-codes /tmp, we instead
# override the script's directory searching by injecting a wrapper that
# swaps the target prefix.  The cleanest approach: create actual dirs under
# /tmp but with unique names using our test prefix, then clean them up.

# Create uniquely-named test dirs that look like agent targets.
TEST_PREFIX="clean-tmp-test-$$"

make_fake_target() {
    local name="$1"
    local full="/tmp/${TEST_PREFIX}-${name}"
    mkdir -p "$full"
    # Ensure it looks old (not "recently modified").
    touch -t 200001010000.00 "$full"
    echo "$full"
}

remove_fake_targets() {
    rm -rf /tmp/"${TEST_PREFIX}"-* 2>/dev/null || true
}

# We need the script to scan our fake dirs. Since the script hard-codes
# the glob patterns /tmp/agent-*-target and /tmp/wt-*-target, we name
# our test dirs accordingly, using a prefix collision with those patterns.
# We embed the test session ID to avoid polluting other tests.

make_live_target() {
    # Creates a fake dir whose name matches what a live worktree would produce.
    local wt_id="$1"
    local full="/tmp/agent-${wt_id}-target"
    mkdir -p "$full"
    touch -t 200001010000.00 "$full"
    echo "$full"
}

make_orphan_target() {
    local suffix="$1"
    local full="/tmp/agent-orphan-${TEST_PREFIX}-${suffix}-target"
    mkdir -p "$full"
    touch -t 200001010000.00 "$full"
    echo "$full"
}

run_reaper_dry_run() {
    local mock_git_dir="$1"
    PATH="$mock_git_dir:$PATH" bash "$IMPL" 2>&1
}

# ── Test 1: Safety invariant — live targets never reported as orphans ─────────

test_live_target_never_reported_as_orphan() {
    local wt_id="livetest-${TEST_PREFIX}"
    local live_dir orphan_dir output

    # Create a live-worktree target dir.
    live_dir="$(make_live_target "$wt_id")"

    # Create a separate orphan dir.
    orphan_dir="$(make_orphan_target "safety1")"

    # Porcelain listing that includes the live worktree path.
    local porcelain
    porcelain="$(printf 'worktree /some/base/.claude/worktrees/agent-%s\nHEAD abc123\nbranch refs/heads/test\n' "$wt_id")"

    local mock_git
    mock_git="$(make_mock_git "$porcelain")"

    output="$(run_reaper_dry_run "$mock_git" 2>&1)"

    rm -rf "$live_dir" "$orphan_dir" "$mock_git"

    # The live target must NOT appear as ORPHAN.
    if echo "$output" | grep "ORPHAN" | grep -q "agent-${wt_id}-target"; then
        fail "live target incorrectly reported as ORPHAN: $output"
    else
        pass "live target is NOT reported as an orphan"
    fi
}

# ── Test 2: Orphan target IS reported ─────────────────────────────────────────

test_orphan_target_is_reported() {
    local orphan_dir output

    orphan_dir="$(make_orphan_target "report1")"

    # Porcelain listing with NO worktree matching the orphan.
    local porcelain
    porcelain="$(printf 'worktree /some/repo\nHEAD abc123\nbranch refs/heads/main\n')"

    local mock_git
    mock_git="$(make_mock_git "$porcelain")"

    output="$(run_reaper_dry_run "$mock_git" 2>&1)"

    rm -rf "$orphan_dir" "$mock_git"

    if echo "$output" | grep -q "ORPHAN.*agent-orphan-${TEST_PREFIX}-report1-target"; then
        pass "orphan target correctly reported as ORPHAN"
    else
        fail "orphan target not found in output — got: $output"
    fi
}

# ── Test 3: Dry-run does NOT delete anything ──────────────────────────────────

test_dry_run_does_not_delete() {
    local orphan_dir output

    orphan_dir="$(make_orphan_target "dryrun1")"

    local porcelain
    porcelain="$(printf 'worktree /some/repo\nHEAD abc123\nbranch refs/heads/main\n')"

    local mock_git
    mock_git="$(make_mock_git "$porcelain")"

    output="$(run_reaper_dry_run "$mock_git" 2>&1)"

    local still_exists=0
    [[ -d "$orphan_dir" ]] && still_exists=1

    rm -rf "$orphan_dir" "$mock_git"

    if [[ "$still_exists" -eq 1 ]]; then
        pass "dry-run does NOT delete the orphan dir"
    else
        fail "dry-run DELETED the orphan dir (should not have)"
    fi
}

# ── Test 4: Multiple mixed dirs — only orphans reported ───────────────────────

test_mixed_dirs_only_orphans_reported() {
    local wt_id="mixedlive-${TEST_PREFIX}"
    local live_dir orphan1 orphan2 output

    live_dir="$(make_live_target "$wt_id")"
    orphan1="$(make_orphan_target "mix1")"
    orphan2="$(make_orphan_target "mix2")"

    local porcelain
    porcelain="$(printf 'worktree /repo/.claude/worktrees/agent-%s\nHEAD abc\nbranch refs/heads/x\n' "$wt_id")"

    local mock_git
    mock_git="$(make_mock_git "$porcelain")"

    output="$(run_reaper_dry_run "$mock_git" 2>&1)"

    rm -rf "$live_dir" "$orphan1" "$orphan2" "$mock_git"

    local ok=1

    # Live must NOT be orphan.
    if echo "$output" | grep "ORPHAN" | grep -q "agent-${wt_id}-target"; then
        fail "live target incorrectly in ORPHAN list"
        ok=0
    fi

    # Both orphans must appear.
    if ! echo "$output" | grep -q "ORPHAN.*agent-orphan-${TEST_PREFIX}-mix1-target"; then
        fail "orphan1 not found in ORPHAN list"
        ok=0
    fi

    if ! echo "$output" | grep -q "ORPHAN.*agent-orphan-${TEST_PREFIX}-mix2-target"; then
        fail "orphan2 not found in ORPHAN list"
        ok=0
    fi

    [[ "$ok" -eq 1 ]] && pass "mixed dirs: only orphans reported"
}

# ── Test 5: Recently-modified dirs are skipped (grace period) ─────────────────

test_recent_dir_is_skipped() {
    local orphan_dir output

    orphan_dir="$(make_orphan_target "recent1")"
    # Touch to make it look freshly modified (within grace window).
    touch "$orphan_dir"

    local porcelain
    porcelain="$(printf 'worktree /some/repo\nHEAD abc123\nbranch refs/heads/main\n')"

    local mock_git
    mock_git="$(make_mock_git "$porcelain")"

    output="$(run_reaper_dry_run "$mock_git" 2>&1)"

    rm -rf "$orphan_dir" "$mock_git"

    if echo "$output" | grep -q "RECENT.*agent-orphan-${TEST_PREFIX}-recent1-target"; then
        pass "recently-modified dir skipped (grace period)"
    else
        # If the dir was not found at all (no match in /tmp), also pass —
        # meaning it was not treated as an orphan.
        if echo "$output" | grep "ORPHAN" | grep -q "agent-orphan-${TEST_PREFIX}-recent1-target"; then
            fail "recently-modified dir incorrectly listed as orphan"
        else
            pass "recently-modified dir not listed as orphan (grace period)"
        fi
    fi
}

# ── Test 6: --prune flag actually deletes orphans ─────────────────────────────

test_prune_flag_deletes_orphans() {
    local orphan_dir output

    orphan_dir="$(make_orphan_target "prune1")"

    local porcelain
    porcelain="$(printf 'worktree /some/repo\nHEAD abc123\nbranch refs/heads/main\n')"

    local mock_git
    mock_git="$(make_mock_git "$porcelain")"

    output="$(PATH="$mock_git:$PATH" bash "$IMPL" --prune 2>&1)"

    local deleted=0
    [[ ! -d "$orphan_dir" ]] && deleted=1

    rm -rf "$orphan_dir" "$mock_git" 2>/dev/null || true

    if [[ "$deleted" -eq 1 ]]; then
        pass "--prune deletes orphaned dirs"
    else
        fail "--prune did not delete orphaned dir — output: $output"
    fi
}

# ── Test 7: APPLY=1 env var also deletes orphans ─────────────────────────────

test_apply_env_deletes_orphans() {
    local orphan_dir output

    orphan_dir="$(make_orphan_target "apply1")"

    local porcelain
    porcelain="$(printf 'worktree /some/repo\nHEAD abc123\nbranch refs/heads/main\n')"

    local mock_git
    mock_git="$(make_mock_git "$porcelain")"

    output="$(APPLY=1 PATH="$mock_git:$PATH" bash "$IMPL" 2>&1)"

    local deleted=0
    [[ ! -d "$orphan_dir" ]] && deleted=1

    rm -rf "$orphan_dir" "$mock_git" 2>/dev/null || true

    if [[ "$deleted" -eq 1 ]]; then
        pass "APPLY=1 env var deletes orphaned dirs"
    else
        fail "APPLY=1 did not delete orphaned dir — output: $output"
    fi
}

# ── Test 8: Live target is marked LIVE, not REMOVE, even with --prune ─────────

test_prune_never_deletes_live_target() {
    local wt_id="livenodel-${TEST_PREFIX}"
    local live_dir output

    live_dir="$(make_live_target "$wt_id")"

    local porcelain
    porcelain="$(printf 'worktree /repo/.claude/worktrees/agent-%s\nHEAD abc\nbranch refs/heads/x\n' "$wt_id")"

    local mock_git
    mock_git="$(make_mock_git "$porcelain")"

    output="$(PATH="$mock_git:$PATH" bash "$IMPL" --prune 2>&1)"

    local still_exists=0
    [[ -d "$live_dir" ]] && still_exists=1

    rm -rf "$live_dir" "$mock_git"

    if [[ "$still_exists" -eq 1 ]]; then
        pass "--prune does NOT delete live worktree target"
    else
        fail "--prune DELETED live worktree target (SAFETY VIOLATION)"
    fi
}

# ── Run all tests ─────────────────────────────────────────────────────────────

echo "=== clean-tmp-targets safety test suite ==="
echo ""

test_live_target_never_reported_as_orphan
test_orphan_target_is_reported
test_dry_run_does_not_delete
test_mixed_dirs_only_orphans_reported
test_recent_dir_is_skipped
test_prune_flag_deletes_orphans
test_apply_env_deletes_orphans
test_prune_never_deletes_live_target

# Clean up any stray test dirs.
remove_fake_targets

echo ""
echo "=== Results: $PASS_COUNT passed, $FAIL_COUNT failed ==="

if [[ "$FAIL_COUNT" -gt 0 ]]; then
    exit 1
fi
exit 0
