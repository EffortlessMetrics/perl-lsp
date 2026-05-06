#!/usr/bin/env bash
# Agent preflight safety checks
# Run before any edit-capable agent starts work.
#
# Exit codes:
#   0 — all checks pass
#   1 — branch issue (on master/main or detached HEAD)
#   2 — worktree issue (not running in an isolated git worktree)
#   3 — conflict issue (unresolved merge conflicts present)
#   4 — cwd issue (running from the main repo root, not a worktree path)
#   6 — stash issue (shared stash has entries — cross-contamination risk)
#
# Usage:
#   bash scripts/agent-preflight.sh
#
# Check 5 computes the recommended CARGO_TARGET_DIR and prints it.
# Agents should set CARGO_TARGET_DIR before running cargo commands:
#   export CARGO_TARGET_DIR="/tmp/agent-$(git branch --show-current | tr '/' '-')-target"

set -uo pipefail

PASS=0
FAIL=0

ok()  { printf 'OK  %s\n' "$1"; PASS=$((PASS + 1)); }
err() { printf 'ERR %s\n' "$1"; FAIL=$((FAIL + 1)); }

echo "=== Agent Preflight Checks ==="
echo ""

# ── Check 1: Not on master or main ───────────────────────────────────────────

CURRENT_BRANCH="$(git branch --show-current 2>/dev/null)"

if [[ -z "$CURRENT_BRANCH" ]]; then
    err "Detached HEAD state. Agents must work on a named branch."
    echo "    Fix: git checkout -b agent-<id> or use a worktree with a branch"
    BRANCH_OK=false
elif [[ "$CURRENT_BRANCH" == "master" || "$CURRENT_BRANCH" == "main" ]]; then
    err "On protected branch '$CURRENT_BRANCH'. Agents must not edit master/main directly."
    echo "    Fix: Work in an isolated worktree with its own branch (isolation: worktree)"
    BRANCH_OK=false
else
    ok "Branch: $CURRENT_BRANCH (not master/main)"
    BRANCH_OK=true
fi

# ── Check 2: Running inside a git worktree (not the main checkout) ────────────

GIT_DIR="$(git rev-parse --git-dir 2>/dev/null)"
GIT_COMMON_DIR="$(git rev-parse --git-common-dir 2>/dev/null)"

if [[ "$GIT_DIR" == "$GIT_COMMON_DIR" ]]; then
    # git-dir equals common-dir → this IS the main checkout, not a worktree
    err "Not in an isolated git worktree. Agents require worktree isolation."
    echo "    Fix: Spawn agent with isolation: worktree in the agent definition"
    echo "    The main checkout is: $GIT_COMMON_DIR"
    WORKTREE_OK=false
else
    ok "Worktree: isolated (git-dir=$GIT_DIR)"
    WORKTREE_OK=true
fi

# ── Check 3: No unresolved merge conflicts ────────────────────────────────────

# Search for conflict markers, skipping the .git directory
CONFLICT_FILES="$(grep -rl --exclude-dir='.git' '^<<<<<<< ' . 2>/dev/null || true)"

if [[ -n "$CONFLICT_FILES" ]]; then
    err "Unresolved merge conflict markers found:"
    while IFS= read -r f; do
        echo "    $f"
    done <<< "$CONFLICT_FILES"
    echo "    Fix: Resolve conflicts, then re-run preflight"
    CONFLICT_OK=false
else
    ok "No unresolved merge conflicts"
    CONFLICT_OK=true
fi

# ── Check 4: cwd must not be the main repo root ─────────────────────────────
# An agent in a worktree can still accidentally cd to (or be spawned in) the
# main checkout path.  The Write/Edit tools resolve absolute paths relative to
# cwd, so writing from the main checkout puts files in the wrong place.

MAIN_REPO_RAW="$(git rev-parse --git-common-dir 2>/dev/null | sed 's|/\.git$||; s|^\.git$|.|')"
# Resolve both paths through readlink/pwd -P so symlinks don't cause mismatches
if [[ -n "$MAIN_REPO_RAW" ]]; then
    MAIN_REPO="$(cd "$MAIN_REPO_RAW" 2>/dev/null && pwd -P)" || MAIN_REPO=""
else
    MAIN_REPO=""
fi
CWD="$(pwd -P)"

if [[ -n "$MAIN_REPO" && "$CWD" = "$MAIN_REPO" ]]; then
    err "cwd is the main repo root ($MAIN_REPO). Agents must run from their worktree."
    echo "    Fix: cd \$(git worktree list | grep \$(git branch --show-current) | awk '{print \$1}')"
    CWD_OK=false
else
    ok "cwd is not the main repo root"
    CWD_OK=true
fi

# ── Check 5: CARGO_TARGET_DIR isolation ─────────────────────────────────────
# Shared target/ across worktrees causes phantom test failures from stale
# artifacts. Each agent needs its own CARGO_TARGET_DIR.

if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
    ok "CARGO_TARGET_DIR already set: $CARGO_TARGET_DIR"
else
    BRANCH_SLUG="$(git branch --show-current 2>/dev/null | tr '/' '-')"
    if [[ -n "$BRANCH_SLUG" ]]; then
        RECOMMENDED_TARGET="/tmp/agent-${BRANCH_SLUG}-target"
        ok "Recommended CARGO_TARGET_DIR=$RECOMMENDED_TARGET"
        echo "    Run: export CARGO_TARGET_DIR=\"$RECOMMENDED_TARGET\""
    else
        # Detached HEAD — use a hash-based fallback
        HEAD_SHORT="$(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
        RECOMMENDED_TARGET="/tmp/agent-detached-${HEAD_SHORT}-target"
        ok "Recommended CARGO_TARGET_DIR=$RECOMMENDED_TARGET (detached HEAD fallback)"
        echo "    Run: export CARGO_TARGET_DIR=\"$RECOMMENDED_TARGET\""
    fi
fi

# ── Check 6: No git stash entries (shared across worktrees) ──────────────────

STASH_COUNT="$(git stash list 2>/dev/null | wc -l)"

if [[ "$STASH_COUNT" -gt 0 ]]; then
    err "Git stash has $STASH_COUNT entries. Stash is SHARED across all worktrees — cross-contamination risk."
    echo "    The stash list is a single global list. 'git stash pop' may restore another agent's changes."
    echo "    Alternatives:"
    echo "      Discard changes: git restore <file>"
    echo "      Save WIP:        git commit -m 'wip' on the branch"
    echo "      Abandon all:     git restore ."
    echo "    Fix: Run 'git stash clear' to drop all stash entries, then re-run preflight"
    STASH_OK=false
else
    ok "No git stash entries (stash is shared — safe)"
    STASH_OK=true
fi

# ── Check 7: pre-push hook is current (Windows MAX_PATH guard) ──────────────
# Warning-only — a stale hook does not block agent work, only push.

REPO_ROOT_AGENT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
INSTALLED_HOOK="$(git rev-parse --git-common-dir 2>/dev/null)/hooks/pre-push"
CHECKED_IN_HOOK="$REPO_ROOT_AGENT/hooks/pre-push"
if [ -f "$INSTALLED_HOOK" ] && [ -f "$CHECKED_IN_HOOK" ]; then
    if ! diff -q "$INSTALLED_HOOK" "$CHECKED_IN_HOOK" >/dev/null 2>&1; then
        printf 'WARN pre-push hook is stale (Windows os error 206 risk)\n'
        printf '     Fix: cargo xtask ci-hygiene install-githooks\n'
    else
        ok "pre-push hook is current"
    fi
fi

# ── Summary ───────────────────────────────────────────────────────────────────

echo ""
echo "=== $PASS passed, $FAIL failed ==="

if [[ "$BRANCH_OK" == false ]]; then
    exit 1
fi

if [[ "$WORKTREE_OK" == false ]]; then
    exit 2
fi

if [[ "$CONFLICT_OK" == false ]]; then
    exit 3
fi

if [[ "$CWD_OK" == false ]]; then
    exit 4
fi

if [[ "$STASH_OK" == false ]]; then
    exit 6
fi

echo ""
echo "Preflight passed. Safe to begin work."
exit 0
