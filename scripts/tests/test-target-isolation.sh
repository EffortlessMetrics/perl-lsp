#!/usr/bin/env bash
# Self-test for per-worktree cargo build-artifact isolation (issue #3854, M5).
#
# Background: builder.md and WORKTREE_PROTOCOL.md used to instruct agents to
# manually `export CARGO_TARGET_DIR="/tmp/agent-<branch>-target"` on every new
# worktree. That convention was the actual bug: a stale export left in a
# shell profile (or inherited by a differently-named worktree/branch) silently
# overrode cargo's per-worktree default for every subsequent shell, regardless
# of which worktree it ran in — the "stale-binary trap" (Fixture 1 of #3777).
#
# The fix retires that convention rather than replacing it with a new
# committed `.cargo/config.toml target-dir` override: cargo's own default
# (unconfigured) `target-dir` already resolves to `<workspace-root>/target`,
# and for a `git worktree` checkout the workspace root IS the worktree's own
# directory — so two independently created worktrees already get two
# different, non-clobbering `target/` directories with zero configuration.
#
# A global `target-dir` override to a new directory name (e.g.
# `./.cargo-target`) was evaluated and rejected for this increment: xtask
# (release.rs, gates.rs, e2e_validate.rs, compare.rs, test_lsp.rs) and two CI
# workflows (ci.yml, ci-nightly.yml) hardcode literal `target/debug` /
# `target/release` binary paths that a new target-dir name would silently
# stop resolving — a real, sprawling, high-risk coupling, not a bounded one.
# See the PR body for the full blast-radius finding.
#
# This test proves the ACTUAL mechanism (cargo's default, unconfigured
# per-worktree target-dir) by creating two throwaway worktrees the way agents
# actually get them (`git worktree add --detach <tmp> <sha>`, mirroring
# harness `isolation: worktree` behavior), and — with CARGO_TARGET_DIR unset —
# asserting `cargo metadata --no-deps --format-version 1 -q | jq -r
# .target_directory` resolves to `<worktree>/target` in each, and that the two
# worktrees' resolved paths differ.
#
# The throwaway worktrees are cut from the CURRENT checked-out commit
# (`git rev-parse HEAD` of this checkout), not `origin/main`. This is
# deliberate: a future PR that changes target-dir configuration (e.g.
# `.cargo/config.toml`) must have THIS test exercise that PR's own HEAD, not
# whatever is on `origin/main` at the time the test happens to run — otherwise
# the self-test would silently validate a stale baseline instead of the
# change actually under test.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

PASS_COUNT=0
FAIL_COUNT=0
TMPDIR_BASE=""

pass() { printf 'PASS %s\n' "$1"; PASS_COUNT=$((PASS_COUNT + 1)); }
fail() { printf 'FAIL %s\n' "$1"; FAIL_COUNT=$((FAIL_COUNT + 1)); }

cleanup() {
  if [[ -n "${TMPDIR_BASE:-}" ]]; then
    if [[ -d "${TMPDIR_BASE}/wt1" ]]; then
      git -C "$REPO_ROOT" worktree remove --force "${TMPDIR_BASE}/wt1" >/dev/null 2>&1 || true
    fi
    if [[ -d "${TMPDIR_BASE}/wt2" ]]; then
      git -C "$REPO_ROOT" worktree remove --force "${TMPDIR_BASE}/wt2" >/dev/null 2>&1 || true
    fi
    git -C "$REPO_ROOT" worktree prune >/dev/null 2>&1 || true
    rm -rf "${TMPDIR_BASE}"
  fi
}
trap cleanup EXIT

if ! command -v jq >/dev/null 2>&1; then
  echo "ERROR: jq not found on PATH"
  exit 1
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "ERROR: cargo not found on PATH"
  exit 1
fi

echo "=== per-worktree cargo target-dir isolation self-test (#3854) ==="
echo ""

TMPDIR_BASE="$(mktemp -d)"

CURRENT_SHA="$(git -C "$REPO_ROOT" rev-parse HEAD)"
echo "Cutting throwaway worktrees from the checked-out commit: ${CURRENT_SHA}"
echo ""

if ! git -C "$REPO_ROOT" worktree add --detach "${TMPDIR_BASE}/wt1" "$CURRENT_SHA" -q 2>/tmp/wt1-add.err; then
  echo "ERROR: failed to create throwaway worktree 1:"
  cat /tmp/wt1-add.err
  exit 1
fi
if ! git -C "$REPO_ROOT" worktree add --detach "${TMPDIR_BASE}/wt2" "$CURRENT_SHA" -q 2>/tmp/wt2-add.err; then
  echo "ERROR: failed to create throwaway worktree 2:"
  cat /tmp/wt2-add.err
  exit 1
fi

# The whole point: CARGO_TARGET_DIR must be unset for this test to exercise
# cargo's default resolution behavior.
unset CARGO_TARGET_DIR || true

TARGET_DIR_1="$(cd "${TMPDIR_BASE}/wt1" && cargo metadata --no-deps --format-version 1 -q 2>/dev/null | jq -r '.target_directory')"
TARGET_DIR_2="$(cd "${TMPDIR_BASE}/wt2" && cargo metadata --no-deps --format-version 1 -q 2>/dev/null | jq -r '.target_directory')"

echo "worktree 1 target_directory: ${TARGET_DIR_1}"
echo "worktree 2 target_directory: ${TARGET_DIR_2}"
echo ""

# --- Assertions --------------------------------------------------------

# 1. Each worktree's target_directory must live under that worktree's own
#    path (not some shared external location). Normalize backslashes to
#    forward slashes first: on Windows, `cargo metadata` (a native binary)
#    emits `C:\...`-style paths while Git Bash's `pwd` emits `/c/...`-style
#    paths for the same location — a string-format mismatch, not a real
#    isolation difference.
norm() { printf '%s' "$1" | tr '\\' '/'; }

WT1_REAL="$(norm "$(cd "${TMPDIR_BASE}/wt1" && pwd)")"
WT2_REAL="$(norm "$(cd "${TMPDIR_BASE}/wt2" && pwd)")"
TARGET_DIR_1_NORM="$(norm "$TARGET_DIR_1")"
TARGET_DIR_2_NORM="$(norm "$TARGET_DIR_2")"

# Compare by basename-of-worktree-dir containment rather than a strict path
# prefix, since Windows drive-letter vs. Git-Bash mount-point notation
# (`C:/...` vs `/c/...`) can otherwise make an identical real path fail a
# literal prefix match.
WT1_LEAF="$(basename "${TMPDIR_BASE}/wt1")"
WT2_LEAF="$(basename "${TMPDIR_BASE}/wt2")"

case "$TARGET_DIR_1_NORM" in
  */"$WT1_LEAF"/*|*/"$WT1_LEAF") pass "worktree 1 target_directory is under its own worktree" ;;
  *) fail "worktree 1 target_directory ($TARGET_DIR_1) is NOT under its own worktree ($WT1_REAL)" ;;
esac

case "$TARGET_DIR_2_NORM" in
  */"$WT2_LEAF"/*|*/"$WT2_LEAF") pass "worktree 2 target_directory is under its own worktree" ;;
  *) fail "worktree 2 target_directory ($TARGET_DIR_2) is NOT under its own worktree ($WT2_REAL)" ;;
esac

# 2. The two worktrees must resolve to DIFFERENT target directories — the
#    actual isolation property this test exists to guard.
if [[ "$TARGET_DIR_1" != "$TARGET_DIR_2" ]]; then
  pass "worktree 1 and worktree 2 resolve to different target directories"
else
  fail "worktree 1 and worktree 2 resolve to the SAME target directory: $TARGET_DIR_1"
fi

# 3. Sanity: also confirmed from a subdirectory inside one worktree (cargo
#    must walk up to find the workspace root, not anchor on cwd).
mkdir -p "${TMPDIR_BASE}/wt1/crates"
TARGET_DIR_1_SUBDIR="$(cd "${TMPDIR_BASE}/wt1/crates" && cargo metadata --no-deps --format-version 1 -q 2>/dev/null | jq -r '.target_directory')"
if [[ "$TARGET_DIR_1_SUBDIR" == "$TARGET_DIR_1" ]]; then
  pass "resolution from a subdirectory inside worktree 1 matches the worktree root's resolution"
else
  fail "subdirectory resolution ($TARGET_DIR_1_SUBDIR) differs from worktree-root resolution ($TARGET_DIR_1)"
fi

echo ""
echo "=== ${PASS_COUNT} passed, ${FAIL_COUNT} failed ==="

if [[ "$FAIL_COUNT" -gt 0 ]]; then
  exit 1
fi
