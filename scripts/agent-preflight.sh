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
#   5 — CARGO_TARGET_DIR is set (defeats automatic per-worktree isolation)
#   6 — stash issue (shared stash has entries — cross-contamination risk)
#
# Usage:
#   bash scripts/agent-preflight.sh
#
# Check 5 ENFORCES target-dir isolation (issue #3854). Cargo's default
# (unconfigured) target-dir already resolves per-worktree — agents must NOT
# export CARGO_TARGET_DIR. This check FAILS (non-zero exit) when the
# variable is set: a stale shell-profile export from a prior session or a
# different worktree/branch silently overrides the correct per-worktree
# default for every subsequently-sourced shell, redirecting builds to the
# wrong worktree (the "stale-binary trap"). There is no legitimate reason
# for an agent to set it under the current convention — unset it and rely
# on the default.

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
# Cargo's default (unconfigured) target-dir resolves to
# <workspace-root>/target, which for a git-worktree checkout is
# <this-worktree>/target — already isolated, automatically, with no setup
# step (issue #3854). A stray CARGO_TARGET_DIR in the environment (usually a
# stale `export` left in a shell profile from a prior session or a different
# worktree/branch) silently overrides that per-worktree default for every
# subsequently-sourced shell — the "stale-binary trap." This check FAILS if
# it's set: unsetting it (not documenting around it) is the enforcement
# point.

if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
    err "CARGO_TARGET_DIR is set (\"$CARGO_TARGET_DIR\") — unset it; cargo's default is already per-worktree isolated and a stale export redirects builds to the wrong worktree (see WORKTREE_PROTOCOL.md / #3854)."
    echo "    Fix: unset CARGO_TARGET_DIR"
    echo "    If this came from a shell profile (~/.bashrc, ~/.zshrc), remove that"
    echo "    line — it is a leftover from another worktree/branch/session, not a"
    echo "    legitimate setting under the current convention."
    TARGET_DIR_OK=false
else
    ok "CARGO_TARGET_DIR is unset — cargo will use this worktree's own target/ (isolation is automatic)"
    TARGET_DIR_OK=true
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

if [[ "$TARGET_DIR_OK" == false ]]; then
    exit 5
fi

if [[ "$STASH_OK" == false ]]; then
    exit 6
fi

echo ""
echo "Preflight passed. Safe to begin work."
exit 0
